use std::thread;
use std::time::Duration;

/// Gives the background message processor time to drain the channel.
///
/// Mutations are asynchronous, so a test that sends a `StateMessage` and reads
/// the result back must wait or it races the processor thread.
pub fn wait_for_processing() {
    thread::sleep(Duration::from_millis(50));
}
