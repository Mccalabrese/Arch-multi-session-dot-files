# My Arch Dotfiles

This is my obsessive setup for a minimal, multi-compositor Arch Linux environment. I run niri mostly but sometimes hyprland (docked on external monitor), and a custom sway session (iGPU-only, hyper optimized for battery). I also have gnome. Gnome is there when a full desktop is needed. Some things, such as annotating on students screens in zoom, just will not work on anything but a full desktop. My other configs do assume you have gnome and all its dependencies.

**Credit to JaKooLit for the original inspiration. I have since heavily modified and optimized it for my needs. Many theme options and examples come from him**

The whole point is efficiency and performance. This setup idles at 4.8W on my ThinkPad X1 Extreme (i7-10850H, 64GB RAM, GTX 1650 Ti).

This is a personal repo, not a beginner's guide. It assumes you know what you're doing.

The Philosophy: Why Rust?

*You'll see all my helper scripts are written in Rust. I'm not a "Rust-acean," but I am a pragmatist.*

*Why not Python? Because Python is the absolute worst. It's a slow, dependency-hell nightmare. I don't trust its supply chain, and honestly, I just don't vibe with it. I'll take C-style syntax any day.*

*Why not shell scripts? My old scripts were a Rube Goldberg machine of pgrep, jq, sed, awk, and cat all piped together. They were fragile, slow, and "worked like crap."*

*But why not Zig? I love Zig. It's the future. But the sad reality is that its API changes so fast, I can't even learn the language before a pacman -Syu breaks everything I've written.*

*So, Rust. It gives me the C-like syntax and performance I want, with a stable ecosystem (cargo) that actually works. It's the best tool for the job right now. If the senior kernel devs hate rust, I stand with them, I make no comment or express no opinion on what should or should not be in the kernel.*

*Why ghostty? It is the best.*

## 1. Core Dependencies (pacman)

This won't be a one-click install. You need to build the base.
Bash

### The compositors and base DE (for services)

sudo pacman -S sway hyprland niri gnome

### Core UI tools

sudo pacman -S waybar hyprlock swayidle wofi rofi

### Key system services & utilities

sudo pacman -S polkit-gnome nm-applet udiskie geoclue greetd greetd-tuigreet
sudo pacman -S pulseaudio # or pipewire-pulse, your call

### Our custom scripts will need these

sudo pacman -S cloudflared pacman-contrib # (for 'checkupdates')

### User apps I use

sudo pacman -S ghostty thunar

### Build toolchain for our Rust apps

sudo pacman -S rustup openssl pkg-config libc

## 2. Manual System Config (The "Gotchas")

You must do these steps as sudo. My scripts depend on this.

### geoclue (for Weather)

The default Mozilla backend is dead. We use Google.

Get a Google Geolocation API key.

Edit /etc/geoclue/geoclue.conf.

Find the [wifi-scan] section and add your key:
Ini, TOML:

    [wifi-scan]
    url=https://www.googleapis.com/geolocation/v1/geocluate?key=YOUR_GOOGLE_KEY_HERE

Restart the service: sudo systemctl restart geoclue.service.

### systemd-resolved (for DNS)

My cloudflare-toggle script manually writes to /etc/resolv.conf. This only works if systemd-resolved isn't fighting you.

    sudo systemctl disable --now systemd-resolved

    sudo rm /etc/resolv.conf 

    sudo touch /etc/resolv.conf

    echo "nameserver 1.1.1.1" | sudo tee /etc/resolv.conf 

### zsh and $PATH

My scripts (and niri/sway/hyprland) will fail if they can't find the Rust binaries. You must add cargo to your path in a way that non-interactive sessions can read.

Create this file: ~/.config/environment.d/99-custom-path.conf

Put this one line in it:

    PATH=$HOME/.cargo/bin:$HOME/.pub-cache/bin:$PATH

Log out and log back in. ~/.zshrc is the wrong place for this.

## 3. The Central Config (Your API Keys)

All of our custom Rust scripts are controlled by one file. This is where you put your API keys and personal paths.

Copy the template:
Bash

    mkdir -p ~/.config/rust-dotfiles
    cp ~/.config/rust-dotfiles/config.toml.template ~/.config/rust-dotfiles/config.toml

Edit the file:
Bash

    nano ~/.config/rust-dotfiles/config.toml

Fill in your secrets. This one file controls your weather API key, wallpaper directory, terminal choice, and more. The .gitignore file is already set up to protect this file from being committed.

## 4. Building the Rust Apps

This repo contains the source code for all my custom scripts in the ~/sysScripts directory. You need to install them.
Bash

### First, init rustup

    rustup-init

### Add cargo to your current shell (you'll log out later)

    source "$HOME/.cargo/env"

### Now, install everything

    cd sysScripts/waybar-switcher && cargo install --path .
    cd sysScripts/waybar-weather && cargo install --path .
    cd sysScripts/sway-workspace && cargo install --path .
    cd sysScripts/update-check && cargo install --path .
    cd sysScripts/cloudflare-toggle && cargo install --path .
    cd sysScripts/wallpaper-manager && cargo install --path .
    cd sysScripts/kb-launcher && cargo install --path .
    cd sysScripts/updater && cargo install --path .

## 5. Setting Up Your Configs & Secrets

I don't commit my API keys. You shouldn't either.

Symlink the "safe" configs:
Bash

    ln -s ~/Arch-multi-session-dot-files/.config/hypr ~/.config/hypr
    ln -s ~/Arch-multi-session-dot-files/.config/sway ~/.config/sway
    ln -s ~/Arch-multi-session-dot-files/.config/niri ~/.config/niri
    ln -s ~/Arch-multi-session-dot-files/.config/rofi ~/.config/rofi
    ln -s ~/Arch-multi-session-dot-files/.config/swaync ~/.config/swaync
    # ... etc for nvim, tmux, etc.
    

Our Rust scripts handle all secrets. You just need to copy the Waybar config templates.

    cp ~/Arch-multi-session-dot-files/.config/waybar/hyprConfig.jsonc.template ~/.config/waybar/hyprConfig.jsonc
    cp ~/Arch-multi-session-dot-files/.config/waybar/swayConfig.jsonc.template ~/.config/waybar/swayConfig.jsonc
    cp ~/Arch-multi-session-dot-files/.config/waybar/niriConfig.jsonc.template ~/.config/waybar/niriConfig.jsonc

## 6. Final Startup

### Neovim Setup

This config uses LazyVim. The configuration is minimal. To use it, you must follow their installation guide first. My personal tweaks can be found in ~/.config/nvim/lua/plugins/.
