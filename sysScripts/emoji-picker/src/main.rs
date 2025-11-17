use anyhow::{anyhow, Context, Result};
use toml;
use shellexpand;
use emojis;
use serde::Deserialize;
use std::fs;
use std::io::Write;
use std::process::{Command, Stdio};

#[derive(Debug, Deserialize)]
struct EmojiConfig {
    rofi_config: String,
    message: String,
}
#[derive(Debug, Deserialize)]
struct GlobalConfig {
    emoji_picker: EmojiConfig,
}
//--- Config Loader ---
fn load_config() -> Result<GlobalConfig> {
    let config_path = shellexpand::tilde("~/.config/rust-dotfiles/config.toml").to_string();
    let config_str = fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read config file from path {}", config_path))?;
    let config: GlobalConfig = toml::from_str(&config_str)
        .context("Failed to parse config.toml. Check for syntax errors.")?;
    Ok(config)
}

fn build_emoji_list() -> String {
    let mut lines = Vec::new();
    // 1. Loop through the *entire* emojis database
    for emoji in emojis::iter() {
        // 2. Create a searchable line
        let line = format!(
            "{} {}",
            emoji.as_str(),
            emoji.name()
        );
        lines.push(line);
    }
    // 3. Join the Vec<String> into one big string
    lines.join("\n")
}
fn show_rofi(list: &str, config: &EmojiConfig) -> Result<String> {
    let rofi_config_path = shellexpand::tilde(&config.rofi_config).to_string();
    let mut child = Command::new("rofi")
        .arg("-i")
        .arg("-dmenu")
        .arg("-config")
        .arg(rofi_config_path)
        .arg("-mesg")
        .arg(&config.message)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .context("Failed to spawn rofi")?;

    // Pipe the list (e.g., "😀 grinning face...") to rofi's stdin
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(list.as_bytes())?;
    }
    let output = child.wait_with_output()?;

    if !output.status.success() && output.status.code() != Some(1) {
         return Err(anyhow!("Rofi failed with an error"));
    }
    // Return the user's selected line
    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}
fn parse_and_copy(selection: &str) -> Result<()> {
    // 1. Parse the line (replaces awk/head/tr)
    let emoji = match selection.split(' ').next() {
        Some(emoji_char) => emoji_char,
        None => return Ok(()), // Empty selection
    };

    // 2. Pipe to wl-copy
    let mut wl_copy_child = Command::new("wl-copy")
        .stdin(Stdio::piped())
        .spawn()
        .context("Failed to spawn 'wl-copy'")?;

    if let Some(mut stdin) = wl_copy_child.stdin.take() {
        stdin.write_all(emoji.as_bytes())?;
    }

    if !wl_copy_child.wait()?.success() {
        return Err(anyhow!("wl-copy failed"));
    }
    
    Ok(())
}
fn main() -> Result<()> {
    let config = load_config()?.emoji_picker;
    let emoji_list_string = build_emoji_list();
    let selection = show_rofi(&emoji_list_string, &config)?;
    if !selection.is_empty() {
        parse_and_copy(&selection)?;
    }
    Ok(())
}
