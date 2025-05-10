use crate::app_state::{AppState, StateMessage};
use anyhow::Result;
use std::{collections::HashSet, fs};

/// Loads URLs from the 'links.txt' file without requiring an AppState.
///
/// Reads all lines from the links.txt file into a vector of strings.
/// Creates an empty file if it doesn't exist.
/// Filters out any entries that aren't valid URLs.
///
/// # Returns
///
/// * `Vec<String>` - Vector containing all valid URLs from the file
///
/// # Example
///
/// ```
/// let links = get_links_from_file();
/// state.send(StateMessage::LoadLinks(links));
/// ```
pub fn get_links_from_file() -> Vec<String> {
    let content = fs::read_to_string("links.txt").unwrap_or_default();
    content
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .filter(|l| url::Url::parse(l).is_ok())
        .collect()
}

/// Sanitizes the links.txt file by removing invalid URLs.
///
/// Reads the file, filters out invalid URLs, and writes the sanitized
/// content back to the file.
///
/// # Returns
///
/// * `usize` - The number of invalid entries that were removed
///
/// # Example
///
/// ```
/// let removed = sanitize_links_file();
/// println!("Removed {} invalid URLs", removed);
/// ```
pub fn sanitize_links_file() -> usize {
    let file_path = "links.txt";
    let content = fs::read_to_string(file_path).unwrap_or_default();

    let lines: Vec<String> = content
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();

    let valid_lines: Vec<String> = lines
        .iter()
        .filter(|l| url::Url::parse(l).is_ok())
        .cloned()
        .collect();

    let removed_count = lines.len() - valid_lines.len();

    if removed_count > 0 {
        let _ = fs::write(file_path, valid_lines.join("\n"));
    }

    removed_count
}

/// Removes a specific URL from the 'links.txt' file.
///
/// Creates a temporary file, writes all lines except the specified URL,
/// then performs an atomic replacement of the original file.
///
/// # Parameters
///
/// * `url` - The URL to remove from the file
///
/// # Returns
///
/// * `Result<()>` - Ok if the URL was removed successfully, or an Error
///
/// # Errors
///
/// May return errors from file operations, such as permission issues
/// or disk space problems.
///
/// # Example
///
/// ```
/// if let Err(e) = remove_link_from_file(&url) {
///     state.add_log(format!("Error removing link: {}", e));
/// }
/// ```
pub fn remove_link_from_file(url: &str) -> Result<()> {
    let file_path = "links.txt";
    let content = fs::read_to_string(file_path).unwrap_or_default();

    // Use a temporary file for atomic writes
    let temp_path = format!("{}.tmp", file_path);
    let new_content: Vec<&str> = content
        .lines()
        .filter(|line| line.trim() != url.trim())
        .collect();

    fs::write(&temp_path, new_content.join("\n"))?;
    fs::rename(&temp_path, file_path)?; // Atomic replace

    Ok(())
}

/// Parses URLs from clipboard content and adds them to the links.txt file
/// without requiring an AppState.
///
/// Filters clipboard content for valid URLs, checks for duplicates against
/// the current links.txt file content, and saves the updated content to the file.
///
/// # Parameters
///
/// * `clipboard_content` - String content from the clipboard to parse
///
/// # Returns
///
/// * `usize` - The number of new URLs that were added
///
/// # Example
///
/// ```
/// let ctx: ClipboardContext = ClipboardProvider::new().unwrap();
/// if let Ok(contents) = ctx.get_contents() {
///     let links_added = add_clipboard_links_to_file(&contents);
///     println!("Added {} URLs", links_added);
/// }
/// ```
pub fn add_clipboard_links_to_file(clipboard_content: &str) -> usize {
    let links: Vec<String> = clipboard_content
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .filter(|l| url::Url::parse(l).is_ok())
        .collect();

    // Current file content to check for duplicates
    let current_links = get_links_from_file();
    let existing: HashSet<_> = current_links.iter().collect();

    // Filter out links that already exist
    let new_links = links
        .into_iter()
        .filter(|link| !existing.contains(link))
        .collect::<Vec<_>>();

    // If links were added, save to file
    if !new_links.is_empty() {
        let all_links = [current_links, new_links.clone()].concat();
        let _ = fs::write("links.txt", all_links.join("\n"));
    }

    new_links.len()
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
/// * `usize` - The number of new URLs that were added
///
/// # Example
///
/// ```
/// let ctx: ClipboardContext = ClipboardProvider::new().unwrap();
/// if let Ok(contents) = ctx.get_contents() {
///     let links_added = add_clipboard_links(&state, &contents);
///     state.add_log(format!("Added {} links", links_added));
/// }
/// ```
pub fn add_clipboard_links(state: &AppState, clipboard_content: &str) -> usize {
    // First add links to file
    let n = add_clipboard_links_to_file(clipboard_content);

    if n > 0 {
        // Then update app state
        let links = get_links_from_file();
        for link in &links {
            state.send(StateMessage::AddToQueue(link.clone()));
        }
    }

    n
}
