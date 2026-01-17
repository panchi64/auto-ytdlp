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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_youtube_watch_url_extraction() {
        let url = "https://www.youtube.com/watch?v=dQw4w9WgXcQ";
        let result = truncate_url_for_display(url);
        assert_eq!(result, "[dQw4w9WgXcQ]");
    }

    #[test]
    fn test_youtu_be_short_url_extraction() {
        let url = "https://youtu.be/dQw4w9WgXcQ";
        let result = truncate_url_for_display(url);
        assert_eq!(result, "[dQw4w9WgXcQ]");
    }

    #[test]
    fn test_youtube_with_extra_params() {
        // URL with playlist parameter
        let url =
            "https://www.youtube.com/watch?v=dQw4w9WgXcQ&list=PLrAXtmErZgOeiKm4sgNOknGvNjby9efdf";
        let result = truncate_url_for_display(url);
        assert_eq!(result, "[dQw4w9WgXcQ]");

        // URL with timestamp parameter
        let url = "https://www.youtube.com/watch?v=dQw4w9WgXcQ&t=120";
        let result = truncate_url_for_display(url);
        assert_eq!(result, "[dQw4w9WgXcQ]");

        // youtu.be with timestamp
        let url = "https://youtu.be/dQw4w9WgXcQ?t=120";
        let result = truncate_url_for_display(url);
        assert_eq!(result, "[dQw4w9WgXcQ]");
    }

    #[test]
    fn test_non_youtube_short_url() {
        // Short URL with a reasonable last segment
        let url = "https://example.com/video123";
        let result = truncate_url_for_display(url);
        assert_eq!(result, "video123");

        // URL with short file name
        let url = "https://example.com/media/clip.mp4";
        let result = truncate_url_for_display(url);
        assert_eq!(result, "clip.mp4");
    }

    #[test]
    fn test_non_youtube_long_url_truncation() {
        // URL with a last segment longer than 30 chars should truncate
        let url = "https://example.com/this_is_a_very_long_segment_that_exceeds_30_characters";
        let result = truncate_url_for_display(url);
        assert!(result.len() <= 30);
        assert!(result.ends_with("..."));

        // URL without a clean last segment (e.g., query string without path)
        // should also truncate if too long
        let url = "https://example.com?param=very_long_value_that_is_way_too_long_to_display";
        let result = truncate_url_for_display(url);
        assert!(result.len() <= 30);
    }

    #[test]
    fn test_trailing_slash_handling() {
        // URL ending with a trailing slash should not use empty last segment
        let url = "https://example.com/path/";
        let result = truncate_url_for_display(url);
        // The last segment after splitting on '/' is empty, so it should fall back
        // to truncation or use the URL itself if short enough
        assert!(!result.is_empty());
        assert!(result.len() <= 30 || result.ends_with("..."));
    }

    #[test]
    fn test_extract_youtube_id_various_formats() {
        // Standard watch URL
        assert_eq!(
            extract_youtube_id("https://www.youtube.com/watch?v=abc123XYZ_-"),
            Some("abc123XYZ_-".to_string())
        );

        // youtu.be format
        assert_eq!(
            extract_youtube_id("https://youtu.be/abc123XYZ_-"),
            Some("abc123XYZ_-".to_string())
        );

        // Non-YouTube URL
        assert_eq!(extract_youtube_id("https://vimeo.com/123456"), None);
    }
}
