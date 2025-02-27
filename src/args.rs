use clap::Parser;
use std::path::PathBuf;

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
}
