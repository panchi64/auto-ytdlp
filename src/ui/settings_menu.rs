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
            settings: state.get_settings(),
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
                    if i < 5 {
                        // Number of settings options - 1
                        self.list_state.select(Some(i + 1));
                    }
                }
                true
            }
            _ => false,
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
                true
            }
            KeyCode::Right => {
                self.option_index += 1;
                self.adjust_option_index();
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

                // Regular settings update
                self.update_setting(state);
                self.editing = false;
                true
            }
            _ => false,
        }
    }

    /// Handle custom input for concurrent downloads
    fn handle_custom_input(&mut self, key: KeyEvent, state: &AppState) -> bool {
        match key.code {
            KeyCode::Esc => {
                self.input_mode = false;
                self.editing = false;
                true
            }
            KeyCode::Enter => {
                // Try to parse the input as a number
                if let Ok(value) = self.custom_input.parse::<usize>() {
                    if value > 0 {
                        self.settings.concurrent_downloads = value;
                        self.input_mode = false;
                        self.editing = false;

                        // Immediately save the updated settings
                        state.update_settings(self.settings.clone());
                    }
                }
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
                    self.option_index = self.option_index.min(6); // 7 options
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
                _ => {}
            }
        }
    }

    /// Update the current setting with the selected option
    fn update_setting(&mut self, state: &AppState) {
        if let Some(i) = self.list_state.selected() {
            match i {
                0 => {
                    // Format preset
                    let new_preset = match self.option_index {
                        0 => FormatPreset::Best,
                        1 => FormatPreset::AudioOnly,
                        2 => FormatPreset::HD1080p,
                        3 => FormatPreset::HD720p,
                        4 => FormatPreset::SD480p,
                        5 => FormatPreset::SD360p,
                        6 => FormatPreset::Custom("bestvideo*+bestaudio/best".to_string()),
                        _ => FormatPreset::Best,
                    };

                    // If switching to Audio Only, auto-select MP3 format
                    if matches!(new_preset, FormatPreset::AudioOnly) {
                        self.settings.output_format = OutputFormat::MP3;
                        // Disable subtitles for audio-only
                        self.settings.write_subtitles = false;
                    }

                    self.settings.format_preset = new_preset;
                }
                1 => {
                    // Output format
                    let is_audio_only =
                        matches!(self.settings.format_preset, FormatPreset::AudioOnly);

                    if is_audio_only {
                        // Only allow audio formats when in audio-only mode
                        self.settings.output_format = match self.option_index {
                            0 => OutputFormat::Auto,
                            1 => OutputFormat::MP3,
                            _ => OutputFormat::Auto,
                        };
                    } else {
                        self.settings.output_format = match self.option_index {
                            0 => OutputFormat::Auto,
                            1 => OutputFormat::MP4,
                            2 => OutputFormat::Mkv,
                            3 => OutputFormat::MP3,
                            4 => OutputFormat::Webm,
                            _ => OutputFormat::Auto,
                        };
                    }
                }
                2 => {
                    // Write subtitles
                    let is_audio_only =
                        matches!(self.settings.format_preset, FormatPreset::AudioOnly);

                    if !is_audio_only {
                        self.settings.write_subtitles = self.option_index == 1;
                    } else {
                        // Subtitles don't apply to audio-only
                        self.settings.write_subtitles = false;
                    }
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
                    // Concurrent downloads
                    self.settings.concurrent_downloads = match self.option_index {
                        0 => 1,
                        1 => 2,
                        2 => 4,
                        3 => 8,
                        // Custom option is handled separately in handle_custom_input
                        _ => self.settings.concurrent_downloads,
                    };
                }
                _ => {}
            }
        }

        // Reset option index
        self.option_index = 0;

        // Automatically save settings
        state.update_settings(self.settings.clone());
    }

    /// Renders the settings menu in a popup
    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        if !self.visible {
            return;
        }

        // Draw a solid black background to completely cover the entire screen
        frame.render_widget(
            Block::default()
                .style(Style::default().bg(Color::Black))
                .borders(Borders::NONE),
            area, // Use the full screen area
        );

        if self.input_mode {
            self.render_input_popup(frame, area); // Pass full screen area
        } else if self.editing {
            self.render_edit_popup(frame, area); // Pass full screen area
        } else {
            // Render the main settings dialog (list of settings)
            let popup_width = 60;
            let popup_height = 15;
            let dialog_x = (area.width.saturating_sub(popup_width)) / 2;
            let dialog_y = (area.height.saturating_sub(popup_height)) / 2;
            let main_dialog_area = Rect::new(dialog_x, dialog_y, popup_width, popup_height);

            let settings_items = [
                format!(
                    "Format Preset: {}",
                    self.format_preset_to_string(&self.settings.format_preset)
                ),
                format!(
                    "Output Format: {}",
                    self.output_format_to_string(&self.settings.output_format)
                ),
                format!(
                    "Write Subtitles: {}",
                    if self.settings.write_subtitles {
                        "Yes"
                    } else {
                        "No"
                    }
                ),
                format!(
                    "Write Thumbnail: {}",
                    if self.settings.write_thumbnail {
                        "Yes"
                    } else {
                        "No"
                    }
                ),
                format!(
                    "Add Metadata: {}",
                    if self.settings.add_metadata {
                        "Yes"
                    } else {
                        "No"
                    }
                ),
                format!(
                    "Concurrent Downloads: {}",
                    self.settings.concurrent_downloads
                ),
            ]
            .iter()
            .map(|i| ListItem::new(i.clone()))
            .collect::<Vec<ListItem>>();

            let settings_list = List::new(settings_items)
                .block(
                    Block::default()
                        .title("Settings")
                        .title_style(Style::default().fg(Color::White))
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::White))
                        .style(Style::default().bg(Color::Black)),
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
                        .style(Style::default().bg(Color::Black)),
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
                    vec![
                        "Best",
                        "Audio Only",
                        "1080p",
                        "720p",
                        "480p",
                        "360p",
                        "Custom",
                    ],
                    "Select Format Preset",
                ),
                1 => {
                    if is_audio_only {
                        // Only show audio-compatible formats when Audio Only is selected
                        (vec!["Auto", "MP3"], "Select Output Format")
                    } else {
                        (
                            vec!["Auto", "MP4", "MKV", "WEBM", "MP3"],
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
                    .style(Style::default().bg(Color::Black)),
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

        let input_text = format!("{}_", self.custom_input);
        let input_widget = Paragraph::new(Text::from(input_text))
            .block(
                Block::default()
                    .title("Enter Concurrent Downloads")
                    .title_style(Style::default().fg(Color::White))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::White))
                    .style(Style::default().bg(Color::Black)),
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
            FormatPreset::Custom(s) => format!("Custom ({})", s),
        }
    }

    /// Convert output format to display string
    fn output_format_to_string(&self, format: &OutputFormat) -> String {
        match format {
            OutputFormat::Auto => "Auto".to_string(),
            OutputFormat::MP4 => "MP4".to_string(),
            OutputFormat::Mkv => "MKV".to_string(),
            OutputFormat::MP3 => "MP3 (audio)".to_string(),
            OutputFormat::Webm => "WEBM".to_string(),
        }
    }
}
