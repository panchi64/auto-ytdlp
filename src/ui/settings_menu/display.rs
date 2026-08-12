use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
    widgets::ListItem,
};

use super::SettingsMenu;
use crate::utils::settings::{FormatPreset, OutputFormat};

pub(super) fn create_setting_item<'a>(name: &'a str, value: &'a str) -> ListItem<'a> {
    let style = Style::default().fg(Color::White);
    let value_style = Style::default().fg(Color::Yellow);
    ListItem::new(Line::from(vec![
        Span::styled(name, style),
        Span::raw(": "),
        Span::styled(value, value_style),
    ]))
}

pub(super) fn bool_to_yes_no(value: bool) -> &'static str {
    if value { "Yes" } else { "No" }
}

/// Create an action item (Apply Preset, Reset, etc.)
pub(super) fn create_action_item(name: &str) -> ListItem<'_> {
    ListItem::new(Line::from(vec![Span::styled(
        name,
        Style::default().fg(Color::Cyan),
    )]))
}

/// Which end of an over-long value to keep when shortening it for display
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Keep {
    /// Keep the beginning (argument lists read from the left)
    Start,
    /// Keep the end (the most specific part of a path)
    End,
}

/// Shorten a value for display, never splitting a multi-byte character
pub(super) fn truncate_for_display(text: &str, max_chars: usize, keep: Keep) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= max_chars {
        return text.to_string();
    }

    let budget = max_chars.saturating_sub(3);
    match keep {
        Keep::Start => format!("{}...", chars[..budget].iter().collect::<String>()),
        Keep::End => format!(
            "...{}",
            chars[chars.len() - budget..].iter().collect::<String>()
        ),
    }
}

impl SettingsMenu {
    /// Convert format preset to display string
    pub(super) fn format_preset_to_string(&self, preset: &FormatPreset) -> &'static str {
        match preset {
            FormatPreset::Best => "Best",
            FormatPreset::AudioOnly => "Audio Only",
            FormatPreset::HD1080p => "1080p",
            FormatPreset::HD720p => "720p",
            FormatPreset::SD480p => "480p",
            FormatPreset::SD360p => "360p",
        }
    }

    /// Convert output format to display string
    pub(super) fn output_format_to_string(&self, format: &OutputFormat) -> &'static str {
        match format {
            OutputFormat::Auto => "Auto",
            OutputFormat::MP4 => "MP4",
            OutputFormat::Mkv => "MKV",
            OutputFormat::MP3 => {
                if self.is_audio_only() {
                    "MP3 (audio)"
                } else {
                    "MP3 (audio only)"
                }
            }
            OutputFormat::Webm => "WEBM",
        }
    }
}

#[cfg(test)]
mod tests;
