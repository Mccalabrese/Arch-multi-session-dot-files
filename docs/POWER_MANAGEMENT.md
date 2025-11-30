
## dGPU Power Management (NVIDIA Hybrid Laptops)

The installer applies NVIDIA-specific runtime power management when an NVIDIA GPU is detected. This section documents the full manual equivalent of those actions.

**Why:** Display managers like `gdm`/`sddm` can probe the dGPU at boot, causing a VRAM lock that prevents `D3cold` (RTD3) and wastes 3–5W at idle. We avoid probing (use `greetd`) and set kernel module parameters and udev rules for proper runtime power management.

### 1. Use `greetd` (avoid dGPU probing)

```bash
sudo systemctl disable --now gdm || true
sudo systemctl disable --now sddm || true
sudo systemctl enable --now greetd.service
sudo tee /etc/greetd/config.toml >/dev/null <<'EOF'
[terminal]
vt = 1
[default_session]
command = "tuigreet --time --remember --sessions /usr/share/wayland-sessions:/usr/share/xsessions"
user = "greeter"
EOF
```

### 2. BIOS Setup

Set: Config > Power > Sleep State to Windows (S0ix) for proper modern standby.

### 3. Kernel Module Parameters

Create `/etc/modprobe.d/nvidia.conf`:

```conf
# Disable GSP firmware (buggy on some Turing cards)
options nvidia NVreg_EnableGpuFirmware=0
# Enable "fine-grained" (0x02) runtime D3
options nvidia NVreg_DynamicPowerManagement=0x02
# Enable S0ix suspend support
options nvidia NVreg_EnableS0ixPowerManagement=1
```

Blacklist `nvidia_uvm` at boot to prevent a VRAM lock; CUDA loads it on demand.

Create `/etc/modprobe.d/99-nvidia-uvm-blacklist.conf`:

```conf
blacklist nvidia_uvm
```

### 4. Udev Runtime Power Rule

Create `/etc/udev/rules.d/90-nvidia-pm.rules`:

```conf
SUBSYSTEM=="pci", ATTR{vendor}=="0x10de", ATTR{power/control}="auto"
```

Reload udev rules (optional):

```bash
sudo udevadm control --reload
sudo udevadm trigger
```

### 5. Modeset and Rebuild

Ensure `nvidia_drm.modeset=1` is present in GRUB default cmdline.

```bash
sudo sed -i 's/GRUB_CMDLINE_LINUX_DEFAULT="\([^"]*\)"/GRUB_CMDLINE_LINUX_DEFAULT="\1 nvidia_drm.modeset=1"/' /etc/default/grub
sudo mkinitcpio -P
sudo grub-mkconfig -o /boot/grub/grub.cfg
```

### 6. Reboot and Verify

After reboot, the dGPU should power off (`Video Memory: Off`) after ~10–15s of idle.

Check status:

```bash
sudo nvidia-smi --query-gpu=power.draw --format=csv,noheader
cat /sys/bus/pci/devices/*10de*/power/runtime_status || true
```
