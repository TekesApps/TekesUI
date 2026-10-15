#!/usr/bin/env python3
"""Check the imported package and active DS. Never reseal or update goldens."""
from __future__ import annotations
import argparse
import hashlib
import json
from pathlib import Path, PurePosixPath
import subprocess
import sys
import zipfile

ROOT = Path(__file__).resolve().parents[1]
IMPORT = Path('design/imports/v0.3.0-proposal1')
ARCHIVE_SHA = 'cafbe021196d2ec9f575df45a3b0433892d0e2cc3cce36b60937227e1a0ccb9c'
HISTORICAL_MANIFEST_SHA = '205a0cc5c58d6c0230e827a53c4a26aff8643d01414033747311c8876e8d374a'


def sha(data):
    return hashlib.sha256(data).hexdigest()


def archive_errors(root):
    errors = []
    imported = root / IMPORT
    receipt = json.loads((imported / 'import.json').read_text())
    archive = imported / 'source.zip'
    if sha(archive.read_bytes()) != ARCHIVE_SHA or receipt['archive_sha256'] != ARCHIVE_SHA:
        return ['original archive hash changed']
    upstream = imported / receipt['upstream_directory']
    expected = {}
    with zipfile.ZipFile(archive) as package:
        for item in package.infolist():
            name = PurePosixPath(item.filename)
            if name.is_absolute() or '..' in name.parts or ((item.external_attr >> 16) & 0o170000) == 0o120000:
                errors.append('unsafe archive entry: ' + item.filename)
                continue
            if not item.is_dir():
                relative = PurePosixPath(*name.parts[1:]).as_posix()
                expected[relative] = sha(package.read(item))
    actual = {p.relative_to(upstream).as_posix(): sha(p.read_bytes())
              for p in upstream.rglob('*') if p.is_file() and not p.is_symlink()}
    if actual != expected or receipt['files'] != expected or receipt['file_count'] != len(expected):
        errors.append('original extracted package or import inventory changed')
    if any(p.is_symlink() for p in upstream.rglob('*')):
        errors.append('symlink in imported package')
    return errors


def historical_errors(root):
    manifest = root / 'design/baseline/manifest.json'
    if sha(manifest.read_bytes()) != HISTORICAL_MANIFEST_SHA:
        return ['historical manifest changed; replacing its golden hashes does not approve a new baseline']
    baseline = json.loads(manifest.read_text())
    directory = root / baseline['snapshots']['directory']
    expected = {f'{page}-{theme}@{scale}x.png' for page in ('primitives', 'tokens', 'static-text')
                for theme in ('light', 'dark', 'light-ic', 'dark-ic') for scale in (1, 2)}
    errors = []
    if {p.name for p in directory.glob('*.png')} != expected:
        errors.append('historical 24-image inventory changed')
    for entry in baseline['snapshots']['files']:
        path = directory / entry['file']
        if path.is_symlink() or not path.is_file() or sha(path.read_bytes()) != entry['sha256']:
            errors.append('historical golden changed: ' + entry['file'])
    return errors


def references_errors(document, origin, root):
    errors = []
    def visit(value):
        if isinstance(value, dict):
            ref = value.get('$ref')
            if isinstance(ref, str) and not ref.startswith(('https:', 'http:', 'urn:')):
                filename, _, pointer = ref.partition('#')
                target = (origin.parent / filename).resolve() if filename else origin
                if not target.is_relative_to(root) or not target.exists():
                    errors.append(f'{origin.relative_to(root)}: missing or outside local ref {ref}')
                elif pointer:
                    try:
                        part = json.loads(target.read_text())
                        for key in pointer.lstrip('/').split('/'):
                            key = key.replace('~1', '/').replace('~0', '~')
                            part = part[int(key)] if isinstance(part, list) else part[key]
                    except (ValueError, KeyError, TypeError, IndexError, OSError):
                        errors.append(f'{origin.relative_to(root)}: invalid JSON pointer {ref}')
            for child in value.values(): visit(child)
        elif isinstance(value, list):
            for child in value: visit(child)
    visit(document)
    return errors


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--skip-projection-check', action='store_true', help='check archive/metadata only; not full acceptance')
    args = parser.parse_args()
    errors = archive_errors(ROOT)
    import jsonschema
    for filename in ('system', 'tokens'):
        path = ROOT / f'design/{filename}.json'
        value = json.loads(path.read_text())
        schema = json.loads(path.with_suffix('.schema.json').read_text())
        jsonschema.Draft202012Validator.check_schema(schema)
        errors.extend(f'{filename}: {e.message}' for e in jsonschema.Draft202012Validator(schema).iter_errors(value))
        errors.extend(references_errors(value, path, ROOT))
    path = ROOT / 'design/integration.json'
    errors.extend(references_errors(json.loads(path.read_text()), path, ROOT))
    errors.extend(historical_errors(ROOT))
    if not args.skip_projection_check:
        result = subprocess.run([sys.executable, str(ROOT / 'scripts/merge-design-system.py'), '--check'], cwd=ROOT)
        if result.returncode:
            errors.append('design projection is stale or invalid')
    for error in errors: print('FAIL:', error, file=sys.stderr)
    if not errors:
        print('PASS: original 134-file package, DS metadata/refs, historical goldens' + (', merged projection' if not args.skip_projection_check else ' (projection not checked)'))
    return int(bool(errors))


if __name__ == '__main__':
    try:
        raise SystemExit(main())
    except (OSError, ValueError, KeyError, zipfile.BadZipFile) as error:
        print('FAIL:', error, file=sys.stderr)
        raise SystemExit(1)
