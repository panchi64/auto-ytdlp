use anyhow::{Context, Result};
use std::{
    fs::{self, File},
    io::BufReader,
    path::PathBuf,
};

use super::Settings;

/// Settings file used by unit tests
///
/// Tests drive the real save/load paths, so without this they would overwrite
/// the developer's own `~/.config/auto-ytdlp/settings.json`. The file is
/// per-thread - cargo runs each test on its own thread, so tests that save
/// settings cannot clobber each other's expectations.
#[cfg(test)]
fn test_settings_path() -> PathBuf {
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
    thread_local! {
        static PATH: PathBuf = {
            let dir = std::env::temp_dir().join("auto-ytdlp-test-config");
            fs::create_dir_all(&dir).ok();
            dir.join(format!(
                "settings-{}.json",
                NEXT_ID.fetch_add(1, Ordering::Relaxed)
            ))
        };
    }
    PATH.with(|path| path.clone())
}

impl Settings {
    fn get_settings_path() -> PathBuf {
        #[cfg(test)]
        return test_settings_path();

        #[cfg(not(test))]
        {
            let mut config_dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
            config_dir.push("auto-ytdlp");
            fs::create_dir_all(&config_dir).ok();
            config_dir.push("settings.json");
            config_dir
        }
    }

    /// Load settings from disk, creating default settings if none exist
    pub fn load() -> Result<Self> {
        let settings_path = Self::get_settings_path();

        if !settings_path.exists() {
            let default_settings = Self::default();
            default_settings.save()?;
            return Ok(default_settings);
        }

        let file = File::open(&settings_path)
            .with_context(|| format!("Failed to open settings file: {:?}", settings_path))?;
        let reader = BufReader::new(file);

        serde_json::from_reader(reader).with_context(|| "Failed to parse settings file".to_string())
    }

    /// Save settings to disk using atomic write (write to temp file, then rename).
    ///
    /// This prevents corrupted settings files if the application crashes mid-write.
    pub fn save(&self) -> Result<()> {
        let settings_path = Self::get_settings_path();
        let temp_path = settings_path.with_extension("json.tmp");

        let settings_json = serde_json::to_string_pretty(self)?;

        fs::write(&temp_path, &settings_json)
            .with_context(|| format!("Failed to write temp settings file: {:?}", temp_path))?;

        // Atomic rename - this operation is atomic on most filesystems
        fs::rename(&temp_path, &settings_path)
            .with_context(|| format!("Failed to rename temp settings to: {:?}", settings_path))
    }
}
