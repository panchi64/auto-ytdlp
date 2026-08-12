use std::time::{Duration, Instant};

use crossterm::event::KeyCode;

use super::UiContext;

mod actions;
mod edit;
mod filter;
mod mouse;
mod normal;

pub use edit::handle_edit_mode_input;
pub use filter::handle_filter_mode_input;
pub use mouse::handle_mouse_input;
pub use normal::handle_normal_mode_input;

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

/// Context for normal mode input handling, grouping related mutable state
pub struct NormalModeContext<'a> {
    pub ctx: &'a mut UiContext,
    pub download_state: &'a mut DownloadState,
    pub force_quit_state: &'a mut ForceQuitState,
    pub last_tick: &'a mut Instant,
    pub tick_rate: Duration,
}

#[cfg(test)]
mod tests;
