use clap::Parser;
use std::path::PathBuf;
use std::sync::OnceLock;

#[derive(Parser, Debug, Clone, Default)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Run in automated mode without TUI
    #[arg(short, long)]
    pub auto: bool,
    /// Max concurrent downloads
    #[arg(short, long, default_value_t = 4)]
    pub concurrent: usize,
    /// Download directory
    #[arg(short, long, default_value = "./yt_dlp_downloads")]
    pub download_dir: PathBuf,
    /// Archive file path
    #[arg(short = 'f', long, default_value = "./download_archive.txt")]
    pub archive_file: PathBuf,

    /// Cached output template (computed once, reused for all downloads)
    #[arg(skip)]
    pub(crate) output_template: OnceLock<String>,
}

impl Args {
    /// Get the output template, computing it once and caching for subsequent calls.
    ///
    /// This avoids repeated path allocations for each download.
    pub fn output_template(&self) -> &str {
        self.output_template.get_or_init(|| {
            self.download_dir
                .join("%(title)s - [%(id)s].%(ext)s")
                .to_string_lossy()
                .to_string()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn test_default_values() {
        // Parse with no arguments (just the program name)
        let args = Args::parse_from(["test"]);

        assert!(!args.auto);
        assert_eq!(args.concurrent, 4);
        assert_eq!(args.download_dir, PathBuf::from("./yt_dlp_downloads"));
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
        assert_eq!(args.download_dir, PathBuf::from("/tmp/downloads"));
    }

    #[test]
    fn test_download_dir_flag_long() {
        let args = Args::parse_from(["test", "--download-dir", "/home/user/videos"]);
        assert_eq!(args.download_dir, PathBuf::from("/home/user/videos"));
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
    fn test_output_template_caching() {
        let args = Args::parse_from(["test", "-d", "/my/downloads"]);

        // First call computes the template
        let template1 = args.output_template();
        assert!(template1.contains("/my/downloads"));
        assert!(template1.contains("%(title)s"));
        assert!(template1.contains("%(id)s"));
        assert!(template1.contains("%(ext)s"));

        // Second call should return the same cached value
        let template2 = args.output_template();
        assert_eq!(template1, template2);

        // Verify they are the same memory address (cached)
        assert!(std::ptr::eq(template1, template2));
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
        assert_eq!(args.download_dir, PathBuf::from("/downloads"));
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
