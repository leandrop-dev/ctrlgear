use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;
use ctrlgear_core::device::arctis_nova_7::PRODUCT_ID as ARCTIS_NOVA_7_PRODUCT_ID;
use ctrlgear_core::device::rival3_wireless::PRODUCT_ID as RIVAL3_WIRELESS_PRODUCT_ID;
use ctrlgear_core::registry::{self, DeviceSummary};
use ctrlgear_core::Error;

mod config;

slint::include_modules!();

/// Colors used for detail-page status text; kept in sync with the `#4caf50`
/// / `#e05050` accents used directly in main.slint for battery/card state.
const STATUS_COLOR_OK: slint::Color = slint::Color::from_rgb_u8(0x4c, 0xaf, 0x50);
const STATUS_COLOR_ERROR: slint::Color = slint::Color::from_rgb_u8(0xe0, 0x50, 0x50);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let window = MainWindow::new()?;
    let tray = AppTray::new()?;

    // Must be set before the window is shown, and after the platform/backend
    // exists (which `MainWindow::new()` just initialized) — `set_xdg_app_id`
    // errors out if called with no platform set up yet. On Wayland, winit
    // can't set a runtime icon at all (icons are resolved from a .desktop
    // file via this app ID) — see packaging/ctrlgear.desktop and the icon
    // installed to hicolor/scalable/apps/ctrlgear.svg by `make install` /
    // the PKGBUILD.
    slint::set_xdg_app_id("ctrlgear")?;

    // Cache of the last discovery, so `select_device`/`apply_dpi` can look
    // up the product ID for the currently selected card by index without
    // re-running discovery (and without threading that data through Slint).
    let summaries: Rc<RefCell<Vec<DeviceSummary>>> = Rc::new(RefCell::new(Vec::new()));
    let selected_product_id: Rc<RefCell<Option<u16>>> = Rc::new(RefCell::new(None));
    let low_battery_notified: Rc<RefCell<std::collections::HashSet<u16>>> =
        Rc::new(RefCell::new(std::collections::HashSet::new()));

    let settings = config::load();
    window.set_poll_interval_minutes(settings.poll_interval_minutes as i32);

    let poll_timer = Rc::new(slint::Timer::default());

    {
        let window = window.as_weak();
        let poll_timer = poll_timer.clone();
        window.unwrap().on_save_poll_interval(move |minutes| {
            let window = window.unwrap();
            let settings = config::Settings { poll_interval_minutes: minutes.max(1) as u32 };
            window.set_poll_interval_minutes(settings.poll_interval_minutes as i32);
            poll_timer.set_interval(Duration::from_secs(settings.poll_interval_minutes as u64 * 60));
            match config::save(&settings) {
                Ok(()) => {
                    window.set_settings_status("Saved.".into());
                    window.set_settings_status_color(STATUS_COLOR_OK);
                }
                Err(e) => {
                    window.set_settings_status(format!("Failed to save: {e}").into());
                    window.set_settings_status_color(STATUS_COLOR_ERROR);
                }
            }

            let window = window.as_weak();
            slint::Timer::single_shot(Duration::from_secs(5), move || {
                if let Some(window) = window.upgrade() {
                    window.set_settings_status("".into());
                }
            });
        });
    }

    {
        let window = window.as_weak();
        let tray = tray.as_weak();
        let summaries = summaries.clone();
        let low_battery_notified = low_battery_notified.clone();
        window.unwrap().on_refresh(move || {
            let window = window.unwrap();
            window.set_loading(true);
            let discovered = discover();
            window.set_devices(to_card_model(&discovered));
            tray.unwrap().set_device_labels(device_labels(&discovered));
            check_low_battery(&discovered, &mut low_battery_notified.borrow_mut());
            *summaries.borrow_mut() = discovered;
            window.set_loading(false);
        });
    }

    {
        let window = window.as_weak();
        let summaries = summaries.clone();
        let selected_product_id = selected_product_id.clone();
        window.unwrap().on_select_device(move |index| {
            let window = window.unwrap();
            let summaries = summaries.borrow();
            let Some(summary) = summaries.get(index as usize) else {
                return;
            };

            *selected_product_id.borrow_mut() = Some(summary.product_id);
            window.set_detail_name(display_name(&summary.name).into());
            window.set_detail_product_id_hex(format!("0x{:04x}", summary.product_id).into());
            window.set_detail_supported(summary.supported);
            window.set_detail_supports_dpi(summary.supports_dpi);
            window.set_detail_has_battery(summary.battery.is_some());
            window.set_detail_battery_percent(summary.battery.map(|b| b.level).unwrap_or(0) as i32);
            window.set_detail_charging(summary.battery.map(|b| b.is_charging).unwrap_or(false));
            window.set_detail_status(summary.note.clone().unwrap_or_default().into());
            window.set_detail_status_color(STATUS_COLOR_ERROR);
            window.set_dpi_input("400,800,1600".into());
            window.set_showing_detail(true);
        });
    }

    {
        let window = window.as_weak();
        window.unwrap().on_back_to_grid(move || {
            window.unwrap().set_showing_detail(false);
        });
    }

    {
        let window = window.as_weak();
        let selected_product_id = selected_product_id.clone();
        window.unwrap().on_apply_dpi(move |values| {
            let window = window.unwrap();
            let Some(product_id) = *selected_product_id.borrow() else {
                return;
            };

            match apply_dpi(product_id, &values) {
                Ok(()) => {
                    window.set_detail_status(format!("DPI set to {}", values).into());
                    window.set_detail_status_color(STATUS_COLOR_OK);
                }
                Err(e) => {
                    window.set_detail_status(e.to_string().into());
                    window.set_detail_status_color(STATUS_COLOR_ERROR);
                }
            }
        });
    }

    {
        let window = window.as_weak();
        window.unwrap().window().on_close_requested(move || {
            window.unwrap().hide().unwrap();
            slint::CloseRequestResponse::HideWindow
        });
    }

    {
        let window = window.as_weak();
        tray.on_show_window(move || {
            window.unwrap().show().unwrap();
        });
    }
    tray.on_quit(|| {
        slint::quit_event_loop().unwrap();
    });

    {
        let window = window.as_weak();
        let tray = tray.as_weak();
        let summaries = summaries.clone();
        let low_battery_notified = low_battery_notified.clone();
        let interval_secs = window.unwrap().get_poll_interval_minutes() as u64 * 60;
        poll_timer.start(slint::TimerMode::Repeated, Duration::from_secs(interval_secs), move || {
            let window = window.unwrap();
            let tray = tray.unwrap();
            let discovered = discover();
            window.set_devices(to_card_model(&discovered));
            tray.set_device_labels(device_labels(&discovered));
            check_low_battery(&discovered, &mut low_battery_notified.borrow_mut());
            *summaries.borrow_mut() = discovered;
        });
    }

    window.invoke_refresh();
    window.show()?;
    tray.show()?;
    slint::run_event_loop()?;
    Ok(())
}

/// The whole app is SteelSeries-only, so the brand prefix on device names
/// (e.g. `SteelSeriesDevice::name()`) is redundant here — strip it for
/// display. The CLI keeps the full name as-is.
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
            Some(format!("{} — {}%", display_name(&d.name), battery.level))
        })
        .collect()
}

fn device_labels(devices: &[DeviceSummary]) -> slint::ModelRc<slint::SharedString> {
    let labels: Vec<slint::SharedString> =
        format_device_labels(devices).into_iter().map(Into::into).collect();
    slint::ModelRc::from(labels.as_slice())
}

const LOW_BATTERY_THRESHOLD: u8 = 25;

/// Fires a low-battery notification the first time a device's level drops
/// below [`LOW_BATTERY_THRESHOLD`], and allows it to fire again only after
/// the device recovers back above the threshold and drops below it again.
fn check_low_battery(devices: &[DeviceSummary], notified: &mut std::collections::HashSet<u16>) {
    for d in devices {
        let Some(battery) = d.battery else { continue };
        let already_notified = notified.contains(&d.product_id);
        if battery.level < LOW_BATTERY_THRESHOLD && !already_notified {
            notified.insert(d.product_id);
            let _ = notify_rust::Notification::new()
                .summary("Low battery")
                .body(&format!("{} is at {}%", display_name(&d.name), battery.level))
                .urgency(notify_rust::Urgency::Critical)
                .show();
        } else if battery.level >= LOW_BATTERY_THRESHOLD && already_notified {
            notified.remove(&d.product_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ctrlgear_core::protocol::BatteryStatus;

    fn device(name: &str, battery: Option<BatteryStatus>) -> DeviceSummary {
        DeviceSummary {
            product_id: 0,
            name: name.to_string(),
            supported: true,
            supports_dpi: false,
            battery,
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
            vec!["Rival 3 Wireless — 80%", "Arctis Nova 7 — 15%"]
        );
    }

    fn device_with_id(product_id: u16, battery: Option<BatteryStatus>) -> DeviceSummary {
        DeviceSummary {
            product_id,
            name: "Test Device".to_string(),
            supported: true,
            supports_dpi: false,
            battery,
            note: None,
        }
    }

    #[test]
    fn check_low_battery_inserts_on_first_drop_below_threshold() {
        let devices = vec![device_with_id(1, Some(BatteryStatus { level: 20, is_charging: false }))];
        let mut notified = std::collections::HashSet::new();

        check_low_battery(&devices, &mut notified);

        assert!(notified.contains(&1));
    }

    #[test]
    fn check_low_battery_does_not_double_insert_while_still_low() {
        let devices = vec![device_with_id(1, Some(BatteryStatus { level: 20, is_charging: false }))];
        let mut notified = std::collections::HashSet::new();

        check_low_battery(&devices, &mut notified);
        check_low_battery(&devices, &mut notified);

        assert_eq!(notified.len(), 1);
        assert!(notified.contains(&1));
    }

    #[test]
    fn check_low_battery_clears_on_recovery_above_threshold() {
        let mut notified = std::collections::HashSet::new();
        notified.insert(1u16);
        let devices = vec![device_with_id(1, Some(BatteryStatus { level: 30, is_charging: false }))];

        check_low_battery(&devices, &mut notified);

        assert!(!notified.contains(&1));
    }

    #[test]
    fn check_low_battery_skips_devices_without_battery() {
        let devices = vec![device_with_id(1, None)];
        let mut notified = std::collections::HashSet::new();

        check_low_battery(&devices, &mut notified);

        assert!(notified.is_empty());
    }
}

fn discover() -> Vec<DeviceSummary> {
    match hidapi::HidApi::new() {
        Ok(api) => registry::discover(&api),
        Err(_) => Vec::new(),
    }
}

fn to_card_model(devices: &[DeviceSummary]) -> slint::ModelRc<DeviceCardData> {
    let cards: Vec<DeviceCardData> = devices
        .iter()
        .map(|d| {
            let (accent_start, accent_end) = accent_colors(d.product_id, d.supported);
            let name = display_name(&d.name);
            DeviceCardData {
                name: name.into(),
                initial: name
                    .chars()
                    .next()
                    .map(|c| c.to_uppercase().to_string())
                    .unwrap_or_default()
                    .into(),
                product_id_hex: format!("0x{:04x}", d.product_id).into(),
                supported: d.supported,
                has_battery: d.battery.is_some(),
                battery_percent: d.battery.map(|b| b.level).unwrap_or(0) as i32,
                charging: d.battery.map(|b| b.is_charging).unwrap_or(false),
                status: d.note.clone().unwrap_or_default().into(),
                accent_start,
                accent_end,
            }
        })
        .collect();
    slint::ModelRc::from(cards.as_slice())
}

/// Per-device row gradient, loosely echoing SteelSeries GG's own device list
/// (each product gets a distinct brand-colored tile). Unrecognized/unsupported
/// devices get a neutral gray.
fn accent_colors(product_id: u16, supported: bool) -> (slint::Color, slint::Color) {
    if !supported {
        return (
            slint::Color::from_rgb_u8(0x55, 0x58, 0x60),
            slint::Color::from_rgb_u8(0x33, 0x35, 0x3a),
        );
    }
    match product_id {
        RIVAL3_WIRELESS_PRODUCT_ID => (
            slint::Color::from_rgb_u8(0xff, 0x8a, 0x3d),
            slint::Color::from_rgb_u8(0xd8, 0x43, 0x15),
        ),
        ARCTIS_NOVA_7_PRODUCT_ID => (
            slint::Color::from_rgb_u8(0x4d, 0xd0, 0xe1),
            slint::Color::from_rgb_u8(0x1a, 0x4a, 0x6b),
        ),
        _ => (
            slint::Color::from_rgb_u8(0x60, 0x63, 0x6b),
            slint::Color::from_rgb_u8(0x38, 0x3a, 0x40),
        ),
    }
}

fn apply_dpi(product_id: u16, values: &str) -> ctrlgear_core::Result<()> {
    let dpi_values: Vec<u16> = values
        .split(',')
        .map(|s| {
            s.trim()
                .parse::<u16>()
                .map_err(|_| Error::InvalidParameter(format!("invalid DPI value: {}", s.trim())))
        })
        .collect::<ctrlgear_core::Result<_>>()?;

    let api = hidapi::HidApi::new()?;
    let device = registry::open(&api, product_id)?;
    device.set_dpi(&dpi_values)
}
