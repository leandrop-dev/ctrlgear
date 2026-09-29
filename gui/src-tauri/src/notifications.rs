//! Low-battery and sound-preview desktop notifications, via `notify-rust` directly.

use crate::config::SoundChoice;
use ctrlgear_core::registry::DeviceSummary;
use std::collections::HashMap;

const LOW_BATTERY_THRESHOLD: u8 = 25;
const BATTERY_SWAP_JUMP_THRESHOLD: u8 = 15;

/// Strips the redundant "SteelSeries" prefix from a device name for display.
fn display_name(name: &str) -> &str {
    name.strip_prefix("SteelSeries ").unwrap_or(name)
}

fn apply_sound(notification: &mut notify_rust::Notification, sound: SoundChoice) {
    match sound {
        SoundChoice::Silent => {
            notification.hint(notify_rust::Hint::SuppressSound(true));
        }
        SoundChoice::Default => {}
        SoundChoice::DialogWarning => {
            notification.sound_name("dialog-warning");
        }
        SoundChoice::MessageNewInstant => {
            notification.sound_name("message-new-instant");
        }
    }
}

/// Fires a short-lived preview notification for `sound`, so the user can hear
/// it before applying.
pub(crate) fn preview_sound(sound: SoundChoice) -> Result<(), String> {
    let mut notification = notify_rust::Notification::new();
    notification.summary("CtrlGear").body("Notification sound preview").timeout(notify_rust::Timeout::Milliseconds(3000));
    apply_sound(&mut notification, sound);
    notification.show().map(|_| ()).map_err(|e| e.to_string())
}

pub(crate) struct BatteryState {
    notified: bool,
    last_level: u8,
}

/// Determines which devices should fire a low-battery notification this
/// poll, updating `states` accordingly. Pure state-tracking logic with no
/// I/O — the actual `notify-rust` call lives in [`fire_low_battery_notifications`].
pub(crate) fn low_battery_transitions(
    devices: &[DeviceSummary],
    states: &mut HashMap<u16, BatteryState>,
) -> Vec<(String, u8)> {
    let mut transitions = Vec::new();

    for d in devices {
        let Some(battery) = d.battery else { continue };
        let state =
            states.entry(d.product_id).or_insert(BatteryState { notified: false, last_level: battery.level });
        let jumped_significantly = battery.level >= state.last_level.saturating_add(BATTERY_SWAP_JUMP_THRESHOLD);
        state.last_level = battery.level;

        if jumped_significantly && state.notified {
            state.notified = false;
            continue;
        }

        if battery.level < LOW_BATTERY_THRESHOLD && !state.notified {
            state.notified = true;
            transitions.push((display_name(&d.name).to_string(), battery.level));
        } else if battery.level >= LOW_BATTERY_THRESHOLD && state.notified {
            state.notified = false;
        }
    }

    transitions
}

/// Fires a desktop notification for each already-computed low-battery
/// transition (see [`low_battery_transitions`]). Timeout and sound follow
/// `persistent`/`sound`. Takes owned data and holds no lock itself — callers
/// must compute `transitions` under the battery-tracker lock, then drop that
/// lock before calling this, and should run it on a blocking task since
/// `notify-rust`'s `show()` blocks on D-Bus.
pub(crate) fn fire_low_battery_notifications(transitions: Vec<(String, u8)>, persistent: bool, sound: SoundChoice) {
    for (name, level) in transitions {
        let mut notification = notify_rust::Notification::new();
        notification
            .summary("Low battery")
            .body(&format!("{name} is at {level}%"))
            .urgency(notify_rust::Urgency::Critical)
            .timeout(if persistent { notify_rust::Timeout::Never } else { notify_rust::Timeout::Default });
        apply_sound(&mut notification, sound);
        let _ = notification.show();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ctrlgear_core::protocol::BatteryStatus;
    use ctrlgear_core::registry::ConnectionType;

    fn device_with_id(product_id: u16, battery: Option<BatteryStatus>) -> DeviceSummary {
        DeviceSummary {
            product_id,
            name: "Test Device".to_string(),
            supported: true,
            supports_dpi: false,
            supports_polling_rate: false,
            supports_button_mapping: false,
            supports_sidetone: false,
            supports_mic_mute_led: false,
            supports_mic_volume: false,
            supports_equalizer: false,
            supports_sleep_timer: false,
            supports_volume_limiter: false,
            supports_bluetooth_settings: false,
            supports_firmware: false,
            supports_rgb: false,
            rgb_zone_count: 0,
            supports_rgb_brightness: false,
            supports_rgb_rainbow: false,
            rgb_default_lighting_choices: Vec::new(),
            dpi_range: None,
            dpi_max_presets: None,
            polling_rate_choices: Vec::new(),
            connection_type: ConnectionType::Unknown,
            battery,
            chatmix: None,
            note: None,
        }
    }

    #[test]
    fn check_low_battery_inserts_on_first_drop_below_threshold() {
        let devices = vec![device_with_id(1, Some(BatteryStatus { level: 20, is_charging: false }))];
        let mut states = HashMap::new();

        let transitions = low_battery_transitions(&devices, &mut states);

        assert!(states.get(&1).unwrap().notified);
        assert_eq!(transitions, vec![("Test Device".to_string(), 20)]);
    }

    #[test]
    fn check_low_battery_does_not_double_insert_while_still_low() {
        let devices = vec![device_with_id(1, Some(BatteryStatus { level: 20, is_charging: false }))];
        let mut states = HashMap::new();

        low_battery_transitions(&devices, &mut states);
        let second_transitions = low_battery_transitions(&devices, &mut states);

        assert_eq!(states.len(), 1);
        assert!(states.get(&1).unwrap().notified);
        assert!(second_transitions.is_empty());
    }

    #[test]
    fn check_low_battery_clears_on_recovery_above_threshold() {
        let mut states = HashMap::new();
        states.insert(1u16, BatteryState { notified: true, last_level: 20 });
        let devices = vec![device_with_id(1, Some(BatteryStatus { level: 30, is_charging: false }))];

        let transitions = low_battery_transitions(&devices, &mut states);

        assert!(!states.get(&1).unwrap().notified);
        assert!(transitions.is_empty());
    }

    #[test]
    fn check_low_battery_skips_devices_without_battery() {
        let devices = vec![device_with_id(1, None)];
        let mut states = HashMap::new();

        let transitions = low_battery_transitions(&devices, &mut states);

        assert!(states.is_empty());
        assert!(transitions.is_empty());
    }

    #[test]
    fn check_low_battery_swap_resets_episode_without_notifying() {
        let mut states = HashMap::new();
        states.insert(1u16, BatteryState { notified: true, last_level: 10 });
        let jumped = vec![device_with_id(1, Some(BatteryStatus { level: 60, is_charging: false }))];

        let jump_transitions = low_battery_transitions(&jumped, &mut states);

        assert!(!states.get(&1).unwrap().notified);
        assert!(jump_transitions.is_empty());

        let dropped_again = vec![device_with_id(1, Some(BatteryStatus { level: 20, is_charging: false }))];

        let drop_transitions = low_battery_transitions(&dropped_again, &mut states);

        assert!(states.get(&1).unwrap().notified);
        assert_eq!(drop_transitions, vec![("Test Device".to_string(), 20)]);
    }
}
