use std::{
    io::{BufRead, BufReader},
    process::{Command, Stdio},
};

use crate::{
    app_state::{AppState, StateMessage},
    args::Args,
    utils::file::remove_link_from_file,
};

pub fn download_worker(url: String, state: AppState, args: Args) {
    if state.is_force_quit() {
        return;
    }

    state.send(StateMessage::AddActiveDownload(url.clone()));

    state.add_log(format!("Starting download: {}", url));

    let output_template = args
        .download_dir
        .join("%(title)s - [%(id)s].%(ext)s")
        .to_str()
        .unwrap()
        .to_string();

    let mut cmd = Command::new("yt-dlp")
        .arg("--format")
        .arg("bestvideo*+bestaudio/best")
        .arg("--download-archive")
        .arg(&args.archive_file)
        .arg("--output")
        .arg(output_template)
        .arg("--newline")
        .arg(&url)
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
