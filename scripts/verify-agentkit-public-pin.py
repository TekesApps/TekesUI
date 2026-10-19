#!/usr/bin/env python3
"""Verify the reviewed public inputs without importing a sibling checkout."""
import argparse
import subprocess
import re
import hashlib
import json
from pathlib import Path

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--revision', help='verify the frozen immutable Git commit, independently of a newer working tree')
args = parser.parse_args()
if args.revision and not re.fullmatch(r'[0-9a-f]{40}', args.revision):
    parser.error('--revision requires a full immutable commit SHA')
ROOT = Path(__file__).resolve().parents[1]
manifest = json.loads((ROOT / 'docs/agentkit/public-source-037.json').read_text())
files = manifest['files']
actual = hashlib.sha256(json.dumps(files, sort_keys=True, separators=(',', ':')).encode()).hexdigest()
if actual != manifest['sha256'] or len(files) != 188:
    raise SystemExit('Public manifest identity mismatch')
for name, expected in files.items():
    path = ROOT / name
    if args.revision:
        data = subprocess.check_output(['git', 'show', f'{args.revision}:{name}'], cwd=ROOT)
    else:
        if not path.is_file() or path.is_symlink():
            raise SystemExit('Public source missing or symlinked: ' + name)
        data = path.read_bytes()
    if hashlib.sha256(data).hexdigest() != expected:
        raise SystemExit('Public source mismatch: ' + name)
print('Verified 188 reviewed public inputs:', actual, 'source:', args.revision or 'working tree')
