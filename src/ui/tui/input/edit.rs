use crossterm::event::KeyCode;

use crate::app_state::AppState;
use crate::ui::tui::flip_index;

use super::actions::downloads_running;
use super::{InputResult, UiContext};

/// Handle queue edit mode input
///
/// `ctx.queue_selected_index` is a display row, so index 0 is the newest link at
/// the top of the panel. Every call into `AppState` flips it back to queue order.
pub fn handle_edit_mode_input(
    key_code: KeyCode,
    state: &AppState,
    ctx: &mut UiContext,
) -> InputResult {
    // Map rows using the length the panel was drawn from, not the live one: links
    // can land in the queue between the frame and this keypress, and because new
    // links go to the back, the drawn length still points every visible row at the
    // URL the user is looking at.
    let queue_len = ctx.queue_drawn_len;
    let page = ctx.page_size();
    let selected = flip_index(ctx.queue_selected_index, queue_len);

    match key_code {
        KeyCode::Up => {
            ctx.queue_selected_index = ctx.queue_selected_index.saturating_sub(1);
        }
        KeyCode::Down => {
            if queue_len > 0 && ctx.queue_selected_index < queue_len - 1 {
                ctx.queue_selected_index += 1;
            }
        }
        KeyCode::PageUp => {
            ctx.queue_selected_index = ctx.queue_selected_index.saturating_sub(page);
        }
        KeyCode::PageDown => {
            ctx.queue_selected_index =
                (ctx.queue_selected_index + page).min(queue_len.saturating_sub(1));
        }
        KeyCode::Home => {
            ctx.queue_selected_index = 0;
        }
        KeyCode::End => {
            ctx.queue_selected_index = queue_len.saturating_sub(1);
        }
        KeyCode::Char('k') | KeyCode::Char('K') => {
            // Move item up the panel, which is one step later in the queue
            if ctx.queue_selected_index > 0
                && let Some(index) = selected
                && let Ok(true) = state.swap_queue_items(index, index + 1)
            {
                ctx.queue_selected_index -= 1;
            }
        }
        KeyCode::Char('j') | KeyCode::Char('J') => {
            // Move item down the panel, which is one step earlier in the queue
            if let Some(index) = selected
                && index > 0
                && let Ok(true) = state.swap_queue_items(index, index - 1)
            {
                ctx.queue_selected_index += 1;
            }
        }
        KeyCode::Char('d') | KeyCode::Char('D') | KeyCode::Delete => {
            if let Some(index) = selected
                && let Ok(Some(removed)) = state.remove_from_queue(index)
            {
                // Show toast notification for removal
                let _ = state.show_toast("URL removed from queue");
                if let Err(e) = state.add_log(format!("Removed from queue: {}", removed)) {
                    eprintln!("Error adding log: {}", e);
                }
                // Adjust selected index if necessary
                let new_len = queue_len - 1;
                if new_len == 0 {
                    ctx.queue_edit_mode = false;
                } else if ctx.queue_selected_index >= new_len {
                    ctx.queue_selected_index = new_len - 1;
                }
            }
        }
        KeyCode::Esc | KeyCode::Enter | KeyCode::Char('e') => {
            ctx.queue_edit_mode = false;
        }
        _ => {}
    }

    ctx.scroll_selection_into_view();
    InputResult::Continue
}

pub(in crate::ui::tui::input) fn handle_edit_mode(state: &AppState, ctx: &mut UiContext) {
    if !downloads_running(state) {
        let queue_len = state.get_queue().map(|q| q.len()).unwrap_or(0);
        if queue_len > 0 {
            ctx.queue_edit_mode = true;
            // Start on the newest link, at the top of the panel
            ctx.queue_selected_index = 0;
            ctx.queue_scroll = 0;
            if let Err(e) = state.add_log(
                "Queue edit mode: ↑↓ Navigate | K/J: Move | D: Delete | Esc: Exit".to_string(),
            ) {
                eprintln!("Error adding log: {}", e);
            }
        } else if let Err(e) = state.add_log("No URLs in queue to edit".to_string()) {
            eprintln!("Error adding log: {}", e);
        }
    } else if let Err(e) = state.add_log("Cannot edit queue while downloads are active".to_string())
    {
        eprintln!("Error adding log: {}", e);
    }
}
