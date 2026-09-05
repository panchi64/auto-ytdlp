use ratatui::{
    Frame,
    layout::{Margin, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem, Scrollbar, ScrollbarOrientation, ScrollbarState},
};

use crate::app_state::UiSnapshot;
use crate::ui::tui::{UiContext, flip_index};

use super::super::style::flash_style;

/// Builds the title for the pending queue panel, which doubles as the mode
/// indicator for edit and filter modes.
fn pending_queue_title(snapshot: &UiSnapshot, ctx: &UiContext) -> String {
    let queue_len = snapshot.queue.len();
    let initial_total = snapshot.initial_total_tasks;
    let use_ascii = snapshot.use_ascii_indicators;

    if ctx.queue_edit_mode {
        let icon = if use_ascii { "[EDIT]" } else { "📝" };
        format!(
            "{} Edit Queue - {}/{} (K/J: Move | D: Delete | Esc: Exit)",
            icon, queue_len, initial_total
        )
    } else if ctx.filter_mode || !ctx.filter_text.is_empty() {
        let match_count = ctx.filtered_indices.len();
        if use_ascii {
            format!(
                "[FILTER: {}] {}/{} matches",
                ctx.filter_text, match_count, queue_len
            )
        } else {
            format!(
                "🔍 [{}] {}/{} matches",
                ctx.filter_text, match_count, queue_len
            )
        }
    } else {
        let icon = if use_ascii {
            if queue_len == 0 { "[OK]" } else { "[Q]" }
        } else if queue_len == 0 {
            "✅"
        } else {
            "📋"
        };
        format!(
            "{} Pending Downloads - {}/{}",
            icon, queue_len, initial_total
        )
    }
}

/// Renders the pending queue panel.
///
/// Rows are drawn newest first, so links added while the user is watching land at
/// the top of the panel, and only the slice starting at `ctx.queue_scroll` is drawn.
/// The underlying queue keeps its FIFO order - workers still pop the oldest link.
pub(in crate::ui::tui::render) fn render_pending_queue(
    frame: &mut Frame,
    area: Rect,
    snapshot: &UiSnapshot,
    ctx: &mut UiContext,
) {
    let queue = &snapshot.queue;

    let border_style = if ctx.filter_mode {
        Style::default().fg(Color::Cyan)
    } else if ctx.queue_edit_mode {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };

    let block = Block::default()
        .title(pending_queue_title(snapshot, ctx))
        .borders(Borders::ALL)
        .border_style(border_style);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Record what was drawn so the input handlers can hit-test the wheel, page
    // against the viewport, and map rows using the same length the user saw
    ctx.queue_area = area;
    ctx.queue_view_height = inner.height as usize;
    ctx.queue_drawn_len = queue.len();
    ctx.scroll_queue_by(0, queue.len());
    let scroll = ctx.queue_scroll;
    let view_height = ctx.queue_view_height;

    let has_filter = !ctx.filter_text.is_empty();
    let visible_items: Vec<ListItem> = (scroll..queue.len())
        .take(view_height)
        .filter_map(|display_index| {
            let queue_index = flip_index(display_index, queue.len())?;
            let url = &queue[queue_index];
            let is_filtered_out = has_filter && !ctx.filtered_indices.contains(&queue_index);

            // A link the filter excludes stays dimmed even while it is flashing,
            // so the panel never reads as if a non-match were a match
            let style = if ctx.queue_edit_mode && display_index == ctx.queue_selected_index {
                Style::default().fg(Color::Yellow).bg(Color::DarkGray)
            } else if is_filtered_out {
                Style::default().fg(Color::DarkGray)
            } else if let Some(progress) = ctx.flash_progress(url) {
                flash_style(progress, snapshot.use_ascii_indicators)
            } else if has_filter {
                Style::default().fg(Color::Green)
            } else {
                Style::default()
            };

            Some(ListItem::new(url.as_str()).style(style))
        })
        .collect();

    frame.render_widget(List::new(visible_items), inner);

    if queue.len() > view_height {
        render_queue_scrollbar(frame, area, scroll, queue.len(), view_height);
    }
}

/// Draws a scroll thumb over the pending panel's right border.
fn render_queue_scrollbar(
    frame: &mut Frame,
    area: Rect,
    scroll: usize,
    queue_len: usize,
    view_height: usize,
) {
    // Content length is the number of scroll positions, so a fully scrolled list
    // puts the thumb flush against the bottom of the track.
    let mut scrollbar_state = ScrollbarState::new(queue_len - view_height + 1)
        .position(scroll)
        .viewport_content_length(view_height);

    frame.render_stateful_widget(
        Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .track_symbol(None)
            .begin_symbol(None)
            .end_symbol(None)
            .thumb_symbol("┃")
            .thumb_style(Style::default().fg(Color::Gray)),
        area.inner(Margin {
            vertical: 1,
            horizontal: 0,
        }),
        &mut scrollbar_state,
    );
}

#[cfg(test)]
mod tests;
