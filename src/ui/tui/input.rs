use std::thread;
use std::time::{Duration, Instant};

use clipboard::ClipboardProvider;
use crossterm::event::KeyCode;

use crate::{
    app_state::{AppState, StateMessage},
    args::Args,
    downloader::{common::validate_dependencies, queue::process_queue},
    errors::AppError,
    utils::file::{add_clipboard_links, get_links_from_file, sanitize_links_file},
};

use super::UiContext;

/// State for managing download thread and graceful shutdown
#[derive(Default)]
pub struct DownloadState {
    pub download_thread_handle: Option<std::thread::JoinHandle<()>>,
    pub await_downloads_on_exit: bool,
}

/// State for force quit confirmation
#[derive(Default)]
pub struct ForceQuitState {
    pub pending: bool,
    pub time: Option<Instant>,
}

impl ForceQuitState {
    /// Check if we're within the 2-second confirmation window
    pub fn is_confirmed(&self) -> bool {
        self.pending
            && self
                .time
                .map(|t| t.elapsed() < Duration::from_secs(2))
                .unwrap_or(false)
    }

    /// Reset the force quit state if timeout expired
    pub fn check_timeout(&mut self) {
        if self.pending
            && let Some(time) = self.time
            && time.elapsed() >= Duration::from_secs(2)
        {
            self.pending = false;
            self.time = None;
        }
    }
}

/// Result of handling a key event
pub enum InputResult {
    /// Continue the main loop
    Continue,
    /// Break from the main loop (exit)
    Break,
    /// No action taken (key not handled)
    Unhandled,
}

/// Handle help overlay input (F1/Esc to close)
pub fn handle_help_overlay_input(key_code: KeyCode, show_help: &mut bool) -> InputResult {
    match key_code {
        KeyCode::F(1) | KeyCode::Esc => {
            *show_help = false;
            InputResult::Continue
        }
        _ => InputResult::Continue,
    }
}

/// Handle filter mode input (search/filter queue)
pub fn handle_filter_mode_input(
    key_code: KeyCode,
    state: &AppState,
    ctx: &mut UiContext,
) -> InputResult {
    match key_code {
        KeyCode::Esc => {
            // Clear filter and exit filter mode
            ctx.filter_mode = false;
            ctx.filter_text.clear();
            ctx.filtered_indices.clear();
            InputResult::Continue
        }
        KeyCode::Enter => {
            // Exit filter mode but keep the filter active
            ctx.filter_mode = false;
            InputResult::Continue
        }
        KeyCode::Backspace => {
            ctx.filter_text.pop();
            update_filtered_indices(state, ctx);
            InputResult::Continue
        }
        KeyCode::Char(c) => {
            ctx.filter_text.push(c);
            update_filtered_indices(state, ctx);
            InputResult::Continue
        }
        _ => InputResult::Continue,
    }
}

/// Update the filtered indices based on the current filter text
fn update_filtered_indices(state: &AppState, ctx: &mut UiContext) {
    ctx.filtered_indices.clear();

    if ctx.filter_text.is_empty() {
        return;
    }

    if let Ok(queue) = state.get_queue() {
        let filter_lower = ctx.filter_text.to_lowercase();
        for (i, url) in queue.iter().enumerate() {
            if url.to_lowercase().contains(&filter_lower) {
                ctx.filtered_indices.push(i);
            }
        }
    }
}

/// Handle queue edit mode input
pub fn handle_edit_mode_input(
    key_code: KeyCode,
    state: &AppState,
    ctx: &mut UiContext,
) -> InputResult {
    let queue_len = state.get_queue().map(|q| q.len()).unwrap_or(0);

    match key_code {
        KeyCode::Up => {
            ctx.queue_selected_index = ctx.queue_selected_index.saturating_sub(1);
        }
        KeyCode::Down => {
            if queue_len > 0 && ctx.queue_selected_index < queue_len - 1 {
                ctx.queue_selected_index += 1;
            }
        }
        KeyCode::Char('k') | KeyCode::Char('K') => {
            // Move item up (swap with previous)
            if ctx.queue_selected_index > 0
                && let Ok(true) =
                    state.swap_queue_items(ctx.queue_selected_index, ctx.queue_selected_index - 1)
            {
                ctx.queue_selected_index -= 1;
            }
        }
        KeyCode::Char('j') | KeyCode::Char('J') => {
            // Move item down (swap with next)
            if queue_len > 0
                && ctx.queue_selected_index < queue_len - 1
                && let Ok(true) =
                    state.swap_queue_items(ctx.queue_selected_index, ctx.queue_selected_index + 1)
            {
                ctx.queue_selected_index += 1;
            }
        }
        KeyCode::Char('d') | KeyCode::Delete => {
            if queue_len > 0
                && let Ok(Some(removed)) = state.remove_from_queue(ctx.queue_selected_index)
            {
                // Show toast notification for removal
                let _ = state.show_toast("URL removed from queue");
                if let Err(e) = state.add_log(format!("Removed from queue: {}", removed)) {
                    eprintln!("Error adding log: {}", e);
                }
                // Adjust selected index if necessary
                let new_len = queue_len - 1;
                if new_len == 0 {
                    ctx.queue_edit_mode = false;
                } else if ctx.queue_selected_index >= new_len {
                    ctx.queue_selected_index = new_len.saturating_sub(1);
                }
            }
        }
        KeyCode::Esc | KeyCode::Enter | KeyCode::Char('e') => {
            ctx.queue_edit_mode = false;
        }
        _ => {}
    }

    InputResult::Continue
}

/// Handle normal mode keyboard input
#[allow(clippy::too_many_arguments)]
pub fn handle_normal_mode_input(
    key_code: KeyCode,
    state: &AppState,
    args: &Args,
    ctx: &mut UiContext,
    download_state: &mut DownloadState,
    force_quit_state: &mut ForceQuitState,
    last_tick: &mut Instant,
    tick_rate: Duration,
) -> InputResult {
    match key_code {
        // F1 for help overlay
        KeyCode::F(1) => {
            ctx.show_help = true;
            InputResult::Continue
        }
        // Uppercase 'Q' (typically from Shift+q or CapsLock+Q) for Force Quit
        KeyCode::Char('Q') => {
            if force_quit_state.is_confirmed() {
                // Second Q within 2 seconds - execute force quit
                if let Err(e) = state.send(StateMessage::SetForceQuit(true)) {
                    eprintln!("Error setting force quit: {}", e);
                }
                if let Err(e) = state.send(StateMessage::SetShutdown(true)) {
                    eprintln!("Error setting shutdown: {}", e);
                }
                if let Err(e) =
                    state.add_log("TUI: Force quit confirmed. Exiting immediately.".to_string())
                {
                    eprintln!("Error adding log: {}", e);
                }
                // await_downloads_on_exit remains false (its default for force quit)
                InputResult::Break
            } else {
                // First Q - set pending and show warning
                force_quit_state.pending = true;
                force_quit_state.time = Some(Instant::now());
                if let Err(e) =
                    state.add_log("Press Shift+Q again within 2 seconds to force quit".to_string())
                {
                    eprintln!("Error adding log: {}", e);
                }
                InputResult::Continue
            }
        }
        // Lowercase 'q' for Graceful Quit
        KeyCode::Char('q') => {
            if let Err(e) = state.send(StateMessage::SetShutdown(true)) {
                eprintln!("Error setting shutdown: {}", e);
            }
            if let Err(e) = state.add_log(
                "TUI: Graceful shutdown (q) initiated. Will wait for downloads to complete."
                    .to_string(),
            ) {
                eprintln!("Error adding log: {}", e);
            }
            download_state.await_downloads_on_exit = true;
            InputResult::Break
        }
        KeyCode::Char('s') => {
            handle_start_stop(state, args, download_state);
            InputResult::Continue
        }
        KeyCode::Char('p') => {
            handle_pause_resume(state, last_tick, tick_rate);
            InputResult::Continue
        }
        KeyCode::Char('r') => {
            handle_reload(state, last_tick, tick_rate);
            InputResult::Continue
        }
        KeyCode::Char('f') => {
            handle_load_file(state, last_tick, tick_rate);
            InputResult::Continue
        }
        KeyCode::Char('a') => {
            handle_add_clipboard(state);
            InputResult::Continue
        }
        KeyCode::Char('e') => {
            handle_edit_mode(state, ctx);
            InputResult::Continue
        }
        KeyCode::Char('/') => {
            // Enter filter mode for queue search
            ctx.filter_mode = true;
            ctx.filter_text.clear();
            ctx.filtered_indices.clear();
            InputResult::Continue
        }
        KeyCode::Char('x') => {
            // Dismiss stale download indicators
            if let Err(e) = state.refresh_all_download_timestamps() {
                eprintln!("Error refreshing timestamps: {}", e);
            }
            InputResult::Continue
        }
        KeyCode::F(2) => {
            // Return Unhandled to let the caller toggle settings menu
            InputResult::Unhandled
        }
        _ => InputResult::Unhandled,
    }
}

fn handle_start_stop(state: &AppState, args: &Args, download_state: &mut DownloadState) {
    if let Ok(is_started) = state.is_started() {
        if !is_started {
            // Start downloads
            match validate_dependencies() {
                Ok(()) => {
                    if let Err(e) = state.reset_for_new_run() {
                        eprintln!("Error resetting state: {}", e);
                    }
                    download_state.await_downloads_on_exit = false;

                    let state_clone = state.clone();
                    let args_clone = args.clone();
                    download_state.download_thread_handle = Some(thread::spawn(move || {
                        process_queue(state_clone, args_clone)
                    }));
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
                            "Download ffmpeg from: https://www.ffmpeg.org/download.html"
                                .to_string(),
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
                "TUI: Stop command issued. Waiting for current downloads to complete gracefully."
                    .to_string(),
            ) {
                eprintln!("Error adding log: {}", e);
            }

            // Wait for downloads to finish
            if let Some(handle) = download_state.download_thread_handle.take() {
                eprintln!("Stopping downloads: Waiting for active downloads to complete...");
                if let Err(e) = handle.join() {
                    let err_msg = format!("Error joining download thread on stop: {:?}", e);
                    if let Err(log_err) = state.add_log(err_msg.clone()) {
                        eprintln!("Error adding log: {}", log_err);
                    }
                    eprintln!("{}", err_msg);
                } else {
                    if let Err(e) = state.add_log("Downloads stopped gracefully.".to_string()) {
                        eprintln!("Error adding log: {}", e);
                    }
                    eprintln!("Downloads stopped gracefully.");
                }
            }

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

fn handle_pause_resume(state: &AppState, last_tick: &mut Instant, tick_rate: Duration) {
    if let Ok(true) = state.is_started() {
        let current_paused = state.is_paused().unwrap_or(false);
        if let Err(e) = state.send(StateMessage::SetPaused(!current_paused)) {
            eprintln!("Error setting paused: {}", e);
        }
        let log_message = if current_paused {
            "Downloads resumed"
        } else {
            "Downloads paused. Press P to resume."
        };
        if let Err(e) = state.add_log(log_message.to_string()) {
            eprintln!("Error adding log: {}", e);
        }
        *last_tick = Instant::now() - tick_rate;
    }
}

fn handle_reload(state: &AppState, last_tick: &mut Instant, tick_rate: Duration) {
    let is_started = state.is_started().unwrap_or(false);
    let is_paused = state.is_paused().unwrap_or(false);
    let is_completed = state.is_completed().unwrap_or(false);

    if !is_started || is_paused || is_completed {
        if let Err(e) = state.reset_for_new_run() {
            eprintln!("Error resetting state: {}", e);
        }

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
        *last_tick = Instant::now() - tick_rate;
    }
}

fn handle_load_file(state: &AppState, last_tick: &mut Instant, tick_rate: Duration) {
    // First sanitize the links file
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

    // Then load links from the file
    match get_links_from_file() {
        Ok(links) => {
            if let Err(e) = state.send(StateMessage::LoadLinks(links)) {
                eprintln!("Error sending links: {}", e);
            }
            if let Err(e) = state.add_log("Links loaded from file".to_string()) {
                eprintln!("Error adding log: {}", e);
            }
        }
        Err(e) => {
            if let Err(log_err) = state.add_log(format!("Error loading links: {}", e)) {
                eprintln!("Error adding log: {}", log_err);
            }
        }
    }
    *last_tick = Instant::now() - tick_rate;
}

fn handle_add_clipboard(state: &AppState) {
    let ctx: Result<clipboard::ClipboardContext, Box<dyn std::error::Error>> =
        ClipboardProvider::new().map_err(|e| {
            Box::new(AppError::Clipboard(e.to_string())) as Box<dyn std::error::Error>
        });

    match ctx {
        Ok(mut ctx) => match ctx.get_contents() {
            Ok(contents) => match add_clipboard_links(state, &contents) {
                Ok(links_added) => {
                    if links_added > 0 {
                        if let Err(e) = state.send(StateMessage::SetCompleted(false)) {
                            eprintln!("Error setting completed flag: {}", e);
                        }
                        let is_active = state.is_started().unwrap_or(false)
                            && !state.is_paused().unwrap_or(false)
                            && !state.is_completed().unwrap_or(false);
                        let msg = if is_active {
                            format!("Queued {} new URLs", links_added)
                        } else {
                            format!("Added {} URLs", links_added)
                        };
                        let _ = state.show_toast(&msg);
                        if let Err(e) = state.add_log(msg) {
                            eprintln!("Error adding log: {}", e);
                        }
                    }
                }
                Err(e) => {
                    if let Err(log_err) =
                        state.add_log(format!("Error adding clipboard links: {}", e))
                    {
                        eprintln!("Error adding log: {}", log_err);
                    }
                }
            },
            Err(e) => {
                if let Err(log_err) =
                    state.add_log(format!("Error getting clipboard contents: {}", e))
                {
                    eprintln!("Error adding log: {}", log_err);
                }
            }
        },
        Err(e) => {
            if let Err(log_err) = state.add_log(format!("Error initializing clipboard: {}", e)) {
                eprintln!("Error adding log: {}", log_err);
            }
        }
    }
}

fn handle_edit_mode(state: &AppState, ctx: &mut UiContext) {
    let is_active = state.is_started().unwrap_or(false)
        && !state.is_paused().unwrap_or(false)
        && !state.is_completed().unwrap_or(false);

    if !is_active {
        let queue_len = state.get_queue().map(|q| q.len()).unwrap_or(0);
        if queue_len > 0 {
            ctx.queue_edit_mode = true;
            ctx.queue_selected_index = 0;
            if let Err(e) = state.add_log(
                "Queue edit mode: ↑↓ Navigate | K/J: Move | D: Delete | Esc: Exit".to_string(),
            ) {
                eprintln!("Error adding log: {}", e);
            }
        } else if let Err(e) = state.add_log("No URLs in queue to edit".to_string()) {
            eprintln!("Error adding log: {}", e);
        }
    } else if let Err(e) = state.add_log("Cannot edit queue while downloads are active".to_string())
    {
        eprintln!("Error adding log: {}", e);
    }
}
