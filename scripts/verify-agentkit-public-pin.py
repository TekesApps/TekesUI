#!/usr/bin/env python3
"""Verify the reviewed public inputs without importing a sibling checkout."""
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
manifest = json.loads((ROOT / 'docs/agentkit/public-source-037.json').read_text())
files = manifest['files']
actual = hashlib.sha256(json.dumps(files, sort_keys=True, separators=(',', ':')).encode()).hexdigest()
if actual != manifest['sha256'] or len(files) != 188:
    raise SystemExit('Public manifest identity mismatch')
for name, expected in files.items():
    path = ROOT / name
    if not path.is_file() or path.is_symlink() or hashlib.sha256(path.read_bytes()).hexdigest() != expected:
        raise SystemExit('Public source mismatch: ' + name)
print('Verified 188 reviewed public inputs:', actual)
