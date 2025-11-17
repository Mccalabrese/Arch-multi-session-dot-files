use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::channel;
use::std::collections::HashSet;
use anyhow::{Context, Result};
use dirs;
use image::imageops::FilterType;
use notify::{RecursiveMode, Watcher};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;
use toml;
use shellexpand;

#[derive(Deserialize, Debug)]
struct WallpaperManagerConfig {
    wallpaper_dir: String,
    swww_params: Vec<String>,
    swaybg_cache_file: String,
    hyprland_refresh_script: String,
    cache_file: String,
    rofi_config_path: String,
    rofi_theme_override: String,
}

#[derive(Deserialize, Debug)]
struct GlobalConfig {
    wallpaper_manager: WallpaperManagerConfig,
}
fn load_config() -> Result<GlobalConfig> {
    let config_path = shellexpand::tilde("~/.config/rust-dotfiles/config.toml").to_string();

    let config_str = fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read config file from path: {}", config_path))?;

    let config: GlobalConfig = toml::from_str(&config_str)
        .context("Failed to parse config.toml. Check for syntax errors.")?;
    
    Ok(config)
}
#[derive(Serialize, Deserialize, Debug, Clone)]
struct Wallpaper {
    name: String,
    path: PathBuf,
    thumb_path: PathBuf,
}
const THUMB_WIDTH: u32 = 500;
fn ensure_thumbnail(original_path: &Path, thumb_dir: &Path) -> Option<PathBuf> {
    let file_name = original_path.file_name()?;
    let thumb_path = thumb_dir.join(file_name);
    //if thumbnail exists dont burn my battery
    if thumb_path.exists() {
        return Some(thumb_path);
    }
    //NOOO we need to burn battery, no thumbnail
    let img = match image::open(original_path) {
        Ok(img) => img,
        Err(_) => return None,
    };
    //Resize
    let thumb = img.resize(THUMB_WIDTH, u32::MAX, FilterType::Nearest);
    //Save
    if let Err(e) = thumb.save(&thumb_path) {
        eprintln!("Failed to save thumb for {:?}: {}", original_path, e);
        return None;
    }
    Some(thumb_path)
}
fn scan_and_update_cache(wall_dir: &Path, cache_file: &Path) -> Result<()> {
    let home = dirs::home_dir().context("Failed to get $HOME")?;
    let thumb_dir = home.join(".cache/wallpaper_thumbs");
    fs::create_dir_all(&thumb_dir)?;
    println!("Scanning wallpapers in {:?}...", wall_dir);
    //collect files
    let entries: Vec<PathBuf> = WalkDir::new(wall_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .map(|e| e.path().to_path_buf())
        .collect();
    //process them in parallel using Rayon
    let wallpapers: Vec<Wallpaper> = entries.par_iter()
        .filter_map(|path| {
            if let Some(ext) = path.extension() {
                let ext_str = ext.to_string_lossy().to_lowercase();
                if ext_str == "mp4" || ext_str == "webm" || ext_str == "mkv" {
                    return None;
                }
            }
            //Generate thumbnail
            let thumb = ensure_thumbnail(path, &thumb_dir)?;
            //Return the struct
            Some(Wallpaper {
                name: path.file_stem()?.to_string_lossy().to_string(),
                path: path.clone(),
                thumb_path: thumb,
            })
        })
        .collect();
    //Write the JSON catalog
    let json = serde_json::to_string(&wallpapers)?;
    fs::write(cache_file, json).context("Failed to write cache file")?;
    //Garbage Collection
    let good_thumbs: HashSet<PathBuf> = wallpapers.into_iter()
        .map(|w| w.thumb_path)
        .collect();
    for entry in fs::read_dir(&thumb_dir)? {
        let entry = entry?;
        let thumb_path = entry.path();
        if !good_thumbs.contains(&thumb_path) {
            println!("Garbage collecting old thumb: {:?}", thumb_path);
            let _ = fs::remove_file(thumb_path);
        }
    }
    println!("Cache update. Found {} wallpapers.", good_thumbs.len());
    Ok(())
}
fn main() -> Result<()> {
    let global_config = load_config()?;
    let config = global_config.wallpaper_manager;
    let wall_dir_str = shellexpand::tilde(&config.wallpaper_dir).to_string();
    let wall_dir = PathBuf::from(wall_dir_str);
    let cache_file_str = shellexpand::tilde(&config.cache_file).to_string();
    let cache_file = PathBuf::from(cache_file_str);
    // Verify the directory exists
    if !wall_dir.exists() {
        anyhow::bail!("Wallpaper directory does not exist: {:?}", wall_dir);
    }
    //Initial scan on startup
    if let Err(e) = scan_and_update_cache(&wall_dir, &cache_file) {
        eprintln!("Initial scan failed: {}", e);
    }
    //Setup the Watcher
    //Create a channel to receive events
    let (tx, rx) = channel();
    //Watcher object
    let mut watcher = notify::recommended_watcher(tx)?;
    //add path to be watched
    watcher.watch(&wall_dir, RecursiveMode::Recursive)?;
    println!("Daemon started. Watching {:?}...", wall_dir);
    //The loop - recv blocks the thread until an event arrives.
    for res in rx {
        match res {
            Ok(_) => {
                println!("Change detected. Refreshing cache...");
                if let Err(e) = scan_and_update_cache(&wall_dir, &cache_file) {
                    eprintln!("Error updating cache: {}", e);
                }
            },
            Err(e) => eprintln!("Watch error {:?}", e),
        }
    }
    Ok(())
}
