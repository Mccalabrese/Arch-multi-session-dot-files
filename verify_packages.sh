#!/bin/bash

# 1. Official Packages (From your Rust 'PACMAN_PACKAGES' const)
OFFICIAL_PACKAGES=(
  "base-devel" "git" "rustup" "openssl" "pkgconf" "glibc" "wget" "curl" "jq"
  "sway" "hyprland" "niri" "gnome" "hyprlock" "swayidle" "hypridle"
  "waybar" "wofi" "rofi" "swaync" "swww" "swaybg"
  "grim" "slurp"
  "tlp" "polkit-gnome" "network-manager-applet" "udiskie" "geoclue" "upower"
  "greetd" "greetd-tuigreet" "pulseaudio" "pipewire" "pipewire-pulse"
  "cloudflared" "pacman-contrib" "fakeroot" "cliphist"
  "wl-clipboard" "ghostty" "thunar" "starship" "neovim" "tmux"
)

# 2. AUR Packages (From your Rust 'AUR_PACKAGES' const)
AUR_PACKAGES=(
  "wlogout"
)

echo "🔍 Verifying OFFICIAL packages..."
echo "---------------------------------------------------"

failed=0
for pkg in "${OFFICIAL_PACKAGES[@]}"; do
  # Check if package exists in sync db (-Si) or is a group (-Sg) or provided by something (-Ssq)
  if pacman -Si "$pkg" &>/dev/null; then
    echo -e "✅ Found: $pkg"
  elif pacman -Sg "$pkg" &>/dev/null; then
    echo -e "✅ Found Group: $pkg"
  elif pacman -Ssq "^$pkg$" &>/dev/null; then
    echo -e "⚠️  Virtual/Provider found: $pkg"
  else
    echo -e "❌ MISSING in Official Repos: $pkg"
    ((failed++))
  fi
done

echo ""
echo "🔍 Verifying AUR packages (via API)..."
echo "---------------------------------------------------"

# Simple function to check AUR API without needing yay installed
check_aur() {
  local pkg=$1
  # Fetch info from AUR RPC. If resultcount is not 0, it exists.
  local response=$(curl -s "https://aur.archlinux.org/rpc/?v=5&type=info&arg[]=$pkg")
  if echo "$response" | grep -q '"resultcount":0'; then
    return 1
  else
    return 0
  fi
}

for pkg in "${AUR_PACKAGES[@]}"; do
  if check_aur "$pkg"; then
    echo -e "✅ Found in AUR: $pkg"
  else
    echo -e "❌ MISSING in AUR: $pkg"
    ((failed++))
  fi
done

echo "---------------------------------------------------"
if [ $failed -eq 0 ]; then
  echo "🎉 All packages are valid and locatable!"
else
  echo "🚨 $failed packages not found. Please correct the lists in main.rs."
fi
