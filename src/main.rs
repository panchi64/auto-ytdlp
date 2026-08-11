mod app_state;
mod args;
mod downloader;
mod errors;
mod ui;
mod utils;

use app_state::{AppState, StateMessage};
use args::Args;
use clap::Parser;
use downloader::{common::validate_dependencies, queue::process_queue};
use errors::Result;
use std::{
    fs::{self, File},
    path::Path,
};
use ui::tui::run_tui;
use utils::file::{LINKS_FILE, get_links_from_file};

fn main() -> Result<()> {
    let args = Args::parse();
    let state = AppState::new();

    state.set_concurrent(args.concurrent)?;

    let settings = state.get_settings().unwrap_or_default();
    let download_dir = args.resolve_download_dir(&settings);
    if let Err(error) = fs::create_dir_all(&download_dir) {
        // In TUI mode this must not be fatal: the directory can come from the
        // saved setting, and aborting here would leave no way to correct it
        // from inside the app.
        if args.auto {
            return Err(error.into());
        }
        state.log_error(
            &format!("Download directory {:?}", download_dir),
            format!("{} - change it with F2", error),
        )?;
    }

    if !Path::new(LINKS_FILE).exists() {
        File::create(LINKS_FILE)?;
    }

    let links = get_links_from_file()?;
    state.send(StateMessage::LoadLinks(links))?;

    if args.auto {
        // Check dependencies before processing in auto mode
        match validate_dependencies() {
            Ok(()) => process_queue(state.clone(), args.clone()),
            Err(error) => {
                eprintln!("Error: {}", error);
                if error.to_string().contains("yt-dlp") {
                    eprintln!(
                        "Please download the latest version of yt-dlp from: https://github.com/yt-dlp/yt-dlp/releases"
                    );
                }
                if error.to_string().contains("ffmpeg") {
                    eprintln!("Please download ffmpeg from: https://www.ffmpeg.org/download.html");
                }
                std::process::exit(1);
            }
        }
    } else {
        run_tui(state.clone(), args.clone())?;
    }

    Ok(())
}
