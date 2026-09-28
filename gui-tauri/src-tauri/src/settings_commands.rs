//! Settings-page commands: read/persist `Settings` fields, reconfigure the
//! live poll loop, and preview a notification sound. Mirrors the
//! Settings-page callbacks in `gui/src/main.rs`.

use crate::config::{self, Settings, SoundChoice, TrayIconStyle};
use crate::notifications;
use crate::{PollIntervalHandle, SettingsState};
use tauri::State;

/// Mutates a copy of the in-memory settings with `mutate`, persists that copy
/// on a blocking task via `persist` (production callers pass [`config::save`];
/// tests inject a temp-path save so they never touch the real config file),
/// then commits it to `state`. Each `state.current` lock is confined to its
/// own statement; `state.save_lock` is held across the persist so overlapping
/// saves serialize. Shared by every `save_*` command below.
pub(crate) async fn save_with(
    state: &SettingsState,
    persist: impl FnOnce(&Settings) -> std::io::Result<()> + Send + 'static,
    mutate: impl FnOnce(&mut Settings),
) -> Result<Settings, String> {
    save_with_then(state, persist, mutate, |_| {}).await
}

/// [`save_with`], then runs `on_saved` before releasing the save lock, so
/// side effects of overlapping saves apply in the same order they persisted.
pub(crate) async fn save_with_then(
    state: &SettingsState,
    persist: impl FnOnce(&Settings) -> std::io::Result<()> + Send + 'static,
    mutate: impl FnOnce(&mut Settings),
    on_saved: impl FnOnce(&Settings),
) -> Result<Settings, String> {
    let _save_guard = state.save_lock.lock().await;
    let mut settings = current_settings(state);
    mutate(&mut settings);

    let to_persist = settings;
    tauri::async_runtime::spawn_blocking(move || persist(&to_persist))
        .await
        .map_err(|e| format!("Settings save task panicked: {e}"))?
        .map_err(|e| e.to_string())?;

    *state.current.lock().unwrap_or_else(|e| e.into_inner()) = settings;
    on_saved(&settings);
    Ok(settings)
}

fn current_settings(state: &SettingsState) -> Settings {
    *state.current.lock().unwrap_or_else(|e| e.into_inner())
}

/// Returns the current in-memory settings, loaded once at startup and kept in
/// sync by every `save_*` command below.
#[tauri::command]
#[specta::specta]
pub fn get_settings(state: State<'_, SettingsState>) -> Settings {
    current_settings(state.inner())
}

/// Sets the battery-poll interval (clamped to 1-120 minutes, matching the
/// frontend's own bounds) and reconfigures the running poll loop. Mirrors
/// `gui/src/main.rs`'s `on_save_poll_interval`.
#[tauri::command]
#[specta::specta]
pub async fn save_poll_interval(
    state: State<'_, SettingsState>,
    poll_interval: State<'_, PollIntervalHandle>,
    minutes: u32,
) -> Result<Settings, String> {
    let minutes = minutes.clamp(1, 120);
    let settings = save_with(state.inner(), config::save, move |s| s.poll_interval_minutes = minutes).await?;
    poll_interval.set_minutes(minutes);
    Ok(settings)
}

/// Sets the tray icon's colored/monochrome style. Mirrors `gui/src/main.rs`'s
/// `on_save_tray_icon_style`.
#[tauri::command]
#[specta::specta]
pub async fn save_tray_icon_style(state: State<'_, SettingsState>, monochrome: bool) -> Result<Settings, String> {
    save_with(state.inner(), config::save, move |s| {
        s.tray_icon_style = if monochrome { TrayIconStyle::Monochrome } else { TrayIconStyle::Colored };
    })
    .await
}

/// Sets whether a low-battery notification stays until manually dismissed.
/// Mirrors `gui/src/main.rs`'s `on_save_notification_persistent`.
#[tauri::command]
#[specta::specta]
pub async fn save_notification_persistent(
    state: State<'_, SettingsState>,
    persistent: bool,
) -> Result<Settings, String> {
    save_with(state.inner(), config::save, move |s| s.notification_persistent = persistent).await
}

/// Sets the sound played by a low-battery notification. Mirrors
/// `gui/src/main.rs`'s `on_save_notification_sound`.
#[tauri::command]
#[specta::specta]
pub async fn save_notification_sound(state: State<'_, SettingsState>, sound: SoundChoice) -> Result<Settings, String> {
    save_with(state.inner(), config::save, move |s| s.notification_sound = sound).await
}

/// Fires a short-lived preview notification for `sound`, off the main thread
/// since `notify-rust`'s `show()` blocks on D-Bus. Mirrors `gui/src/main.rs`'s
/// `preview_notification_sound`.
#[tauri::command]
#[specta::specta]
pub async fn preview_notification_sound(sound: SoundChoice) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || notifications::preview_sound(sound))
        .await
        .map_err(|e| format!("Notification preview task panicked: {e}"))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_with_persists_to_disk_and_updates_state() {
        let dir = std::env::temp_dir().join("ctrlgear-test-settings-commands-round-trip");
        let path = dir.join("config.toml");
        let _ = std::fs::remove_file(&path);
        let state = SettingsState::new(Settings::default());

        let save_path = path.clone();
        let saved = tauri::async_runtime::block_on(save_with(
            &state,
            move |s| config::save_to(Some(save_path), s),
            |s| s.poll_interval_minutes = 42,
        ))
        .unwrap();

        assert_eq!(saved.poll_interval_minutes, 42);
        assert_eq!(state.current.lock().unwrap().poll_interval_minutes, 42);
        assert_eq!(config::load_from(Some(path.clone())).poll_interval_minutes, 42);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn on_saved_runs_while_save_lock_is_held() {
        let path = std::env::temp_dir().join("ctrlgear-test-settings-commands-lock-held").join("config.toml");
        let _ = std::fs::remove_file(&path);
        let state = SettingsState::new(Settings::default());
        let persist_path = path.clone();

        let lock_was_held = std::cell::Cell::new(false);
        tauri::async_runtime::block_on(save_with_then(
            &state,
            move |s| config::save_to(Some(persist_path), s),
            |s| s.poll_interval_minutes = 5,
            |_| lock_was_held.set(state.save_lock.try_lock().is_err()),
        ))
        .unwrap();
        let _ = std::fs::remove_file(&path);
        assert!(lock_was_held.get());
    }

    #[test]
    fn concurrent_saves_keep_both_changes() {
        let dir = std::env::temp_dir().join("ctrlgear-test-settings-commands-concurrent");
        let path = dir.join("config.toml");
        let _ = std::fs::remove_file(&path);
        let state = std::sync::Arc::new(SettingsState::new(Settings::default()));

        let (slow_state, slow_path) = (state.clone(), path.clone());
        let slow = tauri::async_runtime::spawn(async move {
            save_with(
                &slow_state,
                move |s| {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    config::save_to(Some(slow_path), s)
                },
                |s| s.notification_persistent = true,
            )
            .await
        });
        std::thread::sleep(std::time::Duration::from_millis(20));
        let (fast_state, fast_path) = (state.clone(), path.clone());
        let fast = tauri::async_runtime::spawn(async move {
            save_with(
                &fast_state,
                move |s| config::save_to(Some(fast_path), s),
                |s| s.notification_sound = SoundChoice::MessageNewInstant,
            )
            .await
        });
        tauri::async_runtime::block_on(async {
            slow.await.unwrap().unwrap();
            fast.await.unwrap().unwrap();
        });

        let in_memory = current_settings(&state);
        let on_disk = config::load_from(Some(path.clone()));
        let _ = std::fs::remove_file(&path);
        for settings in [in_memory, on_disk] {
            assert!(settings.notification_persistent);
            assert_eq!(settings.notification_sound, SoundChoice::MessageNewInstant);
        }
    }

    #[test]
    fn get_settings_reflects_the_managed_state() {
        let state = SettingsState::new(Settings {
            poll_interval_minutes: 7,
            ..Default::default()
        });

        assert_eq!(current_settings(&state).poll_interval_minutes, 7);
    }
}
