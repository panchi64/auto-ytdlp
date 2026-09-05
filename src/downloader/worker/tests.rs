use super::*;

// ==================== Network Error Detection ====================

#[test]
fn test_check_network_error_unable_to_download() {
    assert!(check_network_error("ERROR: Unable to download webpage"));
    assert!(check_network_error(
        "Unable to download webpage: HTTP Error 503"
    ));
}

#[test]
fn test_check_network_error_http_error() {
    assert!(check_network_error("HTTP Error 404: Not Found"));
    assert!(check_network_error("ERROR: HTTP Error 500"));
    assert!(check_network_error("Got HTTP Error 502 Bad Gateway"));
}

#[test]
fn test_check_network_error_connection() {
    assert!(check_network_error("Connection refused"));
    assert!(check_network_error("Connection reset by peer"));
    assert!(check_network_error("Connection timed out"));
    assert!(check_network_error("ERROR: Connection failed"));
}

#[test]
fn test_check_network_error_timeout() {
    assert!(check_network_error("Timeout while connecting"));
    assert!(check_network_error("Request Timeout"));
    assert!(check_network_error("ERROR: Read Timeout"));
}

#[test]
fn test_check_network_error_network() {
    assert!(check_network_error("Network is unreachable"));
    assert!(check_network_error("Network error occurred"));
    assert!(check_network_error("ERROR: Network failure"));
}

#[test]
fn test_check_network_error_ssl() {
    assert!(check_network_error("SSL: CERTIFICATE_VERIFY_FAILED"));
    assert!(check_network_error("SSL handshake failed"));
    assert!(check_network_error("ERROR: SSL error"));
}

#[test]
fn test_check_network_error_false_positives() {
    // These should NOT be detected as network errors
    assert!(!check_network_error("Video unavailable"));
    assert!(!check_network_error("This video is private"));
    assert!(!check_network_error("ERROR: Unsupported URL"));
    assert!(!check_network_error("[download] 50% of 100.00MiB"));
    assert!(!check_network_error("Downloading video info"));
    assert!(!check_network_error("Format not available"));
}

#[test]
fn test_check_network_error_empty_string() {
    assert!(!check_network_error(""));
}

#[test]
fn test_check_network_error_case_sensitive() {
    // The function is case-sensitive, so these should not match
    assert!(!check_network_error("http error")); // lowercase
    assert!(!check_network_error("connection")); // lowercase
    assert!(!check_network_error("timeout")); // lowercase
    assert!(!check_network_error("network")); // lowercase
    assert!(!check_network_error("ssl")); // lowercase
}

#[test]
fn test_check_network_error_partial_matches() {
    // Partial/embedded matches should still work
    assert!(check_network_error("Some prefix HTTP Error suffix"));
    assert!(check_network_error("xxx Connection yyy"));
    assert!(check_network_error(
        "Error message with SSL certificate issue"
    ));
}

// ==================== Retry Classification ====================

/// Mirrors how the worker decides retryability from captured stderr: only ERROR
/// lines count, because yt-dlp writes routine warnings that contain the same
/// substrings check_network_error matches.
fn stderr_is_retryable(stderr: &str) -> bool {
    stderr
        .lines()
        .filter(|line| line.contains("ERROR"))
        .any(check_network_error)
}

#[test]
fn test_warning_lines_do_not_make_a_permanent_failure_retryable() {
    let stderr = "WARNING: [youtube] Unable to download webpage: HTTP Error 404\n\
                  ERROR: Video unavailable. This video is private\n";
    assert!(
        !stderr_is_retryable(stderr),
        "a private-video failure must not be retried because of a warning line"
    );
}

#[test]
fn test_error_lines_still_mark_network_failures_retryable() {
    let stderr = "WARNING: [youtube] nsig extraction failed\n\
                  ERROR: Unable to download webpage: <urlopen error timed out>\n";
    assert!(stderr_is_retryable(stderr));
}

#[test]
fn test_stderr_without_any_error_line_is_not_retryable() {
    let stderr = "WARNING: Falling back to generic extractor\n\
                  [info] Downloading 1 format(s)\n";
    assert!(!stderr_is_retryable(stderr));
}
