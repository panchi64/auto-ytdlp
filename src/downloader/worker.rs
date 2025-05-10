use std::{
    io::{BufRead, BufReader},
    process::{Command, Stdio},
};

use crate::{
    app_state::{AppState, StateMessage},
    args::Args,
    utils::{file::remove_link_from_file, settings::Settings},
};

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

    // Load user settings, fallback to defaults if loading fails
    let settings = Settings::load().unwrap_or_default();

    let output_template = args
        .download_dir
        .join("%(title)s - [%(id)s].%(ext)s")
        .to_str()
        .unwrap()
        .to_string();

    // Build command using settings
    let mut cmd_args = vec![
        "--download-archive".to_string(),
        args.archive_file.to_string_lossy().to_string(),
    ];

    // Add settings-based arguments
    cmd_args.extend(settings.get_ytdlp_args(&output_template));

    // Add the URL to download
    cmd_args.push(url.clone());

    // Start the command
    let mut cmd = Command::new("yt-dlp")
        .args(&cmd_args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start yt-dlp");

    // Set up to read the command output
    let stdout = cmd.stdout.take().unwrap();
    let reader = BufReader::new(stdout);

    // Process the output lines
    for line in reader.lines().map_while(Result::ok) {
        if state.is_force_quit() {
            let _ = cmd.kill();
            break;
        }

        // Filter and log relevant output lines
        let log_line = if line.contains("ERROR") {
            format!("Error: {}", line)
        } else if line.contains("Destination") || line.contains("[download]") {
            line
        } else {
            continue;
        };

        state.add_log(log_line);
    }

    let status = cmd.wait().expect("Failed to wait on yt-dlp");

    state.send(StateMessage::RemoveActiveDownload(url.clone()));

    if status.success() {
        let _ = remove_link_from_file(&url);

        state.send(StateMessage::IncrementCompleted);

        state.add_log(format!("Completed: {}", url));
    } else {
        state.add_log(format!("Failed: {}", url));
    }

    if state.get_queue().is_empty() && state.get_active_downloads().is_empty() {
        state.send(StateMessage::SetCompleted(true));
    }
}
