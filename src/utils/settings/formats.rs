use serde::{Deserialize, Serialize};

/// Video format preset options
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum FormatPreset {
    #[default]
    Best,
    AudioOnly,
    HD1080p,
    HD720p,
    SD480p,
    SD360p,
}

impl FormatPreset {
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
    MP4,
    Mkv,
    MP3,
    Webm,
}

impl OutputFormat {
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
