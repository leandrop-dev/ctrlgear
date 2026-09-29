//! DPI, polling-rate, and button-mapping write commands, plus the static
//! button-choice data the frontend needs to render the button-mapping
//! dropdowns.

use crate::commands::HID_BUS_LOCK;
use ctrlgear_core::buttons::{Button, ButtonAction, ButtonMapping};
use ctrlgear_core::device::profile::{
    validate_bluetooth_call_volume_mode, validate_equalizer_preset, validate_mic_mute_led_level,
    validate_mic_volume_level, validate_sidetone_level, DeviceProfile, DpiProfile,
    PollingRateProfile, RgbProfile,
};
use ctrlgear_core::registry::{self, Device};
use hidapi::HidApi;
use std::str::FromStr;

/// Keyword/label pairs for the button-mapping action dropdowns. Keywords
/// must each parse via [`ButtonAction::from_str`].
const BUTTON_ACTION_CHOICES: [(&str, &str); 11] = [
    ("disabled", "Disabled"),
    ("dpiswitch", "DPI Switch"),
    ("scrollup", "Scroll Up"),
    ("scrolldown", "Scroll Down"),
    ("default", "Default"),
    ("button1", "Button 1"),
    ("button2", "Button 2"),
    ("button3", "Button 3"),
    ("button4", "Button 4"),
    ("button5", "Button 5"),
    ("button6", "Button 6"),
];

/// One label per physical button slot, in [`Button::all`] order.
const BUTTON_SLOT_LABELS: [&str; 6] = [
    "Button 1 (Left-Click)",
    "Button 2 (Right-Click)",
    "Button 3 (Middle-Click)",
    "Button 4 (Forward)",
    "Button 5 (Back)",
    "Button 6 (DPI Button)",
];

#[derive(Debug, Clone, serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ButtonActionChoice {
    pub keyword: String,
    pub label: String,
}

/// The button-mapping action choices for populating the frontend's
/// dropdowns. Pure/static — no HID access.
#[tauri::command]
#[specta::specta]
pub fn button_action_choices() -> Vec<ButtonActionChoice> {
    BUTTON_ACTION_CHOICES
        .iter()
        .map(|(keyword, label)| ButtonActionChoice {
            keyword: keyword.to_string(),
            label: label.to_string(),
        })
        .collect()
}

/// One label per physical button slot, for the frontend's button-mapping
/// rows. Pure/static — no HID access.
#[tauri::command]
#[specta::specta]
pub fn button_slot_labels() -> Vec<String> {
    BUTTON_SLOT_LABELS.iter().map(|s| s.to_string()).collect()
}

/// Looks up `product_id`'s profile, erroring the same way [`registry::open`]
/// does when the device isn't supported. Pure — no HID access.
fn profile_for(product_id: u16) -> Result<&'static DeviceProfile, String> {
    registry::profile_for(product_id)
        .ok_or_else(|| {
            format!(
                "No configuration support for device {:04x}:{product_id:04x}",
                ctrlgear_core::protocol::STEELSERIES_VENDOR_ID
            )
        })
}

/// Validates a DPI preset list against a device's [`DpiProfile`]: non-empty,
/// no more than `max_presets` values, and every value present in the
/// device's DPI mapping. Pure — mirrors `ConfigurableDpi::set_dpi`'s
/// validation in `core/src/device/profile.rs` without needing an open device.
fn validate_dpi_values(dpi: &DpiProfile, values: &[u16]) -> Result<(), String> {
    if values.is_empty() {
        return Err("At least one DPI value required".to_string());
    }

    if values.len() > dpi.max_presets as usize {
        return Err(format!("Maximum {} DPI presets allowed", dpi.max_presets));
    }

    if let Some(&bad) = values.iter().find(|v| !dpi.mapping.iter().any(|(d, _)| d == *v)) {
        let min = dpi.mapping.first().map(|(d, _)| *d).unwrap_or(0);
        let max = dpi.mapping.last().map(|(d, _)| *d).unwrap_or(0);
        let step = dpi.mapping.get(1).map(|(d, _)| d.saturating_sub(min)).unwrap_or(0);
        return Err(format!("Invalid DPI value: {bad}. Must be {min}-{max} in steps of {step}"));
    }

    Ok(())
}

/// Validates a polling-rate value against a device's [`PollingRateProfile`]
/// choices. Pure — no HID access.
fn validate_polling_rate(polling_rate: &PollingRateProfile, hz: u16) -> Result<(), String> {
    if polling_rate.choices.iter().any(|(h, _)| *h == hz) {
        Ok(())
    } else {
        let choices = polling_rate.choices.iter().map(|(h, _)| h.to_string()).collect::<Vec<_>>().join(", ");
        Err(format!("Invalid polling rate: {hz}. Valid options: {choices}"))
    }
}

/// Validates and parses one keyword per physical button slot into
/// [`ButtonAction`]s. Pure — no HID access.
fn validate_button_actions(actions: &[String]) -> Result<Vec<ButtonAction>, String> {
    let expected = Button::all().len();
    if actions.len() != expected {
        return Err(format!("Expected {expected} button actions, got {}", actions.len()));
    }

    actions.iter().map(|a| ButtonAction::from_str(a).map_err(|e| e.to_string())).collect()
}

/// Validates an RGB zone index against a device's [`RgbProfile`] zone
/// table. Pure — no HID access.
fn validate_rgb_zone(rgb: &RgbProfile, zone: u8) -> Result<(), String> {
    if rgb.zones.iter().any(|(z, _)| *z == zone) {
        Ok(())
    } else {
        let zones = rgb.zones.iter().map(|(z, _)| z.to_string()).collect::<Vec<_>>().join(", ");
        Err(format!("Invalid RGB zone: {zone}. Must be one of: {zones}"))
    }
}

/// Validates a named default-lighting choice against a device's
/// [`RgbProfile`] choice table. Pure — no HID access.
fn validate_default_lighting_choice(rgb: &RgbProfile, choice: &str) -> Result<(), String> {
    if rgb.default_lighting_choices.iter().any(|(name, _)| *name == choice) {
        Ok(())
    } else {
        let choices = rgb.default_lighting_choices.iter().map(|(n, _)| *n).collect::<Vec<_>>().join(", ");
        Err(format!("Invalid default lighting: {choice}. Must be one of: {choices}"))
    }
}

/// Parses a `#rrggbb` (or bare `rrggbb`) hex color string, as typed into the
/// RGB zone-color inputs. Includes an `is_ascii_hexdigit()` guard — without
/// it, `hex.len() == 6`
/// (a *byte* length) can pass for a shorter multi-byte-UTF-8 string (e.g.
/// "aµcde"), and the subsequent byte-index slicing then panics instead of
/// erroring.
fn parse_hex_color(s: &str) -> Result<(u8, u8, u8), String> {
    let hex = s.trim().trim_start_matches('#');
    let invalid = || format!("Invalid color: {s}");
    if hex.len() != 6 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(invalid());
    }
    let r = u8::from_str_radix(&hex[0..2], 16).map_err(|_| invalid())?;
    let g = u8::from_str_radix(&hex[2..4], 16).map_err(|_| invalid())?;
    let b = u8::from_str_radix(&hex[4..6], 16).map_err(|_| invalid())?;
    Ok((r, g, b))
}

/// Opens `product_id` under [`HID_BUS_LOCK`] on the blocking pool and runs
/// `f` against it. Every device-writing command goes through this so a
/// command and the poll loop's `discover()` never talk to the bus at once.
async fn with_device<T, F>(product_id: u16, f: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce(&Device) -> Result<T, String> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = HID_BUS_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let api = HidApi::new().map_err(|e| format!("Failed to open HID API: {e}"))?;
        let device = registry::open(&api, product_id).map_err(|e| e.to_string())?;
        f(&device)
    })
    .await
    .unwrap_or_else(|e| Err(format!("Device command task panicked: {e}")))
}

/// Sets a device's DPI presets.
#[tauri::command]
#[specta::specta]
pub async fn set_dpi(product_id: u16, values: Vec<u16>) -> Result<(), String> {
    let profile = profile_for(product_id)?;
    let dpi = profile.dpi.ok_or_else(|| format!("{} has no configurable DPI", profile.name))?;
    validate_dpi_values(&dpi, &values)?;

    with_device(product_id, move |device| device.set_dpi(&values).map_err(|e| e.to_string())).await
}

/// Sets a device's polling rate.
#[tauri::command]
#[specta::specta]
pub async fn set_polling_rate(product_id: u16, hz: u16) -> Result<(), String> {
    let profile = profile_for(product_id)?;
    let polling_rate =
        profile.polling_rate.ok_or_else(|| format!("{} has no configurable polling rate", profile.name))?;
    validate_polling_rate(&polling_rate, hz)?;

    with_device(product_id, move |device| device.set_polling_rate(hz).map_err(|e| e.to_string())).await
}

/// Sets a device's button mapping. `actions` is one keyword per
/// [`Button::all`] slot.
#[tauri::command]
#[specta::specta]
pub async fn set_button_mapping(product_id: u16, actions: Vec<String>) -> Result<(), String> {
    let profile = profile_for(product_id)?;
    profile
        .buttons
        .ok_or_else(|| format!("{} has no configurable button mapping", profile.name))?;
    let parsed = validate_button_actions(&actions)?;

    let mut mapping = ButtonMapping::default_mapping();
    for (button, action) in Button::all().into_iter().zip(parsed) {
        mapping.set(button, action);
    }

    with_device(product_id, move |device| device.set_button_mapping(&mapping).map_err(|e| e.to_string())).await
}

/// Sets one RGB zone's color.
#[tauri::command]
#[specta::specta]
pub async fn set_rgb_zone(product_id: u16, zone: u8, hex_color: String) -> Result<(), String> {
    let profile = profile_for(product_id)?;
    let rgb = profile.rgb.ok_or_else(|| format!("{} has no configurable RGB lighting", profile.name))?;
    validate_rgb_zone(&rgb, zone)?;
    let (r, g, b) = parse_hex_color(&hex_color)?;

    with_device(product_id, move |device| device.set_rgb_zone_color(zone, r, g, b).map_err(|e| e.to_string())).await
}

/// Sets a device's RGB LED brightness (0-100).
#[tauri::command]
#[specta::specta]
pub async fn set_rgb_brightness(product_id: u16, level: u8) -> Result<(), String> {
    let profile = profile_for(product_id)?;
    let rgb = profile.rgb.ok_or_else(|| format!("{} has no configurable RGB lighting", profile.name))?;
    rgb.brightness_command
        .ok_or_else(|| format!("{} has no configurable LED brightness", profile.name))?;

    with_device(product_id, move |device| device.set_rgb_brightness(level).map_err(|e| e.to_string())).await
}

/// Enables the RGB rainbow effect across every zone. There's no direct
/// "off" command, so the frontend shows an explanatory message instead of
/// calling this when the rainbow checkbox is unchecked.
#[tauri::command]
#[specta::specta]
pub async fn set_rgb_rainbow(product_id: u16) -> Result<(), String> {
    let profile = profile_for(product_id)?;
    let rgb = profile.rgb.ok_or_else(|| format!("{} has no configurable RGB lighting", profile.name))?;
    rgb.rainbow_command
        .ok_or_else(|| format!("{} has no rainbow effect", profile.name))?;

    with_device(product_id, move |device| device.set_rgb_rainbow_effect("all").map_err(|e| e.to_string())).await
}

/// Sets a device's default (startup) lighting.
#[tauri::command]
#[specta::specta]
pub async fn set_rgb_default_lighting(product_id: u16, choice: String) -> Result<(), String> {
    let profile = profile_for(product_id)?;
    let rgb = profile.rgb.ok_or_else(|| format!("{} has no configurable RGB lighting", profile.name))?;
    validate_default_lighting_choice(&rgb, &choice)?;

    with_device(product_id, move |device| device.set_rgb_default_lighting(&choice).map_err(|e| e.to_string())).await
}

/// Sets a headset's sidetone (mic monitoring) level (0-3, Off/Low/Medium/High).
#[tauri::command]
#[specta::specta]
pub async fn set_sidetone(product_id: u16, level: u8) -> Result<(), String> {
    let profile = profile_for(product_id)?;
    profile.sidetone.ok_or_else(|| format!("{} has no configurable sidetone", profile.name))?;
    validate_sidetone_level(level).map_err(|e| e.to_string())?;

    with_device(product_id, move |device| device.set_sidetone(level).map_err(|e| e.to_string())).await
}

/// Sets a headset's mic-mute LED brightness (0-3, off to max).
#[tauri::command]
#[specta::specta]
pub async fn set_mic_mute_led(product_id: u16, level: u8) -> Result<(), String> {
    let profile = profile_for(product_id)?;
    profile.mic_mute_led.ok_or_else(|| format!("{} has no configurable mic mute LED", profile.name))?;
    validate_mic_mute_led_level(level).map_err(|e| e.to_string())?;

    with_device(product_id, move |device| device.set_mic_mute_led_brightness(level).map_err(|e| e.to_string())).await
}

/// Sets a headset's microphone input volume (0-7, off to max).
#[tauri::command]
#[specta::specta]
pub async fn set_mic_volume(product_id: u16, level: u8) -> Result<(), String> {
    let profile = profile_for(product_id)?;
    profile.mic_volume.ok_or_else(|| format!("{} has no configurable mic volume", profile.name))?;
    validate_mic_volume_level(level).map_err(|e| e.to_string())?;

    with_device(product_id, move |device| device.set_mic_volume(level).map_err(|e| e.to_string())).await
}

/// Applies one of a headset's 4 built-in equalizer presets (Flat/Bass/Focus/
/// Smiley, in that index order) — no manual per-band sliders.
#[tauri::command]
#[specta::specta]
pub async fn set_equalizer_preset(product_id: u16, preset: u8) -> Result<(), String> {
    let profile = profile_for(product_id)?;
    let equalizer = profile.equalizer.ok_or_else(|| format!("{} has no configurable equalizer", profile.name))?;
    validate_equalizer_preset(&equalizer, preset).map_err(|e| e.to_string())?;

    with_device(product_id, move |device| device.set_equalizer_preset(preset).map_err(|e| e.to_string())).await
}

/// Sets a headset's auto-sleep timer (minutes of inactivity, 0-255; `0`
/// disables it).
#[tauri::command]
#[specta::specta]
pub async fn set_sleep_timer(product_id: u16, minutes: u8) -> Result<(), String> {
    let profile = profile_for(product_id)?;
    profile.sleep_timer.ok_or_else(|| format!("{} has no configurable sleep timer", profile.name))?;

    with_device(product_id, move |device| device.set_sleep_timer(minutes).map_err(|e| e.to_string())).await
}

/// Toggles a headset's output volume limiter.
#[tauri::command]
#[specta::specta]
pub async fn set_volume_limiter(product_id: u16, enabled: bool) -> Result<(), String> {
    let profile = profile_for(product_id)?;
    profile.volume_limiter.ok_or_else(|| format!("{} has no configurable volume limiter", profile.name))?;

    with_device(product_id, move |device| device.set_volume_limiter(enabled).map_err(|e| e.to_string())).await
}

/// Toggles whether a headset turns Bluetooth on when it powers on.
#[tauri::command]
#[specta::specta]
pub async fn set_bluetooth_power_on(product_id: u16, enabled: bool) -> Result<(), String> {
    let profile = profile_for(product_id)?;
    profile.bluetooth.ok_or_else(|| format!("{} has no configurable bluetooth settings", profile.name))?;

    with_device(product_id, move |device| device.set_bluetooth_when_powered_on(enabled).map_err(|e| e.to_string())).await
}

/// Sets a headset's Bluetooth call-volume mode (`0` off, `1` lower game
/// volume, `2` mute game during calls).
#[tauri::command]
#[specta::specta]
pub async fn set_bluetooth_call_volume(product_id: u16, mode: u8) -> Result<(), String> {
    let profile = profile_for(product_id)?;
    profile.bluetooth.ok_or_else(|| format!("{} has no configurable bluetooth settings", profile.name))?;
    validate_bluetooth_call_volume_mode(mode).map_err(|e| e.to_string())?;

    with_device(product_id, move |device| device.set_bluetooth_call_volume(mode).map_err(|e| e.to_string())).await
}

/// Reads a device's firmware version. Read-only.
#[tauri::command]
#[specta::specta]
pub async fn get_firmware_version(product_id: u16) -> Result<String, String> {
    let profile = profile_for(product_id)?;
    profile.firmware_version.ok_or_else(|| format!("{} has no firmware version", profile.name))?;

    with_device(product_id, |device| device.firmware_version().map_err(|e| e.to_string())).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use ctrlgear_core::device::rival3_wireless::{PRODUCT_ID as RIVAL3_WIRELESS_PRODUCT_ID, RIVAL3_WIRELESS_PROFILE};

    #[test]
    fn button_action_choices_has_11_entries() {
        let choices = button_action_choices();
        assert_eq!(choices.len(), 11);
        assert_eq!(choices[0].keyword, "disabled");
        assert_eq!(choices[0].label, "Disabled");
        assert_eq!(choices[4].keyword, "default");
        assert_eq!(choices[10].keyword, "button6");
    }

    #[test]
    fn every_button_action_choice_keyword_parses() {
        for (keyword, _) in BUTTON_ACTION_CHOICES {
            assert!(ButtonAction::from_str(keyword).is_ok(), "keyword {keyword} failed to parse");
        }
    }

    #[test]
    fn button_slot_labels_len_matches_button_all_len() {
        assert_eq!(button_slot_labels().len(), Button::all().len());
    }

    fn rival3_dpi() -> DpiProfile {
        RIVAL3_WIRELESS_PROFILE.dpi.unwrap()
    }

    fn rival3_polling_rate() -> PollingRateProfile {
        RIVAL3_WIRELESS_PROFILE.polling_rate.unwrap()
    }

    #[test]
    fn validate_dpi_values_rejects_empty_list() {
        assert!(validate_dpi_values(&rival3_dpi(), &[]).is_err());
    }

    #[test]
    fn validate_dpi_values_rejects_too_many_presets() {
        let dpi = rival3_dpi();
        let values: Vec<u16> = (0..(dpi.max_presets as u16 + 1)).map(|_| 800).collect();
        let err = validate_dpi_values(&dpi, &values).unwrap_err();
        assert!(err.contains("Maximum"), "unexpected error: {err}");
    }

    #[test]
    fn validate_dpi_values_rejects_value_outside_mapping() {
        let err = validate_dpi_values(&rival3_dpi(), &[150]).unwrap_err();
        assert!(err.contains("Invalid DPI value"), "unexpected error: {err}");
    }

    #[test]
    fn validate_dpi_values_accepts_values_within_mapping_and_max_presets() {
        assert!(validate_dpi_values(&rival3_dpi(), &[400, 800, 1600]).is_ok());
    }

    #[test]
    fn validate_polling_rate_rejects_unknown_hz() {
        assert!(validate_polling_rate(&rival3_polling_rate(), 999).is_err());
    }

    #[test]
    fn validate_polling_rate_accepts_known_hz() {
        for hz in [125, 250, 500, 1000] {
            assert!(validate_polling_rate(&rival3_polling_rate(), hz).is_ok());
        }
    }

    #[test]
    fn validate_button_actions_rejects_wrong_length() {
        let actions = vec!["default".to_string(); 5];
        assert!(validate_button_actions(&actions).is_err());
    }

    #[test]
    fn validate_button_actions_rejects_unknown_keyword() {
        let mut actions = vec!["default".to_string(); 6];
        actions[0] = "not-a-real-action".to_string();
        assert!(validate_button_actions(&actions).is_err());
    }

    #[test]
    fn validate_button_actions_accepts_known_keywords() {
        let actions =
            vec!["disabled", "dpiswitch", "scrollup", "scrolldown", "default", "button1"]
                .into_iter()
                .map(String::from)
                .collect::<Vec<_>>();
        let parsed = validate_button_actions(&actions).unwrap();
        assert_eq!(parsed.len(), 6);
        assert_eq!(parsed[0], ButtonAction::Disabled);
        assert_eq!(parsed[1], ButtonAction::DpiSwitch);
        assert_eq!(parsed[5], ButtonAction::MouseButton(Button::Button1));
    }

    #[test]
    fn profile_for_unsupported_product_id_errors() {
        assert!(profile_for(0xffff).is_err());
    }

    #[test]
    fn profile_for_supported_product_id_ok() {
        assert!(profile_for(RIVAL3_WIRELESS_PRODUCT_ID).is_ok());
    }

    use ctrlgear_core::device::aerox_gen2_rgb::{AEROX3_PROFILE, PRIME_MINI_PROFILE};

    fn aerox3_rgb() -> RgbProfile {
        AEROX3_PROFILE.rgb.unwrap()
    }

    #[test]
    fn validate_rgb_zone_accepts_known_zone() {
        assert!(validate_rgb_zone(&aerox3_rgb(), 1).is_ok());
        assert!(validate_rgb_zone(&aerox3_rgb(), 3).is_ok());
    }

    #[test]
    fn validate_rgb_zone_rejects_unknown_zone() {
        let err = validate_rgb_zone(&aerox3_rgb(), 4).unwrap_err();
        assert!(err.contains("Invalid RGB zone"), "unexpected error: {err}");
    }

    #[test]
    fn validate_default_lighting_choice_accepts_known_choice() {
        assert!(validate_default_lighting_choice(&aerox3_rgb(), "reactive-rainbow").is_ok());
    }

    #[test]
    fn validate_default_lighting_choice_rejects_unknown_choice() {
        let err = validate_default_lighting_choice(&aerox3_rgb(), "not-a-choice").unwrap_err();
        assert!(err.contains("Invalid default lighting"), "unexpected error: {err}");
    }

    #[test]
    fn parse_hex_color_parses_valid_input() {
        assert_eq!(parse_hex_color("#ff00aa").unwrap(), (0xff, 0x00, 0xaa));
        assert_eq!(parse_hex_color("00FF00").unwrap(), (0x00, 0xff, 0x00));
    }

    #[test]
    fn parse_hex_color_rejects_wrong_length() {
        assert!(parse_hex_color("#fff").is_err());
        assert!(parse_hex_color("#ff00aabb").is_err());
    }

    #[test]
    fn parse_hex_color_rejects_non_hex_ascii() {
        assert!(parse_hex_color("zzzzzz").is_err());
    }

    #[test]
    fn parse_hex_color_rejects_non_ascii_without_panicking() {
        assert!(parse_hex_color("aµcde").is_err());
    }

    #[test]
    fn prime_mini_rgb_zone_is_1() {
        let rgb = PRIME_MINI_PROFILE.rgb.unwrap();
        assert!(validate_rgb_zone(&rgb, 1).is_ok());
        assert!(validate_rgb_zone(&rgb, 2).is_err());
    }
}
