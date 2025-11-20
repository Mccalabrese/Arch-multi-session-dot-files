use std::fs;
use std::path::PathBuf;
use std::process::Command;
use anyhow::{Context, Result};
use serde::Deserialize;
use toml;
use std::env;
use std::thread;
use std::time::Duration;

fn expand_path(path: &str) -> PathBuf {
    if path.starts_with("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(&path[2..]);
        }
    }
    PathBuf::from(path)
}

// --- Config Structs ---

#[derive(Deserialize, Debug)]
struct WaybarSwitcherConfig {
    target_file: String,
    niri_config: String,
    hyprland_config: String,
    sway_config: String,
}

#[derive(Deserialize, Debug)]
struct GlobalConfig {
    waybar_switcher: WaybarSwitcherConfig,
}

// --- Config Loader ---

fn load_config() -> Result<GlobalConfig> {
    let config_path = dirs::home_dir().context("Cannot find home dir")?.join(".config/rust-dotfiles/config.toml");

    let config_str = fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read config file from path: {}", config_path.display()))?;

    let config: GlobalConfig = toml::from_str(&config_str)
        .context("Failed to parse config.toml. Check for syntax errors.")?;
    
    Ok(config)
}

fn get_compositor() -> Option<String> {
    if env::var("NIRI_SOCKET").is_ok() {
        return Some("niri".to_string());
    }
    if env::var("HYPRLAND_INSTANCE_SIGNATURE").is_ok() {
        return Some("hyprland".to_string());
    }
    if env::var("SWAYSOCK").is_ok() {
        return Some("sway".to_string());
    }
    if let Ok(desktop) = env::var("XDG_CURRENT_DESKTOP") {
        let desktop = desktop.to_lowercase();
        if desktop.contains("niri") { return Some("niri".to_string()); }
        if desktop.contains("hyprland") { return Some("hyprland".to_string()); }
        if desktop.contains("sway") { return Some("sway".to_string()); }
    }
    None
}
fn main() -> Result<()> {
    // 1. Load Config
    let global_config = load_config()?;
    let config = global_config.waybar_switcher;
    //2.Detect the compositor
    let compositor = get_compositor().unwrap_or_else(|| "unknown".to_string());
    println!("Detected compositor: {}", compositor);
    // 3. Determine Source File based on Config
    // expand the tilde (~) immediately so fs::copy works
    let source_path_str = match compositor.as_str() {
        "niri" => &config.niri_config,
        "hyprland" => &config.hyprland_config,
        "sway" => &config.sway_config,
        _ => {
            println!("Unknown compositor, defaulting to Hyprland config.");
            &config.hyprland_config
        }
    };
    let source_path = expand_path(source_path_str);
    let target_path = expand_path(&config.target_file);

    println!("Copying config:\n  From: {:?}\n  To:   {:?}", source_path, target_path);

    // 4. Copy the config
    fs::copy(&source_path, &target_path)
        .with_context(|| format!("Failed to copy {:?} to {:?}", source_path, target_path))?;

    // 5. Restart Waybar
    println!("Restarting Waybar...");
    
    let _ = Command::new("pkill").arg("-x").arg("waybar").status();
    thread::sleep(Duration::from_millis(500));
    // Start new instance with the specific config file
    Command::new("waybar")
        .arg("-c")
        .arg(&target_path)
        .spawn()
        .context("Failed to spawn new waybar process")?;

    println!("Waybar restarted successfully.");
    Ok(())
}
