#!/usr/bin/env python3
"""Extend existing brand compiler with complete Surface/use/state theme instances.
No runtime Python, source recoloring, consumer state tables or baseline mutation.
"""
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import sys
sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('theme_merge', ROOT / 'scripts/merge-design-system.py')
merge = importlib.util.module_from_spec(spec)
spec.loader.exec_module(merge)
MODES = ['light', 'dark', 'light-ic', 'dark-ic']
ARTIFACT_LOCK = 'design/theme/artifact.lock.json'
ARTIFACT_LOCK_SCHEMA = 'design/theme/artifact-lock.schema.json'
CHANNELS = ['background', 'text', 'icon', 'boundary', 'focus', 'marker', 'helper']
STATES = ['rest', 'hover', 'pressed', 'disabled', 'selected', 'selected-hover', 'selected-pressed', 'selected-disabled']
USES = {'content': ['rest', 'disabled'], 'action-quiet': STATES, 'action-primary': STATES, 'input': STATES, 'selection': STATES, 'link': STATES[:4], 'text-selection':['rest','disabled'], 'content-selectable':['rest','disabled']}
for _use in ['input-readonly','input-error','input-readonly-error','action-quiet-error','action-primary-error','selection-error','check-unchecked','check-checked','check-mixed','check-unchecked-error','check-checked-error','check-mixed-error']:
    USES[_use] = STATES
for _use in ['control-label','toggle-track','toggle-track-error','toggle-thumb']:
    USES[_use] = STATES
for _use in ['range-track','range-thumb']:
    USES[_use] = STATES[:4]
USES['surface-frame'] = ['rest','disabled']
USES['progress-track'] = ['rest']
USES['data-plot'] = ['rest']
USES['data-category-label'] = ['rest']
STATUS_CONTENT_USES = {'content-status-' + tone for tone in ['info','running','succeeded','failed']}
for _use in STATUS_CONTENT_USES: USES[_use] = ['rest','disabled']
BADGE_USES = {'status-badge-' + tone for tone in ['info','running','succeeded','failed']}
PASSIVE_USES = BADGE_USES | {'skeleton'}
for _use in PASSIVE_USES: USES[_use] = ['rest']
NOTICE_MARKERS = {'notice-marker-info': 'status-info', 'notice-marker-success': 'status-succeeded', 'notice-marker-error': 'status-failed', 'notice-marker-toast-warning': 'status-failed', 'notice-marker-banner-warning': 'status-running'}
for _use in NOTICE_MARKERS: USES[_use] = ['rest']
FOCUS_FREE_USES = set(NOTICE_MARKERS) | PASSIVE_USES | STATUS_CONTENT_USES | {'content','text-selection','control-label','toggle-thumb','range-track','surface-frame','progress-track','data-category-label'}


def validate_use_channels(usage, state, pair):
    if usage == 'data-plot' and (pair['background'] != '$surface' or any(pair[k] is None for k in ['boundary','focus','marker']) or pair['helper'] is not None):
        raise ValueError('invalid data plot anatomy')
    if usage == 'data-category-label' and (pair['background'] != '$surface' or any(pair[k] is not None for k in ['boundary','focus','marker','helper'])):
        raise ValueError('invalid data category anatomy')
    if usage in NOTICE_MARKERS:
        expected = dict(background='$surface',text='primary-text',icon='primary-text',boundary=None,focus=None,marker=NOTICE_MARKERS[usage],helper=None)
        if state != 'rest' or pair != expected:
            raise ValueError('invalid notice marker packet binding: ' + usage + '/' + state)
    if usage in PASSIVE_USES:
        expected = dict(background=('status-' + usage.removeprefix('status-badge-')) if usage in BADGE_USES else 'surface-2',
                        text='c-status-badge-foreground' if usage in BADGE_USES else 'primary-text',
                        icon='c-status-badge-foreground' if usage in BADGE_USES else 'primary-text',
                        boundary=None if usage in BADGE_USES else 'border', focus=None, marker=None, helper=None)
        if state != 'rest' or pair != expected:
            raise ValueError('invalid passive status packet binding: ' + usage + '/' + state)
    if usage == 'link' and any(pair[k] is not None for k in ['marker','helper']):
        raise ValueError('Link has no marker/helper anatomy: ' + state)
    if usage in STATUS_CONTENT_USES and (pair['background'] != '$surface' or any(pair[k] is not None for k in ['boundary','focus','marker','helper'])):
        raise ValueError('passive status content requires parent ground and forbids boundary/focus/marker/helper')
    if usage == 'progress-track' and (pair['boundary'] is None or pair['marker'] is None or any(pair[k] is not None for k in ['focus','helper'])):
        raise ValueError('progress track requires boundary/marker and forbids focus/helper')
    needs_focus = usage not in FOCUS_FREE_USES and 'disabled' not in state
    if needs_focus != (pair['focus'] is not None):
        raise ValueError(f'incomplete or inapplicable focus: {usage}/{state}')
    if (usage == 'range-track' or usage.startswith(('check-checked','check-mixed'))) and pair['marker'] is None:
        raise ValueError(f'missing required marker: {usage}/{state}')
    if usage.startswith('input') and pair['helper'] is None:
        raise ValueError(f'missing required helper: {usage}/{state}')


def expand_profiles(rules):
    expanded = {}
    def expand(name, stack=()):
        if name in stack: raise ValueError('cyclic shared profile: ' + name)
        if name not in rules['profiles']: raise ValueError('unknown profile: ' + name)
        if name in expanded: return expanded[name]
        profile = rules['profiles'][name]
        if 'extends' in profile:
            states = json.loads(json.dumps(expand(profile['extends'], stack + (name,))))
            for pair in states.values(): pair.update(profile.get('overrides', {}))
            for state, values in profile.get('state_overrides', {}).items():
                if state not in states: raise ValueError('override of inapplicable state: ' + state)
                states[state].update(values)
        else: states = profile
        expanded[name] = states
        return states
    for name in rules['profiles']: expand(name)
    return expanded


def validate(rules):
    import jsonschema
    jsonschema.validate(rules, merge.load(ROOT / 'design/theme/rules.schema.json'))
    if rules['rules_version'] != '1.10.0': raise ValueError('unsupported generation rules version')
    profiles = expand_profiles(rules)
    if not {'canvas','code','hint','panel','popover','sidebar','toolbar'} <= set(rules['surfaces']): raise ValueError('missing required Surface')
    for surface, definition in rules['surfaces'].items():
        if not surface or not surface.isascii() or any(c not in 'abcdefghijklmnopqrstuvwxyz0123456789-' for c in surface):
            raise ValueError('invalid Surface identity: ' + surface)
        if set(definition['usages']) != set(USES):
            raise ValueError('Surface must declare every supported use: ' + surface)
        for usage, profile in definition['usages'].items():
            if profile not in profiles: raise ValueError('unknown profile: ' + profile)
            if set(profiles[profile]) != set(USES[usage]):
                raise ValueError('incomplete or unsupported states: ' + surface + '/' + usage)
            if usage == 'range-track' and any(pair['boundary'] is None for state,pair in profiles[profile].items() if state != 'disabled'):
                raise ValueError('enabled range track requires a visible boundary')
            if usage == 'surface-frame' and any(pair['boundary'] is None or any(pair[k] is not None for k in ['focus','marker','helper']) for pair in profiles[profile].values()):
                raise ValueError('surface frame requires boundary and forbids focus/marker/helper')
            for state,pair in profiles[profile].items():validate_use_channels(usage,state,pair)
    for profile in profiles.values():
        if set(profile) != {'rest'} and not {'rest','disabled'} <= set(profile):
            raise ValueError('profile requires rest and disabled or exactly rest')
        for state, pair in profile.items():
            if set(pair) != set(CHANNELS): raise ValueError('incomplete channels')
            if any(pair[k] is None for k in ['background', 'text', 'icon']):
                raise ValueError('visible channels must be defined')
            if 'disabled' in state and pair['focus'] is not None:
                raise ValueError('disabled state must suppress focus')
    return profiles


def bindings(rules):
    profiles = validate(rules)
    rows = []
    for surface, definition in sorted(rules['surfaces'].items()):
        for usage, profile in sorted(definition['usages'].items()):
            for state, pair in sorted(profiles[profile].items()):
                roles = {k: definition['background'] if v == '$surface' else v for k, v in pair.items()}
                rows.append((surface, usage, state, roles))
    return rows


def rust_source(rules):
    text = '// Generated by scripts/generate-theme-system.py; do not edit.\n'
    text += 'const SURFACE_NAMES: &[&str] = &[' + ', '.join(map(json.dumps, sorted(rules['surfaces']))) + '];\n'
    text += 'fn surface_roles(surface: &str, usage: &str, state: &str) -> Option<[Option<&\'static str>; 7]> {\n    Some(match (surface, usage, state) {\n'
    for surface, usage, state, roles in bindings(rules):
        values = ', '.join('None' if roles[k] is None else 'Some(' + json.dumps(roles[k]) + ')' for k in CHANNELS)
        text += '        (' + ', '.join(map(json.dumps, [surface, usage, state])) + ') => [' + values + '],\n'
    return text + '        _ => return None,\n    })\n}\n'


def contrast(a, b):
    def lum(h):
        cs = [int(h[i:i+2], 16) / 255 for i in (1,3,5)]
        cs = [c / 12.92 if c <= .04045 else ((c+.055)/1.055)**2.4 for c in cs]
        return sum(c*w for c,w in zip(cs,[.2126,.7152,.0722]))
    a,b = sorted([lum(a),lum(b)])
    return (b+.05)/(a+.05)


def verify_solid_status_foregrounds(bundle):
    # Solid status label ink is a different use from passive status text on a Surface.
    # Follow the existing quality text thresholds: normal 4.5, enhanced 7.0.
    for mode in MODES:
        foreground = merge.token_value(bundle, 'color', 'c-status-badge-foreground', mode)
        minimum = 7.0 if mode.endswith('-ic') else 4.5
        for tone in ['info', 'running', 'succeeded', 'failed']:
            background = merge.token_value(bundle, 'color', 'status-' + tone, mode)
            if contrast(foreground, background) < minimum:
                raise ValueError(f'insufficient solid status foreground contrast: {mode}/{tone}')

def verify_profile_color_references(profiles, bundle):
    # Match the existing ordinary Rust build's complete shared-role vocabulary check.
    names = {token['name'] for token in bundle['color']['tokens']}
    for name in sorted(profiles):
        for state in sorted(profiles[name]):
            for channel in CHANNELS:
                role = profiles[name][state][channel]
                if role is not None and role != '$surface' and role not in names:
                    raise ValueError(f'unknown color role: {role} in shared profile {name}/{state}/{channel}')


def verify_notice_marker_contrast(usage, mode, colors, parent):
    if usage in NOTICE_MARKERS and (colors['marker'] is None or contrast(colors['marker'],parent) < 3.0):
        raise ValueError('insufficient notice marker-parent contrast: ' + mode + '/' + usage)


def verify_passive_status_contrast(usage, mode, colors, parent):
    if usage in BADGE_USES:
        minimum = 7.0 if mode.endswith('-ic') else 4.5
        for channel in ['text','icon']:
            if contrast(colors[channel],colors['background']) < minimum:
                raise ValueError('insufficient badge solid foreground contrast: ' + mode + '/' + usage + '/' + channel)
        if contrast(colors['background'],parent) < 3.0:
            raise ValueError('insufficient badge fill-parent contrast: ' + mode + '/' + usage)
    if usage == 'skeleton':
        for side, ground in [('fill',colors['background']),('parent',parent)]:
            if colors['boundary'] is None or contrast(colors['boundary'],ground) < 3.0:
                raise ValueError('insufficient skeleton boundary-' + side + ' contrast: ' + mode)



def data_plot_contract():
    contract = merge.load(ROOT / 'design/theme/data-plot-pairs.json')
    expected = {'V:P':('plot.text','parent'), 'V:S':('plot.text','selection.background'), 'C:P':('category.text','parent'), 'M:P':('plot.marker','parent'), 'M:S':('plot.marker','selection.background'), 'F:P':('plot.focus','parent'), 'F:S':('plot.focus','selection.background'), 'F:M':('plot.focus','plot.marker')}
    if set(contract) != {'format','pairs'} or contract['format'] != 'tekesui.data-plot-pairs/1': raise ValueError('invalid data plot pair contract')
    pairs=contract['pairs']
    if len(pairs)!=len(expected) or {p['id'] for p in pairs}!=set(expected): raise ValueError('incomplete data plot pairs')
    for p in pairs:
        text=p['id'][0] in 'VC'
        if set(p)!={'id','ink','ground','normal','increased','applicability'} or (p['ink'],p['ground'])!=expected[p['id']] or p['normal']!=(4.5 if text else 3.0) or p['increased']!=(7.0 if text else 3.0) or p['applicability']!=('actual-contact' if p['id'] in ['F:S','F:M'] else 'owner-band'): raise ValueError('invalid data plot pair semantics')
    return contract


def verify_data_plot_contrast(rules, bundle, contract):
    profiles=expand_profiles(rules)
    for surface, definition in rules['surfaces'].items():
        packets={name:profiles[definition['usages'][usage]][state] for name,usage,state in [('plot','data-plot','rest'),('category','data-category-label','rest'),('selection','selection','selected')]}
        for mode in MODES:
            def color(selector):
                if selector=='parent': role=definition['background']
                else:
                    packet,channel=selector.split('.')
                    role=packets[packet][channel]
                    if role=='$surface': role=definition['background']
                if role is None: raise ValueError('missing data plot ink: '+selector)
                return merge.token_value(bundle,'color',role,mode)
            for pair in contract['pairs']:
                if pair['applicability']!='owner-band': continue
                minimum=pair['increased' if mode.endswith('-ic') else 'normal']
                if contrast(color(pair['ink']),color(pair['ground'])) < minimum:
                    raise ValueError('insufficient data plot contrast: '+mode+'/'+surface+'/'+pair['id'])


def theme_input_contract(rules):
    import jsonschema
    system = merge.load(ROOT / 'design/system.json')
    jsonschema.validate(system, merge.load(ROOT / 'design/system.schema.json'))
    contract = system['theme_system']['theme_input_contract']
    if system['theme_system']['version_policy']['generation_rules'] != rules['rules_version']: raise ValueError('stale Theme version policy pin')
    if contract.get('contract_version') != '1.0.0': raise ValueError('unsupported Theme input contract')
    rule_schema = merge.load(ROOT / 'design/theme/rules.schema.json')
    definition_schema = merge.load(ROOT / 'design/theme/theme.schema.json')
    actual = {'surface_schema_version': rule_schema['properties']['schema_version']['const'],
              'rules_version': rules['rules_version'],
              'definition_schema_version': definition_schema['properties']['schema_version']['const'],
              'instance_format': 'tekesui.theme-instances/2'}
    for field, value in actual.items():
        if contract.get(field) != value: raise ValueError('stale or unsupported Theme input contract field: ' + field)
    if contract.get('supported_modes') != MODES or rules.get('modes') != MODES: raise ValueError('unsupported Theme input modes')
    required_pins = {'design/source/base.tokens.json','design/source/token-metadata.json','design/brand/policy.json',
                     'design/theme/data-plot-pairs.json','design/theme/rules.json','design/theme/rules.schema.json','design/theme/theme.schema.json','design/system.schema.json',ARTIFACT_LOCK_SCHEMA,
                     'scripts/generate-theme-system.py','scripts/merge-design-system.py', str(merge.UPSTREAM / 'requirements.txt'),
                     str(merge.UPSTREAM / 'tools/brand.py'), str(merge.UPSTREAM / 'tools/brand_color.py'), str(merge.UPSTREAM / 'tools/validate.py'),
                     str(merge.UPSTREAM / 'design/tokens/tokens.schema.json'), str(merge.UPSTREAM / 'design/brand/brand.schema.json'),
                     str(merge.UPSTREAM / 'design/brand/policy.schema.json'), str(merge.UPSTREAM / 'design/quality/contrast-pairs.json')}
    for rel, digest in contract['pinned_inputs'].items():
        path_parts = Path(rel)
        if path_parts.is_absolute() or '\\' in rel or '..' in path_parts.parts or path_parts.as_posix() != rel:
            raise ValueError('invalid shared input path: ' + rel)
        path = (ROOT / rel).resolve()
        if not path.is_relative_to(ROOT) or not path.is_file(): raise ValueError('invalid shared input path: ' + rel)
        if hashlib.sha256(path.read_bytes()).hexdigest() != digest: raise ValueError('stale shared input pin: ' + rel)
    if set(contract.get('pinned_inputs', {})) != required_pins: raise ValueError('Theme contract has missing or unknown shared input pins')
    return contract


def effective_base(base, metadata, contract, overrides):
    result = json.loads(json.dumps(base))
    role_metadata = metadata['tokens']
    for category in ('neutral_base', 'status_base'):
        for name, values in overrides[category].items():
            allowed = contract['neutral_base_allowed_roles'] if category == 'neutral_base' else contract['status_base_allowed_roles']
            if name not in allowed or role_metadata.get('color:' + name, {}).get('tier') != 'primitive':
                raise ValueError(f'override must name an existing primitive token: {name}')
            token = next((item for item in result['color']['tokens'] if item['name'] == name), None)
            if token is None: raise ValueError('override names absent base token: ' + name)
            if isinstance(token.get('value'), str) and token['value'].startswith('{'):
                raise ValueError('override cannot replace an alias: ' + name)
            if set(values) != set(MODES): raise ValueError('override must supply all four supported modes: ' + name)
            token['value'] = {m: values[m] for m in MODES}
    return result


def generate():
    rules = merge.load(ROOT / 'design/theme/rules.json')
    contract = theme_input_contract(rules)
    rows = bindings(rules)
    import jsonschema
    sys.path.insert(0, str(ROOT / merge.UPSTREAM / 'tools'))
    import brand
    base = merge.load(ROOT / 'design/source/base.tokens.json')
    policy = merge.load(ROOT / 'design/brand/policy.json')
    metadata = merge.load(ROOT / 'design/source/token-metadata.json')
    definitions = merge.load(ROOT / 'design/theme/themes.json')
    if not isinstance(definitions, list) or not definitions:
        raise ValueError('Theme definitions must be a non-empty array')
    if len({d['id'] for d in definitions}) != len(definitions):
        raise ValueError('duplicate Theme identity')
    # Keep the original package BrandConfig in every consumer Theme artifact.
    # It is part of the token projection and the stable baseline against which
    # external definitions are independently verified, even when no selected
    # definition names it directly.
    sources = ['design/system.json','design/theme/themes.json', 'design/brand/config.json',
               'scripts/generate-theme-system.py', *contract['pinned_inputs']]
    instances = []
    for definition in definitions:
        jsonschema.validate(definition, merge.load(ROOT / 'design/theme/theme.schema.json'))
        config_path = definition['brand_config']
        if not (ROOT / config_path).resolve().is_relative_to(ROOT):
            raise ValueError('theme config escaped source root')
        sources.append(config_path)
        config = merge.load(ROOT / config_path)
        effective = effective_base(base, metadata, contract, definition['inputs']['overrides'])
        bundle, _ = brand.compile_config(config, base=effective, policy=policy, metadata=metadata)
        verify_profile_color_references(expand_profiles(rules), bundle)
        verify_data_plot_contrast(rules, bundle, data_plot_contract())
        verify_solid_status_foregrounds(bundle)
        # Check geometry constraints before overlapping Surface/track guards.
        for mode in MODES:
            for surface, usage, state, roles in rows:
                if usage in PASSIVE_USES or usage in NOTICE_MARKERS:
                    colors = {k: None if v is None else merge.token_value(bundle,'color',v,mode) for k,v in roles.items()}
                    verify_notice_marker_contrast(usage, mode, colors, merge.token_value(bundle,'color',rules['surfaces'][surface]['background'],mode))
                    verify_passive_status_contrast(usage, mode, colors, merge.token_value(bundle,'color',rules['surfaces'][surface]['background'],mode))
        modes = {}
        for mode in MODES:
            surfaces = {}
            for surface, usage, state, roles in rows:
                colors = {k: None if v is None else merge.token_value(bundle,'color',v,mode) for k,v in roles.items()}
                # Quiet labels remain readable even when their action is unavailable
                # (Menu disabled items and Breadcrumb current pages; Issue #206).
                readable_disabled_quiet = 'disabled' in state and usage in ['action-quiet','action-quiet-error']
                if 'disabled' not in state or readable_disabled_quiet:
                    for fg in ['text','icon','marker']:
                        if colors[fg] is None: continue
                        threshold = 7.0 if readable_disabled_quiet and mode.endswith('-ic') else (3.0 if fg == 'marker' else 4.5)
                        if contrast(colors[fg],colors['background']) < threshold:
                            raise ValueError(f'insufficient foreground contrast: {definition["id"]}/{mode}/{surface}/{usage}/{state}/{fg}')
                if colors['helper'] is not None and 'disabled' not in state:
                    label_background = merge.token_value(bundle,'color',rules['surfaces'][surface]['background'],mode)
                    if contrast(colors['helper'],label_background) < 4.5:
                        raise ValueError('insufficient helper foreground contrast')
                if colors['focus'] is not None and 'disabled' not in state:
                    parent = merge.token_value(bundle,'color',rules['surfaces'][surface]['background'],mode)
                    if contrast(colors['focus'],parent) < 3.0:
                        raise ValueError(f'insufficient focus contrast: {definition["id"]}/{mode}/{surface}/{usage}/{state}')
                if usage in ['range-track','progress-track'] and state != 'disabled':
                    parent = merge.token_value(bundle,'color',rules['surfaces'][surface]['background'],mode)
                    if colors['boundary'] is None or min(contrast(colors['boundary'],colors['background']),contrast(colors['boundary'],parent)) < 3.0:
                        raise ValueError(f'insufficient {"range" if usage == "range-track" else "progress"} boundary contrast: {definition["id"]}/{mode}/{surface}/{state}')
                surfaces.setdefault(surface,{}).setdefault(usage,{})[state] = colors
            modes[mode] = surfaces
        compiled_roles = {mode: {token['name']: merge.token_value(bundle, 'color', token['name'], mode) for token in bundle['color']['tokens']} for mode in MODES}
        receipt = {'contract_version': contract['contract_version'], 'contract_sha256': hashlib.sha256(merge.bytes_json(contract)).hexdigest(), 'overrides': definition['inputs']['overrides'], 'brand_config': config_path, 'brand_config_sha256': hashlib.sha256((ROOT/config_path).read_bytes()).hexdigest()}
        instances.append(dict(definition, brand=config, compiled_color_roles=compiled_roles, provenance=receipt, modes=modes))
    result = {'format':'tekesui.theme-instances/2','schema_version':rules['schema_version'],'rules_version':rules['rules_version'],'inputs_sha256':{p:hashlib.sha256((ROOT/p).read_bytes()).hexdigest() for p in sorted(set(sources))},'theme_input_contract':contract,'instances':instances}
    return {'design/theme/generated/instances.json': merge.bytes_json(result), 'design/theme/generated/surface_rules.rs': rust_source(rules).encode()}


def canonical_json(value):
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(',', ':'), allow_nan=False).encode('utf-8')


def artifact_lock_bytes(outputs):
    import jsonschema
    system = merge.load(ROOT / 'design/system.json')
    theme_system = system['theme_system']
    if theme_system['reviewed_artifact_lock'].get('$ref') != './theme/artifact.lock.json':
        raise ValueError('Design System artifact lock reference is unsupported')
    if theme_system['artifact_lock_schema'].get('$ref') != './theme/artifact-lock.schema.json':
        raise ValueError('Design System artifact lock schema reference is unsupported')
    instance_path = 'design/theme/generated/instances.json'
    surface_path = 'design/theme/generated/surface_rules.rs'
    instances = json.loads(outputs[instance_path])
    source_inputs = instances['inputs_sha256']
    themes = [{'id': item['id'], 'version': item['version'], 'schema_version': item['schema_version'], 'brand_config': item['brand_config']} for item in instances['instances']]
    lock = {
        'format': 'tekesui.theme-artifact-lock/1',
        'instance_format': instances['format'],
        'contract_sha256': hashlib.sha256(canonical_json(instances['theme_input_contract'])).hexdigest(),
        'definition_source_sha256': source_inputs['design/theme/themes.json'],
        'source_inputs_sha256': source_inputs,
        'source_inputs_sha256_digest': hashlib.sha256(canonical_json(source_inputs)).hexdigest(),
        'instances_sha256': hashlib.sha256(outputs[instance_path]).hexdigest(),
        'surface_rules_sha256': hashlib.sha256(outputs[surface_path]).hexdigest(),
        'themes': themes,
    }
    jsonschema.validate(lock, merge.load(ROOT / ARTIFACT_LOCK_SCHEMA))
    return merge.bytes_json(lock)


def main():
    p = argparse.ArgumentParser(description=__doc__)
    group = p.add_mutually_exclusive_group()
    group.add_argument('--check',action='store_true')
    group.add_argument('--write-artifact-lock',action='store_true',help='explicitly refresh the reviewed source lock after inspecting the candidate artifacts')
    args = p.parse_args()
    outputs = generate()
    lock_bytes = artifact_lock_bytes(outputs) if args.check or args.write_artifact_lock else None
    if args.check:
        for path, expected in outputs.items():
            file = ROOT / path
            if not file.is_file() or file.read_bytes() != expected:
                raise ValueError('stale Theme output: ' + path)
        lock_file = ROOT / ARTIFACT_LOCK
        if not lock_file.is_file() or lock_file.read_bytes() != lock_bytes:
            raise ValueError('stale Theme artifact lock: ' + ARTIFACT_LOCK)
        print('Theme system and reviewed artifact lock match authoritative inputs.')
        return
    for path, expected in outputs.items():
        file = ROOT / path
        file.parent.mkdir(parents=True,exist_ok=True)
        file.write_bytes(expected)
    if args.write_artifact_lock:
        lock_file = ROOT / ARTIFACT_LOCK
        lock_file.parent.mkdir(parents=True,exist_ok=True)
        lock_file.write_bytes(lock_bytes)
        print('Generated Theme instances, Surface bindings, and explicitly refreshed artifact lock.')
    else:
        print('Generated Theme instances and shared Surface bindings. Artifact lock was not refreshed.')


if __name__ == '__main__':
    try: main()
    except Exception as e:
        print('THEME_SYSTEM_ERROR: ' + str(e), file=sys.stderr)
        sys.exit(1)
