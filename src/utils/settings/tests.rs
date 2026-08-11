
use super::*;

#[test]
fn test_settings_default_values() {
    let settings = Settings::default();

    assert_eq!(settings.format_preset, FormatPreset::Best);
    assert_eq!(settings.output_format, OutputFormat::Auto);
    assert!(!settings.write_subtitles);
    assert!(!settings.write_thumbnail);
    assert!(!settings.add_metadata);
    assert!(!settings.sponsorblock);
    assert_eq!(settings.concurrent_downloads, 4);
    assert!(settings.rate_limit.is_empty());
    assert!(!settings.network_retry);
    assert_eq!(settings.retry_delay, 2);
    assert!(settings.cookies_from_browser.is_empty());
    assert!(!settings.use_ascii_indicators);
    assert!(settings.custom_ytdlp_args.is_empty());
    assert!(settings.reset_stats_on_new_batch);
}

#[test]
fn test_format_preset_best() {
    assert_eq!(
        FormatPreset::Best.get_format_arg(),
        "bestvideo*+bestaudio/best"
    );
}

#[test]
fn test_format_preset_audio_only() {
    assert_eq!(FormatPreset::AudioOnly.get_format_arg(), "bestaudio/best");
}

#[test]
fn test_format_preset_hd1080p() {
    assert_eq!(
        FormatPreset::HD1080p.get_format_arg(),
        "bestvideo[height<=1080]+bestaudio/best[height<=1080]"
    );
}

#[test]
fn test_format_preset_hd720p() {
    assert_eq!(
        FormatPreset::HD720p.get_format_arg(),
        "bestvideo[height<=720]+bestaudio/best[height<=720]"
    );
}

#[test]
fn test_format_preset_sd480p() {
    assert_eq!(
        FormatPreset::SD480p.get_format_arg(),
        "bestvideo[height<=480]+bestaudio/best[height<=480]"
    );
}

#[test]
fn test_format_preset_sd360p() {
    assert_eq!(
        FormatPreset::SD360p.get_format_arg(),
        "bestvideo[height<=360]+bestaudio/best[height<=360]"
    );
}

#[test]
fn test_output_format_auto() {
    assert_eq!(OutputFormat::Auto.get_format_modifier(), None);
}

#[test]
fn test_output_format_mp4() {
    assert_eq!(
        OutputFormat::MP4.get_format_modifier(),
        Some("--merge-output-format mp4")
    );
}

#[test]
fn test_output_format_mkv() {
    assert_eq!(
        OutputFormat::Mkv.get_format_modifier(),
        Some("--merge-output-format mkv")
    );
}

#[test]
fn test_output_format_mp3() {
    assert_eq!(
        OutputFormat::MP3.get_format_modifier(),
        Some("--extract-audio --audio-format mp3")
    );
}

#[test]
fn test_output_format_webm() {
    assert_eq!(
        OutputFormat::Webm.get_format_modifier(),
        Some("--merge-output-format webm")
    );
}

#[test]
fn test_validate_custom_args_empty() {
    assert!(Settings::validate_custom_args("").is_ok());
    assert!(Settings::validate_custom_args("   ").is_ok());
}

#[test]
fn test_validate_custom_args_valid() {
    assert!(Settings::validate_custom_args("--no-playlist").is_ok());
    assert!(Settings::validate_custom_args("--limit-rate 1M --retries 5").is_ok());
    assert!(Settings::validate_custom_args("--user-agent 'My Bot'").is_ok());
}

#[test]
fn test_validate_custom_args_conflicting_flags() {
    let result = Settings::validate_custom_args("--download-archive my_archive.txt");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("--download-archive"));

    let result = Settings::validate_custom_args("-o ~/Downloads");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("-o"));

    let result = Settings::validate_custom_args("--output ~/Downloads");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("--output"));

    let result = Settings::validate_custom_args("--progress-template test");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("--progress-template"));
}

#[test]
fn test_validate_custom_args_unmatched_quotes() {
    let result = Settings::validate_custom_args("--user-agent 'unmatched");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("unmatched quotes"));
}

#[test]
fn test_parse_custom_args_empty() {
    let settings = Settings::default();
    assert!(settings.parse_custom_args().is_empty());

    let settings = Settings {
        custom_ytdlp_args: "   ".to_string(),
        ..Default::default()
    };
    assert!(settings.parse_custom_args().is_empty());
}

#[test]
fn test_parse_custom_args_simple() {
    let settings = Settings {
        custom_ytdlp_args: "--no-playlist --retries 5".to_string(),
        ..Default::default()
    };
    let args = settings.parse_custom_args();
    assert_eq!(args, vec!["--no-playlist", "--retries", "5"]);
}

#[test]
fn test_parse_custom_args_quoted() {
    let settings = Settings {
        custom_ytdlp_args: "--user-agent 'My Custom Agent'".to_string(),
        ..Default::default()
    };
    let args = settings.parse_custom_args();
    assert_eq!(args, vec!["--user-agent", "My Custom Agent"]);
}

#[test]
fn test_get_ytdlp_args_basic() {
    let settings = Settings::default();
    let args = settings.get_ytdlp_args("%(title)s.%(ext)s");

    assert!(args.contains(&"--format".to_string()));
    assert!(args.contains(&"bestvideo*+bestaudio/best".to_string()));
    assert!(args.contains(&"--output".to_string()));
    assert!(args.contains(&"%(title)s.%(ext)s".to_string()));
    assert!(args.contains(&"--newline".to_string()));

    // Default settings should not include optional flags
    assert!(!args.contains(&"--write-auto-subs".to_string()));
    assert!(!args.contains(&"--write-thumbnail".to_string()));
    assert!(!args.contains(&"--add-metadata".to_string()));
}

#[test]
fn test_get_ytdlp_args_all_options() {
    let settings = Settings {
        write_subtitles: true,
        write_thumbnail: true,
        add_metadata: true,
        output_format: OutputFormat::MP4,
        custom_ytdlp_args: "--no-playlist".to_string(),
        ..Default::default()
    };

    let args = settings.get_ytdlp_args("%(title)s.%(ext)s");

    assert!(args.contains(&"--write-auto-subs".to_string()));
    assert!(args.contains(&"--sub-langs".to_string()));
    assert!(args.contains(&"all".to_string()));
    assert!(args.contains(&"--write-thumbnail".to_string()));
    assert!(args.contains(&"--add-metadata".to_string()));
    assert!(args.contains(&"--merge-output-format".to_string()));
    assert!(args.contains(&"mp4".to_string()));
    assert!(args.contains(&"--no-playlist".to_string()));
}

#[test]
fn test_preset_best_quality() {
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
fn test_preset_audio_archive() {
    let settings = SettingsPreset::AudioArchive.apply(&Settings::default());
    assert_eq!(settings.format_preset, FormatPreset::AudioOnly);
    assert_eq!(settings.output_format, OutputFormat::MP3);
    assert!(!settings.write_subtitles);
    assert!(settings.write_thumbnail);
    assert!(settings.add_metadata);
}

#[test]
fn test_preset_fast_download() {
    let settings = SettingsPreset::FastDownload.apply(&Settings::default());
    assert_eq!(settings.format_preset, FormatPreset::Best);
    assert!(!settings.write_subtitles);
    assert!(!settings.write_thumbnail);
    assert!(!settings.add_metadata);
    assert_eq!(settings.concurrent_downloads, 8);
    assert!(!settings.network_retry);
}

#[test]
fn test_preset_bandwidth_saver() {
    let settings = SettingsPreset::BandwidthSaver.apply(&Settings::default());
    assert_eq!(settings.format_preset, FormatPreset::SD480p);
    assert!(!settings.write_subtitles);
    assert!(!settings.write_thumbnail);
    assert!(!settings.add_metadata);
    assert_eq!(settings.concurrent_downloads, 2);
    assert!(settings.network_retry);
    assert_eq!(settings.retry_delay, 5);
}

#[test]
fn test_parse_custom_args_malformed_unclosed_single_quote() {
    let settings = Settings {
        custom_ytdlp_args: "--user-agent 'unclosed".to_string(),
        ..Default::default()
    };
    let args = settings.parse_custom_args();
    assert!(args.is_empty());
}

#[test]
fn test_parse_custom_args_malformed_unclosed_double_quote() {
    let settings = Settings {
        custom_ytdlp_args: "--user-agent \"unclosed".to_string(),
        ..Default::default()
    };
    let args = settings.parse_custom_args();
    assert!(args.is_empty());
}

#[test]
fn test_parse_custom_args_malformed_trailing_backslash() {
    let settings = Settings {
        custom_ytdlp_args: "test\\".to_string(),
        ..Default::default()
    };
    let args = settings.parse_custom_args();
    assert!(args.is_empty());
}

#[test]
fn test_parse_custom_args_valid_double_quotes() {
    let settings = Settings {
        custom_ytdlp_args: "--user-agent \"My Custom Agent\"".to_string(),
        ..Default::default()
    };
    let args = settings.parse_custom_args();
    assert_eq!(args, vec!["--user-agent", "My Custom Agent"]);
}

#[test]
fn test_parse_custom_args_multiple_quoted_segments() {
    let settings = Settings {
        custom_ytdlp_args: "--cookies 'path/to/cookies' --user-agent 'Bot'".to_string(),
        ..Default::default()
    };
    let args = settings.parse_custom_args();
    assert_eq!(
        args,
        vec!["--cookies", "path/to/cookies", "--user-agent", "Bot"]
    );
}

// ==================== Rate Limit Tests ====================

#[test]
fn test_rate_limit_default_empty() {
    let settings = Settings::default();
    assert!(settings.rate_limit.is_empty());
}

#[test]
fn test_rate_limit_args_when_set() {
    let settings = Settings {
        rate_limit: "2M".to_string(),
        ..Default::default()
    };
    let args = settings.get_ytdlp_args("%(title)s.%(ext)s");
    assert!(args.contains(&"--rate-limit".to_string()));
    assert!(args.contains(&"2M".to_string()));
}

#[test]
fn test_rate_limit_args_when_empty() {
    let settings = Settings::default();
    let args = settings.get_ytdlp_args("%(title)s.%(ext)s");
    assert!(!args.contains(&"--rate-limit".to_string()));
}

#[test]
fn test_preset_bandwidth_saver_has_rate_limit() {
    let settings = SettingsPreset::BandwidthSaver.apply(&Settings::default());
    assert_eq!(settings.rate_limit, "2M");
}

#[test]
fn test_preset_best_quality_no_rate_limit() {
    let settings = SettingsPreset::BestQuality.apply(&Settings::default());
    assert!(settings.rate_limit.is_empty());
}

// ==================== SponsorBlock Tests ====================

#[test]
fn test_sponsorblock_default_false() {
    let settings = Settings::default();
    assert!(!settings.sponsorblock);
}

#[test]
fn test_sponsorblock_args_when_enabled() {
    let settings = Settings {
        sponsorblock: true,
        ..Default::default()
    };
    let args = settings.get_ytdlp_args("%(title)s.%(ext)s");
    assert!(args.contains(&"--sponsorblock-remove".to_string()));
    assert!(args.contains(&"all".to_string()));
}

#[test]
fn test_sponsorblock_args_when_disabled() {
    let settings = Settings::default();
    let args = settings.get_ytdlp_args("%(title)s.%(ext)s");
    assert!(!args.contains(&"--sponsorblock-remove".to_string()));
}

#[test]
fn test_all_presets_sponsorblock_false() {
    for preset in SettingsPreset::all() {
        let settings = preset.apply(&Settings::default());
        assert!(
            !settings.sponsorblock,
            "Preset {:?} should have sponsorblock = false",
            preset.name()
        );
    }
}

// ==================== Cookies from Browser Tests ====================

#[test]
fn test_cookies_from_browser_default_empty() {
    let settings = Settings::default();
    assert!(settings.cookies_from_browser.is_empty());
}

#[test]
fn test_cookies_from_browser_args_when_set() {
    let settings = Settings {
        cookies_from_browser: "firefox".to_string(),
        ..Default::default()
    };
    let args = settings.get_ytdlp_args("%(title)s.%(ext)s");
    assert!(args.contains(&"--cookies-from-browser".to_string()));
    assert!(args.contains(&"firefox".to_string()));
}

#[test]
fn test_cookies_from_browser_args_when_empty() {
    let settings = Settings::default();
    let args = settings.get_ytdlp_args("%(title)s.%(ext)s");
    assert!(!args.contains(&"--cookies-from-browser".to_string()));
}

// ==================== Download Directory Tests ====================

#[test]
fn test_download_dir_default_empty() {
    let settings = Settings::default();
    assert!(settings.download_dir.is_empty());
    assert_eq!(settings.resolved_download_dir(), None);
}

#[test]
fn test_resolved_download_dir_when_set() {
    assert_eq!(
        settings_with_dir("/media/videos").resolved_download_dir(),
        Some(PathBuf::from("/media/videos"))
    );
}

#[test]
fn test_resolved_download_dir_whitespace_only_is_none() {
    assert_eq!(settings_with_dir("   ").resolved_download_dir(), None);
}

#[test]
fn test_expand_tilde_home_only() {
    let home = dirs::home_dir().expect("Home directory should be available in tests");
    assert_eq!(expand_tilde("~"), Some(home));
}

#[test]
fn test_expand_tilde_with_subdirectory() {
    let home = dirs::home_dir().expect("Home directory should be available in tests");
    assert_eq!(
        expand_tilde("~/Videos/clips"),
        Some(home.join("Videos/clips"))
    );
}

#[test]
fn test_expand_tilde_accepts_platform_separator() {
    // Windows users type "~\Videos"; both separators must expand
    let expanded = expand_tilde(&format!("~{}Videos", std::path::MAIN_SEPARATOR))
        .expect("Home directory should be available in tests");

    assert!(!expanded.to_string_lossy().starts_with('~'));
    assert!(expanded.ends_with("Videos"));
}

#[test]
fn test_expand_tilde_leaves_absolute_paths_alone() {
    assert_eq!(
        expand_tilde("/media/videos"),
        Some(PathBuf::from("/media/videos"))
    );
    assert_eq!(
        expand_tilde("relative/dir"),
        Some(PathBuf::from("relative/dir"))
    );
}

#[test]
fn test_expand_tilde_leaves_other_users_home_alone() {
    // "~user" is not supported and must be passed through unchanged
    assert_eq!(
        expand_tilde("~someone/videos"),
        Some(PathBuf::from("~someone/videos"))
    );
}

#[test]
fn test_validate_download_dir_empty_is_ok() {
    assert!(Settings::validate_download_dir("").is_ok());
    assert!(Settings::validate_download_dir("   ").is_ok());
}

#[test]
fn test_validate_download_dir_accepts_missing_dir_with_existing_parent() {
    let dir = std::env::temp_dir().join("auto_ytdlp_validate_download_dir");
    let _ = fs::remove_dir_all(&dir);

    assert!(Settings::validate_download_dir(&dir.to_string_lossy()).is_ok());
    // Validation must not touch the filesystem
    assert!(!dir.exists());
}

#[test]
fn test_validate_download_dir_rejects_missing_parents() {
    let dir = std::env::temp_dir().join("auto_ytdlp_missing_tree/a/b");
    let _ = fs::remove_dir_all(
        dir.parent()
            .and_then(|p| p.parent())
            .expect("temp path should have grandparent"),
    );

    let result = Settings::validate_download_dir(&dir.to_string_lossy());

    assert!(result.is_err());
    assert!(!dir.exists());
}

#[test]
fn test_validate_download_dir_accepts_existing_dir() {
    let dir = std::env::temp_dir().join("auto_ytdlp_validate_existing");
    fs::create_dir_all(&dir).expect("Failed to create test directory");

    assert!(Settings::validate_download_dir(&dir.to_string_lossy()).is_ok());

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_validate_download_dir_rejects_file() {
    let file = std::env::temp_dir().join("auto_ytdlp_validate_not_a_dir.txt");
    fs::write(&file, b"not a directory").expect("Failed to create test file");

    let result = Settings::validate_download_dir(&file.to_string_lossy());

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not a directory"));

    let _ = fs::remove_file(&file);
}

#[test]
fn test_preset_preserves_download_dir() {
    let current = settings_with_dir("/media/videos");

    for preset in SettingsPreset::all() {
        let settings = preset.apply(&current);
        assert_eq!(
            settings.download_dir,
            "/media/videos",
            "Preset {:?} should keep the configured download directory",
            preset.name()
        );
    }
}

#[test]
fn test_settings_deserialize_without_download_dir() {
    // Settings files written before this option existed must still load
    let json = r#"{
            "format_preset": "Best",
            "output_format": "Auto",
            "write_subtitles": false,
            "write_thumbnail": false,
            "add_metadata": false,
            "concurrent_downloads": 4,
            "network_retry": false,
            "retry_delay": 2
        }"#;

    let settings: Settings =
        serde_json::from_str(json).expect("Legacy settings JSON should deserialize");

    assert!(settings.download_dir.is_empty());
}

#[test]
fn test_all_presets_cookies_empty() {
    for preset in SettingsPreset::all() {
        let settings = preset.apply(&Settings::default());
        assert!(
            settings.cookies_from_browser.is_empty(),
            "Preset {:?} should have cookies_from_browser empty",
            preset.name()
        );
    }
}
