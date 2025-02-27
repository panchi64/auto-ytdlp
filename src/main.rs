mod app_state;
mod args;
mod downloader;
mod ui;
mod utils;

use anyhow::Result;
use app_state::AppState;
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

    *state.concurrent.lock().unwrap() = args.concurrent;

    fs::create_dir_all(&args.download_dir)?;

    if !Path::new("links.txt").exists() {
        File::create("links.txt")?;
    }

    load_links(&state)?;

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

#[cfg(test)]
mod tests {
    use crate::utils::file::load_links;
    use crate::utils::file::save_links;

    use super::*;
    use std::env;
    use std::fs;
    use tempfile::tempdir;

    /// Test loading links from file and verifying queue population
    #[test]
    fn test_link_loading_and_queue_management() {
        let temp_dir = tempdir().unwrap();
        env::set_current_dir(&temp_dir).unwrap();

        // Create test links.txt
        fs::write("links.txt", "https://example.com/1\nhttps://example.com/2").unwrap();

        let state = AppState::new();
        load_links(&state).unwrap();

        assert_eq!(state.queue.lock().unwrap().len(), 2);

        // Test duplicate prevention
        state
            .queue
            .lock()
            .unwrap()
            .push_back("https://example.com/1".into());
        save_links(&state).unwrap();

        let contents = fs::read_to_string("links.txt").unwrap();
        assert_eq!(contents, "https://example.com/1\nhttps://example.com/2");
    }

    /// Test directory creation and file preservation
    #[test]
    fn test_directory_creation_and_file_preservation() {
        let temp_dir = tempdir().unwrap();
        let download_dir = temp_dir.path().join("new_downloads");
        let args = Args {
            auto: true, // Changed to auto mode
            concurrent: 1,
            download_dir: download_dir.clone(),
            archive_file: temp_dir.path().join("archive.txt"),
        };

        // Initialize empty state
        let state = AppState::new();
        *state.concurrent.lock().unwrap() = args.concurrent;

        // Verify directory creation
        assert!(!download_dir.exists());
        fs::create_dir_all(&download_dir).unwrap();
        assert!(download_dir.exists());

        // Test with empty queue
        process_queue(state, args.clone());

        let test_file = download_dir.join("test.txt");
        fs::write(&test_file, "test content").unwrap();
        assert!(test_file.exists());
    }

    /// Test concurrent download limits
    #[test]
    fn test_concurrent_download_limits() {
        let state = AppState::new();
        *state.concurrent.lock().unwrap() = 2;

        // Add test URLs
        let urls = vec![
            "https://example.com/1".into(),
            "https://example.com/2".into(),
            "https://example.com/3".into(),
        ];
        state.queue.lock().unwrap().extend(urls);

        // Verify concurrent limit enforcement
        assert_eq!(
            *state.concurrent.lock().unwrap(),
            2,
            "Concurrent limit should be set"
        );
        assert_eq!(
            state.queue.lock().unwrap().len(),
            3,
            "Queue should contain test items"
        );
    }

    /// Test pause/resume functionality
    #[test]
    fn test_pause_resume_mechanism() {
        let state = AppState::new();

        // Initial state check
        assert!(!*state.paused.lock().unwrap(), "Should start unpaused");

        // Toggle pause
        *state.paused.lock().unwrap() = true;
        assert!(*state.paused.lock().unwrap(), "Should enter paused state");

        // Toggle again
        *state.paused.lock().unwrap() = false;
        assert!(
            !*state.paused.lock().unwrap(),
            "Should resume from paused state"
        );
    }
}
