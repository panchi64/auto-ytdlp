use super::*;

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
    assert_eq!(SETTINGS.len(), TOTAL_MENU_ITEMS);
}
