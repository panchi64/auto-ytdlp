use crossterm::event::{KeyCode, KeyEvent};

use super::{
    SettingsMenu,
    table::{IDX_CONCURRENT, IDX_CUSTOM_ARGS, IDX_DOWNLOAD_DIR, IDX_RATE_LIMIT, IDX_RETRY_DELAY},
};
use crate::{app_state::AppState, utils::settings::Settings};

impl SettingsMenu {
    /// Handle input while editing a setting
    pub(super) fn handle_editing(&mut self, key: KeyEvent, state: &AppState) -> bool {
        match key.code {
            KeyCode::Esc => {
                self.editing = false;
                true
            }
            KeyCode::Left => {
                if self.option_index > 0 {
                    self.option_index -= 1;
                }
                self.apply_if_toggle(state);
                true
            }
            KeyCode::Right => {
                self.option_index += 1;
                self.adjust_option_index();
                self.apply_if_toggle(state);
                true
            }
            KeyCode::Enter => {
                // The last option of these rows opens the input popup instead
                let custom_value = match (self.list_state.selected(), self.option_index) {
                    (Some(IDX_CONCURRENT), 4) => {
                        Some(self.settings.concurrent_downloads.to_string())
                    }
                    (Some(IDX_RATE_LIMIT), 6) => Some(self.settings.rate_limit.clone()),
                    (Some(IDX_RETRY_DELAY), 4) => Some(self.settings.retry_delay.to_string()),
                    _ => None,
                };
                if let Some(value) = custom_value {
                    self.custom_input = value;
                    self.input_mode = true;
                    return true;
                }

                self.update_setting(state);
                self.editing = false;
                true
            }
            _ => false,
        }
    }

    /// Yes/No rows apply as soon as an arrow key moves them, without an Enter
    fn apply_if_toggle(&mut self, state: &AppState) {
        let is_audio_only = self.is_audio_only();
        if self
            .selected_setting()
            .is_some_and(|setting| setting.is_toggle(is_audio_only))
        {
            self.update_setting(state);
            self.editing = false;
        }
    }

    /// Handle custom input for concurrent downloads, retry delay, or custom args
    pub(super) fn handle_custom_input(&mut self, key: KeyEvent, state: &AppState) -> bool {
        match key.code {
            KeyCode::Esc => {
                self.input_mode = false;
                self.editing = false;
                self.validation_error = None;
                true
            }
            KeyCode::Enter => {
                if let Some(selected_setting_idx) = self.list_state.selected() {
                    match selected_setting_idx {
                        IDX_CONCURRENT => {
                            if let Ok(value) = self.custom_input.parse::<usize>()
                                && value > 0
                            {
                                self.settings.concurrent_downloads = value;
                            }
                        }
                        IDX_RATE_LIMIT => {
                            self.settings.rate_limit = self.custom_input.trim().to_string();
                        }
                        IDX_RETRY_DELAY => {
                            if let Ok(value) = self.custom_input.parse::<u64>()
                                && value > 0
                            {
                                self.settings.retry_delay = value;
                            }
                        }
                        IDX_CUSTOM_ARGS => {
                            match Settings::validate_custom_args(&self.custom_input) {
                                Ok(()) => {
                                    self.settings.custom_ytdlp_args = self.custom_input.clone();
                                    self.validation_error = None;
                                }
                                Err(msg) => {
                                    self.validation_error = Some(msg);
                                    return true; // Don't close input mode
                                }
                            }
                        }
                        IDX_DOWNLOAD_DIR => {
                            let trimmed = self.custom_input.trim().to_string();
                            match Settings::validate_download_dir(&trimmed) {
                                Ok(()) => {
                                    self.settings.download_dir = trimmed;
                                    self.validation_error = None;
                                }
                                Err(msg) => {
                                    self.validation_error = Some(msg);
                                    return true; // Don't close input mode
                                }
                            }
                        }
                        _ => {}
                    }
                }

                self.input_mode = false;
                self.editing = false;
                self.validation_error = None;
                self.persist(state);
                true
            }
            KeyCode::Backspace => {
                self.custom_input.pop();
                self.validation_error = None;
                true
            }
            KeyCode::Char(c) => {
                // Paths and yt-dlp flags take any printable character, rate
                // limits also take '.' (e.g., "1.5M"), everything else is numeric
                if let Some(selected) = self.list_state.selected() {
                    if selected == IDX_CUSTOM_ARGS || selected == IDX_DOWNLOAD_DIR {
                        self.custom_input.push(c);
                        self.validation_error = None;
                    } else if selected == IDX_RATE_LIMIT {
                        if c.is_ascii_alphanumeric() || c == '.' {
                            self.custom_input.push(c);
                        }
                    } else if c.is_ascii_digit() {
                        self.custom_input.push(c);
                    }
                }
                true
            }
            _ => false,
        }
    }

    /// Clamp the option index to the range the selected row allows
    pub(super) fn adjust_option_index(&mut self) {
        let is_audio_only = self.is_audio_only();
        if let Some(setting) = self.selected_setting() {
            self.option_index = self
                .option_index
                .min(setting.max_option_index(is_audio_only));
        }
    }
}

#[cfg(test)]
mod tests;
