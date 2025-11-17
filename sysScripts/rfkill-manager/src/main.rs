use anyhow::{anyhow, Context, Result};
use libc;
use notify_rust::Notification; // <-- Our new crate!
use serde::Deserialize;
use serde_json::json;
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::process::Command;
use sysinfo::System;
use shellexpand;
use toml;

// --- Config Structs ---
#[derive(Deserialize, Debug)]
struct RfkillConfig {
    icon: String,
    text_on: String,
    class_on: String,
    tooltip_on: String,
    text_off: String,
    class_off: String,
    tooltip_off: String,
    bar_process_name: String,
    bar_signal_num: i32,
}

#[derive(Deserialize, Debug)]
struct GlobalConfig {
    rfkill_toggle: RfkillConfig,
}

// --- Config Loader (Copied from our other projects) ---
fn load_config() -> Result<GlobalConfig> {
    let config_path = shellexpand::tilde("~/.config/rust-dotfiles/config.toml").to_string();
    let config_str = fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read config file from path {}", config_path))?;
    let config: GlobalConfig = toml::from_str(&config_str)
        .context("Failed to parse config.toml. Check for syntax errors.")?;
    Ok(config)
}
fn is_blocked() -> Result<bool> {
    let output = Command::new("rfkill")
        .arg("list")
        .arg("all")
        .output()
        .context("Failed to run 'rfkill list'")?;

    if !output.status.success() {
        return Err(anyhow!(
            "rfkill list command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.contains("Soft blocked: yes"))
}
fn run_status(config: &RfkillConfig) -> Result<()> {
    let (text, class, tooltip) = match is_blocked() {
        Ok(true) => (config.text_on.as_str(), config.class_on.as_str(), config.tooltip_on.as_str(),),
        Ok(false) => (config.text_off.as_str(), config.class_off.as_str(), config.tooltip_off.as_str(),),
        Err(e) => {
            eprintln!("rfkill-manager status error: {}", e);
            ("?", "error", "Error checking rfkill")
        }
    };
    println!("{}", json!({
        "text": text,
        "class": class,
        "tooltip": tooltip
    }));
    Ok(())
}
fn run_toggle(config: &RfkillConfig) -> Result<()> {
    //check current state
    let blocked = is_blocked().context("Failed to check rfkill state before toggle")?;
    //set action
    let (action, message) = if blocked {
        ("unblock", "Airplane Mode: OFF")
    } else {
        ("block", "Airplane Mode: ON")
    };
    //Run toggle command
    let status = Command::new("rfkill")
        .arg(action)
        .arg("all")
        .status()?;
    if !status.success() {
        return Err(anyhow!("rfkill {} command failed", action));
    }
    //send notification
    let icon_path = shellexpand::tilde(&config.icon).to_string();
    Notification::new()
        .summary("Airplane Mode")
        .body(message)
        .icon(&icon_path)
        .show()?;
    //send signal to Waybar
    let sys = System::new_all();
    if let Some(waybar_pid) = sys.processes_by_name(OsStr::new(&config.bar_process_name)).next() {
        let pid = waybar_pid.pid().as_u32() as i32;
        let signal_num = libc::SIGRTMIN() + config.bar_signal_num;
        unsafe { libc::kill(pid, signal_num) };
    }
    Ok(())
}
// --- Main Logic ---
fn main() -> Result<()> {
    //get args
    let args: Vec<String> = env::args().collect();
    let mode = args.get(1).map(|s| s.as_str());
    //load config
    let config =load_config()?.rfkill_toggle;
    //match on mode
    match mode {
        Some("--status") => {
            if let Err(e) = run_status(&config) {
                eprintln!("{}", e);
            }
        }
        Some("--toggle") | None => {
            if let Err(e) = run_toggle(&config) {
                Notification::new()
                    .summary("Airplane Mode Error")
                    .body(&e.to_string())
                    .icon("dialog-error")
                    .show()?;
            }
        }
        _ => {
            println!("Unknown argument. Use --status or --toggle.");
        }
    }
    Ok(())
}
