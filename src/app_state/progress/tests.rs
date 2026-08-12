use crate::app_state::DownloadProgress;

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
