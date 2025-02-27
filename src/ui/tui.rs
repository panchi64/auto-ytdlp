use anyhow::Result;
use std::{
    collections::HashSet,
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
    app_state::AppState,
    args::Args,
    downloader::queue::process_queue,
    utils::{
        dependencies::check_dependencies,
        file::{load_links, save_links},
    },
};

pub fn ui(frame: &mut Frame<CrosstermBackend<io::Stdout>>, state: &AppState) {
    let progress = *state.progress.lock().unwrap();
    let queue = state.queue.lock().unwrap().clone();
    let active_downloads = state.active_downloads.lock().unwrap().clone();
    let started = *state.started.lock().unwrap();
    let logs = state.logs.lock().unwrap().clone();
    let initial_total = *state.initial_total_tasks.lock().unwrap();
    let concurrent = *state.concurrent.lock().unwrap();

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
    let status_indicator = if *state.completed.lock().unwrap() {
        "✅ COMPLETED"
    } else if *state.paused.lock().unwrap() {
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
        *state.completed_tasks.lock().unwrap(),
        *state.total_tasks.lock().unwrap(),
        if failed_count > 0 {
            format!(" - ❌ {} Failed", failed_count)
        } else {
            String::new()
        }
    );

    let gauge = Gauge::default()
        .block(Block::default().title(progress_title).borders(Borders::ALL))
        .gauge_style(
            ratatui::style::Style::default().fg(if *state.paused.lock().unwrap() {
                ratatui::style::Color::Yellow
            } else if *state.completed.lock().unwrap() {
                ratatui::style::Color::Green
            } else if failed_count > 0 {
                ratatui::style::Color::Red
            } else if started {
                ratatui::style::Color::Blue
            } else {
                ratatui::style::Color::Gray
            }),
        )
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

    let pending_items: Vec<ListItem> = queue.iter().map(|i| ListItem::new(i.as_str())).collect();
    let pending_list =
        List::new(pending_items).block(Block::default().title(pending_title).borders(Borders::ALL));
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
            "🔄"
        },
        active_downloads.len(),
        concurrent
    );

    let active_items: Vec<ListItem> = active_downloads
        .iter()
        .map(|i| ListItem::new(i.as_str()))
        .collect();
    let active_list =
        List::new(active_items).block(Block::default().title(active_title).borders(Borders::ALL));
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
    let help_text = if *state.completed.lock().unwrap() {
        "Keys: [S]tart New | [A]dd | [R]efresh | [Q]uit | [Shift+Q] Force Quit"
    } else if !started {
        "Keys: [S]tart | [A]dd | [R]efresh | [Q]uit | [Shift+Q] Force Quit"
    } else if *state.paused.lock().unwrap() {
        "Keys: [P]Resume | [S]top | [A]dd | [R]efresh | [Q]uit | [Shift+Q] Force Quit"
    } else {
        "Keys: [P]ause | [S]top | [R]efresh | [Q]uit | [Shift+Q] Force Quit"
    };

    let help = Paragraph::new(help_text)
        .block(
            Block::default()
                .title("Keyboard Controls")
                .borders(Borders::ALL),
        )
        .alignment(ratatui::layout::Alignment::Center);
    frame.render_widget(help, main_layout[3]);
}

pub fn run_tui(state: AppState, args: Args) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let tick_rate = Duration::from_millis(250);
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|f| ui(f, &state))?;

        // Check for completed downloads and show notification
        {
            let completed = *state.completed.lock().unwrap();
            let mut notification_sent = state.notification_sent.lock().unwrap();

            if completed && !*notification_sent {
                Notification::new()
                    .summary("Download Complete")
                    .body("All downloads finished")
                    .show()?;
                *notification_sent = true;
            }
        }

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => {
                        if key.modifiers.contains(KeyModifiers::SHIFT) {
                            *state.force_quit.lock().unwrap() = true;
                            *state.shutdown.lock().unwrap() = true;
                            break;
                        } else {
                            *state.shutdown.lock().unwrap() = true;
                            break;
                        }
                    }
                    KeyCode::Char('s') => {
                        let mut started = state.started.lock().unwrap();
                        let mut shutdown = state.shutdown.lock().unwrap();
                        let mut paused = state.paused.lock().unwrap();

                        if !*started {
                            // Check dependencies before proceeding
                            match check_dependencies() {
                                Ok(()) => {
                                    // Proceed with starting downloads
                                    *shutdown = false;
                                    *paused = false;
                                    *started = true;
                                    *state.completed.lock().unwrap() = false;
                                    *state.progress.lock().unwrap() = 0.0;
                                    *state.completed_tasks.lock().unwrap() = 0;
                                    let queue_len = state.queue.lock().unwrap().len();
                                    *state.total_tasks.lock().unwrap() = queue_len;
                                    *state.notification_sent.lock().unwrap() = false;

                                    // Launch new worker threads
                                    let state_clone = state.clone();
                                    let args_clone = args.clone();
                                    thread::spawn(move || process_queue(state_clone, args_clone));
                                }
                                Err(errors) => {
                                    let mut logs = state.logs.lock().unwrap();
                                    for error in errors {
                                        logs.push(error.clone());
                                        // Provide installation links
                                        if error.contains("yt-dlp") {
                                            logs.push("Download the latest release of yt-dlp from: https://github.com/yt-dlp/yt-dlp/releases".to_string());
                                        }
                                        if error.contains("ffmpeg") {
                                            logs.push("Download ffmpeg from: https://www.ffmpeg.org/download.html".to_string());
                                        }
                                    }
                                }
                            }
                        } else {
                            // Stop ongoing downloads
                            *shutdown = true;
                            *started = false;
                            *paused = false;
                        }
                    }
                    KeyCode::Char('p') => {
                        if *state.started.lock().unwrap() {
                            let mut paused = state.paused.lock().unwrap();
                            *paused = !*paused;
                            // Force UI refresh
                            last_tick = Instant::now() - tick_rate;
                        }
                    }
                    KeyCode::Char('r') => {
                        if let Err(e) = load_links(&state) {
                            state
                                .logs
                                .lock()
                                .unwrap()
                                .push(format!("Error reloading links: {}", e));
                        } else {
                            let queue_len = state.queue.lock().unwrap().len();
                            *state.initial_total_tasks.lock().unwrap() = queue_len;

                            if !*state.started.lock().unwrap() {
                                *state.total_tasks.lock().unwrap() = queue_len;
                            }
                            state
                                .logs
                                .lock()
                                .unwrap()
                                .push("Links refreshed from file".to_string());
                        }
                        last_tick = Instant::now() - tick_rate;
                    }
                    KeyCode::Char('a') => {
                        let started = *state.started.lock().unwrap();
                        let paused = *state.paused.lock().unwrap();
                        let completed = *state.completed.lock().unwrap();

                        if !started || paused || completed {
                            let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();
                            if let Ok(contents) = ctx.get_contents() {
                                let links: Vec<String> = contents
                                    .lines()
                                    .map(|l| l.trim().to_string())
                                    .filter(|l| !l.is_empty())
                                    .filter(|l| url::Url::parse(l).is_ok())
                                    .collect();

                                let links_added = {
                                    let mut queue = state.queue.lock().unwrap();
                                    let existing: HashSet<_> = queue.iter().collect();
                                    let new_links = links
                                        .into_iter()
                                        .filter(|link| !existing.contains(link))
                                        .collect::<Vec<_>>();
                                    queue.extend(new_links.iter().cloned());
                                    new_links.len()
                                };

                                if links_added > 0 {
                                    // Update both current and initial totals
                                    *state.total_tasks.lock().unwrap() += links_added;
                                    *state.initial_total_tasks.lock().unwrap() += links_added;
                                    *state.completed.lock().unwrap() = false;
                                    save_links(&state)?;
                                    state
                                        .logs
                                        .lock()
                                        .unwrap()
                                        .push(format!("Added {} URLs", links_added));
                                }
                            }
                        } else {
                            state
                                .logs
                                .lock()
                                .unwrap()
                                .push("Cannot add links during active downloads".to_string());
                        }
                    }
                    _ => {}
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}
