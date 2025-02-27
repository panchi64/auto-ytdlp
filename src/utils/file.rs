use crate::app_state::AppState;
use anyhow::Result;
use std::{collections::HashSet, fs};

pub fn load_links(state: &AppState) -> Result<()> {
    let content = fs::read_to_string("links.txt").unwrap_or_default();
    let mut queue = state.queue.lock().unwrap();
    *queue = content.lines().map(String::from).collect();
    *state.initial_total_tasks.lock().unwrap() = queue.len();
    *state.total_tasks.lock().unwrap() = queue.len();
    Ok(())
}

pub fn save_links(state: &AppState) -> Result<()> {
    let queue = state.queue.lock().unwrap();
    let mut seen = HashSet::new();
    let unique_links: Vec<_> = queue
        .iter()
        .filter_map(|link| {
            let trimmed = link.trim().to_string();
            seen.insert(trimmed.clone()).then_some(trimmed)
        })
        .collect();
    fs::write("links.txt", unique_links.join("\n"))?;
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
