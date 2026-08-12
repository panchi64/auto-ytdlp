use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
};

use crate::{
    ui::settings_menu::{
        SettingsMenu,
        table::{
            IDX_ADD_METADATA, IDX_ASCII_INDICATORS, IDX_CONCURRENT, IDX_COOKIES_BROWSER,
            IDX_CUSTOM_ARGS, IDX_DOWNLOAD_DIR, IDX_FORMAT_PRESET, IDX_NETWORK_RETRY,
            IDX_OUTPUT_FORMAT, IDX_RATE_LIMIT, IDX_RESET_STATS_ON_BATCH, IDX_RETRY_DELAY,
            IDX_SPONSORBLOCK, IDX_WRITE_SUBTITLES, IDX_WRITE_THUMBNAIL,
        },
    },
    utils::settings::SettingsPreset,
};

impl SettingsMenu {
    /// Render the preset selection popup
    pub(super) fn render_preset_popup(&self, frame: &mut Frame, screen_area: Rect) {
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

        let help_area = Rect::new(popup_x, popup_y + popup_height, popup_width, 1);
        let help = Paragraph::new("↑↓: Select | Enter: Apply | Esc: Cancel")
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(help, help_area);
    }

    /// Render the reset confirmation popup
    pub(super) fn render_reset_confirmation(&self, frame: &mut Frame, screen_area: Rect) {
        let popup_width = 45;
        let popup_height = 5;
        let popup_x = (screen_area.width.saturating_sub(popup_width)) / 2;
        let popup_y = (screen_area.height.saturating_sub(popup_height)) / 2;
        let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

        frame.render_widget(Clear, popup_area);

        let content = vec![
            Line::from("Reset all settings to defaults?"),
            Line::from(Span::styled(
                "The download directory is kept",
                Style::default().fg(Color::DarkGray),
            )),
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
    pub(super) fn render_edit_popup(&self, frame: &mut Frame, screen_area: Rect) {
        if let Some(selected) = self.list_state.selected() {
            let popup_width = 50;
            let popup_height = 3;
            let popup_x = (screen_area.width.saturating_sub(popup_width)) / 2;
            let popup_y = (screen_area.height.saturating_sub(popup_height)) / 2;
            let edit_popup_dialog_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

            frame.render_widget(Clear, edit_popup_dialog_area);

            let (options, title) = self.edit_popup_options(selected);

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

            let help_text = "← →: Change option | Enter: Select | Esc: Cancel";
            let help_popup_area = Rect::new(
                edit_popup_dialog_area.x,
                edit_popup_dialog_area.y + edit_popup_dialog_area.height,
                edit_popup_dialog_area.width,
                1,
            );
            let help_widget =
                Paragraph::new(Text::from(help_text)).style(Style::default().fg(Color::DarkGray));
            frame.render_widget(help_widget, help_popup_area);
        }
    }

    /// Choices and title for the option picker
    ///
    /// The labels are per-row prose, so they stay here rather than in the table.
    fn edit_popup_options(&self, selected: usize) -> (Vec<&'static str>, &'static str) {
        let is_audio_only = self.is_audio_only();

        match selected {
            IDX_FORMAT_PRESET => (
                vec!["Best", "Audio Only", "1080p", "720p", "480p", "360p"],
                "Select Format Preset",
            ),
            IDX_OUTPUT_FORMAT => {
                if is_audio_only {
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
                    (vec!["No"], "Write Subtitles (N/A for Audio)")
                } else {
                    (vec!["No", "Yes"], "Write Subtitles")
                }
            }
            IDX_WRITE_THUMBNAIL => {
                if is_audio_only {
                    // Thumbnails become album art rather than a poster frame
                    (vec!["No", "Yes"], "Write Thumbnail (Album Art)")
                } else {
                    (vec!["No", "Yes"], "Write Thumbnail")
                }
            }
            IDX_ADD_METADATA => (vec!["No", "Yes"], "Add Metadata"),
            IDX_SPONSORBLOCK => (vec!["No", "Yes"], "Remove Sponsor Segments"),
            IDX_CONCURRENT => (vec!["1", "2", "4", "8", "Custom"], "Concurrent Downloads"),
            IDX_RATE_LIMIT => (
                vec!["Unlimited", "500K", "1M", "2M", "5M", "10M", "Custom"],
                "Rate Limit",
            ),
            IDX_NETWORK_RETRY => (vec!["No", "Yes"], "Auto Retry Network Failures"),
            IDX_RETRY_DELAY => (vec!["1", "2", "5", "10", "Custom"], "Retry Delay (seconds)"),
            IDX_COOKIES_BROWSER => (
                vec![
                    "None", "Firefox", "Chrome", "Chromium", "Brave", "Edge", "Opera", "Vivaldi",
                ],
                "Cookies from Browser",
            ),
            IDX_ASCII_INDICATORS => (
                vec!["No", "Yes"],
                "ASCII Indicators (for terminal compatibility)",
            ),
            IDX_RESET_STATS_ON_BATCH => (vec!["No", "Yes"], "Reset Stats When Starting New Batch"),
            _ => (vec![], ""),
        }
    }

    /// Render the input popup for custom values
    pub(super) fn render_input_popup(&self, frame: &mut Frame, screen_area: Rect) {
        let is_custom_args = self.list_state.selected() == Some(IDX_CUSTOM_ARGS);
        let is_rate_limit = self.list_state.selected() == Some(IDX_RATE_LIMIT);
        let is_download_dir = self.list_state.selected() == Some(IDX_DOWNLOAD_DIR);
        let popup_width = if is_custom_args || is_download_dir {
            60
        } else {
            40
        };
        let popup_height = if self.validation_error.is_some() {
            5
        } else {
            3
        };
        let popup_x = (screen_area.width.saturating_sub(popup_width)) / 2;
        let popup_y = (screen_area.height.saturating_sub(popup_height)) / 2;
        let input_popup_dialog_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

        frame.render_widget(Clear, input_popup_dialog_area);

        let title = match self.list_state.selected() {
            Some(IDX_CONCURRENT) => "Enter Concurrent Downloads",
            Some(IDX_RATE_LIMIT) => "Enter Rate Limit (e.g., 750K, 1.5M)",
            Some(IDX_RETRY_DELAY) => "Enter Retry Delay (seconds)",
            Some(IDX_CUSTOM_ARGS) => "Custom yt-dlp Arguments",
            Some(IDX_DOWNLOAD_DIR) => "Download Directory (empty = default)",
            _ => "Enter Value",
        };

        let input_text = format!("{}_", self.custom_input);

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

        let help_text = if is_custom_args {
            "Type arguments | Enter: Save | Esc: Cancel"
        } else if is_download_dir {
            if self.cli_download_dir.is_some() {
                "--download-dir wins this run | Enter: Save | Esc: Cancel"
            } else {
                "e.g., ~/Videos | Enter: Save | Esc: Cancel"
            }
        } else if is_rate_limit {
            "e.g., 500K, 1.5M | Enter: Confirm | Esc: Cancel"
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
}
