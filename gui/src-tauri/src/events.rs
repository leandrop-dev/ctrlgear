//! Typed events pushed from Rust to the frontend.

use crate::chatmix_sinks::ChatMixSinksStatus;
use ctrlgear_core::registry::DeviceSummary;

/// Fresh device list, pushed after every poll-loop or manually triggered discover.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type, tauri_specta::Event)]
#[serde(rename_all = "camelCase")]
#[tauri_specta(event_name = "device-list-updated")]
pub struct DeviceListUpdated(pub Vec<DeviceSummary>);

/// Pushed whenever the ChatMix worker's status changes.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type, tauri_specta::Event)]
#[serde(rename_all = "camelCase")]
#[tauri_specta(event_name = "chatmix-sinks-status-changed")]
pub struct ChatMixSinksStatusChanged(pub ChatMixSinksStatus);
