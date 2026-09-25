mod commands;
mod device_commands;
mod events;

use events::DeviceListUpdated;
use tauri_specta::{collect_commands, collect_events, Event};

/// Interval between automatic device-list refreshes; task 07 wires this to
/// a user-configurable setting.
const POLL_INTERVAL: std::time::Duration = std::time::Duration::from_secs(5 * 60);

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
            spawn_poll_loop(app.handle().clone());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Spawns the background loop that periodically refreshes the device list
/// and pushes it to the frontend as a [`DeviceListUpdated`] event.
fn spawn_poll_loop(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(POLL_INTERVAL);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        interval.tick().await;
        loop {
            interval.tick().await;
            let devices = match commands::discover_devices_async().await {
                Ok(devices) => devices,
                Err(e) => {
                    eprintln!("Poll loop skipped a refresh: {e}");
                    continue;
                }
            };
            if let Err(e) = DeviceListUpdated(devices).emit(&app) {
                eprintln!("Failed to emit device-list-updated: {e}");
            }
        }
    });
}
