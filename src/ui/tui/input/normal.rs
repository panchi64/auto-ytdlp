use std::time::Instant;

use crossterm::event::KeyCode;

use crate::app_state::{AppState, StateMessage};
use crate::args::Args;

use super::actions::{
    handle_add_clipboard, handle_load_file, handle_pause_resume, handle_reload,
    handle_retry_failed, handle_start_stop, handle_ytdlp_update,
};
use super::edit::handle_edit_mode;
use super::mouse::scroll_queue;
use super::{InputResult, NormalModeContext};

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
