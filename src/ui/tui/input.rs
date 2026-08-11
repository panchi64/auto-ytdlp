use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
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

/// Whether workers are actively pulling URLs from the queue.
///
/// A paused session is not running: workers stop popping new URLs while paused.
fn downloads_running(state: &AppState) -> bool {
    state.is_started().unwrap_or(false)
        && !state.is_paused().unwrap_or(false)
        && !state.is_completed().unwrap_or(false)
}

/// Whether any yt-dlp process may still be executing.
///
/// Pausing only stops workers from popping *new* URLs — subprocesses already
/// spawned keep downloading — so operations that disturb the yt-dlp binary or
/// the failed-download list must treat a paused session as still in flight.
fn downloads_in_flight(state: &AppState) -> bool {
    state.is_started().unwrap_or(false) && !state.is_completed().unwrap_or(false)
}

fn handle_start_stop(state: &AppState, args: &Args, download_state: &mut DownloadState) {
    if let Ok(is_started) = state.is_started() {
        if !is_started {
            // The started flag is set by the controller, so it lags this keypress
            // slightly; a live thread handle is the authoritative "already running"
            // signal and keeps a fast second S from spawning a duplicate controller.
            if download_state
                .download_thread_handle
                .as_ref()
                .is_some_and(|handle| !handle.is_finished())
            {
                return;
            }

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

            // Wait for downloads to finish off the UI thread: joining here would
            // freeze rendering and input until every yt-dlp process exits.
            let handle = download_state.download_thread_handle.take();
            let state_clone = state.clone();
            thread::spawn(move || {
                if let Some(handle) = handle {
                    if let Err(e) = handle.join() {
                        if let Err(log_err) = state_clone
                            .add_log(format!("Error joining download thread on stop: {:?}", e))
                        {
                            eprintln!("Error adding log: {}", log_err);
                        }
                    } else if let Err(e) =
                        state_clone.add_log("Downloads stopped gracefully.".to_string())
                    {
                        eprintln!("Error adding log: {}", e);
                    }
                }

                // Clear logs after a short delay when manually stopping downloads
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
    if !downloads_running(state) {
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
                let msg = if links_added > 0 {
                    if let Err(e) = state.send(StateMessage::SetCompleted(false)) {
                        eprintln!("Error setting completed flag: {}", e);
                    }
                    if downloads_running(state) {
                        format!("Queued {} new URLs", links_added)
                    } else {
                        format!("Added {} URLs", links_added)
                    }
                } else {
                    "No new URLs found in clipboard".to_string()
                };
                let _ = state.show_toast(&msg);
                if let Err(e) = state.add_log(msg) {
                    eprintln!("Error adding log: {}", e);
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

/// Guards against overlapping `yt-dlp -U` runs: two updaters rewriting the same
/// binary concurrently can leave a corrupt executable on PATH.
static YTDLP_UPDATE_IN_FLIGHT: AtomicBool = AtomicBool::new(false);

fn handle_ytdlp_update(state: &AppState) {
    if downloads_in_flight(state) {
        if let Err(e) = state.add_log("Cannot update while downloads are active".to_string()) {
            eprintln!("Error adding log: {}", e);
        }
        return;
    }

    if YTDLP_UPDATE_IN_FLIGHT
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        if let Err(e) = state.add_log("yt-dlp update already in progress".to_string()) {
            eprintln!("Error adding log: {}", e);
        }
        return;
    }

    if let Err(e) = state.add_log("Checking for yt-dlp updates...".to_string()) {
        eprintln!("Error adding log: {}", e);
    }

    let state_clone = state.clone();
    thread::spawn(move || {
        let result = Command::new("yt-dlp").arg("-U").output();
        YTDLP_UPDATE_IN_FLIGHT.store(false, Ordering::SeqCst);
        match result {
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
                if let Err(log_err) = state_clone.add_log(format!("Failed to run yt-dlp -U: {}", e))
                {
                    eprintln!("Error adding log: {}", log_err);
                }
                let _ = state_clone.show_toast("yt-dlp update failed");
            }
        }
    });
}

fn handle_retry_failed(state: &AppState) {
    if downloads_in_flight(state) {
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
                // There is pending work again, so the run is no longer complete.
                if let Err(e) = state.send(StateMessage::SetCompleted(false)) {
                    eprintln!("Error setting completed flag: {}", e);
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
    if !downloads_running(state) {
        let queue_len = state.get_queue().map(|q| q.len()).unwrap_or(0);
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
mod tests;
