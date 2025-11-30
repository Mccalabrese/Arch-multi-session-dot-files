
# Installation Guide (Full Manual Equivalent)

This guide reproduces every step performed by `bootstrap.sh` and the Rust installer (`sysScripts/install-wizard/src/main.rs`) so you can execute a manual installation that is functionally identical.

Important: Run commands in order. Use a non-root user unless explicitly told to use `sudo`.

## 0. Pre-Flight and Repository Setup

- Ensure you are NOT root and have connectivity.

```bash
# Fail if run as root
if [ "$EUID" -eq 0 ]; then echo "Do not run as root"; exit 1; fi

# Quick connectivity check
ping -c 1 archlinux.org
```

- Sync keyrings and mirrorlist, then update:

```bash
sudo pacman -Syu --noconfirm archlinux-keyring pacman-mirrorlist
```

- Install base toolchain required by the installer:

```bash
sudo pacman -S --needed --noconfirm base-devel rustup git pkgconf wget curl ca-certificates
```

- Clone the repository if you aren’t already in it:

```bash
git clone https://github.com/mccalabrese/rust-wayland-power.git
cd rust-wayland-power
```

- Initialize Rust toolchain if missing and ensure cargo is in PATH for this session:

```bash
rustup init -y --default-toolchain stable
export PATH="$HOME/.cargo/bin:$PATH"
```

## 1. Sudo Cache Warm-Up

The installer performs a sudo credential refresh to avoid timeouts during long operations:

```bash
sudo -v
```

## 2. Core System Packages (pacman)

Install the union of all packages used by the installer. This includes build tools, hardware support, compositors, Wayland infrastructure, audio, file management, UI/security, fonts, and common apps.

```bash
sudo pacman -S --needed --noconfirm \
  base-devel git go rustup openssl pkgconf glibc wget curl jq \
  man-db man-pages unzip tree linux-headers pciutils pacman-contrib \
  bolt upower tlp bluez bluez-utils blueman brightnessctl udiskie fwupd \
  intel-media-driver libva-utils vulkan-intel \
  sway hyprland niri gnome hyprlock swayidle hypridle xdg-user-dirs-gtk greetd greetd-tuigreet \
  xwayland-satellite qt5-wayland qt6-wayland polkit-gnome geoclue \
  xdg-desktop-portal-gnome xdg-desktop-portal-wlr xdg-desktop-portal-gtk \
  wl-clipboard cliphist \
  pulseaudio pipewire pipewire-pulse pipewire-alsa pipewire-jack wireplumber pavucontrol sof-firmware \
  thunar thunar-volman tumbler gvfs gvfs-mtp gvfs-smb gvfs-gphoto2 file-roller gnome-disk-utility \
  ufw timeshift seahorse gnome-keyring waybar wofi rofi swaync swww swaybg grim slurp mako \
  papirus-icon-theme gnome-themes-extra adwaita-icon-theme \
  ttf-jetbrains-mono-nerd ttf-fira-code ttf-jetbrains-mono noto-fonts noto-fonts-emoji otf-font-awesome \
  zsh starship ghostty tmux fzf ripgrep bat btop fastfetch neovim \
  networkmanager network-manager-applet cloudflared firefox discord tigervnc mpv gparted simple-scan gnome-calculator \
  cups system-config-printer cups-pdf
```

## 3. GPU Detection and Drivers

Identify your GPU vendor with `lspci` and install vendor-specific packages.

```bash
lspci -nn | grep -i 'vga\|3d\|display'
```

- NVIDIA:

```bash
sudo pacman -S --needed --noconfirm nvidia-dkms nvidia-prime nvidia-settings libva-nvidia-driver
```

- AMD:

```bash
sudo pacman -S --needed --noconfirm vulkan-radeon libva-mesa-driver mesa-vdpau xf86-video-amdgpu
```

- Intel: Already covered by `intel-media-driver`, `libva-utils`, `vulkan-intel` above.

Refer to Power Management section for NVIDIA runtime power tuning.

## 4. AUR Packages via `yay`

Bootstrap `yay` from AUR and install community packages.

```bash
if ! command -v yay >/dev/null; then 
  mkdir -p "$HOME/.cache/yay-build" && cd "$HOME/.cache/yay-build"
  git clone https://aur.archlinux.org/yay.git
  cd yay
  makepkg -si --noconfirm
fi

yay -S --needed --noconfirm \
  wlogout zoom slack-desktop ledger-live-bin visual-studio-code-bin pinta ttf-victor-mono ytmdesktop-bin
```

## 5. System Configuration

### 5.1 Disable systemd-resolved and manage resolv.conf directly

```bash
sudo systemctl disable --now systemd-resolved
sudo rm -f /etc/resolv.conf
echo "nameserver 1.1.1.1" | sudo tee /etc/resolv.conf
```

### 5.2 Configure greetd with tuigreet

```bash
sudo systemctl enable --now greetd.service
sudo tee /etc/greetd/config.toml >/dev/null <<'EOF'
[terminal]
vt = 1
[default_session]
command = "tuigreet --time --remember --sessions /usr/share/wayland-sessions:/usr/share/xsessions"
user = "greeter"
EOF
```

### 5.3 Enforce clean session switching

```bash
sudo sed -i 's/^#\?KillUserProcesses=.*/KillUserProcesses=yes/' /etc/systemd/logind.conf
sudo systemctl restart systemd-logind.service
```

### 5.4 Optimize pacman: trim session files via NoExtract

Add unwanted session `.desktop` files to `NoExtract` to keep the greeter list clean.

```bash
sudo sed -i 's/^#\?NoExtract.*/NoExtract = usr\/share\/wayland-sessions\/niri.desktop usr\/share\/wayland-sessions\/hyprland.desktop usr\/share\/wayland-sessions\/sway.desktop usr\/share\/wayland-sessions\/gnome.desktop usr\/share\/wayland-sessions\/gnome-classic.desktop usr\/share\/wayland-sessions\/gnome-classic-wayland.desktop usr\/share\/wayland-sessions\/hyprland-uwsm.desktop usr\/share\/wayland-sessions\/gnome-wayland.desktop/' /etc/pacman.conf
```

Run a full system update afterward to let pacman reconcile these files.

## 6. Rust Tooling and Custom Apps

Set Rust to stable and build the custom scripts.

```bash
rustup default stable
```

Build and install each app from `sysScripts` to `~/.cargo/bin`:

```bash
cd sysScripts
for dir in */; do
  (cd "$dir" && cargo install --path .)
done
```

Apps provided: waybar-switcher, waybar-weather, sway-workspace, update-check, cloudflare-toggle, wallpaper-manager, kb-launcher, updater, power-menu, rfkill-manager, clip-manager, emoji-picker, radio-menu, waybar-finance.

## 7. Secrets and Geoclue Configuration

Create central config and populate API keys and preferences as prompted by the installer wizard.

```bash
mkdir -p ~/.config/rust-dotfiles
cp ~/.config/rust-dotfiles/config.toml.template ~/.config/rust-dotfiles/config.toml
nano ~/.config/rust-dotfiles/config.toml
```

For geoclue (Google Geolocation):

```bash
sudo sed -i 's#^\[wifi-scan\].*#[wifi-scan]\nurl=https://www.googleapis.com/geolocation/v1/geolocate?key=YOUR_GOOGLE_KEY_HERE#' /etc/geoclue/geoclue.conf
sudo systemctl restart geoclue.service
```

## 8. Waybar Config Templates

If you haven’t already, copy Waybar config templates (the installer does this conditionally):

```bash
mkdir -p ~/.config/waybar
cp .config/waybar/hyprConfig.jsonc.template ~/.config/waybar/hyprConfig.jsonc
cp .config/waybar/swayConfig.jsonc.template ~/.config/waybar/swayConfig.jsonc
cp .config/waybar/niriConfig.jsonc.template ~/.config/waybar/niriConfig.jsonc
```

## 9. Dotfiles and Resources Linking

Link configs and enable TLP as done by the installer:

```bash
ln -sf "$PWD/.tmux.conf" ~/.tmux.conf
ln -sf "$PWD/.profile" ~/.profile
sudo ln -sf "$PWD/tlp.conf" /etc/tlp.conf
sudo systemctl enable tlp.service
ln -sf "$PWD/.config/hypr" ~/.config/hypr
ln -sf "$PWD/.config/sway" ~/.config/sway
ln -sf "$PWD/.config/niri" ~/.config/niri
ln -sf "$PWD/.config/rofi" ~/.config/rofi
ln -sf "$PWD/.config/swaync" ~/.config/swaync
ln -sf "$PWD/.config/environment.d" ~/.config/environment.d
ln -sf "$PWD/.config/ghostty" ~/.config/ghostty
ln -sf "$PWD/.config/gtk-3.0" ~/.config/gtk-3.0
ln -sf "$PWD/.config/gtk-4.0" ~/.config/gtk-4.0
ln -sf "$PWD/.config/fastfetch" ~/.config/fastfetch
ln -sf "$PWD/.config/wlogout" ~/.config/wlogout
ln -sf "$PWD/.config/waybar" ~/.config/waybar
```

Copy wallpapers if applicable (the installer handles this):

```bash
mkdir -p ~/Pictures/Wallpapers
cp -r wallpapers/* ~/Pictures/Wallpapers/
```

## 10. PATH for non-interactive sessions

Ensure cargo in PATH for display managers and services:

```bash
mkdir -p ~/.config/environment.d
echo 'PATH=$HOME/.cargo/bin:$HOME/.pub-cache/bin:$PATH' > ~/.config/environment.d/99-custom-path.conf
```

## 11. NVIDIA Power Management (If NVIDIA Present)

Apply kernel parameters, modprobe rules, blacklist CUDA’s `nvidia_uvm` at boot, udev runtime power rule, and rebuild initramfs/GRUB. See `docs/POWER_MANAGEMENT.md` for exact commands.

## 12. Final Notes

- `ufw`, `timeshift`, and `cups` are installed; enable/adjust as desired.
- The installer runs with "fail fast" semantics; replicate that caution manually.
- Reboot recommended after NVIDIA changes and greetd activation.

---

## Installer Actions Log (Reference)

This is a concise sequence mirroring what the automated installer performs:

- Sudo cache refresh: `sudo -v`
- Install common packages (full list above) via pacman
- Detect GPU with `lspci` and install vendor-specific drivers
- Bootstrap and use `yay` for AUR packages: `wlogout`, `zoom`, `slack-desktop`, `ledger-live-bin`, `visual-studio-code-bin`, `pinta`, `ttf-victor-mono`, `ytmdesktop-bin`
- System configuration:
  - Disable `systemd-resolved`; manage `/etc/resolv.conf`
  - Configure `greetd` with `tuigreet`
  - Set `KillUserProcesses=yes` in `logind.conf`
  - Optimize `pacman.conf` `NoExtract` for session files
- Rust setup: `rustup default stable`; build and install custom Rust apps from `sysScripts`
- Secrets and geoclue configuration (Google Geolocation API key)
- Waybar templates and config deployment; dotfiles symlinking; wallpapers copy
- NVIDIA-specific power management (if applicable) including `mkinitcpio -P` and GRUB regen
