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
        && let Some(progress) = parse_fragment_progress(line)
    {
        return ParsedOutput::Progress(progress);
    }

    // Other download info
    ParsedOutput::Info(line.to_string())
}

/// Parses traditional percentage-based progress lines
fn parse_traditional_progress(line: &str) -> Option<ProgressInfo> {
    // Pattern: "[download]  XX.X% of YY.YYMiB at ZZ.ZZMiB/s ETA HH:MM:SS"
    let percent_end = line.find('%')?;
    let percent_start = line[..percent_end].rfind(|c: char| !c.is_ascii_digit() && c != '.')? + 1;

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
mod tests {
    use super::*;

    // ==================== Traditional Progress Parsing ====================

    #[test]
    fn test_parse_traditional_progress() {
        let line = "[download]  45.2% of 100.00MiB at 1.50MiB/s ETA 00:35";
        match parse_ytdlp_line(line) {
            ParsedOutput::Progress(info) => {
                assert!((info.percent - 45.2).abs() < 0.1);
                assert_eq!(info.speed, Some("1.50MiB/s".to_string()));
                assert_eq!(info.eta, Some("00:35".to_string()));
                assert_eq!(info.status, "downloading");
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
    fn test_parse_progress_without_eta() {
        let line = "[download]  25.0% of 50.00MiB at 2.00MiB/s ETA Unknown";
        match parse_ytdlp_line(line) {
            ParsedOutput::Progress(info) => {
                assert!((info.percent - 25.0).abs() < 0.1);
                assert_eq!(info.speed, Some("2.00MiB/s".to_string()));
                assert_eq!(info.eta, None); // "Unknown" should be filtered out
            }
            _ => panic!("Expected Progress"),
        }
    }

    #[test]
    fn test_parse_progress_speed_at_end_of_line() {
        let line = "[download]  10.0% of 100.00MiB at 5.00MiB/s";
        match parse_ytdlp_line(line) {
            ParsedOutput::Progress(info) => {
                assert!((info.percent - 10.0).abs() < 0.1);
                assert_eq!(info.speed, Some("5.00MiB/s".to_string()));
                assert_eq!(info.eta, None);
            }
            _ => panic!("Expected Progress"),
        }
    }

    // ==================== Progress Template Parsing ====================

    #[test]
    fn test_parse_progress_template() {
        let line =
            "|PROGRESS|downloading|45.2%|1.5MiB/s|00:35|47368421|104857600|None|None|PROGRESS_END|";
        match parse_ytdlp_line(line) {
            ParsedOutput::Progress(info) => {
                assert!((info.percent - 45.2).abs() < 0.1);
                assert_eq!(info.status, "downloading");
                assert_eq!(info.speed, Some("1.5MiB/s".to_string()));
                assert_eq!(info.eta, Some("00:35".to_string()));
                assert_eq!(info.downloaded_bytes, Some(47368421));
                assert_eq!(info.total_bytes, Some(104857600));
                assert_eq!(info.fragment_index, None);
                assert_eq!(info.fragment_count, None);
            }
            _ => panic!("Expected Progress"),
        }
    }

    #[test]
    fn test_parse_progress_template_with_fragments() {
        let line =
            "|PROGRESS|downloading|50.0%|2.0MiB/s|01:00|52428800|104857600|5|10|PROGRESS_END|";
        match parse_ytdlp_line(line) {
            ParsedOutput::Progress(info) => {
                assert!((info.percent - 50.0).abs() < 0.1);
                assert_eq!(info.fragment_index, Some(5));
                assert_eq!(info.fragment_count, Some(10));
            }
            _ => panic!("Expected Progress"),
        }
    }

    #[test]
    fn test_parse_progress_template_unknown_eta() {
        let line = "|PROGRESS|downloading|30.0%|1.0MiB/s|Unknown|31457280|104857600|None|None|PROGRESS_END|";
        match parse_ytdlp_line(line) {
            ParsedOutput::Progress(info) => {
                assert!((info.percent - 30.0).abs() < 0.1);
                assert_eq!(info.eta, None); // "Unknown" should be filtered
            }
            _ => panic!("Expected Progress"),
        }
    }

    #[test]
    fn test_parse_progress_template_na_speed() {
        let line =
            "|PROGRESS|downloading|15.0%|N/A|00:30|15728640|104857600|None|None|PROGRESS_END|";
        match parse_ytdlp_line(line) {
            ParsedOutput::Progress(info) => {
                assert!((info.percent - 15.0).abs() < 0.1);
                assert_eq!(info.speed, None); // "N/A" should be filtered
                assert_eq!(info.eta, Some("00:30".to_string()));
            }
            _ => panic!("Expected Progress"),
        }
    }

    #[test]
    fn test_parse_progress_template_finished_status() {
        let line = "|PROGRESS|finished|100%|N/A|N/A|104857600|104857600|None|None|PROGRESS_END|";
        match parse_ytdlp_line(line) {
            ParsedOutput::Progress(info) => {
                assert!((info.percent - 100.0).abs() < 0.1);
                assert_eq!(info.status, "finished");
                assert_eq!(info.speed, None);
                assert_eq!(info.eta, None);
            }
            _ => panic!("Expected Progress"),
        }
    }

    #[test]
    fn test_parse_progress_template_malformed_too_few_parts() {
        // Only 5 parts instead of 8
        let line = "|PROGRESS|downloading|50%|1.0MiB/s|00:30|PROGRESS_END|";
        match parse_ytdlp_line(line) {
            ParsedOutput::Info(_) => {} // Should fall through to Info since template parsing fails
            _ => panic!("Expected Info for malformed template"),
        }
    }

    #[test]
    fn test_parse_progress_template_malformed_missing_end_marker() {
        let line = "|PROGRESS|downloading|50%|1.0MiB/s|00:30|52428800|104857600|None|None|";
        match parse_ytdlp_line(line) {
            ParsedOutput::Info(_) => {} // Should fall through
            _ => panic!("Expected Info for malformed template"),
        }
    }

    // ==================== Fragment Progress Parsing ====================

    #[test]
    fn test_parse_fragment_progress_downloading_item() {
        let line = "[download] Downloading item 5 of 10";
        match parse_ytdlp_line(line) {
            ParsedOutput::Progress(info) => {
                assert_eq!(info.fragment_index, Some(5));
                assert_eq!(info.fragment_count, Some(10));
                assert!((info.percent - 50.0).abs() < 0.1);
                assert_eq!(info.status, "downloading");
            }
            _ => panic!("Expected Progress"),
        }
    }

    #[test]
    fn test_parse_fragment_progress_with_fragment_keyword() {
        // The parser checks for "fragment" keyword AND " of " pattern
        let line = "[download] Downloaded fragment 3 of 12";
        match parse_ytdlp_line(line) {
            ParsedOutput::Progress(info) => {
                assert_eq!(info.fragment_index, Some(3));
                assert_eq!(info.fragment_count, Some(12));
                assert!((info.percent - 25.0).abs() < 0.1);
            }
            _ => panic!("Expected Progress"),
        }
    }

    // ==================== Post-Processing Detection ====================

    #[test]
    fn test_parse_post_process_merger() {
        let line = "[Merger] Merging formats into \"video.mp4\"";
        match parse_ytdlp_line(line) {
            ParsedOutput::PostProcess(msg) => {
                assert!(msg.contains("Merger"));
                assert!(msg.contains("video.mp4"));
            }
            _ => panic!("Expected PostProcess"),
        }
    }

    #[test]
    fn test_parse_post_process_ffmpeg() {
        let line = "[ffmpeg] Destination: video_processed.mp4";
        match parse_ytdlp_line(line) {
            ParsedOutput::PostProcess(msg) => {
                assert!(msg.contains("ffmpeg"));
                assert!(msg.contains("Destination"));
            }
            _ => panic!("Expected PostProcess"),
        }
    }

    // ==================== Already Downloaded Detection ====================

    #[test]
    fn test_parse_already_downloaded_archive() {
        // Note: Lines that start with [download] go through parse_download_line
        // and return Info for unmatched patterns. "Already downloaded" detection
        // only works for lines that don't start with [download].
        let line = "Video abc123 has already been recorded in the archive";
        match parse_ytdlp_line(line) {
            ParsedOutput::AlreadyDownloaded(msg) => {
                assert!(msg.contains("already been recorded in the archive"));
            }
            _ => panic!("Expected AlreadyDownloaded"),
        }
    }

    #[test]
    fn test_parse_already_downloaded_file_exists() {
        // Lines without [download] prefix can trigger AlreadyDownloaded
        let line = "video.mp4 has already been downloaded";
        match parse_ytdlp_line(line) {
            ParsedOutput::AlreadyDownloaded(msg) => {
                assert!(msg.contains("has already been downloaded"));
            }
            _ => panic!("Expected AlreadyDownloaded"),
        }
    }

    #[test]
    fn test_parse_download_line_with_already_downloaded_returns_info() {
        // Lines starting with [download] that contain "already downloaded"
        // go through parse_download_line which returns Info for unmatched patterns
        let line = "[download] video.mp4 has already been downloaded";
        match parse_ytdlp_line(line) {
            ParsedOutput::Info(msg) => {
                assert!(msg.contains("has already been downloaded"));
            }
            _ => panic!("Expected Info (download lines go through different path)"),
        }
    }

    // ==================== Error Detection ====================

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
    fn test_parse_error_inline() {
        let line = "Some message with ERROR in the middle";
        match parse_ytdlp_line(line) {
            ParsedOutput::Error(msg) => {
                assert!(msg.contains("ERROR"));
            }
            _ => panic!("Expected Error"),
        }
    }

    // ==================== Ignored Lines ====================

    #[test]
    fn test_parse_ignore_youtube_extractor() {
        let line = "[youtube] abc123: Downloading webpage";
        match parse_ytdlp_line(line) {
            ParsedOutput::Ignore => {}
            _ => panic!("Expected Ignore for [youtube] line"),
        }
    }

    #[test]
    fn test_parse_ignore_info() {
        let line = "[info] Available formats for abc123";
        match parse_ytdlp_line(line) {
            ParsedOutput::Ignore => {}
            _ => panic!("Expected Ignore for [info] line"),
        }
    }

    #[test]
    fn test_parse_ignore_empty_line() {
        let line = "   ";
        match parse_ytdlp_line(line) {
            ParsedOutput::Ignore => {}
            _ => panic!("Expected Ignore for empty line"),
        }
    }

    #[test]
    fn test_parse_extract_audio_with_destination_returns_destination() {
        // Lines containing "Destination:" are caught before [ExtractAudio] ignore check
        let line = "[ExtractAudio] Destination: audio.mp3";
        match parse_ytdlp_line(line) {
            ParsedOutput::Destination(msg) => {
                assert!(msg.contains("Destination"));
                assert!(msg.contains("audio.mp3"));
            }
            _ => panic!("Expected Destination (Destination: check comes before [ExtractAudio])"),
        }
    }

    #[test]
    fn test_parse_ignore_extract_audio_without_destination() {
        // [ExtractAudio] lines without "Destination:" are ignored
        let line = "[ExtractAudio] Converting audio";
        match parse_ytdlp_line(line) {
            ParsedOutput::Ignore => {}
            _ => panic!("Expected Ignore for [ExtractAudio] line without Destination"),
        }
    }

    // ==================== Size String Parsing ====================

    #[test]
    fn test_parse_size_string_bytes() {
        assert_eq!(parse_size_string("1024b"), Some(1024));
        assert_eq!(parse_size_string("500B"), Some(500));
        assert_eq!(parse_size_string("100"), Some(100));
    }

    #[test]
    fn test_parse_size_string_kib() {
        assert_eq!(parse_size_string("1KiB"), Some(1024));
        assert_eq!(parse_size_string("2.5kib"), Some(2560));
        assert_eq!(parse_size_string("10KB"), Some(10240));
    }

    #[test]
    fn test_parse_size_string_mib() {
        assert_eq!(parse_size_string("1MiB"), Some(1024 * 1024));
        assert_eq!(parse_size_string("100.5MiB"), Some(105381888));
        assert_eq!(parse_size_string("50MB"), Some(50 * 1024 * 1024));
    }

    #[test]
    fn test_parse_size_string_gib() {
        assert_eq!(parse_size_string("1GiB"), Some(1024 * 1024 * 1024));
        assert_eq!(parse_size_string("2GB"), Some(2 * 1024 * 1024 * 1024));
    }

    #[test]
    fn test_parse_size_string_invalid() {
        assert_eq!(parse_size_string("invalid"), None);
        assert_eq!(parse_size_string("100XB"), None);
        assert_eq!(parse_size_string(""), None);
    }

    // ==================== Optional Value Parsing ====================

    #[test]
    fn test_parse_optional_string_valid() {
        assert_eq!(parse_optional_string("hello"), Some("hello".to_string()));
        assert_eq!(
            parse_optional_string("  value  "),
            Some("value".to_string())
        );
    }

    #[test]
    fn test_parse_optional_string_na_variants() {
        assert_eq!(parse_optional_string("NA"), None);
        assert_eq!(parse_optional_string("N/A"), None);
        assert_eq!(parse_optional_string("Unknown"), None);
        assert_eq!(parse_optional_string("None"), None);
        assert_eq!(parse_optional_string(""), None);
        assert_eq!(parse_optional_string("   "), None);
    }

    #[test]
    fn test_parse_optional_u64_valid() {
        assert_eq!(parse_optional_u64("12345"), Some(12345));
        assert_eq!(parse_optional_u64("  999  "), Some(999));
    }

    #[test]
    fn test_parse_optional_u64_invalid() {
        assert_eq!(parse_optional_u64("NA"), None);
        assert_eq!(parse_optional_u64("N/A"), None);
        assert_eq!(parse_optional_u64("None"), None);
        assert_eq!(parse_optional_u64("not_a_number"), None);
        assert_eq!(parse_optional_u64(""), None);
    }

    #[test]
    fn test_parse_optional_u32_valid() {
        assert_eq!(parse_optional_u32("100"), Some(100));
        assert_eq!(parse_optional_u32("  42  "), Some(42));
    }

    #[test]
    fn test_parse_optional_u32_invalid() {
        assert_eq!(parse_optional_u32("NA"), None);
        assert_eq!(parse_optional_u32("N/A"), None);
        assert_eq!(parse_optional_u32("None"), None);
        assert_eq!(parse_optional_u32("abc"), None);
    }

    // ==================== Percent Parsing ====================

    #[test]
    fn test_parse_percent_with_symbol() {
        assert!((parse_percent("45.2%") - 45.2).abs() < 0.1);
        assert!((parse_percent("100%") - 100.0).abs() < 0.1);
        assert!((parse_percent("0%") - 0.0).abs() < 0.1);
    }

    #[test]
    fn test_parse_percent_without_symbol() {
        assert!((parse_percent("75.5") - 75.5).abs() < 0.1);
        assert!((parse_percent("  50.0  ") - 50.0).abs() < 0.1);
    }

    #[test]
    fn test_parse_percent_invalid() {
        assert!((parse_percent("invalid") - 0.0).abs() < 0.1);
        assert!((parse_percent("") - 0.0).abs() < 0.1);
    }

    // ==================== Destination Parsing ====================

    #[test]
    fn test_parse_destination() {
        let line = "[download] Destination: /path/to/video.mp4";
        match parse_ytdlp_line(line) {
            ParsedOutput::Destination(msg) => {
                assert!(msg.contains("Destination"));
                assert!(msg.contains("/path/to/video.mp4"));
            }
            _ => panic!("Expected Destination"),
        }
    }

    #[test]
    fn test_parse_destination_non_download_prefix() {
        let line = "Destination: /some/other/file.mp4";
        match parse_ytdlp_line(line) {
            ParsedOutput::Destination(msg) => {
                assert!(msg.contains("Destination"));
            }
            _ => panic!("Expected Destination"),
        }
    }
}
