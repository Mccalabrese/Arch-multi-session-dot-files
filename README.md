# My Arch Dotfiles

This is my obsessive setup for a minimal, multi-compositor Arch Linux environment. I run niri mostly but sometimes hyprland (docked on external monitor), and a custom sway session (iGPU-only, hyper optimized for battery). I also have gnome. Gnome is there when a full desktop is needed. Some things, such as annotating on students screens in zoom, just will not work on anything but a full desktop. My other configs do assume you have gnome and all its dependencies.

The whole point is efficiency and performance. This setup idles at 4.8W on my ThinkPad X1 Extreme (i7-10850H, 64GB RAM, GTX 1650 Ti).

This is a personal repo, not a beginner's guide. It assumes you know what you're doing.

The Philosophy: Why Rust?

* You'll see all my helper scripts are written in Rust. I'm not a "Rust-acean," but I am a pragmatist.

Why not Python? Because Python is the absolute worst. It's a slow, dependency-hell nightmare. I don't trust its supply chain, and honestly, I just don't vibe with it. I'll take C-style syntax any day.

Why not shell scripts? My old scripts were a Rube Goldberg machine of pgrep, jq, sed, awk, and cat all piped together. They were fragile, slow, and "worked like crap."

But why not Zig? I love Zig. It's the future. But the sad reality is that its API changes so fast, I can't even learn the language before a pacman -Syu breaks everything I've written.

So, Rust. It gives me the C-like syntax and performance I want, with a stable ecosystem (cargo) that actually works. It's the best tool for the job right now. If the senior kernel devs hate rust, I stand with them, I make no comment or express no opinion on what should or should not be in the kernel.

Why ghostty? It is the best. *

## 1. Core Dependencies (pacman)

This won't be a one-click install. You need to build the base.
Bash

### The compositors and base DE (for services)
sudo pacman -S sway hyprland niri gnome

### Core UI tools
sudo pacman -S waybar hyprlock swayidle wofi rofi

### Key system services & utilities
sudo pacman -S polkit-gnome nm-applet udiskie geoclue
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
    Ini, TOML

    [wifi-scan]
    url=https://www.googleapis.com/geolocation/v1/geocluate?key=YOUR_GOOGLE_KEY_HERE

    Restart the service: sudo systemctl restart geoclue.service.

### systemd-resolved (for DNS)

My cloudflare-toggle script manually writes to /etc/resolv.conf. This only works if systemd-resolved isn't fighting you.

    sudo systemctl disable --now systemd-resolved

    sudo rm /etc/resolv.conf (or back it up)

    sudo touch /etc/resolv.conf

    echo "nameserver 1.1.1.1" | sudo tee /etc/resolv.conf (to set a sane default)

### zsh and $PATH

My scripts (and niri/sway/hyprland) will fail if they can't find the Rust binaries. You must add cargo to your path in a way that non-interactive sessions can read.

Create this file: ~/.config/environment.d/99-custom-path.conf

Put this one line in it:

    PATH=$HOME/.cargo/bin:$HOME/.pub-cache/bin:$PATH

Log out and log back in. ~/.zshrc is the wrong place for this.

## 3. Building the Rust Apps

This repo contains the source code for all my custom scripts in the ~/sysScripts directory. You need to install them.
Bash

### First, init rustup
    rustup-init

### Add cargo to your current shell (you'll log out later)
    source "$HOME/.cargo/env"

### Now, install everything
    cd ~/sysScripts/waybar-switcher
    cargo install --path .

    cd ~/sysScripts/waybar-weather
    cargo install --path .

    cd ~/sysScripts/sway-workspace
    cargo install --path .

    cd ~/sysScripts/update-check
    cargo install --path .

    cd ~/sysScripts/cloudflare-toggle
    cargo install --path . # This installs both cf-status and cf-toggle

    cd ~/sysScripts/wallpaper-manager
    cargo install --path . # This installs wp-daemon, wp-select, and wp-apply

## 4. Setting Up Your Configs & Secrets

I don't commit my API keys. You shouldn't either.

Symlink the "safe" configs:
Bash

    ln -s ~/dotfiles/.config/hypr ~/.config/hypr
    ln -s ~/dotfiles/.config/sway ~/.config/sway
    ln -s ~/dotfiles/.config/niri ~/.config/niri
    # ... etc for nvim, tmux, etc.

Copy the "secret" templates: My Waybar configs need an OWM key, so they are templates.
Bash

    cp ~/dotfiles/.config/waybar/hyprConfig.jsonc.template ~/.config/waybar/hyprConfig.jsonc
    cp ~/dotfiles/.config/waybar/swayConfig.jsonc.template ~/.config/waybar/swayConfig.jsonc
    cp ~/dotfiles/.config/wayGbar/niriConfig.jsonc.template ~/.config/waybar/niriConfig.jsonc

Inject your key: Run this sed command to find the placeholder and replace it with your real key.
Bash

    echo "Please paste your OpenWeatherMap (OWM) API Key:"
    read OWM_KEY
    sed -i "s/__OWM_KEY_PLACEHOLDER__/$OWM_KEY/g" ~/.config/waybar/*.jsonc

Add a .gitignore: Make one in your ~/dotfiles root. You must ignore your real config files.
Code snippet

    # Ignore compiled Rust code
    **/target/

    # Ignore secret-holding configs
    .config/waybar/hyprConfig.jsonc
    .config/waybar/swayConfig.jsonc
    .config/waybar/niriConfig.jsonc

    # Ignore cache files
    .cache/

## 5. Final Startup & Waybar Configs

My setup relies on these Rust binaries. Your startup scripts and Waybar configs must point to them using $HOME.

### Startup Scripts

hyprland.conf:
Ini, TOML

    exec-once = $HOME/.cargo/bin/waybar-switcher
    exec-once = swww-daemon --namespace hypr
    exec-once = $HOME/.cargo/bin/wp-daemon
    bind = SUPER, W, exec, $HOME/.cargo/bin/wp-select

niri.conf:
Code snippet

    spawn-at-startup "$HOME/.cargo/bin/waybar-switcher"
    spawn-at-startup "swww-daemon" "--namespace" "niri"
    spawn-at-startup "$HOME/.cargo/bin/wp-daemon"
    binds {
        Mod+W { spawn "$HOME/.cargo/bin/wp-select"; }
    }

sway/config:
Ini, TOML

    exec $HOME/.cargo/bin/waybar-switcher
    exec $HOME/.cargo/bin/wp-daemon
    exec swaybg -i "$(cat $HOME/.cache/swaybg_last_wallpaper)" -m fill &
    bindsym Mod4+w exec $HOME/.cargo/bin/wp-select

### Waybar Configs (Example custom/weather)

In all your .jsonc files:
JSON

"custom/weather": {
    "exec": "OWM_API_KEY=YOUR_KEY_HERE $HOME/.cargo/bin/waybar-weather",
    "return-type": "json",
    "interval": 900
},

*(My waybar-switcher copies the correct config, which already has this. The sed command fixes the key.)*
