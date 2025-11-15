#!/bin/bash
current_factor=$(hyprctl getoption cursor:zoom_factor | awk 'NR==1 {print $2}')
# Ensure factor is at least 1 before dividing
if (( $(echo "$current_factor < 1.0" | bc -l) )); then
     current_factor=1.0
fi
# Prevent zooming out too far (optional minimum, e.g., 0.5 or 1.0)
min_factor=0.5
new_factor=$(echo "$current_factor / 2.0" | bc -l)
if (( $(echo "$new_factor < $min_factor" | bc -l) )); then
    new_factor=$min_factor
fi
hyprctl keyword cursor:zoom_factor "$new_factor"
