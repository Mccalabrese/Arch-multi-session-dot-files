#!/bin/bash
SESSION_NAME="default_tmux" # Or any preferred persistent session name

# Check if the session already exists
if tmux has-session -t "$SESSION_NAME" 2>/dev/null; then
  # If the session exists, attach to it
  tmux attach-session -t "$SESSION_NAME"
else
  # If the session doesn't exist, start a new one and attach
  tmux new-session -s "$SESSION_NAME"
fi