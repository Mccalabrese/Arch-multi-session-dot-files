use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use std::fs;
use std::io::Write; 
use std::process::{Command, Stdio};
use shellexpand;
use toml;

// --- Config Structs ---
#[derive(Deserialize, Debug)]
struct ClipConfig {
    rofi_config: String,
    message: String,
}

#[derive(Deserialize, Debug)]
struct GlobalConfig {
    clip_manager: ClipConfig,
}

// --- Config Loader ---
fn load_config() -> Result<GlobalConfig> {
    let config_path = shellexpand::tilde("~/.config/rust-dotfiles/config.toml").to_string();
    let config_str = fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read config file from path {}", config_path))?;
    let config: GlobalConfig = toml::from_str(&config_str)
        .context("Failed to parse config.toml. Check for syntax errors.")?;
    Ok(config)
}

// --- Logic ---

// Runs "cliphist list" and returns the stdout
fn get_cliphist_list() -> Result<String> {
    let output = Command::new("cliphist")
        .arg("list")
        .output()
        .context("Failed to run 'cliphist list'")?;
    
    if !output.status.success() {
        return Err(anyhow!("cliphist list failed"));
    }
    
    Ok(String::from_utf8(output.stdout)?)
}

// Runs "cliphist decode" on a selection and copies it to clipboard
fn decode_and_copy(selection: &str) -> Result<()> {
    // 1. Spawn `cliphist decode`
    let mut cliphist_child = Command::new("cliphist")
        .arg("decode")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped()) 
        .spawn()
        .context("Failed to spawn 'cliphist decode'")?;

    // 2. Write the selected item (from Rofi) to cliphist's stdin
    if let Some(mut stdin) = cliphist_child.stdin.take() {
        stdin.write_all(selection.as_bytes())
             .context("Failed to write to cliphist stdin")?;
    }

    // 3. Take ownership of cliphist's stdout stream
    let cliphist_stdout = cliphist_child.stdout.take()
        .context("Failed to get stdout from cliphist")?;

    // 4. Spawn `wl-copy` and tell it to use cliphist's stdout as its stdin
    let wl_copy_status = Command::new("wl-copy")
        .stdin(cliphist_stdout) 
        .status()
        .context("Failed to spawn 'wl-copy'")?;
    
    // 5. Wait for the original cliphist process to finish
    cliphist_child.wait()?;

    if !wl_copy_status.success() {
         return Err(anyhow!("wl-copy failed"));
    }

    Ok(())
}

// Runs "cliphist delete" on a selection
fn delete_entry(selection: &str) -> Result<()> {
    let mut child = Command::new("cliphist")
        .arg("delete")
        .stdin(Stdio::piped())
        .spawn()
        .context("Failed to spawn 'cliphist delete'")?;
    
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(selection.as_bytes())
             .context("Failed to write to cliphist stdin")?;
    }
    
    let status = child.wait()?;
    if !status.success() {
        return Err(anyhow!("cliphist delete failed"));
    }
    
    Ok(())
}

// Runs "cliphist wipe"
fn wipe_history() -> Result<()> {
    let status = Command::new("cliphist")
        .arg("wipe")
        .status()
        .context("Failed to run 'cliphist wipe'")?;

    if !status.success() {
        return Err(anyhow!("cliphist wipe failed"));
    }
    
    Ok(())
}

// Shows the Rofi menu and returns the selected item AND the exit code
fn show_rofi(list: &str, config: &ClipConfig) -> Result<(i32, String)> {
    let rofi_config_path = shellexpand::tilde(&config.rofi_config).to_string();

    let mut child = Command::new("rofi")
        .arg("-i") 
        .arg("-dmenu")
        .arg("-kb-custom-1")
        .arg("Control-Delete")
        .arg("-kb-custom-2")
        .arg("Alt-Delete")
        .arg("-config")
        .arg(rofi_config_path)
        .arg("-mesg")
        .arg(&config.message)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .context("Failed to spawn rofi")?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(list.as_bytes())?;
    }

    let output = child.wait_with_output()?;
    
    let selection = String::from_utf8(output.stdout)?.trim().to_string();

    let exit_code = output.status.code().unwrap_or(1); // Default to 1 (Esc) if it fails

    Ok((exit_code, selection))
}


fn main() -> Result<()> {

    loop {
        // 1. Load config and get history
        let config = load_config()?.clip_manager;
        let history_list = get_cliphist_list()?;

        // 2. Show Rofi and get the user's action
        let (exit_code, selection) = show_rofi(&history_list, &config)?;

        // 3. Match on the exit code (this replaces the 'case $?' in bash)
        match exit_code {
            0 => { // 0 = Enter
                if selection.is_empty() {
                    continue; // Rofi was launched but nothing selected
                }
                // User selected an item, so we copy it and exit
                decode_and_copy(&selection)?;
                break;
            }
            1 => break, // 1 = Esc, exit loop
            10 => { // 10 = Ctrl+Del
                // User wants to delete one item, so we delete and *loop again*
                delete_entry(&selection)?;
                continue; 
            }
            11 => { // 11 = Alt+Del
                // User wants to wipe history, so we wipe and *loop again*
                wipe_history()?;
                continue; 
            }
            _ => {
                // Unknown exit code, just exit
                break;
            }
        }
    } 

    Ok(())
}
