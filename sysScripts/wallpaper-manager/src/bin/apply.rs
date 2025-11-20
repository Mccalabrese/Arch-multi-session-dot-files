use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;
use anyhow::{Context, Result};
use dirs;
use std::fs;
use serde::Deserialize;
use toml;

fn expand_path(path: &str) -> PathBuf {
    if path.starts_with("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(&path[2..]);
        }
    }
    PathBuf::from(path)
}

#[derive(Deserialize, Debug)]
struct WallpaperManagerConfig {
    swww_params: Vec<String>,
    swaybg_cache_file: String,
    hyprland_refresh_script: String,
    wallpaper_dir: String,
}

#[derive(Deserialize, Debug)]
struct GlobalConfig {
    wallpaper_manager: WallpaperManagerConfig,
}

fn load_config() -> Result<GlobalConfig> {
    let config_path = dirs::home_dir()
        .context("Cannot find home dir")?
        .join(".config/rust-dotfiles/config.toml");

    let config_str = fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read config file from path: {}", config_path.display()))?;

    let config: GlobalConfig = toml::from_str(&config_str)
        .context("Failed to parse config.toml. Check for syntax errors.")?;
    
    Ok(config)
}

// pkill Helper
fn pkill(name: &str) {
    Command::new("pkill").arg("-x").arg(name).status().ok();
}

// ---
// Specialized "apply" functions
// ---

fn apply_swww_wallpaper(selected_file: &Path, monitor: &str, namespace: &str, swww_params: &[String]) -> Result<()> {
    println!("Applying wallpaper via swww (namespace: {})...", namespace);
    pkill("mpvpaper");
    pkill("swaybg");
    let _ = Command::new("swww-daemon")
        .arg("--namespace")
        .arg(namespace)
        .arg("--format")
        .arg("argb")
        .spawn();
    std::thread::sleep(std::time::Duration::from_millis(100));
    Command::new("swww")
        .arg("img") 
        .arg("--namespace")
        .arg(namespace)
        .arg("-o")
        .arg(monitor)
        .arg(selected_file)
        .args(swww_params)
        .status()
        .context("swww img command failed")?;
    Ok(())
}

fn apply_sway_wallpaper(selected_file: &Path, monitor: &str, cache_filename: &str) -> Result<()> {
    println!("Applying wallpaper for Sway...");
    pkill("swww-daemon");
    pkill("hyprpaper");
    Command::new("swaybg")
        .arg("-o")
        .arg(monitor)
        .arg("-i")
        .arg(selected_file)
        .spawn()
        .context("Failed to run swaybg")?;

    // --- write to the cache file ---
    if let Some(mut cache_path) = dirs::cache_dir() {
        cache_path.push(cache_filename);
        let _ = fs::write(cache_path, selected_file.to_str().unwrap_or(""));
    }

    Ok(())
}

// ---
// main function
// ---
fn main() -> Result<()> {
    //load config
    let global_config = load_config()?;
    let config = global_config.wallpaper_manager;
    // 1. Get arguments
    let args: Vec<String> = env::args().collect();
    let wallpaper_path_str = args.get(1).context("Missing wallpaper path")?;
    let compositor = args.get(2).context("Missing compositor name")?;
    let monitor = args.get(3).context("Missing monitor name")?;

    let wallpaper_path = PathBuf::from(wallpaper_path_str);

    // 2. Route to the correct function
    match compositor.as_str() {
        "hyprland" => {
            apply_swww_wallpaper(&wallpaper_path, monitor, "hypr", &config.swww_params)?;
            let refresh_script = expand_path(&config.hyprland_refresh_script);
            Command::new("bash").arg(refresh_script).status()?;
        }
        "niri" => {
            apply_swww_wallpaper(&wallpaper_path, monitor, "niri", &config.swww_params)?;
        }
        "sway" => {
            apply_sway_wallpaper(&wallpaper_path, monitor, &config.swaybg_cache_file)?;
        }
        _ => anyhow::bail!("Compositor argument '{}' is not recognized.", compositor),
    }

    Ok(())
}
