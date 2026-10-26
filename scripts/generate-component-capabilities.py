#!/usr/bin/env python3
"""Generate the current, evidence-bound Gallery capability inventory.

The workbench registry owns route/API/story metadata. Public paths are resolved
through modules/re-exports and bound by a generated external Rust integration
test. Compilation still does not establish runtime or native acceptance.
"""
from __future__ import annotations
import argparse
import hashlib
from functools import lru_cache
import json
from pathlib import Path
import re
import sys

ROOT = Path(__file__).resolve().parents[1]
REGISTRY = Path('design/gallery/workbench.json')
REVIEWS = Path('design/behavior/gold7-alignment.json')
OUTPUT = Path('design/gallery/capabilities.json')
BINDING_OUTPUT = Path('crates/tekes-ui-controls/tests/component_registry_api.rs')
HISTORICAL_MAP = Path('design/gallery/historical-component-map.json')
PUBLIC_SURFACE = Path('design/gallery/public-api-surface.json')
HISTORICAL_REGISTRY = Path('design/imports/v0.3.0-proposal1/upstream/TekesUI-Design-System-v0.3.0-proposal1/design/registry/components.json')
CRATE_SRC = Path('crates')
COVERAGE = Path('design/gallery/component-coverage.json')
STATE_PROFILES = HISTORICAL_REGISTRY.parent / 'state-profiles.json'
COMPONENT_CONTRACTS = HISTORICAL_REGISTRY.parent.parent / 'component-contracts.json'
TOKENS = Path('design/tokens.json')


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def read_json(path: Path):
    return json.loads((ROOT / path).read_text())


def source_files() -> list[Path]:
    return sorted(p.relative_to(ROOT) for p in (ROOT / CRATE_SRC).rglob('*.rs'))


def module_items(path: Path) -> dict:
    """Index top-level items only; mask Rust comments/literals before brace tracking.

    This supports the explicit modules/flat re-exports used by controls. Unknown
    public syntax fails closed rather than silently shrinking reverse coverage.
    Rust compilation remains the authority for type visibility and cfg behavior.
    """
    text = (ROOT / path).read_text()
    includes = read_json(PUBLIC_SURFACE).get('type_free_includes', []) if 'include!' in text else []
    for entry in includes:
        if digest((ROOT / entry['generator']).read_bytes()) != entry['generator_sha256']:
            raise ValueError(f"{entry['generator']}: reviewed type-free generator changed; audit generated public types")
    return _module_items(path, text, json.dumps(includes, sort_keys=True))


@lru_cache(maxsize=256)
def _module_items(path: Path, text: str, reviewed_includes: str) -> dict:
    masked = list(text)
    literal = re.compile(r"""(?s)//[^\n]*|/\*|(?:br|r)\#*"|b?"|b?'(?:\\.|[^\\'\n])'""")
    offset = 0
    while match := literal.search(text, offset):
        start, end = match.span()
        token = match.group()
        if token == '/*':
            depth = 1
            while depth:
                nested = re.search(r'/\*|\*/', text[end:])
                if not nested:
                    raise ValueError(f'{path}: unterminated Rust comment')
                end += nested.end()
                depth += 1 if nested.group() == '/*' else -1
        elif token.endswith('"'):
            if token.startswith(('r', 'br')):
                hashes = token.count('#')
                closing = '"' + '#' * hashes
                position = text.find(closing, end)
                if position < 0:
                    raise ValueError(f'{path}: unterminated raw string')
                end = position + len(closing)
            else:
                closing = re.search(r'(?s)(?:\\.|[^"\\])*"', text[end:])
                if not closing:
                    raise ValueError(f'{path}: unterminated string')
                end += closing.end()
        for i in range(start, end):
            if masked[i] != '\n':
                masked[i] = ' '
        offset = end
    clean = ''.join(masked)
    depths = []
    depth = 0
    for char in clean:
        depths.append(depth)
        depth += (char in '{([') - (char in '})]')
    if depth != 0:
        raise ValueError(f'{path}: unbalanced Rust source')
    items = {'types': {}, 'modules': {}, 'uses': {}, 'other': set()}
    for match in re.finditer(r'\b(pub(?:\([^)]*\))?\s+)?(struct|enum|trait|type|mod|use|fn|const|static)\s+([A-Za-z_][A-Za-z0-9_]*)', clean):
        if depths[match.start()] != 0:
            continue
        visibility, kind, name = match.groups()
        public = visibility is not None and visibility.strip() == 'pub'
        if kind == 'mod':
            tail = clean[match.end():].lstrip()
            if not tail.startswith(';'):
                if public:
                    raise ValueError(f'{path}: unsupported public inline module {name}; extend the surface index')
                continue
            base = path.parent if path.name in {'lib.rs', 'mod.rs'} else path.with_suffix('')
            child = next((p for p in [base / (name + '.rs'), base / name / 'mod.rs'] if (ROOT / p).is_file()), None)
            if not child:
                raise ValueError(f'{path}: module source missing for {name}')
            items['modules'][name] = (child, public)
        elif kind == 'use' and public:
            end = clean.find(';', match.start())
            expression = clean[match.start():end].removeprefix('pub').strip().removeprefix('use').strip()
            if '{' in expression:
                prefix, names = expression.split('{', 1)
                leaves = [prefix.rstrip(':').strip() + '::' + n.strip() for n in names.rstrip('}').split(',') if n.strip()]
            else:
                leaves = [expression]
            for leaf in leaves:
                if not re.fullmatch(r'[A-Za-z_][A-Za-z0-9_]*(?:::[A-Za-z_][A-Za-z0-9_]*)*(?:\s+as\s+[A-Za-z_][A-Za-z0-9_]*)?', leaf):
                    raise ValueError(f'{path}: unsupported public re-export {leaf!r}; extend the surface index')
                target, *alias = re.split(r'\s+as\s+', leaf)
                exposed = alias[0] if alias else target.rsplit('::', 1)[-1]
                items['uses'][exposed] = target.split('::')
        elif public:
            if kind in {'struct', 'enum', 'trait', 'type'}:
                items['types'][name] = kind
            else:
                items['other'].add(name)
    for match in re.finditer(r'\b[A-Za-z_][A-Za-z0-9_]*\s*!|#\s*\[\s*path\b|\bpub\s+use\s*\{', clean):
        if depths[match.start()] == 0:
            if any(entry['source'] == path.as_posix() and text.startswith(entry['statement'], match.start()) for entry in json.loads(reviewed_includes)):
                continue
            raise ValueError(f'{path}: unsupported top-level macro/path/group import; extend the surface index')
    # Wildcards, macros or new public item forms cannot bypass the inventory.
    for match in re.finditer(r'\bpub\s+(\w+)', clean):
        if depths[match.start()] == 0 and match[1] not in {'struct', 'enum', 'trait', 'type', 'mod', 'use', 'fn', 'const', 'static'}:
            raise ValueError(f'{path}: unsupported public syntax {match[1]}; extend the surface index')
    return items


def resolve_public_item(api: str) -> tuple[str, str, str]:
    if not isinstance(api, str) or not re.fullmatch(r'tekes_ui_controls(?:::[A-Za-z_][A-Za-z0-9_]*)+', api):
        raise ValueError(f'unsupported public component API: {api!r}')
    root = Path('crates/tekes-ui-controls/src/lib.rs')

    def locate(path, parts, internal, seen):
        key = (path, tuple(parts), internal)
        if not parts or key in seen:
            raise ValueError(f'{api}: empty path or cyclic re-export')
        seen = seen | {key}
        items = module_items(path)
        if parts[0] == 'crate':
            return locate(root, parts[1:], internal, seen)
        if parts[0] == 'self':
            return locate(path, parts[1:], internal, seen)
        if len(parts) == 1:
            if parts[0] in items['types']:
                return path.as_posix(), parts[0], items['types'][parts[0]]
            if parts[0] in items['other']:
                return path.as_posix(), parts[0], 'non-type'
        if parts[0] in items['modules']:
            child, public = items['modules'][parts[0]]
            if public or internal:
                if len(parts) == 1:
                    return child.as_posix(), parts[0], 'module'
                return locate(child, parts[1:], internal, seen)
        if parts[0] in items['uses']:
            return locate(path, items['uses'][parts[0]] + parts[1:], True, seen)
        raise ValueError(f'{api}: no public path/re-export at {path} for {"::".join(parts)}')

    return locate(root, api.split('::')[1:], False, set())


def resolve_api(api: str) -> tuple[str, str]:
    source, symbol, kind = resolve_public_item(api)
    if kind in {'non-type', 'module'}:
        raise ValueError(f'{api}: expected a public type, found {kind}')
    return source, symbol


def public_type_surface() -> dict:
    result = {}

    def visit(path, prefix, ancestors):
        if path in ancestors:
            raise ValueError(f'{prefix}: cyclic public module')
        items = module_items(path)
        for name in sorted(set(items['types']) | set(items['uses'])):
            api = prefix + '::' + name
            source, symbol, kind = resolve_public_item(api)
            if kind == 'module':
                raise ValueError(f'{api}: module re-exports require explicit surface support')
            if kind != 'non-type':
                result[api] = {'source': source, 'symbol': symbol, 'kind': kind}
        for name, (child, public) in items['modules'].items():
            if public:
                visit(child, prefix + '::' + name, ancestors | {path})

    visit(Path('crates/tekes-ui-controls/src/lib.rs'), 'tekes_ui_controls', set())
    return dict(sorted(result.items()))


def type_alias_target(api: str) -> str | None:
    source, symbol, kind = resolve_public_item(api)
    if kind != 'type':
        return None
    text = (ROOT / source).read_text()
    match = re.search(r'pub\s+type\s+' + re.escape(symbol) + r'\s*=\s*([A-Za-z_][A-Za-z0-9_]*)\s*;', text)
    if not match:
        raise ValueError(f'{api}: unsupported type alias target; extend alias resolution')
    return api.rsplit('::', 1)[0] + '::' + match[1]


def reverse_coverage(registry: dict) -> dict:
    surface = public_type_surface()
    data = read_json(PUBLIC_SURFACE)
    if data.get('format') != 'tekesui-public-api-surface/1':
        raise ValueError('unsupported public API classification format')
    entries = data.get('types', [])
    paths = [e['api'] for e in entries]
    if len(paths) != len(set(paths)):
        raise ValueError('duplicate public API classification')
    missing, stale = sorted(set(surface) - set(paths)), sorted(set(paths) - set(surface))
    if missing or stale:
        raise ValueError(f'public API classification missing={missing}, stale/private={stale}')
    components = {e['component']: e for e in registry['entries'] if e['kind'] == 'component'}
    owners = {}
    for identity, entry in components.items():
        if not isinstance(entry.get('slug'), str) or not entry['slug']:
            raise ValueError(f'{identity}: missing component route')
        for api in component_bindings(entry):
            owners.setdefault(api, set()).add(identity)
    for entry in entries:
        api, category = entry['api'], entry.get('category')
        if category not in {'component', 'component-adapter', 'component-alias', 'component-support', 'composition', 'shared-support'} or not isinstance(entry.get('reason'), str) or not entry['reason'].strip():
            raise ValueError(f'{api}: explicit category and reason required')
        ids = entry.get('components', [])
        if not isinstance(ids, list) or len(ids) != len(set(ids)) or not set(ids) <= set(components):
            raise ValueError(f'{api}: unknown/duplicate component identity')
        if category == 'component':
            if not ids or set(ids) != owners.get(api, set()):
                raise ValueError(f'{api}: component binding/route coverage mismatch; classified={ids}, registered={sorted(owners.get(api, set()))}')
        elif api in owners:
            raise ValueError(f'{api}: registered component cannot be classified as helper')
        elif category in {'component-support', 'component-adapter', 'component-alias'} and not ids:
            raise ValueError(f'{api}: component support/adapter/alias needs an existing owner')
        if category == 'component-alias':
            source = surface[api]
            if not any(resolve_api(binding) == (source['source'], source['symbol']) or (source['kind'] == 'type' and type_alias_target(api) == binding) for identity in ids for binding in components[identity]['api_bindings']):
                raise ValueError(f'{api}: alias must reference a registered component type')
        elif category in {'composition', 'shared-support'} and ids:
            raise ValueError(f'{api}: shared/composition support must not claim component identity')
    if set(owners) - set(paths):
        raise ValueError(f'component bindings absent from public surface: {sorted(set(owners) - set(paths))}')
    return {'scope': 'tekes_ui_controls public type paths, including aliases; functions/constants and other crates are outside this type audit',
            'source': PUBLIC_SURFACE.as_posix(), 'status': 'classified; compiler bindings and runtime acceptance remain separate',
            'counts': {'paths': len(surface), 'categories': {category: sum(e['category'] == category for e in entries) for category in sorted({e['category'] for e in entries})}},
            'types': [{**entry, **surface[entry['api']], 'routes': [components[identity]['slug'] for identity in entry.get('components', [])]} for entry in sorted(entries, key=lambda e: e['api'])]}


def component_bindings(item: dict) -> list[str]:
    paths = item.get('api_bindings')
    if not isinstance(paths, list) or not paths or not all(isinstance(p, str) for p in paths) or paths[0] != item.get('api') or len(set(paths)) != len(paths):
        raise ValueError(f'{item.get("slug")}: missing/duplicate public API bindings or primary API mismatch')
    for path in paths:
        try:
            resolve_api(path)
        except ValueError as error:
            raise ValueError(f'{item.get("slug")}: {error}') from error
    return paths


def api_source(api: str) -> dict:
    source, symbol = resolve_api(api)
    text = (ROOT / source).read_text()
    declaration = re.search(r'(?m)^[ \t]*pub\s+(?:struct|enum|trait|type)\s+' + re.escape(symbol) + r'\b', text)
    return {'source': source, 'symbol': symbol, 'line': text.count('\n', 0, declaration.start()) + 1}


def binding_test(registry: dict) -> str:
    surface = reverse_coverage(registry)
    surface_calls = []
    for index, entry in enumerate(surface['types']):
        api = entry['api']
        if entry['kind'] == 'trait':
            # Check the public trait bound without imposing dyn compatibility.
            surface_calls.append(f'    #[allow(dead_code)]\n    fn public_trait_{index}<T: {api} + ?Sized>() {{}}\n    actual.push({json.dumps(api)});')
        else:
            call = f'    public_type::<{api}>(&mut actual, {json.dumps(api)});'
            surface_calls.append(call if len(call) <= 100 else f'    public_type::<{api}>(\n        &mut actual,\n        {json.dumps(api)},\n    );')
    calls = []
    for entry in registry['entries']:
        if entry.get('kind') != 'component':
            continue
        for api in component_bindings(entry):
            calls.append(f'''    bind::<{api}>(
        &mut actual,
        {json.dumps(entry['component'])},
        {json.dumps(entry['slug'])},
        {json.dumps(api)},
    );''')
    return '''// Generated by scripts/generate-component-capabilities.py; do not edit.
// Public integration-test context: private modules and missing re-exports fail compilation.
fn bind<'a, T>(
    actual: &mut Vec<(&'a str, &'a str, &'a str)>,
    id: &'a str,
    route: &'a str,
    api: &'a str,
) {
    assert!(!std::any::type_name::<T>().is_empty());
    actual.push((id, route, api));
}

#[test]
fn every_registered_component_binding_is_public_and_matches_the_current_registry() {
    let mut actual = Vec::new();
''' + '\n'.join(calls) + '''
    let registry: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../design/gallery/workbench.json"
    )))
    .unwrap();
    let mut expected = Vec::new();
    for entry in registry["entries"].as_array().unwrap() {
        if entry["kind"] != "component" {
            continue;
        }
        let bindings = entry["api_bindings"]
            .as_array()
            .expect("public API bindings");
        assert_eq!(bindings[0], entry["api"], "primary public API mismatch");
        for api in bindings {
            expected.push((
                entry["component"].as_str().unwrap(),
                entry["slug"].as_str().unwrap(),
                api.as_str().unwrap(),
            ));
        }
    }
    assert_eq!(
        actual, expected,
        "registry/API/route changed; regenerate bindings"
    );
}

fn public_type<T: ?Sized>(actual: &mut Vec<&'static str>, api: &'static str) {
    assert!(!std::any::type_name::<T>().is_empty());
    actual.push(api);
}

#[test]
fn every_classified_public_type_is_accessible_to_a_library_consumer() {
    let mut actual = Vec::new();
''' + '\n'.join(surface_calls) + '''
    let surface: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../design/gallery/public-api-surface.json"
    )))
    .unwrap();
    let expected: Vec<_> = surface["types"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["api"].as_str().unwrap())
        .collect();
    assert_eq!(
        actual, expected,
        "public type surface changed; regenerate bindings"
    );
}

#[test]
fn required_design_states_and_contract_tokens_match_public_inputs() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let read = |path: &str| -> serde_json::Value {
        serde_json::from_str(&std::fs::read_to_string(root.join(path)).unwrap()).unwrap()
    };
    let capabilities = read("design/gallery/capabilities.json");
    let ledger = read("design/gallery/component-coverage.json");
    let historical = read(concat!(
        "design/imports/v0.3.0-proposal1/upstream/",
        "TekesUI-Design-System-v0.3.0-proposal1/design/registry/components.json"
    ));
    let profiles = read(concat!(
        "design/imports/v0.3.0-proposal1/upstream/",
        "TekesUI-Design-System-v0.3.0-proposal1/design/registry/state-profiles.json"
    ));
    for component in capabilities["components"].as_array().unwrap() {
        let id = component["id"].as_str().unwrap();
        let row = ledger["components"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["id"] == id)
            .unwrap();
        let original = historical["components"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["id"] == id);
        let profile = original.map_or(&row["profile"], |row| &row["state_profile"]);
        let mut required = profiles["profiles"][profile.as_str().unwrap()]
            .as_array()
            .unwrap()
            .clone();
        if let Some(original) = original {
            for state in original["states"].as_array().unwrap() {
                if !required.contains(state) {
                    required.push(state.clone());
                }
            }
        } else if let Some(exceptions) = row["profile_exceptions"].as_object() {
            required.retain(|state| !exceptions.contains_key(state.as_str().unwrap()));
        }
        assert_eq!(
            component["state_coverage"]["required"].as_array().unwrap(),
            &required,
            "{id}"
        );
        let mut reviewed: Vec<_> = row["states"]
            .as_array()
            .unwrap()
            .iter()
            .map(|review| review["state"].clone())
            .collect();
        required.sort_by(|a, b| a.as_str().cmp(&b.as_str()));
        reviewed.sort_by(|a, b| a.as_str().cmp(&b.as_str()));
        assert_eq!(reviewed, required, "{id}: missing required state review");
        assert_eq!(
            component["token_dependencies"]["status"],
            "runtime-unverified"
        );
        for reference in component["token_dependencies"]["contract_references"]
            .as_array()
            .unwrap()
        {
            let family = reference["family"].as_str().unwrap();
            let token = reference["token"].as_str().unwrap();
            for theme in tekes_ui_tokens::Theme::ALL {
                let present = match family {
                    "color" => tekes_ui_tokens::proposal::color(token, theme).is_some(),
                    "type" => tekes_ui_tokens::proposal::text(token).is_some(),
                    _ => tekes_ui_tokens::proposal::metric(family, token).is_some(),
                };
                assert!(present, "{id}: missing public token {family}:{token}");
            }
        }
    }
}
'''


def historical_mapping(registry: dict, historical: list[dict]) -> dict:
    data = read_json(HISTORICAL_MAP)
    if data.get('format') != 'tekesui-historical-component-map/1':
        raise ValueError('unsupported historical mapping format')
    if data.get('historical_source') != HISTORICAL_REGISTRY.as_posix():
        raise ValueError('historical mapping must reference the immutable historical registry')
    old = {e['id'] for e in historical}
    current = {e['component'] for e in registry['entries'] if e['kind'] == 'component'}
    entries = data.get('entries', [])
    ids = [e['historical'] for e in entries]
    if len(ids) != len(set(ids)) or set(ids) != old:
        raise ValueError('historical map must cover every historical ID exactly once')
    covered = set()
    for entry in entries:
        targets = entry.get('current', [])
        if not targets or len(targets) != len(set(targets)) or not set(targets) <= current or not entry.get('note'):
            raise ValueError(f'invalid historical mapping for {entry["historical"]}')
        relation = entry.get('relationship')
        if relation == 'same-id':
            if targets != [entry['historical']]:
                raise ValueError('same-id mapping must preserve exact identity')
        elif relation != 'partial-overlap':
            raise ValueError(f'unsupported historical relationship: {relation}')
        covered.update(targets)
    additions = data.get('current_additions', [])
    added = [e['current'] for e in additions]
    if len(added) != len(set(added)) or set(added) != current - covered or any(not e.get('note') for e in additions):
        raise ValueError('historical map must explain every current addition exactly once')
    return data


@lru_cache(maxsize=4)
def _observation_document(raw: bytes) -> dict:
    # Internal read-only cache; source digests are checked against disk each use.
    return json.loads(raw)


def _safe_repository_file(raw: object, owner: str, description: str) -> tuple[str, Path]:
    if not isinstance(raw, str) or not raw:
        raise ValueError(f'{owner}: invalid {description} path')
    relative = Path(raw)
    if relative.is_absolute() or relative.as_posix() != raw or '..' in relative.parts or '.' in relative.parts:
        raise ValueError(f'{owner}: invalid {description} path: {raw!r}')
    root = ROOT.resolve()
    resolved = (ROOT / relative).resolve()
    try:
        resolved.relative_to(root)
    except ValueError as error:
        raise ValueError(f'{owner}: {description} path escapes repository: {raw!r}') from error
    return relative.as_posix(), resolved


def runtime_observation(row: dict) -> dict | None:
    path = row.get('runtime_token_observation')
    if path is None:
        return None
    identity = row.get('id')
    artifact, artifact_path = _safe_repository_file(path, str(identity), 'runtime observation')
    if not artifact.startswith('design/reviews/'):
        raise ValueError(f'{identity}: invalid runtime observation path: {artifact}')
    data = _observation_document(artifact_path.read_bytes())
    if not isinstance(data, dict):
        raise ValueError(f'{identity}: malformed runtime observation document')
    inputs = data.get('inputs_sha256')
    cases_document = data.get('cases')
    excluded = data.get('excluded')
    valid_excluded = ((isinstance(excluded, str) and bool(excluded.strip()))
                      or (isinstance(excluded, list) and bool(excluded)
                          and all(isinstance(item, str) and item for item in excluded)))
    if (data.get('format') != 'tekesui-runtime-token-observation/1'
            or data.get('complete') is not False
            or not isinstance(data.get('scope'), str) or not data['scope'].strip()
            or not isinstance(inputs, dict) or not inputs
            or not isinstance(cases_document, list) or not cases_document
            or not valid_excluded):
        raise ValueError(f'{identity}: runtime observation must carry scope, incomplete boundary, source identity and cases')
    differences = []
    normalized_inputs = {}
    for source, expected in sorted(inputs.items()):
        source_path, resolved = _safe_repository_file(source, str(identity), 'runtime observation source')
        if not isinstance(expected, str) or not re.fullmatch(r'[0-9a-f]{64}', expected):
            raise ValueError(f'{identity}: malformed runtime observation identity for {source_path}')
        if not resolved.is_file():
            raise ValueError(f'{identity}: missing runtime observation source path: {source_path}')
        actual = digest(resolved.read_bytes())
        normalized_inputs[source_path] = expected
        if actual != expected:
            differences.append({'path': source_path, 'expected_sha256': expected, 'actual_sha256': actual})
    cases = []
    for case in cases_document:
        theme = case.get('theme') if isinstance(case, dict) else None
        valid_theme = (isinstance(theme, str) and bool(theme)) or (type(theme) is int and 0 <= theme <= 3)
        if not isinstance(case, dict) or not isinstance(case.get('component'), str) or not case['component'] or not valid_theme:
            raise ValueError(f'{identity}: malformed runtime observation case identity')
        reads = case.get('lookups')
        if not isinstance(reads, list) or any(
            not isinstance(pair, list) or len(pair) != 2
            or any(not isinstance(value, str) or not value for value in pair)
            for pair in reads
        ):
            raise ValueError(f'{identity}: malformed actual runtime lookup set')
        if not reads:
            reason = case.get('zero_lookup_reason')
            if (case.get('no_dynamic_lookups') is not True
                    or case.get('state_asserted') is not True
                    or not isinstance(reason, str) or not reason.strip()):
                raise ValueError(f'{identity}: empty runtime lookup set needs asserted no-op evidence and reason')
        elif case.get('no_dynamic_lookups') is True:
            raise ValueError(f'{identity}: contradictory nonempty runtime lookup set marked empty')
        cases.append(case)
    selected = [case for case in cases if case['component'] == identity]
    if not selected:
        raise ValueError(f'{identity}: missing actual runtime lookup cases for this component identity')
    return {
        'artifact': artifact,
        'cases': selected,
        'scope': data['scope'],
        'excluded': excluded,
        'complete': False,
        'status': 'historical-stale' if differences else 'current',
        'inputs_sha256': normalized_inputs,
        'identity_differences': differences,
    }


def state_observed(state: str, case: dict) -> bool:
    if 'range_state' in case:
        return case.get('state_asserted') is True and state == case['range_state']
    if 'choice_state' in case:
        return case.get('state_asserted') is True and state == case['choice_state']
    if 'content_state' in case:
        return case.get('state_asserted') is True and state == case['content_state']
    if 'navigation_state' in case:
        return case.get('state_asserted') is True and state == case['navigation_state']
    if 'action_state' in case:
        return case.get('state_asserted') is True and state == case['action_state']
    if 'input_state' in case:
        # Exact named public API cases, never infer native/hover coverage from a
        # drawing or from an unrelated selection-control phase.
        if state == 'hover':
            return case['input_state'] == 'hover' and case.get('pointer_hovered') is True
        if state == 'pressed':
            return case['input_state'] == 'pressed' and case.get('pointer_pressed') is True
        return state == case['input_state']
    return {
        'rest': lambda: not case['error'] and not case['disabled'] and not case['focus'] and case['phase'] == 0,
        'hover': lambda: case['phase'] == 1 and not case['disabled'],
        'pressed': lambda: case['phase'] == 2 and not case['disabled'],
        'focus': lambda: case['focus'] and not case['disabled'],
        'disabled': lambda: case['disabled'],
        'checked': lambda: case['value'] == 1,
        'mixed': lambda: case['value'] == 2,
        'error': lambda: case['error'],
    }.get(state, lambda: False)()


def practical_intake(row: dict) -> dict | None:
    """Project bounded, accepted implementation receipts without promoting roots."""
    intake = row.get('practical_intake')
    if intake is None:
        return None
    if not isinstance(intake, dict) or intake.get('status') != 'accepted-bounded-evidence':
        raise ValueError(f'{row.get("id")}: invalid practical intake status')
    if not re.fullmatch(r'[0-9a-f]{40}', str(intake.get('baseline_commit', ''))):
        raise ValueError(f'{row.get("id")}: invalid practical intake baseline identity')
    scope = intake.get('scope')
    if not isinstance(scope, str) or not scope.strip():
        raise ValueError(f'{row.get("id")}: practical intake scope missing')
    for key in ('claims', 'exceptions', 'gaps'):
        values = intake.get(key)
        if not isinstance(values, list) or not values or not all(isinstance(value, str) and value.strip() for value in values):
            raise ValueError(f'{row.get("id")}: practical intake {key} missing or malformed')
    sources = intake.get('source_identity')
    if (not isinstance(sources, dict) or not isinstance(sources.get('sha256'), dict)
            or not sources['sha256'] or not isinstance(sources.get('scope'), str)
            or not sources['scope'].strip()):
        raise ValueError(f'{row.get("id")}: practical intake source identity missing')
    differences = {}
    for source, expected in sources['sha256'].items():
        if not isinstance(source, str) or Path(source).is_absolute() or '..' in Path(source).parts or '.' in Path(source).parts:
            raise ValueError(f'{row.get("id")}: unsafe practical intake source path {source!r}')
        if not isinstance(expected, str) or not re.fullmatch(r'[0-9a-f]{64}', expected):
            raise ValueError(f'{row.get("id")}: malformed practical intake source hash for {source}')
        _, path = _safe_repository_file(source, str(row.get('id')), 'practical intake source')
        if not path.is_file():
            raise ValueError(f'{row.get("id")}: missing practical intake source {source}')
        actual = digest(path.read_bytes())
        if actual != expected:
            differences[source] = {'expected': expected, 'actual': actual}
    evidence = intake.get('evidence')
    if not isinstance(evidence, list) or not evidence:
        raise ValueError(f'{row.get("id")}: practical intake evidence missing')
    projected_evidence = []
    for item in evidence:
        if not isinstance(item, dict) or not isinstance(item.get('issue'), str) or not re.fullmatch(r'#[1-9][0-9]*', item['issue']):
            raise ValueError(f'{row.get("id")}: malformed practical intake issue evidence')
        if item.get('component') != row.get('id'):
            raise ValueError(f'{row.get("id")}: cross-component practical intake evidence')
        if not re.fullmatch(r'[0-9a-f]{40}', str(item.get('commit', ''))):
            raise ValueError(f'{row.get("id")}: malformed practical intake evidence commit')
        artifact = item.get('artifact')
        if not isinstance(artifact, str) or not artifact.startswith('/Volumes/HP P900/DEV/TekesUI/artifacts/'):
            raise ValueError(f'{row.get("id")}: unsafe practical intake artifact path')
        if not re.fullmatch(r'[0-9a-f]{64}', str(item.get('sha256', ''))):
            raise ValueError(f'{row.get("id")}: malformed practical intake artifact hash')
        if not isinstance(item.get('scope'), str) or not item['scope'].strip():
            raise ValueError(f'{row.get("id")}: practical intake evidence scope missing')
        projected_evidence.append({key: item[key] for key in ('issue', 'component', 'commit', 'artifact', 'sha256', 'scope')})
    return {
        'status': 'accepted-source-subset-matched' if not differences else 'accepted-source-subset-stale',
        'baseline_commit': intake['baseline_commit'],
        'scope': scope,
        'claims': intake['claims'],
        'exceptions': intake['exceptions'],
        'gaps': intake['gaps'],
        'consumer_entry': {
            'script': 'scripts/component-context.py',
            'invocation': f'--component {row["id"]}',
            'projection_path': 'component.state_coverage.practical_intake',
        },
        'source_identity': {'scope': sources['scope'], 'sha256': sources['sha256'], 'differences': differences},
        'evidence': projected_evidence,
        'evidence_policy': 'Recorded immutable receipt references; projection checks source hashes but does not re-execute or independently authenticate receipt contents.',
    }


def component_coverage(registry: dict) -> dict:
    """Account for every design-required state without claiming implementation."""
    data = read_json(COVERAGE)
    if data.get('format') != 'tekesui-component-coverage/1':
        raise ValueError('unsupported component coverage format')
    profiles = read_json(STATE_PROFILES)['profiles']
    historical = {e['id']: e for e in read_json(HISTORICAL_REGISTRY)['components']}
    components = {e['component']: e for e in registry['entries'] if e['kind'] == 'component'}
    rows = data.get('components', [])
    ids = [e['id'] for e in rows]
    if len(ids) != len(set(ids)) or set(ids) != set(components):
        raise ValueError(f'component coverage missing={sorted(set(components) - set(ids))}, stale={sorted(set(ids) - set(components))} or duplicate identity')
    result = {}
    known_sources = {p.as_posix() for p in source_files()}
    for row in rows:
        identity = row['id']
        component = components[identity]
        if identity in historical:
            profile = historical[identity]['state_profile']
            required = list(dict.fromkeys(profiles[profile] + historical[identity]['states']))
            if 'profile' in row or row.get('profile_exceptions'):
                raise ValueError(f'{identity}: historical required states cannot be silently overridden')
        else:
            profile = row.get('profile')
            if profile not in profiles or not row.get('profile_reason'):
                raise ValueError(f'{identity}: existing state profile and reason required')
            exceptions = row.get('profile_exceptions', {})
            if not isinstance(exceptions, dict) or not set(exceptions) <= set(profiles[profile]) or any(not isinstance(reason, str) or not reason.strip() for reason in exceptions.values()):
                raise ValueError(f'{identity}: invalid state-profile exceptions')
            required = [state for state in profiles[profile] if state not in exceptions]
        reviews = row.get('states', [])
        states = [e['state'] for e in reviews]
        if len(states) != len(set(states)) or set(states) != set(required):
            raise ValueError(f'{component["slug"]}: required state coverage missing={sorted(set(required)-set(states))}, unexpected={sorted(set(states)-set(required))} or duplicate state')
        observation = runtime_observation(row)
        for review in reviews:
            state = review['state']
            if review.get('runtime') not in {'unverified', 'public-api-verified'} or review.get('experience') != 'unverified':
                raise ValueError(f'{component["slug"]}/{state}: this source ledger cannot promote runtime/native acceptance')
            if review['runtime'] == 'public-api-verified':
                if not observation or len({case['theme'] for case in observation['cases'] if state_observed(state, case)}) != 4:
                    raise ValueError(f'{component["slug"]}/{state}: missing four-theme public API evidence')
            if review.get('fixture') == 'source-reviewed':
                evidence = review.get('evidence', {})
                source = evidence.get('source', '')
                anchor = evidence.get('anchor', '')
                if source not in known_sources or not anchor or anchor not in (ROOT / source).read_text() or evidence.get('story') not in {'states', 'playground', 'variants'} or not evidence.get('note'):
                    raise ValueError(f'{component["slug"]}/{state}: fixture source/anchor/story/review note missing')
            elif review.get('fixture') != 'unverified' or not review.get('note'):
                raise ValueError(f'{component["slug"]}/{state}: explicit fixture review status/note required')
        projected_reviews = reviews
        historical_public_api_verified = [e['state'] for e in reviews if e['runtime'] == 'public-api-verified']
        current_public_api_verified = historical_public_api_verified
        projection = {}
        if observation and observation['status'] == 'historical-stale':
            current_public_api_verified = []
            projected_reviews = []
            for review in reviews:
                projected = dict(review)
                if review['runtime'] == 'public-api-verified':
                    projected['historical_runtime'] = review['runtime']
                    projected['runtime'] = 'unverified'
                    projected['runtime_evidence_status'] = 'stale-historical'
                    projected['runtime_identity_differences'] = observation['identity_differences']
                projected_reviews.append(projected)
            projection = {
                'runtime_observation_status': 'historical-stale',
                'historical_public_api_verified': historical_public_api_verified,
                'runtime_identity_differences': observation['identity_differences'],
            }
        result[identity] = {
            'status': 'required-states-accounted; runtime-evidence-scoped; experience-unverified',
            'source': COVERAGE.as_posix(), 'profile_source': STATE_PROFILES.as_posix(),
            'profile': profile, 'required': required,
            'profile_exceptions': row.get('profile_exceptions', {}),
            'reviews': projected_reviews,
            'public_api_verified': current_public_api_verified,
            'fixture_unverified': [e['state'] for e in reviews if e['fixture'] == 'unverified'],
            **({'practical_intake': practical_intake(row)} if row.get('practical_intake') is not None else {}),
            **projection,
        }
    return result


def contract_token_references(identity: str) -> dict:
    """Extract exact canonical IDs from existing JSON; never infer runtime use."""
    tokens = read_json(TOKENS)['proposal']['tokens']
    index = {}
    by_name = {}
    for family, group in tokens.items():
        values = ([style for g in group['groups'] for style in g['styles']]
                  if family == 'type' else group['tokens'])
        for token in values:
            key = family + ':' + token['name']
            index[key] = token
            by_name.setdefault(token['name'], []).append(key)
    contracts = read_json(COMPONENT_CONTRACTS)['components']
    row = next(row for row in read_json(COVERAGE)['components'] if row['id'] == identity)
    inherited = row.get('token_contract')
    owner = identity
    if inherited is not None:
        if identity in contracts or inherited.get('owner') not in contracts or not inherited.get('reason'):
            raise ValueError(f'{identity}: invalid inherited token contract or implicit override')
        owner = inherited['owner']
    references = {}

    def collect(value, pointer):
        if isinstance(value, dict):
            for key, child in value.items():
                collect(child, pointer + '/' + key.replace('~', '~0').replace('/', '~1'))
        elif isinstance(value, list):
            for i, child in enumerate(value):
                collect(child, pointer + '/' + str(i))
        elif isinstance(value, str):
            name = value[1:-1] if value.startswith('{') and value.endswith('}') else value
            matches = by_name.get(name, [])
            if not matches and re.fullmatch(r'(?:c-|p-|size-|space-|radius-|stroke-|surface-|control-|action-|selection-|row-)[a-z0-9-]+', name):
                raise ValueError(f'{identity}: unknown contract token {name} at {pointer}')
            if len(matches) > 1:
                raise ValueError(f'{identity}: ambiguous contract token {name}')
            if matches:
                references.setdefault(matches[0], []).append(pointer)

    collect(contracts.get(owner, {}), '/components/' + owner)
    closure = set()

    def expand(key, visiting):
        if key in visiting:
            raise ValueError(f'{identity}: cyclic token dependency {key}')
        if key in closure:
            return
        value = index[key]
        family = key.split(':', 1)[0]
        aliases = []
        if family == 'type':
            if isinstance(value.get('color'), str) and value['color'].startswith('{'):
                aliases.append('color:' + value['color'][1:-1])
        else:
            def aliases_in(payload):
                if isinstance(payload, dict):
                    for child in payload.values():
                        aliases_in(child)
                elif isinstance(payload, list):
                    for child in payload:
                        aliases_in(child)
                elif isinstance(payload, str) and payload.startswith('{') and payload.endswith('}'):
                    aliases.append(family + ':' + payload[1:-1])
            aliases_in(value.get('value'))
        for target in aliases:
            if target not in index:
                raise ValueError(f'{identity}: unknown token alias {target}')
            expand(target, visiting | {key})
        closure.add(key)

    for key in references:
        expand(key, set())
    observation = runtime_observation(row)
    observed = sorted({family + ':' + token for case in observation['cases'] for family, token in case['lookups']}) if observation else []
    if any(key not in index for key in observed):
        raise ValueError(f'{identity}: observed unknown canonical token: {sorted(set(observed) - set(index))}')
    result = {'status': 'runtime-unverified', 'tokens': [],
            'contract_source': COMPONENT_CONTRACTS.as_posix(),
            'contract_scope': 'exact canonical IDs present in the existing component JSON only; not a complete effective runtime dependency map',
            'contract_references': [{'family': key.split(':', 1)[0], 'token': key.split(':', 1)[1], 'pointers': references[key]} for key in sorted(references)],
            'contract_alias_closure': sorted(closure),
            'contract_present': owner in contracts, 'contract_owner': owner,
            'contract_inheritance_reason': inherited['reason'] if inherited else None}
    if observation:
        result['runtime_observation'] = {key: value for key, value in observation.items() if key != 'cases'}
        result['runtime_observation'].update(case_count=len(observation['cases']), tokens=observed)
    return result


def generate(binding_override: bytes | None = None) -> dict:
    registry = read_json(REGISTRY)
    review_doc = read_json(REVIEWS)
    entries = registry.get('entries')
    if not isinstance(entries, list):
        raise ValueError('workbench registry entries must be an array')
    reviews = {item['slug']: item for item in review_doc.get('component_reviews', [])}
    files = source_files()
    coverage = component_coverage(registry)
    if binding_override is not None and BINDING_OUTPUT not in files:
        files = sorted([*files, BINDING_OUTPUT])
    components = []
    patterns = []
    examples = []
    seen_slugs: set[str] = set()
    seen_components: set[str] = set()
    for item in entries:
        kind = item.get('kind')
        slug = item.get('slug')
        if not isinstance(slug, str) or not slug:
            raise ValueError('every registry entry needs a stable nonempty slug')
        if slug in seen_slugs:
            raise ValueError(f'duplicate route slug: {slug}')
        seen_slugs.add(slug)
        if kind == 'component':
            component = item.get('component')
            if not isinstance(component, str) or not component:
                raise ValueError(f'{slug}: component id required')
            if component in seen_components:
                raise ValueError(f'duplicate component id: {component}')
            seen_components.add(component)
            api = item.get('api')
            bindings = component_bindings(item)
            source_path, symbol = resolve_api(api)
            review = reviews.get(slug)
            components.append({
                'id': component,
                'route': slug,
                'title': item.get('title'),
                'api': api,
                'source': source_path,
                'source_symbol': {'status': 'declaration-present', 'symbol': symbol, 'public_name': api.rsplit('::', 1)[-1], 'line': api_source(api)['line']},
                'public_api_bindings': {
                    'status': 'compiler-check-required',
                    'test': BINDING_OUTPUT.as_posix(),
                    'paths': bindings,
                    'resolved_sources': {path: api_source(path) for path in bindings},
                    'notes': item.get('api_notes'),
                },
                'stories': {'status': 'registered', 'renderer': item.get('renderer')},
                'maturity': item.get('status', 'unverified'),
                'state_coverage': coverage[component],
                'token_dependencies': contract_token_references(component),
                'contract_code_experience': {
                    'contract': 'specified',
                    'code': review.get('status', 'unverified') if review else 'unverified',
                    'experience': (review.get('experience') if review and isinstance(review.get('experience'), str) else 'unverified'),
                    'evidence': review.get('evidence') if review else None,
                },
            })
        elif kind == 'pattern':
            patterns.append({'route': slug, 'title': item.get('title'), 'status': item.get('status', 'unverified')})
        elif kind == 'example':
            examples.append({'route': slug, 'title': item.get('title'), 'status': item.get('status', 'unverified')})
    historical_path = HISTORICAL_REGISTRY
    historical = read_json(historical_path)['components']
    mapping = historical_mapping(registry, historical)
    reverse = reverse_coverage(registry)
    observation_paths = [Path(row['runtime_token_observation']) for row in read_json(COVERAGE)['components'] if row.get('runtime_token_observation')]
    input_paths = [*observation_paths, REGISTRY, REVIEWS, COVERAGE, STATE_PROFILES, COMPONENT_CONTRACTS, TOKENS, PUBLIC_SURFACE, HISTORICAL_MAP, historical_path, Path('scripts/generate-component-capabilities.py'), Path('scripts/component-context.py'), Path('design/integration.json'), Path('design/gallery/navigation.json'), *files]
    return {
        'format': 'tekesui-component-capabilities/1',
        'source_state': 'content-addressed by input SHA-256 hashes; Git HEAD is intentionally excluded',
        'inputs_sha256': {p.as_posix(): digest(binding_override if p == BINDING_OUTPUT and binding_override is not None else (ROOT / p).read_bytes()) for p in sorted(set(input_paths))},
        'counts': {'components': len(components), 'patterns': len(patterns), 'examples': len(examples)},
        'historical_registry': {
            'source': historical_path.as_posix(),
            'component_ids': sorted(item['id'] for item in historical),
            'current_mapping': 'explicit identity/partial-overlap map; does not assert implementation parity',
            'identity_mapping': mapping,
            'mapping_source': HISTORICAL_MAP.as_posix(),
        },
        'coverage_limits': [
            'Generated public-path integration tests must compile; source resolution alone does not prove public API or behavior.',
            'Contract token references/aliases are indexed; actual per-component runtime dependencies remain unverified.',
            'Experience remains unverified unless linked evidence records an applicable real review.',
            'Explicit historical identity/partial-overlap mappings do not assert feature parity or experience acceptance.',
            'Required design states are accounted with explicit fixture gaps; runtime/state combinations, complete token dependencies and native acceptance remain unverified.',
        ],
        'public_type_surface': reverse,
        'components': components,
        'patterns': patterns,
        'examples': examples,
    }


def validate(data: dict) -> None:
    if data.get('format') != 'tekesui-component-capabilities/1':
        raise ValueError('unsupported capabilities format')
    components = data.get('components')
    if not isinstance(components, list):
        raise ValueError('components must be an array')
    ids = [item.get('id') for item in components]
    routes = [item.get('route') for item in components]
    if len(ids) != len(set(ids)):
        raise ValueError('duplicate component id')
    if len(routes) != len(set(routes)) or any(not r for r in routes):
        raise ValueError('missing or duplicate component route')
    if data.get('public_type_surface') != reverse_coverage(read_json(REGISTRY)):
        raise ValueError('public type classification/source coverage mismatch')
    coverage = component_coverage(read_json(REGISTRY))
    known = {p.as_posix() for p in source_files()}
    for item in components:
        if item.get('state_coverage') != coverage.get(item.get('id')) or item.get('token_dependencies') != contract_token_references(item['id']):
            raise ValueError(f"state/token coverage mismatch for {item.get('id')}")
        api = item.get('api', '')
        if item.get('source') is not None and item.get('source') not in known:
            raise ValueError(f"unknown source file for {item.get('id')}")
        source, symbol = resolve_api(api)
        if item.get('source_symbol', {}).get('symbol') != symbol or item.get('source') != source:
            raise ValueError(f"API symbol/source mismatch for {item.get('id')}")
        bindings = item.get('public_api_bindings', {})
        if bindings.get('status') != 'compiler-check-required' or bindings.get('test') != BINDING_OUTPUT.as_posix():
            raise ValueError(f"missing compile binding for {item.get('id')}")
        paths = bindings.get('paths', [])
        if not paths or paths[0] != api or len(paths) != len(set(paths)):
            raise ValueError(f"missing/duplicate compile paths for {item.get('id')}")
        resolved = {path: api_source(path) for path in paths}
        if resolved != bindings.get('resolved_sources'):
            raise ValueError(f"compile path/source mismatch for {item.get('id')}")
        if item.get('maturity') != 'experimental':
            raise ValueError(f"unexpected maturity claim for {item.get('id')}")
        if item.get('contract_code_experience', {}).get('experience') not in {'unverified', 'pass', 'fail', 'not-applicable'}:
            raise ValueError(f"unknown experience state for {item.get('id')}")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument('--check', action='store_true', help='verify the checked-in capability inventory without writing')
    mode.add_argument('--context', action='store_true', help='print current, fresh capability JSON without writing')
    parser.add_argument('--component', help='with --context, select one exact stable component ID or Gallery route')
    args = parser.parse_args()
    if args.component is not None and not args.context:
        parser.error('--component requires --context')
    read_only = args.check or args.context
    binding = binding_test(read_json(REGISTRY)).encode()
    data = generate(binding)
    validate(data)
    if read_only:
        if not BINDING_OUTPUT.is_file() or BINDING_OUTPUT.read_bytes() != binding:
            print('Stale public API binding test: run scripts/generate-component-capabilities.py', file=sys.stderr)
            return 1
    elif not BINDING_OUTPUT.is_file() or BINDING_OUTPUT.read_bytes() != binding:
        BINDING_OUTPUT.write_bytes(binding)
    expected = (json.dumps(data, indent=2, ensure_ascii=False, sort_keys=True) + '\n').encode()
    if read_only:
        inventory_matches = OUTPUT.is_file() and OUTPUT.read_bytes() == expected
        if args.check and not inventory_matches:
            print('Stale capability inventory: run scripts/generate-component-capabilities.py', file=sys.stderr)
            return 1
        if args.context:
            if args.component is not None:
                selected = [item for item in data['components'] if args.component in (item['id'], item['route'])]
                if len(selected) != 1:
                    print('Unknown current component: ' + args.component, file=sys.stderr)
                    return 1
                data = {'format': 'tekesui-component-context/1', 'inventory': OUTPUT.as_posix(),
                        'inventory_sha256': digest(expected), 'checked_in_inventory_matches': inventory_matches,
                        'projection_status': 'current-input-projection' if inventory_matches else 'current-input-projection-differs-from-checked-in',
                        'component': selected[0], 'coverage_limits': data['coverage_limits']}
            elif not inventory_matches:
                data['context_projection'] = {
                    'status': 'current-input-projection-differs-from-checked-in',
                    'generated_projection_sha256': digest(expected),
                    'checked_in_projection_sha256': digest(OUTPUT.read_bytes()) if OUTPUT.is_file() else None,
                    'persisted': False,
                }
            print(json.dumps(data, indent=2, ensure_ascii=False, sort_keys=True))
        else:
            print('Current capability inventory and public binding test match registry, history, evidence and Rust inputs; run cargo test to compile bindings.')
        return 0
    OUTPUT.write_bytes(expected)
    print(f"Generated {OUTPUT}: {data['counts']}")
    return 0


if __name__ == '__main__':
    try:
        raise SystemExit(main())
    except (OSError, ValueError, KeyError, TypeError) as error:
        print('CAPABILITY_INVENTORY_ERROR:', error, file=sys.stderr)
        raise SystemExit(1)
