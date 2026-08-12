use super::{
    SettingsMenu,
    table::{
        IDX_ADD_METADATA, IDX_ASCII_INDICATORS, IDX_CONCURRENT, IDX_COOKIES_BROWSER,
        IDX_FORMAT_PRESET, IDX_NETWORK_RETRY, IDX_OUTPUT_FORMAT, IDX_RATE_LIMIT,
        IDX_RESET_STATS_ON_BATCH, IDX_RETRY_DELAY, IDX_SPONSORBLOCK, IDX_WRITE_SUBTITLES,
        IDX_WRITE_THUMBNAIL,
    },
};
use crate::{
    app_state::AppState,
    utils::settings::{FormatPreset, OutputFormat},
};

impl SettingsMenu {
    /// Write the picked option back to the settings, then save
    ///
    /// Each row targets a different field, so the option index is decoded per
    /// row rather than through the table.
    pub(super) fn update_setting(&mut self, state: &AppState) {
        if let Some(selected_setting_idx) = self.list_state.selected() {
            match selected_setting_idx {
                IDX_FORMAT_PRESET => {
                    self.settings.format_preset = match self.option_index {
                        0 => FormatPreset::Best,
                        1 => FormatPreset::AudioOnly,
                        2 => FormatPreset::HD1080p,
                        3 => FormatPreset::HD720p,
                        4 => FormatPreset::SD480p,
                        5 => FormatPreset::SD360p,
                        _ => FormatPreset::Best,
                    };
                }
                IDX_OUTPUT_FORMAT => {
                    self.settings.output_format = if self.is_audio_only() {
                        match self.option_index {
                            0 => OutputFormat::Auto,
                            1 => OutputFormat::MP3,
                            _ => OutputFormat::Auto,
                        }
                    } else {
                        match self.option_index {
                            0 => OutputFormat::Auto,
                            1 => OutputFormat::MP4,
                            2 => OutputFormat::Mkv,
                            3 => OutputFormat::Webm,
                            4 => OutputFormat::MP3,
                            _ => OutputFormat::Auto,
                        }
                    };
                }
                IDX_WRITE_SUBTITLES => self.settings.write_subtitles = self.option_index == 1,
                IDX_WRITE_THUMBNAIL => self.settings.write_thumbnail = self.option_index == 1,
                IDX_ADD_METADATA => self.settings.add_metadata = self.option_index == 1,
                IDX_SPONSORBLOCK => self.settings.sponsorblock = self.option_index == 1,
                IDX_CONCURRENT => {
                    self.settings.concurrent_downloads = match self.option_index {
                        0 => 1,
                        1 => 2,
                        2 => 4,
                        3 => 8,
                        _ => self.settings.concurrent_downloads,
                    };
                }
                IDX_RATE_LIMIT => {
                    self.settings.rate_limit = match self.option_index {
                        0 => String::new(),
                        1 => "500K".to_string(),
                        2 => "1M".to_string(),
                        3 => "2M".to_string(),
                        4 => "5M".to_string(),
                        5 => "10M".to_string(),
                        _ => self.settings.rate_limit.clone(), // Custom - keep current
                    };
                }
                IDX_NETWORK_RETRY => self.settings.network_retry = self.option_index == 1,
                IDX_RETRY_DELAY => {
                    self.settings.retry_delay = match self.option_index {
                        0 => 1,
                        1 => 2,
                        2 => 5,
                        3 => 10,
                        _ => self.settings.retry_delay,
                    };
                }
                IDX_COOKIES_BROWSER => {
                    self.settings.cookies_from_browser = match self.option_index {
                        0 => String::new(),
                        1 => "firefox".to_string(),
                        2 => "chrome".to_string(),
                        3 => "chromium".to_string(),
                        4 => "brave".to_string(),
                        5 => "edge".to_string(),
                        6 => "opera".to_string(),
                        7 => "vivaldi".to_string(),
                        _ => String::new(),
                    };
                }
                IDX_ASCII_INDICATORS => self.settings.use_ascii_indicators = self.option_index == 1,
                IDX_RESET_STATS_ON_BATCH => {
                    self.settings.reset_stats_on_new_batch = self.option_index == 1
                }
                _ => {}
            }
        }

        self.option_index = 0;
        self.persist(state);
    }
}

#[cfg(test)]
mod tests;
