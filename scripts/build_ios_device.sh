#!/bin/sh
set -eu

workspace_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
apple_project="$workspace_root/apps/camera/src-tauri/gen/apple/universal-film-camera.xcodeproj"
device_app="$workspace_root/apps/camera/src-tauri/gen/apple/build/arm64/Universal Film Camera.app"

# Remove generated products only. Source, app data, and captured media are not
# under either target path.
xcodebuild -project "$apple_project" -scheme universal-film-camera_iOS -sdk iphoneos -configuration Debug clean
if [ -d "$device_app" ]; then
  rm -rf -- "$device_app"
fi
cd "$workspace_root"
npm run tauri -- ios build --debug --target aarch64 --ci --export-method debugging
