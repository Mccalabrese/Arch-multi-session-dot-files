use std::process::Command;
use anyhow::{Context, Result};
use serde::Deserialize; 

#[derive(Deserialize, Debug)]
struct SwayWorkspace {
    num: i32,
    name: String, 
    focused: bool,
}

fn main() -> Result<()> {
    // 1. Run swaymsg
    let output = Command::new("swaymsg")
        .arg("-t")
        .arg("get_workspaces")
        .output()
        .context("Failed to run 'swaymsg' command")?;

    if !output.status.success() {
        anyhow::bail!("swaymsg failed: {}", String::from_utf8_lossy(&output.stderr));
    }

    // 2. Parse the JSON from the raw bytes (output.stdout)
    let workspaces: Vec<SwayWorkspace> = serde_json::from_slice(&output.stdout)
        .context("Failed to parse swaymsg JSON. The output was not JSON.")?;

    // 3. Find the focused workspace
    let focused_name = workspaces
        .iter()
        .find(|ws| ws.focused) // Find the one where 'focused' is true
        .map(|ws| ws.name.clone()) // Get its 'name' (which is "1", "2: www", etc.)
        .unwrap_or_else(|| "?".to_string()); // Fallback

    println!("{}", focused_name);
    
    Ok(())
}
