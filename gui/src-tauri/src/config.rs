//! Persisted user settings, loaded from and saved to `~/.config/ctrlgear/config.toml`.

#[derive(serde::Serialize, serde::Deserialize, specta::Type, Clone, Copy, PartialEq, Eq, Debug)]
pub enum TrayIconStyle {
    Colored,
    Monochrome,
}

impl Default for TrayIconStyle {
    fn default() -> Self {
        TrayIconStyle::Colored
    }
}

#[derive(serde::Serialize, serde::Deserialize, specta::Type, Clone, Copy, PartialEq, Eq, Debug)]
pub enum SoundChoice {
    Silent,
    Default,
    DialogWarning,
    MessageNewInstant,
}

impl Default for SoundChoice {
    fn default() -> Self {
        SoundChoice::Default
    }
}

#[derive(serde::Serialize, serde::Deserialize, specta::Type, Clone, Copy)]
pub struct Settings {
    pub poll_interval_minutes: u32,
    #[serde(default)]
    pub tray_icon_style: TrayIconStyle,
    #[serde(default)]
    pub notification_persistent: bool,
    #[serde(default)]
    pub notification_sound: SoundChoice,
    #[serde(default)]
    pub chatmix_sinks_enabled: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            poll_interval_minutes: 5,
            tray_icon_style: TrayIconStyle::default(),
            notification_persistent: false,
            notification_sound: SoundChoice::default(),
            chatmix_sinks_enabled: false,
        }
    }
}

fn config_path() -> Option<std::path::PathBuf> {
    dirs::config_dir().map(|d| d.join("ctrlgear").join("config.toml"))
}

pub fn load() -> Settings {
    load_from(config_path())
}

pub fn save(settings: &Settings) -> std::io::Result<()> {
    save_to(config_path(), settings)
}

pub(crate) fn load_from(path: Option<std::path::PathBuf>) -> Settings {
    path.and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| toml::from_str(&s).ok())
        .unwrap_or_default()
}

pub(crate) fn save_to(path: Option<std::path::PathBuf>, settings: &Settings) -> std::io::Result<()> {
    let Some(path) = path else {
        return Ok(());
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let toml = toml::to_string_pretty(settings).unwrap_or_default();
    std::fs::write(path, toml)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_falls_back_to_default_when_no_file_exists() {
        let dir = std::env::temp_dir().join("ctrlgear-test-config-missing");
        let path = dir.join("config.toml");
        let _ = std::fs::remove_file(&path);

        let settings = load_from(Some(path));

        assert_eq!(settings.poll_interval_minutes, 5);
    }

    #[test]
    fn save_then_load_round_trips() {
        let dir = std::env::temp_dir().join("ctrlgear-test-config-roundtrip");
        let path = dir.join("config.toml");

        save_to(Some(path.clone()), &Settings { poll_interval_minutes: 15, ..Default::default() }).unwrap();
        let settings = load_from(Some(path.clone()));

        assert_eq!(settings.poll_interval_minutes, 15);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn save_then_load_round_trips_tray_icon_style() {
        let dir = std::env::temp_dir().join("ctrlgear-test-config-tray-icon-style");
        let path = dir.join("config.toml");

        save_to(
            Some(path.clone()),
            &Settings { tray_icon_style: TrayIconStyle::Monochrome, ..Default::default() },
        )
        .unwrap();
        let settings = load_from(Some(path.clone()));

        assert_eq!(settings.tray_icon_style, TrayIconStyle::Monochrome);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn save_then_load_round_trips_notification_settings() {
        let dir = std::env::temp_dir().join("ctrlgear-test-config-notification-settings");
        let path = dir.join("config.toml");

        save_to(
            Some(path.clone()),
            &Settings {
                notification_persistent: true,
                notification_sound: SoundChoice::DialogWarning,
                ..Default::default()
            },
        )
        .unwrap();
        let settings = load_from(Some(path.clone()));

        assert_eq!(settings.notification_persistent, true);
        assert_eq!(settings.notification_sound, SoundChoice::DialogWarning);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn load_preserves_old_fields_from_a_pre_upgrade_config_file() {
        let dir = std::env::temp_dir().join("ctrlgear-test-config-pre-upgrade");
        let path = dir.join("config.toml");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(&path, "poll_interval_minutes = 15\n").unwrap();

        let settings = load_from(Some(path.clone()));

        assert_eq!(settings.poll_interval_minutes, 15);
        assert_eq!(settings.tray_icon_style, TrayIconStyle::default());
        assert_eq!(settings.notification_sound, SoundChoice::default());
        assert_eq!(settings.chatmix_sinks_enabled, false);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn save_then_load_round_trips_chatmix_sinks_enabled() {
        let dir = std::env::temp_dir().join("ctrlgear-test-config-chatmix-sinks-enabled");
        let path = dir.join("config.toml");

        save_to(
            Some(path.clone()),
            &Settings { chatmix_sinks_enabled: true, ..Default::default() },
        )
        .unwrap();
        let settings = load_from(Some(path.clone()));

        assert_eq!(settings.chatmix_sinks_enabled, true);
        let _ = std::fs::remove_file(&path);
    }
}
