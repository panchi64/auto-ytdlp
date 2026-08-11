use super::*;
use crate::ui::settings_menu::tests::{create_test_state, key_event, test_menu};
use crate::utils::settings::{Settings, SettingsPreset};
use crossterm::event::KeyCode;

// ==================== Boolean Toggle Tests ====================

#[test]
fn test_settings_menu_boolean_toggle_write_thumbnail() {
    let state = create_test_state();
    let mut menu = test_menu(&state);
    menu.toggle();

    menu.list_state.select(Some(IDX_WRITE_THUMBNAIL));

    // Force initial value to false
    menu.settings.write_thumbnail = false;

    // Enter editing mode
    menu.handle_input(key_event(KeyCode::Enter), &state);
    assert!(menu.editing);

    // Toggle with Right arrow (should auto-apply for boolean)
    // option_index starts at 0 (No), Right moves to 1 (Yes)
    menu.handle_input(key_event(KeyCode::Right), &state);

    // Boolean toggle should auto-exit editing mode and set to true
    assert!(!menu.editing);
    assert!(menu.settings.write_thumbnail);
}

#[test]
fn test_settings_menu_boolean_toggle_network_retry() {
    let state = create_test_state();
    let mut menu = test_menu(&state);
    menu.toggle();

    menu.list_state.select(Some(IDX_NETWORK_RETRY));

    // Force initial value to false
    menu.settings.network_retry = false;

    // Enter editing mode
    menu.handle_input(key_event(KeyCode::Enter), &state);

    // Toggle with Right arrow - goes from No (0) to Yes (1)
    menu.handle_input(key_event(KeyCode::Right), &state);

    assert!(menu.settings.network_retry);
}

// ==================== Reset Stats on Batch Toggle Tests ====================

#[test]
fn test_reset_stats_setting_defaults_to_enabled() {
    let state = create_test_state();
    let menu = test_menu(&state);

    // Default should be true (per-session mode)
    assert!(menu.settings.reset_stats_on_new_batch);
}

#[test]
fn test_reset_stats_setting_can_be_disabled() {
    let state = create_test_state();
    let mut menu = test_menu(&state);
    menu.toggle();

    menu.list_state.select(Some(IDX_RESET_STATS_ON_BATCH));

    assert!(menu.settings.reset_stats_on_new_batch);

    // Enter editing mode
    menu.handle_input(key_event(KeyCode::Enter), &state);
    assert!(menu.editing);

    // option_index should be 1 (Yes) since setting is true
    assert_eq!(menu.option_index, 1);

    // Press Left to select "No" (index 0)
    menu.handle_input(key_event(KeyCode::Left), &state);

    // Boolean toggle auto-applies and exits editing
    assert!(!menu.editing);
    assert!(!menu.settings.reset_stats_on_new_batch);
}

#[test]
fn test_reset_stats_setting_can_be_enabled() {
    let state = create_test_state();
    let mut menu = test_menu(&state);
    menu.toggle();

    // Start with setting disabled
    menu.settings.reset_stats_on_new_batch = false;

    menu.list_state.select(Some(IDX_RESET_STATS_ON_BATCH));

    // Enter editing mode
    menu.handle_input(key_event(KeyCode::Enter), &state);
    assert!(menu.editing);

    // option_index should be 0 (No) since setting is false
    assert_eq!(menu.option_index, 0);

    // Press Right to select "Yes" (index 1)
    menu.handle_input(key_event(KeyCode::Right), &state);

    // Boolean toggle auto-applies and exits editing
    assert!(!menu.editing);
    assert!(menu.settings.reset_stats_on_new_batch);
}

#[test]
fn test_reset_stats_setting_persists_to_app_state() {
    let state = create_test_state();
    let mut menu = test_menu(&state);
    menu.toggle();

    // Navigate to Reset Stats on Batch and toggle it off
    menu.list_state.select(Some(IDX_RESET_STATS_ON_BATCH));
    menu.handle_input(key_event(KeyCode::Enter), &state);
    menu.handle_input(key_event(KeyCode::Left), &state);

    // Verify menu settings updated
    assert!(!menu.settings.reset_stats_on_new_batch);

    // Verify AppState was updated
    let app_settings = state.get_settings().expect("failed to read settings");
    assert!(!app_settings.reset_stats_on_new_batch);
}

#[test]
fn test_all_presets_include_reset_stats_setting() {
    // All presets should have the reset_stats_on_new_batch field set
    for preset in SettingsPreset::all() {
        let settings = preset.apply(&Settings::default());
        // All presets default to per-session mode (true)
        assert!(
            settings.reset_stats_on_new_batch,
            "Preset {:?} should have reset_stats_on_new_batch = true",
            preset.name()
        );
    }
}

// ==================== SponsorBlock Toggle Tests ====================

#[test]
fn test_sponsorblock_toggle_on() {
    let state = create_test_state();
    let mut menu = test_menu(&state);
    menu.toggle();

    menu.settings.sponsorblock = false;
    menu.list_state.select(Some(IDX_SPONSORBLOCK));

    // Enter editing mode
    menu.handle_input(key_event(KeyCode::Enter), &state);
    assert!(menu.editing);
    assert_eq!(menu.option_index, 0); // No

    // Toggle to Yes
    menu.handle_input(key_event(KeyCode::Right), &state);
    assert!(!menu.editing); // Boolean auto-applies
    assert!(menu.settings.sponsorblock);
}

#[test]
fn test_sponsorblock_toggle_off() {
    let state = create_test_state();
    let mut menu = test_menu(&state);
    menu.toggle();

    menu.settings.sponsorblock = true;
    menu.list_state.select(Some(IDX_SPONSORBLOCK));

    // Enter editing mode
    menu.handle_input(key_event(KeyCode::Enter), &state);
    assert!(menu.editing);
    assert_eq!(menu.option_index, 1); // Yes

    // Toggle to No
    menu.handle_input(key_event(KeyCode::Left), &state);
    assert!(!menu.editing); // Boolean auto-applies
    assert!(!menu.settings.sponsorblock);
}

// ==================== Rate Limit Tests ====================

#[test]
fn test_rate_limit_preset_selection() {
    let state = create_test_state();
    let mut menu = test_menu(&state);
    menu.toggle();

    menu.list_state.select(Some(IDX_RATE_LIMIT));

    // Enter editing mode
    menu.handle_input(key_event(KeyCode::Enter), &state);
    assert!(menu.editing);
    assert_eq!(menu.option_index, 0); // Unlimited (default)

    // Select 2M (index 3)
    menu.handle_input(key_event(KeyCode::Right), &state); // 500K
    menu.handle_input(key_event(KeyCode::Right), &state); // 1M
    menu.handle_input(key_event(KeyCode::Right), &state); // 2M
    menu.handle_input(key_event(KeyCode::Enter), &state);

    assert!(!menu.editing);
    assert_eq!(menu.settings.rate_limit, "2M");
}

#[test]
fn test_rate_limit_unlimited_clears_value() {
    let state = create_test_state();
    let mut menu = test_menu(&state);
    menu.toggle();

    // Set a rate limit first
    menu.settings.rate_limit = "5M".to_string();
    menu.list_state.select(Some(IDX_RATE_LIMIT));

    // Enter editing mode
    menu.handle_input(key_event(KeyCode::Enter), &state);
    assert_eq!(menu.option_index, 4); // 5M

    // Go back to Unlimited
    menu.handle_input(key_event(KeyCode::Left), &state); // 2M
    menu.handle_input(key_event(KeyCode::Left), &state); // 1M
    menu.handle_input(key_event(KeyCode::Left), &state); // 500K
    menu.handle_input(key_event(KeyCode::Left), &state); // Unlimited
    menu.handle_input(key_event(KeyCode::Enter), &state);

    assert!(menu.settings.rate_limit.is_empty());
}

#[test]
fn test_rate_limit_custom_input() {
    let state = create_test_state();
    let mut menu = test_menu(&state);
    menu.toggle();

    menu.list_state.select(Some(IDX_RATE_LIMIT));

    // Enter editing, go to Custom (index 6)
    menu.handle_input(key_event(KeyCode::Enter), &state);
    for _ in 0..6 {
        menu.handle_input(key_event(KeyCode::Right), &state);
    }
    assert_eq!(menu.option_index, 6);
    menu.handle_input(key_event(KeyCode::Enter), &state);
    assert!(menu.input_mode);

    // Type custom value
    menu.handle_input(key_event(KeyCode::Char('7')), &state);
    menu.handle_input(key_event(KeyCode::Char('5')), &state);
    menu.handle_input(key_event(KeyCode::Char('0')), &state);
    menu.handle_input(key_event(KeyCode::Char('K')), &state);
    menu.handle_input(key_event(KeyCode::Enter), &state);

    assert!(!menu.input_mode);
    assert_eq!(menu.settings.rate_limit, "750K");
}

// ==================== Cookies from Browser Tests ====================

#[test]
fn test_cookies_browser_selection() {
    let state = create_test_state();
    let mut menu = test_menu(&state);
    menu.toggle();

    menu.list_state.select(Some(IDX_COOKIES_BROWSER));

    // Enter editing mode
    menu.handle_input(key_event(KeyCode::Enter), &state);
    assert!(menu.editing);
    assert_eq!(menu.option_index, 0); // None

    // Select Firefox (index 1)
    menu.handle_input(key_event(KeyCode::Right), &state);
    menu.handle_input(key_event(KeyCode::Enter), &state);

    assert!(!menu.editing);
    assert_eq!(menu.settings.cookies_from_browser, "firefox");
}

#[test]
fn test_cookies_browser_chrome() {
    let state = create_test_state();
    let mut menu = test_menu(&state);
    menu.toggle();

    menu.list_state.select(Some(IDX_COOKIES_BROWSER));

    // Enter editing mode and go to Chrome (index 2)
    menu.handle_input(key_event(KeyCode::Enter), &state);
    menu.handle_input(key_event(KeyCode::Right), &state); // Firefox
    menu.handle_input(key_event(KeyCode::Right), &state); // Chrome
    menu.handle_input(key_event(KeyCode::Enter), &state);

    assert_eq!(menu.settings.cookies_from_browser, "chrome");
}

#[test]
fn test_cookies_browser_none_clears() {
    let state = create_test_state();
    let mut menu = test_menu(&state);
    menu.toggle();

    menu.settings.cookies_from_browser = "firefox".to_string();
    menu.list_state.select(Some(IDX_COOKIES_BROWSER));

    // Enter editing mode - should be at Firefox (index 1)
    menu.handle_input(key_event(KeyCode::Enter), &state);
    assert_eq!(menu.option_index, 1);

    // Go to None
    menu.handle_input(key_event(KeyCode::Left), &state);
    menu.handle_input(key_event(KeyCode::Enter), &state);

    assert!(menu.settings.cookies_from_browser.is_empty());
}
