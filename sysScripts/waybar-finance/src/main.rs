use std::fs;
use anyhow::{Result, Context};
use clap::Parser;
use serde::{Deserialize, Serialize};

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    tui: bool,
}
#[derive(Debug, Deserialize, Serialize)]
struct Config {
    stocks: Vec<String>,
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
            ]
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
fn main() -> Result<()> {

    let args = Args::parse();

    if args.tui {
        println!("Initializing TUI mode...");
    } else {
        println!("Outputting JSON for waybar...")
    }
    let config_path = get_config_path()?;
    println!("{}", config_path.display());
    let config = load_config(&config_path)?;
    println!("Loaded config:{:#?}", config);
    Ok(())
}
