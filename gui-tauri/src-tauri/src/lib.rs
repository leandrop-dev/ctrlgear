mod commands;
mod config;
mod device_commands;
mod events;
mod notifications;
mod settings_commands;

use config::Settings;
use events::DeviceListUpdated;
use notifications::BatteryState;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;
use tauri::Manager;
use tauri_specta::{collect_commands, collect_events, Event};

/// In-memory settings shared between every `settings_commands::save_*`
/// command and the poll loop's low-battery check; kept in sync with disk.
pub(crate) struct SettingsState {
    pub(crate) current: Mutex<Settings>,
    pub(crate) save_lock: tokio::sync::Mutex<()>,
}

impl SettingsState {
    pub(crate) fn new(settings: Settings) -> Self {
        Self { current: Mutex::new(settings), save_lock: tokio::sync::Mutex::new(()) }
    }
}

/// Per-device low-battery notification state, tracked across polls so the
/// same drop isn't notified twice. Mirrors `gui/src/main.rs`'s `battery_state`.
pub(crate) struct BatteryTrackerState(pub(crate) Mutex<HashMap<u16, BatteryState>>);

/// Lets `settings_commands::save_poll_interval` reconfigure the running poll
/// loop's interval without restarting it.
pub(crate) struct PollIntervalHandle(tokio::sync::watch::Sender<Duration>);

impl PollIntervalHandle {
    pub(crate) fn set_minutes(&self, minutes: u32) {
        let _ = self.0.send(Duration::from_secs(minutes as u64 * 60));
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri_specta::Builder::<tauri::Wry>::new()
        .commands(collect_commands![
            commands::list_devices,
            device_commands::set_dpi,
            device_commands::set_polling_rate,
            device_commands::set_button_mapping,
            device_commands::button_action_choices,
            device_commands::button_slot_labels,
            device_commands::set_rgb_zone,
            device_commands::set_rgb_brightness,
            device_commands::set_rgb_rainbow,
            device_commands::set_rgb_default_lighting,
            device_commands::set_sidetone,
            device_commands::set_mic_mute_led,
            device_commands::set_mic_volume,
            device_commands::set_equalizer_preset,
            device_commands::set_sleep_timer,
            device_commands::set_volume_limiter,
            device_commands::set_bluetooth_power_on,
            device_commands::set_bluetooth_call_volume,
            device_commands::get_firmware_version,
            settings_commands::get_settings,
            settings_commands::save_poll_interval,
            settings_commands::save_tray_icon_style,
            settings_commands::save_notification_persistent,
            settings_commands::save_notification_sound,
            settings_commands::preview_notification_sound,
        ])
        .events(collect_events![DeviceListUpdated]);

    #[cfg(debug_assertions)]
    builder
        .export(specta_typescript::Typescript::default(), "../src/bindings.ts")
        .expect("failed to export typescript bindings");

    let app = tauri::Builder::default();

    #[cfg(debug_assertions)]
    let app = app.plugin(
        tauri_plugin_mcp_bridge::Builder::new()
            .bind_address("127.0.0.1")
            .build(),
    );

    app.invoke_handler(builder.invoke_handler())
        .setup(move |app| {
            builder.mount_events(app);

            let settings = config::load();
            let (poll_interval_tx, poll_interval_rx) =
                tokio::sync::watch::channel(Duration::from_secs(settings.poll_interval_minutes as u64 * 60));
            app.manage(SettingsState::new(settings));
            app.manage(BatteryTrackerState(Mutex::new(HashMap::new())));
            app.manage(PollIntervalHandle(poll_interval_tx));

            spawn_poll_loop(app.handle().clone(), poll_interval_rx);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Spawns the background loop that periodically refreshes the device list,
/// pushes it to the frontend as a [`DeviceListUpdated`] event, and fires any
/// low-battery notifications. `interval_rx` lets
/// [`settings_commands::save_poll_interval`] change the interval live.
fn spawn_poll_loop(app: tauri::AppHandle, mut interval_rx: tokio::sync::watch::Receiver<Duration>) {
    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(*interval_rx.borrow());
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        interval.tick().await;
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    let devices = match commands::discover_devices_async().await {
                        Ok(devices) => devices,
                        Err(e) => {
                            eprintln!("Poll loop skipped a refresh: {e}");
                            continue;
                        }
                    };

                    let settings = *app.state::<SettingsState>().current.lock().unwrap_or_else(|e| e.into_inner());
                    let transitions = {
                        let tracker_state = app.state::<BatteryTrackerState>();
                        let mut tracked = tracker_state.0.lock().unwrap_or_else(|e| e.into_inner());
                        notifications::low_battery_transitions(&devices, &mut tracked)
                    };
                    if !transitions.is_empty() {
                        tauri::async_runtime::spawn_blocking(move || {
                            notifications::fire_low_battery_notifications(
                                transitions,
                                settings.notification_persistent,
                                settings.notification_sound,
                            );
                        });
                    }

                    if let Err(e) = DeviceListUpdated(devices).emit(&app) {
                        eprintln!("Failed to emit device-list-updated: {e}");
                    }
                }
                Ok(()) = interval_rx.changed() => {
                    interval = tokio::time::interval(*interval_rx.borrow());
                    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
                    interval.tick().await;
                }
            }
        }
    });
}
