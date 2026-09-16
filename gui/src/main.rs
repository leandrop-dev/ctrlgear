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

/// Shows the result of a `config::save` call as the Settings-page toast,
/// auto-clearing it after 5 seconds.
fn show_save_status(window: &MainWindow, result: std::io::Result<()>) {
    match result {
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
}

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
    let battery_state: Rc<RefCell<std::collections::HashMap<u16, BatteryState>>> =
        Rc::new(RefCell::new(std::collections::HashMap::new()));

    let settings_cell: Rc<RefCell<config::Settings>> = Rc::new(RefCell::new(config::load()));
    {
        let settings = settings_cell.borrow();
        window.set_poll_interval_minutes(settings.poll_interval_minutes as i32);
        window.set_tray_icon_monochrome(matches!(settings.tray_icon_style, config::TrayIconStyle::Monochrome));
        tray.set_monochrome(matches!(settings.tray_icon_style, config::TrayIconStyle::Monochrome));
        window.set_notification_persistent(settings.notification_persistent);
        window.set_notification_sound_index(sound_choice_index(settings.notification_sound));
    }

    let poll_timer = Rc::new(slint::Timer::default());

    {
        let window = window.as_weak();
        let poll_timer = poll_timer.clone();
        let settings_cell = settings_cell.clone();
        window.unwrap().on_save_poll_interval(move |minutes| {
            let window = window.unwrap();
            let settings = {
                let mut settings = *settings_cell.borrow();
                settings.poll_interval_minutes = minutes.max(1) as u32;
                *settings_cell.borrow_mut() = settings;
                settings
            };
            window.set_poll_interval_minutes(settings.poll_interval_minutes as i32);
            poll_timer.set_interval(Duration::from_secs(settings.poll_interval_minutes as u64 * 60));
            show_save_status(&window, config::save(&settings));
        });
    }

    {
        let window = window.as_weak();
        let tray = tray.as_weak();
        let settings_cell = settings_cell.clone();
        window.unwrap().on_save_tray_icon_style(move |monochrome| {
            let window = window.unwrap();
            let tray = tray.unwrap();
            let style =
                if monochrome { config::TrayIconStyle::Monochrome } else { config::TrayIconStyle::Colored };
            let settings = {
                let mut settings = *settings_cell.borrow();
                settings.tray_icon_style = style;
                *settings_cell.borrow_mut() = settings;
                settings
            };
            window.set_tray_icon_monochrome(monochrome);
            tray.set_monochrome(monochrome);
            show_save_status(&window, config::save(&settings));
        });
    }

    {
        let window = window.as_weak();
        let settings_cell = settings_cell.clone();
        window.unwrap().on_save_notification_persistent(move |persistent| {
            let window = window.unwrap();
            let settings = {
                let mut settings = *settings_cell.borrow();
                settings.notification_persistent = persistent;
                *settings_cell.borrow_mut() = settings;
                settings
            };
            window.set_notification_persistent(persistent);
            show_save_status(&window, config::save(&settings));
        });
    }

    {
        let window = window.as_weak();
        let settings_cell = settings_cell.clone();
        window.unwrap().on_save_notification_sound(move |index| {
            let window = window.unwrap();
            let sound = sound_choice_from_index(index);
            let settings = {
                let mut settings = *settings_cell.borrow();
                settings.notification_sound = sound;
                *settings_cell.borrow_mut() = settings;
                settings
            };
            window.set_notification_sound_index(sound_choice_index(sound));
            show_save_status(&window, config::save(&settings));
        });
    }

    window.on_preview_sound(preview_notification_sound);

    {
        let window = window.as_weak();
        let tray = tray.as_weak();
        let summaries = summaries.clone();
        let battery_state = battery_state.clone();
        let settings_cell = settings_cell.clone();
        let selected_product_id = selected_product_id.clone();
        window.unwrap().on_refresh(move || {
            let window = window.unwrap();
            window.set_loading(true);
            let discovered = discover();
            window.set_devices(to_card_model(&discovered));
            tray.unwrap().set_device_labels(device_labels(&discovered));
            check_low_battery(&discovered, &mut battery_state.borrow_mut(), &settings_cell.borrow());
            refresh_detail_if_showing(&window, *selected_product_id.borrow(), &discovered);
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
            apply_detail_summary(&window, summary);
            window.set_dpi_input("400,800,1600".into());
            window.set_polling_rate_index(3);
            window.set_sidetone_index(0);
            window.set_mic_mute_led_index(0);
            window.set_mic_volume_index(0);
            window.set_button_mapping_index_0(4);
            window.set_button_mapping_index_1(4);
            window.set_button_mapping_index_2(4);
            window.set_button_mapping_index_3(4);
            window.set_button_mapping_index_4(4);
            window.set_button_mapping_index_5(4);
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
        let selected_product_id = selected_product_id.clone();
        window.unwrap().on_apply_polling_rate(move |index| {
            let window = window.unwrap();
            let Some(product_id) = *selected_product_id.borrow() else {
                return;
            };

            match apply_polling_rate(product_id, index) {
                Ok(()) => {
                    window.set_detail_status("Polling rate applied.".into());
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
        let selected_product_id = selected_product_id.clone();
        window.unwrap().on_apply_sidetone(move |index| {
            let window = window.unwrap();
            let Some(product_id) = *selected_product_id.borrow() else {
                return;
            };

            match apply_sidetone(product_id, index) {
                Ok(()) => {
                    window.set_detail_status("Sidetone applied.".into());
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
        let selected_product_id = selected_product_id.clone();
        window.unwrap().on_apply_mic_mute_led(move |index| {
            let window = window.unwrap();
            let Some(product_id) = *selected_product_id.borrow() else {
                return;
            };

            match apply_mic_mute_led_brightness(product_id, index) {
                Ok(()) => {
                    window.set_detail_status("Mic mute LED brightness applied.".into());
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
        let selected_product_id = selected_product_id.clone();
        window.unwrap().on_apply_mic_volume(move |index| {
            let window = window.unwrap();
            let Some(product_id) = *selected_product_id.borrow() else {
                return;
            };

            match apply_mic_volume(product_id, index) {
                Ok(()) => {
                    window.set_detail_status("Mic volume applied.".into());
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
        let selected_product_id = selected_product_id.clone();
        window.unwrap().on_apply_button_mapping(move |indices| {
            let window = window.unwrap();
            let Some(product_id) = *selected_product_id.borrow() else {
                return;
            };

            match apply_button_mapping(product_id, &indices) {
                Ok(()) => {
                    window.set_detail_status("Button mapping applied.".into());
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
            let window = window.unwrap();
            window.show().unwrap();
            window.invoke_refresh();
        });
    }
    tray.on_quit(|| {
        slint::quit_event_loop().unwrap();
    });

    {
        let window = window.as_weak();
        let tray = tray.as_weak();
        let summaries = summaries.clone();
        let battery_state = battery_state.clone();
        let settings_cell = settings_cell.clone();
        let selected_product_id = selected_product_id.clone();
        let interval_secs = window.unwrap().get_poll_interval_minutes() as u64 * 60;
        poll_timer.start(slint::TimerMode::Repeated, Duration::from_secs(interval_secs), move || {
            let window = window.unwrap();
            let tray = tray.unwrap();
            let discovered = discover();
            window.set_devices(to_card_model(&discovered));
            tray.set_device_labels(device_labels(&discovered));
            check_low_battery(&discovered, &mut battery_state.borrow_mut(), &settings_cell.borrow());
            refresh_detail_if_showing(&window, *selected_product_id.borrow(), &discovered);
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

/// Short label shown next to a device's name in the list when it's reachable
/// over more than one transport (e.g. a wireless dongle plus a USB cable
/// plugged in to charge), since both legitimately produce separate rows.
fn connection_label(kind: registry::ConnectionType) -> &'static str {
    match kind {
        registry::ConnectionType::UsbWired => "USB",
        registry::ConnectionType::Dongle24Ghz => "2.4GHz",
        registry::ConnectionType::Bluetooth => "Bluetooth",
        registry::ConnectionType::Unknown => "",
    }
}

/// Formats a device's valid DPI range as help text for the detail page.
/// The "up to 5 presets" figure is hardcoded because it's currently enforced
/// only by `RIVAL3_WIRELESS_PROFILE`'s `DpiProfile::max_presets` (checked in
/// `ProfiledDevice::set_dpi`) and not exposed through `ConfigurableDpi` —
/// acceptable since Rival 3 Wireless is the only DPI-capable device today;
/// revisit if a device with a different limit is added.
fn dpi_help_text(range: Option<(u16, u16, u16)>) -> String {
    match range {
        Some((min, max, step)) => {
            format!("Supports {min}\u{2013}{max} DPI in steps of {step}, up to 5 presets.")
        }
        None => String::new(),
    }
}

/// Applies a device summary to the detail page's fields. The single place
/// that knows which window properties constitute "the detail view" —
/// called from device selection AND from both refresh paths (manual and
/// periodic), so a field added to one path can't silently go missing from
/// the others (this already happened once, with the chatmix fields).
fn apply_detail_summary(window: &MainWindow, summary: &DeviceSummary) {
    window.set_detail_name(display_name(&summary.name).into());
    window.set_detail_product_id_hex(format!("0x{:04x}", summary.product_id).into());
    window.set_detail_supported(summary.supported);
    window.set_detail_supports_dpi(summary.supports_dpi);
    window.set_detail_supports_polling_rate(summary.supports_polling_rate);
    window.set_detail_supports_button_mapping(summary.supports_button_mapping);
    window.set_detail_supports_sidetone(summary.supports_sidetone);
    window.set_detail_supports_mic_mute_led(summary.supports_mic_mute_led);
    window.set_detail_supports_mic_volume(summary.supports_mic_volume);
    window.set_detail_dpi_help(dpi_help_text(summary.dpi_range).into());
    window.set_detail_has_battery(summary.battery.is_some());
    window.set_detail_battery_percent(summary.battery.map(|b| b.level).unwrap_or(0) as i32);
    window.set_detail_charging(summary.battery.map(|b| b.is_charging).unwrap_or(false));
    window.set_detail_has_chatmix(summary.chatmix.is_some());
    window.set_detail_chatmix_game_percent(summary.chatmix.map(|c| c.game_percent).unwrap_or(0) as i32);
    window.set_detail_chatmix_chat_percent(summary.chatmix.map(|c| c.chat_percent).unwrap_or(0) as i32);
    window.set_detail_status(summary.note.clone().unwrap_or_default().into());
    window.set_detail_status_color(STATUS_COLOR_ERROR);
}

/// Finds the current summary for the selected device in a fresh discover()
/// result, or `None` if nothing is selected or the device dropped out of
/// this discovery (e.g. unplugged) — callers must leave the detail page
/// untouched in that case, not blank it.
fn find_selected_summary(selected: Option<u16>, discovered: &[DeviceSummary]) -> Option<DeviceSummary> {
    let product_id = selected?;
    discovered.iter().find(|d| d.product_id == product_id).cloned()
}

/// If the detail page is open, re-applies the selected device's current
/// summary to it. Called from both refresh paths (manual and periodic) so
/// they can't silently drift on when/how the detail page gets refreshed.
fn refresh_detail_if_showing(window: &MainWindow, selected: Option<u16>, discovered: &[DeviceSummary]) {
    if window.get_showing_detail() {
        if let Some(summary) = find_selected_summary(selected, discovered) {
            apply_detail_summary(window, &summary);
        }
    }
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

fn device_labels(devices: &[DeviceSummary]) -> slint::ModelRc<slint::SharedString> {
    let labels: Vec<slint::SharedString> =
        format_device_labels(devices).into_iter().map(Into::into).collect();
    slint::ModelRc::from(labels.as_slice())
}

const LOW_BATTERY_THRESHOLD: u8 = 25;

/// Maps a `config::SoundChoice` to its `ComboBox` index in the Settings page
/// (`gui/ui/main.slint`'s `notification-sound` `ComboBox` model order: Default
/// sound, Silent, Dialog warning, Message).
fn sound_choice_index(sound: config::SoundChoice) -> i32 {
    match sound {
        config::SoundChoice::Default => 0,
        config::SoundChoice::Silent => 1,
        config::SoundChoice::DialogWarning => 2,
        config::SoundChoice::MessageNewInstant => 3,
    }
}

fn sound_choice_from_index(index: i32) -> config::SoundChoice {
    match index {
        1 => config::SoundChoice::Silent,
        2 => config::SoundChoice::DialogWarning,
        3 => config::SoundChoice::MessageNewInstant,
        _ => config::SoundChoice::Default,
    }
}

/// Fires a short-lived preview notification for the given sound choice, so
/// the user can hear it before applying. Unlike a real low-battery alert,
/// this always auto-dismisses and never uses `Urgency::Critical`.
fn preview_notification_sound(index: i32) {
    let mut notification = notify_rust::Notification::new();
    notification
        .summary("CtrlGear")
        .body("Notification sound preview")
        .timeout(notify_rust::Timeout::Milliseconds(3000));
    match sound_choice_from_index(index) {
        config::SoundChoice::Silent => {
            notification.hint(notify_rust::Hint::SuppressSound(true));
        }
        config::SoundChoice::Default => {}
        config::SoundChoice::DialogWarning => {
            notification.sound_name("dialog-warning");
        }
        config::SoundChoice::MessageNewInstant => {
            notification.sound_name("message-new-instant");
        }
    }
    let _ = notification.show();
}

struct BatteryState {
    notified: bool,
    last_level: u8,
}

const BATTERY_SWAP_JUMP_THRESHOLD: u8 = 15;

/// Determines which devices should fire a low-battery notification this
/// poll, updating `states` accordingly. Pure state-tracking logic with no
/// I/O, so it's safe to call from tests without side effects — the actual
/// `notify-rust` call lives in [`check_low_battery`].
///
/// Fires the first time a device's level drops below
/// [`LOW_BATTERY_THRESHOLD`], and allows it to fire again only after the
/// device recovers back above the threshold and drops below it again.
///
/// A level jump of at least [`BATTERY_SWAP_JUMP_THRESHOLD`] percentage points
/// since the last poll, while a low-battery episode is active, is treated as
/// a battery swap: it clears the episode without notifying on the jump
/// itself, so a later re-drop below the threshold notifies again.
fn low_battery_transitions(
    devices: &[DeviceSummary],
    states: &mut std::collections::HashMap<u16, BatteryState>,
) -> Vec<(String, u8)> {
    let mut transitions = Vec::new();

    for d in devices {
        let Some(battery) = d.battery else { continue };
        let state = states
            .entry(d.product_id)
            .or_insert(BatteryState { notified: false, last_level: battery.level });
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

/// Fires a desktop notification for each low-battery transition. Timeout and
/// sound follow `settings.notification_persistent`/`notification_sound`.
fn check_low_battery(
    devices: &[DeviceSummary],
    states: &mut std::collections::HashMap<u16, BatteryState>,
    settings: &config::Settings,
) {
    for (name, level) in low_battery_transitions(devices, states) {
        let mut notification = notify_rust::Notification::new();
        notification
            .summary("Low battery")
            .body(&format!("{} is at {}%", name, level))
            .urgency(notify_rust::Urgency::Critical)
            .timeout(if settings.notification_persistent {
                notify_rust::Timeout::Never
            } else {
                notify_rust::Timeout::Default
            });
        match settings.notification_sound {
            config::SoundChoice::Silent => {
                notification.hint(notify_rust::Hint::SuppressSound(true));
            }
            config::SoundChoice::Default => {}
            config::SoundChoice::DialogWarning => {
                notification.sound_name("dialog-warning");
            }
            config::SoundChoice::MessageNewInstant => {
                notification.sound_name("message-new-instant");
            }
        }
        let _ = notification.show();
    }
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
            dpi_range: None,
            connection_type: ConnectionType::Unknown,
            battery,
            chatmix: None,
            note: None,
        }
    }

    #[test]
    fn dpi_help_text_formats_the_range() {
        assert_eq!(
            dpi_help_text(Some((100, 18000, 100))),
            "Supports 100\u{2013}18000 DPI in steps of 100, up to 5 presets."
        );
    }

    #[test]
    fn dpi_help_text_empty_when_no_range() {
        assert_eq!(dpi_help_text(None), "");
    }

    #[test]
    fn connection_label_maps_each_connection_type() {
        assert_eq!(connection_label(ConnectionType::UsbWired), "USB");
        assert_eq!(connection_label(ConnectionType::Dongle24Ghz), "2.4GHz");
        assert_eq!(connection_label(ConnectionType::Bluetooth), "Bluetooth");
        assert_eq!(connection_label(ConnectionType::Unknown), "");
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
            dpi_range: None,
            connection_type: ConnectionType::Unknown,
            battery,
            chatmix: None,
            note: None,
        }
    }

    #[test]
    fn check_low_battery_inserts_on_first_drop_below_threshold() {
        let devices = vec![device_with_id(1, Some(BatteryStatus { level: 20, is_charging: false }))];
        let mut states = std::collections::HashMap::new();

        let transitions = low_battery_transitions(&devices, &mut states);

        assert!(states.get(&1).unwrap().notified);
        assert_eq!(transitions, vec![("Test Device".to_string(), 20)]);
    }

    #[test]
    fn check_low_battery_does_not_double_insert_while_still_low() {
        let devices = vec![device_with_id(1, Some(BatteryStatus { level: 20, is_charging: false }))];
        let mut states = std::collections::HashMap::new();

        low_battery_transitions(&devices, &mut states);
        let second_transitions = low_battery_transitions(&devices, &mut states);

        assert_eq!(states.len(), 1);
        assert!(states.get(&1).unwrap().notified);
        assert!(second_transitions.is_empty());
    }

    #[test]
    fn check_low_battery_clears_on_recovery_above_threshold() {
        let mut states = std::collections::HashMap::new();
        states.insert(1u16, BatteryState { notified: true, last_level: 20 });
        let devices = vec![device_with_id(1, Some(BatteryStatus { level: 30, is_charging: false }))];

        let transitions = low_battery_transitions(&devices, &mut states);

        assert!(!states.get(&1).unwrap().notified);
        assert!(transitions.is_empty());
    }

    #[test]
    fn check_low_battery_skips_devices_without_battery() {
        let devices = vec![device_with_id(1, None)];
        let mut states = std::collections::HashMap::new();

        let transitions = low_battery_transitions(&devices, &mut states);

        assert!(states.is_empty());
        assert!(transitions.is_empty());
    }

    #[test]
    fn check_low_battery_swap_resets_episode_without_notifying() {
        let mut states = std::collections::HashMap::new();
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

    #[test]
    fn find_selected_summary_returns_matching_summary() {
        let discovered = vec![device_with_id(1, Some(BatteryStatus { level: 42, is_charging: false }))];
        let result = find_selected_summary(Some(1), &discovered);
        assert_eq!(result.map(|d| d.product_id), Some(1));
    }

    #[test]
    fn find_selected_summary_returns_none_when_nothing_selected() {
        let discovered = vec![device_with_id(1, None)];
        assert!(find_selected_summary(None, &discovered).is_none());
    }

    #[test]
    fn find_selected_summary_returns_none_when_selected_device_not_present() {
        let discovered = vec![device_with_id(2, None)];
        assert!(find_selected_summary(Some(1), &discovered).is_none());
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
                connection_label: connection_label(d.connection_type).into(),
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

fn apply_polling_rate(product_id: u16, index: i32) -> ctrlgear_core::Result<()> {
    let hz = match index {
        0 => 125,
        1 => 250,
        2 => 500,
        _ => 1000,
    };
    let api = hidapi::HidApi::new()?;
    let device = registry::open(&api, product_id)?;
    device.set_polling_rate(hz)
}

fn apply_sidetone(product_id: u16, index: i32) -> ctrlgear_core::Result<()> {
    let level = index.clamp(0, 3) as u8;
    let api = hidapi::HidApi::new()?;
    let device = registry::open(&api, product_id)?;
    device.set_sidetone(level)
}

fn apply_mic_mute_led_brightness(product_id: u16, index: i32) -> ctrlgear_core::Result<()> {
    let level = index.clamp(0, 3) as u8;
    let api = hidapi::HidApi::new()?;
    let device = registry::open(&api, product_id)?;
    device.set_mic_mute_led_brightness(level)
}

fn apply_mic_volume(product_id: u16, index: i32) -> ctrlgear_core::Result<()> {
    let level = index.clamp(0, 7) as u8;
    let api = hidapi::HidApi::new()?;
    let device = registry::open(&api, product_id)?;
    device.set_mic_volume(level)
}

/// Keyword order matches the `ComboBox` `model` in `gui/ui/main.slint`'s
/// button-mapping section, and is understood by `ButtonAction::from_str`
/// (`core/src/buttons.rs`), reused here instead of a second parser.
const BUTTON_ACTION_KEYWORDS: [&str; 11] = [
    "disabled", "dpiswitch", "scrollup", "scrolldown", "default", "button1", "button2", "button3",
    "button4", "button5", "button6",
];

fn apply_button_mapping(product_id: u16, indices: &slint::ModelRc<i32>) -> ctrlgear_core::Result<()> {
    use ctrlgear_core::buttons::{Button, ButtonAction, ButtonMapping};
    use slint::Model;
    use std::str::FromStr;

    let mut mapping = ButtonMapping::default_mapping();
    for (i, button) in Button::all().into_iter().enumerate() {
        let index = indices.row_data(i).unwrap_or(4) as usize;
        let keyword = BUTTON_ACTION_KEYWORDS.get(index).copied().unwrap_or("default");
        let action = ButtonAction::from_str(keyword)?;
        mapping.set(button, action);
    }

    let api = hidapi::HidApi::new()?;
    let device = registry::open(&api, product_id)?;
    device.set_button_mapping(&mapping)
}
