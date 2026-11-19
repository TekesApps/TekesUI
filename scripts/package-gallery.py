#!/usr/bin/env python3
"""Package Gallery with its app icon and embedded-catalog identity manifest; never launch or publish."""
from __future__ import annotations

import argparse
import os
from pathlib import Path
import plistlib
import shutil
import subprocess
import sys
import tempfile
try:
    import tomllib
except ModuleNotFoundError:
    tomllib = None
from tekesui_build import CACHE, assert_unused, build_lock, environment, prepare_cache

ROOT = Path(__file__).resolve().parents[1]
ASSETS = ROOT / 'crates/tekes-ui-gallery/resources/macos'
APP = ROOT / 'target/TekesUI Gallery.app'

def read_workspace_version(manifest_path: Path) -> str:
    text = manifest_path.read_text()
    if tomllib is not None:
        return tomllib.loads(text)['workspace']['package']['version']
    in_ws_pkg = False
    for raw in text.splitlines():
        line = raw.split('#', 1)[0].strip()
        if not line:
            continue
        if line.startswith('['):
            in_ws_pkg = line == '[workspace.package]'
            continue
        if in_ws_pkg and line.startswith('version'):
            key, _, val = line.partition('=')
            if key.strip() == 'version':
                return val.strip().strip('"\'')
    raise RuntimeError(f'Unable to locate [workspace.package].version in {manifest_path}')


def resolve_build_command(args, binary=None):
    offline = bool(getattr(args, 'offline', False) or os.environ.get('TEKESUI_OFFLINE') == '1')
    command = ['cargo', 'build', '--locked']
    if offline:
        command.append('--offline')
    command.extend(['-j', '2', '-p', 'tekes-ui-gallery'])
    if getattr(args, 'release', False):
        command.append('--release')
    return command


def validate_release_tag(version: str, tag: str | None) -> None:
    if not tag:
        return
    normalized = tag.removeprefix('refs/tags/')
    expected = f'v{version}'
    if normalized != expected:
        raise SystemExit(
            f'Release tag {normalized!r} does not match workspace/bundle version {expected!r}'
        )


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, help='Package this existing Gallery binary without rebuilding')
    parser.add_argument('--release', action='store_true', help='Explicit release integration build (default: compact dev build)')
    parser.add_argument('--offline', action='store_true', help='Pass --offline to cargo build (or set TEKESUI_OFFLINE=1)')
    parser.add_argument('--verify-tag', help='Verify release git tag matches workspace and Info.plist version')
    args = parser.parse_args()
    version = read_workspace_version(ROOT / 'Cargo.toml')
    validate_release_tag(version, args.verify_tag)
    if sys.platform != 'darwin':
        parser.error('macOS packaging requires macOS and codesign')
    binary = args.binary
    if binary is None:
        prepare_cache()
        command = resolve_build_command(args)
        if args.release:
            command.append('--release')
        subprocess.run(command, cwd=ROOT, env=environment(), check=True)
        binary = CACHE / ('release' if args.release else 'debug') / 'tekes-ui-gallery'
    binary = binary.resolve(strict=True)
    info = plistlib.loads((ASSETS / 'Info.plist').read_bytes())
    assert version, 'workspace version must be non-empty'
    info['CFBundleShortVersionString'] = version
    if APP.exists():
        assert_unused([APP])
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
        # Glyph::ALL embeds the complete SVG catalog in the executable. Keep the
        # public identity manifest beside it so the package-outside-checkout
        # probe can reject stale binaries against these exact canonical inputs.
        shutil.copy2(ROOT / 'crates/tekes-ui-icons/catalog.json',
                     contents / 'Resources/IconCatalog.json')
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
    with build_lock():
        main()
