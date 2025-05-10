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
    if state.is_force_quit() {
        return;
    }

    state.send(StateMessage::AddActiveDownload(url.clone()));
    state.add_log(format!("Starting download: {}", url));

    let settings = state.get_settings();
    let max_retries = if settings.network_retry { 3 } else { 0 };
    let retry_delay = settings.retry_delay;
    let mut retry_count = 0;
    let mut success = false;

    while retry_count <= max_retries {
        if state.is_force_quit() {
            state.add_log(format!("Force quit: Aborting download task for {}.", url));
            break;
        }

        if retry_count > 0 {
            state.add_log(format!("Retry attempt {} for: {}", retry_count, url));
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
                state.add_log(format!(
                    "Error spawning yt-dlp for {}: {}. Aborting this URL.",
                    url, e
                ));
                break;
            }
        };

        if state.is_force_quit() {
            state.add_log(format!(
                "Force quit: Killing spawned process for {} and aborting.",
                url
            ));
            let _ = cmd.kill();
            break;
        }

        let stdout = match cmd.stdout.take() {
            Some(stdout) => stdout,
            None => {
                state.add_log(format!(
                    "Error: Could not take stdout for {}. Aborting this attempt.",
                    url
                ));
                if !state.is_force_quit() {
                    let _ = cmd.kill();
                    let _ = cmd.wait();
                }
                break;
            }
        };
        let reader = BufReader::new(stdout);
        let mut is_network_error = false;

        for line in reader.lines().map_while(Result::ok) {
            if state.is_force_quit() {
                state.add_log(format!(
                    "Force quit: Killing process during output reading for {}.",
                    url
                ));
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
            state.add_log(log_line);
        }

        if state.is_force_quit() {
            state.add_log(format!(
                "Force quit: Detected after output processing for {}. Ensuring kill.",
                url
            ));
            let _ = cmd.kill();
            break;
        }

        match cmd.wait() {
            Ok(status) => {
                if status.success() {
                    success = true;
                    break;
                } else {
                    state.add_log(format!("yt-dlp exited with error for {}: {}", url, status));
                    if !settings.network_retry || !is_network_error || retry_count >= max_retries {
                        break;
                    }
                }
            }
            Err(e) => {
                state.add_log(format!(
                    "Error waiting for yt-dlp process for {}: {}. Aborting this URL.",
                    url, e
                ));
                break;
            }
        }

        retry_count += 1;
        if state.is_force_quit() {
            state.add_log(format!(
                "Force quit: Detected before retry sleep for {}.",
                url
            ));
            break;
        }
        if retry_count <= max_retries {
            std::thread::sleep(std::time::Duration::from_secs(retry_delay));
        }
    }

    state.send(StateMessage::RemoveActiveDownload(url.clone()));

    if success {
        let _ = remove_link_from_file(&url);

        state.send(StateMessage::IncrementCompleted);

        state.add_log(format!("Completed: {}", url));
    } else if state.is_force_quit() {
        state.add_log(format!("Download aborted due to force quit: {}", url));
    } else if retry_count > 0 {
        state.add_log(format!("Failed after {} retries: {}", retry_count, url));
    } else {
        state.add_log(format!("Failed: {}", url));
    }

    if state.get_queue().is_empty() && state.get_active_downloads().is_empty() {
        if !state.is_force_quit() {
            state.send(StateMessage::SetCompleted(true));
        }
    }
}
