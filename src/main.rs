mod app;

use std::io::{self, Stdout};
use std::time::{Duration, Instant};

use anyhow::Result;
use clap::Parser;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

use app::{App, Jsz, Varz, render};

#[derive(Parser)]
#[command(
    name = "nats-monitor",
    version,
    about = "Real-time terminal dashboard for monitoring NATS servers"
)]
struct Cli {
    /// NATS monitoring URL (host:port)
    #[arg(short, long, default_value = "http://localhost:8222")]
    url: String,

    /// Polling interval in seconds
    #[arg(short, long, default_value = "1")]
    interval: u64,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let url = normalize_url(&cli.url);

    // Set up panic hook to restore terminal
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        original_hook(panic_info);
    }));

    let mut terminal = setup_terminal()?;
    let result = run(&mut terminal, url, cli.interval);
    restore_terminal(&mut terminal)?;
    result
}

/// Normalize URL: add http:// if missing, strip trailing slash
fn normalize_url(url: &str) -> String {
    let url = if !url.starts_with("http://") && !url.starts_with("https://") {
        format!("http://{url}")
    } else {
        url.to_string()
    };
    url.trim_end_matches('/').to_string()
}

fn run(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    url: String,
    interval: u64,
) -> Result<()> {
    let agent = ureq::Agent::new_with_config(
        ureq::config::Config::builder()
            .timeout_global(Some(Duration::from_secs(2)))
            .build(),
    );

    let poll_duration = Duration::from_secs(interval);
    let mut app = App::new(url);
    let mut last_poll = Instant::now() - poll_duration; // force immediate first poll

    loop {
        terminal.draw(|frame| render(frame, &app))?;

        // Poll for keyboard input (50ms timeout for responsive UI)
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => break,
                    KeyCode::Char('c') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                        break
                    }
                    _ => {}
                }
            }
        }

        // Poll NATS at the configured interval
        if last_poll.elapsed() >= poll_duration {
            // Fetch /varz
            let varz_url = format!("{}/varz", &app.url);
            match agent.get(&varz_url).call() {
                Ok(mut resp) => match resp.body_mut().read_json::<Varz>() {
                    Ok(varz) => app.update_varz(varz),
                    Err(e) => app.set_error(format!("varz parse error: {e}")),
                },
                Err(e) => app.set_error(format!("Connection error: {e}")),
            }

            // Fetch /jsz with stream and consumer details
            let jsz_url = format!("{}/jsz?streams=true&consumers=true", &app.url);
            match agent.get(&jsz_url).call() {
                Ok(mut resp) => match resp.body_mut().read_json::<Jsz>() {
                    Ok(jsz) => app.update_jsz(jsz),
                    Err(e) => app.set_error(format!("jsz parse error: {e}")),
                },
                Err(_) => {} // jsz may not be available, don't overwrite varz errors
            }

            last_poll = Instant::now();
        }
    }

    Ok(())
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    Ok(Terminal::new(backend)?)
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}
