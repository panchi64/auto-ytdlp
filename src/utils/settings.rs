use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    io::BufReader,
    path::PathBuf,
};

/// Flags that conflict with custom yt-dlp arguments
const CONFLICTING_FLAGS: &[&str] = &[
    "--download-archive",
    "-a",
    "--output",
    "-o",
    "--progress-template",
];

/// Settings presets for common use cases
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SettingsPreset {
    /// Best quality with all extras (subtitles, thumbnails, metadata)
    BestQuality,
    /// Audio-only with metadata for music archiving
    AudioArchive,
    /// Minimal processing for fast downloads
    FastDownload,
    /// Lower quality to save bandwidth
    BandwidthSaver,
}

impl SettingsPreset {
    /// Get all available presets
    pub const fn all() -> &'static [SettingsPreset] {
        &[
            SettingsPreset::BestQuality,
            SettingsPreset::AudioArchive,
            SettingsPreset::FastDownload,
            SettingsPreset::BandwidthSaver,
        ]
    }

    /// Get the display name for this preset
    pub const fn name(&self) -> &'static str {
        match self {
            SettingsPreset::BestQuality => "Best Quality",
            SettingsPreset::AudioArchive => "Audio Archive",
            SettingsPreset::FastDownload => "Fast Download",
            SettingsPreset::BandwidthSaver => "Bandwidth Saver",
        }
    }

    /// Get the description for this preset
    pub const fn description(&self) -> &'static str {
        match self {
            SettingsPreset::BestQuality => "Best video+audio, subtitles, thumbnails, metadata",
            SettingsPreset::AudioArchive => "Audio-only MP3 with metadata for music libraries",
            SettingsPreset::FastDownload => "Best quality, 8 concurrent, minimal extras",
            SettingsPreset::BandwidthSaver => "480p quality, 2 concurrent downloads",
        }
    }

    /// Create settings configured for this preset
    pub fn apply(&self) -> Settings {
        match self {
            SettingsPreset::BestQuality => Settings {
                format_preset: FormatPreset::Best,
                output_format: OutputFormat::Auto,
                write_subtitles: true,
                write_thumbnail: true,
                add_metadata: true,
                concurrent_downloads: 4,
                network_retry: true,
                retry_delay: 2,
                use_ascii_indicators: false,
                custom_ytdlp_args: String::new(),
                reset_stats_on_new_batch: true,
            },
            SettingsPreset::AudioArchive => Settings {
                format_preset: FormatPreset::AudioOnly,
                output_format: OutputFormat::MP3,
                write_subtitles: false,
                write_thumbnail: true,
                add_metadata: true,
                concurrent_downloads: 4,
                network_retry: true,
                retry_delay: 2,
                use_ascii_indicators: false,
                custom_ytdlp_args: String::new(),
                reset_stats_on_new_batch: true,
            },
            SettingsPreset::FastDownload => Settings {
                format_preset: FormatPreset::Best,
                output_format: OutputFormat::Auto,
                write_subtitles: false,
                write_thumbnail: false,
                add_metadata: false,
                concurrent_downloads: 8,
                network_retry: false,
                retry_delay: 1,
                use_ascii_indicators: false,
                custom_ytdlp_args: String::new(),
                reset_stats_on_new_batch: true,
            },
            SettingsPreset::BandwidthSaver => Settings {
                format_preset: FormatPreset::SD480p,
                output_format: OutputFormat::Auto,
                write_subtitles: false,
                write_thumbnail: false,
                add_metadata: false,
                concurrent_downloads: 2,
                network_retry: true,
                retry_delay: 5,
                use_ascii_indicators: false,
                custom_ytdlp_args: String::new(),
                reset_stats_on_new_batch: true,
            },
        }
    }
}

/// Video format preset options
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum FormatPreset {
    /// Best video and audio quality
    #[default]
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

impl FormatPreset {
    /// Get the yt-dlp format argument string for this preset
    ///
    /// Returns a static string reference to avoid allocations.
    pub fn get_format_arg(&self) -> &'static str {
        match self {
            FormatPreset::Best => "bestvideo*+bestaudio/best",
            FormatPreset::AudioOnly => "bestaudio/best",
            FormatPreset::HD1080p => "bestvideo[height<=1080]+bestaudio/best[height<=1080]",
            FormatPreset::HD720p => "bestvideo[height<=720]+bestaudio/best[height<=720]",
            FormatPreset::SD480p => "bestvideo[height<=480]+bestaudio/best[height<=480]",
            FormatPreset::SD360p => "bestvideo[height<=360]+bestaudio/best[height<=360]",
        }
    }
}

/// Output file format options
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum OutputFormat {
    /// Let yt-dlp decide based on source
    #[default]
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

impl OutputFormat {
    /// Get the yt-dlp output format argument/modifier
    ///
    /// Returns a static string reference to avoid allocations.
    pub fn get_format_modifier(&self) -> Option<&'static str> {
        match self {
            OutputFormat::Auto => None,
            OutputFormat::MP4 => Some("--merge-output-format mp4"),
            OutputFormat::Mkv => Some("--merge-output-format mkv"),
            OutputFormat::MP3 => Some("--extract-audio --audio-format mp3"),
            OutputFormat::Webm => Some("--merge-output-format webm"),
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
    /// Custom yt-dlp arguments (shell-style, validated for conflicts)
    #[serde(default)]
    pub custom_ytdlp_args: String,
    /// Reset download stats when starting a new batch (pressing 'S')
    /// When true (default): counters reset on each new batch
    /// When false: counters accumulate across batches in a session
    #[serde(default = "default_true")]
    pub reset_stats_on_new_batch: bool,
}

/// Default function for serde to use true as default
fn default_true() -> bool {
    true
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
            custom_ytdlp_args: String::new(),
            reset_stats_on_new_batch: true,
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

    /// Validate custom yt-dlp arguments for conflicts
    ///
    /// Returns Ok(()) if valid, or Err with a description of the conflict.
    pub fn validate_custom_args(args: &str) -> std::result::Result<(), String> {
        if args.trim().is_empty() {
            return Ok(());
        }

        // Parse with shlex to handle quoted arguments properly
        let parsed = match shlex::split(args) {
            Some(args) => args,
            None => return Err("Invalid argument syntax (unmatched quotes)".to_string()),
        };

        for arg in &parsed {
            for conflict in CONFLICTING_FLAGS {
                if arg == *conflict || arg.starts_with(&format!("{}=", conflict)) {
                    return Err(format!(
                        "'{}' conflicts with auto-ytdlp's internal handling",
                        conflict
                    ));
                }
            }
        }

        Ok(())
    }

    /// Parse custom arguments into a vector of strings
    ///
    /// Returns an empty vector if parsing fails or args is empty.
    /// Logs a warning to stderr if the arguments contain malformed shell syntax.
    pub fn parse_custom_args(&self) -> Vec<String> {
        if self.custom_ytdlp_args.trim().is_empty() {
            return Vec::new();
        }

        match shlex::split(&self.custom_ytdlp_args) {
            Some(args) => args,
            None => {
                eprintln!(
                    "Warning: custom yt-dlp args have malformed shell syntax (e.g., unclosed quotes): {}",
                    self.custom_ytdlp_args
                );
                Vec::new()
            }
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

        // Write to temporary file first
        fs::write(&temp_path, &settings_json)
            .with_context(|| format!("Failed to write temp settings file: {:?}", temp_path))?;

        // Atomic rename - this operation is atomic on most filesystems
        fs::rename(&temp_path, &settings_path)
            .with_context(|| format!("Failed to rename temp settings to: {:?}", settings_path))
    }

    /// Build the yt-dlp command arguments based on current settings
    pub fn get_ytdlp_args(&self, output_template: &str) -> Vec<String> {
        // Pre-allocate with capacity estimate:
        // Base: 4 (format, format_arg, output, template)
        // + 3 (potential format modifiers)
        // + 4 (potential subtitles: --write-auto-subs --sub-langs all)
        // + 1 (potential thumbnail)
        // + 1 (potential metadata)
        // + 1 (newline)
        // = ~14 max
        let mut args = Vec::with_capacity(14);

        args.push("--format".to_string());
        args.push(self.format_preset.get_format_arg().to_string());
        args.push("--output".to_string());
        args.push(output_template.to_string());

        // Add output format modifiers if any
        if let Some(format_modifier) = self.output_format.get_format_modifier() {
            // Iterate directly without collecting to intermediate Vec
            for modifier in format_modifier.split_whitespace() {
                args.push(modifier.to_string());
            }
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

        // Add custom yt-dlp arguments (already validated)
        args.extend(self.parse_custom_args());

        args
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_default_values() {
        let settings = Settings::default();

        assert_eq!(settings.format_preset, FormatPreset::Best);
        assert_eq!(settings.output_format, OutputFormat::Auto);
        assert!(!settings.write_subtitles);
        assert_eq!(settings.concurrent_downloads, 4);
        assert!(!settings.write_thumbnail);
        assert!(!settings.add_metadata);
        assert!(!settings.network_retry);
        assert_eq!(settings.retry_delay, 2);
        assert!(!settings.use_ascii_indicators);
        assert!(settings.custom_ytdlp_args.is_empty());
        assert!(settings.reset_stats_on_new_batch);
    }

    #[test]
    fn test_format_preset_best() {
        assert_eq!(
            FormatPreset::Best.get_format_arg(),
            "bestvideo*+bestaudio/best"
        );
    }

    #[test]
    fn test_format_preset_audio_only() {
        assert_eq!(FormatPreset::AudioOnly.get_format_arg(), "bestaudio/best");
    }

    #[test]
    fn test_format_preset_hd1080p() {
        assert_eq!(
            FormatPreset::HD1080p.get_format_arg(),
            "bestvideo[height<=1080]+bestaudio/best[height<=1080]"
        );
    }

    #[test]
    fn test_format_preset_hd720p() {
        assert_eq!(
            FormatPreset::HD720p.get_format_arg(),
            "bestvideo[height<=720]+bestaudio/best[height<=720]"
        );
    }

    #[test]
    fn test_format_preset_sd480p() {
        assert_eq!(
            FormatPreset::SD480p.get_format_arg(),
            "bestvideo[height<=480]+bestaudio/best[height<=480]"
        );
    }

    #[test]
    fn test_format_preset_sd360p() {
        assert_eq!(
            FormatPreset::SD360p.get_format_arg(),
            "bestvideo[height<=360]+bestaudio/best[height<=360]"
        );
    }

    #[test]
    fn test_output_format_auto() {
        assert_eq!(OutputFormat::Auto.get_format_modifier(), None);
    }

    #[test]
    fn test_output_format_mp4() {
        assert_eq!(
            OutputFormat::MP4.get_format_modifier(),
            Some("--merge-output-format mp4")
        );
    }

    #[test]
    fn test_output_format_mkv() {
        assert_eq!(
            OutputFormat::Mkv.get_format_modifier(),
            Some("--merge-output-format mkv")
        );
    }

    #[test]
    fn test_output_format_mp3() {
        assert_eq!(
            OutputFormat::MP3.get_format_modifier(),
            Some("--extract-audio --audio-format mp3")
        );
    }

    #[test]
    fn test_output_format_webm() {
        assert_eq!(
            OutputFormat::Webm.get_format_modifier(),
            Some("--merge-output-format webm")
        );
    }

    #[test]
    fn test_validate_custom_args_empty() {
        assert!(Settings::validate_custom_args("").is_ok());
        assert!(Settings::validate_custom_args("   ").is_ok());
    }

    #[test]
    fn test_validate_custom_args_valid() {
        assert!(Settings::validate_custom_args("--no-playlist").is_ok());
        assert!(Settings::validate_custom_args("--limit-rate 1M --retries 5").is_ok());
        assert!(Settings::validate_custom_args("--user-agent 'My Bot'").is_ok());
    }

    #[test]
    fn test_validate_custom_args_conflicting_flags() {
        let result = Settings::validate_custom_args("--download-archive my_archive.txt");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("--download-archive"));

        let result = Settings::validate_custom_args("-o ~/Downloads");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("-o"));

        let result = Settings::validate_custom_args("--output ~/Downloads");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("--output"));

        let result = Settings::validate_custom_args("--progress-template test");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("--progress-template"));
    }

    #[test]
    fn test_validate_custom_args_unmatched_quotes() {
        let result = Settings::validate_custom_args("--user-agent 'unmatched");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unmatched quotes"));
    }

    #[test]
    fn test_parse_custom_args_empty() {
        let settings = Settings::default();
        assert!(settings.parse_custom_args().is_empty());

        let mut settings = Settings::default();
        settings.custom_ytdlp_args = "   ".to_string();
        assert!(settings.parse_custom_args().is_empty());
    }

    #[test]
    fn test_parse_custom_args_simple() {
        let mut settings = Settings::default();
        settings.custom_ytdlp_args = "--no-playlist --retries 5".to_string();
        let args = settings.parse_custom_args();
        assert_eq!(args, vec!["--no-playlist", "--retries", "5"]);
    }

    #[test]
    fn test_parse_custom_args_quoted() {
        let mut settings = Settings::default();
        settings.custom_ytdlp_args = "--user-agent 'My Custom Agent'".to_string();
        let args = settings.parse_custom_args();
        assert_eq!(args, vec!["--user-agent", "My Custom Agent"]);
    }

    #[test]
    fn test_get_ytdlp_args_basic() {
        let settings = Settings::default();
        let args = settings.get_ytdlp_args("%(title)s.%(ext)s");

        assert!(args.contains(&"--format".to_string()));
        assert!(args.contains(&"bestvideo*+bestaudio/best".to_string()));
        assert!(args.contains(&"--output".to_string()));
        assert!(args.contains(&"%(title)s.%(ext)s".to_string()));
        assert!(args.contains(&"--newline".to_string()));

        // Default settings should not include optional flags
        assert!(!args.contains(&"--write-auto-subs".to_string()));
        assert!(!args.contains(&"--write-thumbnail".to_string()));
        assert!(!args.contains(&"--add-metadata".to_string()));
    }

    #[test]
    fn test_get_ytdlp_args_all_options() {
        let mut settings = Settings::default();
        settings.write_subtitles = true;
        settings.write_thumbnail = true;
        settings.add_metadata = true;
        settings.output_format = OutputFormat::MP4;
        settings.custom_ytdlp_args = "--no-playlist".to_string();

        let args = settings.get_ytdlp_args("%(title)s.%(ext)s");

        assert!(args.contains(&"--write-auto-subs".to_string()));
        assert!(args.contains(&"--sub-langs".to_string()));
        assert!(args.contains(&"all".to_string()));
        assert!(args.contains(&"--write-thumbnail".to_string()));
        assert!(args.contains(&"--add-metadata".to_string()));
        assert!(args.contains(&"--merge-output-format".to_string()));
        assert!(args.contains(&"mp4".to_string()));
        assert!(args.contains(&"--no-playlist".to_string()));
    }

    #[test]
    fn test_preset_best_quality() {
        let settings = SettingsPreset::BestQuality.apply();
        assert_eq!(settings.format_preset, FormatPreset::Best);
        assert_eq!(settings.output_format, OutputFormat::Auto);
        assert!(settings.write_subtitles);
        assert!(settings.write_thumbnail);
        assert!(settings.add_metadata);
        assert_eq!(settings.concurrent_downloads, 4);
        assert!(settings.network_retry);
    }

    #[test]
    fn test_preset_audio_archive() {
        let settings = SettingsPreset::AudioArchive.apply();
        assert_eq!(settings.format_preset, FormatPreset::AudioOnly);
        assert_eq!(settings.output_format, OutputFormat::MP3);
        assert!(!settings.write_subtitles);
        assert!(settings.write_thumbnail);
        assert!(settings.add_metadata);
    }

    #[test]
    fn test_preset_fast_download() {
        let settings = SettingsPreset::FastDownload.apply();
        assert_eq!(settings.format_preset, FormatPreset::Best);
        assert!(!settings.write_subtitles);
        assert!(!settings.write_thumbnail);
        assert!(!settings.add_metadata);
        assert_eq!(settings.concurrent_downloads, 8);
        assert!(!settings.network_retry);
    }

    #[test]
    fn test_preset_bandwidth_saver() {
        let settings = SettingsPreset::BandwidthSaver.apply();
        assert_eq!(settings.format_preset, FormatPreset::SD480p);
        assert!(!settings.write_subtitles);
        assert!(!settings.write_thumbnail);
        assert!(!settings.add_metadata);
        assert_eq!(settings.concurrent_downloads, 2);
        assert!(settings.network_retry);
        assert_eq!(settings.retry_delay, 5);
    }

    #[test]
    fn test_parse_custom_args_malformed_unclosed_single_quote() {
        let mut settings = Settings::default();
        settings.custom_ytdlp_args = "--user-agent 'unclosed".to_string();
        let args = settings.parse_custom_args();
        assert!(args.is_empty());
    }

    #[test]
    fn test_parse_custom_args_malformed_unclosed_double_quote() {
        let mut settings = Settings::default();
        settings.custom_ytdlp_args = "--user-agent \"unclosed".to_string();
        let args = settings.parse_custom_args();
        assert!(args.is_empty());
    }

    #[test]
    fn test_parse_custom_args_malformed_trailing_backslash() {
        let mut settings = Settings::default();
        settings.custom_ytdlp_args = "test\\".to_string();
        let args = settings.parse_custom_args();
        assert!(args.is_empty());
    }

    #[test]
    fn test_parse_custom_args_valid_double_quotes() {
        let mut settings = Settings::default();
        settings.custom_ytdlp_args = "--user-agent \"My Custom Agent\"".to_string();
        let args = settings.parse_custom_args();
        assert_eq!(args, vec!["--user-agent", "My Custom Agent"]);
    }

    #[test]
    fn test_parse_custom_args_multiple_quoted_segments() {
        let mut settings = Settings::default();
        settings.custom_ytdlp_args = "--cookies 'path/to/cookies' --user-agent 'Bot'".to_string();
        let args = settings.parse_custom_args();
        assert_eq!(
            args,
            vec!["--cookies", "path/to/cookies", "--user-agent", "Bot"]
        );
    }
}
