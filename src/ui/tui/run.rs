use anyhow::Result;
use std::{
    io,
    time::{Duration, Instant},
};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use notify_rust::Notification;
use ratatui::{Terminal, prelude::CrosstermBackend};

use crate::ui::settings_menu::SettingsMenu;
use crate::{
    app_state::{AppState, StateMessage, UiSnapshot},
    args::Args,
    downloader::common::validate_dependencies,
    utils::file::{get_links_from_file, sanitize_links_file},
};

use super::input::{
    DownloadState, ForceQuitState, InputResult, NormalModeContext, handle_edit_mode_input,
    handle_filter_mode_input, handle_help_overlay_input, handle_mouse_input,
    handle_normal_mode_input,
};
use super::{UiContext, ui};

pub fn run_tui(state: AppState, args: Args) -> Result<()> {
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

    let mut settings_menu = SettingsMenu::new(&state, args.flag_download_dir());

    let tick_rate = Duration::from_millis(100);
    let mut last_tick = Instant::now();

    let mut download_state = DownloadState::default();
    let mut force_quit_state = ForceQuitState::default();

    let mut ui_ctx = UiContext::default();

    loop {
        // Capture UI state snapshot once per frame
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
            failed_count: 0,
        });

        // Note links that arrived since the last frame so they can flash
        ui_ctx.track_new_links(&snapshot.queue);

        terminal.draw(|f| ui(f, &snapshot, &mut settings_menu, &mut ui_ctx))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if crossterm::event::poll(timeout)? {
            match event::read()? {
                Event::Key(key) => {
                    // First check if settings menu should handle the key
                    if settings_menu.is_visible() && settings_menu.handle_input(key, &state) {
                        continue;
                    }

                    if ui_ctx.show_help {
                        handle_help_overlay_input(key.code, &mut ui_ctx.show_help);
                        continue;
                    }

                    if ui_ctx.filter_mode {
                        handle_filter_mode_input(key.code, &state, &mut ui_ctx);
                        continue;
                    }

                    if ui_ctx.queue_edit_mode {
                        handle_edit_mode_input(key.code, &state, &mut ui_ctx);
                        continue;
                    }

                    let result = {
                        let mut nmc = NormalModeContext {
                            ctx: &mut ui_ctx,
                            download_state: &mut download_state,
                            force_quit_state: &mut force_quit_state,
                            last_tick: &mut last_tick,
                            tick_rate,
                        };
                        handle_normal_mode_input(key.code, &state, &args, &mut nmc)
                    };

                    match result {
                        InputResult::Break => break,
                        InputResult::Unhandled => {
                            if key.code == crossterm::event::KeyCode::F(2) {
                                settings_menu = SettingsMenu::new(&state, args.flag_download_dir());
                                settings_menu.toggle();
                            }
                        }
                        InputResult::Continue => {}
                    }
                }
                // Scroll wheel drives the pending list unless an overlay is up
                Event::Mouse(mouse) if !settings_menu.is_visible() && !ui_ctx.show_help => {
                    handle_mouse_input(mouse, &state, &mut ui_ctx);
                }
                _ => {}
            }
        }

        // Handle timed events
        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();

            // Reset force quit confirmation if timeout expired
            force_quit_state.check_timeout();

            if let Ok(is_completed) = state.is_completed()
                && is_completed
            {
                let is_force_quit = state.is_force_quit().unwrap_or(false);
                let is_shutdown = state.is_shutdown().unwrap_or(false);
                let notification_sent = state.is_notification_sent().unwrap_or(false);

                // Show notification when all downloads are completed (only once)
                if !is_force_quit && !is_shutdown && !notification_sent {
                    let _ = Notification::new()
                        .summary("Auto-YTDlp Downloads Completed")
                        .body("All downloads have been completed!")
                        .show();
                    let _ = state.set_notification_sent(true);
                }
            }
        }
    }

    // Graceful shutdown wait
    if download_state.await_downloads_on_exit {
        if let Some(handle) = download_state.download_thread_handle {
            eprintln!("Graceful shutdown: Ensuring all downloads complete before exiting...");
            if let Err(e) = handle.join() {
                eprintln!("Error during final graceful shutdown wait: {:?}", e);
            }
            eprintln!("All downloads completed. Exiting application.");
        } else {
            eprintln!("Graceful shutdown: Download process already handled. Exiting application.");
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
