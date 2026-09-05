use crossterm::event::KeyCode;

use crate::app_state::AppState;

use super::{InputResult, UiContext};

/// Handle filter mode input (search/filter queue)
pub fn handle_filter_mode_input(
    key_code: KeyCode,
    state: &AppState,
    ctx: &mut UiContext,
) -> InputResult {
    match key_code {
        KeyCode::Esc => {
            // Clear filter and exit filter mode
            ctx.filter_mode = false;
            ctx.filter_text.clear();
            ctx.filtered_indices.clear();
            ctx.queue_scroll = 0;
            InputResult::Continue
        }
        KeyCode::Enter => {
            // Exit filter mode but keep the filter active
            ctx.filter_mode = false;
            InputResult::Continue
        }
        KeyCode::Backspace => {
            ctx.filter_text.pop();
            update_filtered_indices(state, ctx);
            InputResult::Continue
        }
        KeyCode::Char(c) => {
            ctx.filter_text.push(c);
            update_filtered_indices(state, ctx);
            InputResult::Continue
        }
        _ => InputResult::Continue,
    }
}

/// Update the filtered indices based on the current filter text
fn update_filtered_indices(state: &AppState, ctx: &mut UiContext) {
    ctx.filtered_indices.clear();

    if ctx.filter_text.is_empty() {
        ctx.queue_scroll = 0;
        return;
    }

    if let Ok(queue) = state.get_queue() {
        let filter_lower = ctx.filter_text.to_lowercase();
        for (i, url) in queue.iter().enumerate() {
            if url.to_lowercase().contains(&filter_lower) {
                ctx.filtered_indices.push(i);
            }
        }
        // Bring the matches into view instead of leaving the user parked on a
        // stretch of dimmed non-matches
        ctx.scroll_to_first_match(queue.len());
    }
}
