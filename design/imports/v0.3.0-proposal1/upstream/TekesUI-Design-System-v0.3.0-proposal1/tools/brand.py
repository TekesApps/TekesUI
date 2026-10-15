#!/usr/bin/env python3
"""Compile consumer brand seeds into full, four-theme token bundles.

This is a build-time reference compiler, not a Rust runtime or design approval.
No network, implicit brand defaults, component-local color math, or hidden repair
of consumer overrides. Generated receipts describe bytes, not authorization.
"""
from __future__ import annotations
import argparse
import copy
import hashlib
import json
from pathlib import Path
import sys
import jsonschema
from brand_color import contrast, delta_ok, parse_hex, tone, to_oklch
from validate import ROOT, THEMES, read, resolve, token_errors, contrast_results

VERSION = '1.0.0'
ALGORITHM = 'tekes-oklch-constrained-v1'
ROLE_NAMES = (
    'brand-seed', 'accent', 'accent-hover', 'accent-pressed',
    'accent-subtle-ground', 'accent-subtle-hover', 'accent-subtle-pressed',
    'accent-subtle-foreground', 'accent-outline',
    'action-primary-ground', 'action-primary-hover', 'action-primary-pressed',
    'action-primary-foreground', 'action-primary-focus-outline',
)
MANAGED = {'p-'+name for name in ROLE_NAMES}


def bytes_json(value: object) -> bytes:
    return (json.dumps(value, ensure_ascii=False, indent=2, allow_nan=False)+'\n').encode()


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def canonical(value: object) -> bytes:
    return json.dumps(value, ensure_ascii=False, sort_keys=True,
                      separators=(',', ':'), allow_nan=False).encode()


def load_external(path: Path) -> dict:
    # Use the same strict parser as all design inputs (including duplicate keys).
    from validate import _object_pairs, _invalid_constant
    return json.loads(path.read_text(encoding='utf-8'),
                      object_pairs_hook=_object_pairs, parse_constant=_invalid_constant)


def config_errors(config: dict) -> list[str]:
    return [f'BRAND_CONFIG_SCHEMA: {list(e.path)}: {e.message}' for e in
            jsonschema.Draft202012Validator(read('design/brand/brand.schema.json')).iter_errors(config)]


def policy_errors(policy: dict) -> list[str]:
    errors = [f'BRAND_POLICY_SCHEMA: {list(e.path)}: {e.message}' for e in
              jsonschema.Draft202012Validator(read('design/brand/policy.schema.json')).iter_errors(policy)]
    if errors: return errors
    if policy['algorithm'] != ALGORITHM: errors.append('BRAND_ALGORITHM_VERSION')
    if not 0 <= policy['subtle_chroma'] <= policy['max_chroma'] <= .4:
        errors.append('BRAND_CHROMA_RANGE')
    if not 0 <= policy['subtle_chroma_factor'] <= 1: errors.append('BRAND_CHROMA_FACTOR')
    if not .001 <= policy['minimum_state_delta_ok'] <= .1: errors.append('BRAND_STATE_DELTA_RANGE')
    for theme, targets in policy['targets'].items():
        if not all(0 < x < 1 for x in targets['subtle']): errors.append('BRAND_SUBTLE_RANGE')
        direction = 1 if theme.startswith('dark') else -1
        if targets['action_step']*direction <= 0 or targets['link_step']*direction <= 0:
            errors.append('BRAND_STATE_DIRECTION: '+theme)
        values = targets['subtle']
        if any((y-x)*direction <= 0 for x,y in zip(values, values[1:])):
            errors.append('BRAND_SUBTLE_ORDER: '+theme)
    return errors


def pick(candidates: list[tuple[float, str]], target: float,
         constraints: list[tuple[str, float]], lower: float = 0., upper: float = 1.) -> tuple[float, str]:
    allowed = [(l, value) for l,value in candidates if lower <= l <= upper and
               all(contrast(value, bg) >= threshold for bg,threshold in constraints)]
    if not allowed: raise ValueError('BRAND_NO_FEASIBLE_COLOR: required color pair has no valid solution')
    # Stable tie-break: lower L, then serialized hex. No random optimization.
    return min(allowed, key=lambda item: (abs(item[0]-target), item[0], item[1]))


def theme_roles(base: dict, seed: str, policy: dict, theme: str) -> dict[str, str]:
    _, c, h = to_oklch(seed)
    c = min(c, policy['max_chroma'])
    steps = policy['tone_steps']; iterations = policy['gamut_iterations']
    candidates = [(i/steps, tone(i/steps, c, h, iterations)) for i in range(steps+1)]
    target = policy['targets'][theme]
    dark = theme.startswith('dark')
    text_min = policy['thresholds']['text_ic' if theme.endswith('-ic') else 'text']
    boundary_min = policy['thresholds']['non_text']
    background = lambda name: resolve(base, 'color', name, theme)
    text_bgs = [background(name) for name in policy['text_backgrounds']]
    control_bgs = [background(name) for name in policy['control_backgrounds']]
    values = {'brand-seed': seed.upper()}
    # This proposal uses dark fills on light surfaces / light fills on dark.
    # On-solid is a paired neutral role, not a hue-bearing brand color.
    on_solid = '#000000' if dark else '#FFFFFF'
    action_constraints = [(x,boundary_min) for x in control_bgs]+[(on_solid,text_min)]
    l, solid = pick(candidates, target['action'], action_constraints)
    values['action-primary-ground'] = solid
    previous_l = l
    for name in ['action-primary-hover', 'action-primary-pressed']:
        step = target['action_step']
        previous_l, value = pick(candidates, previous_l+step, action_constraints,
                                lower=previous_l+.02 if dark else 0.,
                                upper=1. if dark else previous_l-.02)
        values[name] = value
    values['action-primary-foreground'] = on_solid
    values['action-primary-focus-outline'] = on_solid
    link_constraints = [(x,text_min) for x in text_bgs]
    previous_l, values['accent'] = pick(candidates, target['link'], link_constraints)
    for name in ['accent-hover', 'accent-pressed']:
        step = target['link_step']
        previous_l, value = pick(candidates, previous_l+step, link_constraints,
                                lower=previous_l+.018 if dark else 0.,
                                upper=1. if dark else previous_l-.018)
        values[name] = value
    subtle_c = min(c*policy['subtle_chroma_factor'], policy['subtle_chroma'])
    subtle_values = [tone(x,subtle_c,h,iterations) for x in target['subtle']]
    for name,value in zip(['accent-subtle-ground','accent-subtle-hover','accent-subtle-pressed'],subtle_values):
        values[name] = value
    _, values['accent-subtle-foreground'] = pick(candidates,target['subtle_text'],[(x,text_min) for x in subtle_values])
    marker_bgs = control_bgs+[background(x) for x in policy['marker_backgrounds']]+subtle_values
    _, values['accent-outline'] = pick(candidates,target['outline'],[(x,boundary_min) for x in marker_bgs])
    return values


def compile_config(config: dict, *, base: dict | None = None,
                   policy: dict | None = None, metadata: dict | None = None) -> tuple[dict, dict]:
    errors = config_errors(config)
    if errors: raise ValueError('\n'.join(errors))
    policy = read('design/brand/policy.json') if policy is None else policy
    errors = policy_errors(policy)
    if errors: raise ValueError('\n'.join(errors))
    base = read('design/tokens/base.tokens.json') if base is None else base
    metadata = read('design/tokens/token-metadata.json') if metadata is None else metadata
    if any(x['name'] in MANAGED for x in base['color']['tokens']):
        raise ValueError('BRAND_LITERAL_IN_BASE: generated brand primitives must not be authored in base')
    seed = config['seed_srgb'].upper(); parse_hex(seed)
    roles = {theme:theme_roles(base,seed,policy,theme) for theme in THEMES}
    for role, override in config.get('overrides',{}).items():
        # Strict schema ensures every override includes all four themes and a reason.
        for theme in THEMES: roles[theme][role] = override['values'][theme].upper()
    tokens = copy.deepcopy(base)
    for role in ROLE_NAMES:
        tokens['color']['tokens'].append({'name':'p-'+role,
            'value':{theme:roles[theme][role] for theme in THEMES},
            'usage':'由消费者 BrandConfig 和版本化 policy 生成；禁止手改：'+role})
    errors = token_errors(tokens,metadata)
    affected = dependency_names(tokens)
    forbidden_prefixes = ('status-', 'warning', 'diff-', 'syntax-', 'data-', 'c-code-', 'c-chart-', 'surface-')
    forbidden_names = {'border-focused', 'c-input-focus-outline', 'text-selection',
                       'selected-text', 'row-selection', 'row-selection-hover',
                       'button-hover', 'button-pressed', 'primary-text', 'secondary-text',
                       'tertiary-text', 'disabled-text', 'segmented-track', 'segmented-thumb',
                       'user-message-surface', 'hover-wash', 'border', 'separator'}
    for name in affected:
        if name.startswith(forbidden_prefixes) or name in forbidden_names:
            errors.append('BRAND_INDEPENDENT_ROLE_LEAK: '+name)
    for role,override in config.get('overrides',{}).items():
        if len(override['reason'].strip()) < 8: errors.append('BRAND_OVERRIDE_REASON: '+role)
    if not errors:
        errors += [f"BRAND_CONTRAST: {x['theme']} {x['foreground']}/{x['background']}"
                   for x in contrast_results(tokens) if x['status']!='pass']
        for theme in THEMES:
            for name in ['action-primary-foreground', 'action-primary-focus-outline']:
                if roles[theme][name] not in ['#000000', '#FFFFFF']:
                    errors.append('BRAND_ON_SOLID_NONNEUTRAL: '+theme+'/'+name)
            for states in [('action-primary-ground','action-primary-hover','action-primary-pressed'),
                           ('accent','accent-hover','accent-pressed'),
                           ('accent-subtle-ground','accent-subtle-hover','accent-subtle-pressed')]:
                for a,b in zip(states,states[1:]):
                    direction = 1 if theme.startswith('dark') else -1
                    if (to_oklch(roles[theme][b])[0]-to_oklch(roles[theme][a])[0])*direction <= 0:
                        errors.append('BRAND_STATE_ORDER: '+theme+'/'+a+'/'+b)
                    if delta_ok(roles[theme][a],roles[theme][b]) < policy['minimum_state_delta_ok']:
                        errors.append('BRAND_STATE_COLLAPSE: '+theme+'/'+a+'/'+b)
            # Numerical difference is a static guard, not proof of human discrimination.
    if errors: raise ValueError('\n'.join(errors))
    _, c, h = to_oklch(seed)
    ramp = {f'tone-{round(l*1000):03d}':tone(l,min(c,policy['max_chroma']),h,policy['gamut_iterations'])
            for l in [.05,.10,.20,.30,.40,.50,.60,.70,.80,.90,.95]}
    palette = {'schema_version':'1.0','brand_id':config['brand_id'],'brand_version':config['version'],
               'seed_srgb':seed,'algorithm':ALGORITHM,'policy_version':policy['version'],
               'scope':'role candidates; not native rendering or aesthetic approval',
               'ramp_scope':'inspection only; consumers must use semantic tokens, not tone stops',
               'ramp':ramp,'roles':roles,'overrides':config.get('overrides',{}),
               'native_evidence':'unverified'}
    return tokens,palette


def dependency_names(tokens: dict) -> set[str]:
    by = {x['name']:x['value'] for x in tokens['color']['tokens']}
    affected = set(MANAGED)
    changed = True
    while changed:
        changed = False
        for name,value in by.items():
            if name not in affected and isinstance(value,str) and value[1:-1] in affected:
                affected.add(name);changed=True
    return affected


def compile_artifacts(path: Path) -> tuple[dict[str, bytes], dict]:
    config = load_external(path)
    tokens,palette = compile_config(config)
    payloads = {'tokens.json':bytes_json(tokens),'palette.json':bytes_json(palette)}
    source_files = ['design/tokens/base.tokens.json','design/tokens/token-metadata.json',
                    'design/tokens/tokens.schema.json','design/brand/policy.json',
                    'design/brand/brand.schema.json','design/brand/policy.schema.json',
                    'design/quality/contrast-pairs.json','tools/brand.py',
                    'tools/brand_color.py','tools/validate.py']
    inputs = {p:digest((ROOT/p).read_bytes()) for p in source_files}
    receipt = {'receipt_schema_version':'1.0','design_version':'0.3.0-proposal.1',
               'brand_id':config['brand_id'],'brand_version':config['version'],
               'generator_version':VERSION,'algorithm':ALGORITHM,
               'policy_version':read('design/brand/policy.json')['version'],
               'brand_config_sha256':digest(path.read_bytes()),
               'normalized_config_sha256':digest(canonical(config)),
               'inputs_sha256':inputs,'outputs_sha256':{k:digest(v) for k,v in payloads.items()},
               'source_artifact_revision':None,'source_export_sha256':None,
               'approved':False,'native_evidence':'unverified',
               'integrity_is_not_approval':True}
    receipt['theme_contract_sha256'] = digest(canonical({k:v for k,v in receipt.items()
                                                        if k not in ['approved','native_evidence']}))
    payloads['receipt.json'] = bytes_json(receipt)
    return payloads, {'brand_id':config['brand_id'],'brand_version':config['version'],
                      'tokens':tokens,'palette':palette,'receipt':receipt,
                      'checks':contrast_results(tokens)}


def expected_artifacts() -> tuple[dict[str, bytes], list[dict]]:
    gallery = read('design/brand/gallery.json')
    if gallery.get('schema_version')!='1.0' or not gallery.get('profiles'):
        raise ValueError('BRAND_GALLERY_SCHEMA')
    artifacts = {};compiled=[];ids=set()
    for relative in gallery['profiles']:
        path = (ROOT/relative).resolve()
        if ROOT not in path.parents or path.suffix!='.json': raise ValueError('BRAND_PROFILE_PATH')
        payloads,info = compile_artifacts(path)
        bid = info['brand_id']
        if bid in ids: raise ValueError('DUPLICATE_BRAND_ID: '+bid)
        ids.add(bid);compiled.append(info)
        for name,value in payloads.items(): artifacts[f'design/brand/generated/{bid}/{name}']=value
        if bid==gallery['reference_brand']: artifacts['design/tokens/tokens.json']=payloads['tokens.json']
    if gallery['reference_brand'] not in ids: raise ValueError('BRAND_REFERENCE_PROFILE_MISSING')
    expected = set(dependency_names(compiled[0]['tokens']))
    reference = compiled[0]['tokens']
    for info in compiled[1:]:
        candidate = info['tokens']
        if dependency_names(candidate)!=expected: raise ValueError('BRAND_DEPENDENCY_GRAPH_DIFFERS')
        for family in reference:
            if family != 'color' and candidate[family]!=reference[family]:
                raise ValueError('BRAND_NON_COLOR_LEAK: '+family)
        for record in reference['color']['tokens']:
            name=record['name']
            if name not in expected and any(resolve(reference,'color',name,theme)!=resolve(candidate,'color',name,theme) for theme in THEMES):
                raise ValueError('BRAND_INDEPENDENT_ROLE_LEAK: '+name)
    return artifacts,compiled


def freshness_errors(artifacts: dict[str, bytes]) -> list[str]:
    errors=[]
    for path,expected in artifacts.items():
        p=ROOT/path
        if not p.is_file() or p.read_bytes()!=expected: errors.append('STALE_BRAND_OUTPUT: '+path)
    root=ROOT/'design/brand/generated'
    actual={str(p.relative_to(ROOT)) for p in root.rglob('*') if p.is_file()} if root.exists() else set()
    required={p for p in artifacts if p.startswith('design/brand/generated/')}
    if actual!=required: errors.append('ORPHAN_OR_MISSING_BRAND_OUTPUT')
    return errors


def write_artifacts(artifacts: dict[str,bytes]) -> None:
    # Compilation validates all profiles first. Receipt is the last commit marker
    # in each profile. Production must publish an immutable dir and swap its pointer.
    for relative,data in artifacts.items():
        path=ROOT/relative;path.parent.mkdir(parents=True,exist_ok=True)
        tmp=path.with_suffix(path.suffix+'.tmp');tmp.write_bytes(data);tmp.replace(path)


def validate_all(write_report: bool = False) -> tuple[list[str], dict]:
    artifacts,compiled=expected_artifacts()
    errors=freshness_errors(artifacts)
    rows=[]
    for info in compiled:
        checks=info['checks']
        rows.append({'brand_id':info['brand_id'],'seed_srgb':info['palette']['seed_srgb'],
                     'themes':THEMES,'contrast_checks':len(checks),
                     'contrast_failures':sum(x['status']!='pass' for x in checks),
                     'theme_contract_sha256':info['receipt']['theme_contract_sha256'],
                     'native_evidence':'unverified'})
    report={'scope':'brand-build-static','status':'fail' if errors else 'pass','errors':errors,
            'profiles':rows,'contrast_checks':sum(x['contrast_checks'] for x in rows),
            'brand_dependent_color_token_count':len(dependency_names(compiled[0]['tokens'])),
            'independent_roles':'identical across all example brand bundles',
            'renderer_validation':'unverified','consumer_acceptance':'unverified'}
    if write_report:
        (ROOT/'reports').mkdir(exist_ok=True)
        (ROOT/'reports/brand-validation.json').write_bytes(bytes_json(report))
        (ROOT/'reports/brand-contrast-results.json').write_bytes(bytes_json({x['brand_id']:x['checks'] for x in compiled}))
        (ROOT/'reports/brand-token-dependencies.json').write_bytes(bytes_json({'brand_dependent':sorted(dependency_names(compiled[0]['tokens'])),'status':'static-graph-only'}))
    return errors,report


def main() -> int:
    parser=argparse.ArgumentParser(description=__doc__)
    group=parser.add_mutually_exclusive_group(required=True)
    group.add_argument('--all',action='store_true',help='compile the explicit Gallery profiles')
    group.add_argument('--brand',type=Path,help='consumer BrandConfig JSON path')
    parser.add_argument('--out',type=Path,help='new output directory for --brand')
    parser.add_argument('--check',action='store_true',help='verify freshness without writing')
    args=parser.parse_args()
    if args.all:
        if args.out: parser.error('--out is only valid with --brand')
        artifacts,_=expected_artifacts()
        if args.check:
            errors=freshness_errors(artifacts)
            print('\n'.join(errors) if errors else 'All explicit brand outputs are current; native UI unverified.')
            return int(bool(errors))
        write_artifacts(artifacts)
        print('Generated all explicit brand profiles and the neutral reference tokens; no design approved.')
    else:
        if not args.out: parser.error('--brand requires --out')
        payloads,info=compile_artifacts(args.brand)
        if args.check:
            ok=all((args.out/name).is_file() and (args.out/name).read_bytes()==data for name,data in payloads.items())
            print('Current' if ok else 'STALE_BRAND_OUTPUT');return int(not ok)
        if args.out.exists() and any(args.out.iterdir()):
            raise ValueError('OUTPUT_NOT_EMPTY: use a new versioned directory; no in-place consumer patching')
        args.out.mkdir(parents=True,exist_ok=True)
        for name,data in payloads.items(): (args.out/name).write_bytes(data)
        print(f"Generated {info['brand_id']} × four themes to {args.out}; native UI unverified.")
    return 0


if __name__=='__main__':
    try: sys.exit(main())
    except (OSError,ValueError,KeyError,TypeError,jsonschema.SchemaError) as error:
        print(f'BRAND_BUILD_ERROR: {error}',file=sys.stderr);sys.exit(2)
