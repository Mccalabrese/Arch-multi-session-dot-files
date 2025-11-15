#!/usr/bin/env bash

iDIR="$HOME/.config/swaync/images/ja.png"

# kill any existing instance (in case of reload)
pkill swayidle 2>/dev/null

swayidle -w \
  timeout 60 'brightnessctl set 0%' \
  resume 'brightnessctl set 15%' \
  timeout 240 'pidof hyprlock || hyprlock &' \
  timeout 270 'wlr-randr --output eDP-1 --off; wlr-randr --output DP-2 --off; wlr-randr --output DP-3 --off; wlr-randr --output DP-4 --off' \
  resume 'wlr-randr --output eDP-1 --on; wlr-randr --output DP-2 --on; wlr-randr --output DP-3 --on; wlr-randr --output DP-4 --on' \
  timeout 600 'systemctl suspend' \
  before-sleep 'pidof hyprlock || hyprlock &'
