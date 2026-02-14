use ratatui::{
    Frame,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Gauge, LineGauge, List, ListItem, Paragraph},
};

use crate::app_state::{DownloadProgress, UiSnapshot};
use crate::ui::settings_menu::SettingsMenu;

use super::UiContext;

/// Calculate the total height needed to render wrapped lines.
///
/// Accounts for text wrapping when lines exceed the available width.
fn calculate_wrapped_height(lines: &[String], available_width: usize) -> u16 {
    if available_width == 0 {
        return lines.len() as u16;
    }
    lines
        .iter()
        .map(|line| {
            let chars = line.chars().count();
            if chars == 0 {
                1u16
            } else {
                chars.div_ceil(available_width).max(1) as u16
            }
        })
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
                format!("🔍 [{}] {}/{} matches", ctx.filter_text, match_count, total)
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
    let popup_width = 44;
    let popup_height = 24;
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
mod tests {
    use super::*;

    // ========== Log Wrapping Height Calculation Tests ==========

    #[test]
    fn test_wrapped_height_empty_list() {
        let lines: Vec<String> = vec![];
        assert_eq!(calculate_wrapped_height(&lines, 80), 0);
    }

    #[test]
    fn test_wrapped_height_empty_lines_count_as_one() {
        let lines = vec!["".to_string(), "".to_string()];
        assert_eq!(calculate_wrapped_height(&lines, 80), 2);
    }

    #[test]
    fn test_wrapped_height_short_lines_no_wrap() {
        let lines = vec![
            "Short line".to_string(),
            "Another short".to_string(),
            "Third".to_string(),
        ];
        // All lines are under 80 chars, so 3 total lines
        assert_eq!(calculate_wrapped_height(&lines, 80), 3);
    }

    #[test]
    fn test_wrapped_height_exact_width_no_wrap() {
        // A line exactly at the width should not wrap
        let line = "x".repeat(40);
        let lines = vec![line];
        assert_eq!(calculate_wrapped_height(&lines, 40), 1);
    }

    #[test]
    fn test_wrapped_height_line_wraps_once() {
        // A line of 50 chars in 40 width should wrap to 2 lines
        let line = "x".repeat(50);
        let lines = vec![line];
        assert_eq!(calculate_wrapped_height(&lines, 40), 2);
    }

    #[test]
    fn test_wrapped_height_line_wraps_multiple_times() {
        // A line of 100 chars in 40 width should wrap to 3 lines (40+40+20)
        let line = "x".repeat(100);
        let lines = vec![line];
        assert_eq!(calculate_wrapped_height(&lines, 40), 3);
    }

    #[test]
    fn test_wrapped_height_mixed_line_lengths() {
        let lines = vec![
            "Short".to_string(),                      // 1 line
            "x".repeat(50),                           // 2 lines (50 / 40 = 2)
            "".to_string(),                           // 1 line
            "x".repeat(100),                          // 3 lines (100 / 40 = 3)
        ];
        // Total: 1 + 2 + 1 + 3 = 7 lines
        assert_eq!(calculate_wrapped_height(&lines, 40), 7);
    }

    #[test]
    fn test_wrapped_height_zero_width_returns_line_count() {
        // Edge case: zero width should return number of lines to avoid division by zero
        let lines = vec!["test".to_string(), "lines".to_string()];
        assert_eq!(calculate_wrapped_height(&lines, 0), 2);
    }

    #[test]
    fn test_wrapped_height_realistic_error_message() {
        // Simulate a realistic error message that might overflow
        let error_msg = "[ERROR] Failed to download: https://www.youtube.com/watch?v=very_long_video_id_here - Connection timeout after 30 seconds".to_string();
        let lines = vec![error_msg.clone()];

        // This 121-char message in a 60-char terminal wraps to 3 lines (60+60+1)
        let height = calculate_wrapped_height(&lines, 60);
        assert_eq!(height, 3);

        // Verify the math: chars.div_ceil(width)
        assert_eq!(error_msg.chars().count().div_ceil(60), 3);
    }

    #[test]
    fn test_wrapped_height_unicode_characters() {
        // Unicode characters should be counted by char, not bytes
        let line = "🎵".repeat(10); // 10 emoji characters
        let lines = vec![line];
        // 10 chars in 5-char width = 2 lines
        assert_eq!(calculate_wrapped_height(&lines, 5), 2);
    }

    #[test]
    fn test_wrapped_height_single_char_width() {
        // Edge case: width of 1 means each character is its own line
        let line = "abc".to_string();
        let lines = vec![line];
        assert_eq!(calculate_wrapped_height(&lines, 1), 3);
    }

    #[test]
    fn test_wrapped_height_many_log_entries() {
        // Simulate a log with many entries of varying lengths
        let lines: Vec<String> = (0..100)
            .map(|i| format!("Log entry {} with some additional text", i))
            .collect();

        let height = calculate_wrapped_height(&lines, 80);
        // Each line is under 80 chars, so should be exactly 100
        assert_eq!(height, 100);
    }

    // ========== Display Name Truncation Tests ==========

    #[test]
    fn test_truncate_display_name_short_ascii() {
        let result = truncate_display_name("short.mp4", 20);
        assert_eq!(result, "short.mp4");
    }

    #[test]
    fn test_truncate_display_name_exact_fit() {
        let name = "x".repeat(20);
        let result = truncate_display_name(&name, 20);
        assert_eq!(result, name);
    }

    #[test]
    fn test_truncate_display_name_long_ascii() {
        let name = "a".repeat(30);
        let result = truncate_display_name(&name, 20);
        assert!(result.ends_with("..."));
        assert!(result.chars().count() <= 20);
    }

    #[test]
    fn test_truncate_display_name_unicode_no_truncation() {
        let name = "動画テスト";
        let result = truncate_display_name(name, 20);
        assert_eq!(result, name);
    }

    #[test]
    fn test_truncate_display_name_unicode_truncation() {
        // 20 CJK characters, truncate to 10
        let name = "動画テストファイル名前動画テストファイル名前";
        let result = truncate_display_name(name, 10);
        assert!(result.ends_with("..."));
        assert!(result.chars().count() <= 10);
    }

    #[test]
    fn test_truncate_display_name_emoji() {
        let name = "🎵🎶🎧🎤🎸🎹🎺🎻🥁🎼🎵🎶🎧🎤🎸";
        let result = truncate_display_name(name, 10);
        assert!(result.ends_with("..."));
        assert!(result.chars().count() <= 10);
    }

    #[test]
    fn test_truncate_display_name_mixed_ascii_and_unicode() {
        let name = "Video - 日本語のタイトル - Episode 01";
        let result = truncate_display_name(name, 15);
        assert!(result.ends_with("..."));
        assert!(result.chars().count() <= 15);
    }

    #[test]
    fn test_truncate_display_name_empty() {
        let result = truncate_display_name("", 20);
        assert_eq!(result, "");
    }

    #[test]
    fn test_truncate_display_name_zero_max_len() {
        let result = truncate_display_name("test", 0);
        assert!(result.ends_with("..."));
    }
}
