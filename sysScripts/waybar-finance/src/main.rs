use std::fs;
use std::io::stdout;
use anyhow::{Result, Context};
use clap::Parser;
use time::OffsetDateTime;
use serde::{Deserialize, Serialize};
use chrono::{Utc, TimeZone, Datelike, Duration, DateTime};
use yahoo_finance_api::YahooConnector;
use crossterm::{
    event::{self, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{
    prelude::{CrosstermBackend, Terminal},
    widgets::{Block, Borders, ListState, Paragraph, ListItem, List, Clear, Chart, Dataset, Axis, GraphType},
    layout::{Rect, Layout, Direction, Constraint},
    prelude::*,
    style::{Color},
};

//We need different modes for keyboard input, search(edit) and normal
//q when searching must be the letter and not quit
#[derive(Debug, PartialEq)]
enum InputMode {
    Normal,
    Editing,
}
//Bool to determine if we send a tooltip or launch the full TUI
//controlled with -t or -tui flag
#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    tui: bool,
}
//I need candle data for a real chart
#[derive(Debug, Deserialize)]
struct CandleResponse {
    c: Vec<f64>,  //Closing prices
    t: Vec<i64>, //timestamps
    s: String,  //status
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
    state: ListState,
    api_key: Option<String>,
    current_quote: Option<FinnhubQuote>,
    input: String,
    input_mode: InputMode,
    message: String,
    message_color: Color,
    stock_history: Option<Vec<(f64, f64)>>
}
impl App {
    fn new(config: Config, message: String, message_color: Color, stock_history: Option<Vec<(f64, f64)>>) -> Self {
        let mut state = ListState::default();
        state.select(Some(0));
        Self {
            stocks: config.stocks,
            should_quit: false,
            state,
            api_key: config.api_key,
            current_quote: None,
            input: String::new(),
            input_mode: InputMode::Normal,
            message,
            message_color,
            stock_history,
        }
    }
    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.stocks.len() -1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }
    pub fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.stocks.len() -1
                } else {
                    i-1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }
    pub fn delete(&mut self) {
        if let Some(selected) = self.state.selected() {
            if self.stocks.is_empty() {
                return;
            }
            //Remove the item from the data
            self.stocks.remove(selected);
            //Dealing with the state
            if self.stocks.is_empty() {
                self.state.select(None);
            } else if selected >= self.stocks.len() {
                //delete the last item, move the cursor up one
                self.state.select(Some(self.stocks.len() - 1));
            }
            //If we delete from the middle the cursor will land on next item 
            //so I'm going to attempt adding nothing here
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
async fn fetch_history(_client: &reqwest::Client, symbol: &str, _key: &str) -> Result<Vec<(f64, f64)>> {
    let provider = YahooConnector::new()?;
    let end = OffsetDateTime::now_utc();
    let start = end - time::Duration::days(365);
    let response = provider.get_quote_history(symbol, start, end).await
        .context("Yaho API Error")?;
    let quotes = response.quotes().context("No quotes in response")?;
    let points: Vec<(f64, f64)> = quotes.iter()
        .map(|q| (q.timestamp as f64, q.close))
        .collect();
    if points.is_empty() {
        return Err(anyhow::anyhow!("History data is empty"));
    }
    Ok(points)
}
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
fn ui(frame: &mut ratatui::Frame, app: &mut App) {
    //verticle split for main vs footer
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(frame.area());
    //horizontal split (List vs Chart)
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Min(0),
        ])
        .split(main_layout[0]);
    let watchlist: Vec<ListItem> = app
        .stocks
        .iter()
        .map(|s| ListItem::new(s.as_str()))
        .collect();
    let list = List::new(watchlist)
        .block(Block::default()
            .title("Watchlist")
            .borders(Borders::ALL))
        .highlight_style(Style::default().bg(Color::Blue))
        .highlight_symbol(">> ");
    frame.render_stateful_widget(list, content_chunks[0], &mut app.state);
    if let Some(history) = &app.stock_history {
        let first_price = history[0].1;
        let last_price = history.last().unwrap().1;
        let start_ts = history[0].0 as i64;
        let end_ts = history.last().unwrap().0 as i64;
        let start_date = DateTime::from_timestamp(start_ts, 0).unwrap_or_default();
        let end_date = DateTime::from_timestamp(end_ts, 0).unwrap_or_default();
        let start_label = start_date.format("%Y-%m-%d").to_string();
        let end_label = end_date.format("%Y-%m-%d").to_string();
        let chart_color = if last_price >= first_price {
            Color::Green
        } else {
            Color::Red
        };
        let datasets = vec![
            Dataset::default()
                .marker(ratatui::symbols::Marker::Braille)
                .graph_type(GraphType::Line)
                .style(Style::default().fg(chart_color))
                .data(history),
        ];
        //Find y axis bounds 
        let min_price = history.iter().map(|(_, y)| *y).fold(f64::INFINITY, |a, b| a.min(b));
        let max_price = history.iter().map(|(_, y)| *y).fold(f64::NEG_INFINITY, |a, b| a.max(b));
        //Create the chart
        let chart = Chart::new(datasets)
            .block(Block::default().title("1 Year History").borders(Borders::ALL))
            .x_axis(Axis::default()
                .title("Date")
                .style(Style::default().fg(Color::Gray))
                .bounds([history[0].0, history.last().unwrap().0]) //these are times, start to end time
                .labels(vec![
                    Span::raw(start_label),
                    Span::raw(end_label),
                ]))
            .y_axis(Axis::default()
                .title("Price")
                .style(Style::default().fg(Color::Gray))
                .bounds([min_price, max_price])
                .labels(vec![
                    Span::raw(format!("{:.0}", min_price)),
                    Span::raw(format!("{:.0}", max_price)),
                ]));
        frame.render_widget(chart, content_chunks[1]);
    } else {
        let placeholder = Paragraph::new("Press Enter to load Chart")
            .block(Block::default().title("Chart").borders(Borders::ALL));
        frame.render_widget(placeholder, content_chunks[1]);
    }
    if app.input_mode == InputMode::Editing {
        let area = centered_rect(60, 20, frame.area());
        // 1. Clear the space
        frame.render_widget(Clear, area);
        //draw input box
        let input_block = Paragraph::new(app.input.as_str())
            .block(Block::default()
                .borders(Borders::ALL)
                .title("Input Stock Ticker (Press Enter to Confirm, Esc to Cancel)"));
        frame.render_widget(input_block, area);
    }
    let footer = Paragraph::new(app.message.as_str())
        .style(Style::default().fg(app.message_color));
    frame.render_widget(footer, main_layout[1]);

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
            ui(frame, app);
        })?;
        if event::poll(std::time::Duration::from_millis(16))? {
            if let event::Event::Key(key_event) = event::read()? {
                if key_event.kind == KeyEventKind::Press {
                    match app.input_mode {
                        InputMode::Normal => match key_event.code {
                            KeyCode::Char('q') => {
                                app.should_quit = true;
                            }
                            KeyCode::Char('a') => {
                                app.input_mode = InputMode::Editing;
                            }
                            KeyCode::Down => {
                                app.next();
                            }
                            KeyCode::Up => {
                                app.previous();
                            }
                            KeyCode::Enter => {
                                if let Some(selected) = app.state.selected() {
                                    let symbol = app.stocks[selected].clone();
                                    if let Some(api_key) = &app.api_key {
                                        //loading message
                                        app.message = format!("Loading{}...", symbol);
                                        app.message_color = Color::Cyan;
                                        //fetch Quote
                                        match fetch_quote(client, &symbol, api_key).await {
                                            Ok(quote) => {
                                                app.current_quote = Some(quote);
                                                app.message = format!("Loaded {}", symbol);
                                                app.message_color = Color::Green;
                                                //fetch history
                                                match fetch_history(client, &symbol, api_key).await {
                                                    Ok(history) => app.stock_history = Some(history),
                                                    Err(e) => {
                                                        app.stock_history = None;
                                                        app.message = format!("Quote Ok, Chart failed: {}", e);
                                                        app.message_color = Color::Yellow;
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                app.message = format!("Error fetching quote for {}, {}", symbol, e);
                                                app.message_color = Color::Red;
                                            }
                                        }
                                       
                                    }
                                }
                            }
                            KeyCode::Char('d') | KeyCode::Delete => {
                                app.delete();
                            }
                            _ => {}

                        }
                        InputMode::Editing => match key_event.code {
                            KeyCode::Enter => {
                                let new_symbol = app.input.trim().to_uppercase();
                                if new_symbol.is_empty() {
                                    return Ok(());
                                }
                                if app.stocks.contains(&new_symbol) {
                                    app.message = format!("{} is already in the list!", new_symbol);
                                    app.message_color = Color::Yellow;
                                    app.input_mode = InputMode::Normal;
                                    app.input.clear();
                                } else {
                                    //status loading
                                    app.message = format!("Fetching {}...", new_symbol);
                                    //try to fetch
                                    match fetch_quote(client, &new_symbol, app.api_key.as_ref().unwrap()).await {
                                        Ok(quote) => {
                                            //SUCCESS
                                            app.stocks.push(new_symbol.clone());
                                            app.current_quote = Some(quote);
                                            match fetch_history(client, &new_symbol, app.api_key.as_ref().unwrap()).await {
                                                Ok(history) => app.stock_history = Some(history),
                                                Err(e) => {
                                                    app.stock_history = None;
                                                    app.message = format!("Added {}, but chart failed: {}", new_symbol, e);
                                                }
                                            }
                                            app.message = format!("Added {}", new_symbol);
                                            app.message_color = Color::Green;
                                            app.state.select(Some(app.stocks.len() - 1));
                                        }
                                        Err(e) => {
                                            //FAILURE
                                            app.message = format!("Failed: Stock not found or API error. {}", e);
                                            app.message_color = Color::Red;
                                        }
                                    }                                    //Reset Input
                                    app.input.clear();
                                    app.input_mode = InputMode::Normal;
                                }
                            }
                            KeyCode::Esc => {
                                app.input.clear();
                                app.input_mode = InputMode::Normal;
                            }
                            KeyCode::Char(c) => {
                                app.input.push(c);
                            }
                            KeyCode::Backspace => {
                                app.input.pop();
                            }
                            _ => {}
                        }
                    } 
                }
            }
            if app.should_quit {
            break;
            }
        }
    }
    terminal.backend_mut().execute(LeaveAlternateScreen)?;
    disable_raw_mode()?;
    //save new config
    save_config(app)?;
    Ok(())
    
}
fn save_config(app: &App) -> Result<()> {
    let config_path = get_config_path()?;
    //make a new config from App state
    let new_config = Config {
        stocks: app.stocks.clone(),
        api_key: app.api_key.clone(),
    };
    //Serialize to pretty JSON
    let json = serde_json::to_string_pretty(&new_config)
        .context("Failed to serialize config")?;
    //Write to disk
    fs::write(config_path, json).context("Failed to write config file")?;
    Ok(())
}
#[tokio::main]
async fn main() -> Result<()> {
    let client = reqwest::Client::new();
    let args = Args::parse();
    let config_path = get_config_path()?;
    let config = load_config(&config_path)?;
    let mut app = App::new(config, String::from("Ready"), Color::Gray, None);
    if args.tui {
        println!("Initializing TUI mode...");
        run_tui(&client, &mut app).await?
    } else {
        run_waybar_mode(&client).await?; 
    }
    Ok(())
}
