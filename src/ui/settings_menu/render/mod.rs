mod popups;

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::Text,
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
};

use super::{
    SettingsMenu, SubMenu,
    display::{
        Keep, bool_to_yes_no, create_action_item, create_setting_item, truncate_for_display,
    },
    table::{SETTINGS, SETTINGS_COUNT},
};

impl SettingsMenu {
    /// Renders the settings menu in a popup
    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        if !self.visible {
            return;
        }

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
            self.render_input_popup(frame, area);
        } else if self.editing {
            self.render_edit_popup(frame, area);
        } else {
            self.render_settings_list(frame, area);
        }
    }

    fn render_settings_list(&mut self, frame: &mut Frame, area: Rect) {
        let popup_width = 65;
        let popup_height = 24;
        let dialog_x = (area.width.saturating_sub(popup_width)) / 2;
        let dialog_y = (area.height.saturating_sub(popup_height)) / 2;
        let main_dialog_area = Rect::new(dialog_x, dialog_y, popup_width, popup_height);

        frame.render_widget(Clear, main_dialog_area);

        let values = self.setting_values();
        let mut items: Vec<ListItem> = SETTINGS[..SETTINGS_COUNT]
            .iter()
            .zip(values.iter())
            .map(|(setting, value)| create_setting_item(setting.label, value))
            .collect();
        items.extend(
            SETTINGS[SETTINGS_COUNT..]
                .iter()
                .map(|setting| create_action_item(setting.label)),
        );

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

        if let Some(setting) = self.selected_setting() {
            let desc_widget = Paragraph::new(setting.description)
                .style(Style::default().fg(Color::Cyan))
                .wrap(ratatui::widgets::Wrap { trim: true });
            frame.render_widget(desc_widget, chunks[1]);
        }

        let help_text = "↑↓: Navigate | Enter: Edit | Esc: Close";
        let help =
            Paragraph::new(Text::from(help_text)).style(Style::default().fg(Color::DarkGray));
        frame.render_widget(help, chunks[2]);
    }

    /// Value column for each editable row, in table order
    fn setting_values(&self) -> [String; SETTINGS_COUNT] {
        let cookies_display = if self.settings.cookies_from_browser.is_empty() {
            "None".to_string()
        } else {
            // Capitalize first letter for display
            let mut c = self.settings.cookies_from_browser.chars();
            match c.next() {
                None => "None".to_string(),
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            }
        };

        [
            self.format_preset_to_string(&self.settings.format_preset)
                .to_string(),
            self.output_format_to_string(&self.settings.output_format)
                .to_string(),
            self.download_dir_display(),
            bool_to_yes_no(self.settings.write_subtitles).to_string(),
            bool_to_yes_no(self.settings.write_thumbnail).to_string(),
            bool_to_yes_no(self.settings.add_metadata).to_string(),
            bool_to_yes_no(self.settings.sponsorblock).to_string(),
            self.settings.concurrent_downloads.to_string(),
            if self.settings.rate_limit.is_empty() {
                "Unlimited".to_string()
            } else {
                self.settings.rate_limit.clone()
            },
            bool_to_yes_no(self.settings.network_retry).to_string(),
            format!("{} seconds", self.settings.retry_delay),
            cookies_display,
            bool_to_yes_no(self.settings.use_ascii_indicators).to_string(),
            bool_to_yes_no(self.settings.reset_stats_on_new_batch).to_string(),
            if self.settings.custom_ytdlp_args.is_empty() {
                "(none)".to_string()
            } else {
                truncate_for_display(&self.settings.custom_ytdlp_args, 30, Keep::Start)
            },
        ]
    }
}
