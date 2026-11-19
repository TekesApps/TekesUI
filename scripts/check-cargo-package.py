#!/usr/bin/env python3
"""Verify every publishable workspace crate builds standalone from its packaged .crate archive.

Packages all `publish != false` workspace crates with `cargo package --workspace`,
extracts every `.crate` tarball into a clean isolated temporary directory (outside
the checkout so `../../design` does not exist), patches `[patch.crates-io]` to the
unpacked sibling crates, and runs `cargo check --lib --bins` on each one.
"""
from __future__ import annotations

import argparse
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tarfile
import tempfile

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[1]


def workspace_version(cargo_toml: Path) -> str:
    text = cargo_toml.read_text(encoding='utf-8')
    in_pkg = False
    for raw in text.splitlines():
        line = raw.split('#', 1)[0].strip()
        if line == '[workspace.package]':
            in_pkg = True
            continue
        if line.startswith('['):
            in_pkg = False
        if in_pkg:
            m = re.match(r'^version\s*=\s*"([^"]+)"', line)
            if m:
                return m.group(1)
    raise ValueError('Could not determine [workspace.package] version from Cargo.toml')


def publishable_crates(root: Path, default_version: str) -> list[tuple[str, str]]:
    crates = []
    for manifest in sorted((root / 'crates').glob('*/Cargo.toml')):
        text = manifest.read_text(encoding='utf-8')
        in_pkg = False
        name = None
        version = default_version
        publish = True
        for raw in text.splitlines():
            line = raw.split('#', 1)[0].strip()
            if line == '[package]':
                in_pkg = True
                continue
            if line.startswith('['):
                in_pkg = False
            if in_pkg:
                m = re.match(r'^name\s*=\s*"([^"]+)"', line)
                if m:
                    name = m.group(1)
                m = re.match(r'^version\s*=\s*"([^"]+)"', line)
                if m:
                    version = m.group(1)
                if re.match(r'^publish\s*=\s*false\b', line):
                    publish = False
        if name and publish:
            crates.append((name, version))
    return crates


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        '--offline',
        action='store_true',
        help='Pass --offline to cargo check (requires pre-populated cargo cache)',
    )
    args = parser.parse_args()

    version = workspace_version(ROOT / 'Cargo.toml')
    crates = publishable_crates(ROOT, version)
    if not crates:
        raise ValueError('No publishable crates found under crates/')

    with tempfile.TemporaryDirectory(prefix='tekesui-package-check-') as tmp:
        tmp_path = Path(tmp)
        pkg_target = tmp_path / 'pkg-target'
        unpacked_root = tmp_path / 'unpacked'
        verify_target = tmp_path / 'verify-target'
        unpacked_root.mkdir(parents=True)

        pkg_cmd = [
            'cargo',
            'package',
            '--locked',
            '--no-verify',
            '--allow-dirty',
            '--target-dir',
            str(pkg_target),
        ]
        for name, _ in crates:
            pkg_cmd.extend(['-p', name])
        if args.offline:
            pkg_cmd.append('--offline')
        print('Running:', ' '.join(pkg_cmd))
        proc = subprocess.run(pkg_cmd, cwd=ROOT, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        if proc.returncode != 0:
            print(proc.stdout, file=sys.stderr)
            return proc.returncode

        unpacked_dirs: dict[str, Path] = {}
        for name, ver in crates:
            archive = pkg_target / 'package' / f'{name}-{ver}.crate'
            if not archive.is_file():
                raise FileNotFoundError(f'Missing expected package archive: {archive}')
            with tarfile.open(archive, 'r:gz') as tf:
                tf.extractall(unpacked_root, filter='data')
            crate_dir = unpacked_root / f'{name}-{ver}'
            if not crate_dir.is_dir():
                raise FileNotFoundError(f'Archive {archive.name} did not extract to {crate_dir}')
            unpacked_dirs[name] = crate_dir

        cargo_cfg_dir = unpacked_root / '.cargo'
        cargo_cfg_dir.mkdir(parents=True, exist_ok=True)
        toolchain = ROOT / 'rust-toolchain.toml'
        if toolchain.is_file():
            shutil.copy2(toolchain, unpacked_root / 'rust-toolchain.toml')
        patch_lines = ['[patch.crates-io]']
        for name, crate_dir in sorted(unpacked_dirs.items()):
            escaped = str(crate_dir).replace('\\', '\\\\').replace('"', '\\"')
            patch_lines.append(f'{name} = {{ path = "{escaped}" }}')
        (cargo_cfg_dir / 'config.toml').write_text('\n'.join(patch_lines) + '\n', encoding='utf-8')

        env = dict(os.environ)
        env['CARGO_TARGET_DIR'] = str(verify_target)
        # Ensure packaged crate builds run cleanly without accidental workspace overrides.
        env.pop('TEKESUI_DESIGN_ARTIFACT', None)
        env.pop('TEKESUI_THEME_LOCK', None)

        for name, crate_dir in sorted(unpacked_dirs.items()):
            cmd = ['cargo', 'check', '--lib', '--bins']
            if args.offline:
                cmd.append('--offline')
            print(f'Verifying standalone packaged crate: {name}')
            res = subprocess.run(cmd, cwd=crate_dir, env=env, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
            if res.returncode != 0:
                print(res.stdout, file=sys.stderr)
                print(f'FAIL: packaged crate check failed for {name}', file=sys.stderr)
                return res.returncode

    print(f'PASS: verified {len(crates)} publishable crates build standalone from .crate archives.')
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
