use super::table::{SETTINGS, SETTINGS_COUNT};
use super::*;
use crate::app_state::AppState;
use crate::utils::settings::SettingsPreset;
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

// ==================== Concurrent Downloads Wiring ====================

#[test]
fn test_persist_publishes_concurrent_to_shared_state() {
    // process_queue reads the worker count from get_concurrent(), not from the
    // settings struct, so persisting has to update both or the row is inert.
    let state = create_test_state();
    state
        .set_concurrent(4)
        .expect("failed to seed concurrent count");
    let mut menu = test_menu(&state);

    menu.settings.concurrent_downloads = 8;
    menu.persist(&state);

    assert_eq!(
        state.get_concurrent().expect("failed to read concurrent"),
        8
    );
}

#[test]
fn test_applying_a_preset_publishes_its_concurrent_count() {
    let state = create_test_state();
    state
        .set_concurrent(4)
        .expect("failed to seed concurrent count");
    let mut menu = test_menu(&state);

    // Fast Download specifies 8 concurrent downloads
    menu.settings = SettingsPreset::FastDownload.apply(&menu.settings);
    menu.persist(&state);

    assert_eq!(
        state.get_concurrent().expect("failed to read concurrent"),
        8
    );
}

// ==================== Preset Preference Preservation ====================

#[test]
fn test_presets_keep_terminal_and_session_preferences() {
    let current = Settings {
        use_ascii_indicators: true,
        reset_stats_on_new_batch: false,
        download_dir: "/tmp/media".to_string(),
        ..Settings::default()
    };

    for preset in SettingsPreset::all() {
        let applied = preset.apply(&current);
        assert!(
            applied.use_ascii_indicators,
            "{} dropped the ASCII indicator preference",
            preset.name()
        );
        assert!(
            !applied.reset_stats_on_new_batch,
            "{} dropped the cumulative-stats preference",
            preset.name()
        );
        assert_eq!(
            applied.download_dir,
            "/tmp/media",
            "{} dropped the download directory",
            preset.name()
        );
    }
}

#[test]
fn test_presets_still_change_download_behaviour() {
    let current = Settings {
        use_ascii_indicators: true,
        ..Settings::default()
    };
    let applied = SettingsPreset::BandwidthSaver.apply(&current);
    assert_eq!(applied.format_preset, FormatPreset::SD480p);
    assert_eq!(applied.concurrent_downloads, 2);
    assert_eq!(applied.rate_limit, "2M");
}

// ==================== Settings Value Column ====================

#[test]
fn test_every_settings_row_renders_a_value() {
    // The value column is table-driven; a row missing its formatter would show
    // a blank value rather than failing to compile.
    let state = create_test_state();
    let menu = test_menu(&state);

    for (index, setting) in SETTINGS[..SETTINGS_COUNT].iter().enumerate() {
        let render = setting
            .value
            .unwrap_or_else(|| panic!("row {} ({}) has no value fn", index, setting.label));
        assert!(
            !render(&menu).is_empty(),
            "row {} ({}) rendered an empty value",
            index,
            setting.label
        );
    }
}

#[test]
fn test_action_rows_have_no_value_formatter() {
    for setting in &SETTINGS[SETTINGS_COUNT..] {
        assert!(
            setting.value.is_none(),
            "action row {} should not render a value",
            setting.label
        );
    }
}
