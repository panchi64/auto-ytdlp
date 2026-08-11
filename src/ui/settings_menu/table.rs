//! One row per menu item: label, description, and how it is edited.
//!
//! Every part of the menu that used to switch on a bare index - descriptions,
//! option ceilings, which rows toggle, and the audio-only exceptions - reads
//! this table instead.

/// Number of regular settings items (before special actions)
pub(super) const SETTINGS_COUNT: usize = 15;

/// Menu item indices
pub(super) const IDX_FORMAT_PRESET: usize = 0;
pub(super) const IDX_OUTPUT_FORMAT: usize = 1;
pub(super) const IDX_DOWNLOAD_DIR: usize = 2;
pub(super) const IDX_WRITE_SUBTITLES: usize = 3;
pub(super) const IDX_WRITE_THUMBNAIL: usize = 4;
pub(super) const IDX_ADD_METADATA: usize = 5;
pub(super) const IDX_SPONSORBLOCK: usize = 6;
pub(super) const IDX_CONCURRENT: usize = 7;
pub(super) const IDX_RATE_LIMIT: usize = 8;
pub(super) const IDX_NETWORK_RETRY: usize = 9;
pub(super) const IDX_RETRY_DELAY: usize = 10;
pub(super) const IDX_COOKIES_BROWSER: usize = 11;
pub(super) const IDX_ASCII_INDICATORS: usize = 12;
pub(super) const IDX_RESET_STATS_ON_BATCH: usize = 13;
pub(super) const IDX_CUSTOM_ARGS: usize = 14;
pub(super) const IDX_APPLY_PRESET: usize = SETTINGS_COUNT;
pub(super) const IDX_RESET_DEFAULTS: usize = SETTINGS_COUNT + 1;

/// Total number of menu items
pub(super) const TOTAL_MENU_ITEMS: usize = SETTINGS_COUNT + 2;

/// How a row is edited
pub(super) enum SettingKind {
    /// Yes/No, applied as soon as an arrow key changes it
    Bool,
    /// Fixed list of choices, holding the highest valid option index
    Enum(usize),
    /// Free text, edited in the input popup rather than the option picker
    Text,
    /// Not a setting - opens a sub-menu
    Action,
}

pub(super) struct Setting {
    pub(super) label: &'static str,
    pub(super) description: &'static str,
    pub(super) kind: SettingKind,
    /// Replaces the kind's option ceiling while `FormatPreset::AudioOnly` is active
    pub(super) audio_only_max: Option<usize>,
}

impl Setting {
    /// Highest option index the row accepts under the current format preset
    pub(super) fn max_option_index(&self, is_audio_only: bool) -> usize {
        if is_audio_only && let Some(max) = self.audio_only_max {
            return max;
        }
        match self.kind {
            SettingKind::Bool => 1,
            SettingKind::Enum(max) => max,
            SettingKind::Text | SettingKind::Action => 0,
        }
    }

    /// Whether the row applies on the first arrow press instead of waiting for Enter
    pub(super) fn is_toggle(&self, is_audio_only: bool) -> bool {
        matches!(self.kind, SettingKind::Bool) && self.max_option_index(is_audio_only) > 0
    }
}

pub(super) const SETTINGS: [Setting; TOTAL_MENU_ITEMS] = [
    Setting {
        label: "Format Preset",
        description: "Video quality preset - Best downloads highest available quality",
        kind: SettingKind::Enum(5),
        audio_only_max: None,
    },
    Setting {
        label: "Output Format",
        description: "Container format - Auto lets yt-dlp choose based on source",
        kind: SettingKind::Enum(4),
        // Audio-only offers just Auto/MP3
        audio_only_max: Some(1),
    },
    Setting {
        label: "Download Directory",
        description: "Folder media files are saved to (~ expands to your home directory)",
        kind: SettingKind::Text,
        audio_only_max: None,
    },
    Setting {
        label: "Write Subtitles",
        description: "Download subtitles if available (disabled for audio-only)",
        kind: SettingKind::Bool,
        // Subtitles make no sense without video, so audio-only pins this to "No"
        audio_only_max: Some(0),
    },
    Setting {
        label: "Write Thumbnail",
        description: "Save video thumbnail as separate image file",
        kind: SettingKind::Bool,
        audio_only_max: None,
    },
    Setting {
        label: "Add Metadata",
        description: "Embed metadata (title, artist, etc.) into the file",
        kind: SettingKind::Bool,
        audio_only_max: None,
    },
    Setting {
        label: "SponsorBlock",
        description: "Remove sponsor segments from YouTube videos using SponsorBlock",
        kind: SettingKind::Bool,
        audio_only_max: None,
    },
    Setting {
        label: "Concurrent Downloads",
        description: "Number of simultaneous downloads (higher = faster, more bandwidth)",
        kind: SettingKind::Enum(4),
        audio_only_max: None,
    },
    Setting {
        label: "Rate Limit",
        description: "Limit download speed (e.g., 500K, 2M) - Unlimited uses full bandwidth",
        kind: SettingKind::Enum(6),
        audio_only_max: None,
    },
    Setting {
        label: "Network Retry",
        description: "Automatically retry downloads that fail due to network errors",
        kind: SettingKind::Bool,
        audio_only_max: None,
    },
    Setting {
        label: "Retry Delay",
        description: "Seconds to wait before retrying a failed download",
        kind: SettingKind::Enum(4),
        audio_only_max: None,
    },
    Setting {
        label: "Cookies from Browser",
        description: "Use browser cookies for age-restricted or authenticated content",
        kind: SettingKind::Enum(7),
        audio_only_max: None,
    },
    Setting {
        label: "ASCII Indicators",
        description: "Use text indicators [OK] instead of emoji for compatibility",
        kind: SettingKind::Bool,
        audio_only_max: None,
    },
    Setting {
        label: "Reset Stats on Batch",
        description: "Reset download counters when starting a new batch (S key)",
        kind: SettingKind::Bool,
        audio_only_max: None,
    },
    Setting {
        label: "Custom yt-dlp Args",
        description: "Extra yt-dlp flags (e.g., --no-playlist)",
        kind: SettingKind::Text,
        audio_only_max: None,
    },
    Setting {
        label: "Apply Preset...",
        description: "Apply a preset configuration for common use cases",
        kind: SettingKind::Action,
        audio_only_max: None,
    },
    Setting {
        label: "Reset to Defaults...",
        description: "Reset all settings to their default values (keeps the download directory)",
        kind: SettingKind::Action,
        audio_only_max: None,
    },
];

#[cfg(test)]
mod tests;
