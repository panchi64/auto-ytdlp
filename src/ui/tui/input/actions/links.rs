use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use arboard::Clipboard;

use crate::app_state::{AppState, StateMessage};
use crate::errors::AppError;
use crate::utils::file::{add_clipboard_links, get_links_from_file, sanitize_links_file};

use super::downloads::{downloads_in_flight, downloads_running};

pub(in crate::ui::tui::input) fn handle_load_file(
    state: &AppState,
    last_tick: &mut Instant,
    tick_rate: Duration,
) {
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

pub(in crate::ui::tui::input) fn handle_add_clipboard(state: &AppState) {
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

pub(in crate::ui::tui::input) fn handle_ytdlp_update(state: &AppState) {
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

pub(in crate::ui::tui::input) fn handle_retry_failed(state: &AppState) {
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
