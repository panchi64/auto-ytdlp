use ratatui::{
    Frame,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

/// Render a toast notification in the top-right corner
pub(super) fn render_toast(frame: &mut Frame, message: &str) {
    let area = frame.area();
    let toast_width = (message.len() + 4).min(50) as u16;
    let toast_height = 3;
    let toast_x = area.width.saturating_sub(toast_width + 2);
    let toast_y = 1;
    let toast_area = ratatui::layout::Rect::new(toast_x, toast_y, toast_width, toast_height);

    // Clear the area behind the toast
    frame.render_widget(Clear, toast_area);

    let toast_widget = Paragraph::new(message)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .style(Style::default().fg(Color::White));

    frame.render_widget(toast_widget, toast_area);
}

/// Render the help overlay
pub(super) fn render_help_overlay(frame: &mut Frame) {
    let area = frame.area();
    // Never exceed the frame: rendering a Rect taller than the buffer panics, so
    // on a short terminal the popup shrinks and its trailing lines are clipped.
    let popup_width = 44.min(area.width);
    let popup_height = 24.min(area.height);
    let popup_x = (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = ratatui::layout::Rect::new(popup_x, popup_y, popup_width, popup_height);

    // Clear the area behind the popup
    frame.render_widget(Clear, popup_area);

    let help_lines = vec![
        Line::from(Span::styled(
            "DOWNLOAD CONTROLS",
            Style::default().fg(Color::Yellow),
        )),
        Line::from("  S     Start / Stop downloads"),
        Line::from("  P     Pause / Resume"),
        Line::from("  R     Reload queue from file"),
        Line::from("  T     Retry failed downloads"),
        Line::from("  X     Dismiss stale indicators"),
        Line::from(""),
        Line::from(Span::styled(
            "URL MANAGEMENT",
            Style::default().fg(Color::Yellow),
        )),
        Line::from("  A     Add URLs from clipboard"),
        Line::from("  F     Load URLs from links.txt"),
        Line::from("  E     Edit queue (when stopped)"),
        Line::from("  /     Search/filter queue"),
        Line::from("  ↑↓ wheel PgUp/PgDn Home/End  Scroll"),
        Line::from(""),
        Line::from(Span::styled(
            "APPLICATION",
            Style::default().fg(Color::Yellow),
        )),
        Line::from("  U     Update yt-dlp"),
        Line::from("  F1    Toggle this help"),
        Line::from("  F2    Open settings"),
        Line::from("  q     Graceful quit"),
        Line::from("  Q     Force quit (Shift+Q)"),
        Line::from(""),
        Line::from(Span::styled(
            "Press F1 or Esc to close",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let help_widget = Paragraph::new(help_lines)
        .block(
            Block::default()
                .title(" Help ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .style(Style::default().fg(Color::White));

    frame.render_widget(help_widget, popup_area);
}

#[cfg(test)]
mod tests;
