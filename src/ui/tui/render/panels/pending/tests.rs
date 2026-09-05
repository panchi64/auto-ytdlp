use super::*;

use ratatui::{Terminal, backend::TestBackend, buffer::Buffer};

use crate::ui::tui::render::style::FLASH_SHADES;

fn snapshot_with_queue(urls: &[&str]) -> UiSnapshot {
    UiSnapshot {
        progress: 0.0,
        completed_tasks: 0,
        total_tasks: urls.len(),
        initial_total_tasks: urls.len(),
        started: false,
        paused: false,
        completed: false,
        queue: urls.iter().map(|u| u.to_string()).collect(),
        active_downloads: Vec::new(),
        logs: Vec::new(),
        concurrent: 1,
        toast: None,
        use_ascii_indicators: false,
        total_retries: 0,
        failed_count: 0,
    }
}

/// Draws just the pending panel into an off-screen buffer.
fn draw_pending_queue(snapshot: &UiSnapshot, ctx: &mut UiContext, height: u16) -> Buffer {
    let mut terminal =
        Terminal::new(TestBackend::new(24, height)).expect("failed to build test terminal");
    terminal
        .draw(|frame| render_pending_queue(frame, frame.area(), snapshot, ctx))
        .expect("failed to draw pending queue");
    terminal.backend().buffer().clone()
}

/// Text of a rendered row, excluding the panel borders.
fn row_text(buffer: &Buffer, row: u16) -> String {
    (1..buffer.area.width - 1)
        .map(|x| buffer[(x, row + 1)].symbol())
        .collect::<String>()
        .trim_end()
        .to_string()
}

// ========== Pending Queue Panel Tests ==========

#[test]
fn test_pending_queue_lists_newest_link_first() {
    let snapshot = snapshot_with_queue(&["url1", "url2", "url3"]);
    let mut ctx = UiContext::default();

    // 5 rows tall leaves 3 rows inside the borders
    let buffer = draw_pending_queue(&snapshot, &mut ctx, 5);

    assert_eq!(row_text(&buffer, 0), "url3");
    assert_eq!(row_text(&buffer, 1), "url2");
    assert_eq!(row_text(&buffer, 2), "url1");
}

#[test]
fn test_pending_queue_shows_only_the_scrolled_window() {
    let snapshot = snapshot_with_queue(&["url1", "url2", "url3", "url4", "url5"]);
    let mut ctx = UiContext {
        queue_scroll: 2,
        ..Default::default()
    };

    let buffer = draw_pending_queue(&snapshot, &mut ctx, 4);

    // Display order is url5..url1, so scrolling two rows starts at url3
    assert_eq!(ctx.queue_view_height, 2);
    assert_eq!(row_text(&buffer, 0), "url3");
    assert_eq!(row_text(&buffer, 1), "url2");
}

#[test]
fn test_pending_queue_clamps_scroll_past_the_end() {
    let snapshot = snapshot_with_queue(&["url1", "url2", "url3", "url4"]);
    let mut ctx = UiContext {
        queue_scroll: 99,
        ..Default::default()
    };

    let buffer = draw_pending_queue(&snapshot, &mut ctx, 4);

    // 4 links with 2 visible rows bottoms out showing the two oldest
    assert_eq!(ctx.queue_scroll, 2);
    assert_eq!(row_text(&buffer, 0), "url2");
    assert_eq!(row_text(&buffer, 1), "url1");
}

#[test]
fn test_pending_queue_scrollbar_tracks_position() {
    let snapshot = snapshot_with_queue(&["url1", "url2", "url3", "url4", "url5"]);
    let mut ctx = UiContext::default();

    let buffer = draw_pending_queue(&snapshot, &mut ctx, 4);
    let right_edge = buffer.area.width - 1;
    assert_eq!(buffer[(right_edge, 1)].symbol(), "┃");

    // Scrolled to the oldest link, the thumb sits at the bottom of the track
    ctx.queue_scroll = 3;
    let buffer = draw_pending_queue(&snapshot, &mut ctx, 4);
    assert_eq!(buffer[(right_edge, 2)].symbol(), "┃");
}

#[test]
fn test_pending_queue_hides_scrollbar_when_everything_fits() {
    let snapshot = snapshot_with_queue(&["url1", "url2"]);
    let mut ctx = UiContext::default();

    let buffer = draw_pending_queue(&snapshot, &mut ctx, 4);
    let right_edge = buffer.area.width - 1;

    assert_ne!(buffer[(right_edge, 1)].symbol(), "┃");
}

#[test]
fn test_pending_queue_keeps_a_flashing_non_match_dimmed() {
    // A newly added link that the filter excludes must not outshine the
    // matches, or the panel reads as if it were one of them.
    let mut ctx = UiContext {
        filter_text: "keep".to_string(),
        ..Default::default()
    };
    ctx.track_new_links(&snapshot_with_queue(&["keep-1"]).queue);

    let snapshot = snapshot_with_queue(&["keep-1", "drop-1"]);
    ctx.track_new_links(&snapshot.queue);
    // Only "keep-1" matches, at queue index 0
    ctx.filtered_indices = vec![0];

    let buffer = draw_pending_queue(&snapshot, &mut ctx, 4);

    assert_eq!(row_text(&buffer, 0), "drop-1");
    assert_eq!(buffer[(1, 1)].bg, Color::Reset);
    assert_eq!(buffer[(1, 1)].fg, Color::DarkGray);
    // The matching link still renders as a match
    assert_eq!(buffer[(1, 2)].fg, Color::Green);
}

#[test]
fn test_pending_queue_flashes_a_new_link_that_matches_the_filter() {
    let mut ctx = UiContext {
        filter_text: "keep".to_string(),
        ..Default::default()
    };
    ctx.track_new_links(&snapshot_with_queue(&["keep-1"]).queue);

    let snapshot = snapshot_with_queue(&["keep-1", "keep-2"]);
    ctx.track_new_links(&snapshot.queue);
    ctx.filtered_indices = vec![0, 1];

    let buffer = draw_pending_queue(&snapshot, &mut ctx, 4);

    assert_eq!(row_text(&buffer, 0), "keep-2");
    assert_eq!(buffer[(1, 1)].bg, FLASH_SHADES[0]);
}

#[test]
fn test_pending_queue_flashes_only_the_new_link() {
    let mut ctx = UiContext::default();
    ctx.track_new_links(&snapshot_with_queue(&["url1"]).queue);

    let snapshot = snapshot_with_queue(&["url1", "url2"]);
    ctx.track_new_links(&snapshot.queue);

    let buffer = draw_pending_queue(&snapshot, &mut ctx, 4);

    // url2 is newest, so it sits on the top row wearing the flash
    assert_eq!(row_text(&buffer, 0), "url2");
    assert_eq!(buffer[(1, 1)].bg, FLASH_SHADES[0]);
    assert_eq!(buffer[(1, 2)].bg, Color::Reset);
}
