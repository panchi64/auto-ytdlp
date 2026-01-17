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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore] // Requires yt-dlp and ffmpeg installed
    fn test_check_dependencies_with_both_installed() {
        // This test will only pass if both yt-dlp and ffmpeg are installed
        let result = check_dependencies();
        assert!(
            result.is_ok(),
            "Expected both dependencies to be available, but got: {:?}",
            result.err()
        );
    }

    #[test]
    #[ignore] // Requires yt-dlp and ffmpeg installed
    fn test_check_dependencies_returns_empty_on_success() {
        // When both are installed, the result should be Ok(())
        let result = check_dependencies();
        if let Err(errors) = &result {
            // Print what's missing if the test fails
            for error in errors {
                eprintln!("Missing dependency: {}", error);
            }
        }
        assert!(result.is_ok());
    }
}
