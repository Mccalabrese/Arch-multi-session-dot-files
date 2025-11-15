use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;

// --- Config Structs ---

#[derive(Deserialize, Debug)]
struct UpdateCheckConfig {
    command: Vec<String>,
    cache_file: String,
    stale_icon: String,
    error_icon: String,
}

#[derive(Deserialize, Debug)]
struct GlobalConfig {
    update_check: UpdateCheckConfig,
}

// --- Cache Struct ---

#[derive(Serialize, Deserialize, Debug)]
struct Cache {
    count: usize,
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

// --- Cache Read/Write Functions ---

fn read_cache(cache_path: &Path) -> Result<Cache> {
    let json_data = fs::read_to_string(cache_path)
        .context("Failed to read cache file")?;
    let cache: Cache = serde_json::from_str(&json_data)
        .context("Failed to parse cache JSON")?;
    Ok(cache)
}

fn save_cache(count: usize, cache_path: &Path) -> Result<()> {
    let cache = Cache { count };
    let json_data = serde_json::to_string(&cache)?;
    fs::write(cache_path, json_data)
        .context("Failed to write cache file")?;
    Ok(())
}

// --- Check Function (returns a count) ---

fn run_check(command_parts: &[String]) -> Result<usize> {
    let cmd_name = command_parts.get(0)
        .context("Update check 'command' in config.toml is empty")?;
    
    let args = &command_parts[1..];

    let output = Command::new(cmd_name)
        .args(args)
        .output()
        .context(format!("Failed to spawn command: '{}'", cmd_name))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Check command failed:\n{}", stderr);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let count = stdout.lines().count();
    Ok(count)
}

// --- JSON Output Functions ---

fn print_success_json(count: usize) {
    if count > 0 {
        println!("{}", json!({
            "text": count.to_string(),
            "tooltip": format!("{} Updates Available", count),
            "class": "updates"
        }));
    } else {
        println!("{}", json!({
            "text": "0",
            "tooltip": "System is up to date",
            "class": "synced"
        }));
    }
}

fn print_stale_json(stale_count: usize, config: &UpdateCheckConfig) {
    println!("{}", json!({
        "text": format!("{} {}", stale_count, config.stale_icon),
        "tooltip": format!(
            "Update check failed. Showing last known count: {}", 
            stale_count
        ),
        "class": "stale"
    }));
}

fn print_error_json(config: &UpdateCheckConfig, error_msg: &str) {
    println!("{}", json!({
        "text": config.error_icon.clone(),
        "tooltip": format!("Update check failed:\n{}", error_msg),
        "class": "error"
    }));
}

// --- Main Logic ---

fn main() -> Result<()> {
    let config = match load_config() {
        Ok(global_config) => global_config.update_check,
        Err(e) => {
            // Config-level error is unrecoverable
            println!("{}", json!({
                "text": "!",
                "tooltip": format!("Failed to load config.toml:\n{}", e),
                "class": "error"
            }));
            return Err(e);
        }
    };
    
    let cache_path = PathBuf::from(shellexpand::tilde(&config.cache_file).to_string());

    match run_check(&config.command) {
        Ok(count) => {
            // Success! Save to cache and print.
            if let Err(e) = save_cache(count, &cache_path) {
                eprintln!("Warning: Failed to save cache: {}", e);
            }
            print_success_json(count);
        }
        Err(check_err) => {
            // Check failed. Try to read from cache.
            eprintln!("Update check failed: {}", check_err); // For debugging
            match read_cache(&cache_path) {
                Ok(cache) => {
                    // Cache read worked, print stale data.
                    print_stale_json(cache.count, &config);
                }
                Err(cache_err) => {
                    // Total failure: check AND cache read failed.
                    let combined_err = format!("Check Error: {}\nCache Error: {}", check_err, cache_err);
                    print_error_json(&config, &combined_err);
                }
            }
        }
    }

    Ok(())
}
