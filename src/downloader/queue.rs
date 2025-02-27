use std::{thread, time::Duration};

use crate::{
    app_state::{AppState, StateMessage},
    args::Args,
};

use super::worker::download_worker;

pub fn process_queue(state: AppState, args: Args) {
    if state.get_queue().is_empty() {
        state.send(StateMessage::SetCompleted(true));
        return;
    }

    state.reset_for_new_run();

    let mut handles = vec![];
    let concurrent_count = state.get_concurrent();

    // Create worker threads
    for _ in 0..concurrent_count {
        let state_clone = state.clone();
        let args_clone = args.clone();

        let handle = thread::spawn(move || {
            loop {
                if state_clone.is_force_quit() || state_clone.is_shutdown() {
                    break;
                }

                if state_clone.is_paused() {
                    thread::sleep(Duration::from_millis(100));
                    continue;
                }

                // Get next URL from queue
                if let Some(url) = state_clone.pop_queue() {
                    download_worker(url, state_clone.clone(), args_clone.clone());
                } else {
                    thread::sleep(Duration::from_millis(100));

                    if state_clone.get_queue().is_empty()
                        && state_clone.get_active_downloads().is_empty()
                    {
                        // Only break if we're truly done and not just between tasks
                        break;
                    }
                }
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let queue_empty = state.get_queue().is_empty();
    state.send(StateMessage::SetCompleted(queue_empty));
    state.send(StateMessage::SetStarted(false));

    if queue_empty {
        state.add_log("All downloads completed".to_string());
    } else {
        state.add_log("Download processing stopped".to_string());
    }
}
