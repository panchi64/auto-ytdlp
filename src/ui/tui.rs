use anyhow::Result;
use std::{
    thread,
    time::{Duration, Instant},
};

use clipboard::ClipboardProvider;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use notify_rust::Notification;
use ratatui::{
    Frame, Terminal,
    prelude::CrosstermBackend,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Gauge, LineGauge, List, ListItem, Paragraph},
};
use std::io;

use crate::{
    app_state::{AppState, DownloadProgress, StateMessage, UiSnapshot},
    args::Args,
    downloader::{common::validate_dependencies, queue::process_queue},
    errors::AppError,
    ui::settings_menu::SettingsMenu,
    utils::file::{add_clipboard_links, get_links_from_file, sanitize_links_file},
};

/// UI context for additional rendering state not captured in UiSnapshot
#[derive(Default)]
pub struct UiContext {
    pub queue_edit_mode: bool,
    pub queue_selected_index: usize,
    pub show_help: bool,
}

/// Renders the Terminal User Interface (TUI) using a snapshot of the application state.
///
/// This function is responsible for drawing all UI elements including the progress bar,
/// download queues, active downloads, logs, and keyboard control instructions.
///
/// # Parameters
///
/// * `frame` - A mutable reference to the terminal frame to render elements into
/// * `snapshot` - A snapshot of the current application state (captured once per frame)
/// * `settings_menu` - A mutable reference to the settings menu
/// * `ctx` - UI context with additional state like queue edit mode
///
/// # Example
///
/// ```
/// let snapshot = state.get_ui_snapshot()?;
/// terminal.draw(|f| ui(f, &snapshot, &mut settings_menu, &ctx))?;
/// ```
pub fn ui(frame: &mut Frame, snapshot: &UiSnapshot, settings_menu: &mut SettingsMenu, ctx: &UiContext) {
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
                format!("[EDIT] Edit Queue - {}/{} (Up/Down: Navigate | D: Delete | Esc: Exit)", queue.len(), initial_total)
            } else {
                format!("📝 Edit Queue - {}/{} (↑↓: Navigate | D: Delete | Esc: Exit)", queue.len(), initial_total)
            }
        } else {
            let icon = if use_ascii {
                if queue.is_empty() { "[OK]" } else { "[Q]" }
            } else if queue.is_empty() { "✅" } else { "📋" };
            format!(
                "{} Pending Downloads - {}/{}",
                icon,
                queue.len(),
                initial_total
            )
        };

        let pending_items: Vec<ListItem> = queue
            .iter()
            .enumerate()
            .map(|(i, url)| {
                let item = ListItem::new(url.as_str());
                if ctx.queue_edit_mode && i == ctx.queue_selected_index {
                    item.style(Style::default().fg(Color::Yellow).bg(Color::DarkGray))
                } else {
                    item
                }
            })
            .collect();
        let pending_list = List::new(pending_items).block(
            Block::default()
                .title(pending_title)
                .borders(Borders::ALL)
                .border_style(if ctx.queue_edit_mode {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default()
                }),
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
        let help_text = if ctx.queue_edit_mode {
            "↑↓: Navigate | D: Delete URL | Esc/Enter: Exit edit mode"
        } else if is_completed {
            "R: Restart | E: Edit Queue | F1: Help | F2: Settings | Q: Quit | Shift+Q: Force Quit"
        } else if started && is_paused {
            "P: Resume | R: Reload | E: Edit Queue | S: Stop | A: Paste URLs | F1: Help | F2: Settings | Q: Quit"
        } else if started {
            "P: Pause | S: Stop | A: Paste URLs | F1: Help | F2: Settings | Q: Quit | Shift+Q: Force Quit"
        } else {
            "S: Start | R: Reload | E: Edit Queue | A: Paste URLs | F1: Help | F2: Settings | Q: Quit"
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
        let placeholder_widget = Paragraph::new(placeholder)
            .style(Style::default().fg(Color::DarkGray));
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
        let overflow_widget = Paragraph::new(overflow_text)
            .style(Style::default().fg(Color::DarkGray));
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
    let progress_label = if let (Some(frag_idx), Some(frag_count)) = (download.fragment_index, download.fragment_count) {
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
        format!("{}...", &download.display_name[..max_name_len.saturating_sub(3)])
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
            if use_ascii { " [STALE]" } else { " ⚠" },
            Style::default().fg(Color::DarkGray),
        ));
    }

    let info_line = Line::from(info_parts);
    let info_widget = Paragraph::new(info_line);
    frame.render_widget(info_widget, layout[1]);
}

/// Render the help overlay
fn render_help_overlay(frame: &mut Frame) {
    let area = frame.area();
    let popup_width = 44;
    let popup_height = 18;
    let popup_x = (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = ratatui::layout::Rect::new(popup_x, popup_y, popup_width, popup_height);

    // Clear the area behind the popup
    frame.render_widget(ratatui::widgets::Clear, popup_area);

    let help_lines = vec![
        Line::from(Span::styled("DOWNLOAD CONTROLS", Style::default().fg(Color::Yellow))),
        Line::from("  S     Start / Stop downloads"),
        Line::from("  P     Pause / Resume"),
        Line::from("  R     Reload queue from file"),
        Line::from(""),
        Line::from(Span::styled("URL MANAGEMENT", Style::default().fg(Color::Yellow))),
        Line::from("  A     Add URLs from clipboard"),
        Line::from("  F     Load URLs from links.txt"),
        Line::from("  E     Edit queue (when stopped)"),
        Line::from(""),
        Line::from(Span::styled("APPLICATION", Style::default().fg(Color::Yellow))),
        Line::from("  F1    Toggle this help"),
        Line::from("  F2    Open settings"),
        Line::from("  q     Graceful quit"),
        Line::from("  Q     Force quit (Shift+Q)"),
        Line::from(""),
        Line::from(Span::styled("Press F1 or Esc to close", Style::default().fg(Color::DarkGray))),
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

/// Runs the Terminal User Interface (TUI) loop.
///
/// This function initializes the terminal, sets up the application state,
/// and handles the main event loop for the TUI including keyboard input
/// processing and UI rendering.
///
/// # Parameters
///
/// * `state` - The application state
/// * `args` - Command line arguments
///
/// # Returns
///
/// A Result indicating success or failure
///
/// # Errors
///
/// Returns an error if there are issues with terminal setup, event handling,
/// or dependency checks.
pub fn run_tui(state: AppState, args: Args) -> Result<()> {
    // Terminal initialization
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Check dependencies before starting
    if let Err(error) = validate_dependencies() {
        if let Err(e) = state.add_log(format!("Error: {}", error)) {
            eprintln!("Error adding log: {}", e);
        }

        if error.to_string().contains("yt-dlp")
            && let Err(e) = state.add_log("Download the latest release of yt-dlp from: https://github.com/yt-dlp/yt-dlp/releases".to_string())
        {
            eprintln!("Error adding log: {}", e);
        }
        if error.to_string().contains("ffmpeg")
            && let Err(e) = state
                .add_log("Download ffmpeg from: https://www.ffmpeg.org/download.html".to_string())
        {
            eprintln!("Error adding log: {}", e);
        }
    }

    // Sanitize links file and load valid links
    match sanitize_links_file() {
        Ok(removed) => {
            if removed > 0
                && let Err(e) =
                    state.add_log(format!("Removed {} invalid URLs from links.txt", removed))
            {
                eprintln!("Error adding log: {}", e);
            }
        }
        Err(e) => {
            if let Err(log_err) = state.add_log(format!("Error sanitizing links file: {}", e)) {
                eprintln!("Error adding log: {}", log_err);
            }
        }
    }

    // Load any existing links
    match get_links_from_file() {
        Ok(links) => {
            if let Err(e) = state.send(StateMessage::LoadLinks(links)) {
                eprintln!("Error sending links: {}", e);
            }
        }
        Err(e) => {
            if let Err(log_err) = state.add_log(format!("Error loading links: {}", e)) {
                eprintln!("Error adding log: {}", log_err);
            }
        }
    }

    // Create settings menu
    let mut settings_menu = SettingsMenu::new(&state);

    // UI rendering loop
    let tick_rate = Duration::from_millis(100);
    let mut last_tick = Instant::now();

    // --- Added for graceful shutdown ---
    let mut download_thread_handle: Option<std::thread::JoinHandle<()>> = None;
    let mut await_downloads_on_exit = false;
    // --- End of addition ---

    // --- Force quit confirmation state ---
    let mut force_quit_pending = false;
    let mut force_quit_time: Option<Instant> = None;

    // --- Queue edit mode state ---
    let mut queue_edit_mode = false;
    let mut queue_selected_index: usize = 0;

    // --- Help overlay state ---
    let mut show_help = false;

    // Main loop
    loop {
        // Build UI context (only local state not in snapshot)
        let ui_ctx = UiContext {
            queue_edit_mode,
            queue_selected_index,
            show_help,
        };

        // Capture UI state snapshot once per frame (reduces lock acquisitions from 11+ to 1)
        let snapshot = state.get_ui_snapshot().unwrap_or_else(|_| UiSnapshot {
            progress: 0.0,
            completed_tasks: 0,
            total_tasks: 0,
            initial_total_tasks: 0,
            started: false,
            paused: false,
            completed: false,
            queue: std::collections::VecDeque::new(),
            active_downloads: Vec::new(),
            logs: Vec::new(),
            concurrent: 1,
            toast: None,
            use_ascii_indicators: false,
            total_retries: 0,
        });

        // Draw UI using snapshot
        terminal.draw(|f| ui(f, &snapshot, &mut settings_menu, &ui_ctx))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        // Handle input events
        if crossterm::event::poll(timeout)?
            && let Event::Key(key) = event::read()?
        {
                // First check if settings menu should handle the key
                if settings_menu.is_visible() && settings_menu.handle_input(key, &state) {
                    continue;
                }

                // Handle help overlay - F1 or Esc closes it
                if show_help {
                    match key.code {
                        KeyCode::F(1) | KeyCode::Esc => {
                            show_help = false;
                        }
                        _ => {}
                    }
                    continue;
                }

                // Handle queue edit mode
                if queue_edit_mode {
                    let queue_len = state.get_queue().map(|q| q.len()).unwrap_or(0);
                    match key.code {
                        KeyCode::Up => {
                            queue_selected_index = queue_selected_index.saturating_sub(1);
                        }
                        KeyCode::Down => {
                            if queue_len > 0 && queue_selected_index < queue_len - 1 {
                                queue_selected_index += 1;
                            }
                        }
                        KeyCode::Char('d') | KeyCode::Delete => {
                            if queue_len > 0
                                && let Ok(Some(removed)) = state.remove_from_queue(queue_selected_index)
                            {
                                // Show toast notification for removal
                                let _ = state.show_toast("URL removed from queue");
                                if let Err(e) = state.add_log(format!("Removed from queue: {}", removed)) {
                                    eprintln!("Error adding log: {}", e);
                                }
                                // Adjust selected index if necessary
                                let new_len = queue_len - 1;
                                if new_len == 0 {
                                    queue_edit_mode = false;
                                } else if queue_selected_index >= new_len {
                                    queue_selected_index = new_len.saturating_sub(1);
                                }
                            }
                        }
                        KeyCode::Esc | KeyCode::Enter | KeyCode::Char('e') => {
                            queue_edit_mode = false;
                        }
                        _ => {}
                    }
                    continue;
                }

                // Then handle normal application keys
                match key.code {
                    // F1 for help overlay
                    KeyCode::F(1) => {
                        show_help = true;
                    }
                    // Uppercase 'Q' (typically from Shift+q or CapsLock+Q) for Force Quit
                    KeyCode::Char('Q') => {
                        // Check if we're within the 2-second confirmation window
                        let should_force_quit = force_quit_pending
                            && force_quit_time
                                .map(|t| t.elapsed() < Duration::from_secs(2))
                                .unwrap_or(false);

                        if should_force_quit {
                            // Second Q within 2 seconds - execute force quit
                            if let Err(e) = state.send(StateMessage::SetForceQuit(true)) {
                                eprintln!("Error setting force quit: {}", e);
                            }
                            if let Err(e) = state.send(StateMessage::SetShutdown(true)) {
                                eprintln!("Error setting shutdown: {}", e);
                            }
                            if let Err(e) = state.add_log(
                                "TUI: Force quit confirmed. Exiting immediately.".to_string(),
                            ) {
                                eprintln!("Error adding log: {}", e);
                            }
                            // await_downloads_on_exit remains false (its default for force quit)
                            break;
                        } else {
                            // First Q - set pending and show warning
                            force_quit_pending = true;
                            force_quit_time = Some(Instant::now());
                            if let Err(e) = state.add_log(
                                "Press Shift+Q again within 2 seconds to force quit".to_string(),
                            ) {
                                eprintln!("Error adding log: {}", e);
                            }
                        }
                    }
                    // Lowercase 'q' for Graceful Quit
                    KeyCode::Char('q') => {
                        if let Err(e) = state.send(StateMessage::SetShutdown(true)) {
                            eprintln!("Error setting shutdown: {}", e);
                        }
                        if let Err(e) = state.add_log(
                            "TUI: Graceful shutdown (q) initiated. Will wait for downloads to complete.".to_string(),
                        ) {
                            eprintln!("Error adding log: {}", e);
                        }
                        await_downloads_on_exit = true;
                        break;
                    }
                    KeyCode::Char('s') => {
                        if let Ok(is_started) = state.is_started() {
                            if !is_started {
                                // Start downloads
                                match validate_dependencies() {
                                    Ok(()) => {
                                        if let Err(e) = state.reset_for_new_run() {
                                            eprintln!("Error resetting state: {}", e);
                                        }
                                        // Ensure await_downloads_on_exit is reset if we start a new session
                                        // after a previous graceful quit request that didn't exit the app.
                                        // (though current logic breaks loop on Q)
                                        await_downloads_on_exit = false;

                                        let state_clone = state.clone();
                                        let args_clone = args.clone();
                                        // --- Modified to store handle ---
                                        download_thread_handle = Some(thread::spawn(move || {
                                            process_queue(state_clone, args_clone)
                                        }));
                                        // --- End of modification ---
                                    }
                                    Err(error) => {
                                        if let Err(e) = state.add_log(format!("Error: {}", error)) {
                                            eprintln!("Error adding log: {}", e);
                                        }

                                        if error.to_string().contains("yt-dlp")
                                            && let Err(e) = state.add_log(
                                                "Download the latest release of yt-dlp from: https://github.com/yt-dlp/yt-dlp/releases".to_string()
                                            )
                                        {
                                            eprintln!("Error adding log: {}", e);
                                        }
                                        if error.to_string().contains("ffmpeg")
                                            && let Err(e) = state.add_log(
                                                "Download ffmpeg from: https://www.ffmpeg.org/download.html".to_string()
                                            )
                                        {
                                            eprintln!("Error adding log: {}", e);
                                        }
                                    }
                                }
                            } else {
                                // Stop downloads
                                if let Err(e) = state.send(StateMessage::SetShutdown(true)) {
                                    eprintln!("Error setting shutdown: {}", e);
                                }
                                if let Err(e) = state.send(StateMessage::SetStarted(false)) {
                                    eprintln!("Error setting started: {}", e);
                                }
                                if let Err(e) = state.send(StateMessage::SetPaused(false)) {
                                    eprintln!("Error setting paused: {}", e);
                                }
                                if let Err(e) = state.add_log(
                                    "TUI: Stop command issued. Waiting for current downloads to complete gracefully.".to_string()
                                ) {
                                    eprintln!("Error adding log: {}", e);
                                }
                                // --- Added to make 'Stop' also wait gracefully ---
                                // This will make the 'S' (Stop) command also block the TUI until downloads finish.
                                // If this is not desired, remove this block for 'S' (Stop).
                                if let Some(handle) = download_thread_handle.take() {
                                    // .take() so it's not joined again on exit
                                    // Get fresh snapshot for redraw
                                    if let Ok(fresh_snapshot) = state.get_ui_snapshot() {
                                        terminal.draw(|f| ui(f, &fresh_snapshot, &mut settings_menu, &ui_ctx))?; // Show "waiting" log
                                    }
                                    eprintln!(
                                        "Stopping downloads: Waiting for active downloads to complete..."
                                    );
                                    if let Err(e) = handle.join() {
                                        let err_msg = format!(
                                            "Error joining download thread on stop: {:?}",
                                            e
                                        );
                                        if let Err(log_err) = state.add_log(err_msg.clone()) {
                                            eprintln!("Error adding log: {}", log_err);
                                        }
                                        eprintln!("{}", err_msg);
                                    } else {
                                        if let Err(e) = state
                                            .add_log("Downloads stopped gracefully.".to_string())
                                        {
                                            eprintln!("Error adding log: {}", e);
                                        }
                                        eprintln!("Downloads stopped gracefully.");
                                    }
                                    // Get fresh snapshot for redraw after join
                                    if let Ok(fresh_snapshot) = state.get_ui_snapshot() {
                                        terminal.draw(|f| ui(f, &fresh_snapshot, &mut settings_menu, &ui_ctx))?; // Redraw after join
                                    }
                                }
                                // --- End of addition for 'Stop' ---

                                // Clear logs after a short delay when manually stopping downloads
                                let state_clone = state.clone();
                                thread::spawn(move || {
                                    thread::sleep(Duration::from_secs(2));
                                    if let Err(e) = state_clone.clear_logs() {
                                        eprintln!("Error clearing logs: {}", e);
                                    }
                                });
                            }
                        }
                    }
                    KeyCode::Char('p') => {
                        if let Ok(true) = state.is_started() {
                            // Get current paused state and toggle it
                            let current_paused = state.is_paused().unwrap_or(false);
                            if let Err(e) = state.send(StateMessage::SetPaused(!current_paused)) {
                                eprintln!("Error setting paused: {}", e);
                            }
                            // Log the pause/resume action
                            let log_message = if current_paused {
                                "Downloads resumed"
                            } else {
                                "Downloads paused. Press P to resume."
                            };
                            if let Err(e) = state.add_log(log_message.to_string()) {
                                eprintln!("Error adding log: {}", e);
                            }
                            last_tick = Instant::now() - tick_rate;
                        }
                    }
                    KeyCode::Char('r') => {
                        let is_started = state.is_started().unwrap_or(false);
                        let is_paused = state.is_paused().unwrap_or(false);
                        let is_completed = state.is_completed().unwrap_or(false);

                        if !is_started || is_paused || is_completed {
                            // Keep 'R' for restarting when completed
                            if let Err(e) = state.reset_for_new_run() {
                                eprintln!("Error resetting state: {}", e);
                            }

                            // Refresh links in the app state
                            match get_links_from_file() {
                                Ok(links) => {
                                    if let Err(e) = state.send(StateMessage::LoadLinks(links)) {
                                        eprintln!("Error sending links: {}", e);
                                    }
                                }
                                Err(e) => {
                                    eprintln!("Error loading links: {}", e);
                                }
                            }

                            if let Err(e) = state.add_log("Links refreshed from file".to_string()) {
                                eprintln!("Error adding log: {}", e);
                            }
                            last_tick = Instant::now() - tick_rate;
                        }
                    }
                    KeyCode::Char('f') => {
                        // First sanitize the links file
                        match sanitize_links_file() {
                            Ok(removed) => {
                                if removed > 0
                                    && let Err(e) = state.add_log(format!(
                                        "Removed {} invalid URLs from links.txt",
                                        removed
                                    ))
                                {
                                    eprintln!("Error adding log: {}", e);
                                }
                            }
                            Err(e) => {
                                if let Err(log_err) =
                                    state.add_log(format!("Error sanitizing links file: {}", e))
                                {
                                    eprintln!("Error adding log: {}", log_err);
                                }
                            }
                        }

                        // Then load links from the file
                        match get_links_from_file() {
                            Ok(links) => {
                                if let Err(e) = state.send(StateMessage::LoadLinks(links)) {
                                    eprintln!("Error sending links: {}", e);
                                }
                                if let Err(e) = state.add_log("Links loaded from file".to_string())
                                {
                                    eprintln!("Error adding log: {}", e);
                                }
                            }
                            Err(e) => {
                                if let Err(log_err) =
                                    state.add_log(format!("Error loading links: {}", e))
                                {
                                    eprintln!("Error adding log: {}", log_err);
                                }
                            }
                        }
                        last_tick = Instant::now() - tick_rate;
                    }
                    KeyCode::Char('a') => {
                        // Allow adding links at any time - they will be queued for download
                        let ctx: Result<
                            clipboard::ClipboardContext,
                            Box<dyn std::error::Error>,
                        > = ClipboardProvider::new().map_err(|e| {
                            Box::new(AppError::Clipboard(e.to_string()))
                                as Box<dyn std::error::Error>
                        });

                        match ctx {
                            Ok(mut ctx) => match ctx.get_contents() {
                                Ok(contents) => match add_clipboard_links(&state, &contents) {
                                    Ok(links_added) => {
                                        if links_added > 0 {
                                            if let Err(e) =
                                                state.send(StateMessage::SetCompleted(false))
                                            {
                                                eprintln!(
                                                    "Error setting completed flag: {}",
                                                    e
                                                );
                                            }
                                            // Use "Queued" instead of "Added" when downloads are active
                                            let is_active = state.is_started().unwrap_or(false)
                                                && !state.is_paused().unwrap_or(false)
                                                && !state.is_completed().unwrap_or(false);
                                            let msg = if is_active {
                                                format!("Queued {} new URLs", links_added)
                                            } else {
                                                format!("Added {} URLs", links_added)
                                            };
                                            // Show toast notification
                                            let _ = state.show_toast(&msg);
                                            if let Err(e) = state.add_log(msg) {
                                                eprintln!("Error adding log: {}", e);
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        if let Err(log_err) = state.add_log(format!(
                                            "Error adding clipboard links: {}",
                                            e
                                        )) {
                                            eprintln!("Error adding log: {}", log_err);
                                        }
                                    }
                                },
                                Err(e) => {
                                    if let Err(log_err) = state.add_log(format!(
                                        "Error getting clipboard contents: {}",
                                        e
                                    )) {
                                        eprintln!("Error adding log: {}", log_err);
                                    }
                                }
                            },
                            Err(e) => {
                                if let Err(log_err) = state
                                    .add_log(format!("Error initializing clipboard: {}", e))
                                {
                                    eprintln!("Error adding log: {}", log_err);
                                }
                            }
                        }
                    }
                    KeyCode::Char('e') => {
                        // Only allow queue editing when not actively downloading
                        let is_active = state.is_started().unwrap_or(false)
                            && !state.is_paused().unwrap_or(false)
                            && !state.is_completed().unwrap_or(false);
                        if !is_active {
                            let queue_len = state.get_queue().map(|q| q.len()).unwrap_or(0);
                            if queue_len > 0 {
                                queue_edit_mode = true;
                                queue_selected_index = 0;
                                if let Err(e) = state.add_log("Queue edit mode: ↑↓ Navigate | D: Delete | Esc: Exit".to_string()) {
                                    eprintln!("Error adding log: {}", e);
                                }
                            } else if let Err(e) = state.add_log("No URLs in queue to edit".to_string()) {
                                eprintln!("Error adding log: {}", e);
                            }
                        } else if let Err(e) = state.add_log("Cannot edit queue while downloads are active".to_string()) {
                            eprintln!("Error adding log: {}", e);
                        }
                    }
                    KeyCode::F(2) => {
                        settings_menu = SettingsMenu::new(&state);
                        settings_menu.toggle();
                    }
                    _ => {}
                }
        }

        // Handle timed events
        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();

            // Reset force quit confirmation if timeout expired
            if force_quit_pending
                && let Some(time) = force_quit_time
                && time.elapsed() >= Duration::from_secs(2)
            {
                force_quit_pending = false;
                force_quit_time = None;
            }

            // Check if we should send a notification
            if let Ok(is_completed) = state.is_completed()
                && is_completed
            {
                let is_force_quit = state.is_force_quit().unwrap_or(false);
                let is_shutdown = state.is_shutdown().unwrap_or(false);

                // Show notification when all downloads are completed
                if !is_force_quit && !is_shutdown {
                    let _ = Notification::new()
                        .summary("Auto-YTDlp Downloads Completed")
                        .body("All downloads have been completed!")
                        .show();
                }
            }
        }
    } // End of main TUI loop

    // --- Added for graceful shutdown wait ---
    if await_downloads_on_exit {
        if let Some(handle) = download_thread_handle {
            // Don't .take() if 'S' (Stop) might have already taken it
            // Add a log message to state if TUI were still drawing. For console, use eprintln.
            eprintln!("Graceful shutdown: Ensuring all downloads complete before exiting...");
            if let Err(e) = handle.join() {
                eprintln!("Error during final graceful shutdown wait: {:?}", e);
            }
            eprintln!("All downloads completed. Exiting application.");
        } else if await_downloads_on_exit {
            // Handle was already taken (e.g. by 'S' stop) but we still intended to wait
            eprintln!("Graceful shutdown: Download process already handled. Exiting application.");
        }
    }
    // --- End of addition ---

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}
