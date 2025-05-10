use anyhow::Result;
use std::{
    thread,
    time::{Duration, Instant},
};

use clipboard::{ClipboardContext, ClipboardProvider};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use notify_rust::Notification;
use ratatui::{
    prelude::CrosstermBackend,
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph},
    Frame, Terminal,
};
use std::io;

use crate::{
    app_state::{AppState, StateMessage},
    args::Args,
    downloader::queue::process_queue,
    ui::settings_menu::SettingsMenu,
    utils::{
        dependencies::check_dependencies,
        file::{add_clipboard_links, get_links_from_file},
    },
};

/// Renders the Terminal User Interface (TUI) using the current application state.
///
/// This function is responsible for drawing all UI elements including the progress bar,
/// download queues, active downloads, logs, and keyboard control instructions.
///
/// # Parameters
///
/// * `frame` - A mutable reference to the terminal frame to render elements into
/// * `state` - A reference to the current application state
/// * `settings_menu` - A mutable reference to the settings menu
///
/// # Example
///
/// ```
/// terminal.draw(|f| ui(f, &state, &mut settings_menu))?;
/// ```
pub fn ui(
    frame: &mut Frame<CrosstermBackend<io::Stdout>>,
    state: &AppState,
    settings_menu: &mut SettingsMenu,
) {
    if settings_menu.is_visible() {
        settings_menu.render(frame, frame.size());
    } else {
        let progress = state.get_progress();
        let queue = state.get_queue();
        let active_downloads = state.get_active_downloads();
        let started = state.is_started();
        let logs = state.get_logs();
        let initial_total = state.get_initial_total_tasks();
        let concurrent = state.get_concurrent();
        let is_paused = state.is_paused();
        let is_completed = state.is_completed();
        let completed_tasks = state.get_completed_tasks();
        let total_tasks = state.get_total_tasks();

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
            .split(frame.size());

        // ----- Status indicators -----
        let status_indicator = if is_completed {
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
        let progress_title = format!(
            "{} - Progress: {:.1}% ({}/{}){}",
            status_indicator,
            progress,
            completed_tasks,
            total_tasks,
            if failed_count > 0 {
                format!(" - ❌ {} Failed", failed_count)
            } else {
                String::new()
            }
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
        let pending_title = format!(
            "{} Pending Downloads - {}/{}",
            if queue.is_empty() { "✅" } else { "📋" },
            queue.len(),
            initial_total
        );

        let pending_items: Vec<ListItem> =
            queue.iter().map(|i| ListItem::new(i.as_str())).collect();
        let pending_list = List::new(pending_items)
            .block(Block::default().title(pending_title).borders(Borders::ALL));
        frame.render_widget(pending_list, downloads_layout[0]);

        // Active downloads list with status icon
        let active_title = format!(
            "{} Active Downloads - {}/{}",
            if active_downloads.is_empty() {
                if started {
                    "⏸️"
                } else {
                    "⏹️"
                }
            } else {
                "⏳"
            },
            active_downloads.len(),
            concurrent
        );

        let active_items: Vec<ListItem> = active_downloads
            .iter()
            .map(|i| ListItem::new(i.as_str()))
            .collect();
        let active_list = List::new(active_items)
            .block(Block::default().title(active_title).borders(Borders::ALL));
        frame.render_widget(active_list, downloads_layout[1]);

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
        let help_text = if is_completed {
            "R: Restart | Q: Quit"
        } else if started {
            "P: Pause | S: Stop | R: Refresh | Q: Quit | A: Add URLs | F2: Settings"
        } else {
            "S: Start | R: Refresh | Q: Quit | A: Add URLs | F2: Settings"
        };

        let info_widget = Paragraph::new(help_text)
            .block(Block::default().title("Controls").borders(Borders::ALL))
            .style(Style::default().fg(Color::Gray));
        frame.render_widget(info_widget, main_layout[3]);
    }
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
    if let Err(errors) = check_dependencies() {
        for error in errors {
            state.add_log(format!("Error: {}", error));
        }
    }

    // Load any existing links
    let links = get_links_from_file();
    state.send(StateMessage::LoadLinks(links));

    // Create settings menu
    let mut settings_menu = SettingsMenu::new(&state);

    // UI rendering loop
    let tick_rate = Duration::from_millis(100);
    let mut last_tick = Instant::now();

    // Main loop
    loop {
        // Draw UI
        terminal.draw(|f| ui(f, &state, &mut settings_menu))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        // Handle input events
        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                // First check if settings menu should handle the key
                if settings_menu.is_visible() && settings_menu.handle_input(key, &state) {
                    continue;
                }

                // Then handle normal application keys
                match key.code {
                    KeyCode::Char('q') => {
                        if key.modifiers.contains(KeyModifiers::SHIFT) {
                            state.send(StateMessage::SetForceQuit(true));
                            state.send(StateMessage::SetShutdown(true));
                            break;
                        } else {
                            state.send(StateMessage::SetShutdown(true));
                            break;
                        }
                    }
                    KeyCode::Char('s') => {
                        if !state.is_started() {
                            match check_dependencies() {
                                Ok(()) => {
                                    state.reset_for_new_run();

                                    let state_clone = state.clone();
                                    let args_clone = args.clone();
                                    thread::spawn(move || process_queue(state_clone, args_clone));
                                }
                                Err(errors) => {
                                    for error in errors {
                                        state.add_log(error.clone());
                                        if error.contains("yt-dlp") {
                                            state.add_log("Download the latest release of yt-dlp from: https://github.com/yt-dlp/yt-dlp/releases".to_string());
                                        }
                                        if error.contains("ffmpeg") {
                                            state.add_log("Download ffmpeg from: https://www.ffmpeg.org/download.html".to_string());
                                        }
                                    }
                                }
                            }
                        } else {
                            state.send(StateMessage::SetShutdown(true));
                            state.send(StateMessage::SetStarted(false));
                            state.send(StateMessage::SetPaused(false));

                            // Clear logs after a short delay when manually stopping downloads
                            let state_clone = state.clone();
                            thread::spawn(move || {
                                thread::sleep(Duration::from_secs(2));
                                state_clone.clear_logs();
                            });
                        }
                    }
                    KeyCode::Char('p') => {
                        if state.is_started() {
                            state.send(StateMessage::SetPaused(!state.is_paused()));
                            last_tick = Instant::now() - tick_rate;
                        }
                    }
                    KeyCode::Char('r') => {
                        let links = get_links_from_file();
                        state.send(StateMessage::LoadLinks(links));
                        state.add_log("Links refreshed from file".to_string());
                        last_tick = Instant::now() - tick_rate;
                    }
                    KeyCode::Char('a') => {
                        // Only allow adding links when not actively downloading
                        if !state.is_started() || state.is_paused() || state.is_completed() {
                            let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();
                            if let Ok(contents) = ctx.get_contents() {
                                let links_added = add_clipboard_links(&state, &contents);

                                if links_added > 0 {
                                    state.send(StateMessage::SetCompleted(false));
                                    state.add_log(format!("Added {} URLs", links_added));
                                }
                            }
                        } else {
                            state.add_log("Cannot add links during active downloads".to_string());
                        }
                    }
                    KeyCode::F(2) => {
                        settings_menu = SettingsMenu::new(&state);
                        settings_menu.toggle();
                    }
                    _ => {}
                }
            }
        }

        // Handle timed events
        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();

            // Check if we should send a notification
            if state.is_completed() {
                let flags = state.is_force_quit() || state.is_shutdown();

                // Show notification when all downloads are completed
                if !flags {
                    let _ = Notification::new()
                        .summary("Auto-YTDlp Downloads Completed")
                        .body("All downloads have been completed!")
                        .show();
                }
            }
        }
    }

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
