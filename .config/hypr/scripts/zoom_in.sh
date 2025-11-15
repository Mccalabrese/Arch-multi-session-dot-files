#!/bin/bash
current_factor=$(hyprctl getoption cursor:zoom_factor | awk 'NR==1 {print $2}')
# Ensure factor is at least 1 before multiplying
if (( $(echo "$current_factor < 1.0" | bc -l) )); then
    current_factor=1.0
fi
new_factor=$(echo "$current_factor * 2.0" | bc -l)
hyprctl keyword cursor:zoom_factor "$new_factor"
