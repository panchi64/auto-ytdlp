use std::{
    io::{BufRead, BufReader},
    process::{Command, Stdio},
};

use crate::{
    app_state::{update_progress, AppState},
    args::Args,
    utils::file::remove_link_from_file,
};

pub fn download_worker(url: String, state: AppState, args: Args) {
    if *state.force_quit.lock().unwrap() {
        return;
    }

    // Add to active downloads
    {
        let mut active = state.active_downloads.lock().unwrap();
        active.insert(url.clone());
    }

    // Add log entry when download starts
    {
        let mut logs = state.logs.lock().unwrap();
        logs.push(format!("Starting download: {}", url));
    }

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

    let stdout = cmd.stdout.take().unwrap();
    let reader = BufReader::new(stdout);

    // Use map_while to handle potential read errors properly
    for line in reader.lines().map_while(Result::ok) {
        if *state.force_quit.lock().unwrap() {
            cmd.kill().ok();
            break;
        }

        let log_line = if line.contains("ERROR") {
            format!("Error: {}", line)
        } else if line.contains("Destination") || line.contains("[download]") {
            line
        } else {
            continue;
        };

        state.logs.lock().unwrap().push(log_line);
    }

    let status = cmd.wait().expect("Failed to wait on yt-dlp");

    // Remove from active downloads
    state.active_downloads.lock().unwrap().remove(&url);

    // Update logs and remove from links.txt
    let result_msg = if status.success() {
        remove_link_from_file(&url).unwrap();
        *state.completed_tasks.lock().unwrap() += 1;
        format!("Completed: {}", url)
    } else {
        format!("Failed: {}", url)
    };

    state.logs.lock().unwrap().push(result_msg);
    update_progress(&state);
}
