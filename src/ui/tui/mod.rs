mod context;
mod input;
mod render;
mod run;

use std::{
    hash::{DefaultHasher, Hash, Hasher},
    time::Duration,
};

pub use context::UiContext;
pub use render::ui;
pub use run::run_tui;

/// How long a newly queued link stays highlighted in the pending list.
pub const FLASH_DURATION: Duration = Duration::from_millis(1200);

/// Maps a row between queue order (oldest first, the order workers pop from) and
/// display order (newest first, so fresh links appear at the top of the panel).
///
/// The mapping is its own inverse. Returns `None` when the row does not exist.
pub(crate) fn flip_index(index: usize, queue_len: usize) -> Option<usize> {
    queue_len.checked_sub(1)?.checked_sub(index)
}

/// Identifies a URL for flash tracking without owning a copy of it.
///
/// The flash is purely decorative, so a hash collision would at worst tint the
/// wrong row for a moment - cheap enough to run over the whole queue every frame.
fn url_key(url: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    url.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests;
