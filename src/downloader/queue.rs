use std::{thread, time::Duration};

use crate::{app_state::AppState, args::Args};

use super::worker::download_worker;

pub fn process_queue(state: AppState, args: Args) {
    if state.queue.lock().unwrap().is_empty() {
        *state.completed.lock().unwrap() = true;
        return;
    }

    // Initialize total tasks with current queue length
    let queue_len = state.queue.lock().unwrap().len();
    *state.total_tasks.lock().unwrap() = queue_len;
    *state.completed_tasks.lock().unwrap() = 0; // Reset completed count

    let mut handles = vec![];

    // Create worker threads
    for _ in 0..args.concurrent {
        let state_clone = state.clone();
        let args_clone = args.clone();

        let handle = thread::spawn(move || {
            loop {
                // Check exit conditions first
                if *state_clone.force_quit.lock().unwrap() || *state_clone.shutdown.lock().unwrap()
                {
                    break;
                }

                // Handle pause state
                if *state_clone.paused.lock().unwrap() {
                    thread::sleep(Duration::from_millis(100));
                    continue;
                }

                // Get next URL (atomic operation)
                let url = state_clone.queue.lock().unwrap().pop_front();

                if let Some(url) = url {
                    download_worker(url, state_clone.clone(), args_clone.clone());
                } else {
                    // Wait for new items or shutdown
                    thread::sleep(Duration::from_millis(100));
                }
            }
        });
        handles.push(handle);
    }

    // Wait for all workers to finish
    for handle in handles {
        handle.join().unwrap();
    }

    // Mark completion if queue is empty
    let completed = state.queue.lock().unwrap().is_empty();
    *state.completed.lock().unwrap() = completed;
    *state.started.lock().unwrap() = false;
}
