#!/bin/bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
MODE="${1:-verify-single}"
case "$MODE" in
  verify-single|window-single|bundle-single) COUNT=1; PROFILE=Single ;;
  verify-three|window-three|bundle-three) COUNT=3; PROFILE=Three ;;
  *) echo 'usage: app-host-prototype.sh {verify,window,bundle}-{single,three}' >&2; exit 2 ;;
esac
cd "$ROOT"
CACHE="$ROOT/target/apphost-native"
MANIFEST="$ROOT/examples/app-host-demo/Cargo.toml"
mkdir -p "$CACHE/evidence"
# Reuse the coordinated review cache without changing its workspace owner or
# cleaning it. Capture this profile's executable while the build lock is held.
python3 - "$COUNT" "$PROFILE" <<'BUILD_PY'
import sys, subprocess, shutil
from pathlib import Path
sys.path.insert(0, 'scripts')
from tekesui_build import build_lock, environment, CACHE as BUILD_CACHE
count, profile = sys.argv[1:]
with build_lock():
    command = ['cargo', '+1.95.0', 'build', '--manifest-path', 'examples/app-host-demo/Cargo.toml', '--no-default-features']
    if count == '3': command += ['--features', 'three-panels']
    subprocess.run(command, env=environment(), check=True)
    shutil.copy2(BUILD_CACHE / 'debug/tekes-ui-app-host-demo', Path('target/apphost-native') / (profile + '-tekes-ui-app-host-demo'))
BUILD_PY
BIN="$CACHE/${PROFILE}-tekes-ui-app-host-demo"
if [[ "$MODE" = verify-* ]]; then
  "$BIN" --verify | tee "$CACHE/evidence/verify-${PROFILE}.json"
  exit 0
fi
BUNDLE="$CACHE/TekesUI AppHost Native ${PROFILE}.app"
mkdir -p "$BUNDLE/Contents/MacOS" "$BUNDLE/Contents/Resources"
cp "$BIN" "$BUNDLE/Contents/MacOS/tekes-ui-app-host-demo.new"
mv -f "$BUNDLE/Contents/MacOS/tekes-ui-app-host-demo.new" "$BUNDLE/Contents/MacOS/tekes-ui-app-host-demo"
cat > "$BUNDLE/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleIdentifier</key><string>org.tekes.ui.apphost.native.${PROFILE}</string>
<key>CFBundleExecutable</key><string>tekes-ui-app-host-demo</string>
<key>CFBundleName</key><string>TekesUI AppHost Native ${PROFILE}</string>
<key>CFBundleDisplayName</key><string>TekesUI AppHost Native ${PROFILE}</string>
<key>CFBundleVersion</key><string>1</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>NSHighResolutionCapable</key><true/>
</dict></plist>
PLIST
codesign --force --sign - "$BUNDLE"
codesign --verify --deep --strict "$BUNDLE"
shasum -a 256 "$BUNDLE/Contents/MacOS/tekes-ui-app-host-demo" "$ROOT/examples/app-host-demo/Cargo.lock" > "$CACHE/evidence/bundle-${PROFILE}.sha256"
echo "$BUNDLE"
if [[ "$MODE" = window-* ]]; then
  exec "$BUNDLE/Contents/MacOS/tekes-ui-app-host-demo" 2> >(tee "$CACHE/evidence/window-${PROFILE}.log" >&2)
fi
