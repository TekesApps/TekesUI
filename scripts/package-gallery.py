#!/usr/bin/env python3
"""Build the Rust Gallery and package its macOS app icon. Does not launch or publish."""
import argparse
import os
from pathlib import Path
import plistlib
import shutil
import subprocess
import sys
import tempfile
import tomllib

ROOT = Path(__file__).resolve().parents[1]
ASSETS = ROOT / 'crates/tekes-ui-gallery/resources/macos'
APP = ROOT / 'target/TekesUI Gallery.app'

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, help='Package this existing Gallery binary without rebuilding')
    args = parser.parse_args()
    if sys.platform != 'darwin':
        parser.error('macOS packaging requires macOS and codesign')
    binary = args.binary
    if binary is None:
        target = ROOT / 'target/gallery-integration'
        env = dict(os.environ, CARGO_TARGET_DIR=str(target))
        subprocess.run(['cargo', 'build', '--release', '--locked', '--offline', '-p', 'tekes-ui-gallery'], cwd=ROOT, env=env, check=True)
        binary = target / 'release/tekes-ui-gallery'
    binary = binary.resolve(strict=True)
    info = plistlib.loads((ASSETS / 'Info.plist').read_bytes())
    info['CFBundleShortVersionString'] = tomllib.loads((ROOT / 'Cargo.toml').read_text())['workspace']['package']['version']
    if APP.exists():
        existing = plistlib.loads((APP / 'Contents/Info.plist').read_bytes())
        if existing.get('CFBundleIdentifier') != info['CFBundleIdentifier']:
            raise SystemExit('Refusing to replace an unrelated app bundle')
    APP.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='gallery-package-', dir=APP.parent) as tmp:
        staged = Path(tmp) / APP.name
        contents = staged / 'Contents'
        (contents / 'MacOS').mkdir(parents=True)
        (contents / 'Resources').mkdir()
        shutil.copy2(binary, contents / 'MacOS' / info['CFBundleExecutable'])
        compiled = Path(tmp) / 'compiled-icons'
        compiled.mkdir()
        subprocess.run(['xcrun', 'actool', str(ASSETS / 'Gallery.icon'),
                        '--compile', str(compiled), '--platform', 'macosx',
                        '--minimum-deployment-target', info['LSMinimumSystemVersion'],
                        '--app-icon', 'Gallery', '--target-device', 'mac',
                        '--output-partial-info-plist', str(compiled / 'icon-info.plist'),
                        '--output-format', 'human-readable-text'], check=True)
        generated_info = plistlib.loads((compiled / 'icon-info.plist').read_bytes())
        info['CFBundleIconName'] = generated_info['CFBundleIconName']
        shutil.copy2(compiled / 'Assets.car', contents / 'Resources/Assets.car')
        # Keep the supplied ICNS as the older-macOS fallback. Tahoe uses the
        # compiled Icon Composer asset via CFBundleIconName instead.
        shutil.copy2(ASSETS / 'Gallery.icns', contents / 'Resources/Gallery.icns')
        (contents / 'Info.plist').write_bytes(plistlib.dumps(info))
        subprocess.run(['codesign', '--force', '--sign', '-', str(staged)], check=True)
        subprocess.run(['codesign', '--verify', '--strict', str(staged)], check=True)
        backup = Path(tmp) / 'previous.app'
        if APP.exists():
            APP.rename(backup)
        try:
            staged.rename(APP)
        except BaseException:
            if backup.exists():
                backup.rename(APP)
            raise
    print(APP)

if __name__ == '__main__':
    main()
