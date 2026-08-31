#!/bin/sh
set -eu

if [ "$#" -ne 1 ]; then
  echo "usage: $0 <device-udid>" >&2
  exit 64
fi

workspace_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
artifact="$workspace_root/apps/camera/src-tauri/gen/apple/build/arm64/Universal Film Camera.ipa"
device_id=$1

if [ ! -f "$artifact" ]; then
  echo "missing iOS artifact; run ./scripts/build_ios_device.sh first" >&2
  exit 66
fi

install_dir=$(mktemp -d)
trap 'rm -rf -- "$install_dir"' EXIT HUP INT TERM
unzip -q "$artifact" -d "$install_dir"
app_path=$(find "$install_dir/Payload" -maxdepth 1 -name '*.app' -print -quit)

if [ -z "$app_path" ]; then
  echo "the IPA does not contain an application bundle" >&2
  exit 65
fi

xcrun devicectl device install app --device "$device_id" "$app_path"
xcrun devicectl device process launch --device "$device_id" app.universalfilm.camera
