use super::{FormatPreset, OutputFormat, Settings};

/// Settings presets for common use cases
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SettingsPreset {
    BestQuality,
    AudioArchive,
    FastDownload,
    BandwidthSaver,
}

impl SettingsPreset {
    pub const fn all() -> &'static [SettingsPreset] {
        &[
            SettingsPreset::BestQuality,
            SettingsPreset::AudioArchive,
            SettingsPreset::FastDownload,
            SettingsPreset::BandwidthSaver,
        ]
    }

    pub const fn name(&self) -> &'static str {
        match self {
            SettingsPreset::BestQuality => "Best Quality",
            SettingsPreset::AudioArchive => "Audio Archive",
            SettingsPreset::FastDownload => "Fast Download",
            SettingsPreset::BandwidthSaver => "Bandwidth Saver",
        }
    }

    pub const fn description(&self) -> &'static str {
        match self {
            SettingsPreset::BestQuality => "Best video+audio, subtitles, thumbnails, metadata",
            SettingsPreset::AudioArchive => "Audio-only MP3 with metadata for music libraries",
            SettingsPreset::FastDownload => "Best quality, 8 concurrent, minimal extras",
            SettingsPreset::BandwidthSaver => "480p quality, 2 concurrent downloads",
        }
    }

    /// Create settings configured for this preset
    ///
    /// Presets only cover download behaviour, so the download directory from
    /// `current` is carried over instead of being reset.
    ///
    /// Each arm spells out every field, so these literals must be updated in
    /// lockstep with `impl Default for Settings`.
    pub fn apply(&self, current: &Settings) -> Settings {
        let download_dir = current.download_dir.clone();
        match self {
            SettingsPreset::BestQuality => Settings {
                format_preset: FormatPreset::Best,
                output_format: OutputFormat::Auto,
                write_subtitles: true,
                write_thumbnail: true,
                add_metadata: true,
                sponsorblock: false,
                concurrent_downloads: 4,
                rate_limit: String::new(),
                network_retry: true,
                retry_delay: 2,
                cookies_from_browser: String::new(),
                use_ascii_indicators: false,
                custom_ytdlp_args: String::new(),
                reset_stats_on_new_batch: true,
                download_dir,
            },
            SettingsPreset::AudioArchive => Settings {
                format_preset: FormatPreset::AudioOnly,
                output_format: OutputFormat::MP3,
                write_subtitles: false,
                write_thumbnail: true,
                add_metadata: true,
                sponsorblock: false,
                concurrent_downloads: 4,
                rate_limit: String::new(),
                network_retry: true,
                retry_delay: 2,
                cookies_from_browser: String::new(),
                use_ascii_indicators: false,
                custom_ytdlp_args: String::new(),
                reset_stats_on_new_batch: true,
                download_dir,
            },
            SettingsPreset::FastDownload => Settings {
                format_preset: FormatPreset::Best,
                output_format: OutputFormat::Auto,
                write_subtitles: false,
                write_thumbnail: false,
                add_metadata: false,
                sponsorblock: false,
                concurrent_downloads: 8,
                rate_limit: String::new(),
                network_retry: false,
                retry_delay: 1,
                cookies_from_browser: String::new(),
                use_ascii_indicators: false,
                custom_ytdlp_args: String::new(),
                reset_stats_on_new_batch: true,
                download_dir,
            },
            SettingsPreset::BandwidthSaver => Settings {
                format_preset: FormatPreset::SD480p,
                output_format: OutputFormat::Auto,
                write_subtitles: false,
                write_thumbnail: false,
                add_metadata: false,
                sponsorblock: false,
                concurrent_downloads: 2,
                rate_limit: "2M".to_string(),
                network_retry: true,
                retry_delay: 5,
                cookies_from_browser: String::new(),
                use_ascii_indicators: false,
                custom_ytdlp_args: String::new(),
                reset_stats_on_new_batch: true,
                download_dir,
            },
        }
    }
}
