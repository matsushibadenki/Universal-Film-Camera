#!/bin/sh
set -eu

workspace_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
apple_project="$workspace_root/apps/camera/src-tauri/gen/apple/universal-film-camera.xcodeproj"
simulator_app="$workspace_root/apps/camera/src-tauri/gen/apple/build/arm64-sim/Universal Film Camera.app"

# Tauri CLI 2.11.x refuses to rename a freshly archived .app over an older
# build/arm64-sim product. Xcode clean removes only generated build products.
xcodebuild -project "$apple_project" -scheme universal-film-camera_iOS -sdk iphonesimulator -configuration Debug clean
if [ -d "$simulator_app" ]; then
  rm -rf -- "$simulator_app"
fi
cd "$workspace_root"
npm run tauri -- ios build --debug --target aarch64-sim --no-sign --ci
