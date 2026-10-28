#!/usr/bin/env python3
"""Build/check the current public consumer, retain evidence, clean round artifacts."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import plistlib
import re
import shlex
import shutil
import subprocess
import sys
from tekesui_build import ROOT, CACHE, assert_unused, build_lock, environment, prepare_cache, remove_owned

RECEIPTS = ROOT / 'design/reviews/component-runs'
RUNTIME = ROOT / 'target/component-review'
MANIFEST = ROOT / 'examples/composition-consumer/Cargo.toml'
LAYERS = ('api', 'consumer', 'gpu', 'native', 'gold7', 'gallery')
CASES = ('list', 'dialog', 'slider', 'command', 'reading', 'plain', 'number', 'field', 'label', 'menu-tree', 'content-help')  # Only existing real consumers; never silently build Gallery.


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def inputs():
    """Include WIP and generated design inputs, not just HEAD or tracked Rust files."""
    names = subprocess.check_output(['git', 'ls-files', '-co', '--exclude-standard', '-z'], cwd=ROOT).decode().split('\0')
    return {name: digest(ROOT / name) for name in sorted(set(names)) if name
            and not name.startswith(('design/reviews/', 'docs/', 'target/'))
            and (name.startswith(('crates/', 'design/', 'examples/composition-consumer/', 'scripts/'))
                 or name in ('Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml'))
            and (ROOT / name).is_file() and not (ROOT / name).is_symlink()}


def identifier(value):
    if not re.fullmatch(r'[a-z0-9][a-z0-9-]{0,79}', value):
        raise argparse.ArgumentTypeError('Use a short lowercase round ID, e.g. slider-20261019')
    return value


def load(round_id):
    path = RECEIPTS / round_id / 'receipt.json'
    data = json.loads(path.read_text())
    if data['status'] != 'open':
        raise SystemExit('Round is already finished')
    if data['inputs'] != inputs():
        raise SystemExit('Source/WIP changed since build; discard this round and build a new one')
    return path, data


def save(path, data):
    path.write_text(json.dumps(data, indent=2, sort_keys=True) + '\n')


def run(command, log):
    with log.open('w') as output:
        result = subprocess.run(command, cwd=ROOT, env=environment(), stdout=output, stderr=subprocess.STDOUT)
    return {'result': 'passed' if result.returncode == 0 else 'failed',
            'command': command, 'exit_code': result.returncode, 'evidence': log.name, 'sha256': digest(log)}


def build(args):
    receipt = RECEIPTS / args.round / 'receipt.json'
    if receipt.parent.exists():
        raise SystemExit('Round ID already exists; evidence is never overwritten')
    if RUNTIME.exists() and any(RUNTIME.glob('*/.owned.json')):
        raise SystemExit('Finish or discard the previous component round first')
    prepare_cache()
    receipt.parent.mkdir(parents=True)
    runtime = RUNTIME / args.round
    runtime.mkdir(parents=True)
    (runtime / '.owned.json').write_text(json.dumps({'round': args.round, 'root': str(ROOT)}))
    data = {'schema': 1, 'case': args.case, 'status': 'open', 'inputs': inputs(),
            'head': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(),
            'required': list(LAYERS if args.integration else LAYERS[:-1]), 'layers': {},
            'cache': str(CACHE), 'artifacts': []}
    save(receipt, data)
    command = ['cargo', 'build', '--manifest-path', str(MANIFEST), '--example', 'native',
               '--locked', '--offline', '-j', '2', '--message-format=json']
    outcome = run(command, receipt.parent / 'build.log')
    data['build'] = outcome
    if outcome['exit_code']:
        save(receipt, data)
        raise SystemExit('Build failed; log retained. Use finish --discard after investigation')
    executable = None
    for line in (receipt.parent / 'build.log').read_text().splitlines():
        try:
            event = json.loads(line)
        except ValueError:
            continue
        if event.get('reason') == 'compiler-artifact' and event['target']['name'] == 'native' and event.get('executable'):
            executable = Path(event['executable'])
            for name in event['filenames']:
                path = Path(name)
                if CACHE not in path.parents:
                    raise SystemExit('Unexpected artifact outside shared cache')
                data['artifacts'].append({'path': name, 'sha256': digest(path)})
    if executable is None:
        raise SystemExit('Cargo did not report the native consumer executable')
    # Cargo also retains a hash-named example binary alongside its reported
    # executable. Record only the exact matching build, never delete by glob.
    binary_hash = digest(executable)
    for candidate in executable.parent.glob(executable.name + '-*'):
        if (re.fullmatch(executable.name + r'-[0-9a-f]+', candidate.name)
                and candidate.is_file() and not candidate.is_symlink()
                and digest(candidate) == binary_hash):
            data['artifacts'].append({'path': str(candidate), 'sha256': binary_hash})
            depinfo = candidate.with_suffix('.d')
            if depinfo.is_file():
                data['artifacts'].append({'path': str(depinfo), 'sha256': digest(depinfo)})
    depinfo = executable.with_suffix('.d')
    if depinfo.is_file():
        data['artifacts'].append({'path': str(depinfo), 'sha256': digest(depinfo)})
    app = runtime / 'TekesUI Component Review.app'
    macos = app / 'Contents/MacOS'
    macos.mkdir(parents=True)
    shutil.copy2(executable, macos / 'ComponentCore')
    launcher = macos / 'ComponentReview'
    launcher.write_text('#!/bin/sh\n' + ('export TEKES_SELFTEST=text\n' if args.case == 'plain' else '')
                        + 'export TEKESUI_COMPONENT_EVIDENCE=' + shlex.quote(str(receipt.parent / 'native'))
                        + '\ncd "$(dirname "$0")"\nexec ./ComponentCore ' + shlex.quote(args.case) + '\n')
    launcher.chmod(0o755)
    info = {'CFBundleIdentifier': 'com.tekesui.gallery.component-review', 'CFBundleName': 'TekesUI Component Review',
            'CFBundleExecutable': 'ComponentReview', 'CFBundlePackageType': 'APPL',
            'CFBundleVersion': '1', 'NSHighResolutionCapable': True, 'LSMinimumSystemVersion': '14.0'}
    (app / 'Contents/Info.plist').write_bytes(plistlib.dumps(info))
    subprocess.run(['codesign', '--force', '--sign', '-', str(app)], check=True)
    subprocess.run(['codesign', '--verify', '--strict', str(app)], check=True)
    data['app'] = str(app)
    data['binary_sha256'] = binary_hash
    data['source_changed_during_build'] = data['inputs'] != inputs()
    save(receipt, data)
    if data['source_changed_during_build']:
        raise SystemExit('Source changed during build; discard and rebuild')
    print(app)


def check(args):
    path, data = load(args.round)
    prepare_cache()
    commands = {
        'api': ['cargo', 'test', '--locked', '--offline', '-j', '2', '-p', 'tekes-ui-controls', '--lib'],
        'consumer': ['cargo', 'test', '--manifest-path', str(MANIFEST), '--locked', '--offline', '-j', '2', '--tests'],
    }
    if args.gallery:
        commands['gallery'] = ['cargo', 'test', '--locked', '--offline', '-j', '2', '-p', 'tekes-ui-gallery', '--bins']
    for layer, command in commands.items():
        if layer in data['layers']:
            if data['layers'][layer]['result'] != 'passed':
                raise SystemExit('Failed check already recorded; use a new round')
            continue
        outcome = run(command, path.parent / (layer + '.log'))
        data['layers'][layer] = outcome
        save(path, data)
        print(layer + ': ' + outcome['result'], flush=True)
        if outcome['exit_code']:
            raise SystemExit('Check failed; evidence retained')
    if data['inputs'] != inputs():
        raise SystemExit('Source changed during checks; discard this round')


def record(args):
    path, data = load(args.round)
    if args.layer in data['layers']:
        raise SystemExit('Layer already recorded; evidence is immutable')
    source = args.evidence.resolve(strict=True)
    if not source.is_file():
        raise SystemExit('Evidence must be a file')
    dest = path.parent / (args.layer + '-observed' + source.suffix)
    shutil.copy2(source, dest)
    data['layers'][args.layer] = {'result': args.result, 'evidence': dest.name,
                                 'sha256': digest(dest), 'note': args.note, 'kind': 'explicit-observation'}
    save(path, data)


def finish(args):
    # Discard is allowed after source drift, but never labels the round accepted.
    path = RECEIPTS / args.round / 'receipt.json'
    data = json.loads(path.read_text())
    if data['status'] != 'open':
        raise SystemExit('Round already finished')
    if not args.discard:
        load(args.round)
        if data.get('source_changed_during_build') or data.get('build', {}).get('result') != 'passed' or any(
                data['layers'].get(layer, {}).get('result') != 'passed' for layer in data['required']):
            raise SystemExit('Required acceptance layers remain pending/failed; retain artifacts or use --discard')
        for result in data['layers'].values():
            if digest(path.parent / result['evidence']) != result['sha256']:
                raise SystemExit('Evidence changed; refusing acceptance')
    runtime = RUNTIME / args.round
    if json.loads((runtime / '.owned.json').read_text()) != {'round': args.round, 'root': str(ROOT)}:
        raise SystemExit('Runtime ownership mismatch')
    artifacts = [Path(item['path']) for item in data['artifacts']]
    assert_unused([runtime] + artifacts)
    for item in data['artifacts']:
        artifact = Path(item['path'])
        if artifact.exists() and digest(artifact) != item['sha256']:
            raise SystemExit('Artifact was replaced by another build; cleanup refused')
    remove_owned(runtime, RUNTIME)
    for artifact in artifacts:
        remove_owned(artifact, CACHE)
    data['status'] = 'discarded' if args.discard else 'accepted'
    data['cleaned'] = True
    save(path, data)
    print(data['status'] + '; round binaries removed, public cache and evidence retained')


def reset_cache(args):
    if RUNTIME.exists() and any(RUNTIME.glob('*/.owned.json')):
        raise SystemExit('Finish/discard open rounds before resetting the public cache')
    marker = CACHE / '.tekesui-cache.json'
    identity = json.loads(marker.read_text())
    if identity.get('owner') != 'tekesui-component-review-v1' or identity.get('root') != str(ROOT):
        raise SystemExit('Unowned cache')
    assert_unused([CACHE])
    remove_owned(CACHE, ROOT / 'target')
    print('Public cache reset; next build regenerates current dependencies')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest='command', required=True)
    p = commands.add_parser('build'); p.add_argument('case', choices=CASES); p.add_argument('round', type=identifier)
    p.add_argument('--integration', action='store_true', help='Require the full Gallery layer for this round')
    p.set_defaults(action=build)
    p = commands.add_parser('check'); p.add_argument('round', type=identifier); p.add_argument('--gallery', action='store_true'); p.set_defaults(action=check)
    p = commands.add_parser('record'); p.add_argument('round', type=identifier); p.add_argument('layer', choices=('gpu', 'native', 'gold7', 'gallery'))
    p.add_argument('evidence', type=Path); p.add_argument('--result', choices=('passed', 'failed'), required=True)
    p.add_argument('--note', required=True, help='Observed scope, display, and untested boundaries'); p.set_defaults(action=record)
    p = commands.add_parser('finish'); p.add_argument('round', type=identifier); p.add_argument('--discard', action='store_true'); p.set_defaults(action=finish)
    p = commands.add_parser('cache-reset'); p.set_defaults(action=reset_cache)
    args = parser.parse_args()
    with build_lock():
        args.action(args)


if __name__ == '__main__':
    main()
