use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    io::BufReader,
    path::PathBuf,
};

/// Video format preset options
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FormatPreset {
    /// Best video and audio quality
    Best,
    /// Audio only
    AudioOnly,
    /// 1080p resolution
    HD1080p,
    /// 720p resolution
    HD720p,
    /// 480p resolution
    SD480p,
    /// 360p resolution
    SD360p,
}

impl Default for FormatPreset {
    fn default() -> Self {
        Self::Best
    }
}

impl FormatPreset {
    /// Get the yt-dlp format argument string for this preset
    pub fn get_format_arg(&self) -> String {
        match self {
            FormatPreset::Best => "bestvideo*+bestaudio/best".to_string(),
            FormatPreset::AudioOnly => "bestaudio/best".to_string(),
            FormatPreset::HD1080p => {
                "bestvideo[height<=1080]+bestaudio/best[height<=1080]".to_string()
            }
            FormatPreset::HD720p => {
                "bestvideo[height<=720]+bestaudio/best[height<=720]".to_string()
            }
            FormatPreset::SD480p => {
                "bestvideo[height<=480]+bestaudio/best[height<=480]".to_string()
            }
            FormatPreset::SD360p => {
                "bestvideo[height<=360]+bestaudio/best[height<=360]".to_string()
            }
        }
    }
}

/// Output file format options
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OutputFormat {
    /// Let yt-dlp decide based on source
    Auto,
    /// MP4 format
    MP4,
    /// MKV format
    Mkv,
    /// MP3 format (for audio)
    MP3,
    /// WEBM format
    Webm,
}

impl Default for OutputFormat {
    fn default() -> Self {
        Self::Auto
    }
}

impl OutputFormat {
    /// Get the yt-dlp output format argument/modifier
    pub fn get_format_modifier(&self) -> Option<String> {
        match self {
            OutputFormat::Auto => None,
            OutputFormat::MP4 => Some("--merge-output-format mp4".to_string()),
            OutputFormat::Mkv => Some("--merge-output-format mkv".to_string()),
            OutputFormat::MP3 => Some("--extract-audio --audio-format mp3".to_string()),
            OutputFormat::Webm => Some("--merge-output-format webm".to_string()),
        }
    }
}

/// Settings for the auto-ytdlp application
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// Format preset to use
    pub format_preset: FormatPreset,
    /// Output format
    pub output_format: OutputFormat,
    /// Write subtitles if available
    pub write_subtitles: bool,
    /// Number of concurrent downloads
    pub concurrent_downloads: usize,
    /// Write thumbnail if available
    pub write_thumbnail: bool,
    /// Add metadata to file if available
    pub add_metadata: bool,
    /// Automatically retry failed downloads due to network issues
    pub network_retry: bool,
    /// Delay in seconds between retry attempts
    pub retry_delay: u64,
    /// Use ASCII indicators instead of emoji (for terminal compatibility)
    #[serde(default)]
    pub use_ascii_indicators: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            format_preset: FormatPreset::default(),
            output_format: OutputFormat::default(),
            write_subtitles: false,
            concurrent_downloads: 4,
            write_thumbnail: false,
            add_metadata: false,
            network_retry: false,
            retry_delay: 2,
            use_ascii_indicators: false,
        }
    }
}

impl Settings {
    /// Get the settings file path
    fn get_settings_path() -> PathBuf {
        let mut config_dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        config_dir.push("auto-ytdlp");
        fs::create_dir_all(&config_dir).ok();
        config_dir.push("settings.json");
        config_dir
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

        // Write to temporary file first
        fs::write(&temp_path, &settings_json)
            .with_context(|| format!("Failed to write temp settings file: {:?}", temp_path))?;

        // Atomic rename - this operation is atomic on most filesystems
        fs::rename(&temp_path, &settings_path)
            .with_context(|| format!("Failed to rename temp settings to: {:?}", settings_path))
    }

    /// Build the yt-dlp command arguments based on current settings
    pub fn get_ytdlp_args(&self, output_template: &str) -> Vec<String> {
        let mut args = vec![
            "--format".to_string(),
            self.format_preset.get_format_arg(),
            "--output".to_string(),
            output_template.to_string(),
        ];

        // Add output format modifiers if any
        if let Some(format_modifier) = self.output_format.get_format_modifier() {
            let modifiers: Vec<&str> = format_modifier.split_whitespace().collect();
            args.extend(modifiers.iter().map(|s| s.to_string()));
        }

        // Add optional arguments based on settings
        if self.write_subtitles {
            args.push("--write-auto-subs".to_string());
            args.push("--sub-langs".to_string());
            args.push("all".to_string());
        }

        if self.write_thumbnail {
            args.push("--write-thumbnail".to_string());
        }

        if self.add_metadata {
            args.push("--add-metadata".to_string());
        }

        // Always add newline for output processing
        args.push("--newline".to_string());

        args
    }
}
