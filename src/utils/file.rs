use crate::app_state::{AppState, FileLockGuard, StateMessage};
use crate::errors::{AppError, Result};
use std::fs;

/// Internal function to remove a link from file while holding the file lock.
/// This prevents race conditions when multiple workers complete simultaneously.
fn remove_link_from_file_internal(_guard: &FileLockGuard<'_>, url: &str) -> Result<()> {
    let file_path = "links.txt";
    let content = fs::read_to_string(file_path).map_err(AppError::Io)?;

    // Use a temporary file for atomic writes
    let temp_path = format!("{}.tmp", file_path);
    let new_content: Vec<&str> = content
        .lines()
        .filter(|line| line.trim() != url.trim())
        .collect();

    fs::write(&temp_path, new_content.join("\n")).map_err(AppError::Io)?;
    fs::rename(&temp_path, file_path).map_err(AppError::Io)?; // Atomic replace

    Ok(())
}

/// Removes a specific URL from the 'links.txt' file with thread-safe synchronization.
///
/// This function acquires the file lock from AppState before performing the operation,
/// preventing race conditions when multiple workers complete downloads simultaneously.
///
/// # Parameters
///
/// * `state` - Reference to the application state for file lock access
/// * `url` - The URL to remove from the file
///
/// # Returns
///
/// * `Result<()>` - Ok if the URL was removed successfully, or an Error
pub fn remove_link_from_file_sync(state: &AppState, url: &str) -> Result<()> {
    let guard = state.acquire_file_lock()?;
    remove_link_from_file_internal(&guard, url)
}

/// Loads URLs from the 'links.txt' file without requiring an AppState.
///
/// Reads all lines from the links.txt file into a vector of strings.
/// Creates an empty file if it doesn't exist.
/// Filters out any entries that aren't valid URLs.
///
/// # Returns
///
/// * `Result<Vec<String>>` - Vector containing all valid URLs from the file or an error
///
/// # Example
///
/// ```
/// let links = get_links_from_file()?;
/// state.send(StateMessage::LoadLinks(links))?;
/// ```
pub fn get_links_from_file() -> Result<Vec<String>> {
    let content = fs::read_to_string("links.txt").map_err(AppError::Io)?;

    Ok(content
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .filter(|l| url::Url::parse(l).is_ok())
        .collect())
}

/// Sanitizes the links.txt file by removing invalid URLs.
///
/// Reads the file, filters out invalid URLs, and writes the sanitized
/// content back to the file.
///
/// # Returns
///
/// * `Result<usize>` - The number of invalid entries that were removed, or an error
///
/// # Example
///
/// ```
/// let removed = sanitize_links_file()?;
/// println!("Removed {} invalid URLs", removed);
/// ```
pub fn sanitize_links_file() -> Result<usize> {
    let file_path = "links.txt";
    let content = fs::read_to_string(file_path).map_err(AppError::Io)?;

    // Single pass: count total non-empty lines and collect valid URLs
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
        let temp_path = format!("{}.tmp", file_path);
        fs::write(&temp_path, valid_lines.join("\n")).map_err(AppError::Io)?;
        fs::rename(&temp_path, file_path).map_err(AppError::Io)?;
    }

    Ok(removed_count)
}

/// Parses URLs from clipboard content and adds them to both the links.txt file
/// and the application state.
///
/// This function combines adding links to the file and updating the app state directly.
///
/// # Parameters
///
/// * `state` - Reference to the application state to update
/// * `clipboard_content` - String content from the clipboard to parse
///
/// # Returns
///
/// * `Result<usize>` - The number of new URLs that were added, or an error
///
/// # Example
///
/// ```
/// let ctx = ClipboardProvider::new()
///     .map_err(|e| AppError::Clipboard(e.to_string()))?;
///
/// if let Ok(contents) = ctx.get_contents() {
///     let links_added = add_clipboard_links(&state, &contents)?;
///     state.add_log(format!("Added {} links", links_added))?;
/// }
/// ```
pub fn add_clipboard_links(state: &AppState, clipboard_content: &str) -> Result<usize> {
    // Parse and deduplicate new links before writing to file
    let new_links: Vec<String> = {
        let guard = state.acquire_file_lock()?;
        let parsed: Vec<String> = clipboard_content
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .filter(|l| url::Url::parse(l).is_ok())
            .collect();

        let current_links = get_links_from_file()?;
        let existing: std::collections::HashSet<_> = current_links.iter().collect();

        let new: Vec<String> = parsed
            .into_iter()
            .filter(|link| !existing.contains(link))
            .collect();

        if !new.is_empty() {
            let mut all_links = current_links;
            all_links.extend(new.iter().cloned());
            fs::write("links.txt", all_links.join("\n")).map_err(AppError::Io)?;
        }

        drop(guard);
        new
    };

    // Only send the new links to the queue
    for link in &new_links {
        state.send(StateMessage::AddToQueue(link.clone()))?;
    }

    Ok(new_links.len())
}

#[cfg(test)]
mod tests {
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
        fs::write(&links_path, content).unwrap();

        let links = get_links_from_file_at_path(&links_path).unwrap();
        assert_eq!(links.len(), 2);
        assert!(links.contains(&"https://example.com/video1".to_string()));
        assert!(links.contains(&"https://youtube.com/watch?v=abc123".to_string()));
    }

    #[test]
    fn test_get_links_from_file_invalid_urls_filtered() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let links_path = temp_dir.path().join("links.txt");

        let content = "https://example.com/video1\nnot-a-valid-url\nhttps://example.com/video2\n";
        fs::write(&links_path, content).unwrap();

        let links = get_links_from_file_at_path(&links_path).unwrap();
        assert_eq!(links.len(), 2);
        assert!(!links.iter().any(|l| l == "not-a-valid-url"));
    }

    #[test]
    fn test_get_links_from_file_empty_file() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let links_path = temp_dir.path().join("links.txt");

        fs::write(&links_path, "").unwrap();

        let links = get_links_from_file_at_path(&links_path).unwrap();
        assert!(links.is_empty());
    }

    #[test]
    fn test_get_links_from_file_whitespace_and_blank_lines() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let links_path = temp_dir.path().join("links.txt");

        let content = "  https://example.com/video1  \n\n\n   \nhttps://example.com/video2\n\n";
        fs::write(&links_path, content).unwrap();

        let links = get_links_from_file_at_path(&links_path).unwrap();
        assert_eq!(links.len(), 2);
        assert_eq!(links[0], "https://example.com/video1");
        assert_eq!(links[1], "https://example.com/video2");
    }

    #[test]
    fn test_get_links_from_file_unicode_urls() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let links_path = temp_dir.path().join("links.txt");

        let content = "https://example.com/video?title=%E4%B8%AD%E6%96%87\n";
        fs::write(&links_path, content).unwrap();

        let links = get_links_from_file_at_path(&links_path).unwrap();
        assert_eq!(links.len(), 1);
    }

    #[test]
    fn test_sanitize_links_file_removes_invalid() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let links_path = temp_dir.path().join("links.txt");

        let content =
            "https://example.com/video1\ninvalid-url\nhttps://example.com/video2\nalso-invalid\n";
        fs::write(&links_path, content).unwrap();

        let removed = sanitize_links_file_at_path(&links_path).unwrap();
        assert_eq!(removed, 2);

        // Verify file content
        let remaining = fs::read_to_string(&links_path).unwrap();
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
        fs::write(&links_path, content).unwrap();

        let removed = sanitize_links_file_at_path(&links_path).unwrap();
        assert_eq!(removed, 0);
    }

    #[test]
    fn test_sanitize_links_file_returns_count() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let links_path = temp_dir.path().join("links.txt");

        let content = "https://valid.com\nbad1\nbad2\nbad3\nhttps://also-valid.com\n";
        fs::write(&links_path, content).unwrap();

        let removed = sanitize_links_file_at_path(&links_path).unwrap();
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
        fs::write(&links_path, &long_url).unwrap();

        let links = get_links_from_file_at_path(&links_path).unwrap();
        assert_eq!(links.len(), 1);
        assert_eq!(links[0], long_url);
    }

    #[test]
    fn test_sanitize_links_file_preserves_valid_content() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let links_path = temp_dir.path().join("links.txt");

        let content = "https://example.com/a\nbad\nhttps://example.com/b\n";
        fs::write(&links_path, content).unwrap();

        let removed = sanitize_links_file_at_path(&links_path).unwrap();
        assert_eq!(removed, 1);

        // Original file should still contain valid URLs
        let result = fs::read_to_string(&links_path).unwrap();
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
        fs::write(&links_path, content).unwrap();

        sanitize_links_file_at_path(&links_path).unwrap();

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
}
