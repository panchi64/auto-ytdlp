use crate::app_state::AppState;
use crate::errors::Result;

impl AppState {
    pub fn add_log(&self, message: String) -> Result<()> {
        let mut logs = self.logs.lock()?;
        logs.push_back(message);
        // Keep only the latest 1000 log messages (O(1) operation with VecDeque)
        while logs.len() > 1000 {
            logs.pop_front();
        }
        Ok(())
    }

    /// Logs an error message to the TUI logs with context.
    ///
    /// This method formats the error with context and adds it to the visible logs,
    /// making errors visible in the TUI rather than just printing to stderr.
    pub fn log_error(&self, context: &str, error: impl std::fmt::Display) -> Result<()> {
        self.add_log(format!("[ERROR] {}: {}", context, error))
    }

    pub fn clear_logs(&self) -> Result<()> {
        let mut logs = self.logs.lock()?;
        logs.clear();
        logs.push_back("Logs cleared".to_string());
        Ok(())
    }
}
