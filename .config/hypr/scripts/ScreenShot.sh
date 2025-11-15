#!/bin/bash
# /* ---- 💫 https://github.com/JaKooLit 💫 ---- */  ##
# Screenshots scripts

# variables
time=$(date "+%d-%b_%H-%M-%S")
file="Screenshot_${time}_${RANDOM}.png" # Default file name pattern

# --- Determine Screenshot Directory (Robust Method) ---
pictures_base=""
# Check if xdg-user-dir command is available and executable
if command -v xdg-user-dir >/dev/null 2>&1; then
    pictures_base_candidate=$(xdg-user-dir PICTURES 2>/dev/null)
    # Check if the candidate is a valid directory
    if [ -n "$pictures_base_candidate" ] && [ -d "$pictures_base_candidate" ]; then
        pictures_base="$pictures_base_candidate"
    fi
fi

# Fallback if xdg-user-dir failed, was not available, or didn't give a valid directory
if [ -z "$pictures_base" ]; then
    pictures_base="$HOME/Pictures" # Default to $HOME/Pictures
    # Optional: notify user that a fallback is being used if you want to debug xdg-user-dir issues
    # notify-send -u low "Screenshot Script Info" "xdg-user-dir PICTURES failed or not found. Using default: $pictures_base"
fi

dir="$pictures_base/Screenshots" # Define the final screenshots directory

# --- Ensure the directory exists ---
# This needs to happen BEFORE dir is used to construct file paths for grim
if [[ ! -d "$dir" ]]; then
    mkdir -p "$dir"
    if [ $? -ne 0 ]; then
        # If mkdir failed, we have a serious problem.
        notify-send -u critical "Screenshot Script Error" "Could not create screenshot directory: $dir. Check permissions."
        # Fallback to a temporary directory as a last resort
        dir_fallback="/tmp/screenshots_$(whoami)_${RANDOM}"
        notify-send -u low "Screenshot Script Info" "Using temporary fallback directory: $dir_fallback"
        dir="$dir_fallback"
        mkdir -p "$dir"
        if [ $? -ne 0 ]; then
            notify-send -u critical "Screenshot Script Error" "FATAL: Could not create even temporary screenshot directory: $dir. Exiting."
            exit 1
        fi
    fi
fi
# --- End of Directory Setup ---


# These variables depend on $dir being correctly set and existing
iDIR="$HOME/.config/swaync/icons"
iDoR="$HOME/.config/swaync/images"
sDIR="$HOME/.config/hypr/scripts" # Assuming this is for other scripts, not screenshot save path

active_window_class=$(hyprctl -j activewindow | jq -r '(.class)')
active_window_file="Screenshot_${time}_${active_window_class}.png" # This uses $file pattern, but for active window
active_window_path="${dir}/${active_window_file}" # This uses the now robust $dir

notify_cmd_base="notify-send -t 10000 -A action1=Open -A action2=Delete -h string:x-canonical-private-synchronous:shot-notify"
notify_cmd_shot="${notify_cmd_base} -i ${iDIR}/picture.png "
notify_cmd_shot_win="${notify_cmd_base} -i ${iDIR}/picture.png " # Same as above, can be consolidated if always same icon
notify_cmd_NOT="notify-send -u low -i ${iDoR}/note.png "

# notify and view screenshot
notify_view() {
    # $1 is the type of shot ("active", "swappy", or general)
    # $2 (for swappy) is the tmpfile path
    local shot_type="$1"
    local tmp_file_for_swappy="$2" # Only used if shot_type is "swappy"
    local file_to_check # This will be the actual saved file path for most cases

    if [[ "$shot_type" == "active" ]]; then
        file_to_check="${active_window_path}" # Path for active window screenshot
        if [[ -e "$file_to_check" ]]; then
            "${sDIR}/Sounds.sh" --screenshot
            resp=$(timeout 5 ${notify_cmd_shot_win} "Screenshot of:" " ${active_window_class} Saved.")
            case "$resp" in
                action1) xdg-open "$file_to_check" & ;;
                action2) rm "$file_to_check" & ;;
            esac
        else
            ${notify_cmd_NOT} "Screenshot of:" " ${active_window_class} NOT Saved."
            "${sDIR}/Sounds.sh" --error
        fi
    elif [[ "$shot_type" == "swappy" ]]; then
        # For swappy, the notification is different as the file might not be saved yet by swappy itself
        "${sDIR}/Sounds.sh" --screenshot
        resp=$(${notify_cmd_shot} "Screenshot:" "Captured. Edit with Swappy?")
        case "$resp" in
            action1)
                # If swappy saves, it will save to its own chosen location or prompt.
                # The original grim output was in tmp_file_for_swappy.
                # We pass the content of tmp_file_for_swappy to swappy via stdin.
                swappy -f - < "$tmp_file_for_swappy"
                rm "$tmp_file_for_swappy" # Clean up temp file after swappy is done with its content
                ;;
            action2)
                rm "$tmp_file_for_swappy" # User chose to delete before editing with swappy
                ;;
        esac
    else # General shot (now, in5, in10, win, area)
        file_to_check="${dir}/${file}" # Path for general screenshots
        if [[ -e "$file_to_check" ]]; then
            "${sDIR}/Sounds.sh" --screenshot
            resp=$(timeout 5 ${notify_cmd_shot} "Screenshot" "Saved to $file_to_check")
            case "$resp" in
                action1) xdg-open "$file_to_check" & ;;
                action2) rm "$file_to_check" & ;;
            esac
        else
            ${notify_cmd_NOT} "Screenshot" "NOT Saved (File: $file_to_check)"
            "${sDIR}/Sounds.sh" --error
        fi
    fi
}

# countdown
countdown() {
    for sec in $(seq $1 -1 1); do
        notify-send -h string:x-canonical-private-synchronous:shot-notify -t 1000 -i "$iDIR"/timer.png  "Taking shot" "in: $sec secs"
        sleep 1
    done
}

# take shots
shotnow() {
    # file variable is global, dir is now robustly defined
    grim - | tee "${dir}/${file}" | wl-copy # Save to the correct $dir
    sleep 0.5 # Brief pause for file system
    notify_view "general"
}

shot5() {
    countdown '5'
    grim - | tee "${dir}/${file}" | wl-copy
    sleep 0.5
    notify_view "general"
}

shot10() {
    countdown '10'
    grim - | tee "${dir}/${file}" | wl-copy
    sleep 0.5
    notify_view "general"
}

shotwin() {
    # Captures the currently focused window; grim determines geometry
    # active_window_path is already defined using the robust $dir
    hyprctl -j activewindow | jq -r '"\(.at[0]),\(.at[1]) \(.size[0])x\(.size[1])"' | grim -g - "${active_window_path}"
    sleep 0.5
    notify_view "active" # Use "active" to use active_window_path
}

shotarea() {
    # file variable is global, dir is now robustly defined
    local selected_area
    selected_area=$(slurp -d 2>/dev/null) # -d for dimensions, suppress stderr if selection cancelled
    if [ -z "$selected_area" ]; then
        ${notify_cmd_NOT} "Screenshot" "Area selection cancelled."
        "${sDIR}/Sounds.sh" --error
        return
    fi
    grim -g "$selected_area" - | tee "${dir}/${file}" | wl-copy
    sleep 0.5
    notify_view "general"
}

shotactive() {
    # active_window_path is already defined using the robust $dir
    hyprctl -j activewindow | jq -r '"\(.at[0]),\(.at[1]) \(.size[0])x\(.size[1])"' | grim -g - "${active_window_path}"
    sleep 0.5
    notify_view "active"
}

shotswappy() {
    local tmpfile_swappy
    tmpfile_swappy=$(mktemp --suffix=.png) # Create a temp file for grim's output
    local selected_area_swappy
    selected_area_swappy=$(slurp -d 2>/dev/null)

    if [ -z "$selected_area_swappy" ]; then
        ${notify_cmd_NOT} "Screenshot" "Area selection for Swappy cancelled."
        "${sDIR}/Sounds.sh" --error
        rm "$tmpfile_swappy" # Clean up unused temp file
        return
    fi

    grim -g "$selected_area_swappy" "$tmpfile_swappy" # Grim saves directly to the temp file

    if [[ -s "$tmpfile_swappy" ]]; then # Check if grim actually created a file
        wl-copy < "$tmpfile_swappy"
        notify_view "swappy" "$tmpfile_swappy" # Pass tmpfile to notify_view for swappy
        # Note: notify_view for swappy will handle rm of tmpfile_swappy after swappy is done or if user cancels
    else
        ${notify_cmd_NOT} "Screenshot" "Failed to capture for Swappy."
        "${sDIR}/Sounds.sh" --error
        rm "$tmpfile_swappy" # Clean up if grim failed
    fi
}


# Main logic based on argument
if [[ "$1" == "--now" ]]; then
    shotnow
elif [[ "$1" == "--in5" ]]; then
    shot5
elif [[ "$1" == "--in10" ]]; then
    shot10
elif [[ "$1" == "--win" ]]; then
    # 'shotwin' was originally capturing any window based on geometry.
    # For "active window", 'shotactive' is more direct.
    # If 'shotwin' was meant for something else (e.g. select window with slurp), it needs adjustment.
    # Assuming it meant the currently active window as per original logic:
    shotactive
elif [[ "$1" == "--area" ]]; then
    shotarea
elif [[ "$1" == "--active" ]]; then
    shotactive
elif [[ "$1" == "--swappy" ]]; then
    shotswappy
else
    echo -e "Available Options : --now --in5 --in10 --win --area --active --swappy"
fi

exit 0

