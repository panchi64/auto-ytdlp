use super::*;

use ratatui::{Terminal, backend::TestBackend};

#[test]
fn test_help_overlay_fits_a_short_terminal() {
    // Rendering a Rect taller than the frame panics, so the popup must shrink
    let mut terminal =
        Terminal::new(TestBackend::new(80, 24)).expect("failed to build test terminal");
    terminal
        .draw(render_help_overlay)
        .expect("help overlay must fit an 80x24 terminal");

    let mut terminal =
        Terminal::new(TestBackend::new(20, 6)).expect("failed to build test terminal");
    terminal
        .draw(render_help_overlay)
        .expect("help overlay must fit a 20x6 terminal");
}
