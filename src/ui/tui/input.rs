use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

use arboard::Clipboard;
use crossterm::event::{KeyCode, MouseEvent, MouseEventKind};

use crate::{
    app_state::{AppState, StateMessage},
    args::Args,
    downloader::{common::validate_dependencies, queue::process_queue},
    errors::AppError,
    utils::file::{add_clipboard_links, get_links_from_file, sanitize_links_file},
};

use super::{UiContext, flip_index};

/// Rows the pending list moves per mouse wheel notch.
const WHEEL_SCROLL_ROWS: isize = 3;

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
            ctx.queue_scroll = 0;
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
        ctx.queue_scroll = 0;
        return;
    }

    if let Ok(queue) = state.get_queue() {
        let filter_lower = ctx.filter_text.to_lowercase();
        for (i, url) in queue.iter().enumerate() {
            if url.to_lowercase().contains(&filter_lower) {
                ctx.filtered_indices.push(i);
            }
        }
        // Bring the matches into view instead of leaving the user parked on a
        // stretch of dimmed non-matches
        ctx.scroll_to_first_match(queue.len());
    }
}

/// Handle queue edit mode input
///
/// `ctx.queue_selected_index` is a display row, so index 0 is the newest link at
/// the top of the panel. Every call into `AppState` flips it back to queue order.
pub fn handle_edit_mode_input(
    key_code: KeyCode,
    state: &AppState,
    ctx: &mut UiContext,
) -> InputResult {
    // Map rows using the length the panel was drawn from, not the live one: links
    // can land in the queue between the frame and this keypress, and because new
    // links go to the back, the drawn length still points every visible row at the
    // URL the user is looking at.
    let queue_len = ctx.queue_drawn_len;
    let page = ctx.page_size();
    let selected = flip_index(ctx.queue_selected_index, queue_len);

    match key_code {
        KeyCode::Up => {
            ctx.queue_selected_index = ctx.queue_selected_index.saturating_sub(1);
        }
        KeyCode::Down => {
            if queue_len > 0 && ctx.queue_selected_index < queue_len - 1 {
                ctx.queue_selected_index += 1;
            }
        }
        KeyCode::PageUp => {
            ctx.queue_selected_index = ctx.queue_selected_index.saturating_sub(page);
        }
        KeyCode::PageDown => {
            ctx.queue_selected_index =
                (ctx.queue_selected_index + page).min(queue_len.saturating_sub(1));
        }
        KeyCode::Home => {
            ctx.queue_selected_index = 0;
        }
        KeyCode::End => {
            ctx.queue_selected_index = queue_len.saturating_sub(1);
        }
        KeyCode::Char('k') | KeyCode::Char('K') => {
            // Move item up the panel, which is one step later in the queue
            if ctx.queue_selected_index > 0
                && let Some(index) = selected
                && let Ok(true) = state.swap_queue_items(index, index + 1)
            {
                ctx.queue_selected_index -= 1;
            }
        }
        KeyCode::Char('j') | KeyCode::Char('J') => {
            // Move item down the panel, which is one step earlier in the queue
            if let Some(index) = selected
                && index > 0
                && let Ok(true) = state.swap_queue_items(index, index - 1)
            {
                ctx.queue_selected_index += 1;
            }
        }
        KeyCode::Char('d') | KeyCode::Delete => {
            if let Some(index) = selected
                && let Ok(Some(removed)) = state.remove_from_queue(index)
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
                    ctx.queue_selected_index = new_len - 1;
                }
            }
        }
        KeyCode::Esc | KeyCode::Enter | KeyCode::Char('e') => {
            ctx.queue_edit_mode = false;
        }
        _ => {}
    }

    ctx.scroll_selection_into_view();
    InputResult::Continue
}

/// Scrolls the pending list by `delta` rows against the live queue length.
fn scroll_queue(state: &AppState, ctx: &mut UiContext, delta: isize) {
    ctx.scroll_queue_by(delta, state.queue_len().unwrap_or(0));
}

/// Handle mouse input - the wheel scrolls the pending queue list.
///
/// Only wheel events over the pending panel count, so scrolling elsewhere does not
/// move a list the pointer is nowhere near. In edit mode the wheel moves the
/// selection rather than the viewport, otherwise the highlighted row could scroll
/// out of sight and `D` would delete a link the user cannot see.
pub fn handle_mouse_input(mouse: MouseEvent, state: &AppState, ctx: &mut UiContext) {
    let delta = match mouse.kind {
        MouseEventKind::ScrollUp => -WHEEL_SCROLL_ROWS,
        MouseEventKind::ScrollDown => WHEEL_SCROLL_ROWS,
        _ => return,
    };

    if !ctx
        .queue_area
        .contains(ratatui::layout::Position::new(mouse.column, mouse.row))
    {
        return;
    }

    if ctx.queue_edit_mode {
        move_selection_by(ctx, delta);
    } else {
        scroll_queue(state, ctx, delta);
    }
}

/// Moves the edit mode selection by `delta` rows, clamped to the drawn queue, and
/// brings it back into view.
fn move_selection_by(ctx: &mut UiContext, delta: isize) {
    let last_row = ctx.queue_drawn_len.saturating_sub(1);
    ctx.queue_selected_index = ctx
        .queue_selected_index
        .saturating_add_signed(delta)
        .min(last_row);
    ctx.scroll_selection_into_view();
}

/// Context for normal mode input handling, grouping related mutable state
pub struct NormalModeContext<'a> {
    pub ctx: &'a mut UiContext,
    pub download_state: &'a mut DownloadState,
    pub force_quit_state: &'a mut ForceQuitState,
    pub last_tick: &'a mut Instant,
    pub tick_rate: Duration,
}

/// Handle normal mode keyboard input
pub fn handle_normal_mode_input(
    key_code: KeyCode,
    state: &AppState,
    args: &Args,
    nmc: &mut NormalModeContext<'_>,
) -> InputResult {
    match key_code {
        // F1 for help overlay
        KeyCode::F(1) => {
            nmc.ctx.show_help = true;
            InputResult::Continue
        }
        // Uppercase 'Q' (typically from Shift+q or CapsLock+Q) for Force Quit
        KeyCode::Char('Q') => {
            if nmc.force_quit_state.is_confirmed() {
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
                nmc.force_quit_state.pending = true;
                nmc.force_quit_state.time = Some(Instant::now());
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
            nmc.download_state.await_downloads_on_exit = true;
            InputResult::Break
        }
        KeyCode::Char('s') => {
            handle_start_stop(state, args, nmc.download_state);
            InputResult::Continue
        }
        KeyCode::Char('p') => {
            handle_pause_resume(state, nmc.last_tick, nmc.tick_rate);
            InputResult::Continue
        }
        KeyCode::Char('r') => {
            handle_reload(state, nmc.last_tick, nmc.tick_rate);
            InputResult::Continue
        }
        KeyCode::Char('f') => {
            handle_load_file(state, nmc.last_tick, nmc.tick_rate);
            InputResult::Continue
        }
        KeyCode::Char('a') => {
            handle_add_clipboard(state);
            InputResult::Continue
        }
        KeyCode::Char('e') => {
            handle_edit_mode(state, nmc.ctx);
            InputResult::Continue
        }
        KeyCode::Char('/') => {
            // Enter filter mode for queue search
            nmc.ctx.filter_mode = true;
            nmc.ctx.filter_text.clear();
            nmc.ctx.filtered_indices.clear();
            InputResult::Continue
        }
        KeyCode::Char('u') => {
            handle_ytdlp_update(state);
            InputResult::Continue
        }
        KeyCode::Char('t') => {
            handle_retry_failed(state);
            InputResult::Continue
        }
        KeyCode::Char('x') => {
            // Dismiss stale download indicators
            if let Err(e) = state.refresh_all_download_timestamps() {
                eprintln!("Error refreshing timestamps: {}", e);
            }
            InputResult::Continue
        }
        // Pending list scrolling
        KeyCode::Up => {
            scroll_queue(state, nmc.ctx, -1);
            InputResult::Continue
        }
        KeyCode::Down => {
            scroll_queue(state, nmc.ctx, 1);
            InputResult::Continue
        }
        KeyCode::PageUp => {
            let page = nmc.ctx.page_size() as isize;
            scroll_queue(state, nmc.ctx, -page);
            InputResult::Continue
        }
        KeyCode::PageDown => {
            let page = nmc.ctx.page_size() as isize;
            scroll_queue(state, nmc.ctx, page);
            InputResult::Continue
        }
        KeyCode::Home => {
            nmc.ctx.queue_scroll = 0;
            InputResult::Continue
        }
        KeyCode::End => {
            let bottom = nmc.ctx.max_queue_scroll(state.queue_len().unwrap_or(0));
            nmc.ctx.queue_scroll = bottom;
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
    let contents_result = Clipboard::new()
        .map_err(|e| AppError::Clipboard(format!("Failed to initialize clipboard: {}", e)))
        .and_then(|mut clipboard| {
            clipboard
                .get_text()
                .map_err(|e| AppError::Clipboard(format!("Failed to read clipboard: {}", e)))
        });

    match contents_result {
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
                if let Err(log_err) = state.add_log(format!("Error adding clipboard links: {}", e))
                {
                    eprintln!("Error adding log: {}", log_err);
                }
            }
        },
        Err(e) => {
            if let Err(log_err) = state.add_log(format!("{}", e)) {
                eprintln!("Error adding log: {}", log_err);
            }
        }
    }
}

fn handle_ytdlp_update(state: &AppState) {
    let is_started = state.is_started().unwrap_or(false);
    let is_completed = state.is_completed().unwrap_or(false);
    let is_paused = state.is_paused().unwrap_or(false);

    let downloads_active = is_started && !is_completed && !is_paused;
    if downloads_active {
        if let Err(e) = state.add_log("Cannot update while downloads are active".to_string()) {
            eprintln!("Error adding log: {}", e);
        }
        return;
    }

    if let Err(e) = state.add_log("Checking for yt-dlp updates...".to_string()) {
        eprintln!("Error adding log: {}", e);
    }

    let state_clone = state.clone();
    thread::spawn(move || match Command::new("yt-dlp").arg("-U").output() {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);

            for line in stdout.lines().chain(stderr.lines()) {
                let trimmed = line.trim();
                if !trimmed.is_empty()
                    && let Err(e) = state_clone.add_log(trimmed.to_string())
                {
                    eprintln!("Error adding log: {}", e);
                }
            }

            if output.status.success() {
                let _ = state_clone.show_toast("yt-dlp update complete");
            } else {
                let _ = state_clone.show_toast("yt-dlp update failed");
            }
        }
        Err(e) => {
            if let Err(log_err) = state_clone.add_log(format!("Failed to run yt-dlp -U: {}", e)) {
                eprintln!("Error adding log: {}", log_err);
            }
            let _ = state_clone.show_toast("yt-dlp update failed");
        }
    });
}

fn handle_retry_failed(state: &AppState) {
    let is_started = state.is_started().unwrap_or(false);
    let is_completed = state.is_completed().unwrap_or(false);
    let is_paused = state.is_paused().unwrap_or(false);

    let downloads_active = is_started && !is_completed && !is_paused;
    if downloads_active {
        if let Err(e) = state.add_log("Cannot retry while downloads are active".to_string()) {
            eprintln!("Error adding log: {}", e);
        }
        return;
    }

    match state.take_failed_downloads() {
        Ok(failed) => {
            if failed.is_empty() {
                if let Err(e) = state.add_log("No failed downloads to retry".to_string()) {
                    eprintln!("Error adding log: {}", e);
                }
            } else {
                let count = failed.len();
                for url in failed {
                    if let Err(e) = state.send(StateMessage::AddToQueue(url)) {
                        eprintln!("Error re-queuing URL: {}", e);
                    }
                }
                let _ = state.show_toast(format!("Re-queued {} failed downloads", count));
            }
        }
        Err(e) => {
            if let Err(log_err) = state.add_log(format!("Error getting failed downloads: {}", e)) {
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
        let queue_len = state.queue_len().unwrap_or(0);
        if queue_len > 0 {
            ctx.queue_edit_mode = true;
            // Start on the newest link, at the top of the panel
            ctx.queue_selected_index = 0;
            ctx.queue_scroll = 0;
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

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use std::thread;
    use std::time::Duration;

    // Helper to create AppState for testing
    fn create_test_state() -> AppState {
        AppState::new()
    }

    // Helper to create UiContext for testing
    fn create_test_context() -> UiContext {
        UiContext::default()
    }

    // Helper to create Args for testing
    fn create_test_args() -> Args {
        Args::parse_from(["test"])
    }

    // Helper to create NormalModeContext for testing
    fn create_test_nmc<'a>(
        ctx: &'a mut UiContext,
        download_state: &'a mut DownloadState,
        force_quit_state: &'a mut ForceQuitState,
        last_tick: &'a mut Instant,
        tick_rate: Duration,
    ) -> NormalModeContext<'a> {
        NormalModeContext {
            ctx,
            download_state,
            force_quit_state,
            last_tick,
            tick_rate,
        }
    }

    // ==================== ForceQuitState Tests ====================

    #[test]
    fn test_force_quit_state_initial() {
        let state = ForceQuitState::default();
        assert!(!state.pending);
        assert!(state.time.is_none());
        assert!(!state.is_confirmed());
    }

    #[test]
    fn test_force_quit_state_pending_not_confirmed_without_time() {
        let state = ForceQuitState {
            pending: true,
            ..Default::default()
        };
        // Without setting time, is_confirmed should return false
        assert!(!state.is_confirmed());
    }

    #[test]
    fn test_force_quit_state_confirmed_within_timeout() {
        let state = ForceQuitState {
            pending: true,
            time: Some(Instant::now()),
        };
        // Should be confirmed within 2 seconds
        assert!(state.is_confirmed());
    }

    #[test]
    fn test_force_quit_state_not_confirmed_after_timeout() {
        let state = ForceQuitState {
            pending: true,
            // Time is more than 2 seconds ago
            time: Some(Instant::now() - Duration::from_secs(3)),
        };
        assert!(!state.is_confirmed());
    }

    #[test]
    fn test_force_quit_state_check_timeout_resets_state() {
        let mut state = ForceQuitState {
            pending: true,
            time: Some(Instant::now() - Duration::from_secs(3)),
        };

        state.check_timeout();

        assert!(!state.pending);
        assert!(state.time.is_none());
    }

    #[test]
    fn test_force_quit_state_check_timeout_preserves_valid_state() {
        let now = Instant::now();
        let mut state = ForceQuitState {
            pending: true,
            time: Some(now),
        };

        state.check_timeout();

        // State should be preserved if within timeout
        assert!(state.pending);
        assert!(state.time.is_some());
    }

    // ==================== Filter Mode Tests ====================

    #[test]
    fn test_filter_mode_esc_clears_filter() {
        let state = create_test_state();
        let mut ctx = create_test_context();
        ctx.filter_mode = true;
        ctx.filter_text = "test".to_string();
        ctx.filtered_indices = vec![0, 1, 2];

        let result = handle_filter_mode_input(KeyCode::Esc, &state, &mut ctx);

        assert!(!ctx.filter_mode);
        assert!(ctx.filter_text.is_empty());
        assert!(ctx.filtered_indices.is_empty());
        assert!(matches!(result, InputResult::Continue));
    }

    #[test]
    fn test_filter_mode_enter_keeps_filter() {
        let state = create_test_state();
        let mut ctx = create_test_context();
        ctx.filter_mode = true;
        ctx.filter_text = "test".to_string();

        let result = handle_filter_mode_input(KeyCode::Enter, &state, &mut ctx);

        assert!(!ctx.filter_mode);
        assert_eq!(ctx.filter_text, "test");
        assert!(matches!(result, InputResult::Continue));
    }

    #[test]
    fn test_filter_mode_backspace_removes_char() {
        let state = create_test_state();
        let mut ctx = create_test_context();
        ctx.filter_mode = true;
        ctx.filter_text = "test".to_string();

        let result = handle_filter_mode_input(KeyCode::Backspace, &state, &mut ctx);

        assert_eq!(ctx.filter_text, "tes");
        assert!(matches!(result, InputResult::Continue));
    }

    #[test]
    fn test_filter_mode_backspace_on_empty_string() {
        let state = create_test_state();
        let mut ctx = create_test_context();
        ctx.filter_mode = true;
        ctx.filter_text = String::new();

        let result = handle_filter_mode_input(KeyCode::Backspace, &state, &mut ctx);

        assert!(ctx.filter_text.is_empty());
        assert!(matches!(result, InputResult::Continue));
    }

    #[test]
    fn test_filter_mode_char_input_adds_to_filter() {
        let state = create_test_state();
        let mut ctx = create_test_context();
        ctx.filter_mode = true;
        ctx.filter_text = "tes".to_string();

        let result = handle_filter_mode_input(KeyCode::Char('t'), &state, &mut ctx);

        assert_eq!(ctx.filter_text, "test");
        assert!(matches!(result, InputResult::Continue));
    }

    // ==================== Edit Mode Tests ====================

    #[test]
    fn test_edit_mode_up_navigation() {
        let state = create_test_state();
        let mut ctx = create_test_context();
        ctx.queue_edit_mode = true;
        ctx.queue_selected_index = 2;

        handle_edit_mode_input(KeyCode::Up, &state, &mut ctx);

        assert_eq!(ctx.queue_selected_index, 1);
    }

    #[test]
    fn test_edit_mode_up_navigation_at_zero() {
        let state = create_test_state();
        let mut ctx = create_test_context();
        ctx.queue_edit_mode = true;
        ctx.queue_selected_index = 0;

        handle_edit_mode_input(KeyCode::Up, &state, &mut ctx);

        // Should stay at 0 (saturating_sub)
        assert_eq!(ctx.queue_selected_index, 0);
    }

    #[test]
    fn test_edit_mode_down_navigation() {
        let state = create_test_state();
        // Add items to queue
        let _ = state.send(StateMessage::LoadLinks(vec![
            "url1".to_string(),
            "url2".to_string(),
            "url3".to_string(),
        ]));
        // Allow message processing
        thread::sleep(Duration::from_millis(50));

        let mut ctx = drawn_context(3, 3);
        ctx.queue_edit_mode = true;
        ctx.queue_selected_index = 0;

        handle_edit_mode_input(KeyCode::Down, &state, &mut ctx);

        assert_eq!(ctx.queue_selected_index, 1);
    }

    // Helper to build a state whose queue is url1..urlN in queue order,
    // which the pending list shows bottom-up as urlN..url1.
    fn state_with_queue(urls: &[&str]) -> AppState {
        let state = create_test_state();
        let _ = state.send(StateMessage::LoadLinks(
            urls.iter().map(|u| u.to_string()).collect(),
        ));
        thread::sleep(Duration::from_millis(50));
        state
    }

    /// A context in the state a render leaves behind: the panel was drawn from a
    /// queue of `drawn_len` links into a viewport `view_height` rows tall.
    fn drawn_context(drawn_len: usize, view_height: usize) -> UiContext {
        UiContext {
            queue_drawn_len: drawn_len,
            queue_view_height: view_height,
            queue_area: ratatui::layout::Rect::new(0, 0, 40, view_height as u16 + 2),
            ..Default::default()
        }
    }

    #[test]
    fn test_edit_mode_selection_starts_on_newest_link() {
        let state = state_with_queue(&["url1", "url2", "url3"]);
        let mut ctx = create_test_context();

        handle_edit_mode(&state, &mut ctx);

        assert!(ctx.queue_edit_mode);
        assert_eq!(ctx.queue_selected_index, 0);
        assert_eq!(ctx.queue_scroll, 0);
    }

    #[test]
    fn test_edit_mode_move_up_moves_link_toward_top_of_panel() {
        // Displayed newest first: url3, url2, url1. Selecting url2 and pressing K
        // should swap it above url3 on screen, i.e. after url3 in the queue.
        let state = state_with_queue(&["url1", "url2", "url3"]);
        let mut ctx = drawn_context(3, 3);
        ctx.queue_edit_mode = true;
        ctx.queue_selected_index = 1;

        handle_edit_mode_input(KeyCode::Char('k'), &state, &mut ctx);

        let queue = state.get_queue().expect("failed to read queue");
        assert_eq!(queue, vec!["url1", "url3", "url2"]);
        assert_eq!(ctx.queue_selected_index, 0);
    }

    #[test]
    fn test_edit_mode_move_down_moves_link_toward_bottom_of_panel() {
        let state = state_with_queue(&["url1", "url2", "url3"]);
        let mut ctx = drawn_context(3, 3);
        ctx.queue_edit_mode = true;
        ctx.queue_selected_index = 0;

        handle_edit_mode_input(KeyCode::Char('j'), &state, &mut ctx);

        let queue = state.get_queue().expect("failed to read queue");
        assert_eq!(queue, vec!["url1", "url3", "url2"]);
        assert_eq!(ctx.queue_selected_index, 1);
    }

    #[test]
    fn test_edit_mode_move_up_at_top_is_a_no_op() {
        let state = state_with_queue(&["url1", "url2", "url3"]);
        let mut ctx = drawn_context(3, 3);
        ctx.queue_edit_mode = true;
        ctx.queue_selected_index = 0;

        handle_edit_mode_input(KeyCode::Char('k'), &state, &mut ctx);

        assert_eq!(
            state.get_queue().expect("failed to read queue"),
            vec!["url1", "url2", "url3"]
        );
        assert_eq!(ctx.queue_selected_index, 0);
    }

    #[test]
    fn test_edit_mode_move_down_at_bottom_is_a_no_op() {
        let state = state_with_queue(&["url1", "url2", "url3"]);
        let mut ctx = drawn_context(3, 3);
        ctx.queue_edit_mode = true;
        ctx.queue_selected_index = 2;

        handle_edit_mode_input(KeyCode::Char('j'), &state, &mut ctx);

        assert_eq!(
            state.get_queue().expect("failed to read queue"),
            vec!["url1", "url2", "url3"]
        );
        assert_eq!(ctx.queue_selected_index, 2);
    }

    #[test]
    fn test_edit_mode_delete_removes_the_displayed_row() {
        // Top of the panel is the newest link, url3
        let state = state_with_queue(&["url1", "url2", "url3"]);
        let mut ctx = drawn_context(3, 3);
        ctx.queue_edit_mode = true;
        ctx.queue_selected_index = 0;

        handle_edit_mode_input(KeyCode::Char('d'), &state, &mut ctx);

        assert_eq!(
            state.get_queue().expect("failed to read queue"),
            vec!["url1", "url2"]
        );
    }

    #[test]
    fn test_edit_mode_targets_the_drawn_row_when_links_arrive_mid_frame() {
        // The panel was drawn from 3 links with the newest, url3, on the top row.
        // Two more links land before the keypress is handled; deleting row 0 must
        // still remove url3, not a link the user has never seen.
        let state = state_with_queue(&["url1", "url2", "url3"]);
        let mut ctx = drawn_context(3, 3);
        ctx.queue_edit_mode = true;
        ctx.queue_selected_index = 0;

        let _ = state.send(StateMessage::AddToQueue("url4".to_string()));
        let _ = state.send(StateMessage::AddToQueue("url5".to_string()));
        thread::sleep(Duration::from_millis(50));

        handle_edit_mode_input(KeyCode::Char('d'), &state, &mut ctx);

        assert_eq!(
            state.get_queue().expect("failed to read queue"),
            vec!["url1", "url2", "url4", "url5"]
        );
    }

    #[test]
    fn test_edit_mode_end_selects_oldest_link() {
        let state = state_with_queue(&["url1", "url2", "url3"]);
        let mut ctx = drawn_context(3, 2);
        ctx.queue_edit_mode = true;

        handle_edit_mode_input(KeyCode::End, &state, &mut ctx);

        assert_eq!(ctx.queue_selected_index, 2);
        // The view follows the selection off the bottom of the panel
        assert_eq!(ctx.queue_scroll, 1);
    }

    #[test]
    fn test_filter_scrolls_matches_into_view() {
        // Matches can sit anywhere in a long list, and the filter dims rather than
        // hides, so a filter typed while scrolled must bring the matches on screen
        let state = state_with_queue(&[
            "https://example.com/match-me",
            "https://other.com/a",
            "https://other.com/b",
            "https://other.com/c",
            "https://other.com/d",
            "https://other.com/e",
        ]);
        let mut ctx = drawn_context(6, 2);
        ctx.filter_mode = true;

        for c in "match".chars() {
            handle_filter_mode_input(KeyCode::Char(c), &state, &mut ctx);
        }

        assert_eq!(ctx.filtered_indices, vec![0]);
        // The only match is the oldest link, on the bottom display row
        assert_eq!(ctx.queue_scroll, 4);
    }

    #[test]
    fn test_clearing_the_filter_returns_to_the_newest_links() {
        let state = state_with_queue(&["url1", "url2", "url3", "url4", "url5", "url6"]);
        let mut ctx = drawn_context(6, 2);
        ctx.filter_mode = true;
        ctx.filter_text = "url1".to_string();
        ctx.queue_scroll = 4;

        handle_filter_mode_input(KeyCode::Esc, &state, &mut ctx);

        assert_eq!(ctx.queue_scroll, 0);
    }

    #[test]
    fn test_backspacing_the_filter_empty_returns_to_the_top() {
        let state = state_with_queue(&["url1", "url2", "url3", "url4", "url5", "url6"]);
        let mut ctx = drawn_context(6, 2);
        ctx.filter_mode = true;
        ctx.filter_text = "u".to_string();
        ctx.queue_scroll = 4;

        handle_filter_mode_input(KeyCode::Backspace, &state, &mut ctx);

        assert!(ctx.filter_text.is_empty());
        assert_eq!(ctx.queue_scroll, 0);
    }

    // ==================== Pending List Scrolling Tests ====================

    #[test]
    fn test_normal_mode_arrows_scroll_pending_list() {
        let state = state_with_queue(&["url1", "url2", "url3", "url4", "url5"]);
        let mut ctx = create_test_context();
        ctx.queue_view_height = 2;
        let mut download_state = DownloadState::default();
        let mut force_quit_state = ForceQuitState::default();
        let mut last_tick = Instant::now();
        let args = create_test_args();

        let mut nmc = create_test_nmc(
            &mut ctx,
            &mut download_state,
            &mut force_quit_state,
            &mut last_tick,
            Duration::from_millis(100),
        );

        handle_normal_mode_input(KeyCode::Down, &state, &args, &mut nmc);
        handle_normal_mode_input(KeyCode::Down, &state, &args, &mut nmc);
        assert_eq!(nmc.ctx.queue_scroll, 2);

        handle_normal_mode_input(KeyCode::Up, &state, &args, &mut nmc);
        assert_eq!(nmc.ctx.queue_scroll, 1);

        // End stops at the last full page (5 items, 2 rows visible)
        handle_normal_mode_input(KeyCode::End, &state, &args, &mut nmc);
        assert_eq!(nmc.ctx.queue_scroll, 3);

        handle_normal_mode_input(KeyCode::Home, &state, &args, &mut nmc);
        assert_eq!(nmc.ctx.queue_scroll, 0);

        handle_normal_mode_input(KeyCode::PageDown, &state, &args, &mut nmc);
        assert_eq!(nmc.ctx.queue_scroll, 2);
    }

    // Wheel event landing at (column, row) on screen
    fn wheel_at(kind: MouseEventKind, column: u16, row: u16) -> MouseEvent {
        MouseEvent {
            kind,
            column,
            row,
            modifiers: crossterm::event::KeyModifiers::NONE,
        }
    }

    #[test]
    fn test_mouse_wheel_scrolls_pending_list() {
        let state = state_with_queue(&["url1", "url2", "url3", "url4", "url5", "url6"]);
        let mut ctx = drawn_context(6, 2);

        handle_mouse_input(wheel_at(MouseEventKind::ScrollDown, 5, 2), &state, &mut ctx);
        assert_eq!(ctx.queue_scroll, 3);

        handle_mouse_input(wheel_at(MouseEventKind::ScrollUp, 5, 2), &state, &mut ctx);
        assert_eq!(ctx.queue_scroll, 0);
    }

    #[test]
    fn test_mouse_wheel_outside_the_pending_panel_is_ignored() {
        let state = state_with_queue(&["url1", "url2", "url3", "url4", "url5", "url6"]);
        // Panel occupies columns 0..40, rows 0..4
        let mut ctx = drawn_context(6, 2);

        // Over the Logs pane, well below the panel
        handle_mouse_input(
            wheel_at(MouseEventKind::ScrollDown, 5, 20),
            &state,
            &mut ctx,
        );
        assert_eq!(ctx.queue_scroll, 0);

        // Over the Active Downloads pane, to the right of the panel
        handle_mouse_input(
            wheel_at(MouseEventKind::ScrollDown, 60, 2),
            &state,
            &mut ctx,
        );
        assert_eq!(ctx.queue_scroll, 0);
    }

    #[test]
    fn test_mouse_wheel_in_edit_mode_moves_the_selection() {
        // The wheel must not leave the highlighted row off-screen, or D would
        // delete a link the user cannot see.
        let state = state_with_queue(&["url1", "url2", "url3", "url4", "url5", "url6"]);
        let mut ctx = drawn_context(6, 2);
        ctx.queue_edit_mode = true;

        handle_mouse_input(wheel_at(MouseEventKind::ScrollDown, 5, 2), &state, &mut ctx);
        assert_eq!(ctx.queue_selected_index, 3);
        assert_eq!(ctx.queue_scroll, 2);
        assert!(ctx.queue_selected_index >= ctx.queue_scroll);
        assert!(ctx.queue_selected_index < ctx.queue_scroll + ctx.queue_view_height);

        handle_mouse_input(wheel_at(MouseEventKind::ScrollUp, 5, 2), &state, &mut ctx);
        assert_eq!(ctx.queue_selected_index, 0);
        assert_eq!(ctx.queue_scroll, 0);
    }

    #[test]
    fn test_mouse_wheel_in_edit_mode_stops_at_the_oldest_link() {
        let state = state_with_queue(&["url1", "url2", "url3"]);
        let mut ctx = drawn_context(3, 2);
        ctx.queue_edit_mode = true;

        handle_mouse_input(wheel_at(MouseEventKind::ScrollDown, 5, 2), &state, &mut ctx);

        assert_eq!(ctx.queue_selected_index, 2);
    }

    #[test]
    fn test_edit_mode_esc_exits() {
        let state = create_test_state();
        let mut ctx = create_test_context();
        ctx.queue_edit_mode = true;

        let result = handle_edit_mode_input(KeyCode::Esc, &state, &mut ctx);

        assert!(!ctx.queue_edit_mode);
        assert!(matches!(result, InputResult::Continue));
    }

    #[test]
    fn test_edit_mode_enter_exits() {
        let state = create_test_state();
        let mut ctx = create_test_context();
        ctx.queue_edit_mode = true;

        handle_edit_mode_input(KeyCode::Enter, &state, &mut ctx);

        assert!(!ctx.queue_edit_mode);
    }

    #[test]
    fn test_edit_mode_e_exits() {
        let state = create_test_state();
        let mut ctx = create_test_context();
        ctx.queue_edit_mode = true;

        handle_edit_mode_input(KeyCode::Char('e'), &state, &mut ctx);

        assert!(!ctx.queue_edit_mode);
    }

    // ==================== Help Overlay Tests ====================

    #[test]
    fn test_help_overlay_f1_closes() {
        let mut show_help = true;

        let result = handle_help_overlay_input(KeyCode::F(1), &mut show_help);

        assert!(!show_help);
        assert!(matches!(result, InputResult::Continue));
    }

    #[test]
    fn test_help_overlay_esc_closes() {
        let mut show_help = true;

        let result = handle_help_overlay_input(KeyCode::Esc, &mut show_help);

        assert!(!show_help);
        assert!(matches!(result, InputResult::Continue));
    }

    #[test]
    fn test_help_overlay_other_keys_continue() {
        let mut show_help = true;

        let result = handle_help_overlay_input(KeyCode::Char('a'), &mut show_help);

        // Help should still be showing (other keys don't close it)
        assert!(show_help);
        assert!(matches!(result, InputResult::Continue));
    }

    // ==================== Normal Mode Key Mapping Tests ====================

    #[test]
    fn test_normal_mode_f1_shows_help() {
        let state = create_test_state();
        let args = create_test_args();
        let mut ctx = create_test_context();
        let mut download_state = DownloadState::default();
        let mut force_quit_state = ForceQuitState::default();
        let mut last_tick = Instant::now();
        let tick_rate = Duration::from_millis(100);

        let mut nmc = create_test_nmc(
            &mut ctx,
            &mut download_state,
            &mut force_quit_state,
            &mut last_tick,
            tick_rate,
        );
        let result = handle_normal_mode_input(KeyCode::F(1), &state, &args, &mut nmc);

        assert!(ctx.show_help);
        assert!(matches!(result, InputResult::Continue));
    }

    #[test]
    fn test_normal_mode_q_graceful_quit() {
        let state = create_test_state();
        let args = create_test_args();
        let mut ctx = create_test_context();
        let mut download_state = DownloadState::default();
        let mut force_quit_state = ForceQuitState::default();
        let mut last_tick = Instant::now();
        let tick_rate = Duration::from_millis(100);

        let mut nmc = create_test_nmc(
            &mut ctx,
            &mut download_state,
            &mut force_quit_state,
            &mut last_tick,
            tick_rate,
        );
        let result = handle_normal_mode_input(KeyCode::Char('q'), &state, &args, &mut nmc);

        assert!(download_state.await_downloads_on_exit);
        assert!(matches!(result, InputResult::Break));
    }

    #[test]
    fn test_normal_mode_shift_q_initiates_force_quit() {
        let state = create_test_state();
        let args = create_test_args();
        let mut ctx = create_test_context();
        let mut download_state = DownloadState::default();
        let mut force_quit_state = ForceQuitState::default();
        let mut last_tick = Instant::now();
        let tick_rate = Duration::from_millis(100);

        let mut nmc = create_test_nmc(
            &mut ctx,
            &mut download_state,
            &mut force_quit_state,
            &mut last_tick,
            tick_rate,
        );
        let result = handle_normal_mode_input(KeyCode::Char('Q'), &state, &args, &mut nmc);

        assert!(force_quit_state.pending);
        assert!(force_quit_state.time.is_some());
        assert!(matches!(result, InputResult::Continue));
    }

    #[test]
    fn test_normal_mode_shift_q_confirms_force_quit() {
        let state = create_test_state();
        let args = create_test_args();
        let mut ctx = create_test_context();
        let mut download_state = DownloadState::default();
        let mut force_quit_state = ForceQuitState {
            pending: true,
            time: Some(Instant::now()),
        };
        let mut last_tick = Instant::now();
        let tick_rate = Duration::from_millis(100);

        let mut nmc = create_test_nmc(
            &mut ctx,
            &mut download_state,
            &mut force_quit_state,
            &mut last_tick,
            tick_rate,
        );
        let result = handle_normal_mode_input(KeyCode::Char('Q'), &state, &args, &mut nmc);

        assert!(matches!(result, InputResult::Break));
    }

    #[test]
    fn test_normal_mode_p_pause_toggle() {
        let state = create_test_state();
        // Start downloads first
        let _ = state.send(StateMessage::SetStarted(true));
        thread::sleep(Duration::from_millis(50));

        let args = create_test_args();
        let mut ctx = create_test_context();
        let mut download_state = DownloadState::default();
        let mut force_quit_state = ForceQuitState::default();
        let mut last_tick = Instant::now();
        let tick_rate = Duration::from_millis(100);

        let mut nmc = create_test_nmc(
            &mut ctx,
            &mut download_state,
            &mut force_quit_state,
            &mut last_tick,
            tick_rate,
        );
        let result = handle_normal_mode_input(KeyCode::Char('p'), &state, &args, &mut nmc);

        assert!(matches!(result, InputResult::Continue));
    }

    #[test]
    fn test_normal_mode_slash_enters_filter_mode() {
        let state = create_test_state();
        let args = create_test_args();
        let mut ctx = create_test_context();
        let mut download_state = DownloadState::default();
        let mut force_quit_state = ForceQuitState::default();
        let mut last_tick = Instant::now();
        let tick_rate = Duration::from_millis(100);

        let mut nmc = create_test_nmc(
            &mut ctx,
            &mut download_state,
            &mut force_quit_state,
            &mut last_tick,
            tick_rate,
        );
        let result = handle_normal_mode_input(KeyCode::Char('/'), &state, &args, &mut nmc);

        assert!(ctx.filter_mode);
        assert!(ctx.filter_text.is_empty());
        assert!(ctx.filtered_indices.is_empty());
        assert!(matches!(result, InputResult::Continue));
    }

    #[test]
    fn test_normal_mode_f2_returns_unhandled() {
        let state = create_test_state();
        let args = create_test_args();
        let mut ctx = create_test_context();
        let mut download_state = DownloadState::default();
        let mut force_quit_state = ForceQuitState::default();
        let mut last_tick = Instant::now();
        let tick_rate = Duration::from_millis(100);

        let mut nmc = create_test_nmc(
            &mut ctx,
            &mut download_state,
            &mut force_quit_state,
            &mut last_tick,
            tick_rate,
        );
        let result = handle_normal_mode_input(KeyCode::F(2), &state, &args, &mut nmc);

        // F2 returns Unhandled so the caller can toggle settings menu
        assert!(matches!(result, InputResult::Unhandled));
    }

    #[test]
    fn test_normal_mode_unknown_key_unhandled() {
        let state = create_test_state();
        let args = create_test_args();
        let mut ctx = create_test_context();
        let mut download_state = DownloadState::default();
        let mut force_quit_state = ForceQuitState::default();
        let mut last_tick = Instant::now();
        let tick_rate = Duration::from_millis(100);

        let mut nmc = create_test_nmc(
            &mut ctx,
            &mut download_state,
            &mut force_quit_state,
            &mut last_tick,
            tick_rate,
        );
        let result = handle_normal_mode_input(KeyCode::Char('z'), &state, &args, &mut nmc);

        assert!(matches!(result, InputResult::Unhandled));
    }

    // ==================== yt-dlp Update Tests ====================

    #[test]
    fn test_normal_mode_u_handled() {
        let state = create_test_state();
        let args = create_test_args();
        let mut ctx = create_test_context();
        let mut download_state = DownloadState::default();
        let mut force_quit_state = ForceQuitState::default();
        let mut last_tick = Instant::now();
        let tick_rate = Duration::from_millis(100);

        let mut nmc = create_test_nmc(
            &mut ctx,
            &mut download_state,
            &mut force_quit_state,
            &mut last_tick,
            tick_rate,
        );
        let result = handle_normal_mode_input(KeyCode::Char('u'), &state, &args, &mut nmc);

        assert!(matches!(result, InputResult::Continue));
    }

    #[test]
    fn test_normal_mode_u_blocked_when_downloads_active() {
        let state = create_test_state();
        let _ = state.send(StateMessage::SetStarted(true));
        thread::sleep(Duration::from_millis(50));

        let args = create_test_args();
        let mut ctx = create_test_context();
        let mut download_state = DownloadState::default();
        let mut force_quit_state = ForceQuitState::default();
        let mut last_tick = Instant::now();
        let tick_rate = Duration::from_millis(100);

        let mut nmc = create_test_nmc(
            &mut ctx,
            &mut download_state,
            &mut force_quit_state,
            &mut last_tick,
            tick_rate,
        );
        let result = handle_normal_mode_input(KeyCode::Char('u'), &state, &args, &mut nmc);

        assert!(matches!(result, InputResult::Continue));

        // Check that a "Cannot update" log was added
        let snapshot = state
            .get_ui_snapshot()
            .expect("failed to build UI snapshot");
        assert!(
            snapshot
                .logs
                .iter()
                .any(|l| l.contains("Cannot update while downloads are active"))
        );
    }

    // ==================== Retry Failed Tests ====================

    #[test]
    fn test_normal_mode_t_handled() {
        let state = create_test_state();
        let args = create_test_args();
        let mut ctx = create_test_context();
        let mut download_state = DownloadState::default();
        let mut force_quit_state = ForceQuitState::default();
        let mut last_tick = Instant::now();
        let tick_rate = Duration::from_millis(100);

        let mut nmc = create_test_nmc(
            &mut ctx,
            &mut download_state,
            &mut force_quit_state,
            &mut last_tick,
            tick_rate,
        );
        let result = handle_normal_mode_input(KeyCode::Char('t'), &state, &args, &mut nmc);

        assert!(matches!(result, InputResult::Continue));
    }

    #[test]
    fn test_normal_mode_t_requeues_failed() {
        let state = create_test_state();

        // Add failed downloads
        state
            .send(StateMessage::AddFailedDownload(
                "https://example.com/video1".to_string(),
            ))
            .expect("failed to send AddFailedDownload");
        state
            .send(StateMessage::AddFailedDownload(
                "https://example.com/video2".to_string(),
            ))
            .expect("failed to send AddFailedDownload");
        thread::sleep(Duration::from_millis(50));

        let args = create_test_args();
        let mut ctx = create_test_context();
        let mut download_state = DownloadState::default();
        let mut force_quit_state = ForceQuitState::default();
        let mut last_tick = Instant::now();
        let tick_rate = Duration::from_millis(100);

        let mut nmc = create_test_nmc(
            &mut ctx,
            &mut download_state,
            &mut force_quit_state,
            &mut last_tick,
            tick_rate,
        );
        handle_normal_mode_input(KeyCode::Char('t'), &state, &args, &mut nmc);

        // Wait for message processing
        thread::sleep(Duration::from_millis(100));

        // Verify URLs were re-queued
        let queue = state.get_queue().expect("failed to read queue");
        assert_eq!(queue.len(), 2);

        // Failed count should be 0 after take
        let snapshot = state
            .get_ui_snapshot()
            .expect("failed to build UI snapshot");
        assert_eq!(snapshot.failed_count, 0);
    }

    #[test]
    fn test_normal_mode_t_no_failed_logs_message() {
        let state = create_test_state();
        let args = create_test_args();
        let mut ctx = create_test_context();
        let mut download_state = DownloadState::default();
        let mut force_quit_state = ForceQuitState::default();
        let mut last_tick = Instant::now();
        let tick_rate = Duration::from_millis(100);

        let mut nmc = create_test_nmc(
            &mut ctx,
            &mut download_state,
            &mut force_quit_state,
            &mut last_tick,
            tick_rate,
        );
        handle_normal_mode_input(KeyCode::Char('t'), &state, &args, &mut nmc);

        let snapshot = state
            .get_ui_snapshot()
            .expect("failed to build UI snapshot");
        assert!(
            snapshot
                .logs
                .iter()
                .any(|l| l.contains("No failed downloads to retry"))
        );
    }

    #[test]
    fn test_normal_mode_t_blocked_when_downloads_active() {
        let state = create_test_state();
        let _ = state.send(StateMessage::SetStarted(true));
        thread::sleep(Duration::from_millis(50));

        let args = create_test_args();
        let mut ctx = create_test_context();
        let mut download_state = DownloadState::default();
        let mut force_quit_state = ForceQuitState::default();
        let mut last_tick = Instant::now();
        let tick_rate = Duration::from_millis(100);

        let mut nmc = create_test_nmc(
            &mut ctx,
            &mut download_state,
            &mut force_quit_state,
            &mut last_tick,
            tick_rate,
        );
        handle_normal_mode_input(KeyCode::Char('t'), &state, &args, &mut nmc);

        let snapshot = state
            .get_ui_snapshot()
            .expect("failed to build UI snapshot");
        assert!(
            snapshot
                .logs
                .iter()
                .any(|l| l.contains("Cannot retry while downloads are active"))
        );
    }

    // ==================== DownloadState Tests ====================

    #[test]
    fn test_download_state_default() {
        let state = DownloadState::default();
        assert!(state.download_thread_handle.is_none());
        assert!(!state.await_downloads_on_exit);
    }
}
