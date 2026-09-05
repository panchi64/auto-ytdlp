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
    /// Max concurrent downloads (overrides the saved setting)
    ///
    /// Optional rather than defaulted: a hard default here would overwrite the
    /// saved `concurrent_downloads` on every launch, making the settings row inert.
    #[arg(short, long)]
    pub concurrent: Option<usize>,
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
    /// Worker count to use: the `-c` flag if given, otherwise the saved setting
    pub fn resolve_concurrent(&self, settings: &Settings) -> usize {
        self.concurrent.unwrap_or(settings.concurrent_downloads)
    }

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
mod tests;
