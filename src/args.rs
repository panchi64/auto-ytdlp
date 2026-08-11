use crate::utils::settings::{Settings, expand_tilde};
use clap::Parser;
use std::path::PathBuf;

/// Directory used when neither `--download-dir` nor the setting is set
pub const DEFAULT_DOWNLOAD_DIR: &str = "./yt_dlp_downloads";

/// Filename template appended to the download directory
const FILENAME_TEMPLATE: &str = "%(title)s - [%(id)s].%(ext)s";

#[derive(Parser, Debug, Clone, Default)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Run in automated mode without TUI
    #[arg(short, long)]
    pub auto: bool,
    /// Max concurrent downloads
    #[arg(short, long, default_value_t = 4)]
    pub concurrent: usize,
    /// Download directory (overrides the saved setting)
    #[arg(short, long)]
    pub download_dir: Option<PathBuf>,
    /// Archive file path
    #[arg(short = 'f', long, default_value = "./download_archive.txt")]
    pub archive_file: PathBuf,
}

impl Args {
    /// Resolve the directory downloads are written to.
    ///
    /// Precedence: `--download-dir` flag, then the saved setting, then the
    /// built-in default. Both sources expand a leading `~`, since a shell that
    /// does not expand it (quoted arguments, cron, systemd) would otherwise
    /// create a directory literally named `~`.
    pub fn resolve_download_dir(&self, settings: &Settings) -> PathBuf {
        self.flag_download_dir()
            .or_else(|| settings.resolved_download_dir())
            .unwrap_or_else(|| PathBuf::from(DEFAULT_DOWNLOAD_DIR))
    }

    /// The `--download-dir` value with `~` expanded, if the flag was passed
    pub fn flag_download_dir(&self) -> Option<PathBuf> {
        let flag = self.download_dir.as_ref()?;
        match expand_tilde(&flag.to_string_lossy()) {
            Some(expanded) => Some(expanded),
            None => {
                eprintln!(
                    "Warning: cannot expand '{}' (no home directory); using it as given",
                    flag.display()
                );
                Some(flag.clone())
            }
        }
    }

    /// Build the yt-dlp output template for the resolved download directory.
    ///
    /// Computed per download because the directory setting can change while
    /// the application is running.
    pub fn output_template(&self, settings: &Settings) -> String {
        self.resolve_download_dir(settings)
            .join(FILENAME_TEMPLATE)
            .to_string_lossy()
            .into_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::settings::settings_with_dir;
    use clap::Parser;

    #[test]
    fn test_default_values() {
        // Parse with no arguments (just the program name)
        let args = Args::parse_from(["test"]);

        assert!(!args.auto);
        assert_eq!(args.concurrent, 4);
        assert_eq!(args.download_dir, None);
        assert_eq!(args.archive_file, PathBuf::from("./download_archive.txt"));
    }

    #[test]
    fn test_auto_flag_short() {
        let args = Args::parse_from(["test", "-a"]);
        assert!(args.auto);
    }

    #[test]
    fn test_auto_flag_long() {
        let args = Args::parse_from(["test", "--auto"]);
        assert!(args.auto);
    }

    #[test]
    fn test_concurrent_flag_short() {
        let args = Args::parse_from(["test", "-c", "8"]);
        assert_eq!(args.concurrent, 8);
    }

    #[test]
    fn test_concurrent_flag_long() {
        let args = Args::parse_from(["test", "--concurrent", "16"]);
        assert_eq!(args.concurrent, 16);
    }

    #[test]
    fn test_download_dir_flag_short() {
        let args = Args::parse_from(["test", "-d", "/tmp/downloads"]);
        assert_eq!(args.download_dir, Some(PathBuf::from("/tmp/downloads")));
    }

    #[test]
    fn test_download_dir_flag_long() {
        let args = Args::parse_from(["test", "--download-dir", "/home/user/videos"]);
        assert_eq!(args.download_dir, Some(PathBuf::from("/home/user/videos")));
    }

    #[test]
    fn test_resolve_download_dir_falls_back_to_default() {
        let args = Args::parse_from(["test"]);
        let settings = Settings::default();

        assert_eq!(
            args.resolve_download_dir(&settings),
            PathBuf::from(DEFAULT_DOWNLOAD_DIR)
        );
    }

    #[test]
    fn test_resolve_download_dir_uses_setting() {
        let args = Args::parse_from(["test"]);

        assert_eq!(
            args.resolve_download_dir(&settings_with_dir("/media/videos")),
            PathBuf::from("/media/videos")
        );
    }

    #[test]
    fn test_resolve_download_dir_flag_overrides_setting() {
        let args = Args::parse_from(["test", "-d", "/tmp/downloads"]);

        assert_eq!(
            args.resolve_download_dir(&settings_with_dir("/media/videos")),
            PathBuf::from("/tmp/downloads")
        );
    }

    #[test]
    fn test_resolve_download_dir_expands_tilde_from_setting() {
        let args = Args::parse_from(["test"]);

        let resolved = args.resolve_download_dir(&settings_with_dir("~/videos"));

        assert!(!resolved.to_string_lossy().starts_with('~'));
        assert!(resolved.ends_with("videos"));
    }

    #[test]
    fn test_output_template_uses_setting_directory() {
        let args = Args::parse_from(["test"]);

        let template = args.output_template(&settings_with_dir("/media/videos"));

        assert!(template.starts_with("/media/videos"));
        assert!(template.contains("%(title)s"));
        assert!(template.contains("%(id)s"));
        assert!(template.contains("%(ext)s"));
    }

    #[test]
    fn test_archive_file_flag_short() {
        let args = Args::parse_from(["test", "-f", "/tmp/archive.txt"]);
        assert_eq!(args.archive_file, PathBuf::from("/tmp/archive.txt"));
    }

    #[test]
    fn test_archive_file_flag_long() {
        let args = Args::parse_from(["test", "--archive-file", "/home/user/archive.txt"]);
        assert_eq!(args.archive_file, PathBuf::from("/home/user/archive.txt"));
    }

    #[test]
    fn test_output_template_uses_flag_directory() {
        let args = Args::parse_from(["test", "-d", "/my/downloads"]);
        let settings = Settings::default();

        let template = args.output_template(&settings);

        assert!(template.starts_with("/my/downloads"));
        assert!(template.contains("%(title)s"));
        assert!(template.contains("%(id)s"));
        assert!(template.contains("%(ext)s"));
    }

    #[test]
    fn test_combined_flags() {
        let args = Args::parse_from([
            "test",
            "--auto",
            "-c",
            "12",
            "-d",
            "/downloads",
            "-f",
            "/archive.txt",
        ]);

        assert!(args.auto);
        assert_eq!(args.concurrent, 12);
        assert_eq!(args.download_dir, Some(PathBuf::from("/downloads")));
        assert_eq!(args.archive_file, PathBuf::from("/archive.txt"));
    }

    #[test]
    fn test_args_clone() {
        let args = Args::parse_from(["test", "--auto", "-c", "6"]);
        let cloned = args.clone();

        assert_eq!(cloned.auto, args.auto);
        assert_eq!(cloned.concurrent, args.concurrent);
        assert_eq!(cloned.download_dir, args.download_dir);
        assert_eq!(cloned.archive_file, args.archive_file);
    }
}
