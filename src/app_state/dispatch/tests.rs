use std::time::Instant;

use crate::app_state::test_support::wait_for_processing;
use crate::app_state::{AppState, DownloadProgress, StateMessage};

// ========== Active Downloads Tests ==========

#[test]
fn test_add_active_download() {
    let state = AppState::new();
    state
        .send(StateMessage::AddActiveDownload(
            "https://example.com/video".to_string(),
        ))
        .expect("failed to send AddActiveDownload");
    wait_for_processing();

    let active = state
        .get_active_downloads()
        .expect("failed to read active downloads");
    assert!(active.contains("https://example.com/video"));
}

#[test]
fn test_remove_active_download() {
    let state = AppState::new();
    state
        .send(StateMessage::AddActiveDownload(
            "https://example.com/video".to_string(),
        ))
        .expect("failed to send AddActiveDownload");
    wait_for_processing();

    state
        .send(StateMessage::RemoveActiveDownload(
            "https://example.com/video".to_string(),
        ))
        .expect("failed to send RemoveActiveDownload");
    wait_for_processing();

    let active = state
        .get_active_downloads()
        .expect("failed to read active downloads");
    assert!(!active.contains("https://example.com/video"));
}

#[test]
fn test_update_download_progress() {
    let state = AppState::new();
    let url = "https://youtube.com/watch?v=abc123".to_string();
    state
        .send(StateMessage::AddActiveDownload(url.clone()))
        .expect("failed to send AddActiveDownload");
    wait_for_processing();

    let progress = DownloadProgress {
        display_name: "Test Video".to_string(),
        phase: "downloading".to_string(),
        percent: 50.0,
        speed: Some("1.5MiB/s".to_string()),
        eta: Some("00:02:30".to_string()),
        downloaded_bytes: Some(1024 * 1024),
        total_bytes: Some(2 * 1024 * 1024),
        fragment_index: None,
        fragment_count: None,
        last_update: Instant::now(),
    };

    state
        .send(StateMessage::UpdateDownloadProgress {
            url: url.clone(),
            progress,
        })
        .expect("failed to send UpdateDownloadProgress");
    wait_for_processing();

    let snapshot = state
        .get_ui_snapshot()
        .expect("failed to build UI snapshot");
    assert_eq!(snapshot.active_downloads.len(), 1);

    let download = &snapshot.active_downloads[0];
    assert_eq!(download.display_name, "Test Video");
    assert!((download.percent - 50.0).abs() < 0.01);
}

#[test]
fn test_multiple_active_downloads() {
    let state = AppState::new();
    state
        .send(StateMessage::AddActiveDownload("url1".to_string()))
        .expect("failed to send AddActiveDownload");
    state
        .send(StateMessage::AddActiveDownload("url2".to_string()))
        .expect("failed to send AddActiveDownload");
    state
        .send(StateMessage::AddActiveDownload("url3".to_string()))
        .expect("failed to send AddActiveDownload");
    wait_for_processing();

    let active = state
        .get_active_downloads()
        .expect("failed to read active downloads");
    assert_eq!(active.len(), 3);
}

// ========== Load Links Tests ==========

#[test]
fn test_load_links_replaces_queue() {
    let state = AppState::new();
    state
        .send(StateMessage::LoadLinks(vec!["old_url".to_string()]))
        .expect("failed to send LoadLinks");
    wait_for_processing();

    state
        .send(StateMessage::LoadLinks(vec![
            "new1".to_string(),
            "new2".to_string(),
        ]))
        .expect("failed to send LoadLinks");
    wait_for_processing();

    let queue = state.get_queue().expect("failed to read queue");
    assert_eq!(queue.len(), 2);
    assert_eq!(queue[0], "new1");
    assert_eq!(queue[1], "new2");
}

#[test]
fn test_load_links_updates_stats() {
    let state = AppState::new();
    state
        .send(StateMessage::LoadLinks(vec![
            "url1".to_string(),
            "url2".to_string(),
            "url3".to_string(),
        ]))
        .expect("failed to send LoadLinks");
    wait_for_processing();

    let snapshot = state
        .get_ui_snapshot()
        .expect("failed to build UI snapshot");
    assert_eq!(snapshot.total_tasks, 3);
    assert_eq!(snapshot.initial_total_tasks, 3);
}

// ========== AddToQueue Tests ==========

#[test]
fn test_add_to_queue() {
    let state = AppState::new();
    state
        .send(StateMessage::AddToQueue("url1".to_string()))
        .expect("failed to send AddToQueue");
    state
        .send(StateMessage::AddToQueue("url2".to_string()))
        .expect("failed to send AddToQueue");
    wait_for_processing();

    let queue = state.get_queue().expect("failed to read queue");
    assert_eq!(queue.len(), 2);

    let snapshot = state
        .get_ui_snapshot()
        .expect("failed to build UI snapshot");
    assert_eq!(snapshot.initial_total_tasks, 2);
}

// ========== Failed Downloads Tests ==========

#[test]
fn test_add_failed_download() {
    let state = AppState::new();
    state
        .send(StateMessage::AddFailedDownload(
            "https://example.com/video1".to_string(),
        ))
        .expect("failed to send AddFailedDownload");
    wait_for_processing();

    let snapshot = state
        .get_ui_snapshot()
        .expect("failed to build UI snapshot");
    assert_eq!(snapshot.failed_count, 1);
}

#[test]
fn test_add_failed_download_no_duplicates() {
    let state = AppState::new();
    state
        .send(StateMessage::AddFailedDownload(
            "https://example.com/video1".to_string(),
        ))
        .expect("failed to send AddFailedDownload");
    state
        .send(StateMessage::AddFailedDownload(
            "https://example.com/video1".to_string(),
        ))
        .expect("failed to send AddFailedDownload");
    wait_for_processing();

    let snapshot = state
        .get_ui_snapshot()
        .expect("failed to build UI snapshot");
    assert_eq!(snapshot.failed_count, 1);
}

#[test]
fn test_add_multiple_failed_downloads() {
    let state = AppState::new();
    state
        .send(StateMessage::AddFailedDownload(
            "https://example.com/video1".to_string(),
        ))
        .expect("failed to send AddFailedDownload");
    state
        .send(StateMessage::AddFailedDownload(
            "https://example.com/video2".to_string(),
        ))
        .expect("failed to send AddFailedDownload");
    state
        .send(StateMessage::AddFailedDownload(
            "https://example.com/video3".to_string(),
        ))
        .expect("failed to send AddFailedDownload");
    wait_for_processing();

    let snapshot = state
        .get_ui_snapshot()
        .expect("failed to build UI snapshot");
    assert_eq!(snapshot.failed_count, 3);
}

// ========== StateMessage Variant Coverage Tests ==========

#[test]
fn test_all_state_messages_are_processed() {
    // Ensure every StateMessage variant can be sent and processed
    let state = AppState::new();

    state
        .send(StateMessage::AddToQueue("https://example.com".to_string()))
        .expect("failed to send AddToQueue");
    state
        .send(StateMessage::AddActiveDownload(
            "https://example.com".to_string(),
        ))
        .expect("failed to send AddActiveDownload");
    state
        .send(StateMessage::RemoveActiveDownload(
            "https://example.com".to_string(),
        ))
        .expect("failed to send RemoveActiveDownload");
    state
        .send(StateMessage::IncrementCompleted)
        .expect("failed to send IncrementCompleted");
    state
        .send(StateMessage::SetPaused(true))
        .expect("failed to send SetPaused(true)");
    state
        .send(StateMessage::SetStarted(true))
        .expect("failed to send SetStarted(true)");
    state
        .send(StateMessage::SetShutdown(false))
        .expect("failed to send SetShutdown(false)");
    state
        .send(StateMessage::SetForceQuit(false))
        .expect("failed to send SetForceQuit(false)");
    state
        .send(StateMessage::SetCompleted(false))
        .expect("failed to send SetCompleted(false)");
    state
        .send(StateMessage::UpdateProgress)
        .expect("failed to send UpdateProgress");
    state
        .send(StateMessage::LoadLinks(vec![
            "https://example.com".to_string(),
        ]))
        .expect("failed to send LoadLinks");
    state
        .send(StateMessage::UpdateDownloadProgress {
            url: "https://example.com".to_string(),
            progress: DownloadProgress::new("https://example.com"),
        })
        .expect("failed to send UpdateDownloadProgress");
    state
        .send(StateMessage::AddFailedDownload(
            "https://example.com".to_string(),
        ))
        .expect("failed to send AddFailedDownload");

    wait_for_processing();

    // If we get here without panics, all variants are handled
    let snapshot = state
        .get_ui_snapshot()
        .expect("failed to build UI snapshot");
    assert!(snapshot.started);
}
