//! ChatMix-sinks settings commands: read the worker's live status and toggle
//! it on/off, persisting the choice like the other `settings_commands`.

use crate::chatmix_sinks::{ChatMixSinksStatus, WorkerCommand};
use crate::config::{self, Settings};
use crate::settings_commands::save_with_then;
use crate::{ChatMixState, SettingsState};
use std::sync::mpsc;
use tauri::State;

/// Reads the worker's last-known status. Mutex read only — never touches HID
/// or PipeWire directly.
#[tauri::command]
#[specta::specta]
pub fn get_chatmix_sinks_status(state: State<'_, ChatMixState>) -> ChatMixSinksStatus {
    state.status.lock().unwrap_or_else(|e| e.into_inner()).clone()
}

/// Persists `enabled`, then tells the worker to switch while still holding the
/// save lock, so overlapping toggles reach the worker in persisted order.
async fn set_chatmix_sinks_enabled_with(
    settings_state: &SettingsState,
    persist: impl FnOnce(&Settings) -> std::io::Result<()> + Send + 'static,
    tx: &mpsc::Sender<WorkerCommand>,
    enabled: bool,
) -> Result<Settings, String> {
    let command = if enabled { WorkerCommand::Enable } else { WorkerCommand::Disable };
    save_with_then(settings_state, persist, move |s| s.chatmix_sinks_enabled = enabled, |_| {
        let _ = tx.send(command);
    })
    .await
}

/// Toggles the ChatMix-sinks worker on/off and persists the choice. Mirrors
/// the `save_*` commands in `settings_commands.rs`.
#[tauri::command]
#[specta::specta]
pub async fn set_chatmix_sinks_enabled(
    settings_state: State<'_, SettingsState>,
    chatmix_state: State<'_, ChatMixState>,
    enabled: bool,
) -> Result<Settings, String> {
    set_chatmix_sinks_enabled_with(settings_state.inner(), config::save, &chatmix_state.tx, enabled).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlapping_toggles_reach_worker_in_persisted_order() {
        let dir = std::env::temp_dir().join("ctrlgear-test-chatmix-commands-overlap");
        let path = dir.join("config.toml");
        let _ = std::fs::remove_file(&path);
        let state = std::sync::Arc::new(SettingsState::new(Settings::default()));
        let (tx, rx) = mpsc::channel();

        let (slow_state, slow_path, slow_tx) = (state.clone(), path.clone(), tx.clone());
        let slow = tauri::async_runtime::spawn(async move {
            let persist = move |s: &Settings| {
                std::thread::sleep(std::time::Duration::from_millis(100));
                config::save_to(Some(slow_path), s)
            };
            set_chatmix_sinks_enabled_with(&slow_state, persist, &slow_tx, true).await
        });
        std::thread::sleep(std::time::Duration::from_millis(20));
        let (fast_state, fast_path) = (state.clone(), path.clone());
        let fast = tauri::async_runtime::spawn(async move {
            let persist = move |s: &Settings| config::save_to(Some(fast_path), s);
            set_chatmix_sinks_enabled_with(&fast_state, persist, &tx, false).await
        });
        tauri::async_runtime::block_on(async {
            slow.await.unwrap().unwrap();
            fast.await.unwrap().unwrap();
        });

        let received: Vec<WorkerCommand> = rx.try_iter().collect();
        let on_disk = config::load_from(Some(path.clone()));
        let _ = std::fs::remove_file(&path);
        assert!(matches!(received.as_slice(), [WorkerCommand::Enable, WorkerCommand::Disable]));
        assert!(!on_disk.chatmix_sinks_enabled);
        assert!(!state.current.lock().unwrap().chatmix_sinks_enabled);
    }
}
