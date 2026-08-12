use super::ProgressInfo;
use super::scalars::{
    parse_optional_string, parse_optional_u32, parse_optional_u64, parse_percent,
};
use super::{PROGRESS_MARKER_END, PROGRESS_MARKER_START};

/// Parses our custom progress template format
pub(super) fn parse_progress_template(line: &str) -> Option<ProgressInfo> {
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
