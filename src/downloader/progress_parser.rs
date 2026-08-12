use std::time::Instant;

use crate::app_state::DownloadProgress;

mod scalars;
mod template;
mod traditional;

use template::parse_progress_template;
use traditional::{parse_fragment_progress, parse_traditional_progress};

/// Represents a parsed line from yt-dlp output
#[derive(Debug, Clone)]
pub enum ParsedOutput {
    /// Progress update with download information
    Progress(ProgressInfo),
    /// Post-processing status (merging, converting, etc.)
    PostProcess(String),
    /// Destination file path
    Destination(String),
    /// Already downloaded (from archive)
    AlreadyDownloaded(String),
    /// Error message
    Error(String),
    /// Other informational output (should be logged)
    Info(String),
    /// Output that should be ignored (not logged)
    Ignore,
}

/// Progress information extracted from yt-dlp output
#[derive(Debug, Clone, Default)]
pub struct ProgressInfo {
    /// Status: "downloading", "finished", "error"
    pub status: String,
    /// Download percentage (0.0 - 100.0)
    pub percent: f64,
    /// Download speed string (e.g., "1.5MiB/s")
    pub speed: Option<String>,
    /// ETA string (e.g., "00:05:23")
    pub eta: Option<String>,
    /// Downloaded bytes
    pub downloaded_bytes: Option<u64>,
    /// Total bytes
    pub total_bytes: Option<u64>,
    /// Fragment index (for HLS/DASH)
    pub fragment_index: Option<u32>,
    /// Fragment count (for HLS/DASH)
    pub fragment_count: Option<u32>,
}

/// Custom progress template marker for parsing
pub const PROGRESS_MARKER_START: &str = "|PROGRESS|";
pub const PROGRESS_MARKER_END: &str = "|PROGRESS_END|";

/// Parses a line of yt-dlp output
pub fn parse_ytdlp_line(line: &str) -> ParsedOutput {
    let line = line.trim();

    // Skip empty lines
    if line.is_empty() {
        return ParsedOutput::Ignore;
    }

    // Try parsing custom progress template first
    if line.contains(PROGRESS_MARKER_START)
        && line.contains(PROGRESS_MARKER_END)
        && let Some(progress) = parse_progress_template(line)
    {
        return ParsedOutput::Progress(progress);
    }

    // Parse traditional yt-dlp output patterns
    if line.starts_with("[download]") {
        return parse_download_line(line);
    }

    if line.starts_with("[Merger]") || line.starts_with("[ffmpeg]") {
        return ParsedOutput::PostProcess(line.to_string());
    }

    if line.contains("Destination:") {
        return ParsedOutput::Destination(line.to_string());
    }

    if line.contains("has already been recorded in the archive")
        || line.contains("has already been downloaded")
    {
        return ParsedOutput::AlreadyDownloaded(line.to_string());
    }

    if line.contains("ERROR") || line.starts_with("ERROR:") {
        return ParsedOutput::Error(line.to_string());
    }

    // Filter out noise - common lines that don't need logging
    if line.starts_with("[youtube]")
        || line.starts_with("[info]")
        || line.starts_with("[debug]")
        || line.starts_with("[generic]")
        || line.starts_with("[ExtractAudio]")
    {
        return ParsedOutput::Ignore;
    }

    // Everything else is informational
    ParsedOutput::Info(line.to_string())
}

/// Parses traditional [download] lines from yt-dlp
fn parse_download_line(line: &str) -> ParsedOutput {
    // Handle "100% of X" completion line
    if line.contains("100%") && line.contains(" of ") {
        return ParsedOutput::Progress(ProgressInfo {
            status: "finished".to_string(),
            percent: 100.0,
            ..Default::default()
        });
    }

    // Handle progress lines like "[download]  45.2% of 100.00MiB at 1.50MiB/s ETA 00:35"
    if let Some(progress) = parse_traditional_progress(line) {
        return ParsedOutput::Progress(progress);
    }

    // Handle destination lines
    if line.contains("Destination:") {
        return ParsedOutput::Destination(line.to_string());
    }

    // Handle fragment downloads
    if (line.contains("Downloading item") || line.contains("fragment"))
        && let Some(progress) = parse_fragment_progress(line)
    {
        return ParsedOutput::Progress(progress);
    }

    // Other download info
    ParsedOutput::Info(line.to_string())
}

/// Converts ProgressInfo to DownloadProgress for display
pub fn progress_info_to_download_progress(
    display_name: &str,
    info: &ProgressInfo,
) -> DownloadProgress {
    DownloadProgress {
        display_name: display_name.to_string(),
        phase: info.status.clone(),
        percent: info.percent,
        speed: info.speed.clone(),
        eta: info.eta.clone(),
        downloaded_bytes: info.downloaded_bytes,
        total_bytes: info.total_bytes,
        fragment_index: info.fragment_index,
        fragment_count: info.fragment_count,
        last_update: Instant::now(),
    }
}

#[cfg(test)]
mod tests;
