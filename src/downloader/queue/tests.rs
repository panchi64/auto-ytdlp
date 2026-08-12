use super::*;
use crate::app_state::AppState;
use crate::args::Args;
use clap::Parser;

/// Helper function to create Args for testing
fn create_test_args() -> Args {
    Args::parse_from([
        "test",
        "-d",
        "/tmp/test_downloads",
        "-f",
        "/tmp/test_archive.txt",
    ])
}

// ==================== Empty Queue Handling ====================

#[test]
fn test_process_queue_empty_queue_sets_completed() {
    // Create a new app state with an empty queue
    let state = AppState::new();
    let args = create_test_args();

    // Ensure the queue is empty
    assert_eq!(state.queue_len().unwrap_or(0), 0);

    // Process the empty queue
    process_queue(state.clone(), args);

    // Wait for the message to be processed (SetCompleted is sent via channel)
    std::thread::sleep(std::time::Duration::from_millis(100));

    // The completed flag should be set to true
    assert!(state.is_completed().unwrap_or(false));
}

#[test]
fn test_process_queue_empty_queue_does_not_start() {
    // Create a new app state with an empty queue
    let state = AppState::new();
    let args = create_test_args();

    // Process the empty queue
    process_queue(state.clone(), args);

    // Wait for any message processing
    std::thread::sleep(std::time::Duration::from_millis(50));

    // The started flag should still be false (or set to false by the end)
    assert!(!state.is_started().unwrap_or(true));
}

// ==================== Concurrent Worker Count ====================

#[test]
fn test_get_concurrent_returns_expected_value() {
    let state = AppState::new();

    // Set concurrent downloads to a specific value
    let _ = state.set_concurrent(8);

    // Verify the value is set correctly
    assert_eq!(state.get_concurrent().unwrap_or(0), 8);
}

#[test]
fn test_concurrent_default_value() {
    let state = AppState::new();

    // Default concurrent should be some positive number (typically from settings)
    let concurrent = state.get_concurrent().unwrap_or(0);
    assert!(concurrent > 0);
}

// ==================== Shutdown Flag Handling ====================

#[test]
fn test_shutdown_flag_default_false() {
    let state = AppState::new();

    // By default, shutdown should be false
    assert!(!state.is_shutdown().unwrap_or(true));
}

#[test]
fn test_force_quit_flag_default_false() {
    let state = AppState::new();

    // By default, force_quit should be false
    assert!(!state.is_force_quit().unwrap_or(true));
}

#[test]
fn test_shutdown_flag_can_be_set() {
    let state = AppState::new();

    // Set shutdown flag
    let _ = state.send(StateMessage::SetShutdown(true));

    // Allow some time for the message to be processed
    std::thread::sleep(std::time::Duration::from_millis(50));

    // Verify the shutdown flag is set
    assert!(state.is_shutdown().unwrap_or(false));
}

#[test]
fn test_force_quit_flag_can_be_set() {
    let state = AppState::new();

    // Set force quit flag
    let _ = state.send(StateMessage::SetForceQuit(true));

    // Allow some time for the message to be processed
    std::thread::sleep(std::time::Duration::from_millis(50));

    // Verify the force quit flag is set
    assert!(state.is_force_quit().unwrap_or(false));
}

// ==================== Queue State ====================

#[test]
fn test_queue_can_hold_urls() {
    let state = AppState::new();

    // Add URLs to the queue
    let _ = state.send(StateMessage::LoadLinks(vec![
        "https://example.com/video1".to_string(),
        "https://example.com/video2".to_string(),
        "https://example.com/video3".to_string(),
    ]));

    // Allow some time for the message to be processed
    std::thread::sleep(std::time::Duration::from_millis(50));

    // Verify the queue has the expected number of items
    let queue = state.get_queue().unwrap_or_default();
    assert_eq!(queue.len(), 3);
}

#[test]
fn test_pop_queue_returns_url() {
    let state = AppState::new();

    // Add a URL to the queue
    let _ = state.send(StateMessage::LoadLinks(vec![
        "https://example.com/video".to_string(),
    ]));

    // Allow some time for the message to be processed
    std::thread::sleep(std::time::Duration::from_millis(50));

    // Pop from the queue
    let url = state.pop_queue();

    assert!(url.is_ok());
    assert!(url.expect("failed to pop from queue").is_some());
}

#[test]
fn test_pop_queue_empty_returns_none() {
    let state = AppState::new();

    // Ensure the queue is empty
    assert_eq!(state.queue_len().unwrap_or(0), 0);

    // Pop from empty queue should return None
    let url = state.pop_queue();

    assert!(url.is_ok());
    assert!(url.expect("failed to pop from empty queue").is_none());
}
