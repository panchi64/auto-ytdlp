use super::*;

fn queue_of(urls: &[&str]) -> VecDeque<String> {
    urls.iter().map(|u| u.to_string()).collect()
}

// ==================== Scrolling Tests ====================

#[test]
fn test_max_queue_scroll_stops_at_last_full_page() {
    let mut ctx = UiContext {
        queue_view_height: 10,
        ..Default::default()
    };
    assert_eq!(ctx.max_queue_scroll(25), 15);
    // A queue that fits entirely never scrolls
    assert_eq!(ctx.max_queue_scroll(4), 0);

    ctx.queue_view_height = 0;
    assert_eq!(ctx.max_queue_scroll(25), 25);
}

#[test]
fn test_scroll_queue_by_clamps_to_both_ends() {
    let mut ctx = UiContext {
        queue_view_height: 5,
        ..Default::default()
    };

    ctx.scroll_queue_by(3, 20);
    assert_eq!(ctx.queue_scroll, 3);

    // Cannot scroll past the last full page (20 - 5)
    ctx.scroll_queue_by(100, 20);
    assert_eq!(ctx.queue_scroll, 15);

    // Cannot scroll above the newest link
    ctx.scroll_queue_by(-100, 20);
    assert_eq!(ctx.queue_scroll, 0);
}

#[test]
fn test_scroll_queue_by_reclamps_when_queue_shrinks() {
    let mut ctx = UiContext {
        queue_view_height: 5,
        queue_scroll: 15,
        ..Default::default()
    };

    ctx.scroll_queue_by(0, 8);
    assert_eq!(ctx.queue_scroll, 3);
}

#[test]
fn test_scroll_selection_into_view_follows_selection() {
    let mut ctx = UiContext {
        queue_view_height: 5,
        ..Default::default()
    };

    // Selection below the viewport pulls the view down
    ctx.queue_selected_index = 7;
    ctx.scroll_selection_into_view();
    assert_eq!(ctx.queue_scroll, 3);

    // Selection already visible leaves the view alone
    ctx.queue_selected_index = 5;
    ctx.scroll_selection_into_view();
    assert_eq!(ctx.queue_scroll, 3);

    // Selection above the viewport pulls the view up
    ctx.queue_selected_index = 1;
    ctx.scroll_selection_into_view();
    assert_eq!(ctx.queue_scroll, 1);
}

// ==================== New Link Flash Tests ====================

#[test]
fn test_first_frame_does_not_flash_restored_links() {
    let mut ctx = UiContext::default();
    ctx.track_new_links(&queue_of(&["url1", "url2"]));

    assert!(ctx.flash_progress("url1").is_none());
    assert!(ctx.flash_progress("url2").is_none());
}

#[test]
fn test_links_added_after_first_frame_flash() {
    let mut ctx = UiContext::default();
    ctx.track_new_links(&queue_of(&["url1"]));
    ctx.track_new_links(&queue_of(&["url1", "url2"]));

    assert!(ctx.flash_progress("url1").is_none());
    let progress = ctx.flash_progress("url2").expect("new link should flash");
    assert!((0.0..1.0).contains(&progress));
}

#[test]
fn test_flash_is_dropped_when_link_leaves_queue() {
    let mut ctx = UiContext::default();
    ctx.track_new_links(&queue_of(&["url1"]));
    ctx.track_new_links(&queue_of(&["url1", "url2"]));
    assert!(ctx.flash_progress("url2").is_some());

    // url2 gets popped for download
    ctx.track_new_links(&queue_of(&["url1"]));
    assert!(ctx.flash_progress("url2").is_none());
}

#[test]
fn test_unknown_link_has_no_flash() {
    let ctx = UiContext::default();
    assert!(ctx.flash_progress("never-seen").is_none());
}

// ==================== Scroll Anchoring Tests ====================

#[test]
fn test_added_links_shift_a_scrolled_view_to_stay_on_the_same_links() {
    let mut ctx = UiContext {
        queue_view_height: 3,
        queue_scroll: 4,
        ..Default::default()
    };
    ctx.track_new_links(&queue_of(&["url1", "url2", "url3", "url4", "url5", "url6"]));

    // Two links arrive at the back of the queue, i.e. the top of the panel
    ctx.track_new_links(&queue_of(&[
        "url1", "url2", "url3", "url4", "url5", "url6", "url7", "url8",
    ]));

    assert_eq!(ctx.queue_scroll, 6);
}

#[test]
fn test_added_links_stay_visible_when_parked_at_the_top() {
    let mut ctx = UiContext {
        queue_view_height: 3,
        ..Default::default()
    };
    ctx.track_new_links(&queue_of(&["url1", "url2"]));

    ctx.track_new_links(&queue_of(&["url1", "url2", "url3"]));

    assert_eq!(ctx.queue_scroll, 0);
}

// ==================== Filter Scrolling Tests ====================

#[test]
fn test_scroll_to_first_match_puts_the_newest_match_on_top() {
    let mut ctx = UiContext {
        queue_view_height: 3,
        queue_scroll: 0,
        // url2 and url4 match, in queue order
        filtered_indices: vec![1, 3],
        ..Default::default()
    };

    // 8 links display as url8..url1, so url4 sits on display row 4
    ctx.scroll_to_first_match(8);

    assert_eq!(ctx.queue_scroll, 4);
}

#[test]
fn test_scroll_to_first_match_stays_within_the_list() {
    let mut ctx = UiContext {
        queue_view_height: 3,
        // The oldest link matches, i.e. the bottom display row
        filtered_indices: vec![0],
        ..Default::default()
    };

    ctx.scroll_to_first_match(8);

    // Bottom row is display 7, but scrolling stops at the last full page
    assert_eq!(ctx.queue_scroll, 5);
}

#[test]
fn test_scroll_to_first_match_with_no_matches_returns_to_the_top() {
    let mut ctx = UiContext {
        queue_view_height: 3,
        queue_scroll: 4,
        ..Default::default()
    };

    ctx.scroll_to_first_match(8);

    assert_eq!(ctx.queue_scroll, 0);
}

#[test]
fn test_page_size_is_never_zero() {
    let ctx = UiContext::default();
    assert_eq!(ctx.page_size(), 1);

    let ctx = UiContext {
        queue_view_height: 12,
        ..Default::default()
    };
    assert_eq!(ctx.page_size(), 12);
}

#[test]
fn test_added_links_keep_edit_mode_selection_on_the_same_link() {
    let mut ctx = UiContext {
        queue_view_height: 3,
        queue_edit_mode: true,
        queue_selected_index: 1,
        ..Default::default()
    };
    ctx.track_new_links(&queue_of(&["url1", "url2", "url3"]));

    ctx.track_new_links(&queue_of(&["url1", "url2", "url3", "url4", "url5"]));

    // url2 was on display row 1 of 3; with 5 links it is on row 3
    assert_eq!(ctx.queue_selected_index, 3);
    assert_eq!(ctx.queue_scroll, 2);
}
