mod apply;
mod display;
mod edit;
mod input;
mod render;
mod table;

use ratatui::widgets::ListState;
use std::path::PathBuf;

use crate::{
    app_state::AppState,
    args::DEFAULT_DOWNLOAD_DIR,
    utils::settings::{FormatPreset, Settings},
};
use display::{Keep, truncate_for_display};
use table::{SETTINGS, Setting};

/// Sub-menu state for settings menu
#[derive(Default, Clone, Copy, PartialEq, Debug)]
enum SubMenu {
    #[default]
    None,
    /// Showing preset selection
    PresetSelection,
    /// Showing reset confirmation
    ResetConfirmation,
}

/// Settings menu state
pub struct SettingsMenu {
    list_state: ListState,
    settings: Settings,
    visible: bool,
    editing: bool,
    option_index: usize,
    custom_input: String,
    input_mode: bool,
    /// Current sub-menu state
    sub_menu: SubMenu,
    /// Selected preset index when in preset selection
    preset_index: usize,
    /// Validation error message for custom args
    validation_error: Option<String>,
    /// Download directory forced by `--download-dir` for this run, if any.
    /// The setting is still editable (it applies on the next run without the
    /// flag), but the menu must show which directory is actually in use.
    cli_download_dir: Option<PathBuf>,
}

impl SettingsMenu {
    /// Create a new settings menu
    ///
    /// `cli_download_dir` is the resolved `--download-dir` value, which
    /// overrides the download directory setting for the current run.
    pub fn new(state: &AppState, cli_download_dir: Option<PathBuf>) -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));

        Self {
            list_state,
            settings: state.get_settings().unwrap_or_default(),
            visible: false,
            editing: false,
            option_index: 0,
            custom_input: String::new(),
            input_mode: false,
            cli_download_dir,
            sub_menu: SubMenu::None,
            preset_index: 0,
            validation_error: None,
        }
    }

    pub fn toggle(&mut self) {
        self.visible = !self.visible;
        if self.visible {
            self.editing = false;
            self.input_mode = false;
            self.sub_menu = SubMenu::None;
            self.validation_error = None;
        }
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Value shown for the Download Directory row
    ///
    /// Reports the directory downloads actually go to, so the row never claims
    /// a location that `--download-dir` is overriding.
    fn download_dir_display(&self) -> String {
        if let Some(cli_dir) = &self.cli_download_dir {
            return format!(
                "{} (--download-dir)",
                truncate_for_display(&cli_dir.to_string_lossy(), 20, Keep::End)
            );
        }

        if self.settings.download_dir.is_empty() {
            format!("{} (default)", DEFAULT_DOWNLOAD_DIR)
        } else {
            truncate_for_display(&self.settings.download_dir, 34, Keep::End)
        }
    }

    /// Table row the cursor is on
    fn selected_setting(&self) -> Option<&'static Setting> {
        self.list_state.selected().and_then(|i| SETTINGS.get(i))
    }

    fn is_audio_only(&self) -> bool {
        matches!(self.settings.format_preset, FormatPreset::AudioOnly)
    }

    /// Write the settings to disk and publish them to the shared state
    fn persist(&self, state: &AppState) {
        let _ = self.settings.save();
        let _ = state.update_settings(self.settings.clone());
    }
}

#[cfg(test)]
mod tests;
