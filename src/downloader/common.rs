use crate::{
    args::Args,
    utils::{dependencies::check_dependencies, settings::Settings},
};
use anyhow::{Error, Result};

/// Builds the command arguments for yt-dlp based on provided settings and args
///
/// This centralizes the command construction logic to avoid duplication between
/// different parts of the application that need to invoke yt-dlp.
///
/// # Parameters
///
/// * `args` - The command-line arguments containing paths
/// * `settings` - The settings to use (passed in to avoid disk I/O per download)
/// * `url` - The URL to download
///
/// # Returns
///
/// A vector of strings containing all command arguments for yt-dlp
pub fn build_ytdlp_command_args(args: &Args, settings: &Settings, url: &str) -> Vec<String> {
    // Use cached output template (computed once, reused for all downloads)
    let output_template = args.output_template();

    // Start with the archive file argument
    let mut cmd_args = vec![
        "--download-archive".to_string(),
        args.archive_file.to_string_lossy().to_string(),
    ];

    // Add settings-based arguments
    cmd_args.extend(settings.get_ytdlp_args(output_template));

    // Add the URL to download
    cmd_args.push(url.to_string());

    cmd_args
}

/// Validates dependencies and handles messaging for errors
///
/// This centralizes the dependency checking and error handling logic
/// used in multiple places in the application.
///
/// # Returns
///
/// Ok(()) if all dependencies are available, or Err with the error messages
pub fn validate_dependencies() -> Result<()> {
    check_dependencies().map_err(|errors| Error::msg(errors.join("\n")))
}
