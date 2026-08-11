
use super::*;
use crate::utils::settings::settings_with_dir;
use clap::Parser;

#[test]
fn test_default_values() {
    // Parse with no arguments (just the program name)
    let args = Args::parse_from(["test"]);

    assert!(!args.auto);
    assert_eq!(args.concurrent, 4);
    assert_eq!(args.download_dir, None);
    assert_eq!(args.archive_file, PathBuf::from("./download_archive.txt"));
}

#[test]
fn test_auto_flag_short() {
    let args = Args::parse_from(["test", "-a"]);
    assert!(args.auto);
}

#[test]
fn test_auto_flag_long() {
    let args = Args::parse_from(["test", "--auto"]);
    assert!(args.auto);
}

#[test]
fn test_concurrent_flag_short() {
    let args = Args::parse_from(["test", "-c", "8"]);
    assert_eq!(args.concurrent, 8);
}

#[test]
fn test_concurrent_flag_long() {
    let args = Args::parse_from(["test", "--concurrent", "16"]);
    assert_eq!(args.concurrent, 16);
}

#[test]
fn test_download_dir_flag_short() {
    let args = Args::parse_from(["test", "-d", "/tmp/downloads"]);
    assert_eq!(args.download_dir, Some(PathBuf::from("/tmp/downloads")));
}

#[test]
fn test_download_dir_flag_long() {
    let args = Args::parse_from(["test", "--download-dir", "/home/user/videos"]);
    assert_eq!(args.download_dir, Some(PathBuf::from("/home/user/videos")));
}

#[test]
fn test_resolve_download_dir_falls_back_to_default() {
    let args = Args::parse_from(["test"]);
    let settings = Settings::default();

    assert_eq!(
        args.resolve_download_dir(&settings),
        PathBuf::from(DEFAULT_DOWNLOAD_DIR)
    );
}

#[test]
fn test_resolve_download_dir_uses_setting() {
    let args = Args::parse_from(["test"]);

    assert_eq!(
        args.resolve_download_dir(&settings_with_dir("/media/videos")),
        PathBuf::from("/media/videos")
    );
}

#[test]
fn test_resolve_download_dir_flag_overrides_setting() {
    let args = Args::parse_from(["test", "-d", "/tmp/downloads"]);

    assert_eq!(
        args.resolve_download_dir(&settings_with_dir("/media/videos")),
        PathBuf::from("/tmp/downloads")
    );
}

#[test]
fn test_resolve_download_dir_expands_tilde_from_setting() {
    let args = Args::parse_from(["test"]);

    let resolved = args.resolve_download_dir(&settings_with_dir("~/videos"));

    assert!(!resolved.to_string_lossy().starts_with('~'));
    assert!(resolved.ends_with("videos"));
}

#[test]
fn test_output_template_uses_setting_directory() {
    let args = Args::parse_from(["test"]);

    let template = args.output_template(&settings_with_dir("/media/videos"));

    assert!(template.starts_with("/media/videos"));
    assert!(template.contains("%(title)s"));
    assert!(template.contains("%(id)s"));
    assert!(template.contains("%(ext)s"));
}

#[test]
fn test_archive_file_flag_short() {
    let args = Args::parse_from(["test", "-f", "/tmp/archive.txt"]);
    assert_eq!(args.archive_file, PathBuf::from("/tmp/archive.txt"));
}

#[test]
fn test_archive_file_flag_long() {
    let args = Args::parse_from(["test", "--archive-file", "/home/user/archive.txt"]);
    assert_eq!(args.archive_file, PathBuf::from("/home/user/archive.txt"));
}

#[test]
fn test_output_template_uses_flag_directory() {
    let args = Args::parse_from(["test", "-d", "/my/downloads"]);
    let settings = Settings::default();

    let template = args.output_template(&settings);

    assert!(template.starts_with("/my/downloads"));
    assert!(template.contains("%(title)s"));
    assert!(template.contains("%(id)s"));
    assert!(template.contains("%(ext)s"));
}

#[test]
fn test_combined_flags() {
    let args = Args::parse_from([
        "test",
        "--auto",
        "-c",
        "12",
        "-d",
        "/downloads",
        "-f",
        "/archive.txt",
    ]);

    assert!(args.auto);
    assert_eq!(args.concurrent, 12);
    assert_eq!(args.download_dir, Some(PathBuf::from("/downloads")));
    assert_eq!(args.archive_file, PathBuf::from("/archive.txt"));
}

#[test]
fn test_args_clone() {
    let args = Args::parse_from(["test", "--auto", "-c", "6"]);
    let cloned = args.clone();

    assert_eq!(cloned.auto, args.auto);
    assert_eq!(cloned.concurrent, args.concurrent);
    assert_eq!(cloned.download_dir, args.download_dir);
    assert_eq!(cloned.archive_file, args.archive_file);
}
