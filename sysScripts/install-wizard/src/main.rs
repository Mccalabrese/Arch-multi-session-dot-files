//! Arch Linux Production Installer
//!
//! A comprehensive system provisioning tool written in Rust.
//! Designed to take a fresh Arch Linux installation (base + git) and transform it 
//! into a fully configured, multi-session Wayland environment (Niri, Hyprland, Sway).
//!
//! Core Responsibilities:
//! 1. **Hardware Detection:** Automatically identifies GPU vendors (NVIDIA/AMD/Intel) 
//!    via `lspci` and installs the appropriate drivers/VAAPI packages.
//! 2. **Package Management:** Orchestrates `pacman` (official repo) and `yay` (AUR) installations.
//! 3. **Security Hardening:** Configures UFW, Polkit, and secure directory permissions.
//! 4. **Config Deployment:** Links dotfiles and generates machine-specific secrets (API keys) 
//!    securely without storing them in git.
//! 5. **Safety:** Implements "Fail Fast" logic—if a critical step fails, the installer halts immediately.

use colored::*;
use inquire::{Select, Text};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::io::Write;

// --- Enums for Hardware Detection ---
#[derive(Debug, PartialEq)]
enum GpuVendor {
    Nvidia,
    Amd,
    Intel,
    Unknown,
}

// --- Packages ---
// Const for auditing and immutability

const RUST_APPS: &[&str] = &[
    // My custom toolchain
    "waybar-switcher", "waybar-weather", "sway-workspace", "update-check",
    "cloudflare-toggle", "wallpaper-manager", "kb-launcher", "updater",
    "power-menu", "rfkill-manager", "clip-manager", "emoji-picker",
    "radio-menu", "waybar-finance",
];

// Core System (Safe for ALL hardware)
const COMMON_PACKAGES: &[&str] = &[
    // Build / Core
    "base-devel", "git", "go", "rustup", "openssl", "pkgconf", "glibc", "wget", "curl", "jq",
    "man-db", "man-pages", "unzip", "tree", "linux-headers", "pciutils", "pacman-contrib",
    
    // Hardware (Generic)
    "bolt", "upower", "tlp", "bluez", "bluez-utils", "blueman", 
    "brightnessctl", "udiskie", "fwupd",
    "intel-media-driver", "libva-utils", "vulkan-intel", 

    // Compositors & Desktop
    "sway", "hyprland", "niri", "gnome", "hyprlock", "swayidle", "hypridle",
    "xdg-user-dirs-gtk", "greetd", "greetd-tuigreet",

    // Wayland Infra
    "xwayland-satellite", "qt5-wayland", "qt6-wayland", "polkit-gnome", 
    "geoclue", "xdg-desktop-portal-gnome", "xdg-desktop-portal-wlr", "xdg-desktop-portal-gtk",
    "wl-clipboard", "cliphist",

    // Audio
    "pipewire", "pipewire-pulse", "pipewire-alsa", "pipewire-jack",
    "wireplumber", "pavucontrol", "sof-firmware",

    // File Mgmt
    "thunar", "thunar-volman", "tumbler", "gvfs", "gvfs-mtp", "gvfs-smb", "gvfs-gphoto2", 
    "file-roller", "gnome-disk-utility",

    // Security / UI
    "ufw", "timeshift", "seahorse", "gnome-keyring",
    "waybar", "wofi", "rofi", "swaync", "swww", "swaybg", "grim", "slurp", "mako",
    "papirus-icon-theme", "gnome-themes-extra", "adwaita-icon-theme",

    // Fonts
    "ttf-jetbrains-mono-nerd", "ttf-fira-code", "ttf-jetbrains-mono",
    "noto-fonts", "noto-fonts-emoji", "otf-font-awesome",

    // Shell / Apps
    "zsh", "starship", "ghostty", "tmux", "fzf", "ripgrep", "bat", "btop", "fastfetch", "neovim",
    "networkmanager", "network-manager-applet", "cloudflared",
    "firefox", "discord", "tigervnc", "mpv", "gparted", "simple-scan", "gnome-calculator",
    "cups", "system-config-printer", "cups-pdf", "zsh-autosuggestions", "zsh-syntax-highlighting"
];

// Hardware Specific: NVIDIA
const NVIDIA_PACKAGES: &[&str] = &[
    "nvidia-dkms", "nvidia-prime", "nvidia-settings", "libva-nvidia-driver"
];

// Hardware Specific: AMD
const AMD_PACKAGES: &[&str] = &[
    "vulkan-radeon", "libva-mesa-driver", "mesa-vdpau", "xf86-video-amdgpu"
];

// AUR
const AUR_PACKAGES: &[&str] = &[
    "wlogout", "zoom", "slack-desktop", "ledger-live-bin", 
    "visual-studio-code-bin", "pinta", "ttf-victor-mono", "ytmdesktop-bin"
];
// ---------- Main Execution ------_-------
fn main() {
    print_logo();
    println!("{}", "🚀 Starting Rust Wayland Power Installation...".green().bold());

    // 1. Elevate Privileges
    // I check sudo access early, on new arch installs I ran into a permissions issue
    // 'sudo -v' updates the credentials cache, hopefully avoiding timeouts on slower machines
    let status = Command::new("sudo")
        .arg("-v")
        .status()
        .unwrap_or_else(|_| {
            eprintln!("Failed to execute sudo");
            std::process::exit(1);
        });
    if !status.success() {
        eprintln!("{}", "❌ Sudo privileges are required.".red());
        std::process::exit(1);
    }

    // 2. Install Common Packages (Pacman)
    println!("\n{}", "📦 Installing Common Packages...".blue().bold());
    install_pacman_packages(COMMON_PACKAGES);

    // --- NEW: STATE CHECKPOINT LOGIC ---
    let state_file = dirs::home_dir().unwrap().join(".cache/rust_installer_drivers_done");

    if state_file.exists() {
        println!("\n{}", "✅ Drivers already installed (Checkpoint found). Skipping to prevent crash.".green());
    } else {
        println!("\n{}", "🔍 Detecting GPU Hardware...".blue().bold());
        let gpu = detect_gpu();

        // RUN DRIVER INSTALL
        match gpu {
            GpuVendor::Nvidia => {
                println!("   👉 NVIDIA Detected.");
                install_pacman_packages(NVIDIA_PACKAGES);
                apply_nvidia_configs();
            },
            GpuVendor::Amd => {
                println!("   👉 AMD Detected.");
                install_pacman_packages(AMD_PACKAGES);
            },
            GpuVendor::Intel => println!("   👉 Intel Detected (Drivers in common)."),
            GpuVendor::Unknown => println!("   ⚠️  No dedicated GPU detected."),
        }

        // CHECK: Are we in a GUI?
        let is_gui = std::env::var("WAYLAND_DISPLAY").is_ok() || std::env::var("DISPLAY").is_ok();

        if is_gui {
            println!("\n{}", "⚠️  GRAPHICS DRIVERS INSTALLED".yellow().bold());
            println!("We must reboot to load the new kernel modules safely.");
            println!("If we continue now, your session will crash.");
            
            // Create the checkpoint file
            if let Ok(mut file) = fs::File::create(&state_file) {
                writeln!(file, "Drivers installed successfully.").unwrap();
            }

            println!("{}", "✅ Checkpoint saved. Please REBOOT and RUN THIS SCRIPT AGAIN to finish.".green().bold());
            println!("The script will automatically detect this step is done next time.");
            
            // Optional: Ask to reboot now
            let should_reboot = inquire::Confirm::new("Reboot now?")
                .with_default(true)
                .prompt()
                .unwrap_or(true);

            if should_reboot {
                let _ = Command::new("sudo").arg("reboot").status();
            }
            
            std::process::exit(0); // STOP HERE
        }
    }
    // ------
    // 4. AUR
    // This will istall yay for a user to handle community packages (VS Code, Slack, etc)
    #[allow(clippy::const_is_empty)]
    if !AUR_PACKAGES.is_empty() {
        println!("\n{}", "📦 Setting up AUR...".blue().bold());
        install_aur_packages();
    }

    // 5. System Config & hardening
    println!("\n{}", "⚙️  Applying System Configurations...".blue().bold());
    configure_system(); //greetd, logind
    enforce_session_order();
    optimize_pacman_config(); //cleans session list and prevents updates from overriding 
    
    // 6. Rust Apps
    println!("\n{}", "🦀 Setting up Rust & Building Tools...".blue().bold());
    let _ = Command::new("rustup").args(["default", "stable"]).status();
    // Compiles custom rust scripts, installs to ~/.cargo/bin
    build_custom_apps();

    // 7. Link Dotfiles & Copy Wallpapers
    println!("\n{}", "🔗 Linking Config Files & Resources...".blue().bold());
    // I'm using symlinks to keep the git repo as the source of truth
    // Copies wallpapers (to allow user modification without messing with my repo).
    link_dotfiles_and_copy_resources();
    
    // 8. Setup Waybar Configs
    println!("\n{}", "🎨 Configuring Waybar...".blue().bold());
    setup_waybar_configs();

    // 9. Secrets & Final Configs
    println!("\n{}", "🔑 Configuring Secrets & API Keys...".blue().bold());
    //Heres where API prompts will happen
    //~/.config/rust-dotfiles/config.toml keeps users keys out of repo.
    setup_secrets_and_geoclue();

    println!("\n{}", "✅ Installation Complete! Please Reboot.".green().bold());
    print_logo();
}

// --- Helper functions ---

/// Parses `lspci` output to identify GPU vendor IDs.
/// 10de = NVIDIA, 1002 = AMD, 8086 = Intel.
fn detect_gpu() -> GpuVendor {
    let output = Command::new("lspci")
        .arg("-n") //Numeric ID's, string parsing maybe isn't reliable according to the Goog.
        .output();

    match output {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout).to_lowercase();
            if stdout.contains("10de:") { return GpuVendor::Nvidia; }
            if stdout.contains("1002:") { return GpuVendor::Amd; }
            if stdout.contains("8086:") { return GpuVendor::Intel; }
            GpuVendor::Unknown
        },
        Err(_) => {
            println!("   ⚠️  lspci failed. Skipping auto-detection.");
            GpuVendor::Unknown
        }
    }
}
/// appends content to a root-owned file using 'sudo tee -a'
/// I think this was the safest way to do it without exposing to shell injection
fn append_to_root_file(file_path: &str, content: &str) -> std::io::Result<()> {
    // 1. Create the child process 'sudo tee -a file_path'
    let mut child = Command::new("sudo")
        .arg("tee")
        .arg("-a")
        .arg(file_path)
        .stdin(Stdio::piped()) // Pipe content into stdin
        .stdout(Stdio::null()) // Suppress output to terminal
        .spawn()?;

    // 2. Write the content to the child's stdin
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(content.as_bytes())?;
    }

    // 3. Wait for process to finish
    let status = child.wait()?;

    if status.success() {
        Ok(())
    } else {
        Err(std::io::Error::new(std::io::ErrorKind::Other, "sudo tee failed"))
    }
}
//-------- Main Steps ------


/// installs packages via pacman with --needed and --noconfirm
fn install_pacman_packages(packages: &[&str]) {
    if packages.is_empty() { return; }
    let mut args = vec!["-S", "--needed", "--noconfirm"];
    args.extend(packages);
    let status = Command::new("sudo")
        .arg("pacman")
        .args(&args)
        .status()
        .unwrap_or_else(|_| {
            eprintln!("❌ pacman not found or failed to execute.");
            std::process::exit(1);
        });

    // UPDATE: Make failure fatal!
    if !status.success() { 
        eprintln!("{}", "❌ Critical Error: Pacman failed to install packages.".red());
        std::process::exit(1); 
    }
}
/// Bootstraps 'yay' from the AUR git repo if not present.
/// This allows the script to run on a truly clean Arch install.
fn install_aur_packages() {
    let yay_check = Command::new("which").arg("yay").output();
    
    if yay_check.is_err() || !yay_check.unwrap().status.success() {
        println!("   ⬇️  Bootstrapping 'yay'...");
        let home = dirs::home_dir().unwrap_or_else(|| {
             eprintln!("⚠️ Could not determine home directory. Using /tmp as fallback.");
             PathBuf::from("/tmp")
        });        
        let clone_path = home.join("yay-clone");

        if clone_path.exists() { let _ = fs::remove_dir_all(&clone_path); }

        let _ = Command::new("git").args(["clone", "https://aur.archlinux.org/yay.git", clone_path.to_str().unwrap()]).status();
        let status = Command::new("makepkg").arg("-si").arg("--noconfirm").current_dir(&clone_path).status();

        if status.is_err() || !status.unwrap().success() {
            println!("{}", "❌ Failed to bootstrap yay.".red());
            return;
        }
        let _ = fs::remove_dir_all(&clone_path);
    }

    let mut args = vec!["-S", "--needed", "--noconfirm"];
    args.extend(AUR_PACKAGES);
    let status = Command::new("yay")
        .args(&args)
        .status()
        .unwrap_or_else(|_| {
            eprintln!("❌ Failed to run yay");
            std::process::exit(1);
        });
    if !status.success() { eprintln!("{}", "⚠️  AUR Warning.".yellow()); }
}
/// Configures critical system services.
/// 1. Disables systemd-resolved (we use Cloudflared/Dnsmasq).
/// 2. Configures `greetd` (tuigreet) as the display manager.
/// 3. Sets `KillUserProcesses=yes` to prevent lingering sessions.
fn configure_system() {
    // --- 1. SANITIZE MKINITCPIO (Fix Archinstall 2025 Bug) ---
    // This protects NVIDIA users from the 'o"' corruption crash.
    println!("   🧹 Checking mkinitcpio.conf for corruption...");
    let mkinit_path = "/etc/mkinitcpio.conf";
    
    // 1. Check if the file specifically ends with the garbage (ignoring whitespace)
    // We read it first to be safe, rather than firing sed blindly.
    if let Ok(content) = fs::read_to_string(mkinit_path) {
        let trimmed = content.trim(); // Removes trailing \n
        if trimmed.ends_with("o\"") || trimmed.ends_with("o”") {
            println!("   ⚠️  Corruption detected at end of file. Cleaning up...");
            
            // 2. Safe Delete: Only delete the last line ($) if it matches the pattern
            // usage: sed -i '${/^o"$/d}' filename
            let _ = Command::new("sudo")
                .args(["sed", "-i", "${/^o\"$/d}", mkinit_path])
                .status();
                
            // Extra safety: Removing the smart-quote variation just in case
            let _ = Command::new("sudo")
                .args(["sed", "-i", "${/^o”$/d}", mkinit_path])
                .status();
        }
    }
    run_cmd("sudo", &["systemctl", "enable", "geoclue.service"]);
    run_cmd("sudo", &["systemctl", "enable", "bluetooth.service"]);
    run_cmd("sudo", &["systemctl", "enable", "bolt.service"]);

    // --- CLOUDFLARED CONFIGURATION ---
    println!("   🔧 Configuring Cloudflared (DNS Proxy)...");
    
    // 1. Write the Config File
    let cf_config = "proxy-dns: true\nproxy-dns-upstream:\n  - https://1.1.1.1/dns-query\n  - https://1.0.0.1/dns-query\nproxy-dns-port: 53\nproxy-dns-address: 127.0.0.1\n";
    let _ = Command::new("sudo").args(["mkdir", "-p", "/etc/cloudflared"]).status();
    
    let local_cf_conf = "./config.yml";
    if fs::write(local_cf_conf, cf_config).is_ok() {
        let _ = Command::new("sudo").args(["install", "-m", "644", local_cf_conf, "/etc/cloudflared/config.yml"]).status();
        let _ = fs::remove_file(local_cf_conf);
    }

    // 2. Create the Service File
    let cf_service_content = r#"[Unit]
Description=Cloudflared DNS over HTTPS Proxy
After=network.target

[Service]
ExecStart=/usr/bin/cloudflared --config /etc/cloudflared/config.yml
Restart=on-failure
User=root

[Install]
WantedBy=multi-user.target
"#;

    let local_cf_svc = "./cloudflared-dns.service";
    if fs::write(local_cf_svc, cf_service_content).is_ok() {
        let _ = Command::new("sudo").args(["install", "-m", "644", local_cf_svc, "/etc/systemd/system/cloudflared-dns.service"]).status();
        let _ = fs::remove_file(local_cf_svc);
    }

    // 3. Enable it
    run_cmd("sudo", &["systemctl", "daemon-reload"]);
    // Disable the default 'cloudflared' service if installed by pacman to avoid conflicts
    let _ = Command::new("sudo").args(["systemctl", "disable", "--now", "cloudflared"]).status();
    // Enable our custom service
    run_cmd("sudo", &["systemctl", "enable", "cloudflared-dns.service"]);
    println!("   🔧 Configuring Session Environment (PATH)...");
    let env_dir = dirs::home_dir().unwrap().join(".config/environment.d");
    let env_file = env_dir.join("99-cargo-path.conf");

    // Create dir if missing
    if let Ok(_) = fs::create_dir_all(&env_dir) {
        // We write the variable assignment directly
        let content = "PATH=$HOME/.cargo/bin:$PATH\n";
        
        if let Err(e) = fs::write(&env_file, content) {
            eprintln!("   ⚠️ Failed to write environment.d config: {}", e);
        } else {
            println!("   ✅ Global PATH configured for Wayland.");
        }
    }
    println!("   🔧 Configuring Logind...");
    let logind_conf = "/etc/systemd/logind.conf";
    run_cmd("sudo", &["sed", "-i", "s/#KillUserProcesses=no/KillUserProcesses=yes/", logind_conf]);
    run_cmd("sudo", &["sed", "-i", "s/KillUserProcesses=no/KillUserProcesses=yes/", logind_conf]);

    println!("   🔧 Configuring Greetd...");
    let greetd_config = r#"
[terminal]
vt = 1
[default_session]
command = "tuigreet --time --remember --sessions /usr/share/wayland-sessions:/usr/share/xsessions"
user = "greeter"
"#;
    // SECURE FIX: Write to local dir (we own it) instead of /tmp (race condition)
    let _ = fs::write("./greetd_config.toml", greetd_config);
    run_cmd("sudo", &["mv", "./greetd_config.toml", "/etc/greetd/config.toml"]);
    // 1. Disable competitors FIRST to free up the symlink
    // We use status() and ignore errors because these might not be installed
    let _ = Command::new("sudo").args(["systemctl", "disable", "gdm", "sddm", "lightdm"]).status();

    // 2. Enable Greetd with --force to overwrite /etc/systemd/system/display-manager.service
    run_cmd("sudo", &["systemctl", "enable", "--force", "greetd.service"]);
    println!("   🔧 Setting Shell to Zsh...");
    let user = std::env::var("USER").unwrap_or_else(|_| {
        eprintln!("⚠️  Could not detect $USER, defaulting to root");
        "root".to_string()
    });
    let _ = Command::new("sudo").args(["chsh", "-s", "/usr/bin/zsh", &user]).output();
}

fn run_cmd(cmd: &str, args: &[&str]) {
    let status = Command::new(cmd).args(args).status();
    match status {
        Ok(s) if s.success() => {}, // All good
        _ => {
            eprintln!("❌ Critical Error: Failed to run {} {:?}", cmd, args);
            std::process::exit(1);
        }
    }
}
/// Gleans pacman.conf to remove unwanted sessions and prevent future installs.
/// Gnome installs a lot of sessions we don't need, this keeps the list clean.
fn optimize_pacman_config() {
    println!("   🔧 Optimizing pacman.conf & Cleaning Sessions...");
    
    let sessions_to_remove = vec![
        "/usr/share/wayland-sessions/gnome.desktop",
        "/usr/share/wayland-sessions/gnome-classic.desktop",
        "/usr/share/wayland-sessions/gnome-classic-wayland.desktop",
        "/usr/share/wayland-sessions/hyprland-uwsm.desktop"
    ];

    for session in sessions_to_remove {
        let _ = Command::new("sudo").args(["rm", "-f", session]).output();
    }

    let pacman_conf = "/etc/pacman.conf";
    let content = fs::read_to_string(pacman_conf).unwrap_or_default();
    
    if !content.contains("NoExtract = usr/share/wayland-sessions/niri.desktop") {
        println!("   👉 Injecting NoExtract rules into [options]...");
        
        let no_extract_line = "NoExtract = usr/share/wayland-sessions/niri.desktop usr/share/wayland-sessions/hyprland.desktop usr/share/wayland-sessions/sway.desktop usr/share/wayland-sessions/gnome.desktop usr/share/wayland-sessions/gnome-classic.desktop usr/share/wayland-sessions/gnome-classic-wayland.desktop usr/share/wayland-sessions/hyprland-uwsm.desktop usr/share/wayland-sessions/gnome-wayland.desktop";
        
        // Use sed to append ('a') after the line matching '[options]'
        let sed_cmd = format!("/^\\[options\\]/a {}", no_extract_line);
        
        let status = Command::new("sudo")
            .args(["sed", "-i", &sed_cmd, pacman_conf])
            .status();

        match status {
            Ok(s) if s.success() => println!("   ✅ Added NoExtract rules to pacman.conf"),
            _ => eprintln!("   ❌ Failed to patch pacman.conf"),
        }
    } else {
        println!("   ℹ️  NoExtract rules already present.");
    }
}
/// Applies specific fixes for NVIDIA on Wayland.
/// 1. Sets kernel parameters (`nvidia_drm.modeset=1`).
/// 2. Creates modprobe rules to fix suspend/resume.
/// 3. Rebuilds initramfs via `mkinitcpio`.
/// 
/// Security Note: Uses a secure temp file pattern for writing to /etc/.
fn apply_nvidia_configs() {
    println!("   Applying Nvidia Configs...");
    
    // Helper closure: Write to local dir (safe) then install
    let install_securely = |content: &str, dest: &str| {
        let filename = Path::new(dest).file_name().unwrap().to_str().unwrap();
        let local_tmp = format!("./{}", filename);
        
        if let Err(e) = fs::write(&local_tmp, content) {
            eprintln!("❌ Failed to write local file {}: {}", local_tmp, e);
            std::process::exit(1);
        }

        // Use 'install' to copy with root:root ownership and 644 permissions
        let status = Command::new("sudo")
            .args(["install", "-m", "644", "-o", "root", "-g", "root", &local_tmp, dest])
            .status();

        match status {
            Ok(s) if s.success() => {
                 let _ = fs::remove_file(&local_tmp); // Cleanup
            },
            _ => {
                eprintln!("⚠️  Failed to install {} to {}", local_tmp, dest);
            }
        }
    };

    install_securely(
        "options nvidia NVreg_EnableGpuFirmware=0 NVreg_DynamicPowerManagement=0x02 NVreg_EnableS0ixPowerManagement=1\n",
        "/etc/modprobe.d/nvidia.conf"
    );

    install_securely(
        "blacklist nvidia_uvm\n",
        "/etc/modprobe.d/99-nvidia-uvm-blacklist.conf"
    );

    install_securely(
        "SUBSYSTEM==\"pci\", ATTR{vendor}==\"0x10de\", ATTR{power/control}=\"auto\"\n",
        "/etc/udev/rules.d/90-nvidia-pm.rules"
    );

    // GRUB Configuration
    let grub_path = "/etc/default/grub";
    println!("   🔧 Checking GRUB for NVIDIA modeset...");
    let content = fs::read_to_string(grub_path).unwrap_or_default();

    if !content.contains("nvidia_drm.modeset=1") {
        println!("   👉 Adding nvidia_drm.modeset=1 to GRUB...");
        let status = Command::new("sudo")
            .args([
                "sed", "-i", 
                "s/GRUB_CMDLINE_LINUX_DEFAULT=\"[^\"]*/& nvidia_drm.modeset=1/", 
                grub_path
            ])
            .status()
            .expect("Failed to patch GRUB");
            
        if !status.success() {
             println!("   ⚠️  Failed to patch GRUB. Please manually add nvidia_drm.modeset=1");
        }
    }

    println!("   🏗️  Rebuilding Initramfs & GRUB...");
    let _ = Command::new("sudo").args(["mkinitcpio", "-P"]).status();
    let _ = Command::new("sudo").args(["grub-mkconfig", "-o", "/boot/grub/grub.cfg"]).status();
}
///I templated my waybar configs to allow gitignore of my personalization.
///This unpacks them if they don't already exist.
fn setup_waybar_configs() {
    let home = dirs::home_dir().unwrap_or_else(|| {
        eprintln!("⚠️ Could not determine home directory. Using /tmp as fallback.");
        PathBuf::from("/tmp")
    });
    let waybar_dir = home.join(".config/waybar");
    let configs = vec!["hyprConfig.jsonc", "swayConfig.jsonc", "niriConfig.jsonc"];

    for config in configs {
        let template = waybar_dir.join(format!("{}.template", config));
        let target = waybar_dir.join(config);

        if template.exists() && !target.exists() {
            match fs::copy(&template, &target) {
                Ok(_) => println!("   ✅ Created {} from template", config),
                Err(e) => println!("   ⚠️  Failed to create {}: {}", config, e),
            }
        } else if target.exists() {
             println!("   ℹ️  {} already exists", config);
        }
    }
}
/// Interactive wizard to generate the local `config.toml`.
/// Validates input to prevent injection attacks before writing to system files (like /etc/geoclue).
fn setup_secrets_and_geoclue() {
    let home = dirs::home_dir().unwrap_or_else(|| {
        eprintln!("⚠️ Could not determine home directory. Using /tmp as fallback.");
        PathBuf::from("/tmp")
    });
    let config_dir = home.join(".config/rust-dotfiles");
    let config_path = config_dir.join("config.toml");

    let wallpaper_path = home.join("Pictures/Wallpapers");
    fs::create_dir_all(&wallpaper_path).expect("Failed to create wallpaper dir");

    println!("   🧙 We need to generate your central config.toml and configure Location Services.");
    
    let weather_api = Text::new("Enter OpenWeatherMap API Key (get one by making a free account at https://home.openweathermap.org/users/sign_up):").prompt().unwrap_or_else(|e| { eprintln!("❌ Error: {}", e); std::process::exit(1); });
    let finnhub_api = Text::new("Enter Finnhub.io API Key (get one by making a free account at finnhub.io/register):").prompt().unwrap_or_else(|e| { eprintln!("❌ Error: {}", e); std::process::exit(1); });
    
    // SECURE FIX: Validation logic for keys to prevent injection
    let google_geo_api = Text::new("Enter Google Geolocation API Key for Geoclue(get one at console.cloud.google.com/apis/library/geocoding-backend.googleapis.com):").prompt().unwrap_or_else(|e| { eprintln!("❌ Error: {}", e); std::process::exit(1); });
    
    // Simple validation: alphanumeric + underscores/hyphens only
    let is_valid_key = |k: &str| k.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-');
    
    // --- GEOCLUE CONFIGURATION ---
    if !google_geo_api.is_empty() {
        println!("   🌍 Configuring Geoclue...");
        let gc_path = "/etc/geoclue/geoclue.conf";

        // 1. Ensure the wifi source is enabled (uncomment 'enable=true')
        // We use a loose match to catch ';enable=true' or '#enable=true'
        let _ = Command::new("sudo").args(["sed", "-i", "s/^.*enable=true/enable=true/", gc_path]).status();

        // 2. Inject the Key
        // We look for the placeholder URL provided by the package and replace it.
        // The default line usually looks like:
        // #url=https://www.googleapis.com/geolocation/v1/geolocate?key=YOUR_KEY
        
        // We construct a regex-like sed command to find the googleapis line (commented or not) 
        // and replace the WHOLE line with our active key.
        let new_url = format!("url=https://www.googleapis.com/geolocation/v1/geolocate?key={}", google_geo_api);
        
        // This sed command finds any line containing "googleapis.com" and replaces the entire line.
        let status = Command::new("sudo")
            .args(["sed", "-i", &format!("s|^.*googleapis.com.*|{}|", new_url), gc_path])
            .status();

        match status {
             Ok(s) if s.success() => {
                 let _ = Command::new("sudo").args(["systemctl", "restart", "geoclue.service"]).output();
                 println!("   ✅ Geoclue Configured");
             },
             _ => eprintln!("   ❌ Failed to patch geoclue config."),
        }
    } else {
        println!("   ⚠️  No Google API Key provided. Location services may fail.");
    }
    let term_choice = Select::new("Preferred Terminal:", vec!["ghostty", "alacritty", "kitty"]).prompt().unwrap_or("ghostty");
    if config_path.exists() {
        println!("   ℹ️  config.toml already exists. Skipping write.");
        return;
    }

    let config_content = format!(
r#"[global]
pager = "bat --paging=always --style=plain"
terminal = "{}"

[waybar_weather]
owm_api_key = "{}"

[waybar_finance]
api_key = "{}"
stocks = ["SPY", "QQQ", "NVDA"]

[wallpaper_manager]
wallpaper_dir = "{}/Pictures/Wallpapers"
swww_params = ["--transition-fps", "60", "--transition-type", "any", "--transition-duration", "2"]
swaybg_cache_file = "swaybg_last_wallpaper"
hyprland_refresh_script = "~/.config/hypr/scripts/Refresh.sh"
cache_file = "~/.cache/wallpapers.json"
rofi_config_path = "~/.config/rofi/config-wallpaper.rasi"
rofi_theme_override = "element-icon {{ size: 20%; }}"

[update_check]
command_string = "nm-online -q -t 5 && (checkupdates; yay -Qua) || true"
cache_file = "~/.cache/update-check.json"
stale_icon = "⚠"
error_icon = "!"

[updater]
update_command = ["yay", "-Syu"]
icon_success = "~/.config/swaync/images/ja.png"
icon_error = "~/.config/swaync/images/error.png"
window_title = "System Update"

[waybar_switcher]
target_file = "/tmp/waybar-config.jsonc"
niri_config = "~/.config/waybar/niriConfig.jsonc"
hyprland_config = "~/.config/waybar/hyprConfig.jsonc"
sway_config = "~/.config/waybar/swayConfig.jsonc"

[cloudflare_toggle]
text_on = "󰅟"
class_on = "on"
text_off = "⚠︎"
class_off = "off"
resolv_content_on = "nameserver 127.0.0.1"
resolv_content_off = "nameserver 1.1.1.1\nnameserver 1.0.0.1"
bar_process_name = "waybar"
bar_signal_num = 10

[rfkill_toggle]
icon = "~/.config/swaync/images/ja.png"
text_on = "✈️️"
class_on = "on"
tooltip_on = "Airplane Mode: ON"
text_off = "󰀝"
class_off = "off"
tooltip_off = "Airplane Mode: OFF"
bar_process_name = "waybar"
bar_signal_num = 11

[clip_manager]
rofi_config = "~/.config/rofi/config-clipboard.rasi"
message = "CTRL+DEL = Delete Entry | ALT+DEL = Wipe History"

[emoji_picker]
rofi_config = "~/.config/rofi/config-emoji.rasi"
message = "Search Emojis (Name or Keyword)"

[radio_menu]
rofi_config = "~/.config/rofi/config-radio.rasi"
message = "Radio Menu"

[power_menu]
columns = 6
[power_menu.res_2160]
top_margin = 600.0
bottom_margin = 600.0
[power_menu.res_1600]
top_margin = 400.0
bottom_margin = 400.0
[power_menu.res_1440]
top_margin = 400.0
bottom_margin = 400.0
[power_menu.res_1080]
top_margin = 200.0
bottom_margin = 200.0
[power_menu.res_720]
top_margin = 50.0
bottom_margin = 50.0
columns = 3

[kb_launcher.compositor_args]
hyprland = ["--title=KeybindCheatSheetApp"]
sway = ["--title=KeybindCheatSheetApp"]
niri = ["--title=KeybindCheatSheet"]
default = []
[[kb_launcher.sheet]]
name = "Niri"
file = "~/.config/niri/keybinds_niri.txt"

[[kb_launcher.sheet]]
name = "Sway"
file = "~/.config/sway/keybinds_sway.txt"

[[kb_launcher.sheet]]
name = "Hyprland"
file = "~/.config/hypr/keybinds_hypr.txt"

[[kb_launcher.sheet]]
name = "Neovim"
file = "~/.config/nvim/keybinds_nvim.txt"
"#, 
    term_choice, 
    weather_api, 
    finnhub_api, 
    home.display()
    );

    fs::create_dir_all(&config_dir).expect("Failed to create config dir");
    fs::write(&config_path, config_content).expect("Failed to write config.toml");
    println!("   ✅ Config generated at {:?}", config_path);
}
///Build out custom rust apps from sysScripts directory.
fn build_custom_apps() {
    let current_dir = std::env::current_dir().unwrap();
    let sys_scripts_dir = current_dir.parent().unwrap();

    for app in RUST_APPS {
        let app_path = sys_scripts_dir.join(app);
        if app_path.exists() {
            println!("   🔨 Building {}...", app);
            let status = Command::new("cargo").arg("install").arg("--path").arg(".").current_dir(&app_path).stdout(Stdio::null()).status();
            match status {
                Ok(s) if s.success() => println!("     ✅ {}", app),
                _ => println!("     ❌ Failed to build {}", app),
            }
        } else {
            println!("     ⚠️  Missing directory for {}", app);
        }
    }
}

/// Renames session files to enforce a specific order in Greetd/Tuigreet.
/// 1. Niri, 2. Sway, 3. Hyprland, 4. Gnome
fn enforce_session_order() {
    println!("   🔧 Enforcing Session Order (Niri -> Sway -> Hyprland)...");
    
    let updates = vec![
        ("niri.desktop", "1. Niri"),
        ("sway.desktop", "2. Sway (Battery)"),
        ("hyprland.desktop", "3. Hyprland"),
        ("gnome.desktop", "4. Gnome"),
        ("gnome-wayland.desktop", "4. Gnome"), // Handle Arch variation
    ];

    for (file, new_name) in updates {
        let path = format!("/usr/share/wayland-sessions/{}", file);
        if Path::new(&path).exists() {
            // Use sed to replace the Name= line
            // s/^Name=.*/Name=1. Niri/
            let sed_cmd = format!("s/^Name=.*/Name={}/", new_name);
            let _ = Command::new("sudo")
                .args(["sed", "-i", &sed_cmd, &path])
                .status();
        }
    }
}

///Walks through dotfiles in repo and symlinks them to home directory.
fn link_dotfiles_and_copy_resources() {
    let home = dirs::home_dir().unwrap_or_else(|| {
        eprintln!("⚠️ Could not determine home directory. Using /tmp as fallback.");
        PathBuf::from("/tmp")
    });
    let current_dir = std::env::current_dir().unwrap();
    // Assuming binary is in sysScripts/install-wizard, repo root is 2 levels up
    let repo_root = current_dir.parent().unwrap().parent().unwrap();

    let links = vec![
        (".tmux.conf", ".tmux.conf"), (".profile", ".profile"), (".zshrc", ".zshrc"),
        (".config/waybar", ".config/waybar"), (".config/sway", ".config/sway"),
        (".config/hypr", ".config/hypr"), (".config/niri", ".config/niri"),
        (".config/rofi", ".config/rofi"), (".config/swaync", ".config/swaync"),
        (".config/ghostty", ".config/ghostty"), (".config/fastfetch", ".config/fastfetch"),
        (".config/wlogout", ".config/wlogout"),
        (".config/gtk-3.0", ".config/gtk-3.0"), (".config/gtk-4.0", ".config/gtk-4.0"),
        (".config/environment.d", ".config/environment.d"), (".config/mako", ".config/mako"),
    ];

    for (src, dest) in links {
        let src_path = repo_root.join(src);
        let dest_path = home.join(dest);
        create_symlink(&src_path, &dest_path);
    }
    // --- SPECIAL HANDLING FOR NEOVIM ---
    // We only install this if the user has NO config, to avoid angering Vim power users.
    let nvim_dest = home.join(".config/nvim");
    if nvim_dest.exists() {
        println!("   ℹ️  Neovim config found. Skipping to preserve your setup. If you would like my setup just copy ~/rust-wayland-power/.config/nvim to ~/.config/nvim");
        println!("      (Note: The 'Neovim' cheat sheet in kb-launcher may not work)");
    } else {
        println!("   ✨ Installing LazyVim Config...");
        let nvim_src = repo_root.join(".config/nvim");
        create_symlink(&nvim_src, &nvim_dest);
    }
    // Link TLP
    let tlp_src = repo_root.join("tlp.conf");
    let _ = Command::new("sudo").args(["ln", "-sf", tlp_src.to_str().unwrap(), "/etc/tlp.conf"]).status();
    let _ = Command::new("sudo").args(["systemctl", "enable", "tlp.service"]).output();

    // Copy Wallpapers
    println!("   🖼️  Seeding default wallpapers...");
    let wallpaper_src = repo_root.join("wallpapers");
    let wallpaper_dest = home.join("Pictures/Wallpapers");
    
    if wallpaper_src.exists() {
        if let Ok(entries) = fs::read_dir(&wallpaper_src) {
            fs::create_dir_all(&wallpaper_dest).unwrap_or_else(|e| {
                eprintln!("❌ Failed to create wallpaper destination dir: {}", e);
                std::process::exit(1);
            });
            for entry in entries.flatten() {
                let file_name = entry.file_name();
                let dest_path = wallpaper_dest.join(&file_name);
                if !dest_path.exists() {
                    let _ = fs::copy(entry.path(), dest_path);
                }
            }
            println!("   ✅ Copied wallpapers to ~/Pictures/Wallpapers");
        }
    } else {
        println!("   ⚠️  'wallpapers' directory not found in repo root.");
    }
    println!("   🏠 Updating User Directories (XDG)...");
    // This regenerates ~/.config/user-dirs.dirs and ~/.config/gtk-3.0/bookmarks
    // ensuring they point to the *current* user's home, not Michael's.
    let _ = Command::new("xdg-user-dirs-update").status();
}
///Helper to create symlinks, backing up existing files if needed.
fn create_symlink(src: &Path, dest: &Path) {
    if dest.exists() && !dest.is_symlink() {
        let backup = format!("{}.backup", dest.to_string_lossy());
        let _ = fs::rename(dest, &backup);
    }
    if let Some(parent) = dest.parent() { let _ = fs::create_dir_all(parent); }
    if dest.is_symlink() { let _ = fs::remove_file(dest); }
    #[cfg(unix)]
    std::os::unix::fs::symlink(src, dest).unwrap_or_else(|_| eprintln!("Failed to link {:?}", dest));
}

fn print_logo() {
println!(r#"
                                                                                                    
                                             ++++++++++                                             
                                           ++++++++++++++                                           
                                          ++++++++++++++++                                          
                                         ++++++++++++++++++                                         
                                        ++++++++++++++++++++                                        
                                       +++++++++====+++++++++                                       
                                       ++++++=:......:=++++++                                       
                                      +++++=:..........:=+++++                                      
                                      ++++=..............=++++                                      
                                      +++=.=##=......=##-.=+++                                      
                                     ++++:-%%-.-....-%%:.-:++++                                     
                                     +++=.*%%. *....#%%..*.=+++                                     
                                     +++-.#%%#*%....%%%###.-+++                                     
                                     +++-.#%%%%#....#%%%%#.-+++                                     
                                     +++-.+%%%%*....*%%%%+.-+++                                     
                                      ++=.:#%%#:....:#%%#:.=++                                      
                                      +++..:=+:......:+=:..+++                                      
                                     ++++-................-++++                                     
                                     +++++:..............:+++++                                     
                                    +++++++:............:+++++++                                    
                                   +++++**+:............:+**+++++                                   
                                   ++++****+=::......::=+****++++                                   
                                  +++++*********++++*********+++++                                  
                                  +++++++******************+++++++                                  
                                  ++++++:.-+***************:++++++                                  
                                 +++++++....::--------::***-+++++++                                 
                                 ++++++-................+**==++++++                                 
                                 ++++++:................-***-++++++                                 
                                 ++++++:.................***-++++++                                 
                                 ++++++..................+**=++++++                                 
                                 ++++++..................-***++++++                                 
                                 ++++++...................***++++++                                 
                                  +++++:..................=*++++++                                  
                                  +++++-...................:-+++++                                  
                                  +++++=....................=+++++                                  
                                  ++++++:..................:+++++* 
                                   +++++-..................-+++++                                   
                                    +++++:................:+++++                                    
                                    +++++=................++++++                                    
                                     +++++=..............=+++++                                     
                                      +++++=:..........:=+++++                                      
                                       ++++++-........-++++++                                       
                                        ++++++++=--=++++++++                                        
                                          ++++++++++++++++                                          
                                            ++++++++++++                                            
                                               *++++* "#);
}
