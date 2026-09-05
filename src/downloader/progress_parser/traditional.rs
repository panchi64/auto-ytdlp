use super::ProgressInfo;
use super::scalars::parse_size_string;

/// Parses traditional percentage-based progress lines
pub(super) fn parse_traditional_progress(line: &str) -> Option<ProgressInfo> {
    // Pattern: "[download]  XX.X% of YY.YYMiB at ZZ.ZZMiB/s ETA HH:MM:SS"
    let percent_end = line.find('%')?;
    // Walk back over the digits by char, not by byte: adding 1 to a byte index
    // lands mid-character when the char before the number is multi-byte (a title
    // like "Sale—50% off"), and slicing there panics.
    let percent_start = line[..percent_end]
        .char_indices()
        .rev()
        .find(|(_, c)| !c.is_ascii_digit() && *c != '.')
        .map(|(idx, c)| idx + c.len_utf8())?;

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
pub(super) fn parse_fragment_progress(line: &str) -> Option<ProgressInfo> {
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
