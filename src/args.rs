use clap::Parser;
use std::path::PathBuf;
use std::sync::OnceLock;

#[derive(Parser, Debug, Clone)]
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
    output_template: OnceLock<String>,
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
