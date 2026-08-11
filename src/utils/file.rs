use crate::app_state::{AppState, FileLockGuard, StateMessage};
use crate::errors::{AppError, Result};
use std::fs;

/// Default filename for the download queue
pub const LINKS_FILE: &str = "links.txt";

/// Internal function to remove a link from file while holding the file lock.
/// This prevents race conditions when multiple workers complete simultaneously.
fn remove_link_from_file_internal(_guard: &FileLockGuard<'_>, url: &str) -> Result<()> {
    let file_path = LINKS_FILE;
    let content = fs::read_to_string(file_path).map_err(AppError::Io)?;

    // Use a temporary file for atomic writes
    let temp_path = format!("{}.tmp", file_path);
    let new_content: Vec<&str> = content
        .lines()
        .filter(|line| line.trim() != url.trim())
        .collect();

    fs::write(&temp_path, new_content.join("\n") + "\n").map_err(AppError::Io)?;
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
    let content = fs::read_to_string(LINKS_FILE).map_err(AppError::Io)?;

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
    let file_path = LINKS_FILE;
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
        fs::write(&temp_path, valid_lines.join("\n") + "\n").map_err(AppError::Io)?;
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
            fs::write(LINKS_FILE, all_links.join("\n") + "\n").map_err(AppError::Io)?;
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
mod tests;
