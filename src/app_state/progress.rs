use std::time::Instant;

use crate::utils::display::truncate_url_for_display;

/// Progress information for a single download.
///
/// Tracks percentage, speed, ETA, and other metadata for displaying
/// per-download progress bars in the TUI.
#[derive(Clone, Debug)]
pub struct DownloadProgress {
    /// Truncated URL or video title for display
    pub display_name: String,
    /// Current phase: "downloading", "processing", "merging", "finished", "error"
    pub phase: String,
    /// Download percentage (0.0 - 100.0)
    pub percent: f64,
    /// Download speed (e.g., "1.5MiB/s")
    pub speed: Option<String>,
    /// Estimated time remaining (e.g., "00:05:23")
    pub eta: Option<String>,
    /// Bytes downloaded so far
    pub downloaded_bytes: Option<u64>,
    /// Total file size in bytes
    pub total_bytes: Option<u64>,
    /// Current fragment index (for HLS/DASH streams)
    pub fragment_index: Option<u32>,
    /// Total fragment count (for HLS/DASH streams)
    pub fragment_count: Option<u32>,
    /// Timestamp of last progress update (for staleness detection)
    pub last_update: Instant,
}

impl Default for DownloadProgress {
    fn default() -> Self {
        Self {
            display_name: String::new(),
            phase: "downloading".to_string(),
            percent: 0.0,
            speed: None,
            eta: None,
            downloaded_bytes: None,
            total_bytes: None,
            fragment_index: None,
            fragment_count: None,
            last_update: Instant::now(),
        }
    }
}

impl DownloadProgress {
    pub fn new(url: &str) -> Self {
        Self {
            display_name: truncate_url_for_display(url),
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests;
