use std::fs;
use anyhow::{Result, Context};
use clap::Parser;
use serde::{Deserialize, Serialize};
use reqwest::header::HeaderMap;
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
    let mut outputs = Vec::new();
    for symbol in &config.stocks {
        match fetch_quote(client, symbol, api_key).await {
            Ok(quote) => {
                let text = format!("{} ${:.2}", symbol, quote.price);
                outputs.push(text);
            }
            Err(e) => {
                eprintln!("Failed to fetch {}: {}", symbol, e);
                outputs.push(format!("{} ???", symbol));
            }
        }
    }
    println!("{}", outputs.join(" | "));
    Ok(())
}
#[tokio::main]
async fn main() -> Result<()> {
    let client = reqwest::Client::new();
    let args = Args::parse();

    if args.tui {
        println!("Initializing TUI mode...");
    } else {
        run_waybar_mode(&client).await?; 
    }
    Ok(())
}
