use std::{
    io::{BufRead, BufReader},
    process::{Command, Stdio},
    time::Instant,
};

use crate::{
    app_state::{AppState, DownloadProgress, StateMessage},
    args::Args,
    utils::display::truncate_url_for_display,
    utils::file::remove_link_from_file_sync,
};

use super::{
    common::build_ytdlp_command_args,
    progress_parser::{ParsedOutput, parse_ytdlp_line, progress_info_to_download_progress},
};

/// Minimum interval between progress updates to reduce lock contention (250ms)
const PROGRESS_UPDATE_INTERVAL_MS: u64 = 250;

/// Check if force quit has been requested
#[inline]
fn should_abort(state: &AppState) -> bool {
    state.is_force_quit().unwrap_or(false)
}

/// Log a message to the TUI, printing to stderr on failure
fn log_msg(state: &AppState, msg: impl Into<String>) {
    if let Err(e) = state.add_log(msg.into()) {
        eprintln!("Error adding log: {}", e);
    }
}

/// Downloads a single video from the provided URL using yt-dlp.
///
/// This function handles the entire download process for a single URL:
/// 1. Triggers the addition of a URL to the active downloads in the app state
/// 2. Logs the start of the download
/// 3. Spawns a yt-dlp process with appropriate arguments
/// 4. Captures and logs relevant output from yt-dlp
/// 5. Handles process completion, success/failure status
/// 6. Triggers the updates to the download statistics in the app state
/// 7. Triggers the removal of the downloaded URL from the links.txt file if successful
///
/// # Parameters
///
/// * `url` - The URL of the video to download
/// * `state` - The application state to update during download
/// * `args` - Command line arguments containing download settings
///
/// # Example
///
/// ```
/// if let Some(url) = state_clone.pop_queue() {
///     download_worker(url, state_clone.clone(), args_clone.clone());
/// }
/// ```
///
/// # Notes
///
/// This function will exit early if `force_quit` is set in the application state.
/// It updates the progress and completed status in the app state after completion.
pub fn download_worker(url: String, state: AppState, args: Args) {
    if should_abort(&state) {
        return;
    }

    if let Err(e) = state.send(StateMessage::AddActiveDownload(url.clone())) {
        eprintln!("Error adding active download: {}", e);
    }

    log_msg(&state, format!("Starting download: {}", url));

    let settings = state.get_settings().unwrap_or_default();
    let max_retries = if settings.network_retry { 3 } else { 0 };
    let retry_delay = settings.retry_delay;
    let mut retry_count = 0;
    let mut success = false;

    while retry_count <= max_retries {
        if should_abort(&state) {
            log_msg(
                &state,
                format!("Force quit: Aborting download task for {}.", url),
            );
            break;
        }

        if retry_count > 0 {
            log_msg(
                &state,
                format!("Retry attempt {} for: {}", retry_count, url),
            );
        }

        let cmd_args = build_ytdlp_command_args(&args, &settings, &url);
        let mut cmd = match Command::new("yt-dlp")
            .args(&cmd_args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(cmd) => cmd,
            Err(e) => {
                log_msg(
                    &state,
                    format!(
                        "Error spawning yt-dlp for {}: {}. Aborting this URL.",
                        url, e
                    ),
                );
                break;
            }
        };

        if should_abort(&state) {
            log_msg(
                &state,
                format!(
                    "Force quit: Killing spawned process for {} and aborting.",
                    url
                ),
            );
            let _ = cmd.kill();
            let _ = cmd.wait();
            break;
        }

        let stdout = match cmd.stdout.take() {
            Some(stdout) => stdout,
            None => {
                log_msg(
                    &state,
                    format!(
                        "Error: Could not take stdout for {}. Aborting this attempt.",
                        url
                    ),
                );
                if !should_abort(&state) {
                    let _ = cmd.kill();
                    let _ = cmd.wait();
                }
                break;
            }
        };
        let reader = BufReader::new(stdout);
        let mut is_network_error = false;
        let mut last_progress_update = Instant::now();
        let display_name = truncate_url_for_display(&url);

        for line in reader.lines().map_while(Result::ok) {
            if should_abort(&state) {
                log_msg(
                    &state,
                    format!(
                        "Force quit: Killing process during output reading for {}.",
                        url
                    ),
                );
                let _ = cmd.kill();
                let _ = cmd.wait();
                break;
            }

            // Parse the line using the progress parser
            let parsed = parse_ytdlp_line(&line);

            match parsed {
                ParsedOutput::Progress(info) => {
                    // Check network error indicators
                    if line.contains("ERROR") {
                        is_network_error = check_network_error(&line);
                    }

                    // Throttle progress updates to reduce lock contention
                    let elapsed = last_progress_update.elapsed().as_millis() as u64;
                    if elapsed >= PROGRESS_UPDATE_INTERVAL_MS || info.percent >= 100.0 {
                        last_progress_update = Instant::now();

                        let progress = progress_info_to_download_progress(&display_name, &info);
                        if let Err(e) = state.send(StateMessage::UpdateDownloadProgress {
                            url: url.clone(),
                            progress,
                        }) {
                            eprintln!("Error sending progress update: {}", e);
                        }
                    }
                }
                ParsedOutput::PostProcess(msg) => {
                    // Update phase to processing/merging
                    let mut progress = DownloadProgress::new(&url);
                    progress.phase = "processing".to_string();
                    progress.percent = 100.0;
                    if let Err(e) = state.send(StateMessage::UpdateDownloadProgress {
                        url: url.clone(),
                        progress,
                    }) {
                        eprintln!("Error sending progress update: {}", e);
                    }
                    log_msg(&state, format!("{} {}", display_name, msg));
                }
                ParsedOutput::Destination(msg) => {
                    log_msg(&state, format!("{} {}", display_name, msg));
                }
                ParsedOutput::AlreadyDownloaded(msg) => {
                    log_msg(&state, format!("{} {}", display_name, msg));
                }
                ParsedOutput::Error(msg) => {
                    is_network_error = check_network_error(&msg);
                    log_msg(&state, format!("{} Error: {}", display_name, msg));
                }
                ParsedOutput::Info(msg) => {
                    log_msg(&state, format!("{} {}", display_name, msg));
                }
                ParsedOutput::Ignore => {}
            }
        }

        if should_abort(&state) {
            log_msg(
                &state,
                format!(
                    "Force quit: Detected after output processing for {}. Ensuring kill.",
                    url
                ),
            );
            let _ = cmd.kill();
            let _ = cmd.wait();
            break;
        }

        match cmd.wait() {
            Ok(status) => {
                if status.success() {
                    success = true;
                    break;
                } else {
                    log_msg(
                        &state,
                        format!("yt-dlp exited with error for {}: {}", url, status),
                    );
                    if !settings.network_retry || !is_network_error || retry_count >= max_retries {
                        break;
                    }
                }
            }
            Err(e) => {
                log_msg(
                    &state,
                    format!(
                        "Error waiting for yt-dlp process for {}: {}. Aborting this URL.",
                        url, e
                    ),
                );
                break;
            }
        }

        retry_count += 1;
        if should_abort(&state) {
            log_msg(
                &state,
                format!("Force quit: Detected before retry sleep for {}.", url),
            );
            break;
        }
        if retry_count <= max_retries {
            if let Err(e) = state.increment_retries() {
                eprintln!("Error incrementing retries: {}", e);
            }
            std::thread::sleep(std::time::Duration::from_secs(retry_delay));
        }
    }

    if let Err(e) = state.send(StateMessage::RemoveActiveDownload(url.clone())) {
        eprintln!("Error removing active download: {}", e);
    }

    if success {
        if let Err(e) = remove_link_from_file_sync(&state, &url) {
            let _ = state.log_error(&format!("Failed to remove {} from links.txt", url), &e);
        }

        if let Err(e) = state.send(StateMessage::IncrementCompleted) {
            eprintln!("Error incrementing completed: {}", e);
        }

        log_msg(&state, format!("Completed: {}", url));
    } else if should_abort(&state) {
        log_msg(
            &state,
            format!("Download aborted due to force quit: {}", url),
        );
    } else {
        // Record failed download for retry (not force-quit)
        if let Err(e) = state.send(StateMessage::AddFailedDownload(url.clone())) {
            eprintln!("Error recording failed download: {}", e);
        }
        if retry_count > 0 {
            log_msg(
                &state,
                format!("Failed after {} retries: {}", retry_count, url),
            );
        } else {
            log_msg(&state, format!("Failed: {}", url));
        }
    }

    if state.get_queue().unwrap_or_default().is_empty()
        && state.get_active_downloads().unwrap_or_default().is_empty()
        && !should_abort(&state)
        && let Err(e) = state.send(StateMessage::SetCompleted(true))
    {
        eprintln!("Error setting completed: {}", e);
    }
}

/// Checks if an error message indicates a network-related issue
fn check_network_error(line: &str) -> bool {
    line.contains("Unable to download webpage")
        || line.contains("HTTP Error")
        || line.contains("Connection")
        || line.contains("Timeout")
        || line.contains("Network")
        || line.contains("SSL")
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== Network Error Detection ====================

    #[test]
    fn test_check_network_error_unable_to_download() {
        assert!(check_network_error("ERROR: Unable to download webpage"));
        assert!(check_network_error(
            "Unable to download webpage: HTTP Error 503"
        ));
    }

    #[test]
    fn test_check_network_error_http_error() {
        assert!(check_network_error("HTTP Error 404: Not Found"));
        assert!(check_network_error("ERROR: HTTP Error 500"));
        assert!(check_network_error("Got HTTP Error 502 Bad Gateway"));
    }

    #[test]
    fn test_check_network_error_connection() {
        assert!(check_network_error("Connection refused"));
        assert!(check_network_error("Connection reset by peer"));
        assert!(check_network_error("Connection timed out"));
        assert!(check_network_error("ERROR: Connection failed"));
    }

    #[test]
    fn test_check_network_error_timeout() {
        assert!(check_network_error("Timeout while connecting"));
        assert!(check_network_error("Request Timeout"));
        assert!(check_network_error("ERROR: Read Timeout"));
    }

    #[test]
    fn test_check_network_error_network() {
        assert!(check_network_error("Network is unreachable"));
        assert!(check_network_error("Network error occurred"));
        assert!(check_network_error("ERROR: Network failure"));
    }

    #[test]
    fn test_check_network_error_ssl() {
        assert!(check_network_error("SSL: CERTIFICATE_VERIFY_FAILED"));
        assert!(check_network_error("SSL handshake failed"));
        assert!(check_network_error("ERROR: SSL error"));
    }

    #[test]
    fn test_check_network_error_false_positives() {
        // These should NOT be detected as network errors
        assert!(!check_network_error("Video unavailable"));
        assert!(!check_network_error("This video is private"));
        assert!(!check_network_error("ERROR: Unsupported URL"));
        assert!(!check_network_error("[download] 50% of 100.00MiB"));
        assert!(!check_network_error("Downloading video info"));
        assert!(!check_network_error("Format not available"));
    }

    #[test]
    fn test_check_network_error_empty_string() {
        assert!(!check_network_error(""));
    }

    #[test]
    fn test_check_network_error_case_sensitive() {
        // The function is case-sensitive, so these should not match
        assert!(!check_network_error("http error")); // lowercase
        assert!(!check_network_error("connection")); // lowercase
        assert!(!check_network_error("timeout")); // lowercase
        assert!(!check_network_error("network")); // lowercase
        assert!(!check_network_error("ssl")); // lowercase
    }

    #[test]
    fn test_check_network_error_partial_matches() {
        // Partial/embedded matches should still work
        assert!(check_network_error("Some prefix HTTP Error suffix"));
        assert!(check_network_error("xxx Connection yyy"));
        assert!(check_network_error(
            "Error message with SSL certificate issue"
        ));
    }
}
