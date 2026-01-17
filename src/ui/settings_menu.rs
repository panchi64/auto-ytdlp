use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
};

use crate::{
    app_state::AppState,
    utils::settings::{FormatPreset, OutputFormat, Settings, SettingsPreset},
};

/// Number of regular settings items (before special actions)
const SETTINGS_COUNT: usize = 10;

/// Menu item indices
const IDX_FORMAT_PRESET: usize = 0;
const IDX_OUTPUT_FORMAT: usize = 1;
const IDX_WRITE_SUBTITLES: usize = 2;
const IDX_WRITE_THUMBNAIL: usize = 3;
const IDX_ADD_METADATA: usize = 4;
const IDX_CONCURRENT: usize = 5;
const IDX_NETWORK_RETRY: usize = 6;
const IDX_RETRY_DELAY: usize = 7;
const IDX_ASCII_INDICATORS: usize = 8;
const IDX_CUSTOM_ARGS: usize = 9;
const IDX_APPLY_PRESET: usize = 10;
const IDX_RESET_DEFAULTS: usize = 11;

/// Total number of menu items
const TOTAL_MENU_ITEMS: usize = 12;

/// Descriptions for each setting
const SETTING_DESCRIPTIONS: [&str; TOTAL_MENU_ITEMS] = [
    "Video quality preset - Best downloads highest available quality",
    "Container format - Auto lets yt-dlp choose based on source",
    "Download subtitles if available (disabled for audio-only)",
    "Save video thumbnail as separate image file",
    "Embed metadata (title, artist, etc.) into the file",
    "Number of simultaneous downloads (higher = faster, more bandwidth)",
    "Automatically retry downloads that fail due to network errors",
    "Seconds to wait before retrying a failed download",
    "Use text indicators [OK] instead of emoji for compatibility",
    "Extra yt-dlp flags (e.g., --cookies-from-browser firefox)",
    "Apply a preset configuration for common use cases",
    "Reset all settings to their default values",
];

/// Helper function to create a settings list item with consistent styling
fn create_setting_item<'a>(name: &'a str, value: &'a str) -> ListItem<'a> {
    let style = Style::default().fg(Color::White);
    let value_style = Style::default().fg(Color::Yellow);
    ListItem::new(Line::from(vec![
        Span::styled(name, style),
        Span::raw(": "),
        Span::styled(value, value_style),
    ]))
}

/// Helper to convert bool to Yes/No string
fn bool_to_yes_no(value: bool) -> &'static str {
    if value { "Yes" } else { "No" }
}

/// Helper function to create an action item (Apply Preset, Reset, etc.)
fn create_action_item(name: &str) -> ListItem<'_> {
    ListItem::new(Line::from(vec![Span::styled(
        name,
        Style::default().fg(Color::Cyan),
    )]))
}

/// Sub-menu state for settings menu
#[derive(Default, Clone, Copy, PartialEq)]
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
}

impl SettingsMenu {
    /// Create a new settings menu
    pub fn new(state: &AppState) -> Self {
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
            sub_menu: SubMenu::None,
            preset_index: 0,
            validation_error: None,
        }
    }

    /// Toggle menu visibility
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
        if self.visible {
            self.editing = false;
            self.input_mode = false;
            self.sub_menu = SubMenu::None;
            self.validation_error = None;
        }
    }

    /// Is the menu visible
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Handle keyboard input
    pub fn handle_input(&mut self, key: KeyEvent, state: &AppState) -> bool {
        if !self.visible {
            return false;
        }

        // Handle sub-menus first
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
                // Apply the selected preset
                self.settings = presets[self.preset_index].apply();
                let _ = state.update_settings(self.settings.clone());
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
                // Reset to defaults
                self.settings = Settings::default();
                let _ = self.settings.save();
                let _ = state.update_settings(self.settings.clone());
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
                    match selected_setting_idx {
                        IDX_FORMAT_PRESET => {
                            // Format Preset
                            self.option_index = match self.settings.format_preset {
                                FormatPreset::Best => 0,
                                FormatPreset::AudioOnly => 1,
                                FormatPreset::HD1080p => 2,
                                FormatPreset::HD720p => 3,
                                FormatPreset::SD480p => 4,
                                FormatPreset::SD360p => 5,
                            };
                            self.editing = true;
                        }
                        IDX_OUTPUT_FORMAT => {
                            // Output Format
                            let is_audio_only =
                                matches!(self.settings.format_preset, FormatPreset::AudioOnly);
                            if is_audio_only {
                                self.option_index = match self.settings.output_format {
                                    OutputFormat::Auto => 0,
                                    OutputFormat::MP3 => 1,
                                    OutputFormat::MP4 | OutputFormat::Mkv | OutputFormat::Webm => 0,
                                };
                            } else {
                                self.option_index = match self.settings.output_format {
                                    OutputFormat::Auto => 0,
                                    OutputFormat::MP4 => 1,
                                    OutputFormat::Mkv => 2,
                                    OutputFormat::Webm => 3,
                                    OutputFormat::MP3 => 4,
                                };
                            }
                            self.editing = true;
                        }
                        IDX_WRITE_SUBTITLES => {
                            // Write Subtitles
                            let is_audio_only =
                                matches!(self.settings.format_preset, FormatPreset::AudioOnly);
                            if is_audio_only {
                                self.option_index = 0; // "No" is the only practical option shown
                            } else {
                                self.option_index =
                                    if self.settings.write_subtitles { 1 } else { 0 };
                            }
                            self.editing = true;
                        }
                        IDX_WRITE_THUMBNAIL => {
                            // Write Thumbnail
                            self.option_index = if self.settings.write_thumbnail { 1 } else { 0 };
                            self.editing = true;
                        }
                        IDX_ADD_METADATA => {
                            // Add Metadata
                            self.option_index = if self.settings.add_metadata { 1 } else { 0 };
                            self.editing = true;
                        }
                        IDX_CONCURRENT => {
                            // Concurrent Downloads
                            self.option_index = match self.settings.concurrent_downloads {
                                1 => 0,
                                2 => 1,
                                4 => 2,
                                8 => 3,
                                _ => 4, // Index for "Custom"
                            };
                            self.editing = true;
                        }
                        IDX_NETWORK_RETRY => {
                            // Network Retry
                            self.option_index = if self.settings.network_retry { 1 } else { 0 };
                            self.editing = true;
                        }
                        IDX_RETRY_DELAY => {
                            // Retry Delay
                            self.option_index = match self.settings.retry_delay {
                                1 => 0,
                                2 => 1,
                                5 => 2,
                                10 => 3,
                                _ => 4, // Index for "Custom"
                            };
                            self.editing = true;
                        }
                        IDX_ASCII_INDICATORS => {
                            // ASCII Indicators
                            self.option_index = if self.settings.use_ascii_indicators {
                                1
                            } else {
                                0
                            };
                            self.editing = true;
                        }
                        IDX_CUSTOM_ARGS => {
                            // Custom yt-dlp Arguments - enter text input mode
                            self.custom_input = self.settings.custom_ytdlp_args.clone();
                            self.validation_error = None;
                            self.input_mode = true;
                        }
                        IDX_APPLY_PRESET => {
                            // Apply Preset - open preset selection sub-menu
                            self.preset_index = 0;
                            self.sub_menu = SubMenu::PresetSelection;
                        }
                        IDX_RESET_DEFAULTS => {
                            // Reset to Defaults - show confirmation
                            self.sub_menu = SubMenu::ResetConfirmation;
                        }
                        _ => {
                            self.option_index = 0; // Default for safety
                        }
                    }
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

    /// Check if the current setting is a boolean toggle (Yes/No only)
    fn is_boolean_setting(&self) -> bool {
        if let Some(selected) = self.list_state.selected() {
            // Boolean toggles: subtitles, thumbnail, metadata, network_retry, ascii_indicators
            // Note: subtitles is NOT a toggle when audio-only mode is selected
            let is_audio_only = matches!(self.settings.format_preset, FormatPreset::AudioOnly);
            matches!(selected, IDX_WRITE_SUBTITLES if !is_audio_only)
                || matches!(
                    selected,
                    IDX_WRITE_THUMBNAIL | IDX_ADD_METADATA | IDX_NETWORK_RETRY | IDX_ASCII_INDICATORS
                )
        } else {
            false
        }
    }

    /// Handle input while editing a setting
    fn handle_editing(&mut self, key: KeyEvent, state: &AppState) -> bool {
        match key.code {
            KeyCode::Esc => {
                self.editing = false;
                true
            }
            KeyCode::Left => {
                if self.option_index > 0 {
                    self.option_index -= 1;
                }
                // Auto-apply for boolean toggles
                if self.is_boolean_setting() {
                    self.update_setting(state);
                    self.editing = false;
                }
                true
            }
            KeyCode::Right => {
                self.option_index += 1;
                self.adjust_option_index();
                // Auto-apply for boolean toggles
                if self.is_boolean_setting() {
                    self.update_setting(state);
                    self.editing = false;
                }
                true
            }
            KeyCode::Enter => {
                // Special case for custom concurrent downloads
                if let Some(IDX_CONCURRENT) = self.list_state.selected()
                    && self.option_index == 4
                {
                    // Custom option
                    self.custom_input = self.settings.concurrent_downloads.to_string();
                    self.input_mode = true;
                    return true;
                }

                // Special case for custom retry delay
                if let Some(IDX_RETRY_DELAY) = self.list_state.selected()
                    && self.option_index == 4
                {
                    // Custom option
                    self.custom_input = self.settings.retry_delay.to_string();
                    self.input_mode = true;
                    return true;
                }

                // Regular settings update
                self.update_setting(state);
                self.editing = false;
                true
            }
            _ => false,
        }
    }

    /// Handle custom input for concurrent downloads, retry delay, or custom args
    fn handle_custom_input(&mut self, key: KeyEvent, state: &AppState) -> bool {
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
                            // Custom concurrent downloads
                            if let Ok(value) = self.custom_input.parse::<usize>()
                                && value > 0
                            {
                                self.settings.concurrent_downloads = value;
                            }
                        }
                        IDX_RETRY_DELAY => {
                            // Custom retry delay
                            if let Ok(value) = self.custom_input.parse::<u64>()
                                && value > 0
                            {
                                self.settings.retry_delay = value;
                            }
                        }
                        IDX_CUSTOM_ARGS => {
                            // Custom yt-dlp arguments - validate before accepting
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
                        _ => {}
                    }
                }

                self.input_mode = false;
                self.editing = false;
                self.validation_error = None;

                // Immediately save the updated settings
                let _ = self.settings.save();
                let _ = state.update_settings(self.settings.clone());
                true
            }
            KeyCode::Backspace => {
                self.custom_input.pop();
                // Clear validation error when editing
                self.validation_error = None;
                true
            }
            KeyCode::Char(c) => {
                // For custom args, allow any printable character
                // For numeric fields, only allow digits
                if let Some(selected) = self.list_state.selected() {
                    if selected == IDX_CUSTOM_ARGS {
                        self.custom_input.push(c);
                        self.validation_error = None;
                    } else if c.is_ascii_digit() {
                        self.custom_input.push(c);
                    }
                }
                true
            }
            _ => false,
        }
    }

    /// Adjust option index to valid range based on current setting
    fn adjust_option_index(&mut self) {
        // Max option indices for each setting (0-indexed)
        // Settings: Format, Output, Subtitles, Thumbnail, Metadata, Concurrent, Retry, Delay, ASCII
        // Note: Custom args, Apply Preset, Reset are handled via input_mode/sub_menu, not editing
        const MAX_OPTIONS: [usize; SETTINGS_COUNT] = [5, 4, 1, 1, 1, 4, 1, 4, 1, 0];

        if let Some(i) = self.list_state.selected()
            && i < MAX_OPTIONS.len()
        {
            let is_audio_only = matches!(self.settings.format_preset, FormatPreset::AudioOnly);

            // Handle audio-only special cases
            let max = match (i, is_audio_only) {
                (IDX_OUTPUT_FORMAT, true) => 1, // Output format: only Auto/MP3 for audio
                (IDX_WRITE_SUBTITLES, true) => 0, // Subtitles: disabled for audio-only
                _ => MAX_OPTIONS[i],
            };

            self.option_index = if max == 0 {
                0
            } else {
                self.option_index.min(max)
            };
        }
    }

    /// Update the current setting with the selected option
    fn update_setting(&mut self, state: &AppState) {
        if let Some(selected_setting_idx) = self.list_state.selected() {
            match selected_setting_idx {
                IDX_FORMAT_PRESET => {
                    // Format Preset
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
                    // Output Format
                    let is_audio_only =
                        matches!(self.settings.format_preset, FormatPreset::AudioOnly);
                    self.settings.output_format = if is_audio_only {
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
                IDX_WRITE_SUBTITLES => {
                    // Write subtitles
                    self.settings.write_subtitles = self.option_index == 1;
                }
                IDX_WRITE_THUMBNAIL => {
                    // Write thumbnail
                    self.settings.write_thumbnail = self.option_index == 1;
                }
                IDX_ADD_METADATA => {
                    // Add metadata
                    self.settings.add_metadata = self.option_index == 1;
                }
                IDX_CONCURRENT => {
                    // Concurrent Downloads
                    self.settings.concurrent_downloads = match self.option_index {
                        0 => 1,
                        1 => 2,
                        2 => 4,
                        3 => 8,
                        _ => self.settings.concurrent_downloads,
                    };
                }
                IDX_NETWORK_RETRY => {
                    // Network Retry
                    self.settings.network_retry = self.option_index == 1;
                }
                IDX_RETRY_DELAY => {
                    // Retry Delay
                    self.settings.retry_delay = match self.option_index {
                        0 => 1,
                        1 => 2,
                        2 => 5,
                        3 => 10,
                        _ => self.settings.retry_delay,
                    };
                }
                IDX_ASCII_INDICATORS => {
                    // ASCII Indicators
                    self.settings.use_ascii_indicators = self.option_index == 1;
                }
                _ => {}
            }
        }

        // Reset option index
        self.option_index = 0;

        // Automatically save settings
        let _ = self.settings.save();
        let _ = state.update_settings(self.settings.clone());
    }

    /// Renders the settings menu in a popup
    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        if !self.visible {
            return;
        }

        // Handle sub-menus
        match self.sub_menu {
            SubMenu::PresetSelection => {
                self.render_preset_popup(frame, area);
                return;
            }
            SubMenu::ResetConfirmation => {
                self.render_reset_confirmation(frame, area);
                return;
            }
            SubMenu::None => {}
        }

        if self.input_mode {
            self.render_input_popup(frame, area); // Pass full screen area
        } else if self.editing {
            self.render_edit_popup(frame, area); // Pass full screen area
        } else {
            // Render the main settings dialog (list of settings)
            let popup_width = 65;
            let popup_height = 20;
            let dialog_x = (area.width.saturating_sub(popup_width)) / 2;
            let dialog_y = (area.height.saturating_sub(popup_height)) / 2;
            let main_dialog_area = Rect::new(dialog_x, dialog_y, popup_width, popup_height);

            // Clear the area behind the popup
            frame.render_widget(Clear, main_dialog_area);

            // Pre-compute formatted values that need owned strings
            let concurrent_str = self.settings.concurrent_downloads.to_string();
            let retry_delay_str = format!("{} seconds", self.settings.retry_delay);
            let custom_args_display = if self.settings.custom_ytdlp_args.is_empty() {
                "(none)".to_string()
            } else if self.settings.custom_ytdlp_args.len() > 30 {
                format!("{}...", &self.settings.custom_ytdlp_args[..27])
            } else {
                self.settings.custom_ytdlp_args.clone()
            };

            let mut items = vec![
                create_setting_item(
                    "Format Preset",
                    self.format_preset_to_string(&self.settings.format_preset),
                ),
                create_setting_item(
                    "Output Format",
                    self.output_format_to_string(&self.settings.output_format),
                ),
                create_setting_item(
                    "Write Subtitles",
                    bool_to_yes_no(self.settings.write_subtitles),
                ),
                create_setting_item(
                    "Write Thumbnail",
                    bool_to_yes_no(self.settings.write_thumbnail),
                ),
                create_setting_item("Add Metadata", bool_to_yes_no(self.settings.add_metadata)),
                create_setting_item("Concurrent Downloads", &concurrent_str),
                create_setting_item("Network Retry", bool_to_yes_no(self.settings.network_retry)),
                create_setting_item("Retry Delay", &retry_delay_str),
                create_setting_item(
                    "ASCII Indicators",
                    bool_to_yes_no(self.settings.use_ascii_indicators),
                ),
                create_setting_item("Custom yt-dlp Args", &custom_args_display),
            ];

            // Add action items with different styling
            items.push(create_action_item("Apply Preset..."));
            items.push(create_action_item("Reset to Defaults..."));

            let settings_list = List::new(items)
                .block(
                    Block::default()
                        .title("Settings")
                        .title_style(Style::default().fg(Color::White))
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::White))
                        .style(Style::default()),
                )
                .highlight_style(Style::default().fg(Color::Yellow).bg(Color::DarkGray))
                .highlight_symbol("> ");

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(10),
                    Constraint::Length(2),
                    Constraint::Length(2),
                ])
                .split(main_dialog_area);

            frame.render_stateful_widget(settings_list, chunks[0], &mut self.list_state);

            // Show description for the currently selected item
            if let Some(selected) = self.list_state.selected()
                && selected < SETTING_DESCRIPTIONS.len()
            {
                let description = SETTING_DESCRIPTIONS[selected];
                let desc_widget = Paragraph::new(description)
                    .style(Style::default().fg(Color::Cyan))
                    .wrap(ratatui::widgets::Wrap { trim: true });
                frame.render_widget(desc_widget, chunks[1]);
            }

            let help_text = "↑↓: Navigate | Enter: Edit | Esc: Close";
            let help = Paragraph::new(Text::from(help_text))
                .style(Style::default().fg(Color::DarkGray));
            frame.render_widget(help, chunks[2]);
        }
    }

    /// Render the preset selection popup
    fn render_preset_popup(&self, frame: &mut Frame, screen_area: Rect) {
        let popup_width = 55;
        let popup_height = 10;
        let popup_x = (screen_area.width.saturating_sub(popup_width)) / 2;
        let popup_y = (screen_area.height.saturating_sub(popup_height)) / 2;
        let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

        frame.render_widget(Clear, popup_area);

        let presets = SettingsPreset::all();
        let items: Vec<ListItem> = presets
            .iter()
            .enumerate()
            .map(|(i, preset)| {
                let style = if i == self.preset_index {
                    Style::default().fg(Color::Yellow).bg(Color::DarkGray)
                } else {
                    Style::default().fg(Color::White)
                };
                ListItem::new(Line::from(vec![
                    Span::styled(preset.name(), style),
                    Span::raw(" - "),
                    Span::styled(preset.description(), Style::default().fg(Color::Gray)),
                ]))
            })
            .collect();

        let preset_list = List::new(items).block(
            Block::default()
                .title("Select Preset")
                .title_style(Style::default().fg(Color::White))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        );
        frame.render_widget(preset_list, popup_area);

        // Help text below
        let help_area = Rect::new(popup_x, popup_y + popup_height, popup_width, 1);
        let help = Paragraph::new("↑↓: Select | Enter: Apply | Esc: Cancel")
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(help, help_area);
    }

    /// Render the reset confirmation popup
    fn render_reset_confirmation(&self, frame: &mut Frame, screen_area: Rect) {
        let popup_width = 45;
        let popup_height = 5;
        let popup_x = (screen_area.width.saturating_sub(popup_width)) / 2;
        let popup_y = (screen_area.height.saturating_sub(popup_height)) / 2;
        let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

        frame.render_widget(Clear, popup_area);

        let content = vec![
            Line::from(""),
            Line::from("Reset all settings to defaults?"),
            Line::from(""),
            Line::from(vec![
                Span::styled("Y", Style::default().fg(Color::Green)),
                Span::raw(": Yes  "),
                Span::styled("N", Style::default().fg(Color::Red)),
                Span::raw("/"),
                Span::styled("Esc", Style::default().fg(Color::Red)),
                Span::raw(": Cancel"),
            ]),
        ];

        let confirm_widget = Paragraph::new(content)
            .block(
                Block::default()
                    .title("Confirm Reset")
                    .title_style(Style::default().fg(Color::Yellow))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow)),
            )
            .alignment(ratatui::layout::Alignment::Center);
        frame.render_widget(confirm_widget, popup_area);
    }

    /// Render the editing popup for the selected setting
    fn render_edit_popup(&mut self, frame: &mut Frame, screen_area: Rect) {
        if let Some(selected) = self.list_state.selected() {
            let popup_width = 50;
            let popup_height = 3;
            let popup_x = (screen_area.width.saturating_sub(popup_width)) / 2;
            let popup_y = (screen_area.height.saturating_sub(popup_height)) / 2;
            let edit_popup_dialog_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

            // Clear the area behind the popup
            frame.render_widget(Clear, edit_popup_dialog_area);

            let is_audio_only = matches!(self.settings.format_preset, FormatPreset::AudioOnly);

            let (options, title) = match selected {
                IDX_FORMAT_PRESET => (
                    vec!["Best", "Audio Only", "1080p", "720p", "480p", "360p"],
                    "Select Format Preset",
                ),
                IDX_OUTPUT_FORMAT => {
                    if is_audio_only {
                        // Only show audio-compatible formats when Audio Only is selected
                        (vec!["Auto", "MP3"], "Select Output Format")
                    } else {
                        (
                            vec!["Auto", "MP4", "MKV", "WEBM", "MP3 (audio only)"],
                            "Select Output Format",
                        )
                    }
                }
                IDX_WRITE_SUBTITLES => {
                    if is_audio_only {
                        // Subtitles are not applicable for audio-only
                        (vec!["No"], "Write Subtitles (N/A for Audio)")
                    } else {
                        (vec!["No", "Yes"], "Write Subtitles")
                    }
                }
                IDX_WRITE_THUMBNAIL => {
                    if is_audio_only {
                        // Thumbnails are less relevant for audio-only
                        (vec!["No", "Yes"], "Write Thumbnail (Album Art)")
                    } else {
                        (vec!["No", "Yes"], "Write Thumbnail")
                    }
                }
                IDX_ADD_METADATA => (vec!["No", "Yes"], "Add Metadata"),
                IDX_CONCURRENT => (vec!["1", "2", "4", "8", "Custom"], "Concurrent Downloads"),
                IDX_NETWORK_RETRY => (vec!["No", "Yes"], "Auto Retry Network Failures"),
                IDX_RETRY_DELAY => (vec!["1", "2", "5", "10", "Custom"], "Retry Delay (seconds)"),
                IDX_ASCII_INDICATORS => (
                    vec!["No", "Yes"],
                    "ASCII Indicators (for terminal compatibility)",
                ),
                _ => (vec![], ""),
            };

            let mut spans = Vec::new();
            for (i, option) in options.iter().enumerate() {
                let style = if i == self.option_index {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default().fg(Color::White)
                };
                spans.push(Span::styled(option.to_string(), style));
                if i < options.len() - 1 {
                    spans.push(Span::raw(" | "));
                }
            }

            let options_widget = Paragraph::new(Line::from(spans)).block(
                Block::default()
                    .title(title)
                    .title_style(Style::default().fg(Color::White))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::White))
                    .style(Style::default()),
            );
            frame.render_widget(options_widget, edit_popup_dialog_area);

            // Help text for this popup
            let help_text = "← →: Change option | Enter: Select | Esc: Cancel";
            let help_popup_area = Rect::new(
                edit_popup_dialog_area.x,
                edit_popup_dialog_area.y + edit_popup_dialog_area.height,
                edit_popup_dialog_area.width,
                1,
            );
            let help_widget =
                Paragraph::new(Text::from(help_text)).style(Style::default().fg(Color::DarkGray)); // Simple text, no block
            frame.render_widget(help_widget, help_popup_area);
        }
    }

    /// Render the input popup for custom values
    fn render_input_popup(&mut self, frame: &mut Frame, screen_area: Rect) {
        let is_custom_args = self.list_state.selected() == Some(IDX_CUSTOM_ARGS);
        let popup_width = if is_custom_args { 60 } else { 40 };
        let popup_height = if self.validation_error.is_some() { 5 } else { 3 };
        let popup_x = (screen_area.width.saturating_sub(popup_width)) / 2;
        let popup_y = (screen_area.height.saturating_sub(popup_height)) / 2;
        let input_popup_dialog_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

        // Clear the area behind the popup
        frame.render_widget(Clear, input_popup_dialog_area);

        // Dynamic title based on which setting is being edited
        let title = match self.list_state.selected() {
            Some(IDX_CONCURRENT) => "Enter Concurrent Downloads",
            Some(IDX_RETRY_DELAY) => "Enter Retry Delay (seconds)",
            Some(IDX_CUSTOM_ARGS) => "Custom yt-dlp Arguments",
            _ => "Enter Value",
        };

        let input_text = format!("{}_", self.custom_input);

        // Build content with optional error message
        let mut lines = vec![Line::from(Span::styled(
            input_text,
            Style::default().fg(Color::Yellow),
        ))];

        if let Some(ref error) = self.validation_error {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!("Error: {}", error),
                Style::default().fg(Color::Red),
            )));
        }

        let input_widget = Paragraph::new(lines).block(
            Block::default()
                .title(title)
                .title_style(Style::default().fg(Color::White))
                .borders(Borders::ALL)
                .border_style(if self.validation_error.is_some() {
                    Style::default().fg(Color::Red)
                } else {
                    Style::default().fg(Color::White)
                }),
        );
        frame.render_widget(input_widget, input_popup_dialog_area);

        // Help text for this popup
        let help_text = if is_custom_args {
            "Type arguments | Enter: Save | Esc: Cancel"
        } else {
            "Enter a number | Enter: Confirm | Esc: Cancel"
        };
        let help_popup_area = Rect::new(
            input_popup_dialog_area.x,
            input_popup_dialog_area.y + input_popup_dialog_area.height,
            input_popup_dialog_area.width,
            1,
        );
        let help_widget =
            Paragraph::new(Text::from(help_text)).style(Style::default().fg(Color::DarkGray));
        frame.render_widget(help_widget, help_popup_area);
    }

    /// Convert format preset to display string
    ///
    /// Returns a static string reference to avoid allocations.
    fn format_preset_to_string(&self, preset: &FormatPreset) -> &'static str {
        match preset {
            FormatPreset::Best => "Best",
            FormatPreset::AudioOnly => "Audio Only",
            FormatPreset::HD1080p => "1080p",
            FormatPreset::HD720p => "720p",
            FormatPreset::SD480p => "480p",
            FormatPreset::SD360p => "360p",
        }
    }

    /// Convert output format to display string
    ///
    /// Returns a static string reference to avoid allocations.
    fn output_format_to_string(&self, format: &OutputFormat) -> &'static str {
        match format {
            OutputFormat::Auto => "Auto",
            OutputFormat::MP4 => "MP4",
            OutputFormat::Mkv => "MKV",
            OutputFormat::MP3 => {
                if matches!(self.settings.format_preset, FormatPreset::AudioOnly) {
                    "MP3 (audio)"
                } else {
                    "MP3 (audio only)"
                }
            }
            OutputFormat::Webm => "WEBM",
        }
    }
}
