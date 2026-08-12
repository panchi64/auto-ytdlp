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
    display::{create_action_item, create_setting_item},
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

        let values: Vec<String> = SETTINGS[..SETTINGS_COUNT]
            .iter()
            .map(|setting| setting.value.map(|render| render(self)).unwrap_or_default())
            .collect();
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
}
