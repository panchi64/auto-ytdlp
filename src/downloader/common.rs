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
    // Resolved per download so directory changes made in the settings menu
    // take effect without a restart
    let output_template = args.output_template(settings);

    // Start with the archive file argument
    let mut cmd_args = vec![
        "--download-archive".to_string(),
        args.archive_file.to_string_lossy().to_string(),
    ];

    // Add settings-based arguments
    cmd_args.extend(settings.get_ytdlp_args(&output_template));

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
    use crate::utils::settings::{FormatPreset, OutputFormat, Settings, settings_with_dir};
    use clap::Parser;

    const TEST_URL: &str = "https://example.com/video";

    /// Helper function to create Args for testing
    fn create_test_args(download_dir: &str, archive_file: &str) -> Args {
        Args::parse_from(["test", "-d", download_dir, "-f", archive_file])
    }

    /// Builds the yt-dlp args for `settings`, so each test states only the
    /// setting under test and the flags it expects.
    fn args_for(settings: Settings) -> Vec<String> {
        build_ytdlp_command_args(
            &create_test_args("/downloads", "/archive.txt"),
            &settings,
            TEST_URL,
        )
    }

    /// True when `value` appears among the built args.
    fn has(cmd_args: &[String], value: &str) -> bool {
        cmd_args.iter().any(|arg| arg == value)
    }

    // ==================== Basic Command Building ====================

    #[test]
    fn test_build_ytdlp_command_args_includes_archive_file() {
        let cmd_args = args_for(Settings::default());

        assert!(has(&cmd_args, "--download-archive"));
        assert!(has(&cmd_args, "/archive.txt"));
    }

    #[test]
    fn test_build_ytdlp_command_args_includes_url() {
        let cmd_args = args_for(Settings::default());

        assert_eq!(cmd_args.last(), Some(&TEST_URL.to_string()));
    }

    #[test]
    fn test_build_ytdlp_command_args_includes_progress_template() {
        let cmd_args = args_for(Settings::default());

        assert!(has(&cmd_args, "--progress-template"));

        // Find the progress template value
        let template_idx = cmd_args
            .iter()
            .position(|a| a == "--progress-template")
            .expect("--progress-template flag missing from yt-dlp args");
        let template_value = &cmd_args[template_idx + 1];

        assert!(template_value.starts_with("download:"));
        assert!(template_value.contains(PROGRESS_MARKER_START));
        assert!(template_value.contains(PROGRESS_MARKER_END));
    }

    #[test]
    fn test_build_ytdlp_command_args_includes_output_template() {
        let args = create_test_args("/my/downloads", "/archive.txt");
        let cmd_args = build_ytdlp_command_args(&args, &Settings::default(), TEST_URL);

        assert!(has(&cmd_args, "--output"));

        // Find the output template value
        let output_idx = cmd_args
            .iter()
            .position(|a| a == "--output")
            .expect("--output flag missing from yt-dlp args");
        let output_value = &cmd_args[output_idx + 1];

        // Should contain the download directory
        assert!(output_value.contains("/my/downloads"));
        // Should contain the template pattern
        assert!(output_value.contains("%(title)s"));
        assert!(output_value.contains("%(id)s"));
        assert!(output_value.contains("%(ext)s"));
    }

    #[test]
    fn test_build_ytdlp_command_args_output_uses_download_dir_setting() {
        // No -d flag, so the setting decides where files land
        let args = Args::parse_from(["test", "-f", "/archive.txt"]);
        let settings = settings_with_dir("/configured/downloads");
        let url = "https://example.com/video";

        let cmd_args = build_ytdlp_command_args(&args, &settings, url);

        let output_idx = cmd_args.iter().position(|a| a == "--output").unwrap();
        assert!(cmd_args[output_idx + 1].starts_with("/configured/downloads"));
    }

    #[test]
    fn test_build_ytdlp_command_args_download_dir_flag_wins() {
        let args = create_test_args("/from/flag", "/archive.txt");
        let settings = settings_with_dir("/from/settings");
        let url = "https://example.com/video";

        let cmd_args = build_ytdlp_command_args(&args, &settings, url);

        let output_idx = cmd_args.iter().position(|a| a == "--output").unwrap();
        assert!(cmd_args[output_idx + 1].starts_with("/from/flag"));
    }

    // ==================== Format Preset Testing ====================

    #[test]
    fn test_build_ytdlp_command_args_best_format() {
        let cmd_args = args_for(Settings {
            format_preset: FormatPreset::Best,
            ..Default::default()
        });

        assert!(has(&cmd_args, "--format"));
        assert!(has(&cmd_args, "bestvideo*+bestaudio/best"));
    }

    #[test]
    fn test_build_ytdlp_command_args_audio_only_format() {
        let cmd_args = args_for(Settings {
            format_preset: FormatPreset::AudioOnly,
            ..Default::default()
        });

        assert!(has(&cmd_args, "bestaudio/best"));
    }

    #[test]
    fn test_build_ytdlp_command_args_hd1080p_format() {
        let cmd_args = args_for(Settings {
            format_preset: FormatPreset::HD1080p,
            ..Default::default()
        });

        assert!(has(
            &cmd_args,
            "bestvideo[height<=1080]+bestaudio/best[height<=1080]"
        ));
    }

    // ==================== Output Format Testing ====================

    #[test]
    fn test_build_ytdlp_command_args_mp4_output() {
        let cmd_args = args_for(Settings {
            output_format: OutputFormat::MP4,
            ..Default::default()
        });

        assert!(has(&cmd_args, "--merge-output-format"));
        assert!(has(&cmd_args, "mp4"));
    }

    #[test]
    fn test_build_ytdlp_command_args_mp3_output() {
        let cmd_args = args_for(Settings {
            output_format: OutputFormat::MP3,
            ..Default::default()
        });

        assert!(has(&cmd_args, "--extract-audio"));
        assert!(has(&cmd_args, "--audio-format"));
        assert!(has(&cmd_args, "mp3"));
    }

    #[test]
    fn test_build_ytdlp_command_args_auto_output_no_merge_format() {
        let cmd_args = args_for(Settings {
            output_format: OutputFormat::Auto,
            ..Default::default()
        });

        // Auto should not add any --merge-output-format
        assert!(!has(&cmd_args, "--merge-output-format"));
    }

    // ==================== Optional Flags Testing ====================

    #[test]
    fn test_build_ytdlp_command_args_with_subtitles() {
        let cmd_args = args_for(Settings {
            write_subtitles: true,
            ..Default::default()
        });

        assert!(has(&cmd_args, "--write-auto-subs"));
        assert!(has(&cmd_args, "--sub-langs"));
        assert!(has(&cmd_args, "all"));
    }

    #[test]
    fn test_build_ytdlp_command_args_without_subtitles() {
        let cmd_args = args_for(Settings {
            write_subtitles: false,
            ..Default::default()
        });

        assert!(!has(&cmd_args, "--write-auto-subs"));
    }

    #[test]
    fn test_build_ytdlp_command_args_with_thumbnail() {
        let cmd_args = args_for(Settings {
            write_thumbnail: true,
            ..Default::default()
        });

        assert!(has(&cmd_args, "--write-thumbnail"));
    }

    #[test]
    fn test_build_ytdlp_command_args_without_thumbnail() {
        let cmd_args = args_for(Settings {
            write_thumbnail: false,
            ..Default::default()
        });

        assert!(!has(&cmd_args, "--write-thumbnail"));
    }

    #[test]
    fn test_build_ytdlp_command_args_with_metadata() {
        let cmd_args = args_for(Settings {
            add_metadata: true,
            ..Default::default()
        });

        assert!(has(&cmd_args, "--add-metadata"));
    }

    #[test]
    fn test_build_ytdlp_command_args_without_metadata() {
        let cmd_args = args_for(Settings {
            add_metadata: false,
            ..Default::default()
        });

        assert!(!has(&cmd_args, "--add-metadata"));
    }

    // ==================== Combined Settings Testing ====================

    #[test]
    fn test_build_ytdlp_command_args_full_settings() {
        let args = create_test_args("/downloads", "/archive.txt");
        let settings = Settings {
            format_preset: FormatPreset::HD720p,
            output_format: OutputFormat::Mkv,
            write_subtitles: true,
            write_thumbnail: true,
            add_metadata: true,
            ..Default::default()
        };
        let url = "https://youtube.com/watch?v=abc123";

        let cmd_args = build_ytdlp_command_args(&args, &settings, url);

        // Verify all expected flags are present
        assert!(has(&cmd_args, "--download-archive"));
        assert!(has(&cmd_args, "--format"));
        assert!(has(
            &cmd_args,
            "bestvideo[height<=720]+bestaudio/best[height<=720]"
        ));
        assert!(has(&cmd_args, "--merge-output-format"));
        assert!(has(&cmd_args, "mkv"));
        assert!(has(&cmd_args, "--write-auto-subs"));
        assert!(has(&cmd_args, "--write-thumbnail"));
        assert!(has(&cmd_args, "--add-metadata"));
        assert!(has(&cmd_args, "--newline"));
        assert!(has(&cmd_args, "--progress-template"));
        assert_eq!(cmd_args.last(), Some(&url.to_string()));
    }

    #[test]
    fn test_build_ytdlp_command_args_always_includes_newline() {
        let cmd_args = args_for(Settings::default());

        assert!(has(&cmd_args, "--newline"));
    }

    #[test]
    fn test_build_ytdlp_command_args_with_custom_args() {
        let cmd_args = args_for(Settings {
            custom_ytdlp_args: "--cookies cookies.txt --retries 10".to_string(),
            ..Default::default()
        });

        assert!(has(&cmd_args, "--cookies"));
        assert!(has(&cmd_args, "cookies.txt"));
        assert!(has(&cmd_args, "--retries"));
        assert!(has(&cmd_args, "10"));
    }
}
