mod overlays;
mod panels;
mod style;

use ratatui::{
    Frame,
    widgets::{Block, Borders, Gauge},
};

use crate::app_state::UiSnapshot;
use crate::ui::settings_menu::SettingsMenu;

use super::UiContext;
use overlays::{render_help_overlay, render_toast};
use panels::{render_active_downloads, render_footer, render_logs, render_pending_queue};

/// Renders the Terminal User Interface (TUI) using a snapshot of the application state.
///
/// This function is responsible for drawing all UI elements including the progress bar,
/// download queues, active downloads, logs, and keyboard control instructions.
pub fn ui(
    frame: &mut Frame,
    snapshot: &UiSnapshot,
    settings_menu: &mut SettingsMenu,
    ctx: &mut UiContext,
) {
    if settings_menu.is_visible() {
        settings_menu.render(frame, frame.area());
        return;
    }

    // Use pre-captured snapshot data instead of acquiring locks
    let progress = snapshot.progress;
    let active_downloads = &snapshot.active_downloads;
    let started = snapshot.started;
    let concurrent = snapshot.concurrent;
    let is_paused = snapshot.paused;
    let is_completed = snapshot.completed;
    let completed_tasks = snapshot.completed_tasks;
    let total_tasks = snapshot.total_tasks;
    let use_ascii = snapshot.use_ascii_indicators;
    let total_retries = snapshot.total_retries;
    let failed_count = snapshot.failed_count;

    let main_layout = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints(
            [
                ratatui::layout::Constraint::Length(3),
                ratatui::layout::Constraint::Percentage(40),
                ratatui::layout::Constraint::Percentage(40),
                ratatui::layout::Constraint::Length(4),
            ]
            .as_ref(),
        )
        .split(frame.area());

    // ----- Status indicators -----
    let status_indicator = if use_ascii {
        if is_completed {
            "[DONE] COMPLETED"
        } else if is_paused {
            "[PAUSE] PAUSED"
        } else if started {
            "[RUN] RUNNING"
        } else {
            "[STOP] STOPPED"
        }
    } else if is_completed {
        "✅ COMPLETED"
    } else if is_paused {
        "⏸️ PAUSED"
    } else if started {
        "▶️ RUNNING"
    } else {
        "⏹️ STOPPED"
    };

    // ----- Progress bar with status -----
    let retry_info = if total_retries > 0 {
        if use_ascii {
            format!(" | [R] {} retries", total_retries)
        } else {
            format!(" | ↻ {} retries", total_retries)
        }
    } else {
        String::new()
    };

    let progress_title = format!(
        "{} - Progress: {:.1}% ({}/{}){}{}",
        status_indicator,
        progress * 100.0,
        completed_tasks,
        total_tasks,
        if failed_count > 0 {
            if use_ascii {
                format!(" - [X] {} Failed", failed_count)
            } else {
                format!(" - ❌ {} Failed", failed_count)
            }
        } else {
            String::new()
        },
        retry_info
    );

    let gauge = Gauge::default()
        .block(Block::default().title(progress_title).borders(Borders::ALL))
        .gauge_style(ratatui::style::Style::default().fg(if is_paused {
            ratatui::style::Color::Yellow
        } else if is_completed {
            ratatui::style::Color::Green
        } else if failed_count > 0 {
            ratatui::style::Color::Red
        } else if started {
            ratatui::style::Color::Blue
        } else {
            ratatui::style::Color::Gray
        }))
        .ratio(progress);
    frame.render_widget(gauge, main_layout[0]);

    // ----- Downloads area (Pending + Active) -----
    let downloads_layout = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Horizontal)
        .constraints([
            ratatui::layout::Constraint::Percentage(50),
            ratatui::layout::Constraint::Percentage(50),
        ])
        .split(main_layout[1]);

    render_pending_queue(frame, downloads_layout[0], snapshot, ctx);
    render_active_downloads(
        frame,
        downloads_layout[1],
        active_downloads,
        concurrent,
        use_ascii,
        started,
    );

    render_logs(frame, main_layout[2], &snapshot.logs);
    render_footer(frame, main_layout[3], snapshot, ctx);

    if ctx.show_help {
        render_help_overlay(frame);
    }

    if let Some(toast_msg) = &snapshot.toast {
        render_toast(frame, toast_msg);
    }
}
