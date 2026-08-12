use std::thread;
use std::time::{Duration, Instant};

use crate::app_state::{AppState, StateMessage};
use crate::args::Args;
use crate::downloader::{common::validate_dependencies, queue::process_queue};
use crate::utils::file::get_links_from_file;

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
/// Pausing only stops workers from popping *new* URLs — subprocesses already
/// spawned keep downloading — so operations that disturb the yt-dlp binary or
/// the failed-download list must treat a paused session as still in flight.
pub(in crate::ui::tui::input) fn downloads_in_flight(state: &AppState) -> bool {
    state.is_started().unwrap_or(false) && !state.is_completed().unwrap_or(false)
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
