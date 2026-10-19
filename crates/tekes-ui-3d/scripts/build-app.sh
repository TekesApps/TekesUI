#!/bin/sh
set -eu
cd "$(dirname "$0")/../../.."
profile="${TEKESUI3D_PROFILE:-release}"
case "$profile" in
  release) cargo build -p tekes-ui-3d --release --locked ;;
  debug) cargo build -p tekes-ui-3d --locked ;;
  *) printf '%s\n' 'TEKESUI3D_PROFILE must be release or debug' >&2; exit 2 ;;
esac
app="$PWD/target/TekesUI3D.app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
# Replace the inode: overwriting a running signed Mach-O can invalidate its
# mapped code pages and the kernel's code-signature cache.
cp "target/$profile/tekes-ui-3d" "$app/Contents/MacOS/tekes-ui-3d.next"
mv -f "$app/Contents/MacOS/tekes-ui-3d.next" "$app/Contents/MacOS/tekes-ui-3d"
cp crates/tekes-ui-3d/assets/rubiks-cube.glb "$app/Contents/Resources/"
cat > "$app/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleExecutable</key><string>tekes-ui-3d</string>
<key>CFBundleIdentifier</key><string>com.tekesui.3d.demo</string>
<key>CFBundleName</key><string>TekesUI3D</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>NSHighResolutionCapable</key><true/>
</dict></plist>
PLIST
codesign --force --sign - "$app"
printf '%s\n' "$app"
