//! Field parsers for the scalar pieces of yt-dlp's progress output.

pub(super) fn parse_percent(s: &str) -> f64 {
    let s = s.trim().trim_end_matches('%').trim();
    s.parse().unwrap_or(0.0)
}

/// yt-dlp writes "NA"/"N/A"/"Unknown"/"None" for fields it could not determine.
pub(super) fn parse_optional_string(s: &str) -> Option<String> {
    let s = s.trim();
    if s.is_empty() || s == "NA" || s == "N/A" || s == "Unknown" || s == "None" {
        None
    } else {
        Some(s.to_string())
    }
}

pub(super) fn parse_optional_u64(s: &str) -> Option<u64> {
    let s = s.trim();
    if s.is_empty() || s == "NA" || s == "N/A" || s == "None" {
        None
    } else {
        s.parse().ok()
    }
}

pub(super) fn parse_optional_u32(s: &str) -> Option<u32> {
    let s = s.trim();
    if s.is_empty() || s == "NA" || s == "N/A" || s == "None" {
        None
    } else {
        s.parse().ok()
    }
}

/// Parses a size string like "100.50MiB" to bytes
pub(super) fn parse_size_string(s: &str) -> Option<u64> {
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
