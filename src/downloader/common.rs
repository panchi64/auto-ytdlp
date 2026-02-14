use crate::{
    args::Args,
    utils::{dependencies::check_dependencies, settings::Settings},
};
use anyhow::{Error, Result};

use super::progress_parser::{PROGRESS_MARKER_END, PROGRESS_MARKER_START};

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

    // Add custom progress template for structured progress parsing
    // Format: |PROGRESS|status|percent|speed|eta|downloaded|total|frag_idx|frag_count|PROGRESS_END|
    cmd_args.push("--progress-template".to_string());
    cmd_args.push(format!(
        "download:{}%(progress.status)s|%(progress._percent_str)s|%(progress._speed_str)s|%(progress._eta_str)s|%(progress.downloaded_bytes)s|%(progress.total_bytes)s|%(progress.fragment_index)s|%(progress.fragment_count)s{}",
        PROGRESS_MARKER_START,
        PROGRESS_MARKER_END
    ));

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::settings::{FormatPreset, OutputFormat, Settings};
    use clap::Parser;

    /// Helper function to create Args for testing
    fn create_test_args(download_dir: &str, archive_file: &str) -> Args {
        Args::parse_from(["test", "-d", download_dir, "-f", archive_file])
    }

    // ==================== Basic Command Building ====================

    #[test]
    fn test_build_ytdlp_command_args_includes_archive_file() {
        let args = create_test_args("/downloads", "/archive.txt");
        let settings = Settings::default();
        let url = "https://example.com/video";

        let cmd_args = build_ytdlp_command_args(&args, &settings, url);

        assert!(cmd_args.contains(&"--download-archive".to_string()));
        assert!(cmd_args.contains(&"/archive.txt".to_string()));
    }

    #[test]
    fn test_build_ytdlp_command_args_includes_url() {
        let args = create_test_args("/downloads", "/archive.txt");
        let settings = Settings::default();
        let url = "https://example.com/video";

        let cmd_args = build_ytdlp_command_args(&args, &settings, url);

        assert_eq!(cmd_args.last(), Some(&url.to_string()));
    }

    #[test]
    fn test_build_ytdlp_command_args_includes_progress_template() {
        let args = create_test_args("/downloads", "/archive.txt");
        let settings = Settings::default();
        let url = "https://example.com/video";

        let cmd_args = build_ytdlp_command_args(&args, &settings, url);

        assert!(cmd_args.contains(&"--progress-template".to_string()));

        // Find the progress template value
        let template_idx = cmd_args
            .iter()
            .position(|a| a == "--progress-template")
            .unwrap();
        let template_value = &cmd_args[template_idx + 1];

        assert!(template_value.starts_with("download:"));
        assert!(template_value.contains(PROGRESS_MARKER_START));
        assert!(template_value.contains(PROGRESS_MARKER_END));
    }

    #[test]
    fn test_build_ytdlp_command_args_includes_output_template() {
        let args = create_test_args("/my/downloads", "/archive.txt");
        let settings = Settings::default();
        let url = "https://example.com/video";

        let cmd_args = build_ytdlp_command_args(&args, &settings, url);

        assert!(cmd_args.contains(&"--output".to_string()));

        // Find the output template value
        let output_idx = cmd_args.iter().position(|a| a == "--output").unwrap();
        let output_value = &cmd_args[output_idx + 1];

        // Should contain the download directory
        assert!(output_value.contains("/my/downloads"));
        // Should contain the template pattern
        assert!(output_value.contains("%(title)s"));
        assert!(output_value.contains("%(id)s"));
        assert!(output_value.contains("%(ext)s"));
    }

    // ==================== Format Preset Testing ====================

    #[test]
    fn test_build_ytdlp_command_args_best_format() {
        let args = create_test_args("/downloads", "/archive.txt");
        let mut settings = Settings::default();
        settings.format_preset = FormatPreset::Best;
        let url = "https://example.com/video";

        let cmd_args = build_ytdlp_command_args(&args, &settings, url);

        assert!(cmd_args.contains(&"--format".to_string()));
        assert!(cmd_args.contains(&"bestvideo*+bestaudio/best".to_string()));
    }

    #[test]
    fn test_build_ytdlp_command_args_audio_only_format() {
        let args = create_test_args("/downloads", "/archive.txt");
        let mut settings = Settings::default();
        settings.format_preset = FormatPreset::AudioOnly;
        let url = "https://example.com/video";

        let cmd_args = build_ytdlp_command_args(&args, &settings, url);

        assert!(cmd_args.contains(&"bestaudio/best".to_string()));
    }

    #[test]
    fn test_build_ytdlp_command_args_hd1080p_format() {
        let args = create_test_args("/downloads", "/archive.txt");
        let mut settings = Settings::default();
        settings.format_preset = FormatPreset::HD1080p;
        let url = "https://example.com/video";

        let cmd_args = build_ytdlp_command_args(&args, &settings, url);

        assert!(
            cmd_args.contains(&"bestvideo[height<=1080]+bestaudio/best[height<=1080]".to_string())
        );
    }

    // ==================== Output Format Testing ====================

    #[test]
    fn test_build_ytdlp_command_args_mp4_output() {
        let args = create_test_args("/downloads", "/archive.txt");
        let mut settings = Settings::default();
        settings.output_format = OutputFormat::MP4;
        let url = "https://example.com/video";

        let cmd_args = build_ytdlp_command_args(&args, &settings, url);

        assert!(cmd_args.contains(&"--merge-output-format".to_string()));
        assert!(cmd_args.contains(&"mp4".to_string()));
    }

    #[test]
    fn test_build_ytdlp_command_args_mp3_output() {
        let args = create_test_args("/downloads", "/archive.txt");
        let mut settings = Settings::default();
        settings.output_format = OutputFormat::MP3;
        let url = "https://example.com/video";

        let cmd_args = build_ytdlp_command_args(&args, &settings, url);

        assert!(cmd_args.contains(&"--extract-audio".to_string()));
        assert!(cmd_args.contains(&"--audio-format".to_string()));
        assert!(cmd_args.contains(&"mp3".to_string()));
    }

    #[test]
    fn test_build_ytdlp_command_args_auto_output_no_merge_format() {
        let args = create_test_args("/downloads", "/archive.txt");
        let mut settings = Settings::default();
        settings.output_format = OutputFormat::Auto;
        let url = "https://example.com/video";

        let cmd_args = build_ytdlp_command_args(&args, &settings, url);

        // Auto should not add any --merge-output-format
        assert!(!cmd_args.contains(&"--merge-output-format".to_string()));
    }

    // ==================== Optional Flags Testing ====================

    #[test]
    fn test_build_ytdlp_command_args_with_subtitles() {
        let args = create_test_args("/downloads", "/archive.txt");
        let mut settings = Settings::default();
        settings.write_subtitles = true;
        let url = "https://example.com/video";

        let cmd_args = build_ytdlp_command_args(&args, &settings, url);

        assert!(cmd_args.contains(&"--write-auto-subs".to_string()));
        assert!(cmd_args.contains(&"--sub-langs".to_string()));
        assert!(cmd_args.contains(&"all".to_string()));
    }

    #[test]
    fn test_build_ytdlp_command_args_without_subtitles() {
        let args = create_test_args("/downloads", "/archive.txt");
        let mut settings = Settings::default();
        settings.write_subtitles = false;
        let url = "https://example.com/video";

        let cmd_args = build_ytdlp_command_args(&args, &settings, url);

        assert!(!cmd_args.contains(&"--write-auto-subs".to_string()));
    }

    #[test]
    fn test_build_ytdlp_command_args_with_thumbnail() {
        let args = create_test_args("/downloads", "/archive.txt");
        let mut settings = Settings::default();
        settings.write_thumbnail = true;
        let url = "https://example.com/video";

        let cmd_args = build_ytdlp_command_args(&args, &settings, url);

        assert!(cmd_args.contains(&"--write-thumbnail".to_string()));
    }

    #[test]
    fn test_build_ytdlp_command_args_without_thumbnail() {
        let args = create_test_args("/downloads", "/archive.txt");
        let mut settings = Settings::default();
        settings.write_thumbnail = false;
        let url = "https://example.com/video";

        let cmd_args = build_ytdlp_command_args(&args, &settings, url);

        assert!(!cmd_args.contains(&"--write-thumbnail".to_string()));
    }

    #[test]
    fn test_build_ytdlp_command_args_with_metadata() {
        let args = create_test_args("/downloads", "/archive.txt");
        let mut settings = Settings::default();
        settings.add_metadata = true;
        let url = "https://example.com/video";

        let cmd_args = build_ytdlp_command_args(&args, &settings, url);

        assert!(cmd_args.contains(&"--add-metadata".to_string()));
    }

    #[test]
    fn test_build_ytdlp_command_args_without_metadata() {
        let args = create_test_args("/downloads", "/archive.txt");
        let mut settings = Settings::default();
        settings.add_metadata = false;
        let url = "https://example.com/video";

        let cmd_args = build_ytdlp_command_args(&args, &settings, url);

        assert!(!cmd_args.contains(&"--add-metadata".to_string()));
    }

    // ==================== Combined Settings Testing ====================

    #[test]
    fn test_build_ytdlp_command_args_full_settings() {
        let args = create_test_args("/downloads", "/archive.txt");
        let mut settings = Settings::default();
        settings.format_preset = FormatPreset::HD720p;
        settings.output_format = OutputFormat::Mkv;
        settings.write_subtitles = true;
        settings.write_thumbnail = true;
        settings.add_metadata = true;
        let url = "https://youtube.com/watch?v=abc123";

        let cmd_args = build_ytdlp_command_args(&args, &settings, url);

        // Verify all expected flags are present
        assert!(cmd_args.contains(&"--download-archive".to_string()));
        assert!(cmd_args.contains(&"--format".to_string()));
        assert!(
            cmd_args.contains(&"bestvideo[height<=720]+bestaudio/best[height<=720]".to_string())
        );
        assert!(cmd_args.contains(&"--merge-output-format".to_string()));
        assert!(cmd_args.contains(&"mkv".to_string()));
        assert!(cmd_args.contains(&"--write-auto-subs".to_string()));
        assert!(cmd_args.contains(&"--write-thumbnail".to_string()));
        assert!(cmd_args.contains(&"--add-metadata".to_string()));
        assert!(cmd_args.contains(&"--newline".to_string()));
        assert!(cmd_args.contains(&"--progress-template".to_string()));
        assert_eq!(cmd_args.last(), Some(&url.to_string()));
    }

    #[test]
    fn test_build_ytdlp_command_args_always_includes_newline() {
        let args = create_test_args("/downloads", "/archive.txt");
        let settings = Settings::default();
        let url = "https://example.com/video";

        let cmd_args = build_ytdlp_command_args(&args, &settings, url);

        assert!(cmd_args.contains(&"--newline".to_string()));
    }

    #[test]
    fn test_build_ytdlp_command_args_with_custom_args() {
        let args = create_test_args("/downloads", "/archive.txt");
        let mut settings = Settings::default();
        settings.custom_ytdlp_args = "--cookies cookies.txt --retries 10".to_string();
        let url = "https://example.com/video";

        let cmd_args = build_ytdlp_command_args(&args, &settings, url);

        assert!(cmd_args.contains(&"--cookies".to_string()));
        assert!(cmd_args.contains(&"cookies.txt".to_string()));
        assert!(cmd_args.contains(&"--retries".to_string()));
        assert!(cmd_args.contains(&"10".to_string()));
    }
}
