//! DPI, polling-rate, and button-mapping write commands, plus the static
//! button-choice data the frontend needs to render the button-mapping
//! dropdowns. Mirrors `gui/src/main.rs`'s `apply_dpi`/`apply_polling_rate`/
//! `apply_button_mapping` and `BUTTON_ACTION_KEYWORDS`.

use crate::commands::HID_BUS_LOCK;
use ctrlgear_core::buttons::{Button, ButtonAction, ButtonMapping};
use ctrlgear_core::device::profile::{DeviceProfile, DpiProfile, PollingRateProfile, RgbProfile};
use ctrlgear_core::registry::{self, Device};
use hidapi::HidApi;
use std::str::FromStr;

/// Keyword/label pairs for the button-mapping action dropdowns, in the same
/// order as the `ComboBox` `model` in `gui/ui/main.slint`'s button-mapping
/// section. Keywords must each parse via [`ButtonAction::from_str`].
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

/// One label per physical button slot, in [`Button::all`] order. Matches
/// `gui/ui/main.slint`'s button-row labels.
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
/// RGB zone-color inputs. Ported from `gui/src/main.rs::parse_hex_color`,
/// including its `is_ascii_hexdigit()` guard — without it, `hex.len() == 6`
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

/// Sets a device's DPI presets. Mirrors `gui/src/main.rs`'s `apply_dpi`.
#[tauri::command]
#[specta::specta]
pub async fn set_dpi(product_id: u16, values: Vec<u16>) -> Result<(), String> {
    let profile = profile_for(product_id)?;
    let dpi = profile.dpi.ok_or_else(|| format!("{} has no configurable DPI", profile.name))?;
    validate_dpi_values(&dpi, &values)?;

    with_device(product_id, move |device| device.set_dpi(&values).map_err(|e| e.to_string())).await
}

/// Sets a device's polling rate. Mirrors `gui/src/main.rs`'s `apply_polling_rate`.
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
/// [`Button::all`] slot. Mirrors `gui/src/main.rs`'s `apply_button_mapping`.
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

#[cfg(test)]
mod tests {
    use super::*;
    use ctrlgear_core::device::rival3_wireless::{PRODUCT_ID as RIVAL3_WIRELESS_PRODUCT_ID, RIVAL3_WIRELESS_PROFILE};

    #[test]
    fn button_action_choices_has_11_entries_in_slint_order() {
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
