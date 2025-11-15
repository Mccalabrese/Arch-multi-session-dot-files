use std::env;
use std::fs;
use std::process::Command;
use anyhow::{Context, Result};
use sysinfo::System; 
use libc;
use serde::Deserialize; 
use toml; 
use shellexpand; 
use std::ffi::OsStr; 

// --- 1. Config Structs (Same as status.rs) ---
#[derive(Deserialize, Debug)]
struct Config {
    text_on: String,
    class_on: String,
    text_off: String,
    class_off: String,
    resolv_content_on: String,
    resolv_content_off: String,
    bar_process_name: String,
    bar_signal_num: i32,
}

#[derive(Deserialize, Debug)]
struct GlobalConfig {
    cloudflare_toggle: Config,
}

// --- 2. Config Loader (Same as status.rs) ---
fn load_config() -> Result<GlobalConfig> {
    let config_path = shellexpand::tilde("~/.config/rust-dotfiles/config.toml").to_string();
    let config_str = fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read config file from path: {}", config_path))?;
    let config: GlobalConfig = toml::from_str(&config_str)
        .context("Failed to parse config.toml. Check for syntax errors.")?;
    Ok(config)
}

// --- 3. run_as_user ---
fn run_as_user() -> Result<()> {
    // Load config for Waybar signal
    let config = load_config()
        .context("Failed to load config for user")?
        .cloudflare_toggle;

    // Check status 
    let is_running = Command::new("systemctl")
        .arg("is-active")
        .arg("cloudflared-dns")
        .output()?
        .status
        .success();
    
    let mode = if is_running { "--stop" } else { "--start" };

    // Run pkexec 
    let self_exe = env::current_exe()
        .context("Failed to get path to own executable")?;
    let status = Command::new("pkexec")
        .arg(self_exe)
        .arg(mode)
        .status()
        .context("Failed to run pkexec")?;

    // Send signal using config values
    if status.success() {
        let mut sys = System::new_all();
        // Use OsStr::new() for modern sysinfo
        if let Some(waybar_pid) = sys.processes_by_name(OsStr::new(&config.bar_process_name)).next() {
            let pid = waybar_pid.pid().as_u32() as i32;
            // Use signal num from config
            let signal_num = libc::SIGRTMIN() + config.bar_signal_num; 
            let result = unsafe { libc::kill(pid, signal_num) };
            if result == -1 {
                return Err(anyhow::anyhow!("Failed to send signal to {}", config.bar_process_name));
            }
        }
    }
    Ok(())
}

// --- 4. run_as_root ---
fn run_as_root(mode: &str) -> Result<()> {
    // Load config for resolv.conf content
    let config = load_config()
        .context("Failed to load config for root")?
        .cloudflare_toggle;

    if mode == "--start" {
        // Start service
        Command::new("systemctl")
            .arg("enable")
            .arg("--now")
            .arg("cloudflared-dns")
            .status()?
            .success()
            .then_some(())
            .context("Failed to start systemctl service")?;

        // Write resolv.conf from config
        fs::write("/etc/resolv.conf", &config.resolv_content_on)
            .context("Failed to write /etc/resolv.conf")?;
        Ok(())

    } else if mode == "--stop" {
        // Stop service 
        Command::new("systemctl")
            .arg("disable")
            .arg("--now")
            .arg("cloudflared-dns")
            .status()?
            .success()
            .then_some(())
            .context("Failed to stop systemctl service")?;
        
        // Write resolv.conf from config
        fs::write("/etc/resolv.conf", &config.resolv_content_off)
            .context("Failed to write /etc/resolv.conf")?;
        Ok(())
    } else {
        Ok(())
    }
}

// --- 5. Main ---
fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if let Some(mode) = args.get(1) {
        run_as_root(mode)
    } else {
        run_as_user()
    }
}
