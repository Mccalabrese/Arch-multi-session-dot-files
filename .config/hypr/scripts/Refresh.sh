#!/bin/bash
# /* ---- 💫 https://github.com/JaKooLit 💫 ---- */  ##
# Scripts for refreshing ags, waybar, rofi, swaync, wallust

SCRIPTSDIR=$HOME/.config/hypr/scripts
UserScripts=$HOME/.config/hypr/UserScripts

# Define file_exists function
file_exists() {
  if [ -e "$1" ]; then
    return 0 # File exists
  else
    return 1 # File does not exist
  fi
}

# Kill already running processes
_ps=(rofi swaync ags)
for _prs in "${_ps[@]}"; do
  if pidof "${_prs}" >/dev/null; then
    pkill "${_prs}"
  fi
done

# quit ags & relaunch ags
#ags -q && ags &

# some process to kill
for pid in $(pidof rofi swaync ags swaybg); do
  kill -SIGUSR1 "$pid"
done

# relaunch swaync
sleep 0.5
swaync >/dev/null 2>&1 &
# reload swaync
swaync-client --reload-config

exit 0

