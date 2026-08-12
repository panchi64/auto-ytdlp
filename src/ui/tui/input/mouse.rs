use crossterm::event::{MouseEvent, MouseEventKind};

use crate::app_state::AppState;

use super::{UiContext, WHEEL_SCROLL_ROWS};

/// Scrolls the pending list by `delta` rows against the live queue length.
pub(in crate::ui::tui::input) fn scroll_queue(state: &AppState, ctx: &mut UiContext, delta: isize) {
    ctx.scroll_queue_by(delta, state.queue_len().unwrap_or(0));
}

/// Handle mouse input - the wheel scrolls the pending queue list.
///
/// Only wheel events over the pending panel count, so scrolling elsewhere does not
/// move a list the pointer is nowhere near. In edit mode the wheel moves the
/// selection rather than the viewport, otherwise the highlighted row could scroll
/// out of sight and `D` would delete a link the user cannot see.
pub fn handle_mouse_input(mouse: MouseEvent, state: &AppState, ctx: &mut UiContext) {
    let delta = match mouse.kind {
        MouseEventKind::ScrollUp => -WHEEL_SCROLL_ROWS,
        MouseEventKind::ScrollDown => WHEEL_SCROLL_ROWS,
        _ => return,
    };

    if !ctx
        .queue_area
        .contains(ratatui::layout::Position::new(mouse.column, mouse.row))
    {
        return;
    }

    if ctx.queue_edit_mode {
        move_selection_by(ctx, delta);
    } else {
        scroll_queue(state, ctx, delta);
    }
}

/// Moves the edit mode selection by `delta` rows, clamped to the drawn queue, and
/// brings it back into view.
fn move_selection_by(ctx: &mut UiContext, delta: isize) {
    let last_row = ctx.queue_drawn_len.saturating_sub(1);
    ctx.queue_selected_index = ctx
        .queue_selected_index
        .saturating_add_signed(delta)
        .min(last_row);
    ctx.scroll_selection_into_view();
}
