use crate::app_state::test_support::wait_for_processing;
use crate::app_state::{AppState, StateMessage};

#[test]
fn test_pop_queue_empty() {
    let state = AppState::new();
    let result = state.pop_queue().expect("failed to pop from queue");
    assert!(result.is_none());
}

#[test]
fn test_pop_queue_returns_front() {
    let state = AppState::new();
    state
        .send(StateMessage::LoadLinks(vec![
            "url1".to_string(),
            "url2".to_string(),
            "url3".to_string(),
        ]))
        .expect("failed to send LoadLinks");
    wait_for_processing();

    let first = state.pop_queue().expect("failed to pop from queue");
    assert_eq!(first, Some("url1".to_string()));

    let second = state.pop_queue().expect("failed to pop from queue");
    assert_eq!(second, Some("url2".to_string()));
}

#[test]
fn test_get_queue_returns_copy() {
    let state = AppState::new();
    state
        .send(StateMessage::LoadLinks(vec![
            "url1".to_string(),
            "url2".to_string(),
        ]))
        .expect("failed to send LoadLinks");
    wait_for_processing();

    let queue = state.get_queue().expect("failed to read queue");
    assert_eq!(queue.len(), 2);
    assert_eq!(queue[0], "url1");
    assert_eq!(queue[1], "url2");
}

#[test]
fn test_remove_from_queue_valid_index() {
    let state = AppState::new();
    state
        .send(StateMessage::LoadLinks(vec![
            "url1".to_string(),
            "url2".to_string(),
            "url3".to_string(),
        ]))
        .expect("failed to send LoadLinks");
    wait_for_processing();

    let removed = state
        .remove_from_queue(1)
        .expect("failed to remove queue item 1");
    assert_eq!(removed, Some("url2".to_string()));

    let queue = state.get_queue().expect("failed to read queue");
    assert_eq!(queue.len(), 2);
    assert_eq!(queue[0], "url1");
    assert_eq!(queue[1], "url3");
}

#[test]
fn test_remove_from_queue_invalid_index() {
    let state = AppState::new();
    state
        .send(StateMessage::LoadLinks(vec!["url1".to_string()]))
        .expect("failed to send LoadLinks");
    wait_for_processing();

    let removed = state
        .remove_from_queue(5)
        .expect("failed to remove queue item 5");
    assert!(removed.is_none());
}

#[test]
fn test_swap_queue_items_valid() {
    let state = AppState::new();
    state
        .send(StateMessage::LoadLinks(vec![
            "url1".to_string(),
            "url2".to_string(),
            "url3".to_string(),
        ]))
        .expect("failed to send LoadLinks");
    wait_for_processing();

    let success = state
        .swap_queue_items(0, 2)
        .expect("failed to swap queue items 0 and 2");
    assert!(success);

    let queue = state.get_queue().expect("failed to read queue");
    assert_eq!(queue[0], "url3");
    assert_eq!(queue[2], "url1");
}

#[test]
fn test_swap_queue_items_same_index() {
    let state = AppState::new();
    state
        .send(StateMessage::LoadLinks(vec![
            "url1".to_string(),
            "url2".to_string(),
        ]))
        .expect("failed to send LoadLinks");
    wait_for_processing();

    let success = state
        .swap_queue_items(0, 0)
        .expect("failed to swap queue items 0 and 0");
    assert!(!success);
}

#[test]
fn test_swap_queue_items_invalid_index() {
    let state = AppState::new();
    state
        .send(StateMessage::LoadLinks(vec!["url1".to_string()]))
        .expect("failed to send LoadLinks");
    wait_for_processing();

    let success = state
        .swap_queue_items(0, 5)
        .expect("failed to swap queue items 0 and 5");
    assert!(!success);
}

#[test]
fn test_refresh_all_download_timestamps() {
    let state = AppState::new();
    state
        .send(StateMessage::AddActiveDownload("url1".to_string()))
        .expect("failed to send AddActiveDownload");
    state
        .send(StateMessage::AddActiveDownload("url2".to_string()))
        .expect("failed to send AddActiveDownload");
    wait_for_processing();

    // This should not panic
    state
        .refresh_all_download_timestamps()
        .expect("failed to refresh download timestamps");
}
