use super::DownloadProgress;

/// Messages used to update the application state.
///
/// This enum defines all possible state changes that can be applied to the
/// application state through the message passing system.
///
/// Settings are updated directly via `AppState::update_settings()` rather
/// than through message passing.
pub enum StateMessage {
    /// Adds a URL to the download queue.
    AddToQueue(String),

    /// Marks a URL as actively downloading.
    AddActiveDownload(String),

    /// Removes a URL from the active downloads.
    RemoveActiveDownload(String),

    /// Increments the completed downloads counter.
    IncrementCompleted,

    /// Sets the paused state.
    SetPaused(bool),

    /// Sets the started state.
    SetStarted(bool),

    /// Sets the shutdown state.
    SetShutdown(bool),

    /// Sets the force quit state.
    SetForceQuit(bool),

    /// Sets the completed state.
    SetCompleted(bool),

    /// Triggers a progress update calculation.
    UpdateProgress,

    /// Replaces the entire download queue with the provided list.
    LoadLinks(Vec<String>),

    /// Updates progress information for an active download.
    UpdateDownloadProgress {
        url: String,
        progress: DownloadProgress,
    },

    /// Records a URL that failed to download (for retry).
    AddFailedDownload(String),
}
