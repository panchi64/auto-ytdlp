use ratatui::{
    Frame,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Gauge, LineGauge, List, ListItem, Paragraph},
};

use crate::app_state::{DownloadProgress, UiSnapshot};
use crate::ui::settings_menu::SettingsMenu;

use super::UiContext;

/// Renders the Terminal User Interface (TUI) using a snapshot of the application state.
///
/// This function is responsible for drawing all UI elements including the progress bar,
/// download queues, active downloads, logs, and keyboard control instructions.
pub fn ui(
    frame: &mut Frame,
    snapshot: &UiSnapshot,
    settings_menu: &mut SettingsMenu,
    ctx: &UiContext,
) {
    if settings_menu.is_visible() {
        settings_menu.render(frame, frame.area());
    } else {
        // Use pre-captured snapshot data instead of acquiring locks
        let progress = snapshot.progress;
        let queue = &snapshot.queue;
        let active_downloads = &snapshot.active_downloads;
        let started = snapshot.started;
        let logs = &snapshot.logs;
        let initial_total = snapshot.initial_total_tasks;
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

        // Count failed downloads based on log entries
        let failed_count = logs
            .iter()
            .filter(|line| line.starts_with("Failed:"))
            .count();

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
            progress,
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
            .percent(progress as u16);
        frame.render_widget(gauge, main_layout[0]);

        // ----- Downloads area (Pending + Active) -----
        let downloads_layout = ratatui::layout::Layout::default()
            .direction(ratatui::layout::Direction::Horizontal)
            .constraints([
                ratatui::layout::Constraint::Percentage(50),
                ratatui::layout::Constraint::Percentage(50),
            ])
            .split(main_layout[1]);

        // Pending downloads list with status icon
        let pending_title = if ctx.queue_edit_mode {
            if use_ascii {
                format!(
                    "[EDIT] Edit Queue - {}/{} (K/J: Move | D: Delete | Esc: Exit)",
                    queue.len(),
                    initial_total
                )
            } else {
                format!(
                    "📝 Edit Queue - {}/{} (K/J: Move | D: Delete | Esc: Exit)",
                    queue.len(),
                    initial_total
                )
            }
        } else if ctx.filter_mode || !ctx.filter_text.is_empty() {
            // Show filter info
            let match_count = ctx.filtered_indices.len();
            let total = queue.len();
            if use_ascii {
                format!(
                    "[FILTER: {}] {}/{} matches",
                    ctx.filter_text, match_count, total
                )
            } else {
                format!(
                    "🔍 [{}] {}/{} matches",
                    ctx.filter_text, match_count, total
                )
            }
        } else {
            let icon = if use_ascii {
                if queue.is_empty() { "[OK]" } else { "[Q]" }
            } else if queue.is_empty() {
                "✅"
            } else {
                "📋"
            };
            format!(
                "{} Pending Downloads - {}/{}",
                icon,
                queue.len(),
                initial_total
            )
        };

        // Build pending items - highlight matches when filter is active
        let has_filter = !ctx.filter_text.is_empty();
        let pending_items: Vec<ListItem> = queue
            .iter()
            .enumerate()
            .map(|(i, url)| {
                let is_match = has_filter && ctx.filtered_indices.contains(&i);
                let is_selected = ctx.queue_edit_mode && i == ctx.queue_selected_index;

                let style = if is_selected {
                    Style::default().fg(Color::Yellow).bg(Color::DarkGray)
                } else if is_match {
                    Style::default().fg(Color::Green)
                } else if has_filter {
                    Style::default().fg(Color::DarkGray)
                } else {
                    Style::default()
                };

                ListItem::new(url.as_str()).style(style)
            })
            .collect();

        let border_style = if ctx.filter_mode {
            Style::default().fg(Color::Cyan)
        } else if ctx.queue_edit_mode {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        };

        let pending_list = List::new(pending_items).block(
            Block::default()
                .title(pending_title)
                .borders(Borders::ALL)
                .border_style(border_style),
        );
        frame.render_widget(pending_list, downloads_layout[0]);

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
        let text_height = logs.len() as u16;
        let area_height = main_layout[2].height;
        let scroll = text_height.saturating_sub(area_height);

        let logs_widget = Paragraph::new(text_content)
            .block(Block::default().title("Logs").borders(Borders::ALL))
            .scroll((scroll, 0));
        frame.render_widget(logs_widget, main_layout[2]);

        // ----- Help text (keyboard shortcuts) -----
        let help_text = if ctx.filter_mode {
            "Type to filter | Enter: Keep filter | Esc: Clear filter"
        } else if ctx.queue_edit_mode {
            "↑↓: Navigate | K/J: Move Up/Down | D: Delete | Esc: Exit edit mode"
        } else if is_completed {
            "R: Restart | E: Edit Queue | /: Search | F1: Help | F2: Settings | Q: Quit"
        } else if started && is_paused {
            "P: Resume | R: Reload | E: Edit | /: Search | A: Paste | F1: Help | F2: Settings | Q: Quit"
        } else if started {
            "P: Pause | S: Stop | A: Paste URLs | F1: Help | F2: Settings | Q: Quit | Shift+Q: Force Quit"
        } else {
            "S: Start | R: Reload | E: Edit | /: Search | A: Paste | F1: Help | F2: Settings | Q: Quit"
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

    // Display name (truncated if needed)
    let max_name_len = (area.width as usize).saturating_sub(25);
    let display_name = if download.display_name.len() > max_name_len {
        format!(
            "{}...",
            &download.display_name[..max_name_len.saturating_sub(3)]
        )
    } else {
        download.display_name.clone()
    };
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
    let popup_width = 44;
    let popup_height = 21;
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
        Line::from(""),
        Line::from(Span::styled(
            "APPLICATION",
            Style::default().fg(Color::Yellow),
        )),
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
