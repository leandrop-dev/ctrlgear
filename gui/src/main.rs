use std::cell::RefCell;
use std::rc::Rc;
use ctrlgear_core::device::arctis_nova_7::PRODUCT_ID as ARCTIS_NOVA_7_PRODUCT_ID;
use ctrlgear_core::device::rival3_wireless::PRODUCT_ID as RIVAL3_WIRELESS_PRODUCT_ID;
use ctrlgear_core::registry::{self, DeviceSummary};
use ctrlgear_core::Error;

slint::include_modules!();

/// Colors used for detail-page status text; kept in sync with the `#4caf50`
/// / `#e05050` accents used directly in main.slint for battery/card state.
const STATUS_COLOR_OK: slint::Color = slint::Color::from_rgb_u8(0x4c, 0xaf, 0x50);
const STATUS_COLOR_ERROR: slint::Color = slint::Color::from_rgb_u8(0xe0, 0x50, 0x50);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let window = MainWindow::new()?;

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

    {
        let window = window.as_weak();
        let summaries = summaries.clone();
        window.unwrap().on_refresh(move || {
            let window = window.unwrap();
            window.set_loading(true);
            let discovered = discover();
            window.set_devices(to_card_model(&discovered));
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

    window.invoke_refresh();
    window.run()?;
    Ok(())
}

/// The whole app is SteelSeries-only, so the brand prefix on device names
/// (e.g. `SteelSeriesDevice::name()`) is redundant here — strip it for
/// display. The CLI keeps the full name as-is.
fn display_name(name: &str) -> &str {
    name.strip_prefix("SteelSeries ").unwrap_or(name)
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
