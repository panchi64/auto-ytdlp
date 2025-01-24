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
    collections::VecDeque,
    fs, io,
    path::Path,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::{process::Command, sync::Semaphore, time::sleep};

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
    progress: Arc<Mutex<f64>>,
    logs: Arc<Mutex<Vec<String>>>,
    paused: Arc<Mutex<bool>>,
    shutdown: Arc<Mutex<bool>>,
    started: Arc<Mutex<bool>>,
}

impl AppState {
    fn new() -> Self {
        AppState {
            queue: Arc::new(Mutex::new(VecDeque::new())),
            progress: Arc::new(Mutex::new(0.0)),
            logs: Arc::new(Mutex::new(vec![
                "Welcome! Press 's' to start downloads".to_string()
            ])),
            paused: Arc::new(Mutex::new(false)),
            shutdown: Arc::new(Mutex::new(false)),
            started: Arc::new(Mutex::new(false)),
        }
    }
}

async fn download_worker(url: String, state: AppState, semaphore: Arc<Semaphore>) {
    let _permit = semaphore.acquire().await.unwrap();
    if *state.shutdown.lock().unwrap() {
        return;
    }

    let output = Command::new("yt-dlp")
        .arg("--download-archive")
        .arg("download_archive.txt")
        .arg(&url)
        .output()
        .await;

    match output {
        Ok(output) => {
            let mut logs = state.logs.lock().unwrap();
            logs.push(format!("Downloaded: {}", url));
            if !output.status.success() {
                logs.push(format!(
                    "Error downloading {}: {}",
                    url,
                    String::from_utf8_lossy(&output.stderr)
                ));
                Notification::new()
                    .summary("Download Error")
                    .body(&format!("Failed to download {}", url))
                    .show()
                    .ok();
            }
        }
        Err(e) => {
            state.logs.lock().unwrap().push(format!("Error: {}", e));
        }
    }

    let mut queue = state.queue.lock().unwrap();
    if let Some(pos) = queue.iter().position(|item| item == &url) {
        queue.remove(pos);
    }
    update_progress(&state);
}

fn update_progress(state: &AppState) {
    let queue = state.queue.lock().unwrap();
    let total = queue.len() as f64 + 1.0;
    let mut progress = state.progress.lock().unwrap();
    *progress = ((total - queue.len() as f64) / total) * 100.0;
}

async fn process_queue(state: AppState, max_concurrent: usize) {
    let semaphore = Arc::new(Semaphore::new(max_concurrent));
    loop {
        if *state.shutdown.lock().unwrap() {
            break;
        }

        if *state.paused.lock().unwrap() {
            sleep(Duration::from_secs(1)).await;
            continue;
        }

        let url = {
            let mut queue = state.queue.lock().unwrap();
            queue.pop_front()
        };

        if let Some(url) = url {
            let state_clone = state.clone();
            let semaphore_clone = semaphore.clone();
            tokio::spawn(async move {
                download_worker(url, state_clone, semaphore_clone).await;
            });
        } else {
            sleep(Duration::from_millis(500)).await;
        }
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
    let main_layout = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints(
            [
                ratatui::layout::Constraint::Length(3),
                ratatui::layout::Constraint::Min(10),
                ratatui::layout::Constraint::Length(3),
            ]
            .as_ref(),
        )
        .split(frame.size());

    // Progress bar
    let progress = *state.progress.lock().unwrap();
    let gauge = Gauge::default()
        .block(Block::default().title("Progress").borders(Borders::ALL))
        .gauge_style(ratatui::style::Style::default().fg(ratatui::style::Color::Green))
        .percent(progress as u16);
    frame.render_widget(gauge, main_layout[0]);

    // Download list and logs
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

    let queue = state.queue.lock().unwrap();
    let items: Vec<ListItem> = queue.iter().map(|i| ListItem::new(i.as_str())).collect();
    let list = List::new(items).block(Block::default().title("Queue").borders(Borders::ALL));
    frame.render_widget(list, content_layout[0]);

    let logs = state.logs.lock().unwrap();
    let log_items: Vec<ListItem> = logs
        .iter()
        .rev()
        .take(10)
        .map(|l| ListItem::new(l.as_str()))
        .collect();
    let log_list = List::new(log_items).block(Block::default().title("Logs").borders(Borders::ALL));
    frame.render_widget(log_list, content_layout[1]);

    // Help text
    let started = state.started.lock().unwrap();
    let paused = state.paused.lock().unwrap();
    let mut help_text = String::new();

    if !*started {
        help_text.push_str("[S]tart ");
    } else {
        if *paused {
            help_text.push_str("[R]esume ");
        } else {
            help_text.push_str("[P]ause ");
        }
        help_text.push_str("[S]top ");
    }
    help_text.push_str("[Q]uit [A]dd links [R]efresh");

    let help = Paragraph::new(help_text).block(Block::default().borders(Borders::ALL));
    frame.render_widget(help, main_layout[2]);
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
                    KeyCode::Char('q') => break,
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

    Notification::new()
        .summary("Download Complete")
        .body("All downloads finished")
        .show()?;

    Ok(())
}
