use std::fs;
use std::env;
use std::io::Write;
use std::path::{PathBuf, Path};
use std::process::{Command, Stdio};
use anyhow::{anyhow, Context, Result};
use dirs;
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
    wallpaper_dir: String,
    swww_params: Vec<String>,
    swaybg_cache_file: String,
    hyprland_refresh_script: String,
    cache_file: String,
    rofi_config_path: String,
    rofi_theme_override: String,
}

#[derive(Deserialize, Debug)]
struct GlobalConfig {
    wallpaper_manager: WallpaperManagerConfig,
}

// --- Config Loader Function ---
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

#[derive(Deserialize, Debug)]
struct HyprMonitor {
    name: String,
}
#[derive(Deserialize, Debug)]
struct SwayMonitor {
    name: String,
    active: bool,
}
#[derive(Deserialize, Debug, Clone)]
struct Wallpaper {
    name: String,
    path: PathBuf,
    thumb_path: PathBuf,
}

fn get_compositor() -> String {
    if env::var("NIRI_SOCKET").is_ok() { return "niri".to_string(); }
    if env::var("HYPRLAND_INSTANCE_SIGNATURE").is_ok() { return "hyprland".to_string(); }
    if env::var("SWAYSOCK").is_ok() { return "sway".to_string(); }
    
    if let Ok(desktop) = env::var("XDG_CURRENT_DESKTOP") {
        let d = desktop.to_lowercase();
        if d.contains("niri") { return "niri".to_string(); }
        if d.contains("hypr") { return "hyprland".to_string(); }
        if d.contains("sway") { return "sway".to_string(); }
    }
    "unknown".to_string()
}

fn get_monitor_list(compositor: &str) -> Result<Vec<String>> {
    let output;
    match compositor {
        "hyprland" => {
            output = Command::new("hyprctl").arg("-j").arg("monitors").output()?;
            if !output.status.success() {
                anyhow::bail!("hyprctl command failed");
            }
            let monitors: Vec<HyprMonitor> = serde_json::from_slice(&output.stdout)
                .context("Failed to parse hyprctl JSON")?;
            Ok(monitors.into_iter().map(|m| m.name).collect())
        }
        "sway" => {
            output = Command::new("swaymsg").arg("-t").arg("get_outputs").output()?;
            if !output.status.success() {
                anyhow::bail!("swaymsg command failed");
            }
            let monitors: Vec<SwayMonitor> = serde_json::from_slice(&output.stdout)
                .context("Failed to parse swaymsg JSON")?;
            Ok(monitors
                .into_iter()
                .filter(|m| m.active)
                .map(|m| m.name)
                .collect())
        }
        "niri" => {
            output = Command::new("swww")
                .arg("query")
                .arg("--namespace")
                .arg("niri")
                .output()?;
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                anyhow::bail!("swww query failed: {}", stderr);
            }
            let stdout = String::from_utf8_lossy(&output.stdout);
            let monitors: Vec<String> = stdout
                .lines()
                .filter_map(|line| {
                    let parts: Vec<&str> = line.split(':').collect();
                    parts.get(1).map(|s| s.trim().to_string())
                })
                .collect();
            Ok(monitors)
        }
        _ => Err(anyhow!("Unknown compositor for monitor detection")),
    }
}

fn ask_rofi(prompt: &str, items: Vec<String>, config: Option<(&Path, &str)>) -> Result<String> {
    let items_str = items.join("\n");
    let mut cmd = Command::new("rofi");
    cmd.args(["-dmenu", "-i", "-p", prompt, "-markup-rows"]);
    if let Some((conf, theme)) = config {
        cmd.arg("-config").arg(conf);
        cmd.arg("-theme-str").arg(theme);
    }
    cmd.stdin(Stdio::piped()).stdout(Stdio::piped());
    let mut child = cmd.spawn().context("Failed to spawn rofi")?;
    child.stdin.as_mut().unwrap().write_all(items_str.as_bytes())?;
    let output = child.wait_with_output()?;
    if !output.status.success() {
        anyhow::bail!("Rofi was cancelled");
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}

// ---
// main function
// ---
fn main() -> Result<()> {
    let global_config = load_config()?;
    let config = global_config.wallpaper_manager;
    // compositor detection
    let compositor = get_compositor();
    if compositor == "unknown" {
        anyhow::bail!("No supported compositor running.");
    }

    // Get Monitor List
    let monitor_list = get_monitor_list(&compositor)?;
    if monitor_list.is_empty() {
        anyhow::bail!("Could not detect any active monitors.");
    }

    // Ask user to pick a monitor (Rofi 1)
    let chosen_monitor = ask_rofi("Select monitor", monitor_list, None)?;
    // ---
    // All the logic for picking a wallpaper
    // ---
    let cache_file = expand_path(&config.cache_file);
    if !cache_file.exists() {
        anyhow::bail!("Wallpaper cache missing! Please run 'wp-daemon' first.");
    }

    let json_str = fs::read_to_string(&cache_file)?;
    let mut wallpapers: Vec<Wallpaper> = serde_json::from_str(&json_str)?;
    wallpapers.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    let rofi_items: Vec<String> = wallpapers.iter().map(|wp| {
        format!("{}\0icon\x1f{}", wp.name, wp.thumb_path.to_string_lossy())
    }).collect();
    let rofi_conf_path = expand_path(&config.rofi_config_path);

    let selection_name = ask_rofi(
        "Select Wallpaper",
        rofi_items,
        Some((&rofi_conf_path, &config.rofi_theme_override))
    )?;

    let selected_wp = wallpapers.into_iter().find(|w| w.name == selection_name)
        .ok_or_else(|| anyhow!("Selected wallpaper not found in cache"))?;
    // Call apply script
    let current_exe = env::current_exe()?;
    let apply_path = current_exe.parent().unwrap().join("wp-apply");

    // Call 'wp-apply' using its full path
    Command::new(apply_path)
        .arg(selected_wp.path)
        .arg(&compositor)
        .arg(&chosen_monitor)
        .spawn()
        .context("Failed to run 'wp-apply' command")?;

    Ok(())
}
