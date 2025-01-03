//! Contains the logic for the Terminal User Interface, including:
//! - State Handling
//! - Different Views
//! - Keyboard Press Management
use color_eyre::Result;
use crossterm::event::{self, Event};
use ratatui::{DefaultTerminal, Frame};

#[derive(Debug, Default)]
pub struct TuiDisplay {
    exit: bool,
    url_list: Vec<String>,
    terminal_out: Vec<String>,
    first_open: bool.
    // curr_line: Arc<Mutex<Instant>>,
}
impl TuiDisplay {
    fn run_setup(mut terminal: DefaultTerminal) -> Result<()> {
        loop {
            terminal.draw(Self::setup_ui)?;
            if matches!(event::read()?, Event::Key(_)) {
                break Ok(());
            }
        }
    }

    pub fn init() -> Result<()> {
        let terminal = ratatui::init();
        let result = Self::run_setup(terminal);
        ratatui::restore();
        result
    }

    fn setup_ui(frame: &mut Frame) {
        frame.render_widget("hello world", frame.area());
    }
    // pub fn show_main() {}
    // pub fn show_settings() {}
}
