use super::edit::handle_edit_mode;
use super::*;
use crate::app_state::{AppState, StateMessage};
use crate::args::Args;
use clap::Parser;
use crossterm::event::{MouseEvent, MouseEventKind};
use std::thread;
use std::time::Duration;

// Helper to create AppState for testing
fn create_test_state() -> AppState {
    AppState::new()
}

// Helper to create UiContext for testing
fn create_test_context() -> UiContext {
    UiContext::default()
}

// Helper to create Args for testing
fn create_test_args() -> Args {
    Args::parse_from(["test"])
}

/// Owns everything a `NormalModeContext` borrows, so a test declares one
/// value instead of five locals plus a five-argument constructor call, and
/// can inspect the same fields afterwards.
struct TestNmc {
    ctx: UiContext,
    download_state: DownloadState,
    force_quit_state: ForceQuitState,
    last_tick: Instant,
    tick_rate: Duration,
}

impl Default for TestNmc {
    fn default() -> Self {
        Self {
            ctx: create_test_context(),
            download_state: DownloadState::default(),
            force_quit_state: ForceQuitState::default(),
            last_tick: Instant::now(),
            tick_rate: Duration::from_millis(100),
        }
    }
}

impl TestNmc {
    fn context(&mut self) -> NormalModeContext<'_> {
        NormalModeContext {
            ctx: &mut self.ctx,
            download_state: &mut self.download_state,
            force_quit_state: &mut self.force_quit_state,
            last_tick: &mut self.last_tick,
            tick_rate: self.tick_rate,
        }
    }
}

// ==================== ForceQuitState Tests ====================

#[test]
fn test_force_quit_state_initial() {
    let state = ForceQuitState::default();
    assert!(!state.pending);
    assert!(state.time.is_none());
    assert!(!state.is_confirmed());
}

#[test]
fn test_force_quit_state_pending_not_confirmed_without_time() {
    let state = ForceQuitState {
        pending: true,
        ..Default::default()
    };
    // Without setting time, is_confirmed should return false
    assert!(!state.is_confirmed());
}

#[test]
fn test_force_quit_state_confirmed_within_timeout() {
    let state = ForceQuitState {
        pending: true,
        time: Some(Instant::now()),
    };
    // Should be confirmed within 2 seconds
    assert!(state.is_confirmed());
}

#[test]
fn test_force_quit_state_not_confirmed_after_timeout() {
    let state = ForceQuitState {
        pending: true,
        // Time is more than 2 seconds ago
        time: Some(Instant::now() - Duration::from_secs(3)),
    };
    assert!(!state.is_confirmed());
}

#[test]
fn test_force_quit_state_check_timeout_resets_state() {
    let mut state = ForceQuitState {
        pending: true,
        time: Some(Instant::now() - Duration::from_secs(3)),
    };

    state.check_timeout();

    assert!(!state.pending);
    assert!(state.time.is_none());
}

#[test]
fn test_force_quit_state_check_timeout_preserves_valid_state() {
    let now = Instant::now();
    let mut state = ForceQuitState {
        pending: true,
        time: Some(now),
    };

    state.check_timeout();

    // State should be preserved if within timeout
    assert!(state.pending);
    assert!(state.time.is_some());
}

// ==================== Filter Mode Tests ====================

#[test]
fn test_filter_mode_esc_clears_filter() {
    let state = create_test_state();
    let mut ctx = create_test_context();
    ctx.filter_mode = true;
    ctx.filter_text = "test".to_string();
    ctx.filtered_indices = vec![0, 1, 2];

    let result = handle_filter_mode_input(KeyCode::Esc, &state, &mut ctx);

    assert!(!ctx.filter_mode);
    assert!(ctx.filter_text.is_empty());
    assert!(ctx.filtered_indices.is_empty());
    assert!(matches!(result, InputResult::Continue));
}

#[test]
fn test_filter_mode_enter_keeps_filter() {
    let state = create_test_state();
    let mut ctx = create_test_context();
    ctx.filter_mode = true;
    ctx.filter_text = "test".to_string();

    let result = handle_filter_mode_input(KeyCode::Enter, &state, &mut ctx);

    assert!(!ctx.filter_mode);
    assert_eq!(ctx.filter_text, "test");
    assert!(matches!(result, InputResult::Continue));
}

#[test]
fn test_filter_mode_backspace_removes_char() {
    let state = create_test_state();
    let mut ctx = create_test_context();
    ctx.filter_mode = true;
    ctx.filter_text = "test".to_string();

    let result = handle_filter_mode_input(KeyCode::Backspace, &state, &mut ctx);

    assert_eq!(ctx.filter_text, "tes");
    assert!(matches!(result, InputResult::Continue));
}

#[test]
fn test_filter_mode_backspace_on_empty_string() {
    let state = create_test_state();
    let mut ctx = create_test_context();
    ctx.filter_mode = true;
    ctx.filter_text = String::new();

    let result = handle_filter_mode_input(KeyCode::Backspace, &state, &mut ctx);

    assert!(ctx.filter_text.is_empty());
    assert!(matches!(result, InputResult::Continue));
}

#[test]
fn test_filter_mode_char_input_adds_to_filter() {
    let state = create_test_state();
    let mut ctx = create_test_context();
    ctx.filter_mode = true;
    ctx.filter_text = "tes".to_string();

    let result = handle_filter_mode_input(KeyCode::Char('t'), &state, &mut ctx);

    assert_eq!(ctx.filter_text, "test");
    assert!(matches!(result, InputResult::Continue));
}

// ==================== Edit Mode Tests ====================

#[test]
fn test_edit_mode_up_navigation() {
    let state = create_test_state();
    let mut ctx = create_test_context();
    ctx.queue_edit_mode = true;
    ctx.queue_selected_index = 2;

    handle_edit_mode_input(KeyCode::Up, &state, &mut ctx);

    assert_eq!(ctx.queue_selected_index, 1);
}

#[test]
fn test_edit_mode_up_navigation_at_zero() {
    let state = create_test_state();
    let mut ctx = create_test_context();
    ctx.queue_edit_mode = true;
    ctx.queue_selected_index = 0;

    handle_edit_mode_input(KeyCode::Up, &state, &mut ctx);

    // Should stay at 0 (saturating_sub)
    assert_eq!(ctx.queue_selected_index, 0);
}

#[test]
fn test_edit_mode_down_navigation() {
    let state = create_test_state();
    // Add items to queue
    let _ = state.send(StateMessage::LoadLinks(vec![
        "url1".to_string(),
        "url2".to_string(),
        "url3".to_string(),
    ]));
    // Allow message processing
    thread::sleep(Duration::from_millis(50));

    let mut ctx = drawn_context(3, 3);
    ctx.queue_edit_mode = true;
    ctx.queue_selected_index = 0;

    handle_edit_mode_input(KeyCode::Down, &state, &mut ctx);

    assert_eq!(ctx.queue_selected_index, 1);
}

// Helper to build a state whose queue is url1..urlN in queue order,
// which the pending list shows bottom-up as urlN..url1.
fn state_with_queue(urls: &[&str]) -> AppState {
    let state = create_test_state();
    let _ = state.send(StateMessage::LoadLinks(
        urls.iter().map(|u| u.to_string()).collect(),
    ));
    thread::sleep(Duration::from_millis(50));
    state
}

/// A context in the state a render leaves behind: the panel was drawn from a
/// queue of `drawn_len` links into a viewport `view_height` rows tall.
fn drawn_context(drawn_len: usize, view_height: usize) -> UiContext {
    UiContext {
        queue_drawn_len: drawn_len,
        queue_view_height: view_height,
        queue_area: ratatui::layout::Rect::new(0, 0, 40, view_height as u16 + 2),
        ..Default::default()
    }
}

#[test]
fn test_edit_mode_selection_starts_on_newest_link() {
    let state = state_with_queue(&["url1", "url2", "url3"]);
    let mut ctx = create_test_context();

    handle_edit_mode(&state, &mut ctx);

    assert!(ctx.queue_edit_mode);
    assert_eq!(ctx.queue_selected_index, 0);
    assert_eq!(ctx.queue_scroll, 0);
}

#[test]
fn test_edit_mode_move_up_moves_link_toward_top_of_panel() {
    // Displayed newest first: url3, url2, url1. Selecting url2 and pressing K
    // should swap it above url3 on screen, i.e. after url3 in the queue.
    let state = state_with_queue(&["url1", "url2", "url3"]);
    let mut ctx = drawn_context(3, 3);
    ctx.queue_edit_mode = true;
    ctx.queue_selected_index = 1;

    handle_edit_mode_input(KeyCode::Char('k'), &state, &mut ctx);

    let queue = state.get_queue().expect("failed to read queue");
    assert_eq!(queue, vec!["url1", "url3", "url2"]);
    assert_eq!(ctx.queue_selected_index, 0);
}

#[test]
fn test_edit_mode_move_down_moves_link_toward_bottom_of_panel() {
    let state = state_with_queue(&["url1", "url2", "url3"]);
    let mut ctx = drawn_context(3, 3);
    ctx.queue_edit_mode = true;
    ctx.queue_selected_index = 0;

    handle_edit_mode_input(KeyCode::Char('j'), &state, &mut ctx);

    let queue = state.get_queue().expect("failed to read queue");
    assert_eq!(queue, vec!["url1", "url3", "url2"]);
    assert_eq!(ctx.queue_selected_index, 1);
}

#[test]
fn test_edit_mode_move_up_at_top_is_a_no_op() {
    let state = state_with_queue(&["url1", "url2", "url3"]);
    let mut ctx = drawn_context(3, 3);
    ctx.queue_edit_mode = true;
    ctx.queue_selected_index = 0;

    handle_edit_mode_input(KeyCode::Char('k'), &state, &mut ctx);

    assert_eq!(
        state.get_queue().expect("failed to read queue"),
        vec!["url1", "url2", "url3"]
    );
    assert_eq!(ctx.queue_selected_index, 0);
}

#[test]
fn test_edit_mode_move_down_at_bottom_is_a_no_op() {
    let state = state_with_queue(&["url1", "url2", "url3"]);
    let mut ctx = drawn_context(3, 3);
    ctx.queue_edit_mode = true;
    ctx.queue_selected_index = 2;

    handle_edit_mode_input(KeyCode::Char('j'), &state, &mut ctx);

    assert_eq!(
        state.get_queue().expect("failed to read queue"),
        vec!["url1", "url2", "url3"]
    );
    assert_eq!(ctx.queue_selected_index, 2);
}

#[test]
fn test_edit_mode_delete_removes_the_displayed_row() {
    // Top of the panel is the newest link, url3
    let state = state_with_queue(&["url1", "url2", "url3"]);
    let mut ctx = drawn_context(3, 3);
    ctx.queue_edit_mode = true;
    ctx.queue_selected_index = 0;

    handle_edit_mode_input(KeyCode::Char('d'), &state, &mut ctx);

    assert_eq!(
        state.get_queue().expect("failed to read queue"),
        vec!["url1", "url2"]
    );
}

#[test]
fn test_edit_mode_targets_the_drawn_row_when_links_arrive_mid_frame() {
    // The panel was drawn from 3 links with the newest, url3, on the top row.
    // Two more links land before the keypress is handled; deleting row 0 must
    // still remove url3, not a link the user has never seen.
    let state = state_with_queue(&["url1", "url2", "url3"]);
    let mut ctx = drawn_context(3, 3);
    ctx.queue_edit_mode = true;
    ctx.queue_selected_index = 0;

    let _ = state.send(StateMessage::AddToQueue("url4".to_string()));
    let _ = state.send(StateMessage::AddToQueue("url5".to_string()));
    thread::sleep(Duration::from_millis(50));

    handle_edit_mode_input(KeyCode::Char('d'), &state, &mut ctx);

    assert_eq!(
        state.get_queue().expect("failed to read queue"),
        vec!["url1", "url2", "url4", "url5"]
    );
}

#[test]
fn test_edit_mode_end_selects_oldest_link() {
    let state = state_with_queue(&["url1", "url2", "url3"]);
    let mut ctx = drawn_context(3, 2);
    ctx.queue_edit_mode = true;

    handle_edit_mode_input(KeyCode::End, &state, &mut ctx);

    assert_eq!(ctx.queue_selected_index, 2);
    // The view follows the selection off the bottom of the panel
    assert_eq!(ctx.queue_scroll, 1);
}

#[test]
fn test_filter_scrolls_matches_into_view() {
    // Matches can sit anywhere in a long list, and the filter dims rather than
    // hides, so a filter typed while scrolled must bring the matches on screen
    let state = state_with_queue(&[
        "https://example.com/match-me",
        "https://other.com/a",
        "https://other.com/b",
        "https://other.com/c",
        "https://other.com/d",
        "https://other.com/e",
    ]);
    let mut ctx = drawn_context(6, 2);
    ctx.filter_mode = true;

    for c in "match".chars() {
        handle_filter_mode_input(KeyCode::Char(c), &state, &mut ctx);
    }

    assert_eq!(ctx.filtered_indices, vec![0]);
    // The only match is the oldest link, on the bottom display row
    assert_eq!(ctx.queue_scroll, 4);
}

#[test]
fn test_clearing_the_filter_returns_to_the_newest_links() {
    let state = state_with_queue(&["url1", "url2", "url3", "url4", "url5", "url6"]);
    let mut ctx = drawn_context(6, 2);
    ctx.filter_mode = true;
    ctx.filter_text = "url1".to_string();
    ctx.queue_scroll = 4;

    handle_filter_mode_input(KeyCode::Esc, &state, &mut ctx);

    assert_eq!(ctx.queue_scroll, 0);
}

#[test]
fn test_backspacing_the_filter_empty_returns_to_the_top() {
    let state = state_with_queue(&["url1", "url2", "url3", "url4", "url5", "url6"]);
    let mut ctx = drawn_context(6, 2);
    ctx.filter_mode = true;
    ctx.filter_text = "u".to_string();
    ctx.queue_scroll = 4;

    handle_filter_mode_input(KeyCode::Backspace, &state, &mut ctx);

    assert!(ctx.filter_text.is_empty());
    assert_eq!(ctx.queue_scroll, 0);
}

// ==================== Pending List Scrolling Tests ====================

#[test]
fn test_normal_mode_arrows_scroll_pending_list() {
    let state = state_with_queue(&["url1", "url2", "url3", "url4", "url5"]);
    let args = create_test_args();
    let mut nmc = TestNmc::default();
    nmc.ctx.queue_view_height = 2;

    handle_normal_mode_input(KeyCode::Down, &state, &args, &mut nmc.context());
    handle_normal_mode_input(KeyCode::Down, &state, &args, &mut nmc.context());
    assert_eq!(nmc.ctx.queue_scroll, 2);

    handle_normal_mode_input(KeyCode::Up, &state, &args, &mut nmc.context());
    assert_eq!(nmc.ctx.queue_scroll, 1);

    // End stops at the last full page (5 items, 2 rows visible)
    handle_normal_mode_input(KeyCode::End, &state, &args, &mut nmc.context());
    assert_eq!(nmc.ctx.queue_scroll, 3);

    handle_normal_mode_input(KeyCode::Home, &state, &args, &mut nmc.context());
    assert_eq!(nmc.ctx.queue_scroll, 0);

    handle_normal_mode_input(KeyCode::PageDown, &state, &args, &mut nmc.context());
    assert_eq!(nmc.ctx.queue_scroll, 2);
}

// Wheel event landing at (column, row) on screen
fn wheel_at(kind: MouseEventKind, column: u16, row: u16) -> MouseEvent {
    MouseEvent {
        kind,
        column,
        row,
        modifiers: crossterm::event::KeyModifiers::NONE,
    }
}

#[test]
fn test_mouse_wheel_scrolls_pending_list() {
    let state = state_with_queue(&["url1", "url2", "url3", "url4", "url5", "url6"]);
    let mut ctx = drawn_context(6, 2);

    handle_mouse_input(wheel_at(MouseEventKind::ScrollDown, 5, 2), &state, &mut ctx);
    assert_eq!(ctx.queue_scroll, 3);

    handle_mouse_input(wheel_at(MouseEventKind::ScrollUp, 5, 2), &state, &mut ctx);
    assert_eq!(ctx.queue_scroll, 0);
}

#[test]
fn test_mouse_wheel_outside_the_pending_panel_is_ignored() {
    let state = state_with_queue(&["url1", "url2", "url3", "url4", "url5", "url6"]);
    // Panel occupies columns 0..40, rows 0..4
    let mut ctx = drawn_context(6, 2);

    // Over the Logs pane, well below the panel
    handle_mouse_input(
        wheel_at(MouseEventKind::ScrollDown, 5, 20),
        &state,
        &mut ctx,
    );
    assert_eq!(ctx.queue_scroll, 0);

    // Over the Active Downloads pane, to the right of the panel
    handle_mouse_input(
        wheel_at(MouseEventKind::ScrollDown, 60, 2),
        &state,
        &mut ctx,
    );
    assert_eq!(ctx.queue_scroll, 0);
}

#[test]
fn test_mouse_wheel_in_edit_mode_moves_the_selection() {
    // The wheel must not leave the highlighted row off-screen, or D would
    // delete a link the user cannot see.
    let state = state_with_queue(&["url1", "url2", "url3", "url4", "url5", "url6"]);
    let mut ctx = drawn_context(6, 2);
    ctx.queue_edit_mode = true;

    handle_mouse_input(wheel_at(MouseEventKind::ScrollDown, 5, 2), &state, &mut ctx);
    assert_eq!(ctx.queue_selected_index, 3);
    assert_eq!(ctx.queue_scroll, 2);
    assert!(ctx.queue_selected_index >= ctx.queue_scroll);
    assert!(ctx.queue_selected_index < ctx.queue_scroll + ctx.queue_view_height);

    handle_mouse_input(wheel_at(MouseEventKind::ScrollUp, 5, 2), &state, &mut ctx);
    assert_eq!(ctx.queue_selected_index, 0);
    assert_eq!(ctx.queue_scroll, 0);
}

#[test]
fn test_mouse_wheel_in_edit_mode_stops_at_the_oldest_link() {
    let state = state_with_queue(&["url1", "url2", "url3"]);
    let mut ctx = drawn_context(3, 2);
    ctx.queue_edit_mode = true;

    handle_mouse_input(wheel_at(MouseEventKind::ScrollDown, 5, 2), &state, &mut ctx);

    assert_eq!(ctx.queue_selected_index, 2);
}

#[test]
fn test_edit_mode_esc_exits() {
    let state = create_test_state();
    let mut ctx = create_test_context();
    ctx.queue_edit_mode = true;

    let result = handle_edit_mode_input(KeyCode::Esc, &state, &mut ctx);

    assert!(!ctx.queue_edit_mode);
    assert!(matches!(result, InputResult::Continue));
}

#[test]
fn test_edit_mode_enter_exits() {
    let state = create_test_state();
    let mut ctx = create_test_context();
    ctx.queue_edit_mode = true;

    handle_edit_mode_input(KeyCode::Enter, &state, &mut ctx);

    assert!(!ctx.queue_edit_mode);
}

#[test]
fn test_edit_mode_e_exits() {
    let state = create_test_state();
    let mut ctx = create_test_context();
    ctx.queue_edit_mode = true;

    handle_edit_mode_input(KeyCode::Char('e'), &state, &mut ctx);

    assert!(!ctx.queue_edit_mode);
}

// ==================== Help Overlay Tests ====================

#[test]
fn test_help_overlay_f1_closes() {
    let mut show_help = true;

    let result = handle_help_overlay_input(KeyCode::F(1), &mut show_help);

    assert!(!show_help);
    assert!(matches!(result, InputResult::Continue));
}

#[test]
fn test_help_overlay_esc_closes() {
    let mut show_help = true;

    let result = handle_help_overlay_input(KeyCode::Esc, &mut show_help);

    assert!(!show_help);
    assert!(matches!(result, InputResult::Continue));
}

#[test]
fn test_help_overlay_other_keys_continue() {
    let mut show_help = true;

    let result = handle_help_overlay_input(KeyCode::Char('a'), &mut show_help);

    // Help should still be showing (other keys don't close it)
    assert!(show_help);
    assert!(matches!(result, InputResult::Continue));
}

// ==================== Normal Mode Key Mapping Tests ====================

#[test]
fn test_normal_mode_f1_shows_help() {
    let state = create_test_state();
    let args = create_test_args();
    let mut nmc = TestNmc::default();

    let result = handle_normal_mode_input(KeyCode::F(1), &state, &args, &mut nmc.context());

    assert!(nmc.ctx.show_help);
    assert!(matches!(result, InputResult::Continue));
}

#[test]
fn test_normal_mode_q_graceful_quit() {
    let state = create_test_state();
    let args = create_test_args();
    let mut nmc = TestNmc::default();

    let result = handle_normal_mode_input(KeyCode::Char('q'), &state, &args, &mut nmc.context());

    assert!(nmc.download_state.await_downloads_on_exit);
    assert!(matches!(result, InputResult::Break));
}

#[test]
fn test_normal_mode_shift_q_initiates_force_quit() {
    let state = create_test_state();
    let args = create_test_args();
    let mut nmc = TestNmc::default();

    let result = handle_normal_mode_input(KeyCode::Char('Q'), &state, &args, &mut nmc.context());

    assert!(nmc.force_quit_state.pending);
    assert!(nmc.force_quit_state.time.is_some());
    assert!(matches!(result, InputResult::Continue));
}

#[test]
fn test_normal_mode_shift_q_confirms_force_quit() {
    let state = create_test_state();
    let args = create_test_args();
    let mut nmc = TestNmc {
        force_quit_state: ForceQuitState {
            pending: true,
            time: Some(Instant::now()),
        },
        ..Default::default()
    };

    let result = handle_normal_mode_input(KeyCode::Char('Q'), &state, &args, &mut nmc.context());

    assert!(matches!(result, InputResult::Break));
}

#[test]
fn test_normal_mode_p_pause_toggle() {
    let state = create_test_state();
    // Start downloads first
    let _ = state.send(StateMessage::SetStarted(true));
    thread::sleep(Duration::from_millis(50));

    let args = create_test_args();
    let mut nmc = TestNmc::default();

    let result = handle_normal_mode_input(KeyCode::Char('p'), &state, &args, &mut nmc.context());

    assert!(matches!(result, InputResult::Continue));
}

#[test]
fn test_normal_mode_slash_enters_filter_mode() {
    let state = create_test_state();
    let args = create_test_args();
    let mut nmc = TestNmc::default();

    let result = handle_normal_mode_input(KeyCode::Char('/'), &state, &args, &mut nmc.context());

    assert!(nmc.ctx.filter_mode);
    assert!(nmc.ctx.filter_text.is_empty());
    assert!(nmc.ctx.filtered_indices.is_empty());
    assert!(matches!(result, InputResult::Continue));
}

#[test]
fn test_normal_mode_f2_returns_unhandled() {
    let state = create_test_state();
    let args = create_test_args();
    let mut nmc = TestNmc::default();

    let result = handle_normal_mode_input(KeyCode::F(2), &state, &args, &mut nmc.context());

    // F2 returns Unhandled so the caller can toggle settings menu
    assert!(matches!(result, InputResult::Unhandled));
}

#[test]
fn test_normal_mode_unknown_key_unhandled() {
    let state = create_test_state();
    let args = create_test_args();
    let mut nmc = TestNmc::default();

    let result = handle_normal_mode_input(KeyCode::Char('z'), &state, &args, &mut nmc.context());

    assert!(matches!(result, InputResult::Unhandled));
}

// ==================== yt-dlp Update Tests ====================

#[test]
fn test_normal_mode_u_handled() {
    let state = create_test_state();
    let args = create_test_args();
    let mut nmc = TestNmc::default();

    let result = handle_normal_mode_input(KeyCode::Char('u'), &state, &args, &mut nmc.context());

    assert!(matches!(result, InputResult::Continue));
}

#[test]
fn test_normal_mode_u_blocked_when_downloads_active() {
    let state = create_test_state();
    let _ = state.send(StateMessage::SetStarted(true));
    thread::sleep(Duration::from_millis(50));

    let args = create_test_args();
    let mut nmc = TestNmc::default();

    let result = handle_normal_mode_input(KeyCode::Char('u'), &state, &args, &mut nmc.context());

    assert!(matches!(result, InputResult::Continue));

    // Check that a "Cannot update" log was added
    let snapshot = state
        .get_ui_snapshot()
        .expect("failed to build UI snapshot");
    assert!(
        snapshot
            .logs
            .iter()
            .any(|l| l.contains("Cannot update while downloads are active"))
    );
}

// ==================== Retry Failed Tests ====================

#[test]
fn test_normal_mode_t_handled() {
    let state = create_test_state();
    let args = create_test_args();
    let mut nmc = TestNmc::default();

    let result = handle_normal_mode_input(KeyCode::Char('t'), &state, &args, &mut nmc.context());

    assert!(matches!(result, InputResult::Continue));
}

#[test]
fn test_normal_mode_t_requeues_failed() {
    let state = create_test_state();

    // Add failed downloads
    state
        .send(StateMessage::AddFailedDownload(
            "https://example.com/video1".to_string(),
        ))
        .expect("failed to send AddFailedDownload");
    state
        .send(StateMessage::AddFailedDownload(
            "https://example.com/video2".to_string(),
        ))
        .expect("failed to send AddFailedDownload");
    thread::sleep(Duration::from_millis(50));

    let args = create_test_args();
    let mut nmc = TestNmc::default();

    handle_normal_mode_input(KeyCode::Char('t'), &state, &args, &mut nmc.context());

    // Wait for message processing
    thread::sleep(Duration::from_millis(100));

    // Verify URLs were re-queued
    let queue = state.get_queue().expect("failed to read queue");
    assert_eq!(queue.len(), 2);

    // Failed count should be 0 after take
    let snapshot = state
        .get_ui_snapshot()
        .expect("failed to build UI snapshot");
    assert_eq!(snapshot.failed_count, 0);
}

#[test]
fn test_normal_mode_t_no_failed_logs_message() {
    let state = create_test_state();
    let args = create_test_args();
    let mut nmc = TestNmc::default();

    handle_normal_mode_input(KeyCode::Char('t'), &state, &args, &mut nmc.context());

    let snapshot = state
        .get_ui_snapshot()
        .expect("failed to build UI snapshot");
    assert!(
        snapshot
            .logs
            .iter()
            .any(|l| l.contains("No failed downloads to retry"))
    );
}

#[test]
fn test_normal_mode_t_blocked_when_downloads_active() {
    let state = create_test_state();
    let _ = state.send(StateMessage::SetStarted(true));
    thread::sleep(Duration::from_millis(50));

    let args = create_test_args();
    let mut nmc = TestNmc::default();

    handle_normal_mode_input(KeyCode::Char('t'), &state, &args, &mut nmc.context());

    let snapshot = state
        .get_ui_snapshot()
        .expect("failed to build UI snapshot");
    assert!(
        snapshot
            .logs
            .iter()
            .any(|l| l.contains("Cannot retry while downloads are active"))
    );
}

// ==================== DownloadState Tests ====================

#[test]
fn test_download_state_default() {
    let state = DownloadState::default();
    assert!(state.download_thread_handle.is_none());
    assert!(!state.await_downloads_on_exit);
}

// ==================== Guard Predicate Coverage ====================

/// Every key that must refuse to run while yt-dlp subprocesses may be alive.
/// Each entry is the key and a fragment of the refusal it should log.
const GUARDED_KEYS: [(char, &str); 4] = [
    ('u', "Cannot update while downloads are active"),
    ('t', "Cannot retry while downloads are active"),
    ('f', "Cannot load links while downloads are active"),
    ('r', "Cannot reload links while downloads are active"),
];

fn logs_contain(state: &AppState, fragment: &str) -> bool {
    state
        .get_ui_snapshot()
        .expect("failed to build UI snapshot")
        .logs
        .iter()
        .any(|line| line.contains(fragment))
}

#[test]
fn test_guarded_keys_refuse_while_downloads_are_paused() {
    // Pausing stops workers popping new URLs but leaves the spawned yt-dlp
    // processes writing files, so a paused session is still in flight.
    for (key, refusal) in GUARDED_KEYS {
        let state = state_with_queue(&["url1", "url2"]);
        let _ = state.send(StateMessage::SetStarted(true));
        let _ = state.send(StateMessage::SetPaused(true));
        thread::sleep(Duration::from_millis(50));

        let args = create_test_args();
        let mut nmc = TestNmc::default();
        handle_normal_mode_input(KeyCode::Char(key), &state, &args, &mut nmc.context());

        assert!(
            logs_contain(&state, refusal),
            "'{}' was allowed while paused",
            key
        );
    }
}

#[test]
fn test_guarded_keys_refuse_while_the_controller_is_draining() {
    // After S-to-stop the started flag is already false, but the controller is
    // still joining workers - the drain window the guards used to miss.
    for (key, refusal) in GUARDED_KEYS {
        let state = state_with_queue(&["url1", "url2"]);
        state
            .set_controller_active(true)
            .expect("failed to mark controller active");
        let _ = state.send(StateMessage::SetStarted(false));
        thread::sleep(Duration::from_millis(50));

        let args = create_test_args();
        let mut nmc = TestNmc::default();
        handle_normal_mode_input(KeyCode::Char(key), &state, &args, &mut nmc.context());

        assert!(
            logs_contain(&state, refusal),
            "'{}' was allowed mid-drain",
            key
        );
    }
}

#[test]
fn test_guarded_keys_are_allowed_once_the_controller_has_exited() {
    for (key, refusal) in GUARDED_KEYS {
        let state = state_with_queue(&["url1", "url2"]);
        state
            .set_controller_active(false)
            .expect("failed to clear controller active");
        thread::sleep(Duration::from_millis(50));

        let args = create_test_args();
        let mut nmc = TestNmc::default();
        handle_normal_mode_input(KeyCode::Char(key), &state, &args, &mut nmc.context());

        assert!(
            !logs_contain(&state, refusal),
            "'{}' was refused while nothing was running",
            key
        );
    }
}

#[test]
fn test_edit_mode_refuses_while_downloads_are_paused() {
    let state = state_with_queue(&["url1", "url2"]);
    let _ = state.send(StateMessage::SetStarted(true));
    let _ = state.send(StateMessage::SetPaused(true));
    thread::sleep(Duration::from_millis(50));

    let args = create_test_args();
    let mut nmc = TestNmc::default();
    handle_normal_mode_input(KeyCode::Char('e'), &state, &args, &mut nmc.context());

    assert!(!nmc.ctx.queue_edit_mode, "edit mode opened while paused");
    assert!(logs_contain(
        &state,
        "Cannot edit queue while downloads are active"
    ));
}

// ==================== Delete Binding Case ====================

#[test]
fn test_edit_mode_delete_accepts_both_cases_and_the_delete_key() {
    // The panel title and footer both advertise "D: Delete", so uppercase has to
    // work; K/J already accept either case.
    for key in [KeyCode::Char('d'), KeyCode::Char('D'), KeyCode::Delete] {
        let state = state_with_queue(&["url1", "url2", "url3"]);
        let mut ctx = drawn_context(3, 3);
        ctx.queue_edit_mode = true;
        ctx.queue_selected_index = 0;

        handle_edit_mode_input(key, &state, &mut ctx);

        assert_eq!(
            state.queue_len().expect("failed to read queue"),
            2,
            "{:?} did not delete a row",
            key
        );
    }
}

#[test]
fn test_edit_mode_move_keys_accept_both_cases() {
    for (key, expected_top) in [(KeyCode::Char('j'), "url2"), (KeyCode::Char('J'), "url2")] {
        let state = state_with_queue(&["url1", "url2", "url3"]);
        let mut ctx = drawn_context(3, 3);
        ctx.queue_edit_mode = true;
        // Display row 0 is the newest link, url3; move it down the panel
        ctx.queue_selected_index = 0;

        handle_edit_mode_input(key, &state, &mut ctx);

        let queue = state.get_queue().expect("failed to read queue");
        assert_eq!(
            queue.back().map(String::as_str),
            Some(expected_top),
            "{:?} did not reorder the queue",
            key
        );
    }
}
