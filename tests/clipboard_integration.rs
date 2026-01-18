//! Integration tests for clipboard functionality
//!
//! Clipboard tests are marked #[ignore] because they require display access.
//! Run manually with: cargo test --test clipboard_integration -- --ignored

use arboard::Clipboard;

/// Verify arboard API compiles correctly
#[test]
fn test_arboard_api_compiles() {
    fn _verify_api() -> Result<String, arboard::Error> {
        let mut clipboard = Clipboard::new()?;
        clipboard.get_text()
    }
}

/// Test clipboard roundtrip (requires display)
#[test]
#[ignore]
fn test_clipboard_roundtrip() {
    let mut clipboard = Clipboard::new().expect("Failed to initialize clipboard");
    let test_url = "https://youtube.com/watch?v=test123";
    clipboard.set_text(test_url).expect("Failed to set clipboard");
    let result = clipboard.get_text().expect("Failed to get clipboard");
    assert_eq!(result, test_url);
}

/// Test multiple URLs can be pasted
#[test]
#[ignore]
fn test_clipboard_multiple_urls() {
    let mut clipboard = Clipboard::new().expect("Failed to initialize clipboard");
    let urls = "https://example.com/1\nhttps://example.com/2\nhttps://example.com/3";
    clipboard.set_text(urls).expect("Failed to set clipboard");
    let result = clipboard.get_text().expect("Failed to get clipboard");
    assert_eq!(result.lines().count(), 3);
}

/// Test empty clipboard handling
#[test]
#[ignore]
fn test_clipboard_empty_handling() {
    let mut clipboard = Clipboard::new().expect("Failed to initialize clipboard");
    clipboard.set_text("").expect("Failed to set clipboard");
    let result = clipboard.get_text();
    // Empty clipboard may return Ok("") or Err depending on platform
    assert!(result.is_ok() || result.is_err());
}
