use anyhow::Result;
use clap::Parser;
use clipboard::{ClipboardContext, ClipboardProvider};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
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
    fs, io,
    path::Path,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::{io::AsyncBufReadExt, process::Command, sync::Semaphore, time::sleep};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Run in automated mode without TUI
    #[arg(short, long)]
    auto: bool,
    /// Max concurrent downloads
    #[arg(short, long, default_value_t = 5)]
    concurrent: usize,
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

async fn download_worker(url: String, state: AppState, _permit: tokio::sync::OwnedSemaphorePermit) {
    if *state.force_quit.lock().unwrap() {
        return;
    }

    // Add to active downloads
    {
        let mut active = state.active_downloads.lock().unwrap();
        active.insert(url.clone());
    }

    // Add log entry when download starts
    let start_msg = format!("Starting download: {}", url);
    {
        let mut logs = state.logs.lock().unwrap();
        logs.push(start_msg);
    }

    let mut cmd = Command::new("yt-dlp");
    let mut child = cmd
        .arg("--download-archive")
        .arg("download_archive.txt")
        .arg("--newline")
        .arg(&url)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to start yt-dlp");

    let stdout = child.stdout.take().unwrap();
    let mut reader = tokio::io::BufReader::new(stdout).lines();

    while let Some(line) = reader.next_line().await.unwrap_or(None) {
        let log_line = if line.contains("ERROR") {
            format!("Error: {}", line)
        } else if line.contains("Destination") || line.contains("[download]") {
            line.clone()
        } else {
            continue;
        };

        {
            let mut logs = state.logs.lock().unwrap();
            logs.push(log_line);
        }
    }

    let status = child.wait().await.unwrap();

    // Remove from active downloads
    {
        let mut active = state.active_downloads.lock().unwrap();
        active.remove(&url);
    }

    // Update logs and remove from links.txt
    let result_msg = if status.success() {
        remove_link_from_file(&url).await.unwrap();
        format!("Completed: {}", url)
    } else {
        format!("Failed: {}", url)
    };

    {
        let mut logs = state.logs.lock().unwrap();
        logs.push(result_msg);
    }

    update_progress(&state);
}

async fn remove_link_from_file(url: &str) -> Result<()> {
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
    let total = queue.len() as f64 + 1.0;
    let mut progress = state.progress.lock().unwrap();
    *progress = ((total - queue.len() as f64) / total) * 100.0;
}

async fn process_queue(state: AppState, max_concurrent: usize) {
    let semaphore = Arc::new(Semaphore::new(max_concurrent));
    let mut normal_completion = false;

    loop {
        if *state.force_quit.lock().unwrap() {
            break;
        }

        if *state.shutdown.lock().unwrap() {
            break;
        }

        if *state.paused.lock().unwrap() {
            sleep(Duration::from_secs(1)).await;
            continue;
        }

        let url = {
            let queue = state.queue.lock().unwrap();
            queue.front().cloned()
        };

        if let Some(url) = url {
            let permit = semaphore.clone().acquire_owned().await.unwrap();
            if *state.force_quit.lock().unwrap() || *state.shutdown.lock().unwrap() {
                break;
            }

            let state_clone = state.clone();
            let url_clone = url.clone();
            tokio::spawn(async move {
                download_worker(url_clone, state_clone, permit).await;
            });

            let mut queue = state.queue.lock().unwrap();
            queue.pop_front();
            update_progress(&state);
        } else {
            // Queue is empty - normal completion
            normal_completion = true;
            break;
        }
    }

    // Set completion flag if we exited normally
    if normal_completion {
        *state.completed.lock().unwrap() = true;
    }
}

async fn load_links(state: &AppState) -> Result<()> {
    let content = fs::read_to_string("links.txt").unwrap_or_default();
    let mut queue = state.queue.lock().unwrap();
    *queue = content.lines().map(String::from).collect();
    Ok(())
}

async fn save_links(state: &AppState) -> Result<()> {
    let queue = state.queue.lock().unwrap();
    let content = queue.iter().cloned().collect::<Vec<_>>().join("\n");
    fs::write("links.txt", content)?;
    Ok(())
}

fn ui(frame: &mut Frame<CrosstermBackend<io::Stdout>>, state: &AppState) {
    // Clone state for UI rendering
    let queue = state.queue.lock().unwrap().clone();
    let active_downloads = state.active_downloads.lock().unwrap().clone();
    let logs = state.logs.lock().unwrap().clone();
    let progress = *state.progress.lock().unwrap();
    let started = *state.started.lock().unwrap();
    let paused = *state.paused.lock().unwrap();

    let main_layout = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints(
            [
                ratatui::layout::Constraint::Length(3),
                ratatui::layout::Constraint::Percentage(60),
                ratatui::layout::Constraint::Percentage(40),
                ratatui::layout::Constraint::Length(3),
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

    // Download lists
    let content_layout = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Horizontal)
        .constraints(
            [
                ratatui::layout::Constraint::Percentage(50),
                ratatui::layout::Constraint::Percentage(50),
            ]
            .as_ref(),
        )
        .split(main_layout[1]);

    // Pending downloads
    let pending_items: Vec<ListItem> = queue.iter().map(|i| ListItem::new(i.as_str())).collect();
    let pending_list = List::new(pending_items).block(
        Block::default()
            .title("Pending Downloads")
            .borders(Borders::ALL),
    );
    frame.render_widget(pending_list, content_layout[0]);

    // Active downloads
    let active_items: Vec<ListItem> = active_downloads
        .iter()
        .map(|i| ListItem::new(i.as_str()))
        .collect();
    let active_list = List::new(active_items).block(
        Block::default()
            .title("Active Downloads")
            .borders(Borders::ALL),
    );
    frame.render_widget(active_list, content_layout[1]);

    // Logs
    let log_layout = ratatui::layout::Layout::default()
        .constraints([ratatui::layout::Constraint::Min(1)])
        .split(main_layout[2]);

    let log_items: Vec<ListItem> = logs
        .iter()
        .rev()
        .take(20)
        .map(|l| ListItem::new(l.as_str()))
        .collect();
    let log_list = List::new(log_items).block(
        Block::default()
            .title("Download Logs")
            .borders(Borders::ALL),
    );
    frame.render_widget(log_list, log_layout[0]);

    // Help text
    let mut help_text = String::new();
    if !started {
        help_text.push_str("[S]tart ");
    } else {
        if paused {
            help_text.push_str("[R]esume ");
        } else {
            help_text.push_str("[P]ause ");
        }
        help_text.push_str("[S]top ");
    }
    help_text.push_str("[Q]uit [Shift+Q] Force Quit [A]dd links [R]efresh");

    let help = Paragraph::new(help_text).block(Block::default().borders(Borders::ALL));
    frame.render_widget(help, main_layout[3]);
}

async fn run_tui(state: AppState, max_concurrent: usize) -> Result<()> {
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
                        if key.modifiers.contains(event::KeyModifiers::SHIFT) {
                            // Force quit
                            *state.force_quit.lock().unwrap() = true;
                            *state.shutdown.lock().unwrap() = true;
                            break;
                        } else {
                            // Graceful shutdown
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
                            tokio::spawn(async move {
                                process_queue(state_clone, max_concurrent).await;
                            });
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
                        load_links(&state).await?;
                    }
                    KeyCode::Char('a') => {
                        let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();
                        if let Ok(contents) = ctx.get_contents() {
                            {
                                let mut queue = state.queue.lock().unwrap();
                                for line in contents.lines() {
                                    if !line.trim().is_empty() {
                                        queue.push_back(line.trim().to_string());
                                    }
                                }
                            }
                            save_links(&state).await?;
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

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let state = AppState::new();

    if !Path::new("links.txt").exists() {
        fs::File::create("links.txt")?;
    }

    load_links(&state).await?;

    if args.auto {
        let state_clone = state.clone();
        tokio::spawn(async move {
            process_queue(state_clone, args.concurrent).await;
        });

        while !state.queue.lock().unwrap().is_empty() {
            sleep(Duration::from_secs(1)).await;
        }
    } else {
        run_tui(state.clone(), args.concurrent).await?;
    }

    // Only show notification if we completed normally
    if *state.completed.lock().unwrap() {
        Notification::new()
            .summary("Download Complete")
            .body("All downloads finished")
            .show()?;
    }

    Ok(())
}
