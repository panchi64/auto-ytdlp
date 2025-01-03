mod check_deps;
mod download;
mod ui;

use ui::tui::TuiDisplay as tui;

fn main() {
    let _ = tui::init();
}
