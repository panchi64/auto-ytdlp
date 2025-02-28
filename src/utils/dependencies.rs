use std::process::{Command, Stdio};

/// Verifies that all required external dependencies are installed and accessible.
///
/// Checks for the presence and usability of:
/// - yt-dlp: The main downloader tool
/// - ffmpeg: Required for media processing
///
/// # Returns
///
/// * `Result<(), Vec<String>>` - Ok if all dependencies are available, or
///   Err containing a vector of error messages for missing dependencies
///
/// # Example
///
/// ```
/// match check_dependencies() {
///     Ok(()) => {
///         // Start download process
///     },
///     Err(errors) => {
///         for error in errors {
///             state.add_log(error);
///         }
///     }
/// }
/// ```
///
/// # Notes
///
/// The error messages triggered to show in the TUI include suggestions for where
/// to download the missing dependencies, which can be displayed directly to the
/// user.
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
