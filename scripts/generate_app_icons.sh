#!/usr/bin/env bash
set -euo pipefail

project_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$project_root"

npm run tauri -- icon assets/app-icon.svg

if ! command -v ffmpeg >/dev/null 2>&1; then
  echo "ffmpeg is required to remove alpha from iOS AppIcon assets." >&2
  exit 1
fi

icon_temp_dir="$(mktemp -d)"
trap 'rm -rf "$icon_temp_dir"' EXIT

while IFS= read -r -d '' icon_path; do
  output_path="$icon_temp_dir/$(basename "$icon_path")"
  ffmpeg -loglevel error -y -i "$icon_path" -vf format=rgb24 -frames:v 1 "$output_path"
  cp "$output_path" "$icon_path"
done < <(find \
  apps/camera/src-tauri/icons/ios \
  apps/camera/src-tauri/gen/apple/Assets.xcassets/AppIcon.appiconset \
  -type f -name '*.png' -print0)

echo "Generated cross-platform icons and flattened iOS AppIcon alpha."
