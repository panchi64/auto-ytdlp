use std::{
    io::{BufRead, BufReader},
    process::{Command, Stdio},
};

use crate::{
    app_state::{AppState, StateMessage},
    args::Args,
    utils::file::remove_link_from_file,
};

use super::common::build_ytdlp_command_args;

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
    if state.is_force_quit().unwrap_or(false) {
        return;
    }

    if let Err(e) = state.send(StateMessage::AddActiveDownload(url.clone())) {
        eprintln!("Error adding active download: {}", e);
    }

    if let Err(e) = state.add_log(format!("Starting download: {}", url)) {
        eprintln!("Error adding log: {}", e);
    }

    let settings = state.get_settings().unwrap_or_default();
    let max_retries = if settings.network_retry { 3 } else { 0 };
    let retry_delay = settings.retry_delay;
    let mut retry_count = 0;
    let mut success = false;

    while retry_count <= max_retries {
        if state.is_force_quit().unwrap_or(false) {
            if let Err(e) =
                state.add_log(format!("Force quit: Aborting download task for {}.", url))
            {
                eprintln!("Error adding log: {}", e);
            }
            break;
        }

        if retry_count > 0 {
            if let Err(e) = state.add_log(format!("Retry attempt {} for: {}", retry_count, url)) {
                eprintln!("Error adding log: {}", e);
            }
        }

        let cmd_args = build_ytdlp_command_args(&args, &url);
        let mut cmd = match Command::new("yt-dlp")
            .args(&cmd_args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(cmd) => cmd,
            Err(e) => {
                if let Err(log_err) = state.add_log(format!(
                    "Error spawning yt-dlp for {}: {}. Aborting this URL.",
                    url, e
                )) {
                    eprintln!("Error adding log: {}", log_err);
                }
                break;
            }
        };

        if state.is_force_quit().unwrap_or(false) {
            if let Err(e) = state.add_log(format!(
                "Force quit: Killing spawned process for {} and aborting.",
                url
            )) {
                eprintln!("Error adding log: {}", e);
            }
            let _ = cmd.kill();
            break;
        }

        let stdout = match cmd.stdout.take() {
            Some(stdout) => stdout,
            None => {
                if let Err(e) = state.add_log(format!(
                    "Error: Could not take stdout for {}. Aborting this attempt.",
                    url
                )) {
                    eprintln!("Error adding log: {}", e);
                }
                if !state.is_force_quit().unwrap_or(false) {
                    let _ = cmd.kill();
                    let _ = cmd.wait();
                }
                break;
            }
        };
        let reader = BufReader::new(stdout);
        let mut is_network_error = false;

        for line in reader.lines().map_while(Result::ok) {
            if state.is_force_quit().unwrap_or(false) {
                if let Err(e) = state.add_log(format!(
                    "Force quit: Killing process during output reading for {}.",
                    url
                )) {
                    eprintln!("Error adding log: {}", e);
                }
                let _ = cmd.kill();
                break;
            }

            if line.contains("ERROR")
                && (line.contains("Unable to download webpage")
                    || line.contains("HTTP Error")
                    || line.contains("Connection")
                    || line.contains("Timeout")
                    || line.contains("Network")
                    || line.contains("SSL"))
            {
                is_network_error = true;
            }

            let log_line = if line.contains("ERROR") {
                format!("Error: {}", line)
            } else if line.contains("Destination") || line.contains("[download]") {
                line
            } else {
                continue;
            };
            if let Err(e) = state.add_log(log_line) {
                eprintln!("Error adding log: {}", e);
            }
        }

        if state.is_force_quit().unwrap_or(false) {
            if let Err(e) = state.add_log(format!(
                "Force quit: Detected after output processing for {}. Ensuring kill.",
                url
            )) {
                eprintln!("Error adding log: {}", e);
            }
            let _ = cmd.kill();
            break;
        }

        match cmd.wait() {
            Ok(status) => {
                if status.success() {
                    success = true;
                    break;
                } else {
                    if let Err(e) =
                        state.add_log(format!("yt-dlp exited with error for {}: {}", url, status))
                    {
                        eprintln!("Error adding log: {}", e);
                    }
                    if !settings.network_retry || !is_network_error || retry_count >= max_retries {
                        break;
                    }
                }
            }
            Err(e) => {
                if let Err(log_err) = state.add_log(format!(
                    "Error waiting for yt-dlp process for {}: {}. Aborting this URL.",
                    url, e
                )) {
                    eprintln!("Error adding log: {}", log_err);
                }
                break;
            }
        }

        retry_count += 1;
        if state.is_force_quit().unwrap_or(false) {
            if let Err(e) = state.add_log(format!(
                "Force quit: Detected before retry sleep for {}.",
                url
            )) {
                eprintln!("Error adding log: {}", e);
            }
            break;
        }
        if retry_count <= max_retries {
            std::thread::sleep(std::time::Duration::from_secs(retry_delay));
        }
    }

    if let Err(e) = state.send(StateMessage::RemoveActiveDownload(url.clone())) {
        eprintln!("Error removing active download: {}", e);
    }

    if success {
        if let Err(e) = remove_link_from_file(&url) {
            eprintln!("Error removing link from file: {}", e);
        }

        if let Err(e) = state.send(StateMessage::IncrementCompleted) {
            eprintln!("Error incrementing completed: {}", e);
        }

        if let Err(e) = state.add_log(format!("Completed: {}", url)) {
            eprintln!("Error adding log: {}", e);
        }
    } else if state.is_force_quit().unwrap_or(false) {
        if let Err(e) = state.add_log(format!("Download aborted due to force quit: {}", url)) {
            eprintln!("Error adding log: {}", e);
        }
    } else if retry_count > 0 {
        if let Err(e) = state.add_log(format!("Failed after {} retries: {}", retry_count, url)) {
            eprintln!("Error adding log: {}", e);
        }
    } else if let Err(e) = state.add_log(format!("Failed: {}", url)) {
        eprintln!("Error adding log: {}", e);
    }

    if state.get_queue().unwrap_or_default().is_empty()
        && state.get_active_downloads().unwrap_or_default().is_empty()
        && !state.is_force_quit().unwrap_or(false)
    {
        if let Err(e) = state.send(StateMessage::SetCompleted(true)) {
            eprintln!("Error setting completed: {}", e);
        }
    }
}
