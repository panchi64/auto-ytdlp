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

/// Reads the valid URLs out of links.txt, skipping blanks and unparseable lines.
pub fn get_links_from_file() -> Result<Vec<String>> {
    let content = fs::read_to_string(LINKS_FILE).map_err(AppError::Io)?;

    Ok(content
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .filter(|l| url::Url::parse(l).is_ok())
        .collect())
}

/// Drops invalid URLs from links.txt, returning how many were removed.
///
/// Takes the file lock: this is a read-modify-write, so without it a worker
/// finishing mid-scan would have its removal undone by the stale snapshot.
pub fn sanitize_links_file(state: &AppState) -> Result<usize> {
    let guard = state.acquire_file_lock()?;
    sanitize_links_file_internal(&guard)
}

fn sanitize_links_file_internal(_guard: &FileLockGuard<'_>) -> Result<usize> {
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

/// Replaces the download queue with the contents of links.txt.
///
/// `sanitize` drops invalid lines from the file first. Every failure is logged
/// to the TUI rather than stderr, which is invisible under the alternate screen.
pub fn load_links_into_queue(state: &AppState, sanitize: bool) {
    let log = |message: String| {
        if let Err(e) = state.add_log(message) {
            eprintln!("Error adding log: {}", e);
        }
    };

    if sanitize {
        match sanitize_links_file(state) {
            Ok(removed) if removed > 0 => {
                log(format!("Removed {} invalid URLs from links.txt", removed))
            }
            Ok(_) => {}
            Err(e) => log(format!("Error sanitizing links file: {}", e)),
        }
    }

    match get_links_from_file() {
        Ok(links) => {
            if let Err(e) = state.send(StateMessage::LoadLinks(links)) {
                log(format!("Error loading links: {}", e));
            }
        }
        Err(e) => log(format!("Error loading links: {}", e)),
    }
}

/// Adds the URLs found in clipboard text to links.txt and the queue.
///
/// Returns how many were new.
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
        let mut seen: std::collections::HashSet<String> = current_links.iter().cloned().collect();

        // `seen` grows as we go, so a URL repeated inside the pasted text is
        // only taken once - queueing it twice would spawn two yt-dlp processes
        // writing the same file.
        let new: Vec<String> = parsed
            .into_iter()
            .filter(|link| seen.insert(link.clone()))
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
