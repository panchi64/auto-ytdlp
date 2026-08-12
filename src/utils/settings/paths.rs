use std::path::PathBuf;

use super::Settings;

/// Expand a leading `~` in a path to the user's home directory
///
/// Paths without a leading `~` are returned unchanged. Returns `None` only when
/// the path starts with `~` but the home directory cannot be determined (for
/// example a service started without HOME) - callers must not fall back to the
/// literal path, or a directory actually named `~` gets created.
///
/// `~user` is not expanded; resolving another user's home directory is not
/// supported, so such a path is returned unchanged.
pub fn expand_tilde(path: &str) -> Option<PathBuf> {
    let trimmed = path.trim();

    let Some(rest) = trimmed.strip_prefix('~') else {
        return Some(PathBuf::from(trimmed));
    };

    // Accept any platform separator after "~" so "~\Videos" works on Windows
    let rest = match rest.chars().next() {
        None => "",
        Some(c) if std::path::is_separator(c) => rest.trim_start_matches(std::path::is_separator),
        Some(_) => return Some(PathBuf::from(trimmed)),
    };

    let home = dirs::home_dir()?;
    Some(if rest.is_empty() {
        home
    } else {
        home.join(rest)
    })
}

impl Settings {
    /// Validate a download directory entered by the user
    ///
    /// An empty value is valid and means "fall back to the default directory".
    /// A non-empty value is expanded (`~`) and checked, but never created: this
    /// runs on the TUI thread, so it must not block on a slow mount or leave a
    /// stray directory behind. The directory is created at download time.
    pub fn validate_download_dir(dir: &str) -> std::result::Result<(), String> {
        if dir.trim().is_empty() {
            return Ok(());
        }

        let Some(path) = expand_tilde(dir) else {
            return Err("Cannot determine your home directory - use a full path".to_string());
        };

        if path.exists() {
            return if path.is_dir() {
                Ok(())
            } else {
                Err(format!(
                    "'{}' exists but is not a directory",
                    path.display()
                ))
            };
        }

        // Not created yet: require the immediate parent to exist, so a typo in
        // the parent path is rejected instead of creating a whole new tree
        // somewhere the user did not mean.
        match path.parent() {
            Some(parent) if parent.is_dir() => Ok(()),
            Some(parent) => Err(format!("'{}' does not exist", parent.display())),
            None => Err(format!("'{}' is not a valid directory", path.display())),
        }
    }

    /// Get the configured download directory, if one is set
    ///
    /// Returns `None` when the setting is empty or is a `~` path that cannot be
    /// expanded, meaning the caller should fall back to the command-line value
    /// or the built-in default.
    pub fn resolved_download_dir(&self) -> Option<PathBuf> {
        if self.download_dir.trim().is_empty() {
            return None;
        }

        let expanded = expand_tilde(&self.download_dir);
        if expanded.is_none() {
            eprintln!(
                "Warning: cannot expand '{}' (no home directory); using the default download directory",
                self.download_dir
            );
        }
        expanded
    }
}
