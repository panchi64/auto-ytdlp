mod input;
mod render;

use anyhow::Result;
use std::{
    collections::{HashMap, HashSet, VecDeque},
    hash::{DefaultHasher, Hash, Hasher},
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
    DownloadState, ForceQuitState, InputResult, NormalModeContext, handle_edit_mode_input,
    handle_filter_mode_input, handle_help_overlay_input, handle_mouse_input,
    handle_normal_mode_input,
};
pub use render::ui;

/// How long a newly queued link stays highlighted in the pending list.
pub const FLASH_DURATION: Duration = Duration::from_millis(1200);

/// Maps a row between queue order (oldest first, the order workers pop from) and
/// display order (newest first, so fresh links appear at the top of the panel).
///
/// The mapping is its own inverse. Returns `None` when the row does not exist.
pub fn flip_index(index: usize, queue_len: usize) -> Option<usize> {
    queue_len.checked_sub(1)?.checked_sub(index)
}

/// Identifies a URL for flash tracking without owning a copy of it.
///
/// The flash is purely decorative, so a hash collision would at worst tint the
/// wrong row for a moment - cheap enough to run over the whole queue every frame.
fn url_key(url: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    url.hash(&mut hasher);
    hasher.finish()
}

/// UI context for additional rendering state not captured in UiSnapshot
#[derive(Default)]
pub struct UiContext {
    pub queue_edit_mode: bool,
    /// Highlighted row in *display* order, so 0 is the newest link
    pub queue_selected_index: usize,
    pub show_help: bool,
    /// Filter mode for queue search
    pub filter_mode: bool,
    /// Current filter text
    pub filter_text: String,
    /// Indices of queue items that match the filter, in queue order
    pub filtered_indices: Vec<usize>,
    /// First display row visible in the pending list
    pub queue_scroll: usize,
    /// Rows that fit inside the pending list, recorded by the last render
    pub queue_view_height: usize,
    /// Queue length the pending list was last drawn from
    pub queue_drawn_len: usize,
    /// Screen rect the pending list was last drawn into, for mouse hit-testing
    pub queue_area: ratatui::layout::Rect,
    /// Queue contents as of the previous frame; `None` until the first frame
    queue_seen: Option<HashSet<u64>>,
    /// When each recently added URL showed up, driving the highlight flash
    queue_flash: HashMap<u64, Instant>,
}

impl UiContext {
    /// Flags links that appeared in the queue since the last frame so the pending
    /// list can flash them, and forgets flashes that expired or left the queue.
    ///
    /// New links enter at the back of the FIFO, which is the *top* of the panel, so
    /// they push everything below them down a row. A user who has scrolled away from
    /// the top is reading specific links, so the view shifts with them rather than
    /// letting the rows slide; at the top the new links stay visible, which is the
    /// point of showing them there.
    ///
    /// The first frame only records a baseline: links restored from `links.txt` at
    /// startup are not new from the user's point of view.
    pub fn track_new_links(&mut self, queue: &VecDeque<String>) {
        let current: HashSet<u64> = queue.iter().map(|url| url_key(url)).collect();
        let now = Instant::now();

        let mut added = 0usize;
        if let Some(previous) = &self.queue_seen {
            for key in current.difference(previous) {
                self.queue_flash.insert(*key, now);
                added += 1;
            }
        }

        if added > 0 {
            if self.queue_edit_mode {
                // Stay on the link the user selected rather than on the row number
                self.queue_selected_index += added;
                self.queue_scroll += added;
            } else if self.queue_scroll > 0 {
                self.queue_scroll += added;
            }
        }

        self.queue_flash
            .retain(|key, at| current.contains(key) && now.duration_since(*at) < FLASH_DURATION);
        self.queue_seen = Some(current);
    }

    /// How far a link is through its flash, from 0.0 (just added) to 1.0, or
    /// `None` once it is no longer flashing.
    pub fn flash_progress(&self, url: &str) -> Option<f32> {
        let elapsed = self.queue_flash.get(&url_key(url))?.elapsed().as_secs_f32();
        let duration = FLASH_DURATION.as_secs_f32();
        (elapsed < duration).then(|| elapsed / duration)
    }

    /// Rows a PageUp/PageDown moves, always at least one.
    pub fn page_size(&self) -> usize {
        self.queue_view_height.max(1)
    }

    /// Largest scroll offset that still keeps the pending list viewport filled.
    pub fn max_queue_scroll(&self, queue_len: usize) -> usize {
        queue_len.saturating_sub(self.queue_view_height)
    }

    /// Scrolls the pending list by `delta` rows, clamped to the current queue.
    pub fn scroll_queue_by(&mut self, delta: isize, queue_len: usize) {
        self.queue_scroll = self
            .queue_scroll
            .saturating_add_signed(delta)
            .min(self.max_queue_scroll(queue_len));
    }

    /// Scrolls to the display row holding the newest filter match.
    ///
    /// The filter dims non-matches rather than hiding them, so without this a
    /// filter applied while scrolled would leave every match off-screen.
    pub fn scroll_to_first_match(&mut self, queue_len: usize) {
        // `filtered_indices` is in queue order, so its last entry is the newest
        // match, which is the topmost matching row on screen
        let Some(newest_match) = self.filtered_indices.last() else {
            self.queue_scroll = 0;
            return;
        };
        let row = flip_index(*newest_match, queue_len).unwrap_or(0);
        self.queue_scroll = row.min(self.max_queue_scroll(queue_len));
    }

    /// Keeps the edit mode selection inside the visible rows.
    pub fn scroll_selection_into_view(&mut self) {
        let height = self.page_size();
        if self.queue_selected_index < self.queue_scroll {
            self.queue_scroll = self.queue_selected_index;
        } else if self.queue_selected_index >= self.queue_scroll + height {
            self.queue_scroll = self.queue_selected_index + 1 - height;
        }
    }
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
    let mut settings_menu = SettingsMenu::new(&state, args.flag_download_dir());

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
            failed_count: 0,
        });

        // Note links that arrived since the last frame so they can flash
        ui_ctx.track_new_links(&snapshot.queue);

        // Draw UI using snapshot
        terminal.draw(|f| ui(f, &snapshot, &mut settings_menu, &mut ui_ctx))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        // Handle input events
        if crossterm::event::poll(timeout)? {
            match event::read()? {
                Event::Key(key) => {
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
                            // Handle F2 for settings menu toggle
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

            // Check if we should send a notification
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

#[cfg(test)]
mod tests;
