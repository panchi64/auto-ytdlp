use std::{
    fs,
    io::{BufRead, BufReader},
    process::{Command, Stdio},
    thread,
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

/// Cap on retained stderr. The pipe is always drained in full; only what is kept
/// for the error message is bounded, so a runaway warning loop cannot grow the
/// buffer without limit.
const STDERR_CAPTURE_LIMIT: usize = 8 * 1024;

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

/// Runs yt-dlp for one URL, streaming its progress into the app state and
/// removing the URL from links.txt on success.
///
/// Returns early if force quit is set, and retries network failures according to
/// the retry settings.
pub fn download_worker(url: String, state: AppState, args: Args) {
    if should_abort(&state) {
        return;
    }

    if let Err(e) = state.send(StateMessage::AddActiveDownload(url.clone())) {
        eprintln!("Error adding active download: {}", e);
    }

    log_msg(&state, format!("Starting download: {}", url));

    let settings = state.get_settings().unwrap_or_default();

    // The directory can be changed from the settings menu after startup, so
    // make sure it exists before handing the path to yt-dlp
    let download_dir = args.resolve_download_dir(&settings);
    if let Err(e) = fs::create_dir_all(&download_dir) {
        log_msg(
            &state,
            format!("Cannot create download directory {:?}: {}", download_dir, e),
        );
    }

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
        // Drain stderr on its own thread. yt-dlp can emit more than a pipe
        // buffer of warnings, and once that buffer fills it blocks on write -
        // it then stops producing stdout and never exits, hanging this worker
        // and the whole batch behind it.
        let stderr_drain = cmd.stderr.take().map(|stderr| {
            thread::spawn(move || {
                let mut captured = String::new();
                for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                    if captured.len() < STDERR_CAPTURE_LIMIT {
                        captured.push_str(&line);
                        captured.push('\n');
                    }
                }
                captured
            })
        });

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

        // stdout is exhausted, so yt-dlp is done writing; collecting stderr now
        // cannot block, and it is where yt-dlp reports the actual failure.
        let stderr_output = stderr_drain
            .and_then(|handle| handle.join().ok())
            .unwrap_or_default();
        if stderr_output.lines().any(check_network_error) {
            is_network_error = true;
        }

        match cmd.wait() {
            Ok(status) => {
                if status.success() {
                    success = true;
                    break;
                } else {
                    let detail = stderr_output
                        .lines()
                        .rfind(|line| line.contains("ERROR"))
                        .unwrap_or("");
                    log_msg(
                        &state,
                        format!(
                            "yt-dlp exited with error for {}: {} {}",
                            url, status, detail
                        ),
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
mod tests;
