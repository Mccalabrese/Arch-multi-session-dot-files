use std::fs;
use std::process::{Command, Stdio};
use std::path::{Path, PathBuf};
use anyhow::{anyhow, Context, Result};
use notify_rust::{Notification, Urgency};
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
// --- Config Structs ---

#[derive(Deserialize, Debug)]
struct Global {
    terminal: String,
}

#[derive(Deserialize, Debug)]
struct UpdaterConfig {
    update_command: Vec<String>,
    icon_success: String,
    icon_error: String,
    window_title: String,
}

#[derive(Deserialize, Debug)]
struct GlobalConfig {
    global: Global,
    updater: UpdaterConfig,
}

// --- Config Loader ---

fn load_config() -> Result<GlobalConfig> {
    let config_path = dirs::home_dir()
        .context("Cannot find home dir")?
        .join(".config/rust-dotfiles/config.toml");

    let config_str = fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read config: {}", config_path.display()))?;

    let config: GlobalConfig = toml::from_str(&config_str)
        .context("Failed to parse config.toml")?;

    Ok(config)
}

// --- Helper Functions ---

fn check_dependency(cmd: &str) -> bool {
    Command::new(cmd)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn send_notification(summary: &str, body: &str, icon: &Path, urgency: Urgency) -> Result<()> {
    Notification::new()
        .summary(summary)
        .body(body)
        .icon(icon.to_str().unwrap_or(""))
        .urgency(urgency)
        .show()
        .context("Failed to send desktop notification")?;
    Ok(())
}

// --- Main Function ---

fn main() -> Result<()> {
    let config = load_config()?;
    let global_conf = config.global;
    let updater_conf = config.updater;

    // Resolve paths from config
    let icon_error = expand_path(&updater_conf.icon_error);
    let icon_success = expand_path(&updater_conf.icon_success);
    
    // Check dependencies (from config)
    let terminal_cmd = &global_conf.terminal;
    let update_bin = updater_conf.update_command.get(0)
        .context("'update_command' in config.toml is empty")?;

    if !check_dependency(terminal_cmd) {
        let _ = send_notification(
            "Error: Dependency Missing",
            &format!("Terminal not found: {}", terminal_cmd),
            &icon_error,
            Urgency::Critical,
        );
        return Err(anyhow!("Dependency missing: {}", terminal_cmd));
    }

    if !check_dependency(update_bin) {
        let _ = send_notification(
            "Error: Dependency Missing",
            &format!("Update helper not found: {}", update_bin),
            &icon_error,
            Urgency::Critical,
        );
        return Err(anyhow!("Dependency missing: {}", update_bin));
    }

    // Build the update script
    // Safely join the command parts (e.g., ["yay", "-Syu"] -> "yay -Syu")
    let update_cmd_str = updater_conf.update_command.join(" ");
    
    let bash_script = format!("{} \n exit_code=$? \n echo -e '\\n\\n🏁 Update process finished. This window will close in 5 seconds.' \n sleep 5 \n exit $exit_code",
        update_cmd_str
    );

    // Launch the terminal 
    let status = Command::new(terminal_cmd)
        .arg(format!("--title={}", updater_conf.window_title))
        .arg("-e")
        .arg("bash")
        .arg("-c")
        .arg(&bash_script)
        .status()
        .context(format!("Failed to launch terminal: {}", terminal_cmd))?;
    
    // Final notification (using config icons)
    if status.success() {
        send_notification(
            "System Update Complete",
            "Your Arch Linux system has been successfully updated.",
            &icon_success,
            Urgency::Low,
        )?;
    } else {
        send_notification(
            "System Update Failed",
            "The update process encountered an error.",
            &icon_error,
            Urgency::Critical,
        )?;
    }
    Ok(())
}
