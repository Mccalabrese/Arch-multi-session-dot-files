use std::fs;
use std::io::stdout;
use anyhow::{Result, Context};
use clap::Parser;
use serde::{Deserialize, Serialize};
use crossterm::{
    event::{self, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{
    prelude::{CrosstermBackend, Terminal},
    widgets::{Block, Borders, Paragraph},
    layout::*,
    prelude::*,
};

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    tui: bool,
}
#[derive(Debug, Deserialize, Serialize)]
struct Config {
    stocks: Vec<String>,
    api_key: Option<String>,
}
#[derive(Debug, Deserialize)]
struct FinnhubQuote {
    #[serde(rename = "c")]
    price: f64,
    #[serde(rename = "dp")]
    percent: f64,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            stocks: vec![
                "SCHO".to_string(),
                "SPY".to_string(),
                "BITB".to_string(),
                "SGOL".to_string(),
                "QQQ".to_string()
            ],
            api_key: None,
        }
    }
}
#[derive(Debug, Serialize)]
struct WaybarOutput {
    text: String,
    tooltip: String,
    class: String,
}
struct App {
    stocks: Vec<String>,
    should_quit: bool,
}
impl App {
    fn new(config: Config) -> Self {
        Self {
            stocks: config.stocks,
            should_quit: false,
        }
    }
}
fn get_config_path() -> Result<std::path::PathBuf> {
    let config_dir = dirs::config_dir()
        .context("Could not find config directory")?;
    Ok(config_dir.join("waybar-finance/config.json"))
}
fn load_config(path: &std::path::PathBuf) -> Result<Config> {
    if !path.exists() {
        return Ok(Config::default());
    }
    let content = fs::read_to_string(path)
        .context("Failed to read config file")?;
    let config = serde_json::from_str(&content)
        .context("Failed to parse config.json")?;
    Ok(config)
}
async fn fetch_quote(client: &reqwest::Client, symbol: &str, key: &str) -> Result<FinnhubQuote> {
    let url = format!(
        "https://finnhub.io/api/v1/quote?symbol={}&token={}",
        symbol, key
    );
    let resp = client.get(&url).send().await?;
    if !resp.status().is_success() {
        return Err(anyhow::anyhow!("Failed to fetch quote: HTTP {}", resp.status()));
    }
    let quote: FinnhubQuote = resp.json().await?;
    Ok(quote)
}
async fn run_waybar_mode(client: &reqwest::Client) -> Result<()> {
    let config_path = get_config_path()?;
    let config = load_config(&config_path)?;
    let api_key = match &config.api_key {
        Some(k) => k,
        None => {
            eprintln!("Error: API key not found in config.json");
            return Ok(());
        }
    };
    let mut text_parts = Vec::new();
    let mut tooltip_parts = Vec::new();
    for symbol in &config.stocks {
        match fetch_quote(client, symbol, api_key).await {
            Ok(quote) => {
                let (color, icon) = if quote.percent >= 0.0 {
                    ("#a6e3a1", "")
                } else {
                    ("#f38ba8", "")
                };
                let part = format!(
                    "<span color='{}'>{} {:.2} {}</span>",
                    color, symbol, quote.price, icon
                );
                text_parts.push(part);
                tooltip_parts.push(format!("{}: ${:.2} ({:.2}%)", symbol, quote.price, quote.percent));
            }
            Err(_) => {
                text_parts.push(format!("<span color='#6c7086'>{} ???</span>", symbol));
            }
        }
    }
    let output = WaybarOutput {
        text: text_parts.join(" "),
        tooltip: tooltip_parts.join("\n"),
        class: "finance".to_string(),
    };
    println!("{}", serde_json::to_string(&output)?);
    Ok(())
}
fn ui(frame: &mut ratatui::Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Min(0),
        ])
        .split(frame.area());
    let left_block = Block::default()
        .title("Watchlist")
        .borders(Borders::ALL);
    frame.render_widget(left_block, chunks[0]);
    let right_block = Block::default()
        .title("Chart")
        .borders(Borders::ALL);
    frame.render_widget(right_block, chunks[1]);

}
async fn run_tui(client: &reqwest::Client, app: &mut App) -> Result<()> {
    let mut stdout = stdout();
    stdout.execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    loop {
        terminal.draw(|frame| {
            ui(frame, &app);
        })?;
        if event::poll(std::time::Duration::from_millis(16))? {
            if let event::Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press && key.code == KeyCode::Char('q') {
                    app.should_quit = true;
                }
            }
        }
        if app.should_quit {
            break;
        }
    }
    terminal.backend_mut().execute(LeaveAlternateScreen)?;
    disable_raw_mode()?;
    Ok(())
}
#[tokio::main]
async fn main() -> Result<()> {
    let client = reqwest::Client::new();
    let args = Args::parse();
    let config_path = get_config_path()?;
    let config = load_config(&config_path)?;
    let mut app = App::new(config);
    if args.tui {
        println!("Initializing TUI mode...");
        run_tui(&client, &mut app).await?
    } else {
        run_waybar_mode(&client).await?; 
    }
    Ok(())
}
