//! Typed events pushed from Rust to the frontend.

use ctrlgear_core::registry::DeviceSummary;

/// Fresh device list, pushed after every poll-loop or manually triggered discover.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type, tauri_specta::Event)]
#[serde(rename_all = "camelCase")]
#[tauri_specta(event_name = "device-list-updated")]
pub struct DeviceListUpdated(pub Vec<DeviceSummary>);
