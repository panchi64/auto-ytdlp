use crate::app_state::{AppState, StateMessage};
use anyhow::Result;
use std::{collections::HashSet, fs};

pub fn load_links(state: &AppState) -> Result<()> {
    let content = fs::read_to_string("links.txt").unwrap_or_default();
    let links: Vec<String> = content.lines().map(String::from).collect();

    state.send(StateMessage::LoadLinks(links));

    state.add_log("Links loaded from file".to_string());

    Ok(())
}

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
