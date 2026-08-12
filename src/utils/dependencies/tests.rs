use super::*;

#[test]
#[ignore] // Requires yt-dlp and ffmpeg installed
fn test_check_dependencies_with_both_installed() {
    // This test will only pass if both yt-dlp and ffmpeg are installed
    let result = check_dependencies();
    assert!(
        result.is_ok(),
        "Expected both dependencies to be available, but got: {:?}",
        result.err()
    );
}

#[test]
#[ignore] // Requires yt-dlp and ffmpeg installed
fn test_check_dependencies_returns_empty_on_success() {
    // When both are installed, the result should be Ok(())
    let result = check_dependencies();
    if let Err(errors) = &result {
        // Print what's missing if the test fails
        for error in errors {
            eprintln!("Missing dependency: {}", error);
        }
    }
    assert!(result.is_ok());
}
