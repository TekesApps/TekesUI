#!/bin/sh
set -eu
cd "$(dirname "$0")/../../.."
profile="${TEKESUI3D_PROFILE:-release}"
case "$profile" in
  release) cargo build -p tekes-ui-3d --release --locked ;;
  debug) cargo build -p tekes-ui-3d --locked ;;
  *) printf '%s\n' 'TEKESUI3D_PROFILE must be release or debug' >&2; exit 2 ;;
esac
case "${TEKESUI3D_DEMO:-cube}" in
  cube) app_name="TekesUI3D"; executable="tekes-ui-3d"; bundle_id="com.tekesui.3d.demo" ;;
  manhattan) app_name="TekesUI Manhattan"; executable="tekes-ui-manhattan"; bundle_id="com.tekesui.3d.manhattan" ;;
  city-block) app_name="TekesUI City Block"; executable="tekes-ui-city-block"; bundle_id="com.tekesui.3d.city-block" ;;
  health-cabin) app_name="TekesUI Health Cabin"; executable="tekes-ui-health-cabin"; bundle_id="com.tekesui.3d.health-cabin" ;;
  *) printf '%s\n' 'TEKESUI3D_DEMO must be cube, manhattan, health-cabin or city-block' >&2; exit 2 ;;
esac
app="$PWD/target/$app_name.app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
# Replace the inode: overwriting a running signed Mach-O can invalidate its
# mapped code pages and the kernel's code-signature cache.
cp "target/$profile/tekes-ui-3d" "$app/Contents/MacOS/$executable.next"
mv -f "$app/Contents/MacOS/$executable.next" "$app/Contents/MacOS/$executable"
cp crates/tekes-ui-3d/assets/city-block.glb "$app/Contents/Resources/"
cp crates/tekes-ui-3d/assets/city-block.json "$app/Contents/Resources/"
cp crates/tekes-ui-3d/assets/health-cabin.glb "$app/Contents/Resources/"
cp crates/tekes-ui-3d/assets/manhattan.glb "$app/Contents/Resources/"
cp crates/tekes-ui-3d/assets/rubiks-cube.glb "$app/Contents/Resources/"
cat > "$app/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleExecutable</key><string>$executable</string>
<key>CFBundleIdentifier</key><string>$bundle_id</string>
<key>CFBundleName</key><string>$app_name</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>NSHighResolutionCapable</key><true/>
</dict></plist>
PLIST
codesign --force --sign - "$app"
printf '%s\n' "$app"
