
use super::*;
use crate::app_state::AppState;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

// Helper to create a KeyEvent
fn key_event(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

// Helper to create AppState for testing with default settings
// (avoids depending on the settings file on disk, which may be
// modified by other tests that call settings.save())
fn create_test_state() -> AppState {
    let state = AppState::new();
    state
        .update_settings(Settings::default())
        .expect("Failed to reset settings for test");
    state
}

// Helper to create a menu with no --download-dir override
fn test_menu(state: &AppState) -> SettingsMenu {
    SettingsMenu::new(state, None)
}

// ==================== Visibility Toggle Tests ====================

#[test]
fn test_settings_menu_initial_not_visible() {
    let state = create_test_state();
    let menu = test_menu(&state);
    assert!(!menu.is_visible());
}

#[test]
fn test_settings_menu_toggle_opens() {
    let state = create_test_state();
    let mut menu = test_menu(&state);

    menu.toggle();

    assert!(menu.is_visible());
}

#[test]
fn test_settings_menu_toggle_closes() {
    let state = create_test_state();
    let mut menu = test_menu(&state);

    menu.toggle(); // Open
    menu.toggle(); // Close

    assert!(!menu.is_visible());
}

#[test]
fn test_settings_menu_toggle_resets_state() {
    let state = create_test_state();
    let mut menu = test_menu(&state);

    // Set some state
    menu.editing = true;
    menu.input_mode = true;
    menu.sub_menu = SubMenu::PresetSelection;
    menu.validation_error = Some("error".to_string());

    menu.toggle(); // Open (should reset)

    assert!(menu.is_visible());
    assert!(!menu.editing);
    assert!(!menu.input_mode);
    assert_eq!(menu.sub_menu, SubMenu::None);
    assert!(menu.validation_error.is_none());
}

// ==================== Navigation Tests ====================

#[test]
fn test_settings_menu_navigation_down() {
    let state = create_test_state();
    let mut menu = test_menu(&state);
    menu.toggle();

    // Initially at 0
    assert_eq!(menu.list_state.selected(), Some(0));

    menu.handle_input(key_event(KeyCode::Down), &state);

    assert_eq!(menu.list_state.selected(), Some(1));
}

#[test]
fn test_settings_menu_navigation_up() {
    let state = create_test_state();
    let mut menu = test_menu(&state);
    menu.toggle();

    // Navigate down first
    menu.handle_input(key_event(KeyCode::Down), &state);
    menu.handle_input(key_event(KeyCode::Down), &state);

    assert_eq!(menu.list_state.selected(), Some(2));

    menu.handle_input(key_event(KeyCode::Up), &state);

    assert_eq!(menu.list_state.selected(), Some(1));
}

#[test]
fn test_settings_menu_navigation_up_at_top() {
    let state = create_test_state();
    let mut menu = test_menu(&state);
    menu.toggle();

    // Already at 0
    menu.handle_input(key_event(KeyCode::Up), &state);

    // Should stay at 0
    assert_eq!(menu.list_state.selected(), Some(0));
}

#[test]
fn test_settings_menu_navigation_down_at_bottom() {
    let state = create_test_state();
    let mut menu = test_menu(&state);
    menu.toggle();

    // Navigate to bottom
    for _ in 0..TOTAL_MENU_ITEMS {
        menu.handle_input(key_event(KeyCode::Down), &state);
    }

    // Should be at the last item
    assert_eq!(menu.list_state.selected(), Some(TOTAL_MENU_ITEMS - 1));
}

#[test]
fn test_settings_menu_esc_closes() {
    let state = create_test_state();
    let mut menu = test_menu(&state);
    menu.toggle();

    assert!(menu.is_visible());

    menu.handle_input(key_event(KeyCode::Esc), &state);

    assert!(!menu.is_visible());
}

// ==================== Boolean Toggle Tests ====================

#[test]
fn test_settings_menu_boolean_toggle_write_thumbnail() {
    let state = create_test_state();
    let mut menu = test_menu(&state);
    menu.toggle();

    // Navigate to Write Thumbnail (index 3)
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

    // Navigate to Network Retry (index 6)
    menu.list_state.select(Some(IDX_NETWORK_RETRY));

    // Force initial value to false
    menu.settings.network_retry = false;

    // Enter editing mode
    menu.handle_input(key_event(KeyCode::Enter), &state);

    // Toggle with Right arrow - goes from No (0) to Yes (1)
    menu.handle_input(key_event(KeyCode::Right), &state);

    assert!(menu.settings.network_retry);
}

// ==================== Custom Args Validation Tests ====================

#[test]
fn test_custom_args_validation_empty_is_valid() {
    let result = Settings::validate_custom_args("");
    assert!(result.is_ok());
}

#[test]
fn test_custom_args_validation_valid_args() {
    let result = Settings::validate_custom_args("--cookies-from-browser firefox");
    assert!(result.is_ok());
}

#[test]
fn test_custom_args_validation_conflict_download_archive() {
    let result = Settings::validate_custom_args("--download-archive test.txt");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("download-archive"));
}

#[test]
fn test_custom_args_validation_conflict_output() {
    let result = Settings::validate_custom_args("--output '%(title)s.%(ext)s'");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("output"));
}

#[test]
fn test_custom_args_validation_conflict_short_output() {
    let result = Settings::validate_custom_args("-o '%(title)s.%(ext)s'");
    assert!(result.is_err());
}

#[test]
fn test_custom_args_validation_unmatched_quotes() {
    let result = Settings::validate_custom_args("--cookies 'unmatched");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("quotes"));
}

// ==================== Preset Application Tests ====================

#[test]
fn test_preset_best_quality_applies_correct_settings() {
    let settings = SettingsPreset::BestQuality.apply(&Settings::default());

    assert_eq!(settings.format_preset, FormatPreset::Best);
    assert_eq!(settings.output_format, OutputFormat::Auto);
    assert!(settings.write_subtitles);
    assert!(settings.write_thumbnail);
    assert!(settings.add_metadata);
    assert_eq!(settings.concurrent_downloads, 4);
    assert!(settings.network_retry);
}

#[test]
fn test_preset_audio_archive_applies_correct_settings() {
    let settings = SettingsPreset::AudioArchive.apply(&Settings::default());

    assert_eq!(settings.format_preset, FormatPreset::AudioOnly);
    assert_eq!(settings.output_format, OutputFormat::MP3);
    assert!(!settings.write_subtitles);
    assert!(settings.add_metadata);
}

#[test]
fn test_preset_fast_download_applies_correct_settings() {
    let settings = SettingsPreset::FastDownload.apply(&Settings::default());

    assert_eq!(settings.format_preset, FormatPreset::Best);
    assert!(!settings.write_subtitles);
    assert!(!settings.write_thumbnail);
    assert!(!settings.add_metadata);
    assert_eq!(settings.concurrent_downloads, 8);
    assert!(!settings.network_retry);
}

#[test]
fn test_preset_bandwidth_saver_applies_correct_settings() {
    let settings = SettingsPreset::BandwidthSaver.apply(&Settings::default());

    assert_eq!(settings.format_preset, FormatPreset::SD480p);
    assert_eq!(settings.concurrent_downloads, 2);
    assert!(settings.network_retry);
}

// ==================== Reset Confirmation Tests ====================

#[test]
fn test_reset_confirmation_esc_cancels() {
    let state = create_test_state();
    let mut menu = test_menu(&state);
    menu.toggle();
    menu.sub_menu = SubMenu::ResetConfirmation;

    let handled = menu.handle_input(key_event(KeyCode::Esc), &state);

    assert!(handled);
    assert_eq!(menu.sub_menu, SubMenu::None);
}

#[test]
fn test_reset_confirmation_n_cancels() {
    let state = create_test_state();
    let mut menu = test_menu(&state);
    menu.toggle();
    menu.sub_menu = SubMenu::ResetConfirmation;

    let handled = menu.handle_input(key_event(KeyCode::Char('n')), &state);

    assert!(handled);
    assert_eq!(menu.sub_menu, SubMenu::None);
}

#[test]
fn test_reset_confirmation_y_resets() {
    let state = create_test_state();
    let mut menu = test_menu(&state);
    menu.toggle();

    // Modify settings
    menu.settings.concurrent_downloads = 99;
    menu.sub_menu = SubMenu::ResetConfirmation;

    menu.handle_input(key_event(KeyCode::Char('y')), &state);

    // Settings should be reset to default
    assert_eq!(
        menu.settings.concurrent_downloads,
        Settings::default().concurrent_downloads
    );
    assert_eq!(menu.sub_menu, SubMenu::None);
}

// ==================== Format/Preset Display String Tests ====================

#[test]
fn test_format_preset_to_string() {
    let state = create_test_state();
    let menu = test_menu(&state);

    assert_eq!(menu.format_preset_to_string(&FormatPreset::Best), "Best");
    assert_eq!(
        menu.format_preset_to_string(&FormatPreset::AudioOnly),
        "Audio Only"
    );
    assert_eq!(
        menu.format_preset_to_string(&FormatPreset::HD1080p),
        "1080p"
    );
    assert_eq!(menu.format_preset_to_string(&FormatPreset::HD720p), "720p");
    assert_eq!(menu.format_preset_to_string(&FormatPreset::SD480p), "480p");
    assert_eq!(menu.format_preset_to_string(&FormatPreset::SD360p), "360p");
}

#[test]
fn test_output_format_to_string() {
    let state = create_test_state();
    let menu = test_menu(&state);

    assert_eq!(menu.output_format_to_string(&OutputFormat::Auto), "Auto");
    assert_eq!(menu.output_format_to_string(&OutputFormat::MP4), "MP4");
    assert_eq!(menu.output_format_to_string(&OutputFormat::Mkv), "MKV");
    assert_eq!(menu.output_format_to_string(&OutputFormat::Webm), "WEBM");
}

#[test]
fn test_output_format_mp3_string_varies_by_preset() {
    let state = create_test_state();
    let mut menu = test_menu(&state);

    // Default (not audio only)
    assert_eq!(
        menu.output_format_to_string(&OutputFormat::MP3),
        "MP3 (audio only)"
    );

    // With audio only preset
    menu.settings.format_preset = FormatPreset::AudioOnly;
    assert_eq!(
        menu.output_format_to_string(&OutputFormat::MP3),
        "MP3 (audio)"
    );
}

#[test]
fn test_settings_preset_names() {
    assert_eq!(SettingsPreset::BestQuality.name(), "Best Quality");
    assert_eq!(SettingsPreset::AudioArchive.name(), "Audio Archive");
    assert_eq!(SettingsPreset::FastDownload.name(), "Fast Download");
    assert_eq!(SettingsPreset::BandwidthSaver.name(), "Bandwidth Saver");
}

#[test]
fn test_settings_preset_descriptions() {
    // All presets should have non-empty descriptions
    for preset in SettingsPreset::all() {
        assert!(!preset.description().is_empty());
    }
}

// ==================== Input Handling When Not Visible ====================

#[test]
fn test_handle_input_returns_false_when_not_visible() {
    let state = create_test_state();
    let mut menu = test_menu(&state);

    // Menu is not visible
    let result = menu.handle_input(key_event(KeyCode::Down), &state);

    assert!(!result);
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

    // Navigate to Reset Stats on Batch setting
    menu.list_state.select(Some(IDX_RESET_STATS_ON_BATCH));

    // Initially enabled
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

    // Navigate to Reset Stats on Batch setting
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

// ==================== Menu Layout Tests ====================

#[test]
fn test_total_menu_items_count() {
    // Verify SETTINGS_COUNT and TOTAL_MENU_ITEMS are correct
    assert_eq!(SETTINGS_COUNT, 15);
    assert_eq!(TOTAL_MENU_ITEMS, 17);
    assert_eq!(IDX_APPLY_PRESET, SETTINGS_COUNT);
    assert_eq!(IDX_RESET_DEFAULTS, SETTINGS_COUNT + 1);
}

#[test]
fn test_setting_descriptions_count() {
    assert_eq!(SETTING_DESCRIPTIONS.len(), TOTAL_MENU_ITEMS);
}

// ==================== Download Directory Tests ====================

/// Build a menu opened on the download directory item
fn download_dir_menu(state: &AppState) -> SettingsMenu {
    let mut menu = test_menu(state);
    menu.toggle();
    menu.list_state.select(Some(IDX_DOWNLOAD_DIR));
    menu
}

#[test]
fn test_download_dir_enter_opens_input_mode() {
    let state = create_test_state();
    let mut menu = download_dir_menu(&state);

    menu.handle_input(key_event(KeyCode::Enter), &state);

    assert!(menu.input_mode);
    assert!(!menu.editing);
    assert!(menu.custom_input.is_empty());
}

#[test]
fn test_download_dir_input_accepts_path_characters() {
    let state = create_test_state();
    let mut menu = download_dir_menu(&state);
    menu.handle_input(key_event(KeyCode::Enter), &state);

    for c in "~/My Videos-1".chars() {
        menu.handle_input(key_event(KeyCode::Char(c)), &state);
    }

    assert_eq!(menu.custom_input, "~/My Videos-1");
}

#[test]
fn test_download_dir_saves_valid_path() {
    let temp_dir = std::env::temp_dir().join("auto_ytdlp_menu_download_dir");
    let _ = std::fs::remove_dir_all(&temp_dir);

    let state = create_test_state();
    let mut menu = download_dir_menu(&state);
    menu.handle_input(key_event(KeyCode::Enter), &state);
    menu.custom_input = temp_dir.to_string_lossy().to_string();

    menu.handle_input(key_event(KeyCode::Enter), &state);

    assert!(!menu.input_mode);
    assert!(menu.validation_error.is_none());
    assert_eq!(menu.settings.download_dir, temp_dir.to_string_lossy());
    // Accepted without being created - creation happens at download time
    assert!(!temp_dir.exists());
}

#[test]
fn test_download_dir_rejects_path_with_missing_parents() {
    let missing = std::env::temp_dir().join("auto_ytdlp_no_such_tree/a/b/c");

    let state = create_test_state();
    let mut menu = download_dir_menu(&state);
    menu.handle_input(key_event(KeyCode::Enter), &state);
    menu.custom_input = missing.to_string_lossy().to_string();

    menu.handle_input(key_event(KeyCode::Enter), &state);

    assert!(menu.input_mode);
    assert!(menu.validation_error.is_some());
    assert!(!missing.exists());
}

#[test]
fn test_download_dir_rejects_path_that_is_a_file() {
    let temp_file = std::env::temp_dir().join("auto_ytdlp_menu_not_a_dir.txt");
    std::fs::write(&temp_file, b"not a directory").expect("Failed to create test file");

    let state = create_test_state();
    let mut menu = download_dir_menu(&state);
    menu.handle_input(key_event(KeyCode::Enter), &state);
    menu.custom_input = temp_file.to_string_lossy().to_string();

    menu.handle_input(key_event(KeyCode::Enter), &state);

    // Stays in input mode so the user can correct the path
    assert!(menu.input_mode);
    assert!(menu.validation_error.is_some());
    assert!(menu.settings.download_dir.is_empty());

    let _ = std::fs::remove_file(&temp_file);
}

#[test]
fn test_download_dir_empty_input_clears_setting() {
    let state = create_test_state();
    let mut menu = download_dir_menu(&state);
    menu.settings.download_dir = "/some/dir".to_string();

    menu.handle_input(key_event(KeyCode::Enter), &state);
    menu.custom_input = "   ".to_string();
    menu.handle_input(key_event(KeyCode::Enter), &state);

    assert!(menu.settings.download_dir.is_empty());
    assert!(menu.validation_error.is_none());
}

#[test]
fn test_preset_keeps_download_dir() {
    let state = create_test_state();
    let mut menu = test_menu(&state);
    menu.toggle();
    menu.settings.download_dir = "/media/videos".to_string();

    menu.list_state.select(Some(IDX_APPLY_PRESET));
    menu.handle_input(key_event(KeyCode::Enter), &state);
    menu.handle_input(key_event(KeyCode::Enter), &state); // Apply first preset

    assert_eq!(menu.settings.download_dir, "/media/videos");
}

#[test]
fn test_reset_to_defaults_keeps_download_dir() {
    let state = create_test_state();
    let mut menu = test_menu(&state);
    menu.toggle();
    menu.settings.download_dir = "/media/videos".to_string();
    menu.settings.concurrent_downloads = 8;

    menu.list_state.select(Some(IDX_RESET_DEFAULTS));
    menu.handle_input(key_event(KeyCode::Enter), &state); // Open confirmation
    menu.handle_input(key_event(KeyCode::Char('y')), &state);

    assert_eq!(menu.settings.download_dir, "/media/videos");
    // Everything else is back to defaults
    assert_eq!(
        menu.settings.concurrent_downloads,
        Settings::default().concurrent_downloads
    );
}

#[test]
fn test_preset_persists_to_disk() {
    let state = create_test_state();
    let mut menu = test_menu(&state);
    menu.toggle();

    menu.list_state.select(Some(IDX_APPLY_PRESET));
    menu.handle_input(key_event(KeyCode::Enter), &state);
    menu.preset_index = 1; // Audio Archive
    menu.handle_input(key_event(KeyCode::Enter), &state);

    let saved = Settings::load().expect("Settings should load after applying a preset");
    assert_eq!(saved.format_preset, menu.settings.format_preset);
    assert_eq!(saved.output_format, menu.settings.output_format);
}

// ==================== Display Helper Tests ====================

#[test]
fn test_download_dir_display_reports_cli_override() {
    let state = create_test_state();
    let mut menu = SettingsMenu::new(&state, Some(PathBuf::from("/tmp/scratch")));
    menu.settings.download_dir = "/media/videos".to_string();

    let display = menu.download_dir_display();

    assert!(display.contains("/tmp/scratch"));
    assert!(display.contains("--download-dir"));
    assert!(!display.contains("/media/videos"));
}

#[test]
fn test_download_dir_display_without_override() {
    let state = create_test_state();
    let mut menu = test_menu(&state);
    assert!(menu.download_dir_display().contains("(default)"));

    menu.settings.download_dir = "/media/videos".to_string();
    assert_eq!(menu.download_dir_display(), "/media/videos");
}

#[test]
fn test_truncate_for_display_keeps_requested_end() {
    assert_eq!(
        truncate_for_display("/short/path", 20, Keep::End),
        "/short/path"
    );
    assert_eq!(
        truncate_for_display("/a/very/long/path/to/some/videos", 15, Keep::End),
        ".../some/videos"
    );
    assert_eq!(
        truncate_for_display("--user-agent Some Very Long Agent", 15, Keep::Start),
        "--user-agent..."
    );
}

#[test]
fn test_truncate_for_display_handles_multibyte() {
    // Byte-slicing this at 27 bytes would land mid-character and panic
    let value = "--paths home:/Users/me/Vidéos/dump";

    let truncated = truncate_for_display(value, 30, Keep::Start);

    assert!(truncated.ends_with("..."));
    assert_eq!(truncated.chars().count(), 30);
}
