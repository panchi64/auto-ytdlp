use std::{
    collections::{HashMap, HashSet, VecDeque},
    time::Instant,
};

use super::{FLASH_DURATION, flip_index, url_key};

/// UI context for additional rendering state not captured in UiSnapshot
#[derive(Default)]
pub struct UiContext {
    pub queue_edit_mode: bool,
    /// Highlighted row in *display* order, so 0 is the newest link
    pub queue_selected_index: usize,
    pub show_help: bool,
    /// Filter mode for queue search
    pub filter_mode: bool,
    /// Current filter text
    pub filter_text: String,
    /// Indices of queue items that match the filter, in queue order
    pub filtered_indices: Vec<usize>,
    /// First display row visible in the pending list
    pub queue_scroll: usize,
    /// Rows that fit inside the pending list, recorded by the last render
    pub queue_view_height: usize,
    /// Queue length the pending list was last drawn from
    pub queue_drawn_len: usize,
    /// Screen rect the pending list was last drawn into, for mouse hit-testing
    pub queue_area: ratatui::layout::Rect,
    /// Queue contents as of the previous frame; `None` until the first frame.
    /// Only `track_new_links` and `flash_progress` should touch this pair - the
    /// flash invariant leaks if anything else writes them.
    pub(super) queue_seen: Option<HashSet<u64>>,
    /// When each recently added URL showed up, driving the highlight flash
    pub(super) queue_flash: HashMap<u64, Instant>,
}

impl UiContext {
    /// Flags links that appeared in the queue since the last frame so the pending
    /// list can flash them, and forgets flashes that expired or left the queue.
    ///
    /// New links enter at the back of the FIFO, which is the *top* of the panel, so
    /// they push everything below them down a row. A user who has scrolled away from
    /// the top is reading specific links, so the view shifts with them rather than
    /// letting the rows slide; at the top the new links stay visible, which is the
    /// point of showing them there.
    ///
    /// The first frame only records a baseline: links restored from `links.txt` at
    /// startup are not new from the user's point of view.
    pub fn track_new_links(&mut self, queue: &VecDeque<String>) {
        let current: HashSet<u64> = queue.iter().map(|url| url_key(url)).collect();
        let now = Instant::now();

        let mut added = 0usize;
        if let Some(previous) = &self.queue_seen {
            for key in current.difference(previous) {
                self.queue_flash.insert(*key, now);
                added += 1;
            }
        }

        if added > 0 {
            if self.queue_edit_mode {
                // Stay on the link the user selected rather than on the row number
                self.queue_selected_index += added;
                self.queue_scroll += added;
            } else if self.queue_scroll > 0 {
                self.queue_scroll += added;
            }
        }

        self.queue_flash
            .retain(|key, at| current.contains(key) && now.duration_since(*at) < FLASH_DURATION);
        self.queue_seen = Some(current);
    }

    /// How far a link is through its flash, from 0.0 (just added) to 1.0, or
    /// `None` once it is no longer flashing.
    pub fn flash_progress(&self, url: &str) -> Option<f32> {
        let elapsed = self.queue_flash.get(&url_key(url))?.elapsed().as_secs_f32();
        let duration = FLASH_DURATION.as_secs_f32();
        (elapsed < duration).then(|| elapsed / duration)
    }

    /// Rows a PageUp/PageDown moves, always at least one.
    pub fn page_size(&self) -> usize {
        self.queue_view_height.max(1)
    }

    /// Largest scroll offset that still keeps the pending list viewport filled.
    pub fn max_queue_scroll(&self, queue_len: usize) -> usize {
        queue_len.saturating_sub(self.queue_view_height)
    }

    /// Scrolls the pending list by `delta` rows, clamped to the current queue.
    pub fn scroll_queue_by(&mut self, delta: isize, queue_len: usize) {
        self.queue_scroll = self
            .queue_scroll
            .saturating_add_signed(delta)
            .min(self.max_queue_scroll(queue_len));
    }

    /// Scrolls to the display row holding the newest filter match.
    ///
    /// The filter dims non-matches rather than hiding them, so without this a
    /// filter applied while scrolled would leave every match off-screen.
    pub fn scroll_to_first_match(&mut self, queue_len: usize) {
        // `filtered_indices` is in queue order, so its last entry is the newest
        // match, which is the topmost matching row on screen
        let Some(newest_match) = self.filtered_indices.last() else {
            self.queue_scroll = 0;
            return;
        };
        let row = flip_index(*newest_match, queue_len).unwrap_or(0);
        self.queue_scroll = row.min(self.max_queue_scroll(queue_len));
    }

    /// Keeps the edit mode selection inside the visible rows.
    pub fn scroll_selection_into_view(&mut self) {
        let height = self.page_size();
        if self.queue_selected_index < self.queue_scroll {
            self.queue_scroll = self.queue_selected_index;
        } else if self.queue_selected_index >= self.queue_scroll + height {
            self.queue_scroll = self.queue_selected_index + 1 - height;
        }
    }
}

#[cfg(test)]
mod tests;
