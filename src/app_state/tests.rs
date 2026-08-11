
use super::*;
use std::thread;
use std::time::Duration;

// Helper to wait for message processing
fn wait_for_processing() {
    thread::sleep(Duration::from_millis(50));
}

// ========== Initialization Tests ==========

#[test]
fn test_new_creates_default_state() {
    let state = AppState::new();

    // Verify default flags
    assert!(!state.is_paused().expect("failed to read paused flag"));
    assert!(!state.is_started().expect("failed to read started flag"));
    assert!(!state.is_shutdown().expect("failed to read shutdown flag"));
    assert!(
        !state
            .is_force_quit()
            .expect("failed to read force-quit flag")
    );
    assert!(!state.is_completed().expect("failed to read completed flag"));
}

#[test]
fn test_new_creates_empty_queue() {
    let state = AppState::new();
    let queue = state.get_queue().expect("failed to read queue");
    assert!(queue.is_empty());
}

#[test]
fn test_new_has_welcome_logs() {
    let state = AppState::new();
    let snapshot = state
        .get_ui_snapshot()
        .expect("failed to build UI snapshot");

    assert!(snapshot.logs.len() >= 2);
    assert!(snapshot.logs[0].contains("Welcome"));
    assert!(snapshot.logs[1].contains("quit"));
}

#[test]
fn test_new_loads_settings() {
    let state = AppState::new();
    let settings = state.get_settings().expect("failed to read settings");
    // Settings should load (either from file or default)
    assert!(settings.concurrent_downloads > 0);
}

// ========== Queue Operations Tests ==========

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

// ========== Flag Mutations Tests ==========

#[test]
fn test_set_paused() {
    let state = AppState::new();
    state
        .send(StateMessage::SetPaused(true))
        .expect("failed to send SetPaused(true)");
    wait_for_processing();
    assert!(state.is_paused().expect("failed to read paused flag"));

    state
        .send(StateMessage::SetPaused(false))
        .expect("failed to send SetPaused(false)");
    wait_for_processing();
    assert!(!state.is_paused().expect("failed to read paused flag"));
}

#[test]
fn test_set_started() {
    let state = AppState::new();
    state
        .send(StateMessage::SetStarted(true))
        .expect("failed to send SetStarted(true)");
    wait_for_processing();
    assert!(state.is_started().expect("failed to read started flag"));

    state
        .send(StateMessage::SetStarted(false))
        .expect("failed to send SetStarted(false)");
    wait_for_processing();
    assert!(!state.is_started().expect("failed to read started flag"));
}

#[test]
fn test_set_shutdown() {
    let state = AppState::new();
    state
        .send(StateMessage::SetShutdown(true))
        .expect("failed to send SetShutdown(true)");
    wait_for_processing();
    assert!(state.is_shutdown().expect("failed to read shutdown flag"));
}

#[test]
fn test_set_force_quit() {
    let state = AppState::new();
    state
        .send(StateMessage::SetForceQuit(true))
        .expect("failed to send SetForceQuit(true)");
    wait_for_processing();
    assert!(
        state
            .is_force_quit()
            .expect("failed to read force-quit flag")
    );
}

#[test]
fn test_set_completed() {
    let state = AppState::new();
    state
        .send(StateMessage::SetCompleted(true))
        .expect("failed to send SetCompleted(true)");
    wait_for_processing();
    assert!(state.is_completed().expect("failed to read completed flag"));
}

// ========== Progress Tracking Tests ==========

#[test]
fn test_increment_completed() {
    let state = AppState::new();
    state
        .send(StateMessage::LoadLinks(vec![
            "url1".to_string(),
            "url2".to_string(),
        ]))
        .expect("failed to send LoadLinks");
    state
        .send(StateMessage::SetStarted(true))
        .expect("failed to send SetStarted(true)");
    wait_for_processing();

    state
        .send(StateMessage::IncrementCompleted)
        .expect("failed to send IncrementCompleted");
    wait_for_processing();

    let snapshot = state
        .get_ui_snapshot()
        .expect("failed to build UI snapshot");
    assert_eq!(snapshot.completed_tasks, 1);
    assert!((snapshot.progress - 0.5).abs() < 0.01);
}

#[test]
fn test_update_progress_calculation() {
    let state = AppState::new();
    state
        .send(StateMessage::LoadLinks(vec![
            "url1".to_string(),
            "url2".to_string(),
            "url3".to_string(),
            "url4".to_string(),
        ]))
        .expect("failed to send LoadLinks");
    state
        .send(StateMessage::SetStarted(true))
        .expect("failed to send SetStarted(true)");
    wait_for_processing();

    // Complete 2 out of 4 tasks
    state
        .send(StateMessage::IncrementCompleted)
        .expect("failed to send IncrementCompleted");
    state
        .send(StateMessage::IncrementCompleted)
        .expect("failed to send IncrementCompleted");
    wait_for_processing();

    let snapshot = state
        .get_ui_snapshot()
        .expect("failed to build UI snapshot");
    assert!((snapshot.progress - 0.5).abs() < 0.01);
}

#[test]
fn test_auto_completion_detection() {
    let state = AppState::new();
    state
        .send(StateMessage::LoadLinks(vec![
            "url1".to_string(),
            "url2".to_string(),
        ]))
        .expect("failed to send LoadLinks");
    state
        .send(StateMessage::SetStarted(true))
        .expect("failed to send SetStarted(true)");
    wait_for_processing();

    // Complete all tasks
    state
        .send(StateMessage::IncrementCompleted)
        .expect("failed to send IncrementCompleted");
    state
        .send(StateMessage::IncrementCompleted)
        .expect("failed to send IncrementCompleted");
    wait_for_processing();

    // Give extra time for auto-completion to be detected
    thread::sleep(Duration::from_millis(100));

    let snapshot = state
        .get_ui_snapshot()
        .expect("failed to build UI snapshot");
    assert!(snapshot.completed);
    assert!((snapshot.progress - 1.0).abs() < 0.01);
}

#[test]
fn test_progress_zero_when_no_tasks() {
    let state = AppState::new();
    state
        .send(StateMessage::UpdateProgress)
        .expect("failed to send UpdateProgress");
    wait_for_processing();

    let snapshot = state
        .get_ui_snapshot()
        .expect("failed to build UI snapshot");
    assert!((snapshot.progress - 0.0).abs() < 0.01);
}

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

// ========== Log Management Tests ==========

#[test]
fn test_add_log() {
    let state = AppState::new();
    state
        .add_log("Test log message".to_string())
        .expect("failed to add log");

    let snapshot = state
        .get_ui_snapshot()
        .expect("failed to build UI snapshot");
    assert!(
        snapshot
            .logs
            .iter()
            .any(|log| log.contains("Test log message"))
    );
}

#[test]
fn test_log_limit_1000() {
    let state = AppState::new();

    // Add 1100 logs
    for i in 0..1100 {
        state
            .add_log(format!("Log message {}", i))
            .expect("failed to add log");
    }

    let snapshot = state
        .get_ui_snapshot()
        .expect("failed to build UI snapshot");
    // Should have at most 1000 logs
    assert!(snapshot.logs.len() <= 1000);

    // Oldest logs should be removed (logs 0-99 and welcome messages gone)
    assert!(
        !snapshot
            .logs
            .iter()
            .any(|log| log.contains("Log message 0"))
    );

    // Recent logs should still be there
    assert!(
        snapshot
            .logs
            .iter()
            .any(|log| log.contains("Log message 1099"))
    );
}

#[test]
fn test_clear_logs() {
    let state = AppState::new();
    state
        .add_log("Test log 1".to_string())
        .expect("failed to add log");
    state
        .add_log("Test log 2".to_string())
        .expect("failed to add log");
    state.clear_logs().expect("failed to clear logs");

    let snapshot = state
        .get_ui_snapshot()
        .expect("failed to build UI snapshot");
    assert_eq!(snapshot.logs.len(), 1);
    assert!(snapshot.logs[0].contains("Logs cleared"));
}

#[test]
fn test_log_error() {
    let state = AppState::new();
    state
        .log_error("Download", "Connection timeout")
        .expect("failed to log error");

    let snapshot = state
        .get_ui_snapshot()
        .expect("failed to build UI snapshot");
    assert!(snapshot.logs.iter().any(|log| {
        log.contains("[ERROR]") && log.contains("Download") && log.contains("Connection timeout")
    }));
}

// ========== Toast Notification Tests ==========

#[test]
fn test_show_toast() {
    let state = AppState::new();
    state
        .show_toast("Test notification")
        .expect("failed to show toast");

    let snapshot = state
        .get_ui_snapshot()
        .expect("failed to build UI snapshot");
    assert_eq!(snapshot.toast, Some("Test notification".to_string()));
}

#[test]
fn test_show_toast_accepts_string() {
    let state = AppState::new();
    state
        .show_toast(String::from("String toast"))
        .expect("failed to show toast");

    let snapshot = state
        .get_ui_snapshot()
        .expect("failed to build UI snapshot");
    assert_eq!(snapshot.toast, Some("String toast".to_string()));
}

#[test]
fn test_clear_toast() {
    let state = AppState::new();
    state
        .show_toast("Test notification")
        .expect("failed to show toast");
    state.clear_toast().expect("failed to clear toast");

    let snapshot = state
        .get_ui_snapshot()
        .expect("failed to build UI snapshot");
    assert!(snapshot.toast.is_none());
}

#[test]
fn test_toast_auto_expiry() {
    let state = AppState::new();

    // Directly set toast with old timestamp
    {
        let mut toast = state.toast.lock().expect("toast mutex was poisoned");
        *toast = Some((
            "Old toast".to_string(),
            Instant::now() - Duration::from_secs(5),
        ));
    }

    // get_ui_snapshot should clear expired toast
    let snapshot = state
        .get_ui_snapshot()
        .expect("failed to build UI snapshot");
    assert!(snapshot.toast.is_none());
}

// ========== Settings Tests ==========

#[test]
fn test_get_settings() {
    let state = AppState::new();
    let settings = state.get_settings().expect("failed to read settings");
    // Should have valid default settings
    assert!(settings.concurrent_downloads > 0);
}

#[test]
fn test_update_settings() {
    let state = AppState::new();
    let mut new_settings = state.get_settings().expect("failed to read settings");
    new_settings.concurrent_downloads = 8;
    new_settings.write_subtitles = true;

    state
        .update_settings(new_settings)
        .expect("failed to update settings");

    let updated = state.get_settings().expect("failed to read settings");
    assert_eq!(updated.concurrent_downloads, 8);
    assert!(updated.write_subtitles);
}

// ========== UI Snapshot Tests ==========

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

// ========== Concurrent Access Tests ==========

#[test]
fn test_concurrent_get_settings() {
    let state = AppState::new();
    let handles: Vec<_> = (0..10)
        .map(|_| {
            let state_clone = state.clone();
            thread::spawn(move || {
                for _ in 0..100 {
                    let _ = state_clone.get_settings().expect("failed to read settings");
                }
            })
        })
        .collect();

    for handle in handles {
        handle.join().expect("spawned test thread panicked");
    }
}

#[test]
fn test_concurrent_flag_access() {
    let state = AppState::new();
    let handles: Vec<_> = (0..5)
        .map(|i| {
            let state_clone = state.clone();
            thread::spawn(move || {
                for _ in 0..50 {
                    state_clone
                        .send(StateMessage::SetPaused(i % 2 == 0))
                        .expect("failed to send SetPaused");
                    let _ = state_clone.is_paused().expect("failed to read paused flag");
                }
            })
        })
        .collect();

    for handle in handles {
        handle.join().expect("spawned test thread panicked");
    }
}

#[test]
fn test_concurrent_queue_access() {
    let state = AppState::new();
    state
        .send(StateMessage::LoadLinks(vec![
            "url1".to_string(),
            "url2".to_string(),
            "url3".to_string(),
            "url4".to_string(),
            "url5".to_string(),
        ]))
        .expect("failed to send LoadLinks");
    wait_for_processing();

    let handles: Vec<_> = (0..3)
        .map(|_| {
            let state_clone = state.clone();
            thread::spawn(move || {
                let _ = state_clone.pop_queue().expect("failed to pop from queue");
                let _ = state_clone.get_queue().expect("failed to read queue");
            })
        })
        .collect();

    for handle in handles {
        handle.join().expect("spawned test thread panicked");
    }
}

#[test]
fn test_concurrent_log_writes() {
    let state = AppState::new();
    let handles: Vec<_> = (0..5)
        .map(|i| {
            let state_clone = state.clone();
            thread::spawn(move || {
                for j in 0..20 {
                    state_clone
                        .add_log(format!("Thread {} log {}", i, j))
                        .expect("failed to add log");
                }
            })
        })
        .collect();

    for handle in handles {
        handle.join().expect("spawned test thread panicked");
    }

    let snapshot = state
        .get_ui_snapshot()
        .expect("failed to build UI snapshot");
    // All 100 logs should be present (5 threads x 20 logs + 2 welcome logs)
    assert!(snapshot.logs.len() >= 100);
}

// ========== Reset and File Lock Tests ==========

#[test]
fn test_reset_for_new_run() {
    let state = AppState::new();

    // Set up various state
    state
        .send(StateMessage::SetStarted(true))
        .expect("failed to send SetStarted(true)");
    state
        .send(StateMessage::SetPaused(true))
        .expect("failed to send SetPaused(true)");
    state
        .send(StateMessage::SetCompleted(true))
        .expect("failed to send SetCompleted(true)");
    wait_for_processing();

    state
        .increment_retries()
        .expect("failed to increment retry counter");
    state.show_toast("Test").expect("failed to show toast");

    state
        .reset_for_new_run()
        .expect("failed to reset state for a new run");

    assert!(!state.is_paused().expect("failed to read paused flag"));
    assert!(!state.is_started().expect("failed to read started flag"));
    assert!(!state.is_completed().expect("failed to read completed flag"));
    assert!(!state.is_shutdown().expect("failed to read shutdown flag"));
    assert!(
        !state
            .is_force_quit()
            .expect("failed to read force-quit flag")
    );

    let snapshot = state
        .get_ui_snapshot()
        .expect("failed to build UI snapshot");
    assert!(snapshot.toast.is_none());
    assert_eq!(snapshot.total_retries, 0);
}

#[test]
fn test_acquire_file_lock() {
    let state = AppState::new();
    let _lock = state
        .acquire_file_lock()
        .expect("failed to acquire file lock");
    // Lock is acquired, will be released when _lock drops
}

#[test]
fn test_set_concurrent() {
    let state = AppState::new();
    state
        .set_concurrent(12)
        .expect("failed to set concurrent downloads to 12");
    assert_eq!(
        state
            .get_concurrent()
            .expect("failed to read concurrent-downloads setting"),
        12
    );
}

#[test]
fn test_increment_and_reset_retries() {
    let state = AppState::new();
    state
        .increment_retries()
        .expect("failed to increment retry counter");
    state
        .increment_retries()
        .expect("failed to increment retry counter");
    state
        .increment_retries()
        .expect("failed to increment retry counter");

    let snapshot = state
        .get_ui_snapshot()
        .expect("failed to build UI snapshot");
    assert_eq!(snapshot.total_retries, 3);

    state
        .reset_retries()
        .expect("failed to reset retry counter");

    let snapshot = state
        .get_ui_snapshot()
        .expect("failed to build UI snapshot");
    assert_eq!(snapshot.total_retries, 0);
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

// ========== DownloadProgress Tests ==========

#[test]
fn test_download_progress_default() {
    let progress = DownloadProgress::default();
    assert!(progress.display_name.is_empty());
    assert_eq!(progress.phase, "downloading");
    assert!((progress.percent - 0.0).abs() < 0.01);
    assert!(progress.speed.is_none());
    assert!(progress.eta.is_none());
}

#[test]
fn test_download_progress_new_youtube() {
    let progress = DownloadProgress::new("https://www.youtube.com/watch?v=dQw4w9WgXcQ");
    assert!(progress.display_name.contains("dQw4w9WgXcQ"));
}

#[test]
fn test_download_progress_new_other_url() {
    let progress = DownloadProgress::new("https://example.com/video.mp4");
    assert!(progress.display_name.contains("video.mp4"));
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

// ========== Notification Flag Tests ==========

#[test]
fn test_notification_not_sent_initially() {
    let state = AppState::new();
    assert!(
        !state
            .is_notification_sent()
            .expect("failed to read notification-sent flag")
    );
}

#[test]
fn test_notification_can_be_marked_as_sent() {
    let state = AppState::new();
    state
        .set_notification_sent(true)
        .expect("failed to set notification-sent flag");
    assert!(
        state
            .is_notification_sent()
            .expect("failed to read notification-sent flag")
    );
}

#[test]
fn test_notification_flag_resets_on_new_run() {
    let state = AppState::new();

    // Mark notification as sent
    state
        .set_notification_sent(true)
        .expect("failed to set notification-sent flag");
    assert!(
        state
            .is_notification_sent()
            .expect("failed to read notification-sent flag")
    );

    // Reset for new run
    state
        .reset_for_new_run()
        .expect("failed to reset state for a new run");

    // Notification flag should be reset
    assert!(
        !state
            .is_notification_sent()
            .expect("failed to read notification-sent flag")
    );
}

#[test]
fn test_notification_lifecycle_across_multiple_runs() {
    let state = AppState::new();

    // First run: notification starts unsent
    assert!(
        !state
            .is_notification_sent()
            .expect("failed to read notification-sent flag")
    );

    // Simulate completion notification being sent
    state
        .set_notification_sent(true)
        .expect("failed to set notification-sent flag");
    assert!(
        state
            .is_notification_sent()
            .expect("failed to read notification-sent flag")
    );

    // Subsequent checks should still show sent (idempotent)
    assert!(
        state
            .is_notification_sent()
            .expect("failed to read notification-sent flag")
    );

    // User restarts downloads
    state
        .reset_for_new_run()
        .expect("failed to reset state for a new run");

    // Second run: notification should be unsent again
    assert!(
        !state
            .is_notification_sent()
            .expect("failed to read notification-sent flag")
    );

    // Second completion
    state
        .set_notification_sent(true)
        .expect("failed to set notification-sent flag");
    assert!(
        state
            .is_notification_sent()
            .expect("failed to read notification-sent flag")
    );
}

#[test]
fn test_notification_flag_independent_of_completion_state() {
    let state = AppState::new();

    // Set completed without setting notification
    state
        .send(StateMessage::SetCompleted(true))
        .expect("failed to send SetCompleted(true)");
    wait_for_processing();

    // Notification flag should still be false
    assert!(state.is_completed().expect("failed to read completed flag"));
    assert!(
        !state
            .is_notification_sent()
            .expect("failed to read notification-sent flag")
    );

    // Set notification independently
    state
        .set_notification_sent(true)
        .expect("failed to set notification-sent flag");
    assert!(
        state
            .is_notification_sent()
            .expect("failed to read notification-sent flag")
    );
}

// ========== Reset Stats on New Batch Tests ==========

#[test]
fn test_new_batch_resets_counters_by_default() {
    let state = AppState::new();

    // Explicitly enable per-session mode (settings file on disk may differ)
    let mut settings = state.get_settings().expect("failed to read settings");
    settings.reset_stats_on_new_batch = true;
    state
        .update_settings(settings)
        .expect("failed to update settings");

    // Load initial batch and complete some downloads
    state
        .send(StateMessage::LoadLinks(vec![
            "url1".to_string(),
            "url2".to_string(),
            "url3".to_string(),
        ]))
        .expect("failed to send LoadLinks");
    state
        .send(StateMessage::SetStarted(true))
        .expect("failed to send SetStarted(true)");
    state
        .send(StateMessage::IncrementCompleted)
        .expect("failed to send IncrementCompleted");
    state
        .send(StateMessage::IncrementCompleted)
        .expect("failed to send IncrementCompleted");
    // Simulate workers draining the queue
    state
        .send(StateMessage::LoadLinks(vec![]))
        .expect("failed to send LoadLinks(empty)");
    wait_for_processing();

    let snapshot = state
        .get_ui_snapshot()
        .expect("failed to build UI snapshot");
    assert_eq!(snapshot.completed_tasks, 2);

    // Start a new batch (simulates pressing 'S' again with empty queue)
    state
        .reset_for_new_run()
        .expect("failed to reset state for a new run");

    // With default setting (reset_stats_on_new_batch = true),
    // counters should be reset based on current queue size (0)
    let snapshot = state
        .get_ui_snapshot()
        .expect("failed to build UI snapshot");
    assert_eq!(snapshot.total_tasks, 0);
    assert_eq!(snapshot.initial_total_tasks, 0);
    assert_eq!(snapshot.completed_tasks, 0);
}

#[test]
fn test_new_batch_preserves_counters_when_cumulative_mode_enabled() {
    let state = AppState::new();

    // Disable reset (enable cumulative mode)
    let mut settings = state.get_settings().expect("failed to read settings");
    settings.reset_stats_on_new_batch = false;
    state
        .update_settings(settings)
        .expect("failed to update settings");

    // Load initial batch and complete some downloads
    state
        .send(StateMessage::LoadLinks(vec![
            "url1".to_string(),
            "url2".to_string(),
            "url3".to_string(),
        ]))
        .expect("failed to send LoadLinks");
    state
        .send(StateMessage::SetStarted(true))
        .expect("failed to send SetStarted(true)");
    state
        .send(StateMessage::IncrementCompleted)
        .expect("failed to send IncrementCompleted");
    state
        .send(StateMessage::IncrementCompleted)
        .expect("failed to send IncrementCompleted");
    wait_for_processing();

    let snapshot = state
        .get_ui_snapshot()
        .expect("failed to build UI snapshot");
    assert_eq!(snapshot.total_tasks, 3);
    assert_eq!(snapshot.initial_total_tasks, 3);
    assert_eq!(snapshot.completed_tasks, 2);

    // Start a new batch
    state
        .reset_for_new_run()
        .expect("failed to reset state for a new run");

    // With cumulative mode, total_tasks and initial_total_tasks should be preserved
    let snapshot = state
        .get_ui_snapshot()
        .expect("failed to build UI snapshot");
    assert_eq!(snapshot.total_tasks, 3);
    assert_eq!(snapshot.initial_total_tasks, 3);
    // completed_tasks carries over with the totals, so progress stays coherent
    assert_eq!(snapshot.completed_tasks, 2);
}

#[test]
fn test_cumulative_mode_accumulates_across_batches() {
    let state = AppState::new();

    // Enable cumulative mode
    let mut settings = state.get_settings().expect("failed to read settings");
    settings.reset_stats_on_new_batch = false;
    state
        .update_settings(settings)
        .expect("failed to update settings");

    // First batch: 3 URLs
    state
        .send(StateMessage::LoadLinks(vec![
            "url1".to_string(),
            "url2".to_string(),
            "url3".to_string(),
        ]))
        .expect("failed to send LoadLinks");
    state
        .send(StateMessage::SetStarted(true))
        .expect("failed to send SetStarted(true)");
    state
        .send(StateMessage::IncrementCompleted)
        .expect("failed to send IncrementCompleted");
    state
        .send(StateMessage::IncrementCompleted)
        .expect("failed to send IncrementCompleted");
    state
        .send(StateMessage::IncrementCompleted)
        .expect("failed to send IncrementCompleted");
    wait_for_processing();

    // All 3 completed
    let snapshot = state
        .get_ui_snapshot()
        .expect("failed to build UI snapshot");
    assert_eq!(snapshot.initial_total_tasks, 3);
    assert_eq!(snapshot.completed_tasks, 3);

    // Reset for second batch
    state
        .reset_for_new_run()
        .expect("failed to reset state for a new run");

    // Add more URLs to queue (simulating user adding new links)
    state
        .send(StateMessage::AddToQueue("url4".to_string()))
        .expect("failed to send AddToQueue");
    state
        .send(StateMessage::AddToQueue("url5".to_string()))
        .expect("failed to send AddToQueue");
    wait_for_processing();

    // In cumulative mode: initial_total_tasks preserved (3) + new additions (2) = 5
    let snapshot = state
        .get_ui_snapshot()
        .expect("failed to build UI snapshot");
    assert_eq!(snapshot.initial_total_tasks, 5);
    assert_eq!(snapshot.total_tasks, 5);
    // completed_tasks carries over too, so finishing url4/url5 reaches 5/5
    assert_eq!(snapshot.completed_tasks, 3);
}

#[test]
fn test_per_session_mode_fresh_count_each_batch() {
    let state = AppState::new();

    // Explicitly enable per-session mode (settings file on disk may differ)
    let mut settings = state.get_settings().expect("failed to read settings");
    settings.reset_stats_on_new_batch = true;
    state
        .update_settings(settings)
        .expect("failed to update settings");

    // First batch: 3 URLs, complete all
    state
        .send(StateMessage::LoadLinks(vec![
            "url1".to_string(),
            "url2".to_string(),
            "url3".to_string(),
        ]))
        .expect("failed to send LoadLinks");
    state
        .send(StateMessage::SetStarted(true))
        .expect("failed to send SetStarted(true)");
    state
        .send(StateMessage::IncrementCompleted)
        .expect("failed to send IncrementCompleted");
    state
        .send(StateMessage::IncrementCompleted)
        .expect("failed to send IncrementCompleted");
    state
        .send(StateMessage::IncrementCompleted)
        .expect("failed to send IncrementCompleted");
    // Simulate workers draining the queue
    state
        .send(StateMessage::LoadLinks(vec![]))
        .expect("failed to send LoadLinks(empty)");
    wait_for_processing();

    let snapshot = state
        .get_ui_snapshot()
        .expect("failed to build UI snapshot");
    assert_eq!(snapshot.completed_tasks, 3);

    // Reset for second batch (queue is empty)
    state
        .reset_for_new_run()
        .expect("failed to reset state for a new run");

    // Counters should be zeroed (queue is empty)
    let snapshot = state
        .get_ui_snapshot()
        .expect("failed to build UI snapshot");
    assert_eq!(snapshot.initial_total_tasks, 0);
    assert_eq!(snapshot.total_tasks, 0);
    assert_eq!(snapshot.completed_tasks, 0);

    // Add new URLs
    state
        .send(StateMessage::AddToQueue("url4".to_string()))
        .expect("failed to send AddToQueue");
    state
        .send(StateMessage::AddToQueue("url5".to_string()))
        .expect("failed to send AddToQueue");
    wait_for_processing();

    // Fresh count: only the 2 new URLs
    let snapshot = state
        .get_ui_snapshot()
        .expect("failed to build UI snapshot");
    assert_eq!(snapshot.initial_total_tasks, 2);
    assert_eq!(snapshot.total_tasks, 2);
}

#[test]
fn test_switching_modes_mid_session() {
    let state = AppState::new();

    // Start in per-session mode (default)
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
        .send(StateMessage::IncrementCompleted)
        .expect("failed to send IncrementCompleted");
    wait_for_processing();

    // Switch to cumulative mode
    let mut settings = state.get_settings().expect("failed to read settings");
    settings.reset_stats_on_new_batch = false;
    state
        .update_settings(settings)
        .expect("failed to update settings");

    // Reset - should preserve counters in cumulative mode
    state
        .reset_for_new_run()
        .expect("failed to reset state for a new run");

    let snapshot = state
        .get_ui_snapshot()
        .expect("failed to build UI snapshot");
    assert_eq!(snapshot.initial_total_tasks, 2);
    assert_eq!(snapshot.total_tasks, 2);

    // Switch back to per-session mode
    let mut settings = state.get_settings().expect("failed to read settings");
    settings.reset_stats_on_new_batch = true;
    state
        .update_settings(settings)
        .expect("failed to update settings");

    // Reset - should reset to current queue size (2 items still in queue)
    state
        .reset_for_new_run()
        .expect("failed to reset state for a new run");

    let snapshot = state
        .get_ui_snapshot()
        .expect("failed to build UI snapshot");
    assert_eq!(snapshot.initial_total_tasks, 2);
    assert_eq!(snapshot.total_tasks, 2);
}

// ========== StateMessage Variant Coverage Tests ==========

// ========== Failed Downloads Tests ==========

#[test]
fn test_failed_downloads_initially_empty() {
    let state = AppState::new();
    let snapshot = state
        .get_ui_snapshot()
        .expect("failed to build UI snapshot");
    assert_eq!(snapshot.failed_count, 0);
}

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

#[test]
fn test_take_failed_downloads_drains() {
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
    wait_for_processing();

    let failed = state
        .take_failed_downloads()
        .expect("failed to drain failed downloads");
    assert_eq!(failed.len(), 2);
    assert!(failed.contains(&"https://example.com/video1".to_string()));
    assert!(failed.contains(&"https://example.com/video2".to_string()));

    // Should be empty after take
    let snapshot = state
        .get_ui_snapshot()
        .expect("failed to build UI snapshot");
    assert_eq!(snapshot.failed_count, 0);
}

#[test]
fn test_take_failed_downloads_empty() {
    let state = AppState::new();
    let failed = state
        .take_failed_downloads()
        .expect("failed to drain failed downloads");
    assert!(failed.is_empty());
}

#[test]
fn test_reset_for_new_run_clears_failed_downloads() {
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

    state
        .reset_for_new_run()
        .expect("failed to reset state for a new run");

    let snapshot = state
        .get_ui_snapshot()
        .expect("failed to build UI snapshot");
    assert_eq!(snapshot.failed_count, 0);
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
