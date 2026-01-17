use std::{thread, time::Duration};

use crate::{
    app_state::{AppState, StateMessage},
    args::Args,
};

use super::worker::download_worker;

/// Processes the download queue using multiple worker threads.
///
/// This function is the main orchestrator of the download process. It:
/// 1. Checks if the queue is empty and marks as completed if so
/// 2. Resets application state for a new download run
/// 3. Creates a controller thread to monitor the queue
/// 4. Creates worker threads only when downloads are ready to start
/// 5. Each worker thread pulls URLs from the queue and processes them
/// 6. Handles pausing, shutdown, and force quit conditions
/// 7. Waits for all worker threads to complete
/// 8. Updates application state and logs completion status
///
/// # Parameters
///
/// * `state` - The application state containing the download queue
/// * `args` - Command line arguments with download configuration
///
/// # Example
///
/// ```
/// // Start processing the download queue in a separate thread
/// let state_clone = state.clone();
/// let args_clone = args.clone();
/// thread::spawn(move || process_queue(state_clone, args_clone));
/// ```
///
/// # Notes
///
/// Each worker thread will continue running until one of these conditions is met:
/// - The queue is empty AND there are no active downloads
/// - The application is shutting down
/// - A force quit is requested
///
/// Workers will pause processing (but not exit) when the pause flag is set.
pub fn process_queue(state: AppState, args: Args) {
    if state.get_queue().unwrap_or_default().is_empty() {
        if let Err(e) = state.send(StateMessage::SetCompleted(true)) {
            eprintln!("Error setting completed: {}", e);
        }
        return;
    }

    if let Err(e) = state.reset_for_new_run() {
        eprintln!("Error resetting state: {}", e);
    }

    // Create a single controller thread instead of immediately creating all worker threads
    let state_clone = state.clone();
    let args_clone = args.clone();

    let controller = thread::spawn(move || {
        let mut worker_handles = vec![];
        let mut workers_created = false;

        loop {
            if state_clone.is_force_quit().unwrap_or(false)
                || state_clone.is_shutdown().unwrap_or(false)
            {
                // If force_quit is set, we want to exit the controller loop immediately.
                // Worker threads also check this flag and should start terminating.
                // The download_worker itself is modified to exit quickly on force_quit.
                if state_clone.is_force_quit().unwrap_or(false)
                    && let Err(e) = state_clone
                        .add_log("Controller: Force quit detected, exiting main loop.".to_string())
                {
                    eprintln!("Error adding log: {}", e);
                }
                break;
            }

            if state_clone.is_paused().unwrap_or(false) {
                thread::sleep(Duration::from_millis(100));
                continue;
            }

            // Check if we need to start processing and haven't created workers yet
            if !workers_created && !state_clone.get_queue().unwrap_or_default().is_empty() {
                // Create worker threads only when we're about to start processing
                let concurrent_count = state_clone.get_concurrent().unwrap_or(1);
                workers_created = true;

                for _ in 0..concurrent_count {
                    let worker_state = state_clone.clone();
                    let worker_args = args_clone.clone();

                    let handle = thread::spawn(move || {
                        loop {
                            if worker_state.is_force_quit().unwrap_or(false)
                                || worker_state.is_shutdown().unwrap_or(false)
                            {
                                break;
                            }

                            if worker_state.is_paused().unwrap_or(false) {
                                thread::sleep(Duration::from_millis(100));
                                continue;
                            }

                            // Get next URL from queue
                            if let Ok(Some(url)) = worker_state.pop_queue() {
                                // Wrap download_worker in catch_unwind to handle panics gracefully
                                let url_clone = url.clone();
                                let state_for_panic = worker_state.clone();
                                let result = std::panic::catch_unwind(
                                    std::panic::AssertUnwindSafe(|| {
                                        download_worker(
                                            url_clone,
                                            worker_state.clone(),
                                            worker_args.clone(),
                                        );
                                    }),
                                );

                                if result.is_err() {
                                    // Ensure cleanup on panic - remove from active downloads
                                    let _ = state_for_panic
                                        .send(StateMessage::RemoveActiveDownload(url.clone()));
                                    let _ = state_for_panic.log_error(
                                        "Worker panic",
                                        format!("Worker panicked while downloading {}, recovered", url),
                                    );
                                }
                            } else {
                                thread::sleep(Duration::from_millis(100));

                                if worker_state.get_queue().unwrap_or_default().is_empty()
                                    && worker_state
                                        .get_active_downloads()
                                        .unwrap_or_default()
                                        .is_empty()
                                {
                                    // Only break if we're truly done and not just between tasks
                                    break;
                                }
                            }
                        }
                    });
                    worker_handles.push(handle);
                }
            }

            // Check if we're done
            if workers_created
                && state_clone.get_queue().unwrap_or_default().is_empty()
                && state_clone
                    .get_active_downloads()
                    .unwrap_or_default()
                    .is_empty()
            {
                break;
            }

            thread::sleep(Duration::from_millis(100));
        }

        // After controller loop exits (due to completion, shutdown, or force_quit)

        if state_clone.is_force_quit().unwrap_or(false) {
            if let Err(e) = state_clone.add_log(
                "Controller: Force quit active. Not waiting for worker threads to join."
                    .to_string(),
            ) {
                eprintln!("Error adding log: {}", e);
            }
            // Worker threads are expected to terminate themselves upon detecting is_force_quit().
            // The download_worker function is also modified to not block on cmd.wait() during a force quit.
            // Thus, we don't join worker_handles here to ensure a fast exit.
        } else {
            // If not a force quit (i.e., normal completion or graceful shutdown), wait for workers.
            if let Err(e) = state_clone
                .add_log("Controller: Waiting for worker threads to complete.".to_string())
            {
                eprintln!("Error adding log: {}", e);
            }
            for handle in worker_handles {
                if let Err(e) = handle.join()
                    && let Err(log_err) =
                        state_clone.add_log(format!("Controller: Worker thread panicked: {:?}", e))
                {
                    eprintln!("Error adding log: {}", log_err);
                }
            }
            if let Err(e) =
                state_clone.add_log("Controller: All worker threads completed.".to_string())
            {
                eprintln!("Error adding log: {}", e);
            }
        }

        let queue_empty = state_clone.get_queue().unwrap_or_default().is_empty();
        let active_downloads_empty = state_clone
            .get_active_downloads()
            .unwrap_or_default()
            .is_empty();

        // Update final status based on whether it was a force quit or not
        if state_clone.is_force_quit().unwrap_or(false) {
            if let Err(e) =
                state_clone.add_log("Download processing forcefully stopped.".to_string())
            {
                eprintln!("Error adding log: {}", e);
            }
            // Do not set SetCompleted(true) on force quit, even if queue became empty by chance.
            // The state should reflect an interruption.
        } else if queue_empty && active_downloads_empty {
            if let Err(e) = state_clone.send(StateMessage::SetCompleted(true)) {
                eprintln!("Error setting completed: {}", e);
            }
            if let Err(e) =
                state_clone.add_log("All downloads completed or queue is empty.".to_string())
            {
                eprintln!("Error adding log: {}", e);
            }
        } else {
            // This case covers normal stop (shutdown flag) where queue might not be empty.
            if let Err(e) = state_clone.add_log("Download processing stopped.".to_string()) {
                eprintln!("Error adding log: {}", e);
            }
        }

        if let Err(e) = state_clone.send(StateMessage::SetStarted(false)) {
            eprintln!("Error setting started: {}", e);
        } // Always mark as not started

        // Clear logs after a short delay, but only if not a force quit.
        // For force quit, we want to preserve the logs detailing the forceful termination.
        let mut log_clear_handle: Option<thread::JoinHandle<()>> = None;

        if !state_clone.is_force_quit().unwrap_or(false) {
            let final_state_clone = state_clone.clone();
            log_clear_handle = Some(thread::spawn(move || {
                thread::sleep(Duration::from_secs(2));
                // Check again in case state changed, though unlikely for a detached thread task like this.
                if !final_state_clone.is_completed().unwrap_or(false)
                    && !final_state_clone.is_shutdown().unwrap_or(false)
                {
                    // If not completed and not a normal shutdown, maybe don't clear logs?
                    // For now, let's stick to original logic: clear logs if not force_quit.
                    // The original logic was to clear logs anyway after a delay.
                }
                if let Err(e) =
                    final_state_clone.add_log("Clearing logs after completion/stop.".to_string())
                {
                    eprintln!("Error adding log: {}", e);
                } // Log before clear
                if let Err(e) = final_state_clone.clear_logs() {
                    eprintln!("Error clearing logs: {}", e);
                }
            }));
        }

        if let Some(handle) = log_clear_handle
            && let Err(e) = handle.join()
            && let Err(log_err) = state_clone.add_log(format!(
                "Log clearing thread panicked: {:?}. Logs may not be cleared.",
                e
            ))
        {
            eprintln!("Error adding log: {}", log_err);
        }
    });

    // This join is for the controller thread itself.
    // If force_quit is true, the controller thread should now exit quickly because it
    // doesn't .join() its own worker_handles.
    if let Err(e) = controller.join() {
        // Log controller panic, this might be important especially in --auto mode.
        eprintln!("Controller thread panicked: {:?}", e);
    }
}
