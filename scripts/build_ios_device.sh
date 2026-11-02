#!/bin/sh
set -eu

workspace_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
apple_project="$workspace_root/apps/camera/src-tauri/gen/apple/universal-film-camera.xcodeproj"
device_app="$workspace_root/apps/camera/src-tauri/gen/apple/build/arm64/Universal Film Camera.app"

# Keep the device build on the same Rust/Swift compatibility path as the iOS
# compile check, without requiring callers to set temporary environment values.
task_compiler=$(rustup which --toolchain stable rustc)
task_rustdoc=$(rustup which --toolchain stable rustdoc)
PATH="$workspace_root/scripts/ios-swift-compat:$(dirname -- "$task_compiler"):$PATH"
export PATH
RUSTC="$task_compiler"
RUSTDOC="$task_rustdoc"
export RUSTC RUSTDOC

# Remove generated products only. Source, app data, and captured media are not
# under either target path.
xcodebuild -project "$apple_project" -scheme universal-film-camera_iOS -sdk iphoneos -configuration Debug clean
if [ -d "$device_app" ]; then
  rm -rf -- "$device_app"
fi
cd "$workspace_root"
npm run tauri -- ios build --debug --target aarch64 --ci --export-method debugging

# Compilation/signing alone cannot detect the iOS 27 no-scene launch trap.
# Validate the exported artifact, not only the source override plist.
task_ipa="$workspace_root/apps/camera/src-tauri/gen/apple/build/arm64/Universal Film Camera.ipa"
task_scenes=$(unzip -p "$task_ipa" 'Payload/Universal Film Camera.app/Info.plist' |
  plutil -extract UIApplicationSceneManifest.UIApplicationSupportsMultipleScenes raw -o - -)
if [ "$task_scenes" != "true" ]; then
  printf '%s\n' 'The exported app is missing the Tao UIScene lifecycle opt-in.' >&2
  exit 1
fi
