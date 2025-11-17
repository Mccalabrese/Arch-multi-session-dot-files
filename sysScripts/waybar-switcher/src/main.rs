use std::ffi::OsStr;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use anyhow::{Context, Result};
use serde::Deserialize;
use sysinfo::System;
use toml;
use shellexpand;

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
    let config_path = shellexpand::tilde("~/.config/rust-dotfiles/config.toml").to_string();

    let config_str = fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read config file from path: {}", config_path))?;

    let config: GlobalConfig = toml::from_str(&config_str)
        .context("Failed to parse config.toml. Check for syntax errors.")?;
    
    Ok(config)
}

fn get_compositor() -> Option<String> {
    let sys = System::new_all();

    if sys.processes_by_name(OsStr::new("niri")).next().is_some() {
        return Some("niri".to_string());
    }
    if sys.processes_by_name(OsStr::new("Hyprland")).next().is_some() {
        return Some("hyprland".to_string());
    }
    if sys.processes_by_name(OsStr::new("sway")).next().is_some() {
        return Some("sway".to_string());
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
    let source_path = PathBuf::from(shellexpand::tilde(source_path_str).to_string());
    let target_path = PathBuf::from(shellexpand::tilde(&config.target_file).to_string());

    println!("Copying config:\n  From: {:?}\n  To:   {:?}", source_path, target_path);

    // 4. Copy the config
    fs::copy(&source_path, &target_path)
        .with_context(|| format!("Failed to copy {:?} to {:?}", source_path, target_path))?;

    // 5. Restart Waybar
    println!("Restarting Waybar...");
    
    // Kill existing instances
    Command::new("killall")
        .arg("waybar")
        .status()
        .ok(); // ignore errors here (e.g., if waybar wasn't running)

    // Start new instance with the specific config file
    Command::new("waybar")
        .arg("-c")
        .arg(target_path)
        .spawn()
        .context("Failed to spawn new waybar process")?;

    println!("Waybar restarted successfully.");
    Ok(())
}
