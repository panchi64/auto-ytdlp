use ratatui::{
    Frame,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, LineGauge, Paragraph},
};

use crate::app_state::DownloadProgress;

use super::super::style::{format_bytes, truncate_display_name};

/// Render active downloads with per-download progress bars
pub(in crate::ui::tui::render) fn render_active_downloads(
    frame: &mut Frame,
    area: ratatui::layout::Rect,
    downloads: &[DownloadProgress],
    concurrent: usize,
    use_ascii: bool,
    started: bool,
) {
    // Build title with status icon
    let active_icon = if use_ascii {
        if downloads.is_empty() {
            if started { "[WAIT]" } else { "[STOP]" }
        } else {
            "[DL]"
        }
    } else if downloads.is_empty() {
        if started { "⏸️" } else { "⏹️" }
    } else {
        "⏳"
    };
    let active_title = format!(
        "{} Active Downloads - {}/{}",
        active_icon,
        downloads.len(),
        concurrent
    );

    let block = Block::default().title(active_title).borders(Borders::ALL);
    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    if downloads.is_empty() {
        // Show placeholder when no active downloads
        let placeholder = if started {
            "Waiting for downloads..."
        } else {
            "Press S to start downloads"
        };
        let placeholder_widget =
            Paragraph::new(placeholder).style(Style::default().fg(Color::DarkGray));
        frame.render_widget(placeholder_widget, inner_area);
        return;
    }

    // Calculate how many downloads we can show (2 lines per download)
    let max_visible = (inner_area.height as usize) / 2;
    let visible_downloads = downloads.len().min(max_visible);
    let overflow = downloads.len().saturating_sub(max_visible);

    // Create layout for visible downloads
    let mut constraints = Vec::with_capacity(visible_downloads + if overflow > 0 { 1 } else { 0 });
    for _ in 0..visible_downloads {
        constraints.push(ratatui::layout::Constraint::Length(2));
    }
    if overflow > 0 {
        constraints.push(ratatui::layout::Constraint::Length(1));
    }

    let download_layout = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints(constraints)
        .split(inner_area);

    // Render each visible download
    for (i, dl) in downloads.iter().take(visible_downloads).enumerate() {
        render_single_download_progress(frame, download_layout[i], dl, use_ascii);
    }

    // Show overflow indicator if needed
    if overflow > 0 {
        let overflow_text = format!("+{} more...", overflow);
        let overflow_widget =
            Paragraph::new(overflow_text).style(Style::default().fg(Color::DarkGray));
        frame.render_widget(overflow_widget, download_layout[visible_downloads]);
    }
}

/// Render a single download's progress
fn render_single_download_progress(
    frame: &mut Frame,
    area: ratatui::layout::Rect,
    download: &DownloadProgress,
    use_ascii: bool,
) {
    // Determine color based on phase and staleness
    let is_stale = download.last_update.elapsed().as_secs() > 30;
    let color = if is_stale {
        Color::DarkGray
    } else {
        match download.phase.as_str() {
            "downloading" => Color::Blue,
            "processing" | "merging" => Color::Yellow,
            "finished" => Color::Green,
            "error" => Color::Red,
            _ => Color::Cyan,
        }
    };

    // Split area into two lines: progress bar and info
    let layout = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            ratatui::layout::Constraint::Length(1),
            ratatui::layout::Constraint::Length(1),
        ])
        .split(area);

    // Line 1: Progress bar with LineGauge
    let ratio = (download.percent / 100.0).clamp(0.0, 1.0);

    // Build progress label
    let progress_label = if let (Some(frag_idx), Some(frag_count)) =
        (download.fragment_index, download.fragment_count)
    {
        // Show fragment progress for HLS/DASH
        format!("frag {}/{}", frag_idx, frag_count)
    } else {
        format!("{:.1}%", download.percent)
    };

    let line_gauge = LineGauge::default()
        .ratio(ratio)
        .label(progress_label)
        .filled_style(Style::default().fg(color).add_modifier(Modifier::BOLD))
        .unfilled_style(Style::default().fg(Color::DarkGray));

    frame.render_widget(line_gauge, layout[0]);

    // Line 2: Info line with display name, speed, ETA
    let mut info_parts: Vec<Span> = Vec::with_capacity(4);

    // Display name (truncated if needed, char-aware to avoid UTF-8 panics)
    let max_name_len = (area.width as usize).saturating_sub(25);
    let display_name = truncate_display_name(&download.display_name, max_name_len);
    info_parts.push(Span::styled(display_name, Style::default().fg(color)));

    // Size info (downloaded/total)
    if let Some(total) = download.total_bytes {
        let downloaded = download.downloaded_bytes.unwrap_or(0);
        info_parts.push(Span::raw(" "));
        info_parts.push(Span::styled(
            format!("{}/{}", format_bytes(downloaded), format_bytes(total)),
            Style::default().fg(Color::White),
        ));
    }

    // Speed
    if let Some(ref speed) = download.speed {
        info_parts.push(Span::raw(" "));
        info_parts.push(Span::styled(
            speed.clone(),
            Style::default().fg(Color::Cyan),
        ));
    }

    // ETA
    if let Some(ref eta) = download.eta {
        info_parts.push(Span::raw(" ETA:"));
        info_parts.push(Span::styled(
            eta.clone(),
            Style::default().fg(Color::Magenta),
        ));
    }

    // Stale indicator
    if is_stale {
        info_parts.push(Span::styled(
            if use_ascii {
                " [STALE - X:dismiss]"
            } else {
                " ⚠ (X:dismiss)"
            },
            Style::default().fg(Color::DarkGray),
        ));
    }

    let info_line = Line::from(info_parts);
    let info_widget = Paragraph::new(info_line);
    frame.render_widget(info_widget, layout[1]);
}
