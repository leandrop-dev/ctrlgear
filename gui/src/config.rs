#[derive(serde::Serialize, serde::Deserialize)]
pub struct Settings {
    pub poll_interval_minutes: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Self { poll_interval_minutes: 5 }
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

fn load_from(path: Option<std::path::PathBuf>) -> Settings {
    path.and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| toml::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_to(path: Option<std::path::PathBuf>, settings: &Settings) -> std::io::Result<()> {
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

        save_to(Some(path.clone()), &Settings { poll_interval_minutes: 15 }).unwrap();
        let settings = load_from(Some(path.clone()));

        assert_eq!(settings.poll_interval_minutes, 15);
        let _ = std::fs::remove_file(&path);
    }
}
