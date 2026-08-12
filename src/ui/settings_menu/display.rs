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

/// Value column renderers, one per settings row.
///
/// These hang off the descriptor table so a new setting brings its own
/// formatter instead of being appended to a positional array that has to stay
/// aligned by hand.
pub(super) mod values {
    use super::{Keep, SettingsMenu, bool_to_yes_no, truncate_for_display};

    pub(in crate::ui::settings_menu) fn format_preset(menu: &SettingsMenu) -> String {
        menu.format_preset_to_string(&menu.settings.format_preset)
            .to_string()
    }

    pub(in crate::ui::settings_menu) fn output_format(menu: &SettingsMenu) -> String {
        menu.output_format_to_string(&menu.settings.output_format)
            .to_string()
    }

    pub(in crate::ui::settings_menu) fn download_dir(menu: &SettingsMenu) -> String {
        menu.download_dir_display()
    }

    pub(in crate::ui::settings_menu) fn write_subtitles(menu: &SettingsMenu) -> String {
        bool_to_yes_no(menu.settings.write_subtitles).to_string()
    }

    pub(in crate::ui::settings_menu) fn write_thumbnail(menu: &SettingsMenu) -> String {
        bool_to_yes_no(menu.settings.write_thumbnail).to_string()
    }

    pub(in crate::ui::settings_menu) fn add_metadata(menu: &SettingsMenu) -> String {
        bool_to_yes_no(menu.settings.add_metadata).to_string()
    }

    pub(in crate::ui::settings_menu) fn sponsorblock(menu: &SettingsMenu) -> String {
        bool_to_yes_no(menu.settings.sponsorblock).to_string()
    }

    pub(in crate::ui::settings_menu) fn concurrent(menu: &SettingsMenu) -> String {
        menu.settings.concurrent_downloads.to_string()
    }

    pub(in crate::ui::settings_menu) fn rate_limit(menu: &SettingsMenu) -> String {
        if menu.settings.rate_limit.is_empty() {
            "Unlimited".to_string()
        } else {
            menu.settings.rate_limit.clone()
        }
    }

    pub(in crate::ui::settings_menu) fn network_retry(menu: &SettingsMenu) -> String {
        bool_to_yes_no(menu.settings.network_retry).to_string()
    }

    pub(in crate::ui::settings_menu) fn retry_delay(menu: &SettingsMenu) -> String {
        format!("{} seconds", menu.settings.retry_delay)
    }

    pub(in crate::ui::settings_menu) fn cookies_browser(menu: &SettingsMenu) -> String {
        let browser = &menu.settings.cookies_from_browser;
        let mut chars = browser.chars();
        match chars.next() {
            None => "None".to_string(),
            Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        }
    }

    pub(in crate::ui::settings_menu) fn ascii_indicators(menu: &SettingsMenu) -> String {
        bool_to_yes_no(menu.settings.use_ascii_indicators).to_string()
    }

    pub(in crate::ui::settings_menu) fn reset_stats(menu: &SettingsMenu) -> String {
        bool_to_yes_no(menu.settings.reset_stats_on_new_batch).to_string()
    }

    pub(in crate::ui::settings_menu) fn custom_args(menu: &SettingsMenu) -> String {
        if menu.settings.custom_ytdlp_args.is_empty() {
            "(none)".to_string()
        } else {
            truncate_for_display(&menu.settings.custom_ytdlp_args, 30, Keep::Start)
        }
    }
}

#[cfg(test)]
mod tests;
