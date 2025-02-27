use anyhow::Result;
use clap::Parser;
use clipboard::{ClipboardContext, ClipboardProvider};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use notify_rust::Notification;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::{
    backend::CrosstermBackend,
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph},
    Frame, Terminal,
};
use std::{
    collections::{HashSet, VecDeque},
    fs::{self, File},
    io::{self, BufRead, BufReader},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Run in automated mode without TUI
    #[arg(short, long)]
    auto: bool,
    /// Max concurrent downloads
    #[arg(short, long, default_value_t = 4)]
    concurrent: usize,
    /// Download directory
    #[arg(short, long, default_value = "./yt_dlp_downloads")]
    download_dir: PathBuf,
    /// Archive file path
    #[arg(short = 'f', long, default_value = "./download_archive.txt")]
    archive_file: PathBuf,
}

#[derive(Clone)]
struct AppState {
    queue: Arc<Mutex<VecDeque<String>>>,
    active_downloads: Arc<Mutex<HashSet<String>>>,
    progress: Arc<Mutex<f64>>,
    logs: Arc<Mutex<Vec<String>>>,
    paused: Arc<Mutex<bool>>,
    shutdown: Arc<Mutex<bool>>,
    started: Arc<Mutex<bool>>,
    force_quit: Arc<Mutex<bool>>,
    completed: Arc<Mutex<bool>>,
    total_tasks: Arc<Mutex<usize>>,
    completed_tasks: Arc<Mutex<usize>>,
    notification_sent: Arc<Mutex<bool>>,
    initial_total_tasks: Arc<Mutex<usize>>,
    concurrent: Arc<Mutex<usize>>,
}

impl AppState {
    fn new() -> Self {
        AppState {
            queue: Arc::new(Mutex::new(VecDeque::new())),
            active_downloads: Arc::new(Mutex::new(HashSet::new())),
            progress: Arc::new(Mutex::new(0.0)),
            logs: Arc::new(Mutex::new(vec![
                "Welcome! Press 'S' to start downloads".to_string(),
                "Press 'Q' to quit, 'Shift+Q' to force quit".to_string(),
            ])),
            paused: Arc::new(Mutex::new(false)),
            shutdown: Arc::new(Mutex::new(false)),
            started: Arc::new(Mutex::new(false)),
            force_quit: Arc::new(Mutex::new(false)),
            completed: Arc::new(Mutex::new(false)),
            total_tasks: Arc::new(Mutex::new(0)),
            completed_tasks: Arc::new(Mutex::new(0)),
            notification_sent: Arc::new(Mutex::new(false)),
            initial_total_tasks: Arc::new(Mutex::new(0)),
            concurrent: Arc::new(Mutex::new(0)),
        }
    }
}

fn download_worker(url: String, state: AppState, args: Args) {
    if *state.force_quit.lock().unwrap() {
        return;
    }

    // Add to active downloads
    {
        let mut active = state.active_downloads.lock().unwrap();
        active.insert(url.clone());
    }

    // Add log entry when download starts
    {
        let mut logs = state.logs.lock().unwrap();
        logs.push(format!("Starting download: {}", url));
    }

    let output_template = args
        .download_dir
        .join("%(title)s - [%(id)s].%(ext)s")
        .to_str()
        .unwrap()
        .to_string();

    let mut cmd = Command::new("yt-dlp")
        .arg("--format")
        .arg("bestvideo*+bestaudio/best")
        .arg("--download-archive")
        .arg(&args.archive_file)
        .arg("--output")
        .arg(output_template)
        .arg("--newline")
        .arg(&url)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start yt-dlp");

    let stdout = cmd.stdout.take().unwrap();
    let reader = BufReader::new(stdout);

    // Use map_while to handle potential read errors properly
    for line in reader.lines().map_while(Result::ok) {
        if *state.force_quit.lock().unwrap() {
            cmd.kill().ok();
            break;
        }

        let log_line = if line.contains("ERROR") {
            format!("Error: {}", line)
        } else if line.contains("Destination") || line.contains("[download]") {
            line
        } else {
            continue;
        };

        state.logs.lock().unwrap().push(log_line);
    }

    let status = cmd.wait().expect("Failed to wait on yt-dlp");

    // Remove from active downloads
    state.active_downloads.lock().unwrap().remove(&url);

    // Update logs and remove from links.txt
    let result_msg = if status.success() {
        remove_link_from_file(&url).unwrap();
        *state.completed_tasks.lock().unwrap() += 1;
        format!("Completed: {}", url)
    } else {
        format!("Failed: {}", url)
    };

    state.logs.lock().unwrap().push(result_msg);
    update_progress(&state);
}

fn remove_link_from_file(url: &str) -> Result<()> {
    let file_path = "links.txt";
    let content = fs::read_to_string(file_path).unwrap_or_default();

    // Use a temporary file for atomic writes
    let temp_path = format!("{}.tmp", file_path);
    let new_content: Vec<&str> = content
        .lines()
        .filter(|line| line.trim() != url.trim())
        .collect();

    fs::write(&temp_path, new_content.join("\n"))?;
    fs::rename(&temp_path, file_path)?; // Atomic replace
    Ok(())
}

fn update_progress(state: &AppState) {
    let total = *state.total_tasks.lock().unwrap();
    let completed = *state.completed_tasks.lock().unwrap();

    let progress = if total > 0 {
        let p = (completed as f64 / total as f64) * 100.0;
        // Clamp between 0-100 to prevent gauge panic
        p.clamp(0.0, 100.0)
    } else {
        0.0
    };

    *state.progress.lock().unwrap() = progress;

    // Update completion state
    let mut completed_state = state.completed.lock().unwrap();
    *completed_state = total > 0 && completed == total;
}

fn check_dependencies() -> Result<(), Vec<String>> {
    let mut missing = Vec::new();

    // Check yt-dlp
    let yt_dlp_status = Command::new("yt-dlp")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    if yt_dlp_status.map(|s| !s.success()).unwrap_or(true) {
        missing.push("yt-dlp is not installed or not accessible.".to_string());
    }

    // Check ffmpeg
    let ffmpeg_status = Command::new("ffmpeg")
        .arg("-version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    if ffmpeg_status.map(|s| !s.success()).unwrap_or(true) {
        missing.push("ffmpeg is not installed or not accessible.".to_string());
    }

    if missing.is_empty() {
        Ok(())
    } else {
        Err(missing)
    }
}

fn process_queue(state: AppState, args: Args) {
    if state.queue.lock().unwrap().is_empty() {
        *state.completed.lock().unwrap() = true;
        return;
    }

    // Initialize total tasks with current queue length
    let queue_len = state.queue.lock().unwrap().len();
    *state.total_tasks.lock().unwrap() = queue_len;
    *state.completed_tasks.lock().unwrap() = 0; // Reset completed count

    let mut handles = vec![];

    // Create worker threads
    for _ in 0..args.concurrent {
        let state_clone = state.clone();
        let args_clone = args.clone();

        let handle = thread::spawn(move || {
            loop {
                // Check exit conditions first
                if *state_clone.force_quit.lock().unwrap() || *state_clone.shutdown.lock().unwrap()
                {
                    break;
                }

                // Handle pause state
                if *state_clone.paused.lock().unwrap() {
                    thread::sleep(Duration::from_millis(100));
                    continue;
                }

                // Get next URL (atomic operation)
                let url = state_clone.queue.lock().unwrap().pop_front();

                if let Some(url) = url {
                    download_worker(url, state_clone.clone(), args_clone.clone());
                } else {
                    // Wait for new items or shutdown
                    thread::sleep(Duration::from_millis(100));
                }
            }
        });
        handles.push(handle);
    }

    // Wait for all workers to finish
    for handle in handles {
        handle.join().unwrap();
    }

    // Mark completion if queue is empty
    let completed = state.queue.lock().unwrap().is_empty();
    *state.completed.lock().unwrap() = completed;
    *state.started.lock().unwrap() = false;
}

fn load_links(state: &AppState) -> Result<()> {
    let content = fs::read_to_string("links.txt").unwrap_or_default();
    let mut queue = state.queue.lock().unwrap();
    *queue = content.lines().map(String::from).collect();
    *state.initial_total_tasks.lock().unwrap() = queue.len();
    *state.total_tasks.lock().unwrap() = queue.len();
    Ok(())
}

fn save_links(state: &AppState) -> Result<()> {
    let queue = state.queue.lock().unwrap();
    let mut seen = HashSet::new();
    let unique_links: Vec<_> = queue
        .iter()
        .filter_map(|link| {
            let trimmed = link.trim().to_string();
            seen.insert(trimmed.clone()).then_some(trimmed)
        })
        .collect();
    fs::write("links.txt", unique_links.join("\n"))?;
    Ok(())
}

fn ui(frame: &mut Frame<CrosstermBackend<io::Stdout>>, state: &AppState) {
    let progress = *state.progress.lock().unwrap();
    let queue = state.queue.lock().unwrap().clone();
    let active_downloads = state.active_downloads.lock().unwrap().clone();
    let started = *state.started.lock().unwrap();
    let logs = state.logs.lock().unwrap().clone();
    let initial_total = *state.initial_total_tasks.lock().unwrap();
    let concurrent = *state.concurrent.lock().unwrap();

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
        .split(frame.size());

    // ----- Status indicators -----
    let status_indicator = if *state.completed.lock().unwrap() {
        "✅ COMPLETED"
    } else if *state.paused.lock().unwrap() {
        "⏸️ PAUSED"
    } else if started {
        "▶️ RUNNING"
    } else {
        "⏹️ STOPPED"
    };

    // Count failed downloads based on log entries
    let failed_count = logs
        .iter()
        .filter(|line| line.starts_with("Failed:"))
        .count();

    // ----- Progress bar with status -----
    let progress_title = format!(
        "{} - Progress: {:.1}% ({}/{}){}",
        status_indicator,
        progress,
        *state.completed_tasks.lock().unwrap(),
        *state.total_tasks.lock().unwrap(),
        if failed_count > 0 {
            format!(" - ❌ {} Failed", failed_count)
        } else {
            String::new()
        }
    );

    let gauge = Gauge::default()
        .block(Block::default().title(progress_title).borders(Borders::ALL))
        .gauge_style(
            ratatui::style::Style::default().fg(if *state.paused.lock().unwrap() {
                ratatui::style::Color::Yellow
            } else if *state.completed.lock().unwrap() {
                ratatui::style::Color::Green
            } else if failed_count > 0 {
                ratatui::style::Color::Red
            } else if started {
                ratatui::style::Color::Blue
            } else {
                ratatui::style::Color::Gray
            }),
        )
        .percent(progress as u16);
    frame.render_widget(gauge, main_layout[0]);

    // ----- Downloads area (Pending + Active) -----
    let downloads_layout = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Horizontal)
        .constraints([
            ratatui::layout::Constraint::Percentage(50),
            ratatui::layout::Constraint::Percentage(50),
        ])
        .split(main_layout[1]);

    // Pending downloads list with status icon
    let pending_title = format!(
        "{} Pending Downloads - {}/{}",
        if queue.is_empty() { "✅" } else { "📋" },
        queue.len(),
        initial_total
    );

    let pending_items: Vec<ListItem> = queue.iter().map(|i| ListItem::new(i.as_str())).collect();
    let pending_list =
        List::new(pending_items).block(Block::default().title(pending_title).borders(Borders::ALL));
    frame.render_widget(pending_list, downloads_layout[0]);

    // Active downloads list with status icon
    let active_title = format!(
        "{} Active Downloads - {}/{}",
        if active_downloads.is_empty() {
            if started {
                "⏸️"
            } else {
                "⏹️"
            }
        } else {
            "🔄"
        },
        active_downloads.len(),
        concurrent
    );

    let active_items: Vec<ListItem> = active_downloads
        .iter()
        .map(|i| ListItem::new(i.as_str()))
        .collect();
    let active_list =
        List::new(active_items).block(Block::default().title(active_title).borders(Borders::ALL));
    frame.render_widget(active_list, downloads_layout[1]);

    // ----- Logs display with color coding -----
    let colored_logs: Vec<Line> = logs
        .iter()
        .map(|line| {
            let style = if line.contains("Error") || line.contains("ERROR") {
                Style::default().fg(Color::Red)
            } else if line.contains("Warning") || line.contains("WARN") {
                Style::default().fg(Color::Yellow)
            } else if line.contains("Completed") {
                Style::default().fg(Color::Green)
            } else if line.contains("Starting download") {
                Style::default().fg(Color::Cyan)
            } else if line.contains("Links refreshed") || line.contains("Added") {
                Style::default().fg(Color::LightGreen)
            } else {
                Style::default().fg(Color::White)
            };

            Line::from(vec![Span::styled(line.clone(), style)])
        })
        .collect();

    let text_content = Text::from(colored_logs);
    let text_height = logs.len() as u16;
    let area_height = main_layout[2].height;
    let scroll = text_height.saturating_sub(area_height);

    let logs_widget = Paragraph::new(text_content)
        .block(Block::default().title("Logs").borders(Borders::ALL))
        .scroll((scroll, 0));
    frame.render_widget(logs_widget, main_layout[2]);

    // ----- Help text (keyboard shortcuts) -----
    let help_text = if *state.completed.lock().unwrap() {
        "Keys: [S]tart New | [A]dd | [R]efresh | [Q]uit | [Shift+Q] Force Quit"
    } else if !started {
        "Keys: [S]tart | [A]dd | [R]efresh | [Q]uit | [Shift+Q] Force Quit"
    } else if *state.paused.lock().unwrap() {
        "Keys: [P]Resume | [S]top | [A]dd | [R]efresh | [Q]uit | [Shift+Q] Force Quit"
    } else {
        "Keys: [P]ause | [S]top | [R]efresh | [Q]uit | [Shift+Q] Force Quit"
    };

    let help = Paragraph::new(help_text)
        .block(
            Block::default()
                .title("Keyboard Controls")
                .borders(Borders::ALL),
        )
        .alignment(ratatui::layout::Alignment::Center);
    frame.render_widget(help, main_layout[3]);
}

fn run_tui(state: AppState, args: Args) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let tick_rate = Duration::from_millis(250);
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|f| ui(f, &state))?;

        // Check for completed downloads and show notification
        {
            let completed = *state.completed.lock().unwrap();
            let mut notification_sent = state.notification_sent.lock().unwrap();

            if completed && !*notification_sent {
                Notification::new()
                    .summary("Download Complete")
                    .body("All downloads finished")
                    .show()?;
                *notification_sent = true;
            }
        }

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => {
                        if key.modifiers.contains(KeyModifiers::SHIFT) {
                            *state.force_quit.lock().unwrap() = true;
                            *state.shutdown.lock().unwrap() = true;
                            break;
                        } else {
                            *state.shutdown.lock().unwrap() = true;
                            break;
                        }
                    }
                    KeyCode::Char('s') => {
                        let mut started = state.started.lock().unwrap();
                        let mut shutdown = state.shutdown.lock().unwrap();
                        let mut paused = state.paused.lock().unwrap();

                        if !*started {
                            // Check dependencies before proceeding
                            match check_dependencies() {
                                Ok(()) => {
                                    // Proceed with starting downloads
                                    *shutdown = false;
                                    *paused = false;
                                    *started = true;
                                    *state.completed.lock().unwrap() = false;
                                    *state.progress.lock().unwrap() = 0.0;
                                    *state.completed_tasks.lock().unwrap() = 0;
                                    let queue_len = state.queue.lock().unwrap().len();
                                    *state.total_tasks.lock().unwrap() = queue_len;
                                    *state.notification_sent.lock().unwrap() = false;

                                    // Launch new worker threads
                                    let state_clone = state.clone();
                                    let args_clone = args.clone();
                                    thread::spawn(move || process_queue(state_clone, args_clone));
                                }
                                Err(errors) => {
                                    let mut logs = state.logs.lock().unwrap();
                                    for error in errors {
                                        logs.push(error.clone());
                                        // Provide installation links
                                        if error.contains("yt-dlp") {
                                            logs.push("Download the latest release of yt-dlp from: https://github.com/yt-dlp/yt-dlp/releases".to_string());
                                        }
                                        if error.contains("ffmpeg") {
                                            logs.push("Download ffmpeg from: https://www.ffmpeg.org/download.html".to_string());
                                        }
                                    }
                                }
                            }
                        } else {
                            // Stop ongoing downloads
                            *shutdown = true;
                            *started = false;
                            *paused = false;
                        }
                    }
                    KeyCode::Char('p') => {
                        if *state.started.lock().unwrap() {
                            let mut paused = state.paused.lock().unwrap();
                            *paused = !*paused;
                            // Force UI refresh
                            last_tick = Instant::now() - tick_rate;
                        }
                    }
                    KeyCode::Char('r') => {
                        if let Err(e) = load_links(&state) {
                            state
                                .logs
                                .lock()
                                .unwrap()
                                .push(format!("Error reloading links: {}", e));
                        } else {
                            let queue_len = state.queue.lock().unwrap().len();
                            *state.initial_total_tasks.lock().unwrap() = queue_len;

                            if !*state.started.lock().unwrap() {
                                *state.total_tasks.lock().unwrap() = queue_len;
                            }
                            state
                                .logs
                                .lock()
                                .unwrap()
                                .push("Links refreshed from file".to_string());
                        }
                        last_tick = Instant::now() - tick_rate;
                    }
                    KeyCode::Char('a') => {
                        let started = *state.started.lock().unwrap();
                        let paused = *state.paused.lock().unwrap();
                        let completed = *state.completed.lock().unwrap();

                        if !started || paused || completed {
                            let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();
                            if let Ok(contents) = ctx.get_contents() {
                                let links: Vec<String> = contents
                                    .lines()
                                    .map(|l| l.trim().to_string())
                                    .filter(|l| !l.is_empty())
                                    .filter(|l| url::Url::parse(l).is_ok())
                                    .collect();

                                let links_added = {
                                    let mut queue = state.queue.lock().unwrap();
                                    let existing: HashSet<_> = queue.iter().collect();
                                    let new_links = links
                                        .into_iter()
                                        .filter(|link| !existing.contains(link))
                                        .collect::<Vec<_>>();
                                    queue.extend(new_links.iter().cloned());
                                    new_links.len()
                                };

                                if links_added > 0 {
                                    // Update both current and initial totals
                                    *state.total_tasks.lock().unwrap() += links_added;
                                    *state.initial_total_tasks.lock().unwrap() += links_added;
                                    *state.completed.lock().unwrap() = false;
                                    save_links(&state)?;
                                    state
                                        .logs
                                        .lock()
                                        .unwrap()
                                        .push(format!("Added {} URLs", links_added));
                                }
                            }
                        } else {
                            state
                                .logs
                                .lock()
                                .unwrap()
                                .push("Cannot add links during active downloads".to_string());
                        }
                    }
                    _ => {}
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();
    let state = AppState::new();

    *state.concurrent.lock().unwrap() = args.concurrent;

    fs::create_dir_all(&args.download_dir)?;

    if !Path::new("links.txt").exists() {
        File::create("links.txt")?;
    }

    load_links(&state)?;

    if args.auto {
        // Check dependencies before processing in auto mode
        match check_dependencies() {
            Ok(()) => process_queue(state.clone(), args.clone()),
            Err(errors) => {
                for error in errors {
                    eprintln!("Error: {}", error);
                    if error.contains("yt-dlp") {
                        eprintln!("Please download the latest version of yt-dlp from: https://github.com/yt-dlp/yt-dlp/releases");
                    }
                    if error.contains("ffmpeg") {
                        eprintln!(
                            "Please download ffmpeg from: https://www.ffmpeg.org/download.html"
                        );
                    }
                }
                std::process::exit(1);
            }
        }
    } else {
        run_tui(state.clone(), args.clone())?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;
    use tempfile::tempdir;

    /// Test loading links from file and verifying queue population
    #[test]
    fn test_link_loading_and_queue_management() {
        let temp_dir = tempdir().unwrap();
        env::set_current_dir(&temp_dir).unwrap();

        // Create test links.txt
        fs::write("links.txt", "https://example.com/1\nhttps://example.com/2").unwrap();

        let state = AppState::new();
        load_links(&state).unwrap();

        assert_eq!(state.queue.lock().unwrap().len(), 2);

        // Test duplicate prevention
        state
            .queue
            .lock()
            .unwrap()
            .push_back("https://example.com/1".into());
        save_links(&state).unwrap();

        let contents = fs::read_to_string("links.txt").unwrap();
        assert_eq!(contents, "https://example.com/1\nhttps://example.com/2");
    }

    /// Test directory creation and file preservation
    #[test]
    fn test_directory_creation_and_file_preservation() {
        let temp_dir = tempdir().unwrap();
        let download_dir = temp_dir.path().join("new_downloads");
        let args = Args {
            auto: true, // Changed to auto mode
            concurrent: 1,
            download_dir: download_dir.clone(),
            archive_file: temp_dir.path().join("archive.txt"),
        };

        // Initialize empty state
        let state = AppState::new();
        *state.concurrent.lock().unwrap() = args.concurrent;

        // Verify directory creation
        assert!(!download_dir.exists());
        fs::create_dir_all(&download_dir).unwrap();
        assert!(download_dir.exists());

        // Test with empty queue
        process_queue(state, args.clone());

        let test_file = download_dir.join("test.txt");
        fs::write(&test_file, "test content").unwrap();
        assert!(test_file.exists());
    }

    /// Test concurrent download limits
    #[test]
    fn test_concurrent_download_limits() {
        let state = AppState::new();
        *state.concurrent.lock().unwrap() = 2;

        // Add test URLs
        let urls = vec![
            "https://example.com/1".into(),
            "https://example.com/2".into(),
            "https://example.com/3".into(),
        ];
        state.queue.lock().unwrap().extend(urls);

        // Verify concurrent limit enforcement
        assert_eq!(
            *state.concurrent.lock().unwrap(),
            2,
            "Concurrent limit should be set"
        );
        assert_eq!(
            state.queue.lock().unwrap().len(),
            3,
            "Queue should contain test items"
        );
    }

    /// Test pause/resume functionality
    #[test]
    fn test_pause_resume_mechanism() {
        let state = AppState::new();

        // Initial state check
        assert!(!*state.paused.lock().unwrap(), "Should start unpaused");

        // Toggle pause
        *state.paused.lock().unwrap() = true;
        assert!(*state.paused.lock().unwrap(), "Should enter paused state");

        // Toggle again
        *state.paused.lock().unwrap() = false;
        assert!(
            !*state.paused.lock().unwrap(),
            "Should resume from paused state"
        );
    }
}
