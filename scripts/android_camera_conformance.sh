#!/bin/sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
PROJECT_DIR=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
ADB=${ADB:-/Users/littlebuddha/Library/Android/sdk/platform-tools/adb}
PACKAGE_ID=app.universalfilm.camera
APK="$PROJECT_DIR/apps/camera/src-tauri/gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk"
ACTION=${1:-doctor}
REPORT_DIR=${2:-"$PROJECT_DIR/artifacts/android-camera-conformance"}

require_adb() {
  if [ ! -x "$ADB" ]; then
    echo "adb not found or not executable: $ADB" >&2
    exit 1
  fi
}

device_serial() {
  "$ADB" devices | awk 'NR > 1 && $2 == "device" { print $1 }'
}

require_one_device() {
  SERIALS=$(device_serial)
  COUNT=$(printf '%s\n' "$SERIALS" | awk 'NF { count++ } END { print count + 0 }')
  if [ "$COUNT" -ne 1 ]; then
    echo "exactly one authorized Android device is required; found $COUNT" >&2
    "$ADB" devices -l >&2
    exit 2
  fi
  SERIAL=$SERIALS
}

require_adb

case "$ACTION" in
  doctor)
    "$ADB" version
    "$ADB" devices -l
    ;;
  install)
    require_one_device
    if [ ! -f "$APK" ]; then
      echo "APK not found: $APK" >&2
      exit 3
    fi
    "$ADB" -s "$SERIAL" install -r "$APK"
    "$ADB" -s "$SERIAL" shell pm grant "$PACKAGE_ID" android.permission.CAMERA || true
    "$ADB" -s "$SERIAL" shell pm grant "$PACKAGE_ID" android.permission.RECORD_AUDIO || true
    "$ADB" -s "$SERIAL" shell monkey -p "$PACKAGE_ID" -c android.intent.category.LAUNCHER 1
    echo "installed and launched on $SERIAL"
    ;;
  snapshot)
    require_one_device
    mkdir -p "$REPORT_DIR"
    PREFIX="$REPORT_DIR/$SERIAL"
    "$ADB" -s "$SERIAL" shell getprop > "$PREFIX-getprop.txt"
    "$ADB" -s "$SERIAL" shell dumpsys media.camera > "$PREFIX-media-camera.txt"
    "$ADB" -s "$SERIAL" shell dumpsys package "$PACKAGE_ID" > "$PREFIX-package.txt"
    "$ADB" -s "$SERIAL" shell run-as "$PACKAGE_ID" ls -laR files > "$PREFIX-app-files.txt" 2>&1 || true
    "$ADB" -s "$SERIAL" logcat -d -v threadtime > "$PREFIX-logcat.txt"
    echo "snapshot written to $REPORT_DIR"
    ;;
  *)
    echo "usage: $0 {doctor|install|snapshot} [report-directory]" >&2
    exit 64
    ;;
esac
