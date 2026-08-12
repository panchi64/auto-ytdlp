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
