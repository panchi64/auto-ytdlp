use crate::app_state::test_support::wait_for_processing;
use crate::app_state::{AppState, StateMessage};

#[test]
fn test_ui_snapshot_captures_all_state() {
    let state = AppState::new();

    // Set up various state
    state
        .send(StateMessage::LoadLinks(vec![
            "url1".to_string(),
            "url2".to_string(),
        ]))
        .expect("failed to send LoadLinks");
    state
        .send(StateMessage::SetStarted(true))
        .expect("failed to send SetStarted(true)");
    state
        .send(StateMessage::SetPaused(true))
        .expect("failed to send SetPaused(true)");
    state
        .send(StateMessage::AddActiveDownload("url1".to_string()))
        .expect("failed to send AddActiveDownload");
    wait_for_processing();

    state
        .add_log("Test log".to_string())
        .expect("failed to add log");
    state
        .show_toast("Test toast")
        .expect("failed to show toast");
    state
        .set_concurrent(6)
        .expect("failed to set concurrent downloads to 6");

    let snapshot = state
        .get_ui_snapshot()
        .expect("failed to build UI snapshot");

    assert!(snapshot.started);
    assert!(snapshot.paused);
    assert!(!snapshot.completed);
    assert_eq!(snapshot.queue.len(), 2);
    assert_eq!(snapshot.active_downloads.len(), 1);
    assert!(snapshot.logs.iter().any(|l| l.contains("Test log")));
    assert_eq!(snapshot.toast, Some("Test toast".to_string()));
    assert_eq!(snapshot.concurrent, 6);
    assert_eq!(snapshot.initial_total_tasks, 2);
}

#[test]
fn test_ui_snapshot_includes_retry_count() {
    let state = AppState::new();
    state
        .increment_retries()
        .expect("failed to increment retry counter");
    state
        .increment_retries()
        .expect("failed to increment retry counter");

    let snapshot = state
        .get_ui_snapshot()
        .expect("failed to build UI snapshot");
    assert_eq!(snapshot.total_retries, 2);
}

#[test]
fn test_ui_snapshot_includes_failed_count() {
    let state = AppState::new();
    state
        .send(StateMessage::AddFailedDownload("url1".to_string()))
        .expect("failed to send AddFailedDownload");
    state
        .send(StateMessage::AddFailedDownload("url2".to_string()))
        .expect("failed to send AddFailedDownload");
    wait_for_processing();

    let snapshot = state
        .get_ui_snapshot()
        .expect("failed to build UI snapshot");
    assert_eq!(snapshot.failed_count, 2);
}
