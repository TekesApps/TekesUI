"""Refresh actual bounded probes against the current source; preserve historical files."""
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[3]
OUT = Path(__file__).resolve().parent / 'token-probes'
JOBS = ['action', 'input', 'selection', 'combo-async', 'command-async', 'content-status', 'slider-composition']


def hashes(paths):
    return {p: hashlib.sha256((ROOT / p).read_bytes()).hexdigest() for p in sorted(paths)}


def refresh_direct(name, env_key, test, count):
    old = json.loads((ROOT / f'design/reviews/field-metadata-20261019/token-probes/{name}.json').read_text())
    paths = set(old['inputs_sha256'])
    for prefix in ['crates/tekes-ui-controls/src', 'crates/tekes-ui-tokens/src', 'crates/tekes-ui-design-export/src']:
        paths.update(p.relative_to(ROOT).as_posix() for p in (ROOT / prefix).rglob('*.rs'))
    paths.add(Path(__file__).relative_to(ROOT).as_posix())
    before = hashes(paths)
    command = ['cargo', 'test', '-p', 'tekes-ui-controls', '--features', 'token-audit', '--test', test, '--locked']
    with tempfile.TemporaryDirectory(prefix='tekesui-current-probe-') as folder:
        raw = Path(folder) / 'observed.json'
        env = os.environ.copy()
        env[env_key] = str(raw)
        subprocess.run(command, cwd=ROOT, env=env, check=True)
        data = json.loads(raw.read_text())
    assert data['complete'] is False and len(data['cases']) == count
    if hashes(paths) != before:
        raise RuntimeError('source changed during the public API probe')
    data.update(format=old['format'], inputs_sha256=before, public_api_test=old['public_api_test'], probe_command=command)
    if 'scope' not in data:
        data['scope'] = old['scope']
    with (OUT / f'{name}.json').open('x') as handle:
        json.dump(data, handle, indent=2)
        handle.write('\n')


def main():
    OUT.mkdir(exist_ok=True)
    for name in JOBS:
        subprocess.run([sys.executable, f'scripts/audit-{name}-tokens.py', '--output', str(OUT / f'{name}.json')], cwd=ROOT, check=True)
    refresh_direct('field', 'TEKESUI_FIELD_AUDIT_OUT', 'field_contract', 48)
    refresh_direct('number', 'TEKESUI_NUMBER_STATE_AUDIT_OUT', 'number_field_contract', 60)
    # Rebind only after every real probe completed successfully, and confirm all
    # resulting receipts still refer to current bytes before changing coverage.
    for path in OUT.glob('*.json'):
        data = json.loads(path.read_text())
        if hashes(data['inputs_sha256']) != data['inputs_sha256']:
            raise RuntimeError(f'{path}: source drift after probe')
    print('Nine fresh bounded observations verified; current coverage bindings left unchanged pending native candidate acceptance.')


if __name__ == '__main__':
    main()
