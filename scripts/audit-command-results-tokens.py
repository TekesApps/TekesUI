#!/usr/bin/env python3
"""Bounded production CommandPalette trigger and result observations; no runtime status type."""
import argparse, hashlib, importlib.util, json, os, subprocess, tempfile
from pathlib import Path
ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('trigger', ROOT/'scripts/audit-choice-trigger-tokens.py')
trigger = importlib.util.module_from_spec(spec)
spec.loader.exec_module(trigger)
def inputs():
    data = trigger.inputs()
    for path in ['scripts/audit-command-results-tokens.py', 'crates/tekes-ui-controls/tests/command_result_states.rs']:
        data[path] = hashlib.sha256((ROOT/path).read_bytes()).hexdigest()
    return data
def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    if args.output.exists(): parser.error('use a fresh evidence path')
    before = inputs()
    command = ['cargo','test','-p','tekes-ui-controls','--features','token-audit','--test','command_result_states','--locked']
    with tempfile.TemporaryDirectory(prefix='tekesui-command-results-') as folder:
        folder = Path(folder)
        subprocess.run(['python3', 'scripts/audit-choice-trigger-tokens.py', '--output', str(folder/'trigger.json')], cwd=ROOT, check=True)
        data = json.loads((folder/'trigger.json').read_text())
        env = os.environ.copy(); env['TEKESUI_COMMAND_RESULTS_AUDIT_OUT'] = str(folder/'results.json')
        subprocess.run(command, cwd=ROOT, env=env, check=True)
        results = json.loads((folder/'results.json').read_text())
    assert len(results['cases']) == 36 and all(c['state_asserted'] is True for c in results['cases'])
    assert {c['choice_state'] for c in results['cases']} == {'selected', 'open', 'empty'}
    if before != inputs(): raise RuntimeError('source changed during probe')
    data['cases'] += results['cases']
    assert len(data['cases']) == 120
    data.update(inputs_sha256=before, probe_command=command,
                public_api_test='crates/tekes-ui-controls/tests/command_result_states.rs',
                scope='Original CommandPaletteView five trigger states and three result states; caller TextSurface; four themes/three brands; real repeated activation; CoreText layout with no-op glyph painter',
                excluded=['loading/error composition', 'complete combinations/dependency extraction', 'native AX/VoiceOver/physical IME'])
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open('x') as stream: json.dump(data, stream, indent=2); stream.write('\n')
    print('Wrote120 bounded trigger/result cases; complete=False')
if __name__ == '__main__': main()
