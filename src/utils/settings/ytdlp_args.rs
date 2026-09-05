use super::Settings;

/// Flags that conflict with custom yt-dlp arguments
const CONFLICTING_FLAGS: &[&str] = &[
    "--download-archive",
    "-a",
    "--output",
    "-o",
    "--progress-template",
];

impl Settings {
    /// Validate custom yt-dlp arguments for conflicts
    ///
    /// Returns Ok(()) if valid, or Err with a description of the conflict.
    pub fn validate_custom_args(args: &str) -> std::result::Result<(), String> {
        if args.trim().is_empty() {
            return Ok(());
        }

        // shlex handles quoted arguments the way a shell would
        let parsed = match shlex::split(args) {
            Some(args) => args,
            None => return Err("Invalid argument syntax (unmatched quotes)".to_string()),
        };

        for arg in &parsed {
            for conflict in CONFLICTING_FLAGS {
                if arg == *conflict || arg.starts_with(&format!("{}=", conflict)) {
                    return Err(format!(
                        "'{}' conflicts with auto-ytdlp's internal handling",
                        conflict
                    ));
                }
            }
        }

        Ok(())
    }

    /// Parse custom arguments into a vector of strings
    ///
    /// Returns an empty vector if parsing fails or args is empty.
    pub fn parse_custom_args(&self) -> Vec<String> {
        if self.custom_ytdlp_args.trim().is_empty() {
            return Vec::new();
        }

        match shlex::split(&self.custom_ytdlp_args) {
            Some(args) => args,
            None => {
                eprintln!(
                    "Warning: custom yt-dlp args have malformed shell syntax (e.g., unclosed quotes): {}",
                    self.custom_ytdlp_args
                );
                Vec::new()
            }
        }
    }

    /// Build the yt-dlp command arguments based on current settings
    pub fn get_ytdlp_args(&self, output_template: &str) -> Vec<String> {
        // 4 base args plus at most 10 from the optional settings below
        let mut args = Vec::with_capacity(14);

        args.push("--format".to_string());
        args.push(self.format_preset.get_format_arg().to_string());
        args.push("--output".to_string());
        args.push(output_template.to_string());

        if let Some(format_modifier) = self.output_format.get_format_modifier() {
            for modifier in format_modifier.split_whitespace() {
                args.push(modifier.to_string());
            }
        }

        if self.write_subtitles {
            args.push("--write-auto-subs".to_string());
            args.push("--sub-langs".to_string());
            args.push("all".to_string());
        }

        if self.write_thumbnail {
            args.push("--write-thumbnail".to_string());
        }

        if self.add_metadata {
            args.push("--add-metadata".to_string());
        }

        if self.sponsorblock {
            args.push("--sponsorblock-remove".to_string());
            args.push("all".to_string());
        }

        if !self.rate_limit.is_empty() {
            args.push("--rate-limit".to_string());
            args.push(self.rate_limit.clone());
        }

        if !self.cookies_from_browser.is_empty() {
            args.push("--cookies-from-browser".to_string());
            args.push(self.cookies_from_browser.clone());
        }

        // Line-buffered output is what the progress parser reads
        args.push("--newline".to_string());

        args.extend(self.parse_custom_args());

        args
    }
}
