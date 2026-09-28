//! Tauri IPC commands, organized by feature area (device list today; later
//! tasks add sibling modules such as `dpi_commands`).

use ctrlgear_core::registry::{self, DeviceSummary};
use hidapi::{HidApi, HidError};
use std::sync::Mutex;

/// Serializes every HID transaction (discover or device write) against the
/// SteelSeries bus, so no two commands (or a command and the poll loop) ever
/// talk to the bus at once. Mirrors `HID_BUS_LOCK` in the Slint GUI's
/// `gui/src/main.rs`. Shared with [`crate::device_commands`].
pub(crate) static HID_BUS_LOCK: Mutex<()> = Mutex::new(());

/// Enumerates connected SteelSeries devices, opening a fresh [`HidApi`] and
/// serializing bus access via [`HID_BUS_LOCK`]. Blocking — callers must run
/// this off the async runtime (e.g. via `spawn_blocking`).
pub(crate) fn discover_devices() -> Result<Vec<DeviceSummary>, String> {
    let _guard = HID_BUS_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    discover_with(HidApi::new())
}

/// Runs [`discover_devices`] on the blocking pool, folding a panicked task
/// into the same error channel as a HID failure.
pub(crate) async fn discover_devices_async() -> Result<Vec<DeviceSummary>, String> {
    tauri::async_runtime::spawn_blocking(discover_devices)
        .await
        .unwrap_or_else(|e| Err(format!("Device discovery task panicked: {e}")))
}

/// Pure wrapper around [`registry::discover`] taking an already-attempted
/// [`HidApi`] open, so the HID-failure path is unit-testable without
/// hardware.
fn discover_with(api: Result<HidApi, HidError>) -> Result<Vec<DeviceSummary>, String> {
    api.map(|api| registry::discover(&api))
        .map_err(|e| format!("Failed to open HID API: {e}"))
}

/// Lists every connected SteelSeries device, supported or not, and refreshes
/// the tray menu's battery rows. Errors when the HID subsystem itself can't
/// be opened, so the UI can tell that apart from "no devices connected".
#[tauri::command]
#[specta::specta]
pub async fn list_devices(app: tauri::AppHandle) -> Result<Vec<DeviceSummary>, String> {
    let devices = discover_devices_async().await?;
    crate::tray::refresh_menu(&app, &devices);
    Ok(devices)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discover_with_hid_api_error_returns_err_without_panicking() {
        let err = discover_with(Err(HidError::InitializationError)).unwrap_err();
        assert!(err.starts_with("Failed to open HID API"));
    }
}
