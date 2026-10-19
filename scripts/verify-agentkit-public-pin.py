#!/usr/bin/env python3
"""Verify the reviewed public inputs without importing a sibling checkout."""
import argparse
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('manifest', nargs='?', default='docs/agentkit/public-source-037.json')
args = parser.parse_args()
manifest = json.loads((ROOT / args.manifest).read_text())
files = manifest['files']
actual = hashlib.sha256(json.dumps(files, sort_keys=True, separators=(',', ':')).encode()).hexdigest()
if not files or actual != manifest['sha256']:
    raise SystemExit('Public manifest identity mismatch')
for name, expected in files.items():
    path = ROOT / name
    if Path(name).is_absolute() or '..' in Path(name).parts:
        raise SystemExit('Unsafe public source path: ' + name)
    if not path.is_file() or path.is_symlink() or hashlib.sha256(path.read_bytes()).hexdigest() != expected:
        raise SystemExit('Public source mismatch: ' + name)
print('Verified', len(files), 'declared public inputs:', actual)
