use super::*;
use crate::app_state::AppState;
use crate::ui::settings_menu::tests::{create_test_state, key_event, test_menu};

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
