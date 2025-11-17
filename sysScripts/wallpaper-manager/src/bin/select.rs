use std::fs;
use std::env;
use std::io::Write;
use std::path::{PathBuf, Path};
use std::process::{Command, Stdio};
use anyhow::{anyhow, Context, Result};
use dirs;
use serde::Deserialize;
use sysinfo::{System};
use toml;
use shellexpand;

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
    let config_path = shellexpand::tilde("~/.config/rust-dotfiles/config.toml").to_string();

    let config_str = fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read config file from path: {}", config_path))?;

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
    let mut sys = System::new_all();
    sys.refresh_processes();
    if sys.processes_by_name("niri").next().is_some() {
        "niri".to_string()
    } else if sys.processes_by_name("Hyprland").next().is_some() {
        "hyprland".to_string()
    } else if sys.processes_by_name("sway").next().is_some() {
        "sway".to_string()
    } else {
        "unknown".to_string()
    }
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

fn ask_rofi(prompt: &str, items: Vec<String>) -> Result<String> {
    let items_str = items.join("\n");
    let mut rofi = Command::new("rofi")
        .arg("-dmenu")
        .arg("-i") // Case-insensitive
        .arg("-p")
        .arg(prompt)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .context("Failed to spawn rofi (for monitor)")?;
    rofi.stdin.as_mut().unwrap().write_all(items_str.as_bytes())?;
    let output = rofi.wait_with_output()?;
    if !output.status.success() {
        anyhow::bail!("Rofi was cancelled (monitor selection)");
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
    let chosen_monitor = ask_rofi("Select monitor", monitor_list)?;
    if chosen_monitor.is_empty() {
        anyhow::bail!("No monitor selected.");
    }

    // ---
    // All the logic for picking a wallpaper
    // ---
    let cache_file_str = shellexpand::tilde(&config.cache_file).to_string();
    let cache_file = PathBuf::from(cache_file_str);
    if !cache_file.exists() {
        anyhow::bail!("Wallpaper cache missing! Please run 'wp-daemon' first.");
    }

    let json_str = fs::read_to_string(&cache_file)?;
    let mut wallpapers: Vec<Wallpaper> = serde_json::from_str(&json_str)?;
    wallpapers.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    let mut rofi_input = String::new();
    for wp in &wallpapers {
        let line = format!("{}\0icon\x1f{}\n", 
            wp.name, 
            wp.thumb_path.to_string_lossy()
        );
        rofi_input.push_str(&line);
    }

    // Ask user to pick a wallpaper (Rofi 2)
    let rofi_config_str = shellexpand::tilde(&config.rofi_config_path).to_string();
    let rofi_config = PathBuf::from(rofi_config_str);
    let theme_override = &config.rofi_theme_override;
    
    let mut rofi = Command::new("rofi")
        .arg("-dmenu")
        .arg("-i")
        .arg("-p")
        .arg("Select Wallpaper")
        .arg("-markup-rows")
        .arg("-config")
        .arg(rofi_config)
        .arg("-theme-str")
        .arg(theme_override)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .context("Failed to spawn rofi (for wallpaper)")?;

    rofi.stdin.as_mut().unwrap().write_all(rofi_input.as_bytes())?;
    let output = rofi.wait_with_output()?;
    if !output.status.success() {
        anyhow::bail!("Rofi was cancelled (wallpaper selection)");
    }
    
    let selection_name = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if selection_name.is_empty() {
        anyhow::bail!("No wallpaper selected.");
    }

    let selected_wp = wallpapers.into_iter()
        .find(|w| w.name == selection_name)
        .ok_or_else(|| anyhow!("Selected wallpaper not found in cache"))?;

    // ---
    // Call the "Dumb" Apply Script
    // We pass it all the answers so it doesn't have to think.
    // ---
    let current_exe = env::current_exe()
        .context("Failed to find path of our own executable")?;

    // Get the directory our executable lives in (e.g., /home/user/.cargo/bin)
    let bin_dir = current_exe.parent()
        .context("Failed to get parent directory of our executable")?;

    // Build the full, absolute path to our sibling 'wp-apply'
    let apply_path = bin_dir.join("wp-apply");

    // Call 'wp-apply' using its full path
    Command::new(apply_path)
        .arg(selected_wp.path)
        .arg(&compositor)
        .arg(&chosen_monitor)
        .spawn()
        .context("Failed to run 'wp-apply' command")?;

    Ok(())
}
