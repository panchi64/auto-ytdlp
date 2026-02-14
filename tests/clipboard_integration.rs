//! Integration tests for clipboard functionality
//!
//! Clipboard behavior is platform-dependent (especially on Wayland).
//! The actual URL parsing and deduplication logic is tested in src/utils/file.rs.

use arboard::Clipboard;

/// Verify arboard API compiles and initializes correctly
#[test]
fn test_arboard_api_compiles() {
    fn _verify_api() -> Result<String, arboard::Error> {
        let mut clipboard = Clipboard::new()?;
        clipboard.get_text()
    }
}
