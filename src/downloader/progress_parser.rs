//! Parser for yt-dlp output lines.
//!
//! Parses both structured progress template output and traditional yt-dlp output
//! to extract download progress information.

use std::time::Instant;

use crate::app_state::DownloadProgress;

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
    if line.contains(PROGRESS_MARKER_START) && line.contains(PROGRESS_MARKER_END)
        && let Some(progress) = parse_progress_template(line) {
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

/// Parses our custom progress template format
fn parse_progress_template(line: &str) -> Option<ProgressInfo> {
    // Format: |PROGRESS|status|percent|speed|eta|downloaded|total|frag_idx|frag_count|PROGRESS_END|
    let start = line.find(PROGRESS_MARKER_START)? + PROGRESS_MARKER_START.len();
    let end = line.find(PROGRESS_MARKER_END)?;

    if end <= start {
        return None;
    }

    let content = &line[start..end];
    let parts: Vec<&str> = content.split('|').collect();

    if parts.len() < 8 {
        return None;
    }

    let status = parts[0].to_string();
    let percent = parse_percent(parts[1]);
    let speed = parse_optional_string(parts[2]);
    let eta = parse_optional_string(parts[3]);
    let downloaded_bytes = parse_optional_u64(parts[4]);
    let total_bytes = parse_optional_u64(parts[5]);
    let fragment_index = parse_optional_u32(parts[6]);
    let fragment_count = parse_optional_u32(parts[7]);

    Some(ProgressInfo {
        status,
        percent,
        speed,
        eta,
        downloaded_bytes,
        total_bytes,
        fragment_index,
        fragment_count,
    })
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
        && let Some(progress) = parse_fragment_progress(line) {
            return ParsedOutput::Progress(progress);
        }

    // Other download info
    ParsedOutput::Info(line.to_string())
}

/// Parses traditional percentage-based progress lines
fn parse_traditional_progress(line: &str) -> Option<ProgressInfo> {
    // Pattern: "[download]  XX.X% of YY.YYMiB at ZZ.ZZMiB/s ETA HH:MM:SS"
    let percent_end = line.find('%')?;
    let percent_start = line[..percent_end]
        .rfind(|c: char| !c.is_ascii_digit() && c != '.')?
        + 1;

    let percent_str = &line[percent_start..percent_end];
    let percent: f64 = percent_str.trim().parse().ok()?;

    let mut info = ProgressInfo {
        status: if percent >= 100.0 {
            "finished"
        } else {
            "downloading"
        }
        .to_string(),
        percent,
        ..Default::default()
    };

    // Extract speed if present
    if let Some(at_idx) = line.find(" at ") {
        let speed_start = at_idx + 4;
        if let Some(speed_end) = line[speed_start..].find(' ') {
            info.speed = Some(line[speed_start..speed_start + speed_end].to_string());
        } else {
            // Speed is at end of line
            info.speed = Some(line[speed_start..].trim().to_string());
        }
    }

    // Extract ETA if present
    if let Some(eta_idx) = line.find("ETA ") {
        let eta_start = eta_idx + 4;
        let eta_str = line[eta_start..].trim();
        if !eta_str.is_empty() && eta_str != "Unknown" {
            info.eta = Some(eta_str.to_string());
        }
    }

    // Extract total size if present
    if let Some(of_idx) = line.find(" of ") {
        let size_start = of_idx + 4;
        if let Some(size_end) = line[size_start..].find(' ') {
            let size_str = &line[size_start..size_start + size_end];
            info.total_bytes = parse_size_string(size_str);
        }
    }

    Some(info)
}

/// Parses fragment-based progress (for HLS/DASH streams)
fn parse_fragment_progress(line: &str) -> Option<ProgressInfo> {
    // Pattern: "[download] Downloading item X of Y"
    // Or: "[download] Got X fragments out of Y"
    let mut info = ProgressInfo {
        status: "downloading".to_string(),
        ..Default::default()
    };

    // Try to extract "X of Y" pattern
    if let Some(of_idx) = line.find(" of ") {
        // Find the number before "of"
        let before_of = &line[..of_idx];
        let current: u32 = before_of
            .chars()
            .rev()
            .take_while(|c| c.is_ascii_digit())
            .collect::<String>()
            .chars()
            .rev()
            .collect::<String>()
            .parse()
            .ok()?;

        // Find the number after "of"
        let after_of = &line[of_idx + 4..];
        let total: u32 = after_of
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect::<String>()
            .parse()
            .ok()?;

        info.fragment_index = Some(current);
        info.fragment_count = Some(total);

        if total > 0 {
            info.percent = (current as f64 / total as f64) * 100.0;
        }
    }

    if info.fragment_index.is_some() {
        Some(info)
    } else {
        None
    }
}

/// Parses a percentage string (handles "XX.X%" format)
fn parse_percent(s: &str) -> f64 {
    let s = s.trim().trim_end_matches('%').trim();
    s.parse().unwrap_or(0.0)
}

/// Parses an optional string field (handles "NA", "N/A", empty, etc.)
fn parse_optional_string(s: &str) -> Option<String> {
    let s = s.trim();
    if s.is_empty() || s == "NA" || s == "N/A" || s == "Unknown" || s == "None" {
        None
    } else {
        Some(s.to_string())
    }
}

/// Parses an optional u64 field
fn parse_optional_u64(s: &str) -> Option<u64> {
    let s = s.trim();
    if s.is_empty() || s == "NA" || s == "N/A" || s == "None" {
        None
    } else {
        s.parse().ok()
    }
}

/// Parses an optional u32 field
fn parse_optional_u32(s: &str) -> Option<u32> {
    let s = s.trim();
    if s.is_empty() || s == "NA" || s == "N/A" || s == "None" {
        None
    } else {
        s.parse().ok()
    }
}

/// Parses a size string like "100.50MiB" to bytes
fn parse_size_string(s: &str) -> Option<u64> {
    let s = s.trim();

    // Try to find the numeric part
    let num_end = s
        .find(|c: char| !c.is_ascii_digit() && c != '.')
        .unwrap_or(s.len());
    let num_str = &s[..num_end];
    let num: f64 = num_str.parse().ok()?;

    let suffix = s[num_end..].to_lowercase();
    let multiplier: f64 = match suffix.as_str() {
        "b" | "" => 1.0,
        "kib" | "kb" | "k" => 1024.0,
        "mib" | "mb" | "m" => 1024.0 * 1024.0,
        "gib" | "gb" | "g" => 1024.0 * 1024.0 * 1024.0,
        "tib" | "tb" | "t" => 1024.0 * 1024.0 * 1024.0 * 1024.0,
        _ => return None,
    };

    Some((num * multiplier) as u64)
}

/// Converts ProgressInfo to DownloadProgress for a given URL
pub fn progress_info_to_download_progress(
    url: &str,
    display_name: &str,
    info: &ProgressInfo,
) -> DownloadProgress {
    DownloadProgress {
        url: url.to_string(),
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
mod tests {
    use super::*;

    #[test]
    fn test_parse_traditional_progress() {
        let line = "[download]  45.2% of 100.00MiB at 1.50MiB/s ETA 00:35";
        match parse_ytdlp_line(line) {
            ParsedOutput::Progress(info) => {
                assert!((info.percent - 45.2).abs() < 0.1);
                assert_eq!(info.speed, Some("1.50MiB/s".to_string()));
                assert_eq!(info.eta, Some("00:35".to_string()));
            }
            _ => panic!("Expected Progress"),
        }
    }

    #[test]
    fn test_parse_100_percent() {
        let line = "[download] 100% of 50.00MiB in 00:10";
        match parse_ytdlp_line(line) {
            ParsedOutput::Progress(info) => {
                assert!((info.percent - 100.0).abs() < 0.1);
                assert_eq!(info.status, "finished");
            }
            _ => panic!("Expected Progress"),
        }
    }

    #[test]
    fn test_parse_progress_template() {
        let line = "|PROGRESS|downloading|45.2%|1.5MiB/s|00:35|47368421|104857600|None|None|PROGRESS_END|";
        match parse_ytdlp_line(line) {
            ParsedOutput::Progress(info) => {
                assert!((info.percent - 45.2).abs() < 0.1);
                assert_eq!(info.status, "downloading");
                assert_eq!(info.speed, Some("1.5MiB/s".to_string()));
            }
            _ => panic!("Expected Progress"),
        }
    }

    #[test]
    fn test_parse_error() {
        let line = "ERROR: Unable to download webpage";
        match parse_ytdlp_line(line) {
            ParsedOutput::Error(msg) => {
                assert!(msg.contains("ERROR"));
            }
            _ => panic!("Expected Error"),
        }
    }

    #[test]
    fn test_parse_post_process() {
        let line = "[Merger] Merging formats into \"video.mp4\"";
        match parse_ytdlp_line(line) {
            ParsedOutput::PostProcess(msg) => {
                assert!(msg.contains("Merger"));
            }
            _ => panic!("Expected PostProcess"),
        }
    }
}
