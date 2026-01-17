mod input;
mod render;

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

use input::{
    DownloadState, ForceQuitState, InputResult, handle_edit_mode_input, handle_filter_mode_input,
    handle_help_overlay_input, handle_normal_mode_input,
};
pub use render::ui;

/// UI context for additional rendering state not captured in UiSnapshot
#[derive(Default)]
pub struct UiContext {
    pub queue_edit_mode: bool,
    pub queue_selected_index: usize,
    pub show_help: bool,
    /// Filter mode for queue search
    pub filter_mode: bool,
    /// Current filter text
    pub filter_text: String,
    /// Indices of queue items that match the filter
    pub filtered_indices: Vec<usize>,
}

/// Runs the Terminal User Interface (TUI) loop.
///
/// This function initializes the terminal, sets up the application state,
/// and handles the main event loop for the TUI including keyboard input
/// processing and UI rendering.
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

    // Download and shutdown state
    let mut download_state = DownloadState::default();
    let mut force_quit_state = ForceQuitState::default();

    // UI context (queue edit mode, help overlay, etc.)
    let mut ui_ctx = UiContext::default();

    // Main loop
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

            // Handle help overlay
            if ui_ctx.show_help {
                handle_help_overlay_input(key.code, &mut ui_ctx.show_help);
                continue;
            }

            // Handle filter mode
            if ui_ctx.filter_mode {
                handle_filter_mode_input(key.code, &state, &mut ui_ctx);
                continue;
            }

            // Handle queue edit mode
            if ui_ctx.queue_edit_mode {
                handle_edit_mode_input(key.code, &state, &mut ui_ctx);
                continue;
            }

            // Handle normal mode input
            let result = handle_normal_mode_input(
                key.code,
                &state,
                &args,
                &mut ui_ctx,
                &mut download_state,
                &mut force_quit_state,
                &mut last_tick,
                tick_rate,
            );

            match result {
                InputResult::Break => break,
                InputResult::Unhandled => {
                    // Handle F2 for settings menu toggle
                    if key.code == crossterm::event::KeyCode::F(2) {
                        settings_menu = SettingsMenu::new(&state);
                        settings_menu.toggle();
                    }
                }
                InputResult::Continue => {}
            }
        }

        // Handle timed events
        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();

            // Reset force quit confirmation if timeout expired
            force_quit_state.check_timeout();

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
