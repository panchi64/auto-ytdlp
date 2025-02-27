use std::process::{Command, Stdio};

pub fn check_dependencies() -> Result<(), Vec<String>> {
    let mut missing = Vec::new();

    // Check yt-dlp
    let yt_dlp_status = Command::new("yt-dlp")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    if yt_dlp_status.map(|s| !s.success()).unwrap_or(true) {
        missing.push("yt-dlp is not installed or not accessible.".to_string());
    }

    // Check ffmpeg
    let ffmpeg_status = Command::new("ffmpeg")
        .arg("-version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    if ffmpeg_status.map(|s| !s.success()).unwrap_or(true) {
        missing.push("ffmpeg is not installed or not accessible.".to_string());
    }

    if missing.is_empty() {
        Ok(())
    } else {
        Err(missing)
    }
}
