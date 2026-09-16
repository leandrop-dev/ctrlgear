//! Routes a connected headset's ChatMix dial to two virtual PipeWire sinks
//! ("CtrlGear Game" / "CtrlGear Chat") that the user manually assigns apps
//! to, continuously syncing each sink's volume to the dial's
//! `game_percent`/`chat_percent`.
//!
//! Runs entirely on a dedicated, persistent worker thread spawned once by
//! [`spawn`] for the app's whole lifetime — never spawned/killed per toggle,
//! which would otherwise let a rapid toggle-off/toggle-on race two workers
//! against the same fixed node names. The worker starts idle (no PipeWire
//! objects, no HID access) and only begins polling after [`WorkerCommand::Enable`].
//!
//! PipeWire objects (`MainLoopRc`/`ContextRc`/`CoreRc`/`Node`) are thread-affine
//! and are created on and never leave this worker thread.

use std::cell::Cell;
use std::rc::Rc;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use ctrlgear_core::registry::{self, Device};

use pipewire as pw;
use pw::properties::properties;
use pw::spa::param::ParamType;
use pw::spa::pod::serialize::PodSerializer;
use pw::spa::pod::{Object, Pod, Property, Value, ValueArray};
use pw::spa::utils::SpaTypes;

const GAME_SINK_NAME: &str = "CtrlGear Game";
const CHAT_SINK_NAME: &str = "CtrlGear Chat";
const TICK_INTERVAL: Duration = Duration::from_millis(100);
const ROUNDTRIP_STEP: Duration = Duration::from_millis(20);
const ROUNDTRIP_TIMEOUT: Duration = Duration::from_millis(300);

/// Commands sent from the UI thread to the persistent worker thread over the
/// channel returned by [`spawn`]. The worker starts idle and only creates
/// PipeWire objects between `Enable` and the next `Disable`/`Shutdown`.
pub enum WorkerCommand {
    Enable,
    Disable,
    Shutdown,
}

/// What the poll loop should do on a given tick, given the current state.
/// Returned by the pure [`next_action`] — see it for the exact rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    NoOp,
    CreateSinksAndPush,
    /// Not produced by [`next_action`] itself (a failed creation attempt
    /// immediately becomes `NoOp` on the following tick, per its rules) —
    /// kept as part of `Action` because the tick loop's own creation-attempt
    /// handling conceptually passes through this outcome after an actually
    /// failed [`PwContext::create_sinks`] call.
    #[allow(dead_code)]
    CreateSinksFailed,
    PushVolumes,
}

/// Pure decision function for a single poll tick. No I/O — safe to unit test
/// directly, following this codebase's pure-decision-function-plus-thin-wrapper
/// pattern (see `gui/src/main.rs`'s `low_battery_transitions`/`check_low_battery`).
///
/// - No device, no sinks: `NoOp`.
/// - Device present, no sinks, no prior failure: `CreateSinksAndPush` — this
///   is what lets a device that connects after the toggle was switched on
///   still activate the feature, since this runs every tick.
/// - Device present, no sinks, prior failure already recorded: `NoOp` — don't
///   retry-spam every 100ms. The caller clears the failure flag whenever the
///   held device transitions from absent to present, so a real device
///   state change (e.g. losing and regaining the device) still retries.
/// - Device present, sinks exist: `PushVolumes`, regardless of the failure
///   flag (irrelevant once sinks already exist).
/// - Device NOT present, sinks exist: `NoOp` — sinks are only destroyed by
///   `Disable`/`Shutdown`, never just because the device disconnected.
fn next_action(device_present: bool, sinks_exist: bool, sinks_failed: bool) -> Action {
    match (device_present, sinks_exist, sinks_failed) {
        (false, _, _) => Action::NoOp,
        (true, true, _) => Action::PushVolumes,
        (true, false, true) => Action::NoOp,
        (true, false, false) => Action::CreateSinksAndPush,
    }
}

/// Finds the first connected device whose profile supports ChatMix, mirroring
/// `cli/src/main.rs`'s `open_matching` pattern filtered to `supports_chatmix()`.
fn discover_chatmix_device() -> Option<Device> {
    let api = hidapi::HidApi::new().ok()?;
    for profile in registry::PROFILES {
        for &product_id in profile.product_ids {
            if let Ok(device) = registry::open(&api, product_id) {
                if device.supports_chatmix() {
                    return Some(device);
                }
            }
        }
    }
    None
}

/// The two live sink node proxies, owned for as long as the feature is enabled.
struct Sinks {
    game: pw::node::Node,
    chat: pw::node::Node,
}

/// The worker thread's PipeWire connection. Created once, lazily reused
/// across `Enable`/`Disable` cycles for the worker's lifetime — a connected
/// client with no nodes is invisible to `wpctl status`, so keeping it open
/// while idle doesn't violate "zero nodes / no polling" while disabled.
struct PwContext {
    mainloop: pw::main_loop::MainLoopRc,
    core: pw::core::CoreRc,
}

impl PwContext {
    fn connect() -> Result<Self, String> {
        pw::init();
        let mainloop = pw::main_loop::MainLoopRc::new(None)
            .map_err(|e| format!("failed to create PipeWire main loop: {e}"))?;
        let context = pw::context::ContextRc::new(&mainloop, None)
            .map_err(|e| format!("failed to create PipeWire context: {e}"))?;
        let core = context
            .connect_rc(None)
            .map_err(|e| format!("failed to connect to PipeWire: {e}"))?;
        Ok(Self { mainloop, core })
    }

    /// Non-blocking pump of the PipeWire loop, so bound listeners/proxies get
    /// their events processed between ticks without blocking this thread.
    fn pump(&self) {
        self.mainloop.loop_().iterate(pw::loop_::Timeout::None);
    }

    /// Pumps the loop in bounded steps until a `core.sync` reply for this
    /// roundtrip is seen or `max_wait` elapses — never blocks indefinitely,
    /// since this worker also owns the tick loop that must keep servicing
    /// `Disable`/`Shutdown`.
    fn roundtrip(&self, max_wait: Duration) {
        let done = Rc::new(Cell::new(false));
        let done_clone = done.clone();
        let Ok(pending) = self.core.sync(0) else { return };
        let _listener = self
            .core
            .add_listener_local()
            .done(move |id, seq| {
                if id == pw::core::PW_ID_CORE && seq == pending {
                    done_clone.set(true);
                }
            })
            .register();

        let mut waited = Duration::ZERO;
        while !done.get() && waited < max_wait {
            self.mainloop.loop_().iterate(pw::loop_::Timeout::Finite(ROUNDTRIP_STEP));
            waited += ROUNDTRIP_STEP;
        }
    }

    /// Creates a single plain `Audio/Sink` node via the `adapter` factory
    /// backed by `support.null-audio-sink` — the same mechanism PipeWire's
    /// own virtual-sink config snippets and `pw-loopback`/`module-null-sink`
    /// use, which produces a real `Audio/Sink` node (not a `Filter`).
    fn create_one_sink(&self, name: &str) -> Option<pw::node::Node> {
        self.core
            .create_object::<pw::node::Node>(
                "adapter",
                &properties! {
                    "factory.name" => "support.null-audio-sink",
                    "node.name" => name,
                    "node.description" => name,
                    "media.class" => "Audio/Sink",
                    "audio.position" => "FL,FR",
                    "object.linger" => "false",
                },
            )
            .ok()
    }

    /// Attempts to create both sink nodes. If either fails (including a
    /// PipeWire-side error reported asynchronously during the roundtrip
    /// below), destroys whichever one succeeded rather than leaving a
    /// half-created pair, and returns `Err`.
    fn create_sinks(&self) -> Result<Sinks, String> {
        let failed = Rc::new(Cell::new(false));
        let failed_clone = failed.clone();
        let error_listener = self
            .core
            .add_listener_local()
            .error(move |_id, _seq, _res, message| {
                eprintln!("chatmix: PipeWire reported an error while creating sinks: {message}");
                failed_clone.set(true);
            })
            .register();

        let game = self.create_one_sink(GAME_SINK_NAME);
        let chat = self.create_one_sink(CHAT_SINK_NAME);

        self.roundtrip(ROUNDTRIP_TIMEOUT);
        drop(error_listener);

        match (game, chat) {
            (Some(game), Some(chat)) if !failed.get() => Ok(Sinks { game, chat }),
            (game, chat) => {
                if let Some(game) = game {
                    let _ = self.core.destroy_object(game);
                }
                if let Some(chat) = chat {
                    let _ = self.core.destroy_object(chat);
                }
                self.roundtrip(ROUNDTRIP_STEP * 5);
                Err("Failed to create one or both ChatMix sink nodes.".to_string())
            }
        }
    }

    fn destroy_sinks(&self, sinks: Sinks) {
        let _ = self.core.destroy_object(sinks.game);
        let _ = self.core.destroy_object(sinks.chat);
        self.roundtrip(ROUNDTRIP_STEP * 5);
    }

    /// Sets a node's linear channel volume (0.0-1.0, no curve) via a
    /// `SPA_PROP_channelVolumes` `Props` param.
    fn push_volume(&self, node: &pw::node::Node, percent: u8) {
        let linear = (percent as f32 / 100.0).clamp(0.0, 1.0);
        let object = Object {
            type_: SpaTypes::ObjectParamProps.as_raw(),
            id: ParamType::Props.as_raw(),
            properties: vec![Property::new(
                pw::spa::sys::SPA_PROP_channelVolumes as u32,
                Value::ValueArray(ValueArray::Float(vec![linear, linear])),
            )],
        };

        let Ok((cursor, _)) =
            PodSerializer::serialize(std::io::Cursor::new(Vec::new()), &Value::Object(object))
        else {
            return;
        };
        let bytes = cursor.into_inner();
        if let Some(pod) = Pod::from_bytes(&bytes) {
            node.set_param(ParamType::Props, 0, pod);
        }
    }
}

/// Reads the held device's ChatMix status under [`crate::HID_BUS_LOCK`] (held
/// for the full HID transaction), so this never interleaves with the
/// existing battery-poll timer's own HID traffic against the same device.
fn read_chatmix(device: &Device) -> Option<ctrlgear_core::protocol::ChatMixStatus> {
    let _guard = crate::HID_BUS_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    device.chatmix_status().ok()
}

/// Reads the current ChatMix status and pushes it to both sinks. Returns
/// `false` if the HID read failed, so the caller can drop the held device to
/// force rediscovery next tick — sink volumes are left at their last value.
fn read_and_push(pw_ctx: &PwContext, sinks: &Sinks, device: &Device) -> bool {
    match read_chatmix(device) {
        Some(status) => {
            pw_ctx.push_volume(&sinks.game, status.game_percent);
            pw_ctx.push_volume(&sinks.chat, status.chat_percent);
            true
        }
        None => false,
    }
}

/// Spawns the persistent chatmix worker thread and returns the command
/// channel to it plus a receiver that gets a single message right before the
/// thread exits after `Shutdown` (used for `tray.on_quit`'s bounded wait).
///
/// `startup_error` is written whenever sink creation fails, whether from an
/// ordinary manual toggle-on or from the automatic startup `Enable` sent
/// when the setting was persisted as on — read (and cleared) once by
/// `gui/src/main.rs`'s `on_settings_opened` handler.
pub fn spawn(startup_error: Arc<Mutex<Option<String>>>) -> (mpsc::Sender<WorkerCommand>, mpsc::Receiver<()>) {
    let (cmd_tx, cmd_rx) = mpsc::channel();
    let (done_tx, done_rx) = mpsc::channel();
    thread::Builder::new()
        .name("chatmix-worker".to_string())
        .spawn(move || run(cmd_rx, done_tx, startup_error))
        .expect("failed to spawn chatmix worker thread");
    (cmd_tx, done_rx)
}

fn report_failure(startup_error: &Arc<Mutex<Option<String>>>, message: String) {
    eprintln!("chatmix: {message}");
    *startup_error.lock().unwrap_or_else(|e| e.into_inner()) = Some(message);
}

/// Drains every [`WorkerCommand`] buffered on `cmd_rx` since the last tick
/// and reduces the batch to `(latest_toggle, should_shutdown)`.
///
/// `Shutdown` is reflected in `should_shutdown` regardless of where in the
/// batch it appears or what follows it — it must never be silently
/// overridden by a rogue trailing `Enable`/`Disable` in the same drain pass.
/// `Enable`/`Disable` are order-sensitive: `latest_toggle` (`Some(true)` for
/// the former, `Some(false)` for the latter) is overwritten by each one as
/// it's processed in arrival order, so e.g. `Enable, Disable, Enable` in one
/// batch correctly reduces to `Some(true)` rather than collapsing to
/// "disabled because a Disable was seen somewhere in the batch". `None`
/// means no `Enable`/`Disable` was seen at all.
fn drain_commands(cmd_rx: &mpsc::Receiver<WorkerCommand>) -> (Option<bool>, bool) {
    let mut latest_toggle: Option<bool> = None;
    let mut should_shutdown = false;
    loop {
        match cmd_rx.try_recv() {
            Ok(WorkerCommand::Enable) => latest_toggle = Some(true),
            Ok(WorkerCommand::Disable) => latest_toggle = Some(false),
            Ok(WorkerCommand::Shutdown) => should_shutdown = true,
            Err(mpsc::TryRecvError::Empty) => break,
            Err(mpsc::TryRecvError::Disconnected) => {
                should_shutdown = true;
                break;
            }
        }
    }
    (latest_toggle, should_shutdown)
}

fn run(cmd_rx: mpsc::Receiver<WorkerCommand>, done_tx: mpsc::Sender<()>, startup_error: Arc<Mutex<Option<String>>>) {
    let pw_ctx = match PwContext::connect() {
        Ok(ctx) => Some(ctx),
        Err(e) => {
            eprintln!("chatmix: {e}");
            None
        }
    };

    let mut held_device: Option<Device> = None;
    let mut sinks: Option<Sinks> = None;
    let mut sinks_failed = false;

    loop {
        // Idle: block waiting for the next command — no polling, no PipeWire
        // objects created, no HID access until `Enable` arrives.
        match cmd_rx.recv() {
            Ok(WorkerCommand::Enable) => {}
            Ok(WorkerCommand::Disable) => continue,
            Ok(WorkerCommand::Shutdown) | Err(_) => {
                if let (Some(pw_ctx), Some(s)) = (&pw_ctx, sinks.take()) {
                    pw_ctx.destroy_sinks(s);
                }
                let _ = done_tx.send(());
                return;
            }
        }

        // Enabled: poll roughly every 100ms until Disable/Shutdown.
        loop {
            let (latest_toggle, should_shutdown) = drain_commands(&cmd_rx);

            if should_shutdown {
                if let (Some(pw_ctx), Some(s)) = (&pw_ctx, sinks.take()) {
                    pw_ctx.destroy_sinks(s);
                }
                let _ = done_tx.send(());
                return;
            }
            if latest_toggle == Some(false) {
                if let (Some(pw_ctx), Some(s)) = (&pw_ctx, sinks.take()) {
                    pw_ctx.destroy_sinks(s);
                }
                sinks_failed = false;
                held_device = None;
                break;
            }

            if held_device.is_none() {
                let discovered = discover_chatmix_device();
                if discovered.is_some() {
                    // A device (re)appeared: a real state change, so allow a
                    // previously-failed creation attempt to retry.
                    sinks_failed = false;
                }
                held_device = discovered;
            }

            let device_present = held_device.is_some();
            let sinks_exist = sinks.is_some();

            match next_action(device_present, sinks_exist, sinks_failed) {
                Action::NoOp | Action::CreateSinksFailed => {}
                Action::CreateSinksAndPush => match &pw_ctx {
                    Some(pw_ctx) => match pw_ctx.create_sinks() {
                        Ok(new_sinks) => {
                            if let Some(device) = &held_device {
                                if !read_and_push(pw_ctx, &new_sinks, device) {
                                    held_device = None;
                                }
                            }
                            sinks = Some(new_sinks);
                            sinks_failed = false;
                        }
                        Err(e) => {
                            sinks_failed = true;
                            report_failure(&startup_error, e);
                        }
                    },
                    None => {
                        sinks_failed = true;
                        report_failure(
                            &startup_error,
                            "PipeWire is not available; ChatMix sinks were not created.".to_string(),
                        );
                    }
                },
                Action::PushVolumes => {
                    if let (Some(pw_ctx), Some(s), Some(device)) = (&pw_ctx, &sinks, &held_device) {
                        if !read_and_push(pw_ctx, s, device) {
                            held_device = None;
                        }
                    }
                }
            }

            if let Some(pw_ctx) = &pw_ctx {
                pw_ctx.pump();
            }
            thread::sleep(TICK_INTERVAL);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_action_no_device_no_sinks_is_noop() {
        assert_eq!(next_action(false, false, false), Action::NoOp);
    }

    #[test]
    fn next_action_no_device_no_sinks_with_stale_failure_is_still_noop() {
        assert_eq!(next_action(false, false, true), Action::NoOp);
    }

    #[test]
    fn next_action_device_present_no_sinks_no_failure_creates_and_pushes() {
        assert_eq!(next_action(true, false, false), Action::CreateSinksAndPush);
    }

    #[test]
    fn next_action_device_present_no_sinks_prior_failure_does_not_retry_spam() {
        assert_eq!(next_action(true, false, true), Action::NoOp);
    }

    #[test]
    fn next_action_device_present_sinks_exist_pushes_volumes() {
        assert_eq!(next_action(true, true, false), Action::PushVolumes);
    }

    #[test]
    fn next_action_device_present_sinks_exist_pushes_volumes_even_with_stale_failure_flag() {
        assert_eq!(next_action(true, true, true), Action::PushVolumes);
    }

    #[test]
    fn next_action_device_absent_sinks_exist_does_not_destroy_them() {
        assert_eq!(next_action(false, true, false), Action::NoOp);
    }

    #[test]
    fn next_action_device_absent_sinks_exist_with_stale_failure_is_still_noop() {
        assert_eq!(next_action(false, true, true), Action::NoOp);
    }

    #[test]
    fn drain_commands_enable_disable_enable_ends_up_enabled() {
        let (tx, rx) = mpsc::channel();
        tx.send(WorkerCommand::Enable).unwrap();
        tx.send(WorkerCommand::Disable).unwrap();
        tx.send(WorkerCommand::Enable).unwrap();

        assert_eq!(drain_commands(&rx), (Some(true), false));
    }

    #[test]
    fn drain_commands_enable_shutdown_enable_still_shuts_down() {
        let (tx, rx) = mpsc::channel();
        tx.send(WorkerCommand::Enable).unwrap();
        tx.send(WorkerCommand::Shutdown).unwrap();
        tx.send(WorkerCommand::Enable).unwrap();

        assert_eq!(drain_commands(&rx), (Some(true), true));
    }

    #[test]
    fn drain_commands_disable_enable_disable_ends_up_disabled() {
        let (tx, rx) = mpsc::channel();
        tx.send(WorkerCommand::Disable).unwrap();
        tx.send(WorkerCommand::Enable).unwrap();
        tx.send(WorkerCommand::Disable).unwrap();

        assert_eq!(drain_commands(&rx), (Some(false), false));
    }

    #[test]
    fn drain_commands_empty_channel_is_no_toggle_no_shutdown() {
        let (_tx, rx) = mpsc::channel();
        assert_eq!(drain_commands(&rx), (None, false));
    }
}
