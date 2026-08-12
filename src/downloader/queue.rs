use std::{thread, time::Duration};

use crate::{
    app_state::{AppState, StateMessage},
    args::Args,
};

use super::worker::download_worker;

/// Returns whether the queue is empty, treating a lock failure as "not empty".
///
/// A failed read must never be mistaken for "nothing left to do", which would
/// make the controller declare the batch complete while URLs are still queued.
fn queue_is_empty(state: &AppState) -> bool {
    state.get_queue().map(|q| q.is_empty()).unwrap_or(false)
}

/// Returns whether there are no active downloads, treating a lock failure as
/// "downloads still active" for the same reason as [`queue_is_empty`].
fn active_downloads_empty(state: &AppState) -> bool {
    state
        .get_active_downloads()
        .map(|a| a.is_empty())
        .unwrap_or(false)
}

/// Spawns a controller thread that keeps N workers draining the download queue.
///
/// Workers exit once the queue is empty and nothing is downloading, or on shutdown
/// or force quit. The pause flag makes them sleep rather than exit.
pub fn process_queue(state: AppState, args: Args) {
    match state.get_queue() {
        Ok(queue) => {
            if queue.is_empty() {
                if let Err(e) = state.send(StateMessage::SetCompleted(true)) {
                    eprintln!("Error setting completed: {}", e);
                }
                return;
            }
        }
        Err(e) => {
            // A lock failure is not an empty queue: bail out without claiming the
            // batch finished, so queued URLs are not silently dropped.
            if let Err(log_err) = state.log_error("Queue read failed", format!("{}", e)) {
                eprintln!("Error adding log: {}", log_err);
            }
            return;
        }
    }

    if let Err(e) = state.reset_for_new_run() {
        eprintln!("Error resetting state: {}", e);
    }

    // Mark the run as started only after the reset, which clears the flag.
    if let Err(e) = state.send(StateMessage::SetStarted(true)) {
        eprintln!("Error setting started: {}", e);
    }

    // Create a single controller thread instead of immediately creating all worker threads
    let state_clone = state.clone();
    let args_clone = args.clone();

    let controller = thread::spawn(move || {
        let mut worker_handles: Vec<thread::JoinHandle<()>> = vec![];
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

            // Spawn workers when there is queued work and none are alive. Workers
            // exit once the queue drains, so URLs added later (clipboard, retry)
            // need a fresh batch rather than a one-shot "created" latch.
            let workers_alive = worker_handles.iter().any(|handle| !handle.is_finished());
            if !workers_alive && !queue_is_empty(&state_clone) {
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
                                let result =
                                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                                        download_worker(
                                            url_clone,
                                            worker_state.clone(),
                                            worker_args.clone(),
                                        );
                                    }));

                                if result.is_err() {
                                    // Ensure cleanup on panic - remove from active downloads
                                    let _ = state_for_panic
                                        .send(StateMessage::RemoveActiveDownload(url.clone()));
                                    let _ = state_for_panic.log_error(
                                        "Worker panic",
                                        format!(
                                            "Worker panicked while downloading {}, recovered",
                                            url
                                        ),
                                    );
                                }
                            } else {
                                thread::sleep(Duration::from_millis(100));

                                if queue_is_empty(&worker_state)
                                    && active_downloads_empty(&worker_state)
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
                && queue_is_empty(&state_clone)
                && active_downloads_empty(&state_clone)
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

        let queue_empty = queue_is_empty(&state_clone);
        let downloads_drained = active_downloads_empty(&state_clone);

        // Update final status based on whether it was a force quit or not
        if state_clone.is_force_quit().unwrap_or(false) {
            if let Err(e) =
                state_clone.add_log("Download processing forcefully stopped.".to_string())
            {
                eprintln!("Error adding log: {}", e);
            }
            // Do not set SetCompleted(true) on force quit, even if queue became empty by chance.
            // The state should reflect an interruption.
        } else if queue_empty && downloads_drained {
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
        if !state_clone.is_force_quit().unwrap_or(false) {
            let final_state_clone = state_clone.clone();
            thread::spawn(move || {
                thread::sleep(Duration::from_secs(2));
                if let Err(e) = final_state_clone.clear_logs() {
                    eprintln!("Error clearing logs: {}", e);
                }
            });
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

#[cfg(test)]
mod tests;
