use crossterm::event::{KeyCode, KeyEvent};

use super::{
    SettingsMenu, SubMenu,
    table::{
        IDX_ADD_METADATA, IDX_APPLY_PRESET, IDX_ASCII_INDICATORS, IDX_CONCURRENT,
        IDX_COOKIES_BROWSER, IDX_CUSTOM_ARGS, IDX_DOWNLOAD_DIR, IDX_FORMAT_PRESET,
        IDX_NETWORK_RETRY, IDX_OUTPUT_FORMAT, IDX_RATE_LIMIT, IDX_RESET_DEFAULTS,
        IDX_RESET_STATS_ON_BATCH, IDX_RETRY_DELAY, IDX_SPONSORBLOCK, IDX_WRITE_SUBTITLES,
        IDX_WRITE_THUMBNAIL, TOTAL_MENU_ITEMS,
    },
};
use crate::{
    app_state::AppState,
    utils::settings::{FormatPreset, OutputFormat, Settings, SettingsPreset},
};

impl SettingsMenu {
    pub fn handle_input(&mut self, key: KeyEvent, state: &AppState) -> bool {
        if !self.visible {
            return false;
        }

        match self.sub_menu {
            SubMenu::PresetSelection => return self.handle_preset_selection(key, state),
            SubMenu::ResetConfirmation => return self.handle_reset_confirmation(key, state),
            SubMenu::None => {}
        }

        if self.input_mode {
            self.handle_custom_input(key, state)
        } else if self.editing {
            self.handle_editing(key, state)
        } else {
            self.handle_menu_navigation(key, state)
        }
    }

    /// Handle preset selection sub-menu
    fn handle_preset_selection(&mut self, key: KeyEvent, state: &AppState) -> bool {
        let presets = SettingsPreset::all();
        match key.code {
            KeyCode::Esc => {
                self.sub_menu = SubMenu::None;
                true
            }
            KeyCode::Up => {
                if self.preset_index > 0 {
                    self.preset_index -= 1;
                }
                true
            }
            KeyCode::Down => {
                if self.preset_index < presets.len() - 1 {
                    self.preset_index += 1;
                }
                true
            }
            KeyCode::Enter => {
                // Apply the selected preset (keeps the configured download directory)
                self.settings = presets[self.preset_index].apply(&self.settings);
                self.persist(state);
                self.sub_menu = SubMenu::None;
                true
            }
            _ => false,
        }
    }

    /// Handle reset confirmation sub-menu
    fn handle_reset_confirmation(&mut self, key: KeyEvent, state: &AppState) -> bool {
        match key.code {
            KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => {
                self.sub_menu = SubMenu::None;
                true
            }
            KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y') => {
                // Reset to defaults, keeping the download directory so a reset
                // never silently relocates the user's media (presets keep it too)
                self.settings = Settings {
                    download_dir: self.settings.download_dir.clone(),
                    ..Settings::default()
                };
                self.persist(state);
                self.sub_menu = SubMenu::None;
                true
            }
            _ => false,
        }
    }

    /// Handle input while navigating the menu
    fn handle_menu_navigation(&mut self, key: KeyEvent, _state: &AppState) -> bool {
        match key.code {
            KeyCode::Esc => {
                self.visible = false;
                true
            }
            KeyCode::Enter => {
                if let Some(selected_setting_idx) = self.list_state.selected() {
                    self.activate(selected_setting_idx);
                }
                true
            }
            KeyCode::Up => {
                if let Some(i) = self.list_state.selected()
                    && i > 0
                {
                    self.list_state.select(Some(i - 1));
                }
                true
            }
            KeyCode::Down => {
                if let Some(i) = self.list_state.selected()
                    && i < TOTAL_MENU_ITEMS - 1
                {
                    self.list_state.select(Some(i + 1));
                }
                true
            }
            _ => false,
        }
    }

    /// Open the selected row for editing
    ///
    /// The option picker needs the current value as an index, and every row
    /// maps a different type, so the conversion stays a match on the row.
    fn activate(&mut self, index: usize) {
        match index {
            IDX_FORMAT_PRESET => self.begin_edit(match self.settings.format_preset {
                FormatPreset::Best => 0,
                FormatPreset::AudioOnly => 1,
                FormatPreset::HD1080p => 2,
                FormatPreset::HD720p => 3,
                FormatPreset::SD480p => 4,
                FormatPreset::SD360p => 5,
            }),
            IDX_OUTPUT_FORMAT => {
                let option = if self.is_audio_only() {
                    match self.settings.output_format {
                        OutputFormat::Auto => 0,
                        OutputFormat::MP3 => 1,
                        OutputFormat::MP4 | OutputFormat::Mkv | OutputFormat::Webm => 0,
                    }
                } else {
                    match self.settings.output_format {
                        OutputFormat::Auto => 0,
                        OutputFormat::MP4 => 1,
                        OutputFormat::Mkv => 2,
                        OutputFormat::Webm => 3,
                        OutputFormat::MP3 => 4,
                    }
                };
                self.begin_edit(option);
            }
            IDX_DOWNLOAD_DIR => self.begin_text_input(self.settings.download_dir.clone()),
            IDX_WRITE_SUBTITLES => self.begin_edit(usize::from(self.settings.write_subtitles)),
            IDX_WRITE_THUMBNAIL => self.begin_edit(usize::from(self.settings.write_thumbnail)),
            IDX_ADD_METADATA => self.begin_edit(usize::from(self.settings.add_metadata)),
            IDX_SPONSORBLOCK => self.begin_edit(usize::from(self.settings.sponsorblock)),
            IDX_CONCURRENT => self.begin_edit(match self.settings.concurrent_downloads {
                1 => 0,
                2 => 1,
                4 => 2,
                8 => 3,
                _ => 4, // Index for "Custom"
            }),
            IDX_RATE_LIMIT => self.begin_edit(match self.settings.rate_limit.as_str() {
                "" => 0,
                "500K" => 1,
                "1M" => 2,
                "2M" => 3,
                "5M" => 4,
                "10M" => 5,
                _ => 6, // Custom
            }),
            IDX_NETWORK_RETRY => self.begin_edit(usize::from(self.settings.network_retry)),
            IDX_RETRY_DELAY => self.begin_edit(match self.settings.retry_delay {
                1 => 0,
                2 => 1,
                5 => 2,
                10 => 3,
                _ => 4, // Index for "Custom"
            }),
            IDX_COOKIES_BROWSER => {
                self.begin_edit(match self.settings.cookies_from_browser.as_str() {
                    "" => 0,
                    "firefox" => 1,
                    "chrome" => 2,
                    "chromium" => 3,
                    "brave" => 4,
                    "edge" => 5,
                    "opera" => 6,
                    "vivaldi" => 7,
                    _ => 0,
                })
            }
            IDX_ASCII_INDICATORS => {
                self.begin_edit(usize::from(self.settings.use_ascii_indicators))
            }
            IDX_RESET_STATS_ON_BATCH => {
                self.begin_edit(usize::from(self.settings.reset_stats_on_new_batch))
            }
            IDX_CUSTOM_ARGS => self.begin_text_input(self.settings.custom_ytdlp_args.clone()),
            IDX_APPLY_PRESET => {
                self.preset_index = 0;
                self.sub_menu = SubMenu::PresetSelection;
            }
            IDX_RESET_DEFAULTS => self.sub_menu = SubMenu::ResetConfirmation,
            _ => self.option_index = 0,
        }
    }

    /// Show the option picker with `option` preselected
    ///
    /// The index is clamped to what the row currently allows, which is how
    /// audio-only pins subtitles to "No".
    fn begin_edit(&mut self, option: usize) {
        self.option_index = option;
        self.adjust_option_index();
        self.editing = true;
    }

    fn begin_text_input(&mut self, current: String) {
        self.custom_input = current;
        self.validation_error = None;
        self.input_mode = true;
    }
}

#[cfg(test)]
mod tests;
