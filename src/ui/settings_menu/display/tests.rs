use super::*;
use crate::ui::settings_menu::tests::{create_test_state, test_menu};
use crate::utils::settings::SettingsPreset;

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

// ==================== Truncation Tests ====================

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
