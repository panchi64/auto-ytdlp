use super::*;
use crate::ui::settings_menu::tests::{create_test_state, key_event, test_menu};

// ==================== Navigation Tests ====================

#[test]
fn test_settings_menu_navigation_down() {
    let state = create_test_state();
    let mut menu = test_menu(&state);
    menu.toggle();

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

    menu.handle_input(key_event(KeyCode::Up), &state);

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

// ==================== Input Handling When Not Visible ====================

#[test]
fn test_handle_input_returns_false_when_not_visible() {
    let state = create_test_state();
    let mut menu = test_menu(&state);

    // Menu is not visible
    let result = menu.handle_input(key_event(KeyCode::Down), &state);

    assert!(!result);
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
