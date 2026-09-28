//! System tray icon, battery-row menu, and menu click handling.

use crate::config::TrayIconStyle;
use crate::TrayState;
use ctrlgear_core::registry::DeviceSummary;
use tauri::image::Image;
use tauri::menu::{IsMenuItem, Menu, MenuEvent, MenuItem, MenuItemBuilder, PredefinedMenuItem};
use tauri::tray::{TrayIcon, TrayIconBuilder};
use tauri::{AppHandle, Manager, Theme, Wry};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TrayAsset {
    Colored,
    MonoLight,
    MonoDark,
}

/// Picks the tray asset: monochrome contrasts with the theme, colored ignores it.
fn tray_asset_for(style: TrayIconStyle, theme: Theme) -> TrayAsset {
    match style {
        TrayIconStyle::Colored => TrayAsset::Colored,
        TrayIconStyle::Monochrome => match theme {
            Theme::Dark => TrayAsset::MonoLight,
            _ => TrayAsset::MonoDark,
        },
    }
}

fn tray_icon_bytes(asset: TrayAsset) -> &'static [u8] {
    match asset {
        TrayAsset::Colored => include_bytes!("../icons/tray/colored.png"),
        TrayAsset::MonoLight => include_bytes!("../icons/tray/mono-light.png"),
        TrayAsset::MonoDark => include_bytes!("../icons/tray/mono-dark.png"),
    }
}

fn tray_image(asset: TrayAsset) -> tauri::Result<Image<'static>> {
    Image::from_bytes(tray_icon_bytes(asset))
}

/// Strips the redundant "SteelSeries" prefix from a device name for display.
fn display_name(name: &str) -> &str {
    name.strip_prefix("SteelSeries ").unwrap_or(name)
}

/// Formats each battery-powered device as "Name — NN%" for the tray menu's
/// disabled info rows. Devices without a battery are omitted.
fn format_device_labels(devices: &[DeviceSummary]) -> Vec<String> {
    devices
        .iter()
        .filter_map(|d| {
            let battery = d.battery?;
            let charging = if battery.is_charging { " (Charging)" } else { "" };
            Some(format!("{} — {}%{}", display_name(&d.name), battery.level, charging))
        })
        .collect()
}

fn build_menu(app: &AppHandle, labels: &[String]) -> tauri::Result<Menu<Wry>> {
    let mut items: Vec<Box<dyn IsMenuItem<Wry>>> = Vec::new();
    for label in labels {
        items.push(Box::new(MenuItem::new(app, label, false, None::<&str>)?));
    }
    if !labels.is_empty() {
        items.push(Box::new(PredefinedMenuItem::separator(app)?));
    }
    items.push(Box::new(MenuItemBuilder::with_id("show_window", "Show window").build(app)?));
    items.push(Box::new(MenuItemBuilder::with_id("quit", "Quit").build(app)?));

    let refs: Vec<&dyn IsMenuItem<Wry>> = items.iter().map(AsRef::as_ref).collect();
    Menu::with_items(app, &refs)
}

fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

fn handle_menu_event(app: &AppHandle, event: MenuEvent) {
    match event.id().as_ref() {
        "show_window" => show_main_window(app),
        "quit" => app.exit(0),
        _ => {}
    }
}

/// Builds the tray icon, its (initially empty) menu, and wires the menu
/// click handler. Called once from `setup`.
pub(crate) fn build(app: &AppHandle, style: TrayIconStyle, theme: Theme) -> tauri::Result<TrayIcon<Wry>> {
    let menu = build_menu(app, &[])?;
    let icon = tray_image(tray_asset_for(style, theme))?;
    TrayIconBuilder::new()
        .menu(&menu)
        .icon(icon)
        .tooltip("CtrlGear")
        .on_menu_event(handle_menu_event)
        .build(app)
}

/// Rebuilds the tray menu's device-label rows from the latest poll. No-op if
/// the tray failed to build at startup.
pub(crate) fn refresh_menu(app: &AppHandle, devices: &[DeviceSummary]) {
    let Some(tray) = app.state::<TrayState>().0.clone() else { return };
    match build_menu(app, &format_device_labels(devices)) {
        Ok(menu) => {
            if let Err(e) = tray.set_menu(Some(menu)) {
                eprintln!("Failed to set tray menu: {e}");
            }
        }
        Err(e) => eprintln!("Failed to build tray menu: {e}"),
    }
}

/// Swaps the tray icon for `style`/`theme`. No-op if the tray failed to
/// build at startup.
pub(crate) fn apply_icon(app: &AppHandle, style: TrayIconStyle, theme: Theme) {
    let Some(tray) = app.state::<TrayState>().0.clone() else { return };
    match tray_image(tray_asset_for(style, theme)) {
        Ok(image) => {
            if let Err(e) = tray.set_icon(Some(image)) {
                eprintln!("Failed to set tray icon: {e}");
            }
        }
        Err(e) => eprintln!("Failed to load tray icon: {e}"),
    }
}

/// The main window's current theme, or [`Theme::Light`] if it can't be read.
pub(crate) fn main_window_theme(app: &AppHandle) -> Theme {
    app.get_webview_window("main").and_then(|w| w.theme().ok()).unwrap_or(Theme::Light)
}

/// [`apply_icon`] using [`main_window_theme`].
pub(crate) fn apply_icon_style(app: &AppHandle, style: TrayIconStyle) {
    apply_icon(app, style, main_window_theme(app));
}

#[cfg(test)]
mod tests {
    use super::*;
    use ctrlgear_core::protocol::BatteryStatus;
    use ctrlgear_core::registry::ConnectionType;

    fn device(name: &str, battery: Option<BatteryStatus>) -> DeviceSummary {
        DeviceSummary {
            product_id: 0,
            name: name.to_string(),
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
    fn format_device_labels_empty_for_no_devices() {
        assert_eq!(format_device_labels(&[]), Vec::<String>::new());
    }

    #[test]
    fn format_device_labels_skips_devices_without_battery() {
        let devices = vec![device("SteelSeries Rival 3 Wireless", None)];
        assert_eq!(format_device_labels(&devices), Vec::<String>::new());
    }

    #[test]
    fn format_device_labels_formats_name_and_percent() {
        let devices = vec![device(
            "SteelSeries Arctis Nova 7",
            Some(BatteryStatus { level: 42, is_charging: false }),
        )];
        assert_eq!(format_device_labels(&devices), vec!["Arctis Nova 7 — 42%"]);
    }

    #[test]
    fn format_device_labels_handles_multiple_devices() {
        let devices = vec![
            device("SteelSeries Rival 3 Wireless", Some(BatteryStatus { level: 80, is_charging: true })),
            device("SteelSeries Arctis Nova 7", Some(BatteryStatus { level: 15, is_charging: false })),
        ];
        assert_eq!(
            format_device_labels(&devices),
            vec!["Rival 3 Wireless — 80% (Charging)", "Arctis Nova 7 — 15%"]
        );
    }

    #[test]
    fn format_device_labels_marks_charging_devices() {
        let devices = vec![device(
            "SteelSeries Rival 3 Wireless",
            Some(BatteryStatus { level: 55, is_charging: true }),
        )];
        assert_eq!(format_device_labels(&devices), vec!["Rival 3 Wireless — 55% (Charging)"]);
    }

    #[test]
    fn tray_asset_for_colored_ignores_theme() {
        assert_eq!(tray_asset_for(TrayIconStyle::Colored, Theme::Light), TrayAsset::Colored);
        assert_eq!(tray_asset_for(TrayIconStyle::Colored, Theme::Dark), TrayAsset::Colored);
    }

    #[test]
    fn tray_asset_for_monochrome_light_theme_uses_dark_icon() {
        assert_eq!(tray_asset_for(TrayIconStyle::Monochrome, Theme::Light), TrayAsset::MonoDark);
    }

    #[test]
    fn tray_asset_for_monochrome_dark_theme_uses_light_icon() {
        assert_eq!(tray_asset_for(TrayIconStyle::Monochrome, Theme::Dark), TrayAsset::MonoLight);
    }
}
