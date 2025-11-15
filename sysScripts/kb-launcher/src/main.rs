use serde::Deserialize;
use std::fs;
use anyhow::{Context, Result};
use shellexpand;
use toml;
use std::io::Write;
use std::process::{Command, Stdio};
use sysinfo::System;
use std::ffi::OsStr;

#[derive(Deserialize, Debug)]
struct Sheet {
    name: String,
    file: String,
}

#[derive(Deserialize, Debug)]
struct CompositorArgs {
    hyprland: Vec<String>,
    sway: Vec<String>,
    niri: Vec<String>,
    default: Vec<String>,
}

#[derive(Deserialize, Debug)]
struct Global {
    terminal: String,
    pager: String,
}

#[derive(Deserialize, Debug)]
struct KbLauncherConfig {
    compositor_args: CompositorArgs,
    sheet: Vec<Sheet>,
}

#[derive(Deserialize, Debug)]
struct GlobalConfig {
    global: Global,
    kb_launcher: KbLauncherConfig,
}
//Finds and parses the gloable config file
fn load_config() -> Result<GlobalConfig> {
    //Get the path to the config file (shellexpand::tilde())
    let config_path = shellexpand::tilde("~/.config/rust-dotfiles/config.toml").to_string();
    //Read the files contents into a string(read_to_string())
    let config_str = fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read config file from path {}", config_path))?;
        
    //Parse the String into our GlobalConfig struct(from_str())
    let config: GlobalConfig = toml::from_str(&config_str)
        .context("Failed to parse config file")?;
    //return the config (or an error)
    return Ok(config)
}
fn get_compositor() -> String {
    let sys = System::new_all();

    if sys.processes_by_name(OsStr::new("niri")).next().is_some() {
        "niri".to_string()
    } else if sys.processes_by_name(OsStr::new("Hyprland")).next().is_some() {
        "hyprland".to_string()
    } else if sys.processes_by_name(OsStr::new("sway")).next().is_some() {
        "sway".to_string()
    } else {
        "default".to_string() // Use "default" as a fallback
    }
}
fn show_rofi_menu(sheets: &[Sheet]) -> Result<String> {
    let menu_string = sheets
        .iter()
        .map(|s| s.name.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let mut child = Command::new("rofi")
        .arg("-dmenu")
        .arg("-i")
        .arg("-p")
        .arg("View Cheat Sheet:")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .context("Failed to spawn rofi. Is it installed and in your $PATH?")?;
    //Write out menu string to rofi's stdin
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(menu_string.as_bytes())
            .context("Failed to write to rofi stdin")?;
    }
    //Wait for rofi to exit and get its output
    let output = child.wait_with_output()
        .context("Failed to wait for rofi to exit")?;
    if !output.status.success() {
        //Not an error, just the user hitting esc in rofi
        anyhow::bail!("No selection made in rofi.");
    }
    //convert the raw stdout bytes (Vec<u8>) to a string
    let choice = String::from_utf8(output.stdout)
        .context("Failed to parse rofi output as UTF-8")?;
    Ok(choice.trim().to_string())

}
fn main() -> Result<()> {
    //load the configuration
    let global_config = load_config()?;
    let global_conf = global_config.global;
    let kb_config = global_config.kb_launcher;
    //show the rofi menu
    let chosen_sheet_name = show_rofi_menu(&kb_config.sheet)?;
    //find the matching sheet struct from our config
    let chosen_sheet = kb_config.sheet
        .iter()
        .find(|s| s.name == chosen_sheet_name)
        .context("Failed to find chosen sheet")?;
    //expand the ~ in the file path
    let sheet_path = shellexpand::tilde(&chosen_sheet.file).to_string();
    let compositor = get_compositor();
    //terminal arguments for specific compositor
    let compositor_args = match compositor.as_str() {
        "hyprland" => &kb_config.compositor_args.hyprland,
        "sway" => &kb_config.compositor_args.sway,
        "niri" => &kb_config.compositor_args.niri,
        _ => &kb_config.compositor_args.default,
    };
    //construct the inner shell command
    let inner_cmd = format!("{} '{}'; printf %s 'Press any key to close...' read -n 1 -s -r", global_conf.pager, sheet_path);
    //spawn the terminal comand
    Command::new(&global_conf.terminal)
        .args(compositor_args)
        .arg("-e")
        .arg("sh")
        .arg("-c")
        .arg(&inner_cmd)
        .spawn()
        .context(format!("Failed to spawn terminal: {}", global_conf.terminal))?;
    Ok(())
}
