use std::thread;
use std::time::{Duration, Instant};

use crate::app_state::{AppState, StateMessage};
use crate::args::Args;
use crate::downloader::{common::validate_dependencies, queue::process_queue};
use crate::utils::file::load_links_into_queue;

use super::super::DownloadState;

/// Whether workers are actively pulling URLs from the queue.
///
/// A paused session is not running: workers stop popping new URLs while paused.
pub(in crate::ui::tui::input) fn downloads_running(state: &AppState) -> bool {
    state.is_started().unwrap_or(false)
        && !state.is_paused().unwrap_or(false)
        && !state.is_completed().unwrap_or(false)
}

/// Whether any yt-dlp process may still be executing.
///
/// Neither `started` nor `paused` can answer this. Pausing only stops workers
/// popping *new* URLs - subprocesses already spawned keep downloading - and the
/// stop keypress clears `started` while the drain is still under way. The
/// controller flag covers both: it is set before the controller spawns and
/// cleared only after every worker has been joined.
///
/// A lock failure reads as "in flight" so a guard errs towards refusing rather
/// than disturbing a live download.
pub(in crate::ui::tui::input) fn downloads_in_flight(state: &AppState) -> bool {
    state.is_controller_active().unwrap_or(true)
        || (state.is_started().unwrap_or(false) && !state.is_completed().unwrap_or(false))
        || state.active_download_count().map(|n| n > 0).unwrap_or(true)
}

pub(in crate::ui::tui::input) fn handle_start_stop(
    state: &AppState,
    args: &Args,
    download_state: &mut DownloadState,
) {
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
                // Say so: after a stop the header reads STOPPED while the drain
                // runs on for minutes, and a silent return looks like a dead key.
                if let Err(e) = state.add_log(
                    "Still finishing the previous run - downloads will not restart until it drains"
                        .to_string(),
                ) {
                    eprintln!("Error adding log: {}", e);
                }
                let _ = state.show_toast("Still stopping the previous run");
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

            // Watch for the drain off the UI thread: joining here would freeze
            // rendering and input until every yt-dlp process exits. The handle
            // deliberately stays in `download_state` - a graceful quit joins it,
            // and the duplicate-controller guard above reads it.
            let state_clone = state.clone();
            thread::spawn(move || {
                // Waits on the controller, not on the active-download set: that
                // set is momentarily empty between two URLs, which would declare
                // the drain finished while a worker was still spawning.
                while downloads_in_flight(&state_clone) {
                    thread::sleep(Duration::from_millis(100));
                }
                if let Err(e) = state_clone.add_log("Downloads stopped gracefully.".to_string()) {
                    eprintln!("Error adding log: {}", e);
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

pub(in crate::ui::tui::input) fn handle_pause_resume(
    state: &AppState,
    last_tick: &mut Instant,
    tick_rate: Duration,
) {
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

pub(in crate::ui::tui::input) fn handle_reload(
    state: &AppState,
    last_tick: &mut Instant,
    tick_rate: Duration,
) {
    // Guarded on in-flight, not running: a paused session still has yt-dlp
    // subprocesses writing files, and replacing the queue would re-queue the URL
    // one of them is already downloading.
    if downloads_in_flight(state) {
        if let Err(e) = state.add_log("Cannot reload links while downloads are active".to_string())
        {
            eprintln!("Error adding log: {}", e);
        }
        return;
    }

    if let Err(e) = state.reset_for_new_run() {
        eprintln!("Error resetting state: {}", e);
    }

    match load_links_into_queue(state, false) {
        Ok(count) => {
            if let Err(e) = state.add_log(format!("Links refreshed from file ({})", count)) {
                eprintln!("Error adding log: {}", e);
            }
        }
        Err(e) => {
            if let Err(log_err) = state.add_log(format!("Could not refresh links: {}", e)) {
                eprintln!("Error adding log: {}", log_err);
            }
        }
    }
    *last_tick = Instant::now() - tick_rate;
}
