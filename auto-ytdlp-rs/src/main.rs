use anyhow::Result;
use clap::Parser;
use clipboard::{ClipboardContext, ClipboardProvider};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use notify_rust::Notification;
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
    #[arg(short, long, default_value = "download_archive.txt")]
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
        format!("Completed: {}", url)
    } else {
        format!("Failed: {}", url)
    };

    state.logs.lock().unwrap().push(result_msg);
    update_progress(&state);
}

fn remove_link_from_file(url: &str) -> Result<()> {
    let content = fs::read_to_string("links.txt").unwrap_or_default();
    let new_content: Vec<&str> = content
        .lines()
        .filter(|line| line.trim() != url.trim())
        .collect();
    fs::write("links.txt", new_content.join("\n"))?;
    Ok(())
}

fn update_progress(state: &AppState) {
    let queue = state.queue.lock().unwrap();
    let active = state.active_downloads.lock().unwrap();
    let total = queue.len() + active.len();

    if total == 0 {
        return;
    }

    let completed = total - (queue.len() + active.len());
    let mut progress = state.progress.lock().unwrap();
    *progress = (completed as f64 / total as f64) * 100.0;
}

fn process_queue(state: AppState, args: Args) {
    let mut handles = vec![];

    // Create worker threads based on concurrent limit
    for _ in 0..args.concurrent {
        let state_clone = state.clone();
        let args_clone = args.clone();

        let handle = thread::spawn(move || {
            loop {
                // Check exit conditions
                if *state_clone.force_quit.lock().unwrap() {
                    break;
                }

                // Check pause state
                if *state_clone.paused.lock().unwrap() {
                    thread::sleep(Duration::from_millis(100));
                    continue;
                }

                // Get next URL
                let url = {
                    let mut queue = state_clone.queue.lock().unwrap();
                    queue.pop_front()
                };

                if let Some(url) = url {
                    // Process the download
                    download_worker(url, state_clone.clone(), args_clone.clone());
                    update_progress(&state_clone);
                } else {
                    // Queue is empty, check shutdown status
                    if *state_clone.shutdown.lock().unwrap() {
                        break;
                    }
                    // Wait before checking again
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
    if state.queue.lock().unwrap().is_empty() {
        *state.completed.lock().unwrap() = true;
    }
}

fn load_links(state: &AppState) -> Result<()> {
    let content = fs::read_to_string("links.txt").unwrap_or_default();
    let mut queue = state.queue.lock().unwrap();
    *queue = content.lines().map(String::from).collect();
    Ok(())
}

fn save_links(state: &AppState) -> Result<()> {
    let queue = state.queue.lock().unwrap();
    // Deduplicate while preserving order of first occurrence
    let mut seen = HashSet::new();
    let unique_links: Vec<_> = queue
        .iter()
        .filter_map(|link| {
            let trimmed = link.trim().to_string();
            if seen.insert(trimmed.clone()) {
                Some(trimmed)
            } else {
                None
            }
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
    let paused = *state.paused.lock().unwrap();
    let logs = state.logs.lock().unwrap().clone();

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

    // Progress bar
    let gauge = Gauge::default()
        .block(Block::default().title("Progress").borders(Borders::ALL))
        .gauge_style(ratatui::style::Style::default().fg(ratatui::style::Color::Green))
        .percent(progress as u16);
    frame.render_widget(gauge, main_layout[0]);

    // Downloads area (Pending + Active)
    let downloads_layout = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Horizontal)
        .constraints([
            ratatui::layout::Constraint::Percentage(50),
            ratatui::layout::Constraint::Percentage(50),
        ])
        .split(main_layout[1]);

    // Pending downloads list
    let pending_items: Vec<ListItem> = queue.iter().map(|i| ListItem::new(i.as_str())).collect();
    let pending_list = List::new(pending_items).block(
        Block::default()
            .title("Pending Downloads")
            .borders(Borders::ALL),
    );
    frame.render_widget(pending_list, downloads_layout[0]);

    // Active downloads list
    let active_items: Vec<ListItem> = active_downloads
        .iter()
        .map(|i| ListItem::new(i.as_str()))
        .collect();
    let active_list = List::new(active_items).block(
        Block::default()
            .title("Active Downloads")
            .borders(Borders::ALL),
    );
    frame.render_widget(active_list, downloads_layout[1]);

    // Logs display
    let log_text = logs.join("\n");
    let text_height = logs.len() as u16;
    let area_height = main_layout[2].height;
    let scroll = text_height.saturating_sub(area_height);

    let logs_widget = Paragraph::new(log_text)
        .block(Block::default().title("Logs").borders(Borders::ALL))
        .scroll((scroll, 0));
    frame.render_widget(logs_widget, main_layout[2]);

    // Help text
    let completed = *state.completed.lock().unwrap();
    let (line1, line2) = if !started || completed {
        (
            "Keys: [S]tart Downloads  [A]dd from Clipboard  [R]efresh",
            "      [Q]uit  [Shift+Q] Force Quit",
        )
    } else if paused {
        (
            "Keys: [R]esume  [S]top  [A]dd  [R]efresh",
            "      [Q]uit  [Shift+Q] Force Quit",
        )
    } else {
        (
            "Keys: [P]ause  [S]top  [A]dd  [R]efresh",
            "      [Q]uit  [Shift+Q] Force Quit",
        )
    };

    let help_text = format!("{}\n{}", line1, line2);
    let help = Paragraph::new(help_text).block(Block::default().borders(Borders::ALL));
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

                        if !*started {
                            *started = true;
                            *shutdown = false;
                            let state_clone = state.clone();
                            let args_clone = args.clone();
                            thread::spawn(move || process_queue(state_clone, args_clone));
                        } else {
                            *shutdown = true;
                            *started = false;
                        }
                    }
                    KeyCode::Char('p') => {
                        if *state.started.lock().unwrap() {
                            let mut paused = state.paused.lock().unwrap();
                            *paused = !*paused;
                        }
                    }
                    KeyCode::Char('r') => {
                        load_links(&state)?;
                    }
                    KeyCode::Char('a') => {
                        let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();
                        if let Ok(contents) = ctx.get_contents() {
                            // Process clipboard contents first
                            let links: Vec<String> = contents
                                .lines()
                                .map(|l| l.trim().to_string())
                                .filter(|l| !l.is_empty())
                                .collect();

                            // Lock queue only during modification
                            let links_added = {
                                let mut queue = state.queue.lock().unwrap();
                                let existing: HashSet<_> = queue.iter().cloned().collect();
                                let new_links = links
                                    .into_iter()
                                    .filter(|link| !existing.contains(link))
                                    .collect::<Vec<_>>();
                                queue.extend(new_links.iter().cloned());
                                new_links.len()
                            };

                            // Save and log after releasing queue lock
                            if links_added > 0 {
                                save_links(&state)?;
                                state
                                    .logs
                                    .lock()
                                    .unwrap()
                                    .push(format!("Added {} links from clipboard", links_added));
                            }
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

    fs::create_dir_all(&args.download_dir)?;

    if !Path::new("links.txt").exists() {
        File::create("links.txt")?;
    }

    load_links(&state)?;

    if args.auto {
        process_queue(state.clone(), args.clone());
    } else {
        run_tui(state.clone(), args.clone())?;
    }

    if *state.completed.lock().unwrap() {
        Notification::new()
            .summary("Download Complete")
            .body("All downloads finished")
            .show()?;
    }

    Ok(())
}
