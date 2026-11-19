"""Compile consumer brand seeds into full, four-theme token bundles.

Owned build-time reference compiler, replacing direct imports from
design/imports/v0.3.0-proposal1/.
"""
from __future__ import annotations

import copy
import hashlib
import json
from pathlib import Path

import jsonschema

from .brand_color import contrast, delta_ok, parse_hex, to_oklch, tone
from .validate_tokens import (
    DEFAULT_ROOT,
    THEMES,
    contrast_results,
    read,
    resolve,
    token_errors,
)

VERSION = '1.0.0'
ALGORITHM = 'tekes-oklch-constrained-v1'
ROLE_NAMES = (
    'brand-seed', 'accent', 'accent-hover', 'accent-pressed',
    'accent-subtle-ground', 'accent-subtle-hover', 'accent-subtle-pressed',
    'accent-subtle-foreground', 'accent-outline',
    'action-primary-ground', 'action-primary-hover', 'action-primary-pressed',
    'action-primary-foreground', 'action-primary-focus-outline',
)
MANAGED = {'p-' + name for name in ROLE_NAMES}


def bytes_json(value: object) -> bytes:
    return (json.dumps(value, ensure_ascii=False, indent=2, allow_nan=False) + '\n').encode()


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def canonical(value: object) -> bytes:
    return json.dumps(
        value,
        ensure_ascii=False,
        sort_keys=True,
        separators=(',', ':'),
        allow_nan=False,
    ).encode()


def load_external(path: Path) -> dict:
    from .validate_tokens import load_json
    return load_json(path)


def config_errors(config: dict, *, root: Path = DEFAULT_ROOT, schema: dict | None = None) -> list[str]:
    brand_schema = read('design/brand/brand.schema.json', root) if schema is None else schema
    return [
        f'BRAND_CONFIG_SCHEMA: {list(e.path)}: {e.message}'
        for e in jsonschema.Draft202012Validator(brand_schema).iter_errors(config)
    ]


def policy_errors(policy: dict, *, root: Path = DEFAULT_ROOT, schema: dict | None = None) -> list[str]:
    policy_schema = read('design/brand/policy.schema.json', root) if schema is None else schema
    errors = [
        f'BRAND_POLICY_SCHEMA: {list(e.path)}: {e.message}'
        for e in jsonschema.Draft202012Validator(policy_schema).iter_errors(policy)
    ]
    if errors:
        return errors
    if policy['algorithm'] != ALGORITHM:
        errors.append('BRAND_ALGORITHM_VERSION')
    if not 0 <= policy['subtle_chroma'] <= policy['max_chroma'] <= .4:
        errors.append('BRAND_CHROMA_RANGE')
    if not 0 <= policy['subtle_chroma_factor'] <= 1:
        errors.append('BRAND_CHROMA_FACTOR')
    if not .001 <= policy['minimum_state_delta_ok'] <= .1:
        errors.append('BRAND_STATE_DELTA_RANGE')
    for theme, targets in policy['targets'].items():
        if not all(0 < x < 1 for x in targets['subtle']):
            errors.append('BRAND_SUBTLE_RANGE')
        direction = 1 if theme.startswith('dark') else -1
        if targets['action_step'] * direction <= 0 or targets['link_step'] * direction <= 0:
            errors.append('BRAND_STATE_DIRECTION: ' + theme)
        values = targets['subtle']
        if any((y - x) * direction <= 0 for x, y in zip(values, values[1:])):
            errors.append('BRAND_SUBTLE_ORDER: ' + theme)
    return errors


def pick(
    candidates: list[tuple[float, str]],
    target: float,
    constraints: list[tuple[str, float]],
    lower: float = 0.,
    upper: float = 1.,
) -> tuple[float, str]:
    allowed = [
        (l, value) for l, value in candidates
        if lower <= l <= upper
        and all(contrast(value, bg) >= threshold for bg, threshold in constraints)
    ]
    if not allowed:
        raise ValueError('BRAND_NO_FEASIBLE_COLOR: required color pair has no valid solution')
    # Stable tie-break: lower L, then serialized hex. No random optimization.
    return min(allowed, key=lambda item: (abs(item[0] - target), item[0], item[1]))


def theme_roles(base: dict, seed: str, policy: dict, theme: str) -> dict[str, str]:
    _, c, h = to_oklch(seed)
    c = min(c, policy['max_chroma'])
    steps = policy['tone_steps']
    iterations = policy['gamut_iterations']
    candidates = [(i / steps, tone(i / steps, c, h, iterations)) for i in range(steps + 1)]
    target = policy['targets'][theme]
    dark = theme.startswith('dark')
    text_min = policy['thresholds']['text_ic' if theme.endswith('-ic') else 'text']
    boundary_min = policy['thresholds']['non_text']
    background = lambda name: resolve(base, 'color', name, theme)
    text_bgs = [background(name) for name in policy['text_backgrounds']]
    control_bgs = [background(name) for name in policy['control_backgrounds']]
    values = {'brand-seed': seed.upper()}
    on_solid = '#000000' if dark else '#FFFFFF'
    action_constraints = [(x, boundary_min) for x in control_bgs] + [(on_solid, text_min)]
    l, solid = pick(candidates, target['action'], action_constraints)
    values['action-primary-ground'] = solid
    previous_l = l
    for name in ['action-primary-hover', 'action-primary-pressed']:
        step = target['action_step']
        previous_l, value = pick(
            candidates,
            previous_l + step,
            action_constraints,
            lower=previous_l + .02 if dark else 0.,
            upper=1. if dark else previous_l - .02,
        )
        values[name] = value
    values['action-primary-foreground'] = on_solid
    values['action-primary-focus-outline'] = on_solid
    link_constraints = [(x, text_min) for x in text_bgs]
    previous_l, values['accent'] = pick(candidates, target['link'], link_constraints)
    for name in ['accent-hover', 'accent-pressed']:
        step = target['link_step']
        previous_l, value = pick(
            candidates,
            previous_l + step,
            link_constraints,
            lower=previous_l + .018 if dark else 0.,
            upper=1. if dark else previous_l - .018,
        )
        values[name] = value
    subtle_c = min(c * policy['subtle_chroma_factor'], policy['subtle_chroma'])
    subtle_values = [tone(x, subtle_c, h, iterations) for x in target['subtle']]
    for name, value in zip(
        ['accent-subtle-ground', 'accent-subtle-hover', 'accent-subtle-pressed'],
        subtle_values,
    ):
        values[name] = value
    _, values['accent-subtle-foreground'] = pick(
        candidates,
        target['subtle_text'],
        [(x, text_min) for x in subtle_values],
    )
    marker_bgs = control_bgs + [background(x) for x in policy['marker_backgrounds']] + subtle_values
    _, values['accent-outline'] = pick(
        candidates,
        target['outline'],
        [(x, boundary_min) for x in marker_bgs],
    )
    return values


def dependency_names(tokens: dict) -> set[str]:
    by = {x['name']: x['value'] for x in tokens['color']['tokens']}
    affected = set(MANAGED)
    changed = True
    while changed:
        changed = False
        for name, value in by.items():
            if name not in affected and isinstance(value, str) and value[1:-1] in affected:
                affected.add(name)
                changed = True
    return affected


def compile_config(
    config: dict,
    *,
    base: dict | None = None,
    policy: dict | None = None,
    metadata: dict | None = None,
    root: Path = DEFAULT_ROOT,
) -> tuple[dict, dict]:
    errors = config_errors(config, root=root)
    if errors:
        raise ValueError('\n'.join(errors))
    policy = read('design/brand/policy.json', root) if policy is None else policy
    errors = policy_errors(policy, root=root)
    if errors:
        raise ValueError('\n'.join(errors))
    base = read('design/source/base.tokens.json', root) if base is None else base
    metadata = read('design/source/token-metadata.json', root) if metadata is None else metadata
    if any(x['name'] in MANAGED for x in base['color']['tokens']):
        raise ValueError(
            'BRAND_LITERAL_IN_BASE: generated brand primitives must not be authored in base'
        )
    seed = config['seed_srgb'].upper()
    parse_hex(seed)
    roles = {theme: theme_roles(base, seed, policy, theme) for theme in THEMES}
    for role, override in config.get('overrides', {}).items():
        for theme in THEMES:
            roles[theme][role] = override['values'][theme].upper()
    tokens = copy.deepcopy(base)
    for role in ROLE_NAMES:
        tokens['color']['tokens'].append({
            'name': 'p-' + role,
            'value': {theme: roles[theme][role] for theme in THEMES},
            'usage': '由消费者 BrandConfig 和版本化 policy 生成；禁止手改：' + role,
        })
    errors = token_errors(tokens, metadata, root=root)
    affected = dependency_names(tokens)
    forbidden_prefixes = (
        'status-', 'warning', 'diff-', 'syntax-', 'data-',
        'c-code-', 'c-chart-', 'surface-',
    )
    forbidden_names = {
        'border-focused', 'c-input-focus-outline', 'text-selection',
        'selected-text', 'row-selection', 'row-selection-hover',
        'button-hover', 'button-pressed', 'primary-text', 'secondary-text',
        'tertiary-text', 'disabled-text', 'segmented-track', 'segmented-thumb',
        'user-message-surface', 'hover-wash', 'border', 'separator',
    }
    for name in affected:
        if name.startswith(forbidden_prefixes) or name in forbidden_names:
            errors.append('BRAND_INDEPENDENT_ROLE_LEAK: ' + name)
    for role, override in config.get('overrides', {}).items():
        if len(override['reason'].strip()) < 8:
            errors.append('BRAND_OVERRIDE_REASON: ' + role)
    if not errors:
        errors += [
            f"BRAND_CONTRAST: {x['theme']} {x['foreground']}/{x['background']}"
            for x in contrast_results(tokens, root=root)
            if x['status'] != 'pass'
        ]
        for theme in THEMES:
            for name in ['action-primary-foreground', 'action-primary-focus-outline']:
                if roles[theme][name] not in ['#000000', '#FFFFFF']:
                    errors.append('BRAND_ON_SOLID_NONNEUTRAL: ' + theme + '/' + name)
            for states in [
                ('action-primary-ground', 'action-primary-hover', 'action-primary-pressed'),
                ('accent', 'accent-hover', 'accent-pressed'),
                ('accent-subtle-ground', 'accent-subtle-hover', 'accent-subtle-pressed'),
            ]:
                for a, b in zip(states, states[1:]):
                    direction = 1 if theme.startswith('dark') else -1
                    if (to_oklch(roles[theme][b])[0] - to_oklch(roles[theme][a])[0]) * direction <= 0:
                        errors.append('BRAND_STATE_ORDER: ' + theme + '/' + a + '/' + b)
                    if delta_ok(roles[theme][a], roles[theme][b]) < policy['minimum_state_delta_ok']:
                        errors.append('BRAND_STATE_COLLAPSE: ' + theme + '/' + a + '/' + b)
    if errors:
        raise ValueError('\n'.join(errors))
    _, c, h = to_oklch(seed)
    ramp = {
        f'tone-{round(l * 1000):03d}': tone(
            l, min(c, policy['max_chroma']), h, policy['gamut_iterations']
        )
        for l in [.05, .10, .20, .30, .40, .50, .60, .70, .80, .90, .95]
    }
    palette = {
        'schema_version': '1.0',
        'brand_id': config['brand_id'],
        'brand_version': config['version'],
        'seed_srgb': seed,
        'algorithm': ALGORITHM,
        'policy_version': policy['version'],
        'scope': 'role candidates; not native rendering or aesthetic approval',
        'ramp_scope': 'inspection only; consumers must use semantic tokens, not tone stops',
        'ramp': ramp,
        'roles': roles,
        'overrides': config.get('overrides', {}),
        'native_evidence': 'unverified',
    }
    return tokens, palette
