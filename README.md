<p align="center">
  <img src="logo.svg" width="200" height="200" alt="Rust Wayland Power Logo">
</p>

<h1 align="center">Arch Linux Dotfiles: Niri, Hyprland & Sway</h1>

<p align="left">
  <a href="./LICENSE"><img src="https://img.shields.io/badge/License-MIT-green.svg" alt="MIT License"></a>
  <a href="https://archlinux.org/"><img src="https://img.shields.io/badge/Arch%20Linux-1793D1?logo=arch-linux&logoColor=white" alt="Arch Linux"></a>
  <a href="https://wayland.freedesktop.org/"><img src="https://img.shields.io/badge/Wayland-34BE5B?logo=wayland&logoColor=white" alt="Wayland"></a>
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/Rust-000000?logo=rust&logoColor=white" alt="Rust"></a>
  <a href="https://www.gnome.org/"><img src="https://img.shields.io/badge/GNOME-4A86CF?logo=gnome&logoColor=white" alt="GNOME"></a>
  <a href="https://neovim.io/"><img src="https://img.shields.io/badge/Neovim-57A143?logo=neovim&logoColor=white" alt="Neovim"></a>
  <a href="https://github.com/tmux/tmux"><img src="https://img.shields.io/badge/tmux-1BB91F?logo=tmux&logoColor=white" alt="tmux"></a>
  <a href="#my-arch-dotfiles"><img src="https://img.shields.io/badge/Compositors-Niri%20%7C%20Hyprland%20%7C%20Sway-6A5ACD" alt="Compositors: Niri | Hyprland | Sway"></a>
  <a href="https://www.shellcheck.net/"><img src="https://img.shields.io/badge/shellcheck-recommended-blue" alt="ShellCheck recommended"></a>
</p>

[![Donate](https://img.shields.io/badge/Donate-Crypto-yellow.svg?style=for-the-badge&logo=bitcoin&logoColor=white)](#support-the-project)

This is my obsessive setup for a minimal, multi-compositor Arch Linux environment. I run **Niri** primarily, **Hyprland** when docked, and a custom **Sway** session (iGPU-only) hyper-optimized for battery life. Gnome is also there when a full desktop is needed (like teaching and annotating on students screens in zoom).

The whole point is efficiency and performance. This setup idles at **4.8W** on my ThinkPad X1 Extreme (i7-10850H, 64GB RAM, GTX 1650 Ti, 4k display). I worked diligently on NVIDIA Optimus / Hybrid Graphics Fixes to achieve this.

---

## 🚀 Installation

### ⚠️ Beta Warning
>
> **The Rust Installer is currently in BETA.**
> While I use this daily, it performs major system changes (package installation, config linking, systemd services).
>
> **If you encounter errors:** Please open an issue with your **Hardware Details** (GPU, CPU, Laptop Model) and the full error log.

### Automatic Install (Recommended)

This repo includes a `bootstrap.sh` script that handles dependency checking, git cloning, and launches the Rust-based installation wizard.

**The installer will ask for 3 API keys, it may be easier to get them before running the script**
**get your API keys at:**
**<https://home.openweathermap.org/users/sign_up>**
**finnhub.io/register**
**console.cloud.google.com/apis/library/geocoding-backend.googleapis.com**

**The weather module relies on Geoclue, which needs the Google Maps Geolocation API to find your laptop (since most laptops don't have GPS).**

**You must enable Billing for this API to work. Don't panic: Google gives $200/month free credit, which is millions of requests. You will never pay a cent for personal use, but you must have a card on file.**

    Go to Google Cloud Console: console.cloud.google.com

    Create a Project: Name it "Arch-Weather" or similar.

    Enable the API:

        Search for "Geolocation API" (Not Geocoding, specifically Geolocation).

        Click Enable.

    Create Credentials:

        Go to APIs & Services > Credentials.

        Click Create Credentials > API Key.

        Recommended: Click "Restrict Key" and lock it to "Geolocation API" only for security.

    Enable Billing:

        Go to Billing in the sidebar.

        Link a credit card. (Again, you won't be charged unless you make >100,000 requests/month).

    Copy the Key: Paste this into the Installer when prompted.

**If you do not have drivers for your graphics card yet (fresh arch install) the updater will install them and ask you to reboot, instructions are given in the installer window but allow the reboot then rerun the installer with the same curl command after your computer turns back on and it will continue where it left off.**

```bash
cd ~ && curl -O https://raw.githubusercontent.com/Mccalabrese/rust-wayland-power/main/bootstrap.sh && chmod +x bootstrap.sh && ./bootstrap.sh
```

### Manual Installation

If you prefer to understand every change being made to your system, or if the installer fails, please consult the manual guide 👉 [docs/MANUAL_INSTALL.md](docs/MANUAL_INSTALL.md).

## ⚡ Power Management (NVIDIA Optimus)

Achieving sub-5W idle on a workstation requires specific BIOS, Kernel, and Udev Configurations 👉 [docs/POWER_MANAGEMENT.md](docs/POWER_MANAGEMENT.md).

**Credit to JaKooLit for the original inspiration. I have since heavily modified and optimized it for my needs. Many theme options and examples are available on <https://github.com/JaKooLit/Hyprland-Dots>**

## My Custom Rust Binaries

All the helper scripts in this repo have been rewritten in Rust for maximum performance and stability. **These tools can be used independently of the rest of my dotfiles.** Here is what each one does:

- **`waybar-switcher`**: A small utility that runs at login to detect which session you're in (Niri, Hyprland, or Sway) and automatically loads the correct Waybar config.
- **`waybar-weather`**: The weather module in Waybar. It's a custom-built app that securely gets your API key from the central config, finds your location, and fetches the weather.

<p align="center">
  <img src="screenshots/waybar-weather.png" width="28%" alt="Waybar weather module"/>
  </p>

- **`sway-workspace`**: A simple helper that reliably gets the current workspace name for the Waybar module in Sway.
- **`update-check`**: The update icon in Waybar. It safely checks for new `pacman` and `yay` updates and shows the count. It's network-aware and displays a "stale" count if you're offline.
- **`cloudflare-toggle`**:
  - **`cf-status`**: The "CF" icon in Waybar that shows if you are using Cloudflare DNS over HTTPS.
  - **`cf-toggle`**: The `on-click` script that securely toggles Cloudflare DNS over HTTPS on or off using `pkexec`.

<p align="center">
  <img src="screenshots/cf-toggle.png" width="60%" alt="Cloudflare toggle module"/>
  </p>

- **`wallpaper-manager`**: A 3-part system that manages all your wallpapers.
  - **`wp-daemon`**: A silent, background daemon that watches your wallpaper folder for changes and auto-generates thumbnails.
  - **`wp-select`**: The Rofi-based pop-up menu (`Mod+W`) that lets you see your wallpaper thumbnails and choose a new one.
  - **`wp-apply`**: The back-end tool that actually sets the wallpaper, using the correct tool for your session (`swaybg` or `swww`).

<p align="center">
  <img src="screenshots/wallpaper-manager.png" width="70%" alt="Wallpaper rofi app"/>
  </p>

- **`kb-launcher`**: The keybind cheat sheet (`Mod+Shift+P`). It's a pop-up menu that reads your `.txt` files to show you the keybinds for Niri, Sway, Hyprland, or Neovim.

<p align="center">
  <img src="screenshots/keyhints.png" width="32%" alt="keyhint sheet"/>
  <img src="screenshots/keyhints-selector.png" width="60%" alt="Keyhint session selector (rofi)" />
  </p>

- **`updater`**: The `on-click` script for the `update-check` module. It simply launches your terminal (`ghostty`) to run the actual system update.
    > ⚠️**WARNING!!!! My updater runs fwupdmgr as sudo with a -y flag.**
    > **Do not update with my updater** unless you are prepared to reboot or get sent into a bios update.
    > If you are not comfortable with this, find the "let bash_script = format!(..)" block of updater/src/main.rs (line 113) and change it to:
>
```rust
let bash_script = format!("{} \n exit_code=$? \n echo -e '\\n\\n🏁 Update process finished. This window will close in 5 seconds.' \n sleep 5 \n exit $exit_code", update_cmd_str);
```
>
- > Then rebuild:

```bash
cargo install --path .
```

- **`power-menu`**: The graphical power menu (`Ctrl+Alt+P`). It's a compositor-aware launcher for `wlogout` that automatically calculates the correct screen position and scaling for Niri, Hyprland, and Sway.

<p align="center">
  <img src="screenshots/power-menu.png" width="60%" alt="Power Menu formatting"/>
  </p>

- **`rfkill-manager`**:
  - `--status`: The airplane icon in your SwayNC, showing if "Airplane Mode" is on or off.
  - `--toggle`: The `on-click` action (in your `swaync` panel or on a keybind) that toggles all wireless (Wi-Fi & Bluetooth) on or off.
- **`clip-manager`**: The clipboard history manager (`Mod+Alt+V`). It uses `cliphist` as a backend and pipes your selection to Rofi, allowing you to copy, delete, or wipe your clipboard history.

<p align="center">
  <img src="screenshots/clipboard.png" width="58%" alt="clipboard manager"/>
  </p>

- **`emoji-picker`**: The emoji selector (`Mod+Alt+E`). It uses a built-in Rust emoji database to give you a fast, searchable Rofi menu for copying any emoji.

<p align="center">
  <img src="screenshots/emoji.png" width="42%" alt="Emoji rofi app"/>
  </p>

- **`radio-menu`**: A rofi based, extremely lightweight and low power consumption internet radio streaming tool that caches favorite stations for quick selection.

<p align="center">
  <img src="screenshots/rofi-radio.png" width="85%" alt="rofi radio app"/>
  </p>

- **`waybar-finance`**: A lightweight TUI market watch list application with charts, and security details. Designed to be run in a terminal multiplexer like tmux or ghostty. This will request that you create and enter a Finnhub.io API key on first run. The app will display quoting of your watchlist items as a tooltip on hover over its icon in Waybar.

<p align="center">
  <img src="screenshots/waybar-finance-tooltip.png" width="32%" alt="waybar finance tooltip"/>
  <img src="screenshots/waybar-finance-tui.png" width="65%" alt="finance TUI" />
  </p>

## Screenshots

<p align="center">
  <img src="screenshots/sway.png" width="48%" alt="Sway iGPU-only Idle"/>
  <img src="screenshots/gnome.png" width="48%" alt="Gnome Desktop"/>
  </p>
<p align="center">
  <img src="screenshots/niri.png" width="90%" alt="Niri Overview"/>
  <img src="screenshots/hyprland.png" width="90%" alt="Hyprland Desktop"/>
</p>

### The Philosophy: Why Rust?

You'll see all my helper scripts are written in Rust. I'm not a "Rust-acean," but I am a pragmatist.

Why not Python? I chose Rust to avoid dependency hell and runtime errors. I wanted compiled, single-binary tools that sip battery and don't break when I update system Python packages.

Why not shell scripts? My old scripts were a disaster zone of pgrep, jq, sed, awk, and cat all piped together. They were fragile, slow, and hard to maintain.

**I like Zig, I like C, I make zero claim at all about whether Rust should or should not be in the kernel.**

## Support the Project

If these dotfiles saved you time or improved your battery life, consider buying me a coffee (or a textbook). I'm a computer engineering student, and every bit helps!

- **BTC:** `bc1qvanmtuwn3jd5wurlj3wc9j7ddcwcwwzhjj6ndx`
- **ETH:** `0x1dC04e98A9762fdb86F09437813Ce73D9112dF96`
