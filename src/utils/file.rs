use crate::app_state::{AppState, StateMessage};
use anyhow::Result;
use std::{collections::HashSet, fs};

/// Loads URLs from the 'links.txt' file into the application state.
///
/// Reads all lines from the links.txt file and updates the application state's
/// download queue with these URLs. Creates an empty file if it doesn't exist.
///
/// # Parameters
///
/// * `state` - Reference to the application state to update
///
/// # Returns
///
/// * `Result<()>` - Ok if links were loaded successfully, or an Error
///
/// # Errors
///
/// This function handles file not found by using an empty default string,
/// but may return errors from other file operations.
///
/// # Example
///
/// ```
/// if let Err(e) = load_links(&state) {
///     state.add_log(format!("Error loading links: {}", e));
/// }
/// ```
pub fn load_links(state: &AppState) -> Result<()> {
    let content = fs::read_to_string("links.txt").unwrap_or_default();
    let links: Vec<String> = content.lines().map(String::from).collect();

    state.send(StateMessage::LoadLinks(links));

    state.add_log("Links loaded from file".to_string());

    Ok(())
}

/// Saves the current download queue to the 'links.txt' file.
///
/// Writes all unique URLs from the current queue to the links.txt file,
/// removing any duplicates in the process.
///
/// # Parameters
///
/// * `state` - Reference to the application state containing the queue to save
///
/// # Returns
///
/// * `Result<()>` - Ok if links were saved successfully, or an Error
///
/// # Errors
///
/// May return errors from file operations, such as permission issues
/// or disk space problems.
///
/// # Example
///
/// ```
/// if let Err(e) = save_links(&state) {
///     state.add_log(format!("Error saving links: {}", e));
/// }
/// ```
pub fn save_links(state: &AppState) -> Result<()> {
    let queue = state.get_queue();

    // Filter out duplicates
    let mut seen = HashSet::new();
    let unique_links: Vec<_> = queue
        .iter()
        .filter_map(|link| {
            let trimmed = link.trim().to_string();
            seen.insert(trimmed.clone()).then_some(trimmed)
        })
        .collect();

    fs::write("links.txt", unique_links.join("\n"))?;

    state.add_log("Links saved to file".to_string());

    Ok(())
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

/// Parses URLs from clipboard content and adds them to the download queue.
///
/// Filters clipboard content for valid URLs, checks for duplicates against
/// the current queue, adds new URLs to the queue, and saves the updated
/// queue to the links.txt file.
///
/// # Parameters
///
/// * `state` - Reference to the application state to update
/// * `clipboard_content` - String content from the clipboard to parse
///
/// # Returns
///
/// * `usize` - The number of new URLs that were added to the queue
///
/// # Example
///
/// ```
/// let ctx: ClipboardContext = ClipboardProvider::new().unwrap();
/// if let Ok(contents) = ctx.get_contents() {
///     let links_added = add_links_from_clipboard(&state, &contents);
///     state.add_log(format!("Added {} URLs", links_added));
/// }
/// ```
pub fn add_links_from_clipboard(state: &AppState, clipboard_content: &str) -> usize {
    let links: Vec<String> = clipboard_content
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .filter(|l| url::Url::parse(l).is_ok())
        .collect();

    // Current queue to check for duplicates
    let current_queue = state.get_queue();
    let existing: HashSet<_> = current_queue.iter().collect();

    // Filter out links that already exist in the queue
    let new_links = links
        .into_iter()
        .filter(|link| !existing.contains(link))
        .collect::<Vec<_>>();

    for link in &new_links {
        state.send(StateMessage::AddToQueue(link.clone()));
    }

    // If links were added, save to file
    if !new_links.is_empty() {
        let _ = save_links(state);
    }

    new_links.len()
}
