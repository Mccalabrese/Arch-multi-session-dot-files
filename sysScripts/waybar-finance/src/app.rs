use ratatui::widgets::ListState;
use ratatui::style::Color;
use serde::{Deserialize, Serialize};

use crate::network::{FinnhubQuote, YahooSearchResult};

/// Defines the input state of the TUI.
/// We use a state machine approach to change keybindings based on context.
#[derive(Debug, PartialEq)]
pub enum InputMode {
    Normal, //Navigation and viewing
    Editing,  // Typing in the search bar
    KeyEntry, // Force-prompt for API key on first run
}
#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub stocks: Vec<String>,
    pub api_key: Option<String>,
}
// Default configuration for new users
impl Default for Config {
    fn default() -> Self {
        Self {
            stocks: vec![
                "SCHO".into(),
                "SPY".into(),
                "BITB".into(),
                "SGOL".into(),
                "QQQ".into(),
            ],
            api_key: None,
        }
    }
}
/// Holds the runtime state of the TUI application.
pub struct App {
    pub stocks: Vec<String>,
    pub should_quit: bool,
    pub state: ListState, // tracks the selected item in the stock list
    pub api_key: Option<String>,

    // Cached Data
    pub current_quote: Option<FinnhubQuote>,
    pub stock_history: Option<Vec<(f64, f64)>>,
    pub details: Option<StockDetails>,
    pub search_results: Vec<YahooSearchResult>,
    pub search_state: ListState,
    pub market_status: Option<MarketStatus>,
    
    // Input Handling
    pub input: String,
    pub input_mode: InputMode,
    
    // UI Feedback
    pub message: String,
    pub message_color: Color,


}


impl App {
    pub fn new(config: Config, message: String, message_color: Color, stock_history: Option<Vec<(f64, f64)>>) -> Self {
        let mut state = ListState::default();
        state.select(Some(0));
        // Detect if this is a first run (missing API key) and force KeyEntry mode.
        let (input_mode, msg, color) = if config.api_key.is_some() {
            (InputMode::Normal, message, message_color)
        } else {
            (
                InputMode::KeyEntry,
                "Welcome! Please enter your Finnhub API Key.".to_string(),
                Color::Yellow
            )
        };
        Self {
            stocks: config.stocks,
            should_quit: false,
            state,
            api_key: config.api_key,
            current_quote: None,
            input: String::new(),
            input_mode,
            message: msg,
            message_color: color,
            stock_history,
            details: None,
            search_results: vec![],
            search_state: ListState::default(),
            market_status: None,
        }
    }
    /// Moves the selection index down, wrapping around if necessary.
    pub fn next_search(&mut self) {
        let i = match self.search_state.selected() {
            Some(i) => {
                if i >= self.search_results.len().saturating_sub(1) {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.search_state.select(Some(i));
    }
    pub fn previous_search(&mut self) {
        let i = match self.search_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.search_results.len().saturating_sub(1)
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.search_state.select(Some(i));
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

/// Defines the data for the detailed stock view.
#[derive(Debug, Clone)]
pub struct StockDetails {
    pub market_cap: u64,
    pub pe_ratio: Option<f64>,
    pub dividend_yield: Option<f64>,
    pub high_52w: f64,
    pub low_52w: f64,
    pub year_return: Option<f64>,
}
/// Defines the current market status (bond yields, yield curve etc)
#[derive(Debug, Clone)]
pub struct MarketStatus {
    pub yield_10y: f64,
    pub yield_5y: f64,
    pub yield_3m: f64,
}
/// Calculation for yield curve.
impl MarketStatus {
    // Calculate the 10Y - 3M spread
    pub fn spread_10y_3m(&self) -> f64 {
        self.yield_10y - self.yield_3m
    }
}

