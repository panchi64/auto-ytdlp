use crate::app_state::AppState;
use crate::errors::Result;

impl AppState {
    pub fn is_paused(&self) -> Result<bool> {
        let flags = self.flags.lock()?;
        Ok(flags.paused)
    }

    pub fn is_started(&self) -> Result<bool> {
        let flags = self.flags.lock()?;
        Ok(flags.started)
    }

    pub fn is_completed(&self) -> Result<bool> {
        let flags = self.flags.lock()?;
        Ok(flags.completed)
    }

    pub fn is_shutdown(&self) -> Result<bool> {
        let flags = self.flags.lock()?;
        Ok(flags.shutdown)
    }

    pub fn is_force_quit(&self) -> Result<bool> {
        let flags = self.flags.lock()?;
        Ok(flags.force_quit)
    }

    /// Whether the download controller thread is alive.
    ///
    /// Set synchronously rather than through the channel: callers check it
    /// immediately after a keypress, and a queued message would still read
    /// stale.
    pub fn set_controller_active(&self, value: bool) -> Result<()> {
        let mut flags = self.flags.lock()?;
        flags.controller_active = value;
        Ok(())
    }

    pub fn is_controller_active(&self) -> Result<bool> {
        let flags = self.flags.lock()?;
        Ok(flags.controller_active)
    }

    pub fn is_notification_sent(&self) -> Result<bool> {
        let flags = self.flags.lock()?;
        Ok(flags.notification_sent)
    }

    pub fn set_notification_sent(&self, value: bool) -> Result<()> {
        let mut flags = self.flags.lock()?;
        flags.notification_sent = value;
        Ok(())
    }
}
