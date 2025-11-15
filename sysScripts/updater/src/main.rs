use std::fs; // We need this now
use std::process::{Command, Stdio};
use std::path::{Path, PathBuf}; // Use std::path::Path
use anyhow::{anyhow, Context, Result};
use notify_rust::{Notification, Urgency};
use serde::Deserialize;
use shlex; // <-- NEW
use toml; // <-- NEW
use shellexpand; // <-- NEW

// --- 1. NEW: Config Structs ---

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

// --- 2. NEW: Config Loader ---

fn load_config() -> Result<GlobalConfig> {
    let config_path = shellexpand::tilde("~/.config/rust-dotfiles/config.toml").to_string();
    let config_str = fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read config file from path: {}", config_path))?;
    let config: GlobalConfig = toml::from_str(&config_str)
        .context("Failed to parse config.toml. Check for syntax errors.")?;
    Ok(config)
}

// --- 3. Helper Functions (Unchanged) ---

fn check_dependency(cmd: &str) -> bool {
    Command::new(cmd)
        .arg("-h") // Using -h is a bit of a guess, --help is safer
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok()
}

// Note: Changed icon to take &Path
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

// --- 4. REFACTORED Main Function ---

fn main() -> Result<()> {
    // 1. Load config
    let config = load_config().context("Failed to load configuration")?;
    let global_conf = config.global;
    let updater_conf = config.updater;

    // 2. Resolve paths from config
    let icon_error = PathBuf::from(shellexpand::tilde(&updater_conf.icon_error).to_string());
    let icon_success = PathBuf::from(shellexpand::tilde(&updater_conf.icon_success).to_string());
    
    // 3. Check dependencies (from config)
    let terminal_cmd = &global_conf.terminal;
    let update_cmd_name = updater_conf.update_command.get(0)
        .context("'update_command' in config.toml is empty")?;

    if !check_dependency(terminal_cmd) {
        send_notification(
            "Error: Dependency Missing",
            &format!("Terminal not found: {}", terminal_cmd),
            &icon_error,
            Urgency::Critical,
        )?;
        return Err(anyhow!("Dependency missing: {}", terminal_cmd));
    }

    if !check_dependency(update_cmd_name) {
        send_notification(
            "Error: Dependency Missing",
            &format!("Update helper not found: {}", update_cmd_name),
            &icon_error,
            Urgency::Critical,
        )?;
        return Err(anyhow!("Dependency missing: {}", update_cmd_name));
    }

    // 4. Build the update script
    // Safely join the command parts (e.g., ["yay", "-Syu"] -> "yay -Syu")
    let update_cmd_str = shlex::join(updater_conf.update_command.iter().map(AsRef::as_ref));
    
    let update_script = format!(
        r#"
        {}
        exit_code=$?
        echo -e '\n\n🏁 Update process finished. This window will close in 5 seconds.'
        sleep 5
        exit $exit_code
    "#,
        update_cmd_str // Inject our config-driven command
    );

    // 5. Launch the terminal (from config)
    let status = Command::new(terminal_cmd) // Use config terminal
        .arg(format!("--title={}", updater_conf.window_title)) // Use config title
        .arg("-e")
        .arg("bash")
        .arg("-c")
        .arg(&update_script)
        .status()
        .context(format!("Failed to launch terminal: {}", terminal_cmd))?;
    
    // 6. Final notification (using config icons)
    if status.success() {
        send_notification(
            "System Update Complete",
            "Your Arch Linux system has been successfully updated.",
            &icon_success, // Use config icon
            Urgency::Low,
        )?;
    } else {
        send_notification(
            "System Update Failed",
            "The update process encountered an error.",
            &icon_error, // Use config icon
            Urgency::Critical,
        )?;
    }
    Ok(())
}
