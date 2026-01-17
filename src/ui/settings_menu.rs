use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};

use crate::{
    app_state::AppState,
    utils::settings::{FormatPreset, OutputFormat, Settings},
};

/// Settings menu state
pub struct SettingsMenu {
    list_state: ListState,
    settings: Settings,
    visible: bool,
    editing: bool,
    option_index: usize,
    custom_input: String,
    input_mode: bool,
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
        }
    }

    /// Toggle menu visibility
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
        if self.visible {
            self.editing = false;
            self.input_mode = false;
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

        if self.input_mode {
            self.handle_custom_input(key, state)
        } else if self.editing {
            self.handle_editing(key, state)
        } else {
            self.handle_menu_navigation(key, state)
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
                        0 => {
                            // Format Preset
                            self.option_index = match self.settings.format_preset {
                                FormatPreset::Best => 0,
                                FormatPreset::AudioOnly => 1,
                                FormatPreset::HD1080p => 2,
                                FormatPreset::HD720p => 3,
                                FormatPreset::SD480p => 4,
                                FormatPreset::SD360p => 5,
                            };
                        }
                        1 => {
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
                        }
                        2 => {
                            // Write Subtitles
                            let is_audio_only =
                                matches!(self.settings.format_preset, FormatPreset::AudioOnly);
                            if is_audio_only {
                                self.option_index = 0; // "No" is the only practical option shown
                            } else {
                                self.option_index =
                                    if self.settings.write_subtitles { 1 } else { 0 };
                            }
                        }
                        3 => {
                            // Write Thumbnail
                            self.option_index = if self.settings.write_thumbnail { 1 } else { 0 };
                        }
                        4 => {
                            // Add Metadata
                            self.option_index = if self.settings.add_metadata { 1 } else { 0 };
                        }
                        5 => {
                            // Concurrent Downloads
                            self.option_index = match self.settings.concurrent_downloads {
                                1 => 0,
                                2 => 1,
                                4 => 2,
                                8 => 3,
                                _ => 4, // Index for "Custom"
                            };
                        }
                        6 => {
                            // Network Retry
                            self.option_index = if self.settings.network_retry { 1 } else { 0 };
                        }
                        7 => {
                            // Retry Delay
                            self.option_index = match self.settings.retry_delay {
                                1 => 0,
                                2 => 1,
                                5 => 2,
                                10 => 3,
                                _ => 4, // Index for "Custom"
                            };
                        }
                        8 => {
                            // ASCII Indicators
                            self.option_index = if self.settings.use_ascii_indicators { 1 } else { 0 };
                        }
                        _ => {
                            self.option_index = 0; // Default for safety
                        }
                    }
                } else {
                    // Should not happen if a list item is selected, but set a safe default.
                    self.option_index = 0;
                }
                self.editing = true;
                true
            }
            KeyCode::Up => {
                if let Some(i) = self.list_state.selected() {
                    if i > 0 {
                        self.list_state.select(Some(i - 1));
                    }
                }
                true
            }
            KeyCode::Down => {
                if let Some(i) = self.list_state.selected() {
                    if i < 8 {
                        // Number of settings options - 1 (increased to 8 for ascii_indicators)
                        self.list_state.select(Some(i + 1));
                    }
                }
                true
            }
            _ => false,
        }
    }

    /// Check if the current setting is a boolean toggle (Yes/No only)
    fn is_boolean_setting(&self) -> bool {
        if let Some(selected) = self.list_state.selected() {
            // Indices 2, 3, 4, 6, 8 are boolean toggles (subtitles, thumbnail, metadata, network_retry, ascii_indicators)
            // Note: subtitles (2) is NOT a toggle when audio-only mode is selected
            let is_audio_only = matches!(self.settings.format_preset, FormatPreset::AudioOnly);
            matches!(selected, 2 if !is_audio_only) || matches!(selected, 3 | 4 | 6 | 8)
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
                if let Some(5) = self.list_state.selected() {
                    if self.option_index == 4 {
                        // Custom option
                        self.custom_input = self.settings.concurrent_downloads.to_string();
                        self.input_mode = true;
                        return true;
                    }
                }

                // Special case for custom retry delay
                if let Some(7) = self.list_state.selected() {
                    if self.option_index == 4 {
                        // Custom option
                        self.custom_input = self.settings.retry_delay.to_string();
                        self.input_mode = true;
                        return true;
                    }
                }

                // Regular settings update
                self.update_setting(state);
                self.editing = false;
                true
            }
            _ => false,
        }
    }

    /// Handle custom input for concurrent downloads or retry delay
    fn handle_custom_input(&mut self, key: KeyEvent, state: &AppState) -> bool {
        match key.code {
            KeyCode::Esc => {
                self.input_mode = false;
                self.editing = false;
                true
            }
            KeyCode::Enter => {
                if let Some(selected_setting_idx) = self.list_state.selected() {
                    match selected_setting_idx {
                        5 => {
                            // Custom concurrent downloads
                            if let Ok(value) = self.custom_input.parse::<usize>() {
                                if value > 0 {
                                    self.settings.concurrent_downloads = value;
                                }
                            }
                        }
                        7 => {
                            // Custom retry delay
                            if let Ok(value) = self.custom_input.parse::<u64>() {
                                if value > 0 {
                                    self.settings.retry_delay = value;
                                }
                            }
                        }
                        _ => {}
                    }
                }

                self.input_mode = false;
                self.editing = false;

                // Immediately save the updated settings
                let _ = state.update_settings(self.settings.clone());
                true
            }
            KeyCode::Backspace => {
                self.custom_input.pop();
                true
            }
            KeyCode::Char(c) if c.is_ascii_digit() => {
                self.custom_input.push(c);
                true
            }
            _ => false,
        }
    }

    /// Adjust option index to valid range based on current setting
    fn adjust_option_index(&mut self) {
        if let Some(i) = self.list_state.selected() {
            let is_audio_only = matches!(self.settings.format_preset, FormatPreset::AudioOnly);

            match i {
                0 => {
                    // Format preset options
                    self.option_index = self.option_index.min(5); // 6 options now (0-5)
                }
                1 => {
                    // Output format options
                    if is_audio_only {
                        self.option_index = self.option_index.min(1); // 2 options for audio-only
                    } else {
                        self.option_index = self.option_index.min(4); // 5 options for video
                    }
                }
                2 => {
                    // Subtitles options
                    if is_audio_only {
                        self.option_index = 0; // Only "No" option for audio-only
                    } else {
                        self.option_index = self.option_index.min(1); // 2 options for video
                    }
                }
                3..=4 => {
                    // Thumbnail and metadata options
                    self.option_index = self.option_index.min(1); // 2 options (true/false)
                }
                5 => {
                    // Concurrent downloads (1, 2, 4, 8, Custom)
                    self.option_index = self.option_index.min(4); // 5 options
                }
                6 => {
                    // Network retry options (true/false)
                    self.option_index = self.option_index.min(1); // 2 options (true/false)
                }
                7 => {
                    // Retry delay options (1, 2, 5, 10 seconds, Custom)
                    self.option_index = self.option_index.min(4); // 5 options
                }
                8 => {
                    // ASCII indicators options (true/false)
                    self.option_index = self.option_index.min(1); // 2 options (true/false)
                }
                _ => {}
            }
        }
    }

    /// Update the current setting with the selected option
    fn update_setting(&mut self, state: &AppState) {
        if let Some(selected_setting_idx) = self.list_state.selected() {
            match selected_setting_idx {
                0 => {
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
                1 => {
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
                2 => {
                    // Write subtitles
                    self.settings.write_subtitles = self.option_index == 1;
                }
                3 => {
                    // Write thumbnail
                    self.settings.write_thumbnail = self.option_index == 1;
                }
                4 => {
                    // Add metadata
                    self.settings.add_metadata = self.option_index == 1;
                }
                5 => {
                    // Concurrent Downloads
                    self.settings.concurrent_downloads = match self.option_index {
                        0 => 1,
                        1 => 2,
                        2 => 4,
                        3 => 8,
                        _ => self.settings.concurrent_downloads,
                    };
                }
                6 => {
                    // Network Retry
                    self.settings.network_retry = self.option_index == 1;
                }
                7 => {
                    // Retry Delay
                    self.settings.retry_delay = match self.option_index {
                        0 => 1,
                        1 => 2,
                        2 => 5,
                        3 => 10,
                        _ => self.settings.retry_delay,
                    };
                }
                8 => {
                    // ASCII Indicators
                    self.settings.use_ascii_indicators = self.option_index == 1;
                }
                _ => {}
            }
        }

        // Reset option index
        self.option_index = 0;

        // Automatically save settings
        let _ = state.update_settings(self.settings.clone());
    }

    /// Renders the settings menu in a popup
    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        if !self.visible {
            return;
        }

        if self.input_mode {
            self.render_input_popup(frame, area); // Pass full screen area
        } else if self.editing {
            self.render_edit_popup(frame, area); // Pass full screen area
        } else {
            // Render the main settings dialog (list of settings)
            let popup_width = 60;
            let popup_height = 16;
            let dialog_x = (area.width.saturating_sub(popup_width)) / 2;
            let dialog_y = (area.height.saturating_sub(popup_height)) / 2;
            let main_dialog_area = Rect::new(dialog_x, dialog_y, popup_width, popup_height);

            let style = Style::default().fg(Color::White);
            let value_style = Style::default().fg(Color::Yellow);

            let items = vec![
                ListItem::new(Line::from(vec![
                    Span::styled("Format Preset", style),
                    Span::raw(": "),
                    Span::styled(
                        self.format_preset_to_string(&self.settings.format_preset),
                        value_style,
                    ),
                ])),
                ListItem::new(Line::from(vec![
                    Span::styled("Output Format", style),
                    Span::raw(": "),
                    Span::styled(
                        self.output_format_to_string(&self.settings.output_format),
                        value_style,
                    ),
                ])),
                ListItem::new(Line::from(vec![
                    Span::styled("Write Subtitles", style),
                    Span::raw(": "),
                    Span::styled(
                        if self.settings.write_subtitles {
                            "Yes"
                        } else {
                            "No"
                        },
                        value_style,
                    ),
                ])),
                ListItem::new(Line::from(vec![
                    Span::styled("Write Thumbnail", style),
                    Span::raw(": "),
                    Span::styled(
                        if self.settings.write_thumbnail {
                            "Yes"
                        } else {
                            "No"
                        },
                        value_style,
                    ),
                ])),
                ListItem::new(Line::from(vec![
                    Span::styled("Add Metadata", style),
                    Span::raw(": "),
                    Span::styled(
                        if self.settings.add_metadata {
                            "Yes"
                        } else {
                            "No"
                        },
                        value_style,
                    ),
                ])),
                ListItem::new(Line::from(vec![
                    Span::styled("Concurrent Downloads", style),
                    Span::raw(": "),
                    Span::styled(self.settings.concurrent_downloads.to_string(), value_style),
                ])),
                ListItem::new(Line::from(vec![
                    Span::styled("Network Retry", style),
                    Span::raw(": "),
                    Span::styled(
                        if self.settings.network_retry {
                            "Yes"
                        } else {
                            "No"
                        },
                        value_style,
                    ),
                ])),
                ListItem::new(Line::from(vec![
                    Span::styled("Retry Delay", style),
                    Span::raw(": "),
                    Span::styled(
                        format!("{} seconds", self.settings.retry_delay),
                        value_style,
                    ),
                ])),
                ListItem::new(Line::from(vec![
                    Span::styled("ASCII Indicators", style),
                    Span::raw(": "),
                    Span::styled(
                        if self.settings.use_ascii_indicators {
                            "Yes"
                        } else {
                            "No"
                        },
                        value_style,
                    ),
                ])),
            ];

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
                .constraints([Constraint::Min(8), Constraint::Length(3)].as_ref())
                .split(main_dialog_area);

            frame.render_stateful_widget(settings_list, chunks[0], &mut self.list_state);

            let help_text = "↑↓: Navigate | Enter: Edit | Esc: Close";
            let help = Paragraph::new(Text::from(help_text))
                .block(
                    Block::default()
                        .borders(Borders::TOP)
                        .border_style(Style::default().fg(Color::White))
                        .style(Style::default()),
                )
                .style(Style::default().fg(Color::Gray));
            frame.render_widget(help, chunks[1]);
        }
    }

    /// Render the editing popup for the selected setting
    fn render_edit_popup(&mut self, frame: &mut Frame, screen_area: Rect) {
        if let Some(selected) = self.list_state.selected() {
            let popup_width = 50;
            let popup_height = 3;
            let popup_x = (screen_area.width.saturating_sub(popup_width)) / 2;
            let popup_y = (screen_area.height.saturating_sub(popup_height)) / 2;
            let edit_popup_dialog_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

            let is_audio_only = matches!(self.settings.format_preset, FormatPreset::AudioOnly);

            let (options, title) = match selected {
                0 => (
                    vec!["Best", "Audio Only", "1080p", "720p", "480p", "360p"],
                    "Select Format Preset",
                ),
                1 => {
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
                2 => {
                    if is_audio_only {
                        // Subtitles are not applicable for audio-only
                        (vec!["No"], "Write Subtitles (N/A for Audio)")
                    } else {
                        (vec!["No", "Yes"], "Write Subtitles")
                    }
                }
                3 => {
                    if is_audio_only {
                        // Thumbnails are less relevant for audio-only
                        (vec!["No", "Yes"], "Write Thumbnail (Album Art)")
                    } else {
                        (vec!["No", "Yes"], "Write Thumbnail")
                    }
                }
                4 => (vec!["No", "Yes"], "Add Metadata"),
                5 => (vec!["1", "2", "4", "8", "Custom"], "Concurrent Downloads"),
                6 => (vec!["No", "Yes"], "Auto Retry Network Failures"),
                7 => (vec!["1", "2", "5", "10", "Custom"], "Retry Delay"),
                8 => (vec!["No", "Yes"], "ASCII Indicators (for terminal compatibility)"),
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
        let popup_width = 40;
        let popup_height = 3;
        let popup_x = (screen_area.width.saturating_sub(popup_width)) / 2;
        let popup_y = (screen_area.height.saturating_sub(popup_height)) / 2;
        let input_popup_dialog_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

        // Dynamic title based on which setting is being edited
        let title = match self.list_state.selected() {
            Some(5) => "Enter Concurrent Downloads",
            Some(7) => "Enter Retry Delay (seconds)",
            _ => "Enter Value",
        };

        let input_text = format!("{}_", self.custom_input);
        let input_widget = Paragraph::new(Text::from(input_text))
            .block(
                Block::default()
                    .title(title)
                    .title_style(Style::default().fg(Color::White))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::White))
                    .style(Style::default()),
            )
            .style(Style::default().fg(Color::Yellow));
        frame.render_widget(input_widget, input_popup_dialog_area);

        // Help text for this popup
        let help_text = "Enter a number | Enter: Confirm | Esc: Cancel";
        let help_popup_area = Rect::new(
            input_popup_dialog_area.x,
            input_popup_dialog_area.y + input_popup_dialog_area.height,
            input_popup_dialog_area.width,
            1,
        );
        let help_widget =
            Paragraph::new(Text::from(help_text)).style(Style::default().fg(Color::DarkGray)); // Simple text, no block
        frame.render_widget(help_widget, help_popup_area);
    }

    /// Convert format preset to display string
    fn format_preset_to_string(&self, preset: &FormatPreset) -> String {
        match preset {
            FormatPreset::Best => "Best".to_string(),
            FormatPreset::AudioOnly => "Audio Only".to_string(),
            FormatPreset::HD1080p => "1080p".to_string(),
            FormatPreset::HD720p => "720p".to_string(),
            FormatPreset::SD480p => "480p".to_string(),
            FormatPreset::SD360p => "360p".to_string(),
        }
    }

    /// Convert output format to display string
    fn output_format_to_string(&self, format: &OutputFormat) -> String {
        match format {
            OutputFormat::Auto => "Auto".to_string(),
            OutputFormat::MP4 => "MP4".to_string(),
            OutputFormat::Mkv => "MKV".to_string(),
            OutputFormat::MP3 => {
                if matches!(self.settings.format_preset, FormatPreset::AudioOnly) {
                    "MP3 (audio)".to_string()
                } else {
                    "MP3 (audio only)".to_string()
                }
            }
            OutputFormat::Webm => "WEBM".to_string(),
        }
    }
}
