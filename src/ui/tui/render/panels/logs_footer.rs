use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph},
};

use crate::app_state::UiSnapshot;
use crate::ui::tui::UiContext;

use super::super::style::calculate_wrapped_height;

pub(in crate::ui::tui::render) fn render_logs(frame: &mut Frame, area: Rect, logs: &[String]) {
    let colored_logs: Vec<Line> = logs
        .iter()
        .map(|line| {
            let style = if line.contains("Error") || line.contains("ERROR") {
                Style::default().fg(Color::Red)
            } else if line.contains("Warning") || line.contains("WARN") {
                Style::default().fg(Color::Yellow)
            } else if line.contains("Completed") {
                Style::default().fg(Color::Green)
            } else if line.contains("Starting download") {
                Style::default().fg(Color::Cyan)
            } else if line.contains("Links refreshed") || line.contains("Added") {
                Style::default().fg(Color::LightGreen)
            } else {
                Style::default().fg(Color::White)
            };

            Line::from(vec![Span::styled(line.clone(), style)])
        })
        .collect();

    // Account for borders: 2 for left/right, 2 for top/bottom
    let inner_width = area.width.saturating_sub(2) as usize;
    let inner_height = area.height.saturating_sub(2);

    let total_rendered_lines = calculate_wrapped_height(logs, inner_width);
    let scroll = total_rendered_lines.saturating_sub(inner_height);

    let logs_widget = Paragraph::new(Text::from(colored_logs))
        .block(Block::default().title("Logs").borders(Borders::ALL))
        .wrap(ratatui::widgets::Wrap { trim: true })
        .scroll((scroll, 0));
    frame.render_widget(logs_widget, area);
}

pub(in crate::ui::tui::render) fn render_footer(
    frame: &mut Frame,
    area: Rect,
    snapshot: &UiSnapshot,
    ctx: &UiContext,
) {
    let started = snapshot.started;
    let is_paused = snapshot.paused;
    let is_completed = snapshot.completed;
    let failed_count = snapshot.failed_count;

    let failed_hint = if failed_count > 0 && (!started || is_completed) {
        format!(" | T: Retry {} failed", failed_count)
    } else {
        String::new()
    };

    let help_text_owned;
    let help_text: &str = if ctx.filter_mode {
        "Type to filter | Enter: Keep filter | Esc: Clear filter"
    } else if ctx.queue_edit_mode {
        "↑↓: Navigate | K/J: Move Up/Down | D: Delete | Esc: Exit edit mode"
    } else if is_completed {
        help_text_owned = format!(
            "R: Restart | E: Edit Queue | /: Search | U: Update yt-dlp{} | F1: Help | F2: Settings | Q: Quit",
            failed_hint
        );
        &help_text_owned
    } else if started && is_paused {
        "P: Resume | R: Reload | E: Edit | /: Search | A: Paste | F1: Help | F2: Settings | Q: Quit"
    } else if started {
        "P: Pause | S: Stop | A: Paste URLs | F1: Help | F2: Settings | Q: Quit | Shift+Q: Force Quit"
    } else {
        help_text_owned = format!(
            "S: Start | R: Reload | E: Edit | /: Search | A: Paste | U: Update{} | F1: Help | F2: Settings | Q: Quit",
            failed_hint
        );
        &help_text_owned
    };

    let info_widget = Paragraph::new(help_text)
        .block(Block::default().title("Controls").borders(Borders::ALL))
        .style(Style::default().fg(Color::Gray));
    frame.render_widget(info_widget, area);
}
