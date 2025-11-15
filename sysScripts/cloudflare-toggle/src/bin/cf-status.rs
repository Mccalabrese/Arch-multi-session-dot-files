use std::fs;
use std::process::Command;
use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::json;
use toml;
use shellexpand;

// --- Config Structs ---
#[derive(Deserialize, Debug)]
struct Config {
    text_on: String,
    class_on: String,
    text_off: String,
    class_off: String,
    // Add other fields so serde doesn't complain
    resolv_content_on: String,
    resolv_content_off: String,
    bar_process_name: String,
    bar_signal_num: i32,
}

#[derive(Deserialize, Debug)]
struct GlobalConfig {
    cloudflare_toggle: Config,
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

fn main() -> Result<()> {
    let config = load_config().map(|gc| gc.cloudflare_toggle);
    //check if systemd service is active
    let service_active = Command::new("systemctl")
        .arg("is-active")
        .arg("cloudflared-dns")
        .output()?
        .status
        .success();
    //check /etc/resolv.conf
    let resolv_conf = std::fs::read_to_string("/etc/resolv.conf")
        .unwrap_or_else(|_| "Error".to_string());
    let (text, class, tooltip) = if service_active {
        (
            config.as_ref().map_or("ON", |c| &c.text_on),
            config.as_ref().map_or("on", |c| &c.class_on),
            format!("Cloudflared:Running\nresolv.conf: {}", resolv_conf)
        )
    } else {
        (
            config.as_ref().map_or("OFF", |c| &c.text_off),
            config.as_ref().map_or("off", |c| &c.class_off),
            format!("Cloudflared: Stopped\nresolv.conf: {}", resolv_conf)
        )
    };
    //print json for waybar
    println!("{}", json!({
        "text": text,
        "class": class,
        "tooltip": tooltip
    }));
    Ok(())
}
