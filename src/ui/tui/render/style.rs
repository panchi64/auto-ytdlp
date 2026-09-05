use ratatui::{
    style::{Color, Style},
    text::Span,
};

/// Background shades a freshly added link fades through, brightest first.
pub(super) const FLASH_SHADES: [Color; 3] = [
    Color::Rgb(0, 84, 46),
    Color::Rgb(0, 56, 31),
    Color::Rgb(0, 30, 17),
];

/// Style for a link partway through its "just added" flash.
///
/// The fade needs truecolor, which is the same capability gap the ASCII indicator
/// setting exists for, so compatibility mode gets a flat basic-ANSI highlight.
pub(super) fn flash_style(progress: f32, use_ascii: bool) -> Style {
    if use_ascii {
        return Style::default().fg(Color::Black).bg(Color::Green);
    }

    let step = ((progress * FLASH_SHADES.len() as f32) as usize).min(FLASH_SHADES.len() - 1);
    Style::default().fg(Color::White).bg(FLASH_SHADES[step])
}

/// Format bytes into human-readable string (e.g., "1.5MiB")
pub(super) fn format_bytes(bytes: u64) -> String {
    const KIB: f64 = 1024.0;
    const MIB: f64 = KIB * 1024.0;
    const GIB: f64 = MIB * 1024.0;

    let bytes_f = bytes as f64;
    if bytes_f >= GIB {
        format!("{:.1}GiB", bytes_f / GIB)
    } else if bytes_f >= MIB {
        format!("{:.1}MiB", bytes_f / MIB)
    } else if bytes_f >= KIB {
        format!("{:.1}KiB", bytes_f / KIB)
    } else {
        format!("{}B", bytes)
    }
}

/// Truncates a display name to fit within a maximum character width.
///
/// Uses char-aware truncation to avoid panics on multi-byte UTF-8 strings.
/// Appends "..." when truncation occurs.
pub(super) fn truncate_display_name(name: &str, max_len: usize) -> String {
    let char_count = name.chars().count();
    if char_count > max_len {
        let truncated: String = name.chars().take(max_len.saturating_sub(3)).collect();
        format!("{}...", truncated)
    } else {
        name.to_string()
    }
}

/// Calculate the number of rows a single line occupies once wrapped.
///
/// Mirrors ratatui's `Wrap { trim: true }`: text breaks at whitespace, words
/// longer than the row are split, and widths are measured in display cells
/// (so double-width glyphs count as two).
fn wrapped_line_height(line: &str, available_width: usize) -> u16 {
    let mut rows: u16 = 1;
    let mut used = 0usize;

    for word in line.split_whitespace() {
        let width = Span::raw(word).width();

        if used > 0 {
            if used + 1 + width <= available_width {
                used += 1 + width;
                continue;
            }
            // Doesn't fit after the current content: start a new row.
            rows = rows.saturating_add(1);
        }

        // A word wider than the row spills over onto further rows.
        let overflow_rows = width.saturating_sub(1) / available_width;
        rows = rows.saturating_add(overflow_rows as u16);
        used = width - overflow_rows * available_width;
    }

    rows
}

/// Calculate the total height needed to render wrapped lines.
///
/// Accounts for text wrapping when lines exceed the available width.
pub(super) fn calculate_wrapped_height(lines: &[String], available_width: usize) -> u16 {
    if available_width == 0 {
        return lines.len() as u16;
    }
    lines
        .iter()
        .map(|line| wrapped_line_height(line, available_width))
        .sum()
}

#[cfg(test)]
mod tests;
