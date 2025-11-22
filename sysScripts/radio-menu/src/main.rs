use anyhow::{anyhow, Context, Result};
use notify_rust::Notification;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

// --- Helper: Expand Path ---
fn expand_path(path: &str) -> PathBuf {
    if path.starts_with("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(&path[2..]);
        }
    }
    PathBuf::from(path)
}

// --- Structs ---
#[derive(Deserialize, Serialize, Debug, Clone)]
struct Station {
    name: String,
    url_resolved: String,
    tags: String,
    stationuuid: String,
}

#[derive(Deserialize, Debug)]
struct RadioConfig {
    rofi_config: String,
    message: String,
}

#[derive(Deserialize, Debug)]
struct GlobalConfig {
    radio_menu: RadioConfig,
}

// --- Paths & Config ---
fn get_config_path() -> PathBuf {
    dirs::home_dir()
        .expect("Could not find home directory")
        .join(".config/rust-dotfiles/config.toml")
}

fn get_favorites_path() -> PathBuf {
    dirs::home_dir()
        .expect("Could not find home directory")
        .join(".config/rust-dotfiles/radio_favorites.json")
}

fn load_config() -> Result<GlobalConfig> {
    let path = get_config_path();
    let content = fs::read_to_string(&path).context("Failed to read config.toml")?;
    let config: GlobalConfig = toml::from_str(&content).context("Failed to parse config.toml")?;
    Ok(config)
}

// --- Logic: Radio API ---
fn search_stations(query: &str) -> Result<Vec<Station>> {
    let url = format!("https://de1.api.radio-browser.info/json/stations/byname/{}", query);
    let response = reqwest::blocking::get(&url)?.json::<Vec<Station>>()?;
    Ok(response.into_iter().take(15).collect())
}

// --- Logic: Favorites ---
fn load_favorites() -> Result<Vec<Station>> {
    let path = get_favorites_path();
    if !path.exists() {
        return Ok(Vec::new());
    }
    let data = fs::read_to_string(path)?;
    let stations: Vec<Station> = serde_json::from_str(&data).unwrap_or_default();
    Ok(stations)
}

fn save_favorite(station: Station) -> Result<()> {
    let path = get_favorites_path();
    let mut favorites = load_favorites()?;
    if !favorites.iter().any(|s| s.stationuuid == station.stationuuid) {
        favorites.push(station);
        let json = serde_json::to_string_pretty(&favorites)?;
        fs::write(path, json)?;
    }
    Ok(())
}

fn remove_favorite(station_name: &str) -> Result<()> {
    let path = get_favorites_path();
    let mut favorites = load_favorites()?;
    favorites.retain(|s| s.name != station_name);
    let json = serde_json::to_string_pretty(&favorites)?;
    fs::write(path, json)?;
    Ok(())
}

// --- Logic: Player ---
fn stop_radio() {
    let _ = Command::new("pkill").arg("-x").arg("mpv").status();
}

fn play_station(station_name: &str, url: &str) -> Result<()> {
    stop_radio();
    
    Command::new("mpv")
        .arg("--no-video")
        .arg(url)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context("Failed to spawn mpv")?;

    let _ = Notification::new()
        .summary("Radio Playing")
        .body(station_name)
        .icon("media-playback-start")
        .show();
        
    Ok(())
}

// --- UI: Rofi ---
fn show_rofi(options: &[String], prompt: &str, config: &RadioConfig, custom_message: Option<&str>) -> Result<(i32, String)> {
    let input = options.join("\n");
    let rofi_config_path = expand_path(&config.rofi_config);
    let msg = custom_message.unwrap_or(&config.message);

    let mut child = Command::new("rofi")
        .arg("-dmenu")
        .arg("-i")
        .arg("-p")
        .arg(prompt)
        .arg("-config")
        .arg(rofi_config_path)
        .arg("-mesg")
        .arg(msg)
        .arg("-markup-rows")
        .arg("-kb-custom-1")
        .arg("Control+s")
        .arg("-kb-custom-2")
        .arg("Control+r")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(input.as_bytes())?;
    }

    let output = child.wait_with_output()?;
    
    let code = output.status.code().unwrap_or(1);

    if code != 0 && code != 1 && code != 10 && code != 11 {
        return Err(anyhow!("Rofi failed with exit code: {}", code));
    }

    let selection = String::from_utf8(output.stdout)?.trim().to_string();

    Ok((code, selection))
}

// --- Main Flow ---
fn main() -> Result<()> {
    let global_config = load_config()?;
    let config = global_config.radio_menu;
    
    loop {
        let favorites = load_favorites()?;
        
        let mut menu_options = Vec::new();
        menu_options.push("⏹ Stop Radio".to_string());
        menu_options.push("🔍 Search Online...".to_string());
        
        for station in &favorites {
            menu_options.push(format!("⭐ {}", station.name));
        }

        let (code, selection) = show_rofi(
            &menu_options, 
            "Radio", 
            &config, 
            Some("<b>Enter:</b> Play  |  <b>Ctrl+R:</b> Remove Favorite")
        )?;

        if code == 1 { break; } // Esc from Main Menu

        if selection == "⏹ Stop Radio" {
            stop_radio();
            let _ = Notification::new().summary("Radio").body("Stopped").show();
            break; 
        } else if selection == "🔍 Search Online..." {
            // --- SEARCH LOOP (NEW LOGIC) ---
            loop {
                // 1. Get Query
                let (s_code, query) = show_rofi(&[], "Search Query", &config, Some("Type to search station name..."))?;
                
                // If Esc (1) or empty query, break back to Main Menu
                if s_code == 1 || query.is_empty() { break; }

                // 2. Perform Search
                let results = search_stations(&query)?;

                // 3. Handle No Results
                if results.is_empty() {
                    // Show error dialog
                    let (retry_code, _) = show_rofi(
                        &["🔄 Try Again".to_string()], 
                        "No Results", 
                        &config, 
                        Some(&format!("No stations found for '<b>{}</b>'", query))
                    )?;
                    
                    // If they hit Esc on the error dialog, go back to Main Menu
                    if retry_code == 1 { break; }
                    
                    // Otherwise (Enter on "Try Again"), 'continue' loops back to search bar
                    continue;
                }

                // 4. Show Results
                let result_names: Vec<String> = results.iter().map(|s| s.name.clone()).collect();
                let (r_code, picked_name) = show_rofi(
                    &result_names, 
                    "Results", 
                    &config, 
                    Some("<b>Enter:</b> Play  |  <b>Ctrl+S:</b> Save to Favorites  |  <b>Esc:</b> Back to Search")
                )?;
                
                // If Esc on results list, go back to search bar
                if r_code == 1 { continue; }

                // Handle Selection
                if let Some(station) = results.into_iter().find(|s| s.name == picked_name) {
                    if r_code == 10 { 
                        // Ctrl+S -> Save
                        save_favorite(station.clone())?;
                        play_station(&station.name, &station.url_resolved)?;
                        let _ = Notification::new().summary("Radio").body("Station Saved").show();
                        // Exit completely after playing
                        return Ok(()); 
                    } else if r_code == 0 {
                        // Enter -> Play
                        play_station(&station.name, &station.url_resolved)?;
                        // Exit completely after playing
                        return Ok(());
                    }
                }
            }
            // If we break out of search loop, we break out of main loop too (close app)
            break;

        } else if selection.starts_with("⭐ ") { 
            let clean_name = selection.strip_prefix("⭐ ").unwrap_or(&selection);
            
            if code == 11 {
                // Ctrl+R: Remove Favorite
                remove_favorite(clean_name)?;
                let _ = Notification::new().summary("Radio").body("Favorite Removed").show();
                continue; // Loop main menu to refresh list
            } else {
                // Enter: Play Favorite
                if let Some(station) = favorites.iter().find(|s| s.name == clean_name) {
                    play_station(&station.name, &station.url_resolved)?;
                }
                break;
            }
        } else {
            break;
        }
    }

    Ok(())
}
