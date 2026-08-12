use super::*;
use crate::app_state::AppState;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

// Helper to create a KeyEvent
pub(super) fn key_event(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

// Helper to create AppState for testing with default settings
// (avoids depending on the settings file on disk, which may be
// modified by other tests that call settings.save())
pub(super) fn create_test_state() -> AppState {
    let state = AppState::new();
    state
        .update_settings(Settings::default())
        .expect("Failed to reset settings for test");
    state
}

// Helper to create a menu with no --download-dir override
pub(super) fn test_menu(state: &AppState) -> SettingsMenu {
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

// ==================== Download Directory Display Tests ====================

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
