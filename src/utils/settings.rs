use serde::{Deserialize, Serialize};

mod formats;
mod paths;
mod persistence;
mod presets;
mod ytdlp_args;

pub use formats::{FormatPreset, OutputFormat};
pub use paths::expand_tilde;
pub use presets::SettingsPreset;

/// Settings for the auto-ytdlp application
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub format_preset: FormatPreset,
    pub output_format: OutputFormat,
    pub write_subtitles: bool,
    pub write_thumbnail: bool,
    pub add_metadata: bool,
    #[serde(default)]
    pub sponsorblock: bool,
    pub concurrent_downloads: usize,
    /// Rate limit for downloads (e.g., "500K", "2M"), empty for unlimited
    #[serde(default)]
    pub rate_limit: String,
    /// Automatically retry failed downloads due to network issues
    pub network_retry: bool,
    /// Delay in seconds between retry attempts
    pub retry_delay: u64,
    /// Browser to pull cookies from for authenticated content (e.g., "firefox")
    #[serde(default)]
    pub cookies_from_browser: String,
    /// Use ASCII indicators instead of emoji (for terminal compatibility)
    #[serde(default)]
    pub use_ascii_indicators: bool,
    /// Custom yt-dlp arguments (shell-style, validated for conflicts)
    #[serde(default)]
    pub custom_ytdlp_args: String,
    /// When false, counters accumulate across batches in a session
    #[serde(default = "default_true")]
    pub reset_stats_on_new_batch: bool,
    /// Directory media files are downloaded to, empty to use the built-in
    /// default (or whatever `--download-dir` was passed on the command line).
    /// A leading `~` is expanded to the user's home directory.
    #[serde(default)]
    pub download_dir: String,
}

fn default_true() -> bool {
    true
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            format_preset: FormatPreset::default(),
            output_format: OutputFormat::default(),
            write_subtitles: false,
            write_thumbnail: false,
            add_metadata: false,
            sponsorblock: false,
            concurrent_downloads: 4,
            rate_limit: String::new(),
            network_retry: false,
            retry_delay: 2,
            cookies_from_browser: String::new(),
            use_ascii_indicators: false,
            custom_ytdlp_args: String::new(),
            reset_stats_on_new_batch: true,
            download_dir: String::new(),
        }
    }
}

/// Settings with only the download directory configured
#[cfg(test)]
pub fn settings_with_dir(dir: &str) -> Settings {
    Settings {
        download_dir: dir.to_string(),
        ..Settings::default()
    }
}

#[cfg(test)]
mod tests;
