#!/usr/bin/env python3
"""Audit color relationships against the live compiler, not reference HEX values.

Read-only unless --output is supplied. The default also checks the active saved
report against current inputs; --output explicitly produces its replacement.
Does not generate tokens or alter imports.
Run with target/design-tools-venv/bin/python (jsonschema is required).
"""
from __future__ import annotations

import argparse
import copy
import hashlib
import importlib
import json
import re
from pathlib import Path
import subprocess
import sys
import tempfile

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[1]
UPSTREAM = Path('design/imports/v0.3.0-proposal1/upstream/TekesUI-Design-System-v0.3.0-proposal1')
sys.path.insert(0, str(ROOT / UPSTREAM / 'tools'))
brand = importlib.import_module('brand')
validate = importlib.import_module('validate')
color = importlib.import_module('brand_color')


def read(path):
    return json.loads((ROOT / path).read_text())


def source_revision(root=ROOT):
    """Optional provenance; input hashes remain the audit authority.

    Do not inherit a parent checkout's HEAD for an exported source directory.
    A local initialized repository may also intentionally have no commit yet.
    """
    if not (root / '.git').exists():
        return None
    result = subprocess.run(
        ['git', '-C', str(root), 'rev-parse', '--verify', 'HEAD'],
        stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True,
    )
    if result.returncode:
        return None
    revision = result.stdout.strip()
    if not re.fullmatch(r'[0-9a-f]{40}|[0-9a-f]{64}', revision):
        raise ValueError('invalid Git revision in audit provenance')
    return revision


def relationship_errors(tokens, policy):
    errors = []
    for theme in validate.THEMES:
        get = lambda name: validate.resolve(tokens, 'color', name, theme)
        # Light high layers may coincide. Their distinction needs geometry.
        surfaces = [validate.luminance(validate.rgb(get(f'surface-{i}'))) for i in range(4)]
        if any(a > b for a, b in zip(surfaces, surfaces[1:])):
            errors.append(f'{theme}: surface luminance order')
        direction = 1 if theme.startswith('dark') else -1
        for states in [('action-primary-ground', 'action-primary-hover', 'action-primary-pressed'),
                       ('accent', 'accent-hover', 'accent-pressed'),
                       ('accent-subtle-ground', 'accent-subtle-hover', 'accent-subtle-pressed')]:
            for a, b in zip(states, states[1:]):
                if (color.to_oklch(get(b))[0] - color.to_oklch(get(a))[0]) * direction <= 0:
                    errors.append(f'{theme}: state direction {a}/{b}')
                if color.delta_ok(get(a), get(b)) < policy['minimum_state_delta_ok']:
                    errors.append(f'{theme}: collapsed state {a}/{b}')
        expected_on_solid = '#000000' if theme.startswith('dark') else '#FFFFFF'
        if get('action-primary-foreground') != expected_on_solid:
            errors.append(f'{theme}: paired neutral foreground')
    return errors


def independent_errors(reference, candidate):
    # Everything outside the compiler's transitive brand dependency graph must
    # stay equal, not merely a short hand-picked list of neutral colors.
    managed = brand.dependency_names(reference)
    errors = []
    for item in reference['color']['tokens']:
        name = item['name']
        if name in managed:
            continue
        for theme in validate.THEMES:
            if validate.resolve(reference, 'color', name, theme) != validate.resolve(candidate, 'color', name, theme):
                errors.append(f'{theme}: brand-independent role changed: {name}')
    return errors


def local_contrast_results(tokens, pairs):
    results = []
    for pair in pairs:
        for theme in validate.THEMES:
            foreground = validate.rgb(validate.resolve(tokens, 'color', pair['foreground'], theme))
            background = validate.rgb(validate.resolve(tokens, 'color', pair['background'], theme))
            ratio = validate.contrast(foreground, background)
            minimum = pair['ic' if theme.endswith('-ic') else 'normal']
            results.append(dict(theme=theme, foreground=pair['foreground'],
                                background=pair['background'], overlay=None,
                                ratio=ratio, minimum=minimum,
                                status='pass' if ratio >= minimum else 'fail'))
    return results


def audit():
    contract = read('design/behavior/reference-alignment.json')
    base = read('design/source/base.tokens.json')
    policy = read('design/brand/policy.json')
    metadata = read('design/source/token-metadata.json')
    layout = read('design/source/layout.json')
    active = read('design/tokens.json')['proposal']['tokens']
    gallery = read('design/gallery/brands.json')
    registry = read('design/brand/gallery.json')
    components = read(UPSTREAM / 'design/registry/components.json')['components']
    recipes = read(UPSTREAM / 'design/registry/recipes.json')['recipes']
    core_ids = {item['id'] for item in components}
    local_ids = {item['id'] for item in contract['local_recipes']}
    known = {item['id'] for item in components + recipes} | local_ids
    errors = []
    inputs = [Path('design/behavior/reference-alignment.json'), Path('design/source/base.tokens.json'),
              Path('design/source/token-metadata.json'), Path('design/source/layout.json'),
              Path('design/brand/policy.json'),
              Path('design/brand/config.json'), Path('design/tokens.json'),
              Path('design/gallery/brands.json'), Path('scripts/check-reference-alignment.py'),
              Path('docs/design/reference-alignment-20261016.md'),
              UPSTREAM / 'tools/brand.py', UPSTREAM / 'tools/brand_color.py',
              UPSTREAM / 'tools/validate.py', UPSTREAM / 'design/quality/contrast-pairs.json',
              UPSTREAM / 'design/registry/components.json', UPSTREAM / 'design/registry/recipes.json']
    if len(contract['components']) != 15 or len({c['id'] for c in contract['components']}) != 15:
        errors.append('reference component inventory must contain 15 unique groups')
    for recipe in contract['local_recipes']:
        if not set(recipe['component_ids']) <= core_ids:
            errors.append(f"unknown primitive in local recipe: {recipe['id']}")
    for component in contract['components']:
        if not set(component['design_ids']) <= known:
            errors.append(f"unknown component mapping: {component['id']}")
        if component['native_acceptance'] != 'unverified':
            errors.append(f"native acceptance needs separate evidence: {component['id']}")
    attachment = Path(contract['source']['path'])
    provenance = 'unverified: original local attachment is unavailable on this host'
    if attachment.is_file():
        provenance = 'pass' if hashlib.sha256(attachment.read_bytes()).hexdigest() == contract['source']['sha256'] else 'fail'
        if provenance == 'fail':
            errors.append('attachment SHA256 differs from the reviewed document')
    profiles = []
    reference = None
    all_pairs = []
    inputs.append(Path('design/brand/gallery.json'))
    registered = sorted(registry['presets'], key=lambda p: p['runtime_index'])
    for preset in registered:
        name = preset['id']
        config_path = Path(preset['config'])
        inputs.append(config_path)
        tokens, _ = brand.compile_config(read(config_path), base=base, policy=policy, metadata=metadata)
        expected_colors = next(p['color'] for p in gallery['presets'] if p['id'] == name)
        if name == registry['reference_brand'] and tokens['color'] != active['color']:
            errors.append(f'{name}: consumer reference differs from active tokens')
        if tokens['color'] != expected_colors:
            errors.append(f'{name}: compiled colors differ from live projection')
        errors.extend(f'{name}: {e}' for e in relationship_errors(tokens, policy))
        if reference is None:
            reference = tokens
        else:
            errors.extend(f'{name}: {e}' for e in independent_errors(reference, tokens))
        for role in list(contract['color_roles'].values()) + [r for c in contract['components'] for r in c['color_roles']]:
            for theme in validate.THEMES:
                validate.resolve(tokens, 'color', role, theme)
        pairs = validate.contrast_results(tokens)
        pairs.extend(local_contrast_results(tokens, contract.get('local_contrast_pairs', [])))
        all_pairs.extend(dict(profile=name, **p) for p in pairs)
        errors.extend(f"{name}: contrast {p['theme']} {p['foreground']}/{p['background']}" for p in pairs if p['status'] != 'pass')
        profiles.append({'id': name, 'themes': 4, 'declared_contrast_checks': len(pairs),
                         'contrast_failures': sum(p['status'] != 'pass' for p in pairs)})
    managed_roles = brand.dependency_names(reference)
    revision = source_revision()
    return {
        'format': 'tekesui-reference-alignment-report/1',
        'date': contract['date'],
        'source_commit': revision,
        'source_state': ('working-tree; hashes identify audited inputs, not a clean commit' if revision
                         else 'source snapshot; Git HEAD unavailable; input hashes identify audited bytes'),
        'inputs_sha256': {str(p): hashlib.sha256((ROOT / p).read_bytes()).hexdigest() for p in sorted(set(inputs))},
        'attachment_provenance': provenance,
        'static_result': 'fail' if errors else 'pass',
        'profiles': profiles,
        'brand_independent_role_count': sum(
            item['name'] not in managed_roles
            for item in reference['color']['tokens']
        ),
        'components': contract['components'],
        'elevation_contract': {
            'source': 'design/source/layout.json',
            'shadow_policy': layout['shadow'],
            'definition': layout['elevation'],
            'scope': 'Declared design policy; not renderer, pixel or native acceptance',
        },
        'differences_requiring_review': ['brand seed versus fixed Clay', 'theme-paired on-solid foreground',
                                   'neutral focus', 'opaque constrained subtle tones versus alpha tint',
                                   'semantic elevation differs from the reference; current roles are defined in design/source/layout.json', 'no press scaling', 'typography and composer sizes'],
        'native_result': 'unverified',
        'limitations': ['Declared contrast pairs only; arbitrary nesting is not covered',
                       'No pixel capture, real IME, VoiceOver, provider or performance acceptance',
                       'Planned recipes and proposed density profiles remain unimplemented'],
        'errors': errors,
        'contrast_checks': all_pairs,
    }


def saved_report_errors(current, saved):
    errors = []
    expected = current['inputs_sha256']
    actual = saved.get('inputs_sha256', {})
    for path in sorted(set(expected) | set(actual)):
        if expected.get(path) != actual.get(path):
            errors.append('active reference report has stale input: ' + path)
    # Compare consumer policy, not Git/host attachment provenance or floating
    # samples. A new commit or unavailable local attachment does not stale inputs.
    for field in ['format', 'date', 'static_result',
                  'profiles', 'brand_independent_role_count', 'components',
                  'elevation_contract', 'differences_requiring_review',
                  'native_result', 'limitations', 'errors']:
        if saved.get(field) != current[field]:
            errors.append('active reference report differs from current policy: ' + field)
    return errors


def self_test():
    # Mutate relationships, not reference HEX constants: order reversal, vanished
    # state distinction, and brand leakage must all be rejected.
    # Actual exported and unborn repositories must not need a fabricated commit.
    assert source_revision(ROOT / 'scripts') is None
    with tempfile.TemporaryDirectory(prefix='tekesui-reference-provenance-') as directory:
        root = Path(directory)
        assert source_revision(root) is None
        subprocess.run(['git', 'init', '--quiet', str(root)], check=True)
        assert source_revision(root) is None
    tokens = read('design/tokens.json')['proposal']['tokens']
    policy = read('design/brand/policy.json')
    assert not relationship_errors(tokens, policy)
    def changed(name, value):
        candidate = copy.deepcopy(tokens)
        next(t for t in candidate['color']['tokens'] if t['name'] == name)['value'] = value
        return candidate
    assert any('surface luminance order' in e for e in relationship_errors(changed('surface-0', '#FFFFFF'), policy))
    assert any('collapsed state' in e for e in relationship_errors(changed('accent-hover', '{accent}'), policy))
    assert independent_errors(tokens, changed('primary-text', '#FF0000'))
    local_pairs = read('design/behavior/reference-alignment.json')['local_contrast_pairs']
    assert all(p['status'] == 'pass' for p in local_contrast_results(tokens, local_pairs))
    assert any(p['status'] == 'fail' for p in local_contrast_results(
        changed('c-status-badge-foreground', '#151515'), local_pairs))
    current = audit()
    assert not saved_report_errors(current, copy.deepcopy(current))
    stale = copy.deepcopy(current)
    stale['inputs_sha256']['design/source/layout.json'] = '0' * 64
    assert any('design/source/layout.json' in e for e in saved_report_errors(current, stale))
    old_policy = copy.deepcopy(current)
    old_policy['elevation_contract']['shadow_policy'] = 'none'
    assert any('elevation_contract' in e for e in saved_report_errors(current, old_policy))
    false_claim = copy.deepcopy(current)
    false_claim['native_result'] = 'pass'
    assert any('native_result' in e for e in saved_report_errors(current, false_claim))
    print('PASS: baseline plus surface-order, state-collapse, brand-leakage and status-label contrast negative cases')
    print('PASS: active-report input, elevation policy and native-claim negative cases')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, help='explicitly save a numerical report')
    parser.add_argument('--self-test', action='store_true')
    args = parser.parse_args()
    if args.self_test:
        self_test()
        return 0
    report = audit()
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(json.dumps(report, ensure_ascii=False, indent=2, allow_nan=False) + '\n')
    else:
        reference = read('design/behavior/reference-alignment.json')['acceptance']['report']['$ref']
        path = ROOT / 'design/behavior' / reference
        if not path.is_file():
            report['errors'].append('active reference report is missing: ' + reference)
        else:
            report['errors'].extend(saved_report_errors(report, json.loads(path.read_text())))
        if report['errors']:
            report['static_result'] = 'fail'
    print(f"{report['static_result'].upper()}: 15 component mappings; {len(report['profiles'])} brand profiles × 4 themes; {len(report['contrast_checks'])} declared contrast checks. Native acceptance: unverified.")
    for error in report['errors']:
        print('FAIL:', error, file=sys.stderr)
    return int(bool(report['errors']))


if __name__ == '__main__':
    raise SystemExit(main())
