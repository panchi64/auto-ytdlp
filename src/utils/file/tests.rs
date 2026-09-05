use super::*;
use tempfile::TempDir;

/// Test helper to read links from a specific file path (not the hardcoded "links.txt")
fn get_links_from_file_at_path(path: &std::path::Path) -> Result<Vec<String>> {
    let content = fs::read_to_string(path).map_err(AppError::Io)?;

    Ok(content
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .filter(|l| url::Url::parse(l).is_ok())
        .collect())
}

/// Test helper to sanitize a links file at a specific path
fn sanitize_links_file_at_path(path: &std::path::Path) -> Result<usize> {
    let content = fs::read_to_string(path).map_err(AppError::Io)?;

    let mut total_non_empty = 0usize;
    let valid_lines: Vec<&str> = content
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .inspect(|_| total_non_empty += 1)
        .filter(|l| url::Url::parse(l).is_ok())
        .collect();

    let removed_count = total_non_empty - valid_lines.len();

    if removed_count > 0 {
        fs::write(path, valid_lines.join("\n")).map_err(AppError::Io)?;
    }

    Ok(removed_count)
}

#[test]
fn test_get_links_from_file_valid_urls() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let links_path = temp_dir.path().join("links.txt");

    let content = "https://example.com/video1\nhttps://youtube.com/watch?v=abc123\n";
    fs::write(&links_path, content).expect("failed to write test links file");

    let links = get_links_from_file_at_path(&links_path).expect("failed to read links file");
    assert_eq!(links.len(), 2);
    assert!(links.contains(&"https://example.com/video1".to_string()));
    assert!(links.contains(&"https://youtube.com/watch?v=abc123".to_string()));
}

#[test]
fn test_get_links_from_file_invalid_urls_filtered() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let links_path = temp_dir.path().join("links.txt");

    let content = "https://example.com/video1\nnot-a-valid-url\nhttps://example.com/video2\n";
    fs::write(&links_path, content).expect("failed to write test links file");

    let links = get_links_from_file_at_path(&links_path).expect("failed to read links file");
    assert_eq!(links.len(), 2);
    assert!(!links.iter().any(|l| l == "not-a-valid-url"));
}

#[test]
fn test_get_links_from_file_empty_file() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let links_path = temp_dir.path().join("links.txt");

    fs::write(&links_path, "").expect("failed to write empty test links file");

    let links = get_links_from_file_at_path(&links_path).expect("failed to read links file");
    assert!(links.is_empty());
}

#[test]
fn test_get_links_from_file_whitespace_and_blank_lines() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let links_path = temp_dir.path().join("links.txt");

    let content = "  https://example.com/video1  \n\n\n   \nhttps://example.com/video2\n\n";
    fs::write(&links_path, content).expect("failed to write test links file");

    let links = get_links_from_file_at_path(&links_path).expect("failed to read links file");
    assert_eq!(links.len(), 2);
    assert_eq!(links[0], "https://example.com/video1");
    assert_eq!(links[1], "https://example.com/video2");
}

#[test]
fn test_get_links_from_file_unicode_urls() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let links_path = temp_dir.path().join("links.txt");

    let content = "https://example.com/video?title=%E4%B8%AD%E6%96%87\n";
    fs::write(&links_path, content).expect("failed to write test links file");

    let links = get_links_from_file_at_path(&links_path).expect("failed to read links file");
    assert_eq!(links.len(), 1);
}

#[test]
fn test_sanitize_links_file_removes_invalid() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let links_path = temp_dir.path().join("links.txt");

    let content =
        "https://example.com/video1\ninvalid-url\nhttps://example.com/video2\nalso-invalid\n";
    fs::write(&links_path, content).expect("failed to write test links file");

    let removed = sanitize_links_file_at_path(&links_path).expect("failed to sanitize links file");
    assert_eq!(removed, 2);

    // Verify file content
    let remaining = fs::read_to_string(&links_path).expect("failed to read back links file");
    assert!(remaining.contains("https://example.com/video1"));
    assert!(remaining.contains("https://example.com/video2"));
    assert!(!remaining.contains("invalid-url"));
    assert!(!remaining.contains("also-invalid"));
}

#[test]
fn test_sanitize_links_file_no_invalid_urls() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let links_path = temp_dir.path().join("links.txt");

    let content = "https://example.com/video1\nhttps://example.com/video2\n";
    fs::write(&links_path, content).expect("failed to write test links file");

    let removed = sanitize_links_file_at_path(&links_path).expect("failed to sanitize links file");
    assert_eq!(removed, 0);
}

#[test]
fn test_sanitize_links_file_returns_count() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let links_path = temp_dir.path().join("links.txt");

    let content = "https://valid.com\nbad1\nbad2\nbad3\nhttps://also-valid.com\n";
    fs::write(&links_path, content).expect("failed to write test links file");

    let removed = sanitize_links_file_at_path(&links_path).expect("failed to sanitize links file");
    assert_eq!(removed, 3);
}

#[test]
fn test_get_links_file_not_found() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let links_path = temp_dir.path().join("links.txt");

    // Don't create links.txt
    let result = get_links_from_file_at_path(&links_path);
    assert!(result.is_err());
}

#[test]
fn test_get_links_very_long_urls() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let links_path = temp_dir.path().join("links.txt");

    // Create a very long but valid URL
    let long_path = "a".repeat(500);
    let long_url = format!("https://example.com/{}", long_path);
    fs::write(&links_path, &long_url).expect("failed to write test links file");

    let links = get_links_from_file_at_path(&links_path).expect("failed to read links file");
    assert_eq!(links.len(), 1);
    assert_eq!(links[0], long_url);
}

#[test]
fn test_sanitize_links_file_preserves_valid_content() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let links_path = temp_dir.path().join("links.txt");

    let content = "https://example.com/a\nbad\nhttps://example.com/b\n";
    fs::write(&links_path, content).expect("failed to write test links file");

    let removed = sanitize_links_file_at_path(&links_path).expect("failed to sanitize links file");
    assert_eq!(removed, 1);

    // Original file should still contain valid URLs
    let result = fs::read_to_string(&links_path).expect("failed to read back links file");
    assert!(result.contains("https://example.com/a"));
    assert!(result.contains("https://example.com/b"));
    assert!(!result.contains("bad"));
}

#[test]
fn test_sanitize_links_file_no_temp_file_left_behind() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let links_path = temp_dir.path().join("links.txt");
    let temp_path = temp_dir.path().join("links.txt.tmp");

    let content = "https://example.com/a\nbad\n";
    fs::write(&links_path, content).expect("failed to write test links file");

    sanitize_links_file_at_path(&links_path).expect("failed to sanitize links file");

    // No temp file should remain after the operation
    assert!(!temp_path.exists());
}

/// Helper that mirrors the dedup logic used in add_clipboard_links
fn deduplicate_links(existing: &[String], clipboard: &str) -> Vec<String> {
    let parsed: Vec<String> = clipboard
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .filter(|l| url::Url::parse(l).is_ok())
        .collect();

    let existing_set: std::collections::HashSet<_> = existing.iter().collect();

    parsed
        .into_iter()
        .filter(|link| !existing_set.contains(link))
        .collect()
}

#[test]
fn test_deduplicate_links_no_duplicates() {
    let existing = vec!["https://example.com/a".to_string()];
    let clipboard = "https://example.com/b\nhttps://example.com/c\n";

    let new = deduplicate_links(&existing, clipboard);
    assert_eq!(new.len(), 2);
    assert!(new.contains(&"https://example.com/b".to_string()));
    assert!(new.contains(&"https://example.com/c".to_string()));
}

#[test]
fn test_deduplicate_links_all_duplicates() {
    let existing = vec![
        "https://example.com/a".to_string(),
        "https://example.com/b".to_string(),
    ];
    let clipboard = "https://example.com/a\nhttps://example.com/b\n";

    let new = deduplicate_links(&existing, clipboard);
    assert!(new.is_empty());
}

#[test]
fn test_deduplicate_links_mixed() {
    let existing = vec!["https://example.com/a".to_string()];
    let clipboard = "https://example.com/a\nhttps://example.com/b\n";

    let new = deduplicate_links(&existing, clipboard);
    assert_eq!(new.len(), 1);
    assert_eq!(new[0], "https://example.com/b");
}

#[test]
fn test_deduplicate_links_filters_invalid_urls() {
    let existing: Vec<String> = vec![];
    let clipboard = "https://example.com/a\nnot-a-url\nhttps://example.com/b\n";

    let new = deduplicate_links(&existing, clipboard);
    assert_eq!(new.len(), 2);
    assert!(!new.iter().any(|l| l == "not-a-url"));
}

#[test]
fn test_deduplicate_links_empty_clipboard() {
    let existing = vec!["https://example.com/a".to_string()];
    let clipboard = "";

    let new = deduplicate_links(&existing, clipboard);
    assert!(new.is_empty());
}

#[test]
fn test_deduplicate_links_whitespace_only_clipboard() {
    let existing: Vec<String> = vec![];
    let clipboard = "   \n\n  \n";

    let new = deduplicate_links(&existing, clipboard);
    assert!(new.is_empty());
}

#[test]
fn test_deduplicate_links_trims_whitespace() {
    let existing: Vec<String> = vec![];
    let clipboard = "  https://example.com/a  \n  https://example.com/b  \n";

    let new = deduplicate_links(&existing, clipboard);
    assert_eq!(new.len(), 2);
    assert_eq!(new[0], "https://example.com/a");
    assert_eq!(new[1], "https://example.com/b");
}

// ==================== Real links.txt Pipeline ====================
//
// These drive the production functions against the per-thread test queue file,
// rather than the path-parameterized copies above.

fn write_queue_file(contents: &str) {
    fs::write(links_file_path(), contents).expect("failed to write the test queue file");
}

fn read_queue_file() -> String {
    fs::read_to_string(links_file_path()).expect("failed to read the test queue file")
}

fn test_state() -> AppState {
    AppState::new()
}

#[test]
fn test_clipboard_paste_dedupes_within_the_pasted_text() {
    write_queue_file("");
    let state = test_state();

    let added = add_clipboard_links(
        &state,
        "https://example.com/a\nhttps://example.com/a\nhttps://example.com/b\n",
    )
    .expect("failed to add clipboard links");

    assert_eq!(added, 2, "the repeated URL should only count once");
    let file = read_queue_file();
    assert_eq!(file.matches("https://example.com/a").count(), 1);
    assert_eq!(file.matches("https://example.com/b").count(), 1);
}

#[test]
fn test_clipboard_paste_skips_urls_already_in_the_file() {
    write_queue_file("https://example.com/a\n");
    let state = test_state();

    let added = add_clipboard_links(&state, "https://example.com/a\nhttps://example.com/c\n")
        .expect("failed to add clipboard links");

    assert_eq!(added, 1);
    assert_eq!(
        read_queue_file().matches("https://example.com/a").count(),
        1
    );
}

#[test]
fn test_sanitize_drops_invalid_lines_and_reports_the_count() {
    write_queue_file("https://example.com/a\nnot a url\n\nhttps://example.com/b\n");
    let state = test_state();

    let removed = sanitize_links_file(&state).expect("failed to sanitize");

    assert_eq!(removed, 1);
    let file = read_queue_file();
    assert!(!file.contains("not a url"));
    assert!(file.contains("https://example.com/a"));
    assert!(file.contains("https://example.com/b"));
}

#[test]
fn test_sanitize_is_a_no_op_when_every_line_is_valid() {
    write_queue_file("https://example.com/a\nhttps://example.com/b\n");
    let state = test_state();

    assert_eq!(sanitize_links_file(&state).expect("failed to sanitize"), 0);
    assert_eq!(read_queue_file().lines().count(), 2);
}

#[test]
fn test_load_links_into_queue_reports_the_count_it_loaded() {
    write_queue_file("https://example.com/a\nhttps://example.com/b\n");
    let state = test_state();

    let count = load_links_into_queue(&state, true).expect("expected the load to succeed");

    assert_eq!(count, 2);
    std::thread::sleep(std::time::Duration::from_millis(50));
    assert_eq!(state.queue_len().expect("failed to read queue"), 2);
}

#[test]
fn test_load_links_into_queue_errors_instead_of_reporting_success() {
    // A missing queue file must surface as Err, or the caller logs "Links loaded
    // from file" and the user acts on a stale queue.
    let state = test_state();
    write_queue_file("https://example.com/a\n");
    load_links_into_queue(&state, false).expect("failed to seed the queue");
    std::thread::sleep(std::time::Duration::from_millis(50));

    fs::remove_file(links_file_path()).expect("failed to remove the test queue file");

    assert!(
        load_links_into_queue(&state, false).is_err(),
        "a missing queue file must not report success"
    );
    // The previous queue is left untouched rather than silently emptied
    assert_eq!(state.queue_len().expect("failed to read queue"), 1);
}

#[test]
fn test_removing_a_link_leaves_the_rest_of_the_file_intact() {
    write_queue_file("https://example.com/a\nhttps://example.com/b\nhttps://example.com/c\n");
    let state = test_state();

    remove_link_from_file_sync(&state, "https://example.com/b").expect("failed to remove link");

    let file = read_queue_file();
    assert!(!file.contains("https://example.com/b"));
    assert!(file.contains("https://example.com/a"));
    assert!(file.contains("https://example.com/c"));
}
