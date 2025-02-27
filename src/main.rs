mod app_state;
mod args;
mod downloader;
mod ui;
mod utils;

use anyhow::Result;
use app_state::{AppState, StateMessage};
use args::Args;
use clap::Parser;
use downloader::queue::process_queue;
use std::{
    fs::{self, File},
    path::Path,
};
use ui::tui::run_tui;
use utils::{dependencies::check_dependencies, file::load_links};

fn main() -> Result<()> {
    let args = Args::parse();
    let state = AppState::new();

    state.set_concurrent(args.concurrent);

    fs::create_dir_all(&args.download_dir)?;

    if !Path::new("links.txt").exists() {
        File::create("links.txt")?;
    }

    let links = fs::read_to_string("links.txt")
        .unwrap_or_default()
        .lines()
        .map(String::from)
        .collect::<Vec<_>>();
    state.send(StateMessage::LoadLinks(links));

    if args.auto {
        // Check dependencies before processing in auto mode
        match check_dependencies() {
            Ok(()) => process_queue(state.clone(), args.clone()),
            Err(errors) => {
                for error in errors {
                    eprintln!("Error: {}", error);
                    if error.contains("yt-dlp") {
                        eprintln!("Please download the latest version of yt-dlp from: https://github.com/yt-dlp/yt-dlp/releases");
                    }
                    if error.contains("ffmpeg") {
                        eprintln!(
                            "Please download ffmpeg from: https://www.ffmpeg.org/download.html"
                        );
                    }
                }
                std::process::exit(1);
            }
        }
    } else {
        run_tui(state.clone(), args.clone())?;
    }

    Ok(())
}
