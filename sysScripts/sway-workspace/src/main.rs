use anyhow::{Context, Result};
use swayipc::Connection;

fn main() -> Result<()> {
    let mut connection = Connection::new()
        .context("Failed to connect to sway IPC. Is sway running?")?;
    let workspaces = connection.get_workspaces()
        .context("Failed to fetch workspaces")?;

    // 3. Find the focused workspace
    let focused_name = workspaces
        .into_iter()
        .find(|ws| ws.focused) // Find the one where 'focused' is true
        .map(|ws| ws.name) // Get its 'name' (which is "1", "2: www", etc.)
        .unwrap_or_else(|| "?".to_string()); // Fallback

    println!("{}", focused_name);
    
    Ok(())
}
