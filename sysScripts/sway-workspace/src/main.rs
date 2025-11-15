use std::process::Command;
use serde::Deserialize;
use anyhow::{Context, Result};
//template for parsed swaymsg output
#[derive(Deserialize)]
struct Workspace {
    num: i32,
    focused: bool,
    name: Option<String>,
}
fn main() -> Result<()> {
    //Run swaymsg to get workspaces
    let output = Command::new("swaymsg")
        .arg("-t")
        .arg("get_workspaces")
        .output()
        .context("Failed to run 'swaymsg' command")?;
    //check if swaymsg reported an error
    if !output.status.success() {
        anyhow::bail!("swaymsg failed: {}", String::from_utf8_lossy(&output.stderr));
    }
    //make sure that the output is valid UTF-8 text
    let stdout_str = String::from_utf8(output.stdout)
        .context("swaymsg output was not valid UTF-8")?;
    //parse workspace into Vec<workspace>
    let workspaces: Vec<Workspace> = serde_json::from_str(&stdout_str)
        .context("Failed to parse swaymsg JSON")?;

    //set default name
    let focused_name = workspaces
        .iter()
        .find(|ws| ws.focused)
        .map(|ws| {
            ws.name.clone().unwrap_or_else(|| ws.num.to_string())
        })
        .unwrap_or_else(|| "?".to_string());
    println!("{}", focused_name);
    Ok(())
}
