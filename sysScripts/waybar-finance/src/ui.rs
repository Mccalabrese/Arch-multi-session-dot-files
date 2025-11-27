use std::io::stdout;
use anyhow::Result;
use chrono::DateTime;
use ratatui::{
    prelude::{CrosstermBackend, Terminal},
    widgets::{Block, Borders, Paragraph, ListItem, List, Clear, Chart, Dataset, Axis, GraphType},
    layout::{Rect, Layout, Direction, Constraint},
    prelude::*,
    style::{Color},
};
use crossterm::{
    event::{self, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use crate::app::{App, InputMode, StockDetails};
use crate::config::save_config;
use crate::network::{fetch_quote, fetch_details, fetch_history, FinnhubQuote};

pub enum AppEvent {
    //Network results
    QuoteFetched(String, Result<FinnhubQuote>),
    HistoryFetched(String, Result<Vec<(f64, f64)>>),
    DetailsFetched(String, Result<StockDetails>),
    Input(crossterm::event::Event),
    Tick,
}

pub async fn run_tui(client: &reqwest::Client, app: &mut App) -> Result<()> {
    let mut stdout = stdout();
    stdout.execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    //create event channel
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<AppEvent>();
    terminal.clear()?;
    //start event tick task
    let tx_tick = tx.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(250));
        loop {
            interval.tick().await;
            if tx_tick.send(AppEvent::Tick).is_err() {
                break;
            }
        }
    });
    //start input event task
    let tx_iput = tx.clone();
    tokio::task::spawn_blocking(move || {
        loop {
            if let Ok(event) = crossterm::event::read() {
                if tx_iput.send(AppEvent::Input(event)).is_err() {
                    break;
                }
            }
        }
    });
    //main loop
    loop {
        terminal.draw(|frame| {
            ui(frame, app);
        })?;
        if let Some(event) = rx.recv().await {
            match event {
                AppEvent::Tick => {
                    //just let the loop spin, no action
                }
                AppEvent::QuoteFetched(sym, res) => {
                    match res {
                        Ok(q) => {
                            app.current_quote = Some(q);
                            app.message = format!("Updated {}", sym);
                            app.message_color = Color::Red;
                        }
                        Err(e) => {
                            app.message = format!("Error: {}", e);
                            app.message_color = Color::Red;
                        }
                    }
                }
                AppEvent::HistoryFetched(sym, res) => {
                    match res {
                        Ok(h) => app.stock_history = Some(h),
                        Err(_) => app.stock_history = None,
                    }
                }
                AppEvent::DetailsFetched(sym, res) => {
                    match res {
                        Ok(d) => app.details = Some(d),
                        Err(e) => {
                            app.details = None;
                            app.message = format!("Details fetch failed for {}: {}", sym, e);
                            app.message_color = Color::Red;
                        }
                    }
                }
                AppEvent::Input(event) => {
                    match event {
                        crossterm::event::Event::Paste(pasted_text) => {
                            app.input.push_str(&pasted_text);
                            app.message = "Pasted text".to_string();
                            app.message_color = Color::Yellow;
                        }
                        crossterm::event::Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                            match app.input_mode {
                                InputMode::KeyEntry => match key_event.code {
                                    KeyCode::Char(c) => {
                                        app.input.push(c);
                                    }
                                    KeyCode::Backspace => {
                                        app.input.pop();
                                    }
                                    KeyCode::Enter => {
                                        let key = app.input.trim().to_string();
                                        if !key.is_empty() {
                                            // 1. Save to App State
                                            app.api_key = Some(key);
            
                                            // 2. Reset UI
                                            app.input.clear();
                                            app.input_mode = InputMode::Normal;
                                            app.message = "API Key Saved! Press 'q' to quit.".to_string();
                                            app.message_color = Color::Green;
            
                                            // 3. Save to Disk IMMEDIATELY
                                            if let Err(e) = save_config(app) {
                                                app.message = format!("Failed to save config: {}", e);
                                                app.message_color = Color::Red;
                                            }
                                        }
                                    }
                                    KeyCode::Esc | KeyCode::Char('q') => {
                                        app.should_quit = true;
                                    }
                                    _ => {}
                                    },
                                    InputMode::Normal => match key_event.code {
                                    KeyCode::Char('q') => app.should_quit = true,
                                    KeyCode::Char('a') => {
                                        app.input_mode = InputMode::Editing;
                                        app.message = "Enter Symbol...".to_string();
                                        app.message_color = Color::Yellow;
                                    }
                                    KeyCode::Down => app.next(),
                                    KeyCode::Up => app.previous(),
                                    KeyCode::Enter => {
                                        if let Some(selected) = app.state.selected() {
                                            let new_symbol = app.stocks[selected].clone();
                                            if let Some(api_key) = &app.api_key {
                                                let symbol = new_symbol.clone();
                                                let client_clone = client.clone();
                                                let api_key_clone = api_key.clone();
                                                let tx_clone = tx.clone();
                                                
                                                app.message = format!("Fetching {}...", symbol);
                                                app.message_color = Color::Cyan;

                                                tokio::spawn(async move {
                                                    let q_res = fetch_quote(&client_clone, &symbol, &api_key_clone).await;
                                                    let _ = tx_clone.send(AppEvent::QuoteFetched(symbol.clone(), q_res));
                                                    
                                                    let h_res = fetch_history(&client_clone, &symbol, &api_key_clone).await;
                                                    let _ = tx_clone.send(AppEvent::HistoryFetched(symbol.clone(), h_res));

                                                    let d_res = fetch_details(&client_clone, &symbol, &api_key_clone).await;
                                                    let _ = tx_clone.send(AppEvent::DetailsFetched(symbol.clone(), d_res));
                                                });
                                            }
                                        }
                                    }
                                    KeyCode::Char('d') | KeyCode::Delete => app.delete(),
                                    _ => {}
                                },
                                InputMode::Editing => match key_event.code {
                                    KeyCode::Enter => {
                                        let new_symbol = app.input.trim().to_uppercase();
                                        if !new_symbol.is_empty() {
                                            if app.stocks.contains(&new_symbol) {
                                                app.message = format!("{} exists!", new_symbol);
                                                app.message_color = Color::Yellow;
                                                app.input.clear();
                                                app.input_mode = InputMode::Normal;
                                            } else {
                                                if let Some(api_key) = &app.api_key {
                                                    let client_clone = client.clone();
                                                    let api_key_clone = api_key.clone();
                                                    let tx_clone = tx.clone();
                                                    let symbol = new_symbol.clone();

                                                    app.message = format!("Adding {}...", symbol);
                                                    app.stocks.push(symbol.clone());
                                                    app.state.select(Some(app.stocks.len() - 1));
                                                    app.input.clear();
                                                    app.input_mode = InputMode::Normal;

                                                    tokio::spawn(async move {
                                                        let q_res = fetch_quote(&client_clone, &symbol, &api_key_clone).await;
                                                        let _ = tx_clone.send(AppEvent::QuoteFetched(symbol.clone(), q_res));
                                                        
                                                        let h_res = fetch_history(&client_clone, &symbol, &api_key_clone).await;
                                                        let _ = tx_clone.send(AppEvent::HistoryFetched(symbol.clone(), h_res));

                                                        let d_res = fetch_details(&client_clone, &symbol, &api_key_clone).await;
                                                        let _ = tx_clone.send(AppEvent::DetailsFetched(symbol.clone(), d_res));
                                                    });
                                                }
                                            }
                                        }
                                    }
                                    KeyCode::Esc => {
                                        app.input.clear();
                                        app.input_mode = InputMode::Normal;
                                        app.message = "Ready".to_string();
                                        app.message_color = Color::Gray;
                                    }
                                    KeyCode::Char(c) => app.input.push(c),
                                    KeyCode::Backspace => { app.input.pop(); },
                                    _ => {}
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        if app.should_quit {
            terminal.backend_mut().execute(LeaveAlternateScreen)?;
            disable_raw_mode()?;
            //save new config
            save_config(app)?;
            std::process::exit(0);
        }
    }
    Ok(())
}


pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
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

pub fn ui(frame: &mut ratatui::Frame, app: &mut App) {
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
    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(70),
            Constraint::Percentage(30),
        ])
        .split(content_chunks[1]);
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
        frame.render_widget(chart, right_chunks[0]);
    } else {
        let placeholder = Paragraph::new("Press Enter to load Chart")
            .block(Block::default().title("Chart").borders(Borders::ALL));
        frame.render_widget(placeholder, right_chunks[0]);
    }
    // 1. Define the Parent Block (Border & Title)
    let details_block = Block::default()
        .title("Fundamentals")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::White));

    // 2. Render the Parent Block immediately to draw the border
    frame.render_widget(details_block.clone(), right_chunks[1]);
    // 3. Calculate the area INSIDE the border (so text doesn't overwrite the line)
    let details_area = details_block.inner(right_chunks[1]);

    // 4. Split that inner area into 3 Columns
    let col_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Ratio(1, 3), // Column 1 (33%)
            Constraint::Ratio(1, 3), // Column 2 (33%)
            Constraint::Ratio(1, 3), // Column 3 (33%)
        ])
        .split(details_area);

    if let Some(details) = &app.details {
        // Helper for N/A
        let fmt_num = |opt: Option<f64>, suffix: &str| -> String {
            opt.map(|v| format!("{:.2}{}", v, suffix)).unwrap_or("N/A".to_string())
        };

        // COLUMN 1: Price Action
        // We need the current price. If we fetched details, we likely have the quote too.
        // Let's grab price from app.current_quote or details if you added it there.
        // Assuming app.current_quote is available:
        let price_str = if let Some(q) = &app.current_quote {
            format!("${:.2}", q.price)
        } else {
            "N/A".to_string()
        };

        let col1_text = vec![
            Line::from(vec![Span::styled("Price:    ", Style::default().fg(Color::Gray)), Span::raw(price_str)]),
            Line::from(vec![Span::styled("52W High: ", Style::default().fg(Color::Gray)), Span::styled(format!("${:.2}", details.high_52w), Style::default().fg(Color::Green))]),
            Line::from(vec![Span::styled("52W Low:  ", Style::default().fg(Color::Gray)), Span::styled(format!("${:.2}", details.low_52w), Style::default().fg(Color::Red))]),
        ];

        // COLUMN 2: Valuation
        let col2_text = vec![
            Line::from(vec![Span::styled("Mkt Cap:  ", Style::default().fg(Color::Gray)), Span::raw(format!("${:.2}B", details.market_cap as f64 / 1_000.0))]), // Billions
            Line::from(vec![Span::styled("P/E Ratio:", Style::default().fg(Color::Gray)), Span::raw(fmt_num(details.pe_ratio, ""))]),
            Line::from(vec![Span::styled("Div Yield:", Style::default().fg(Color::Gray)), Span::raw(fmt_num(details.dividend_yield, "%"))]),
        ];

        // COLUMN 3: Volatility / Extra
        let col3_text = vec![
            Line::from(vec![Span::styled("Beta:     ", Style::default().fg(Color::Gray)), Span::raw(fmt_num(details.beta, ""))]),
            // Add more fields here later (e.g., Volume, EPS)
            Line::from(vec![Span::styled("Status:   ", Style::default().fg(Color::Gray)), Span::styled("Active", Style::default().fg(Color::Green))]),
        ];

        // Render the columns
        frame.render_widget(Paragraph::new(col1_text), col_chunks[0]);
        frame.render_widget(Paragraph::new(col2_text), col_chunks[1]);
        frame.render_widget(Paragraph::new(col3_text), col_chunks[2]);

    } else {
        // If no details loaded yet, show loading in the middle column
        frame.render_widget(Paragraph::new("Loading..."), col_chunks[1]);
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
    if app.input_mode == InputMode::KeyEntry {
        let area = centered_rect(60, 20, frame.area());
        // 1. Clear the space
        frame.render_widget(Clear, area);
        //draw input box
        let input_block = Paragraph::new(app.input.as_str())
            .block(Block::default()
                .borders(Borders::ALL)
                .title("Enter Finnhub API Key. This is an app requirement. Visit finnhub.io/register to obtain a key. (Press Enter to Save)"));
        frame.render_widget(input_block, area);
    }
    let footer = Paragraph::new(app.message.as_str())
        .style(Style::default().fg(app.message_color));
    frame.render_widget(footer, main_layout[1]);

}

