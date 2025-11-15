#!/bin/bash

# --- Define your desired opacity values ---
OPAQUE_ACTIVE="1.0"
OPAQUE_INACTIVE="1.0"
TRANSPARENT_ACTIVE="0.9"  # <--- Adjust if you want different transparency
TRANSPARENT_INACTIVE="0.8" # <--- Adjust if you want different transparency
# -----------------------------------------

STATE_FILE="/tmp/hypr_opacity_state.tmp" # Remembers if opacity is ON or OFF

# Read the current state (assume transparent if state file missing)
CURRENT_STATE=$(cat "$STATE_FILE" 2>/dev/null || echo "transparent")

if [[ "$CURRENT_STATE" == "transparent" ]]; then
  # Currently Transparent -> Switch to Opaque
  hyprctl keyword decoration:active_opacity "$OPAQUE_ACTIVE" > /dev/null
  hyprctl keyword decoration:inactive_opacity "$OPAQUE_INACTIVE" > /dev/null
  echo "opaque" > "$STATE_FILE"
  notify-send -u low -i view-conceal-symbolic "Opacity: OFF" # Example icon
else
  # Currently Opaque -> Switch to Transparent
  hyprctl keyword decoration:active_opacity "$TRANSPARENT_ACTIVE" > /dev/null
  hyprctl keyword decoration:inactive_opacity "$TRANSPARENT_INACTIVE" > /dev/null
  echo "transparent" > "$STATE_FILE"
  notify-send -u low -i view-reveal-symbolic "Opacity: ON ($TRANSPARENT_ACTIVE/$TRANSPARENT_INACTIVE)" # Example icon
fi

exit 0
