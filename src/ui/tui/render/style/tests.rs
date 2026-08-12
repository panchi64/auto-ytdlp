use super::*;

// ========== New Link Flash Tests ==========

#[test]
fn test_flash_style_fades_through_the_shades() {
    assert_eq!(flash_style(0.0, false).bg, Some(FLASH_SHADES[0]));
    assert_eq!(flash_style(0.5, false).bg, Some(FLASH_SHADES[1]));
    assert_eq!(flash_style(0.9, false).bg, Some(FLASH_SHADES[2]));
}

#[test]
fn test_flash_style_uses_basic_ansi_in_compatibility_mode() {
    // Truecolor shades would not survive a terminal that needs ASCII indicators
    for progress in [0.0, 0.5, 0.9] {
        let style = flash_style(progress, true);
        assert_eq!(style.bg, Some(Color::Green));
        assert_eq!(style.fg, Some(Color::Black));
    }
}

#[test]
fn test_flash_style_clamps_at_the_dimmest_shade() {
    assert_eq!(flash_style(1.0, false).bg, Some(FLASH_SHADES[2]));
    assert_eq!(flash_style(5.0, false).bg, Some(FLASH_SHADES[2]));
}

// ========== Log Wrapping Height Calculation Tests ==========

#[test]
fn test_wrapped_height_empty_list() {
    let lines: Vec<String> = vec![];
    assert_eq!(calculate_wrapped_height(&lines, 80), 0);
}

#[test]
fn test_wrapped_height_empty_lines_count_as_one() {
    let lines = vec!["".to_string(), "".to_string()];
    assert_eq!(calculate_wrapped_height(&lines, 80), 2);
}

#[test]
fn test_wrapped_height_short_lines_no_wrap() {
    let lines = vec![
        "Short line".to_string(),
        "Another short".to_string(),
        "Third".to_string(),
    ];
    // All lines are under 80 chars, so 3 total lines
    assert_eq!(calculate_wrapped_height(&lines, 80), 3);
}

#[test]
fn test_wrapped_height_exact_width_no_wrap() {
    // A line exactly at the width should not wrap
    let line = "x".repeat(40);
    let lines = vec![line];
    assert_eq!(calculate_wrapped_height(&lines, 40), 1);
}

#[test]
fn test_wrapped_height_line_wraps_once() {
    // A line of 50 chars in 40 width should wrap to 2 lines
    let line = "x".repeat(50);
    let lines = vec![line];
    assert_eq!(calculate_wrapped_height(&lines, 40), 2);
}

#[test]
fn test_wrapped_height_line_wraps_multiple_times() {
    // A line of 100 chars in 40 width should wrap to 3 lines (40+40+20)
    let line = "x".repeat(100);
    let lines = vec![line];
    assert_eq!(calculate_wrapped_height(&lines, 40), 3);
}

#[test]
fn test_wrapped_height_mixed_line_lengths() {
    let lines = vec![
        "Short".to_string(), // 1 line
        "x".repeat(50),      // 2 lines (50 / 40 = 2)
        "".to_string(),      // 1 line
        "x".repeat(100),     // 3 lines (100 / 40 = 3)
    ];
    // Total: 1 + 2 + 1 + 3 = 7 lines
    assert_eq!(calculate_wrapped_height(&lines, 40), 7);
}

#[test]
fn test_wrapped_height_zero_width_returns_line_count() {
    // Edge case: zero width should return number of lines to avoid division by zero
    let lines = vec!["test".to_string(), "lines".to_string()];
    assert_eq!(calculate_wrapped_height(&lines, 0), 2);
}

#[test]
fn test_wrapped_height_realistic_error_message() {
    // Simulate a realistic error message that might overflow
    let error_msg = "[ERROR] Failed to download: https://www.youtube.com/watch?v=very_long_video_id_here - Connection timeout after 30 seconds".to_string();
    let lines = vec![error_msg.clone()];

    // This 121-char message in a 60-char terminal wraps to 3 lines (60+60+1)
    let height = calculate_wrapped_height(&lines, 60);
    assert_eq!(height, 3);

    // Verify the math: chars.div_ceil(width)
    assert_eq!(error_msg.chars().count().div_ceil(60), 3);
}

#[test]
fn test_wrapped_height_unicode_characters() {
    // Widths are measured in display cells, matching what the renderer does
    let line = "🎵".repeat(10); // 10 double-width emoji = 20 cells
    let lines = vec![line];
    // 20 cells in 5-cell width = 4 lines
    assert_eq!(calculate_wrapped_height(&lines, 5), 4);
}

#[test]
fn test_wrapped_height_breaks_on_word_boundaries() {
    // Word wrapping pushes a word that doesn't fit onto the next row, so the
    // height exceeds the naive ceil(chars / width) estimate of 2.
    let lines = vec!["Starting download: https://www.youtube.com/watch?v=abcdefghij".to_string()];
    assert_eq!(calculate_wrapped_height(&lines, 40), 3);
}

#[test]
fn test_wrapped_height_single_char_width() {
    // Edge case: width of 1 means each character is its own line
    let line = "abc".to_string();
    let lines = vec![line];
    assert_eq!(calculate_wrapped_height(&lines, 1), 3);
}

#[test]
fn test_wrapped_height_many_log_entries() {
    // Simulate a log with many entries of varying lengths
    let lines: Vec<String> = (0..100)
        .map(|i| format!("Log entry {} with some additional text", i))
        .collect();

    let height = calculate_wrapped_height(&lines, 80);
    // Each line is under 80 chars, so should be exactly 100
    assert_eq!(height, 100);
}

// ========== Display Name Truncation Tests ==========

#[test]
fn test_truncate_display_name_short_ascii() {
    let result = truncate_display_name("short.mp4", 20);
    assert_eq!(result, "short.mp4");
}

#[test]
fn test_truncate_display_name_exact_fit() {
    let name = "x".repeat(20);
    let result = truncate_display_name(&name, 20);
    assert_eq!(result, name);
}

#[test]
fn test_truncate_display_name_long_ascii() {
    let name = "a".repeat(30);
    let result = truncate_display_name(&name, 20);
    assert!(result.ends_with("..."));
    assert!(result.chars().count() <= 20);
}

#[test]
fn test_truncate_display_name_unicode_no_truncation() {
    let name = "動画テスト";
    let result = truncate_display_name(name, 20);
    assert_eq!(result, name);
}

#[test]
fn test_truncate_display_name_unicode_truncation() {
    // 20 CJK characters, truncate to 10
    let name = "動画テストファイル名前動画テストファイル名前";
    let result = truncate_display_name(name, 10);
    assert!(result.ends_with("..."));
    assert!(result.chars().count() <= 10);
}

#[test]
fn test_truncate_display_name_emoji() {
    let name = "🎵🎶🎧🎤🎸🎹🎺🎻🥁🎼🎵🎶🎧🎤🎸";
    let result = truncate_display_name(name, 10);
    assert!(result.ends_with("..."));
    assert!(result.chars().count() <= 10);
}

#[test]
fn test_truncate_display_name_mixed_ascii_and_unicode() {
    let name = "Video - 日本語のタイトル - Episode 01";
    let result = truncate_display_name(name, 15);
    assert!(result.ends_with("..."));
    assert!(result.chars().count() <= 15);
}

#[test]
fn test_truncate_display_name_empty() {
    let result = truncate_display_name("", 20);
    assert_eq!(result, "");
}

#[test]
fn test_truncate_display_name_zero_max_len() {
    let result = truncate_display_name("test", 0);
    assert!(result.ends_with("..."));
}
