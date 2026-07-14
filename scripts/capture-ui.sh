#!/usr/bin/env bash
set -euo pipefail

output="${1:-artifacts/slate-ui-$(date +%Y%m%d-%H%M%S).png}"
mkdir -p "$(dirname "$output")"

window_json="$(hyprctl clients -j)"
window_box="$(
  jq -r '
    map(select(.title == "Slate"))
    | last
    | if . == null then empty else "\(.at[0]),\(.at[1]) \(.size[0])x\(.size[1])" end
  ' <<<"$window_json"
)"

if [[ -z "$window_box" ]]; then
  printf 'No visible Slate window found. Start the app first.\n' >&2
  exit 1
fi

grim -g "$window_box" "$output"
printf '%s\n' "$output"
