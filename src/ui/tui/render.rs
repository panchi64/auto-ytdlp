use ratatui::{
    Frame,
    layout::{Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, Borders, Clear, Gauge, LineGauge, List, ListItem, Paragraph, Scrollbar,
        ScrollbarOrientation, ScrollbarState,
    },
};

use crate::app_state::{DownloadProgress, UiSnapshot};
use crate::ui::settings_menu::SettingsMenu;

use super::{UiContext, flip_index};

/// Background shades a freshly added link fades through, brightest first.
const FLASH_SHADES: [Color; 3] = [
    Color::Rgb(0, 84, 46),
    Color::Rgb(0, 56, 31),
    Color::Rgb(0, 30, 17),
];

/// Calculate the number of rows a single line occupies once wrapped.
///
/// Mirrors ratatui's `Wrap { trim: true }`: text breaks at whitespace, words
/// longer than the row are split, and widths are measured in display cells
/// (so double-width glyphs count as two).
fn wrapped_line_height(line: &str, available_width: usize) -> u16 {
    let mut rows: u16 = 1;
    let mut used = 0usize;

    for word in line.split_whitespace() {
        let width = Span::raw(word).width();

        if used > 0 {
            if used + 1 + width <= available_width {
                used += 1 + width;
                continue;
            }
            // Doesn't fit after the current content: start a new row.
            rows = rows.saturating_add(1);
        }

        // A word wider than the row spills over onto further rows.
        let overflow_rows = width.saturating_sub(1) / available_width;
        rows = rows.saturating_add(overflow_rows as u16);
        used = width - overflow_rows * available_width;
    }

    rows
}

/// Calculate the total height needed to render wrapped lines.
///
/// Accounts for text wrapping when lines exceed the available width.
fn calculate_wrapped_height(lines: &[String], available_width: usize) -> u16 {
    if available_width == 0 {
        return lines.len() as u16;
    }
    lines
        .iter()
        .map(|line| wrapped_line_height(line, available_width))
        .sum()
}

/// Renders the Terminal User Interface (TUI) using a snapshot of the application state.
///
/// This function is responsible for drawing all UI elements including the progress bar,
/// download queues, active downloads, logs, and keyboard control instructions.
pub fn ui(
    frame: &mut Frame,
    snapshot: &UiSnapshot,
    settings_menu: &mut SettingsMenu,
    ctx: &mut UiContext,
) {
    if settings_menu.is_visible() {
        settings_menu.render(frame, frame.area());
    } else {
        // Use pre-captured snapshot data instead of acquiring locks
        let progress = snapshot.progress;
        let active_downloads = &snapshot.active_downloads;
        let started = snapshot.started;
        let logs = &snapshot.logs;
        let concurrent = snapshot.concurrent;
        let is_paused = snapshot.paused;
        let is_completed = snapshot.completed;
        let completed_tasks = snapshot.completed_tasks;
        let total_tasks = snapshot.total_tasks;
        let use_ascii = snapshot.use_ascii_indicators;
        let total_retries = snapshot.total_retries;

        let main_layout = ratatui::layout::Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints(
                [
                    ratatui::layout::Constraint::Length(3),
                    ratatui::layout::Constraint::Percentage(40),
                    ratatui::layout::Constraint::Percentage(40),
                    ratatui::layout::Constraint::Length(4),
                ]
                .as_ref(),
            )
            .split(frame.area());

        // ----- Status indicators -----
        let status_indicator = if use_ascii {
            if is_completed {
                "[DONE] COMPLETED"
            } else if is_paused {
                "[PAUSE] PAUSED"
            } else if started {
                "[RUN] RUNNING"
            } else {
                "[STOP] STOPPED"
            }
        } else if is_completed {
            "✅ COMPLETED"
        } else if is_paused {
            "⏸️ PAUSED"
        } else if started {
            "▶️ RUNNING"
        } else {
            "⏹️ STOPPED"
        };

        let failed_count = snapshot.failed_count;

        // ----- Progress bar with status -----
        let retry_info = if total_retries > 0 {
            if use_ascii {
                format!(" | [R] {} retries", total_retries)
            } else {
                format!(" | ↻ {} retries", total_retries)
            }
        } else {
            String::new()
        };

        let progress_title = format!(
            "{} - Progress: {:.1}% ({}/{}){}{}",
            status_indicator,
            progress * 100.0,
            completed_tasks,
            total_tasks,
            if failed_count > 0 {
                if use_ascii {
                    format!(" - [X] {} Failed", failed_count)
                } else {
                    format!(" - ❌ {} Failed", failed_count)
                }
            } else {
                String::new()
            },
            retry_info
        );

        let gauge = Gauge::default()
            .block(Block::default().title(progress_title).borders(Borders::ALL))
            .gauge_style(ratatui::style::Style::default().fg(if is_paused {
                ratatui::style::Color::Yellow
            } else if is_completed {
                ratatui::style::Color::Green
            } else if failed_count > 0 {
                ratatui::style::Color::Red
            } else if started {
                ratatui::style::Color::Blue
            } else {
                ratatui::style::Color::Gray
            }))
            .ratio(progress);
        frame.render_widget(gauge, main_layout[0]);

        // ----- Downloads area (Pending + Active) -----
        let downloads_layout = ratatui::layout::Layout::default()
            .direction(ratatui::layout::Direction::Horizontal)
            .constraints([
                ratatui::layout::Constraint::Percentage(50),
                ratatui::layout::Constraint::Percentage(50),
            ])
            .split(main_layout[1]);

        render_pending_queue(frame, downloads_layout[0], snapshot, ctx);

        // Active downloads with per-download progress bars
        render_active_downloads(
            frame,
            downloads_layout[1],
            active_downloads,
            concurrent,
            use_ascii,
            started,
        );

        // ----- Logs display with color coding -----
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

        let text_content = Text::from(colored_logs);
        // Account for borders: 2 for left/right, 2 for top/bottom
        let inner_width = main_layout[2].width.saturating_sub(2) as usize;
        let inner_height = main_layout[2].height.saturating_sub(2);

        let total_rendered_lines = calculate_wrapped_height(logs, inner_width);
        let scroll = total_rendered_lines.saturating_sub(inner_height);

        let logs_widget = Paragraph::new(text_content)
            .block(Block::default().title("Logs").borders(Borders::ALL))
            .wrap(ratatui::widgets::Wrap { trim: true })
            .scroll((scroll, 0));
        frame.render_widget(logs_widget, main_layout[2]);

        // ----- Help text (keyboard shortcuts) -----
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
        frame.render_widget(info_widget, main_layout[3]);

        // ----- Help overlay (F1) -----
        if ctx.show_help {
            render_help_overlay(frame);
        }

        // ----- Toast notification -----
        if let Some(toast_msg) = &snapshot.toast {
            render_toast(frame, toast_msg);
        }
    }
}

/// Builds the title for the pending queue panel, which doubles as the mode
/// indicator for edit and filter modes.
fn pending_queue_title(snapshot: &UiSnapshot, ctx: &UiContext) -> String {
    let queue_len = snapshot.queue.len();
    let initial_total = snapshot.initial_total_tasks;
    let use_ascii = snapshot.use_ascii_indicators;

    if ctx.queue_edit_mode {
        let icon = if use_ascii { "[EDIT]" } else { "📝" };
        format!(
            "{} Edit Queue - {}/{} (K/J: Move | D: Delete | Esc: Exit)",
            icon, queue_len, initial_total
        )
    } else if ctx.filter_mode || !ctx.filter_text.is_empty() {
        let match_count = ctx.filtered_indices.len();
        if use_ascii {
            format!(
                "[FILTER: {}] {}/{} matches",
                ctx.filter_text, match_count, queue_len
            )
        } else {
            format!(
                "🔍 [{}] {}/{} matches",
                ctx.filter_text, match_count, queue_len
            )
        }
    } else {
        let icon = if use_ascii {
            if queue_len == 0 { "[OK]" } else { "[Q]" }
        } else if queue_len == 0 {
            "✅"
        } else {
            "📋"
        };
        format!(
            "{} Pending Downloads - {}/{}",
            icon, queue_len, initial_total
        )
    }
}

/// Style for a link partway through its "just added" flash.
///
/// The fade needs truecolor, which is the same capability gap the ASCII indicator
/// setting exists for, so compatibility mode gets a flat basic-ANSI highlight.
fn flash_style(progress: f32, use_ascii: bool) -> Style {
    if use_ascii {
        return Style::default().fg(Color::Black).bg(Color::Green);
    }

    let step = ((progress * FLASH_SHADES.len() as f32) as usize).min(FLASH_SHADES.len() - 1);
    Style::default().fg(Color::White).bg(FLASH_SHADES[step])
}

/// Renders the pending queue panel.
///
/// Rows are drawn newest first, so links added while the user is watching land at
/// the top of the panel, and only the slice starting at `ctx.queue_scroll` is drawn.
/// The underlying queue keeps its FIFO order - workers still pop the oldest link.
fn render_pending_queue(frame: &mut Frame, area: Rect, snapshot: &UiSnapshot, ctx: &mut UiContext) {
    let queue = &snapshot.queue;

    let border_style = if ctx.filter_mode {
        Style::default().fg(Color::Cyan)
    } else if ctx.queue_edit_mode {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };

    let block = Block::default()
        .title(pending_queue_title(snapshot, ctx))
        .borders(Borders::ALL)
        .border_style(border_style);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Record what was drawn so the input handlers can hit-test the wheel, page
    // against the viewport, and map rows using the same length the user saw
    ctx.queue_area = area;
    ctx.queue_view_height = inner.height as usize;
    ctx.queue_drawn_len = queue.len();
    ctx.scroll_queue_by(0, queue.len());
    let scroll = ctx.queue_scroll;
    let view_height = ctx.queue_view_height;

    let has_filter = !ctx.filter_text.is_empty();
    let visible_items: Vec<ListItem> = (scroll..queue.len())
        .take(view_height)
        .filter_map(|display_index| {
            let queue_index = flip_index(display_index, queue.len())?;
            let url = &queue[queue_index];
            let is_filtered_out = has_filter && !ctx.filtered_indices.contains(&queue_index);

            // A link the filter excludes stays dimmed even while it is flashing,
            // so the panel never reads as if a non-match were a match
            let style = if ctx.queue_edit_mode && display_index == ctx.queue_selected_index {
                Style::default().fg(Color::Yellow).bg(Color::DarkGray)
            } else if is_filtered_out {
                Style::default().fg(Color::DarkGray)
            } else if let Some(progress) = ctx.flash_progress(url) {
                flash_style(progress, snapshot.use_ascii_indicators)
            } else if has_filter {
                Style::default().fg(Color::Green)
            } else {
                Style::default()
            };

            Some(ListItem::new(url.as_str()).style(style))
        })
        .collect();

    frame.render_widget(List::new(visible_items), inner);

    if queue.len() > view_height {
        render_queue_scrollbar(frame, area, scroll, queue.len(), view_height);
    }
}

/// Draws a scroll thumb over the pending panel's right border.
fn render_queue_scrollbar(
    frame: &mut Frame,
    area: Rect,
    scroll: usize,
    queue_len: usize,
    view_height: usize,
) {
    // Content length is the number of scroll positions, so a fully scrolled list
    // puts the thumb flush against the bottom of the track.
    let mut scrollbar_state = ScrollbarState::new(queue_len - view_height + 1)
        .position(scroll)
        .viewport_content_length(view_height);

    frame.render_stateful_widget(
        Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .track_symbol(None)
            .begin_symbol(None)
            .end_symbol(None)
            .thumb_symbol("┃")
            .thumb_style(Style::default().fg(Color::Gray)),
        area.inner(Margin {
            vertical: 1,
            horizontal: 0,
        }),
        &mut scrollbar_state,
    );
}

/// Format bytes into human-readable string (e.g., "1.5MiB")
fn format_bytes(bytes: u64) -> String {
    const KIB: f64 = 1024.0;
    const MIB: f64 = KIB * 1024.0;
    const GIB: f64 = MIB * 1024.0;

    let bytes_f = bytes as f64;
    if bytes_f >= GIB {
        format!("{:.1}GiB", bytes_f / GIB)
    } else if bytes_f >= MIB {
        format!("{:.1}MiB", bytes_f / MIB)
    } else if bytes_f >= KIB {
        format!("{:.1}KiB", bytes_f / KIB)
    } else {
        format!("{}B", bytes)
    }
}

/// Render a toast notification in the top-right corner
fn render_toast(frame: &mut Frame, message: &str) {
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

/// Render active downloads with per-download progress bars
fn render_active_downloads(
    frame: &mut Frame,
    area: ratatui::layout::Rect,
    downloads: &[DownloadProgress],
    concurrent: usize,
    use_ascii: bool,
    started: bool,
) {
    // Build title with status icon
    let active_icon = if use_ascii {
        if downloads.is_empty() {
            if started { "[WAIT]" } else { "[STOP]" }
        } else {
            "[DL]"
        }
    } else if downloads.is_empty() {
        if started { "⏸️" } else { "⏹️" }
    } else {
        "⏳"
    };
    let active_title = format!(
        "{} Active Downloads - {}/{}",
        active_icon,
        downloads.len(),
        concurrent
    );

    let block = Block::default().title(active_title).borders(Borders::ALL);
    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    if downloads.is_empty() {
        // Show placeholder when no active downloads
        let placeholder = if started {
            "Waiting for downloads..."
        } else {
            "Press S to start downloads"
        };
        let placeholder_widget =
            Paragraph::new(placeholder).style(Style::default().fg(Color::DarkGray));
        frame.render_widget(placeholder_widget, inner_area);
        return;
    }

    // Calculate how many downloads we can show (2 lines per download)
    let max_visible = (inner_area.height as usize) / 2;
    let visible_downloads = downloads.len().min(max_visible);
    let overflow = downloads.len().saturating_sub(max_visible);

    // Create layout for visible downloads
    let mut constraints = Vec::with_capacity(visible_downloads + if overflow > 0 { 1 } else { 0 });
    for _ in 0..visible_downloads {
        constraints.push(ratatui::layout::Constraint::Length(2));
    }
    if overflow > 0 {
        constraints.push(ratatui::layout::Constraint::Length(1));
    }

    let download_layout = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints(constraints)
        .split(inner_area);

    // Render each visible download
    for (i, dl) in downloads.iter().take(visible_downloads).enumerate() {
        render_single_download_progress(frame, download_layout[i], dl, use_ascii);
    }

    // Show overflow indicator if needed
    if overflow > 0 {
        let overflow_text = format!("+{} more...", overflow);
        let overflow_widget =
            Paragraph::new(overflow_text).style(Style::default().fg(Color::DarkGray));
        frame.render_widget(overflow_widget, download_layout[visible_downloads]);
    }
}

/// Truncates a display name to fit within a maximum character width.
///
/// Uses char-aware truncation to avoid panics on multi-byte UTF-8 strings.
/// Appends "..." when truncation occurs.
fn truncate_display_name(name: &str, max_len: usize) -> String {
    let char_count = name.chars().count();
    if char_count > max_len {
        let truncated: String = name.chars().take(max_len.saturating_sub(3)).collect();
        format!("{}...", truncated)
    } else {
        name.to_string()
    }
}

/// Render a single download's progress
fn render_single_download_progress(
    frame: &mut Frame,
    area: ratatui::layout::Rect,
    download: &DownloadProgress,
    use_ascii: bool,
) {
    // Determine color based on phase and staleness
    let is_stale = download.last_update.elapsed().as_secs() > 30;
    let color = if is_stale {
        Color::DarkGray
    } else {
        match download.phase.as_str() {
            "downloading" => Color::Blue,
            "processing" | "merging" => Color::Yellow,
            "finished" => Color::Green,
            "error" => Color::Red,
            _ => Color::Cyan,
        }
    };

    // Split area into two lines: progress bar and info
    let layout = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            ratatui::layout::Constraint::Length(1),
            ratatui::layout::Constraint::Length(1),
        ])
        .split(area);

    // Line 1: Progress bar with LineGauge
    let ratio = (download.percent / 100.0).clamp(0.0, 1.0);

    // Build progress label
    let progress_label = if let (Some(frag_idx), Some(frag_count)) =
        (download.fragment_index, download.fragment_count)
    {
        // Show fragment progress for HLS/DASH
        format!("frag {}/{}", frag_idx, frag_count)
    } else {
        format!("{:.1}%", download.percent)
    };

    let line_gauge = LineGauge::default()
        .ratio(ratio)
        .label(progress_label)
        .filled_style(Style::default().fg(color).add_modifier(Modifier::BOLD))
        .unfilled_style(Style::default().fg(Color::DarkGray));

    frame.render_widget(line_gauge, layout[0]);

    // Line 2: Info line with display name, speed, ETA
    let mut info_parts: Vec<Span> = Vec::with_capacity(4);

    // Display name (truncated if needed, char-aware to avoid UTF-8 panics)
    let max_name_len = (area.width as usize).saturating_sub(25);
    let display_name = truncate_display_name(&download.display_name, max_name_len);
    info_parts.push(Span::styled(display_name, Style::default().fg(color)));

    // Size info (downloaded/total)
    if let Some(total) = download.total_bytes {
        let downloaded = download.downloaded_bytes.unwrap_or(0);
        info_parts.push(Span::raw(" "));
        info_parts.push(Span::styled(
            format!("{}/{}", format_bytes(downloaded), format_bytes(total)),
            Style::default().fg(Color::White),
        ));
    }

    // Speed
    if let Some(ref speed) = download.speed {
        info_parts.push(Span::raw(" "));
        info_parts.push(Span::styled(
            speed.clone(),
            Style::default().fg(Color::Cyan),
        ));
    }

    // ETA
    if let Some(ref eta) = download.eta {
        info_parts.push(Span::raw(" ETA:"));
        info_parts.push(Span::styled(
            eta.clone(),
            Style::default().fg(Color::Magenta),
        ));
    }

    // Stale indicator
    if is_stale {
        info_parts.push(Span::styled(
            if use_ascii {
                " [STALE - X:dismiss]"
            } else {
                " ⚠ (X:dismiss)"
            },
            Style::default().fg(Color::DarkGray),
        ));
    }

    let info_line = Line::from(info_parts);
    let info_widget = Paragraph::new(info_line);
    frame.render_widget(info_widget, layout[1]);
}

/// Render the help overlay
pub fn render_help_overlay(frame: &mut Frame) {
    let area = frame.area();
    // Never exceed the frame: rendering a Rect taller than the buffer panics, so
    // on a short terminal the popup shrinks and its trailing lines are clipped.
    let popup_width = 44.min(area.width);
    let popup_height = 24.min(area.height);
    let popup_x = (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = ratatui::layout::Rect::new(popup_x, popup_y, popup_width, popup_height);

    // Clear the area behind the popup
    frame.render_widget(ratatui::widgets::Clear, popup_area);

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
