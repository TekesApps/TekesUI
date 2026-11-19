"""Token validation and WCAG contrast checks backed by owned repo schemas and contracts."""
from __future__ import annotations

import json
import pathlib
import re

import jsonschema

DEFAULT_ROOT = pathlib.Path(__file__).resolve().parents[2]
THEMES = ['light', 'dark', 'light-ic', 'dark-ic']
BASE_STATES = {
    'rest', 'hover', 'pressed', 'focus', 'disabled',
    'selected', 'checked', 'loading', 'error',
}


def _object_pairs(pairs):
    out = {}
    for key, value in pairs:
        if key in out:
            raise ValueError('DUPLICATE_JSON_KEY: ' + key)
        out[key] = value
    return out


def _invalid_constant(value):
    raise ValueError('NONFINITE_JSON_NUMBER: ' + value)


def load_json(path: pathlib.Path):
    return json.loads(
        path.read_text(encoding='utf-8'),
        object_pairs_hook=_object_pairs,
        parse_constant=_invalid_constant,
    )


def read(relative: str, root: pathlib.Path = DEFAULT_ROOT):
    return load_json(root / relative)


def resolve(
    tokens: dict,
    family: str,
    name: str,
    theme: str = 'light',
    stack: tuple = (),
):
    key = (family, name)
    if key in stack:
        raise ValueError(f'ALIAS_CYCLE: {stack + (key,)}')
    records = [t for t in tokens[family]['tokens'] if t['name'] == name]
    if len(records) != 1:
        raise ValueError(f'ALIAS_TARGET: {family}:{name} resolves to {len(records)} tokens')
    value = records[0]['value']
    if isinstance(value, str) and value.startswith('{'):
        if not re.fullmatch(r'\{[a-z][a-z0-9_-]*\}', value):
            raise ValueError(f'ALIAS_SYNTAX: {value}')
        return resolve(tokens, family, value[1:-1], theme, stack + (key,))
    return value[theme] if isinstance(value, dict) else value


def rgb(color: str):
    return tuple(int(color[i:i + 2], 16) / 255 for i in (1, 3, 5))


def luminance(c):
    v = [x / 12.92 if x <= 0.04045 else ((x + 0.055) / 1.055) ** 2.4 for x in c]
    return .2126 * v[0] + .7152 * v[1] + .0722 * v[2]


def contrast(a, b):
    x, y = sorted([luminance(a), luminance(b)])
    return (y + .05) / (x + .05)


def contrast_thresholds(root: pathlib.Path = DEFAULT_ROOT) -> dict:
    return read('design/source/contrast-pairs.json', root)['thresholds']


def contrast_results(tokens: dict, root: pathlib.Path = DEFAULT_ROOT) -> list[dict]:
    result = []
    for pair in read('design/source/contrast-pairs.json', root)['pairs']:
        for theme in THEMES:
            fg = rgb(resolve(tokens, 'color', pair['foreground'], theme))
            bg = rgb(resolve(tokens, 'color', pair['background'], theme))
            overlay = pair.get('overlay')
            if overlay:
                oc = rgb(resolve(tokens, 'color', overlay['color'], theme))
                alpha = overlay.get('opacity_by_theme', {}).get(theme)
                if alpha is None:
                    alpha = resolve(tokens, 'opacity', overlay['opacity'], theme)
                # Rendering contract: source-over in sRGB for these solid UI recipes.
                bg = tuple(alpha * c + (1 - alpha) * b for c, b in zip(oc, bg))
            ratio = contrast(fg, bg)
            minimum = pair['ic' if theme.endswith('-ic') else 'normal']
            result.append({
                'theme': theme,
                'foreground': pair['foreground'],
                'background': pair['background'],
                'overlay': overlay,
                'ratio': ratio,
                'minimum': minimum,
                'status': 'pass' if ratio >= minimum else 'fail',
            })
    return result


def token_errors(
    tokens: dict,
    metadata: dict,
    *,
    root: pathlib.Path = DEFAULT_ROOT,
    schema: dict | None = None,
) -> list[str]:
    errors = []
    token_schema = read('design/source/tokens.schema.json', root) if schema is None else schema
    validator = jsonschema.Draft202012Validator(token_schema)
    errors += [f'SCHEMA: {list(e.path)}: {e.message}' for e in validator.iter_errors(tokens)]
    if errors:
        return errors
    expected = set()
    for family, obj in tokens.items():
        if family == 'type':
            continue
        seen = set()
        for token in obj['tokens']:
            n = token['name']
            key = family + ':' + n
            expected.add(key)
            if n in seen:
                errors.append('DUPLICATE_TOKEN: ' + key)
            seen.add(n)
            meta = metadata['tokens'].get(key)
            if not meta:
                errors.append('MISSING_METADATA: ' + key)
                continue
            if meta['unit'] != metadata['family_units'][family]:
                errors.append('UNIT: ' + key)
            tier = meta['tier']
            value = token['value']
            is_alias = isinstance(value, str) and value.startswith('{')
            if tier not in ['primitive', 'semantic', 'component']:
                errors.append('TIER: ' + key)
            if tier != 'primitive' and not is_alias:
                errors.append('UNBOUND_LITERAL: ' + key)
            if is_alias:
                target = metadata['tokens'].get(family + ':' + value[1:-1])
                if not target:
                    errors.append('MISSING_ALIAS_TARGET: ' + key)
                elif (
                    {'primitive': 0, 'semantic': 1, 'component': 2}[target['tier']]
                    > {'primitive': 0, 'semantic': 1, 'component': 2}[tier]
                ):
                    errors.append('REVERSE_DEPENDENCY: ' + key)
            for theme in THEMES:
                try:
                    resolved = resolve(tokens, family, n, theme)
                except (ValueError, KeyError) as e:
                    errors.append(str(e))
                    continue
                if family != 'color' and (
                    isinstance(resolved, bool) or not isinstance(resolved, (int, float))
                ):
                    errors.append('TOKEN_TYPE: ' + key)
    if set(metadata['tokens']) != expected:
        errors.append('ORPHAN_METADATA')
    fonts = {f['name'] for f in tokens['type']['fonts']}
    families = {f['name'] for f in tokens['type']['families']}
    for family in tokens['type']['families']:
        if not set(family['fonts']) <= fonts:
            errors.append('FONT_REFERENCE: ' + family['name'])
    names = set()
    for group in tokens['type']['groups']:
        for style in group['styles']:
            if style['name'] in names:
                errors.append('DUPLICATE_TEXT_STYLE: ' + style['name'])
            names.add(style['name'])
            if style['family'] not in families:
                errors.append('FONT_FAMILY: ' + style['name'])
            if style['lineHeight'] < style['fontSize']:
                errors.append('LINE_HEIGHT: ' + style['name'])
            try:
                resolve(tokens, 'color', style['color'][1:-1])
            except (ValueError, KeyError) as e:
                errors.append(str(e))
    return errors
