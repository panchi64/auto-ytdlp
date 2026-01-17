/// Truncates a URL for display purposes.
///
/// For YouTube URLs, extracts the video ID. For other URLs,
/// shows the last portion of the URL path.
pub fn truncate_url_for_display(url: &str) -> String {
    // Try to extract YouTube video ID
    if (url.contains("youtube.com") || url.contains("youtu.be"))
        && let Some(id) = extract_youtube_id(url)
    {
        return format!("[{}]", id);
    }

    // For other URLs, use the last path segment or truncate
    if let Some(last_segment) = url.rsplit('/').next()
        && !last_segment.is_empty()
        && last_segment.len() <= 30
    {
        return last_segment.to_string();
    }

    // Fallback: truncate the URL
    if url.len() > 30 {
        format!("{}...", &url[..27])
    } else {
        url.to_string()
    }
}

/// Extracts the video ID from a YouTube URL
fn extract_youtube_id(url: &str) -> Option<String> {
    // Handle youtu.be/VIDEO_ID format
    if url.contains("youtu.be/")
        && let Some(id_start) = url.find("youtu.be/")
    {
        let id_portion = &url[id_start + 9..];
        let id = id_portion.split(&['?', '&', '/'][..]).next()?;
        if !id.is_empty() {
            return Some(id.to_string());
        }
    }

    // Handle youtube.com/watch?v=VIDEO_ID format
    if url.contains("v=")
        && let Some(v_start) = url.find("v=")
    {
        let id_portion = &url[v_start + 2..];
        let id = id_portion.split(&['?', '&', '/'][..]).next()?;
        if !id.is_empty() {
            return Some(id.to_string());
        }
    }

    None
}
