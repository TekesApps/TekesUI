#!/usr/bin/env python3
"""Run bounded public component token-lookup audits across shared controls and consumers.

Consolidates the 22 per-family audit scripts into one parameterized runner.
Historical evidence files are never overwritten; use a fresh --output path.
This is an explicit development audit, not a benchmark or complete dependency extractor.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[1]
REGISTRY_COMPONENT_REF = (
    'design/imports/v0.3.0-proposal1/upstream/'
    'TekesUI-Design-System-v0.3.0-proposal1/design/registry/components.json'
)


def _hash_paths(paths, root=ROOT):
    digests = {}
    for rel in sorted(set(paths)):
        path = root / rel
        if not path.is_file():
            raise FileNotFoundError(f'missing audit input: {rel}')
        digests[rel] = hashlib.sha256(path.read_bytes()).hexdigest()
    return digests


def _common_paths(root=ROOT):
    paths = set()
    for prefix in [
        'crates/tekes-ui-controls/src',
        'crates/tekes-ui-tokens/src',
        'crates/tekes-ui-design-export/src',
    ]:
        paths.update(p.relative_to(root).as_posix() for p in (root / prefix).rglob('*.rs'))
    for prefix in ['design/source', 'design/brand']:
        paths.update(p.relative_to(root).as_posix() for p in (root / prefix).rglob('*.json'))
    paths.update([
        'design/tokens.json',
        'crates/tekes-ui-controls/Cargo.toml',
        'crates/tekes-ui-tokens/Cargo.toml',
        'scripts/audit-component-tokens.py',
    ])
    return paths


def _consumer_src_paths(root=ROOT):
    return [
        p.relative_to(root).as_posix()
        for p in (root / 'examples/composition-consumer/src').rglob('*.rs')
    ]


def _host_paths(host_dir: Path, root=ROOT):
    resolved = host_dir if host_dir.is_absolute() else (root / host_dir)
    if not resolved.is_dir():
        return []
    result = []
    for p in resolved.rglob('*'):
        if p.is_file() and p.suffix in ['.rs', '.toml', '.lock']:
            try:
                result.append(p.relative_to(root).as_posix())
            except ValueError:
                pass
    return result


def _require_host(spec, host_override: Path | None, root=ROOT) -> Path:
    host = host_override if host_override is not None else spec['default_host']
    resolved = host if host.is_absolute() else (root / host)
    if not (resolved / 'Cargo.toml').is_file():
        raise FileNotFoundError(
            f"Suite '{spec['id']}' targets removed historical review host "
            f"'{spec['default_host']}'; pass --host <path> to run against an explicit host crate."
        )
    return host


def suite_input_paths(suite_id: str, host_override: Path | None = None, root=ROOT):
    spec = SUITES[suite_id]
    paths = set()
    if spec.get('base'):
        paths.update(suite_input_paths(spec['base'], host_override=host_override, root=root))
    else:
        paths.update(_common_paths(root))
        for inherited in spec.get('inherit_inputs', []):
            paths.update(suite_input_paths(inherited, host_override=host_override, root=root))
    for rel in spec.get('extra_files', []):
        paths.add(rel)
    if spec.get('include_registry') and (root / REGISTRY_COMPONENT_REF).is_file():
        paths.add(REGISTRY_COMPONENT_REF)
    if spec.get('include_consumer_src'):
        paths.update(_consumer_src_paths(root))
    if spec.get('include_root_lock'):
        paths.add('Cargo.lock')
    if 'default_host' in spec:
        host = _require_host(spec, host_override, root=root)
        paths.update(_host_paths(host, root=root))
        for rel in spec.get('optional_host_files', []):
            if (root / rel).is_file():
                paths.add(rel)
    return sorted(paths)


def suite_inputs(suite_id: str, host_override: Path | None = None, root=ROOT):
    return _hash_paths(suite_input_paths(suite_id, host_override=host_override, root=root), root=root)


def _check_action(data, _extra):
    assert data['complete'] is False and len(data['cases']) == 576
    assert {c['component'] for c in data['cases']} == {
        'tui.button',
        'tui.icon-button',
        'tui.link',
        'tui.checkbox',
        'tui.segmented-control',
    }


def _check_navigation(data, _extra):
    assert data['complete'] is False and len(data['cases']) == 288
    assert {c['component'] for c in data['cases']} == {
        'tui.disclosure',
        'tui.breadcrumb',
        'tui.tabs',
    }


def _check_input(data, _extra):
    assert data['complete'] is False and len(data['cases']) == 612
    assert {c['component'] for c in data['cases']} == {
        'tui.text-field',
        'tui.text-area',
        'tui.search-field',
    }


def _check_selection(data, _extra):
    assert data['complete'] is False and len(data['cases']) == 2016
    assert {c['component'] for c in data['cases']} == {
        'tui.checkbox',
        'tui.radio-group',
        'tui.switch',
    }


def _check_choice_selection(data, _extra):
    assert len(data['cases']) == 204 and all(c['state_asserted'] is True for c in data['cases'])
    assert len([c for c in data['cases'] if c['component'] == 'tui.select']) == 108
    assert len([c for c in data['cases'] if c['component'] == 'tui.combo-box']) == 96


def _check_choice_trigger(data, _extra):
    assert len(data['cases']) == 84
    assert all(c['state_asserted'] is True and c['part'] == 'trigger' for c in data['cases'])
    assert {c['action_state'] for c in data['cases']} == {
        'rest',
        'hover',
        'pressed',
        'focus',
        'disabled',
    }


def _check_layout_surface(data, _extra):
    assert data['complete'] is False and len(data['cases']) == 192
    assert sum(c['component'] == 'tui.stack' for c in data['cases']) == 96
    assert all(
        c['state_asserted']
        and (
            bool(c['lookups'])
            if c['component'] == 'tui.surface'
            else not c['lookups'] and c['no_dynamic_lookups'] and c['zero_lookup_reason']
        )
        for c in data['cases']
    )


def _check_menu_content(data, _extra):
    assert len(data['cases']) == 288 and data['complete'] is False
    assert all(c['state_asserted'] and c['lookups'] for c in data['cases'])


def _check_passive_feedback(data, _extra):
    assert data['complete'] is False and len(data['cases']) == 216
    assert all(c['state_asserted'] and c['lookups'] for c in data['cases'])


def _check_popover_content(data, _extra):
    assert len(data['cases']) == 384 and data['complete'] is False
    assert all(c['state_asserted'] and c['lookups'] for c in data['cases'])


def _check_popup(data, _extra):
    assert len(data['cases']) == 192 and data['complete'] is False
    assert all(c['state_asserted'] is False for c in data['cases'])


def _check_slider(data, _extra):
    assert len(data['cases']) == 216
    assert all(c['state_asserted'] is True for c in data['cases'])


def _check_tooltip(data, _extra):
    assert len(data['cases']) == 240 and data['complete'] is False
    assert all(
        c['state_asserted'] and bool(c['lookups']) != c['no_dynamic_lookups']
        for c in data['cases']
    )


def _check_collection_focus(data, _extra):
    assert len(data['cases']) == 504 and data['complete'] is False
    assert all(c['state_asserted'] and c['lookups'] for c in data['cases'])


def _check_grid_layout(data, _extra):
    assert data['complete'] is False and len(data['cases']) == 96
    assert all(
        c['state_asserted']
        and not c['lookups']
        and c['no_dynamic_lookups']
        and c['zero_lookup_reason']
        for c in data['cases']
    )


def _check_separator(data, _extra):
    assert data['complete'] is False and len(data['cases']) == 144
    assert all(
        c['state_asserted'] and c['lookups'] and c['component'] == 'tui.separator'
        for c in data['cases']
    )


def _merge_command_results(base_data, extra):
    assert len(extra['cases']) == 36 and all(c['state_asserted'] is True for c in extra['cases'])
    assert {c['choice_state'] for c in extra['cases']} == {'selected', 'open', 'empty'}
    base_data['cases'] += extra['cases']
    assert len(base_data['cases']) == 120
    return base_data


def _merge_command_async(base_data, extra):
    assert len(extra['cases']) == 48 and all(c['state_asserted'] is True for c in extra['cases'])
    base_data['cases'] += extra['cases']
    assert len(base_data['cases']) == 168
    return base_data


def _merge_select_async(base_data, extra):
    assert len(extra['cases']) == 24 and all(c['state_asserted'] is True for c in extra['cases'])
    assert {c['choice_state'] for c in extra['cases']} == {'loading', 'error'}
    for case in extra['cases']:
        case['context'] = 'caller background refresh with cached options; not input validation'
    base_data['cases'] += extra['cases']
    assert len(base_data['cases']) == 228
    return base_data


def _merge_combo_async(base_data, extra):
    assert len(extra['cases']) == 36 and all(c['state_asserted'] is True for c in extra['cases'])
    base_data['cases'] += extra['cases']
    assert len(base_data['cases']) == 264
    return base_data


def _merge_content_status(base_data, extra):
    assert len(base_data['cases']) == 288 and len(extra['cases']) == 120
    excluded = list(
        dict.fromkeys(
            [x for x in base_data['excluded'] if x != 'content loading/error/empty contracts']
            + extra['excluded']
        )
    )
    return {
        'format': base_data['format'],
        'complete': False,
        'scope': (
            base_data['scope'].replace(
                '10 content-status requirements/combinations remain deferred, '
                'not reinterpreted as header color flags.',
                '',
            )
            + ' '
            + extra['scope']
        ),
        'cases': base_data['cases'] + extra['cases'],
        'excluded': excluded,
        'header_only_deferred': base_data['deferred'],
        'content_status_coverage': 'caller-owned public composition verified; no new builtin adapter or broad migration',
    }


def _merge_slider_composition(base_data, extra):
    assert len(extra['cases']) == 48 and all(c['state_asserted'] is True for c in extra['cases'])
    base_data['cases'] += extra['cases']
    assert len(base_data['cases']) == 264
    return base_data


SUITES = {
    'action': {
        'id': 'action',
        'description': 'Public action family lookups (576 bounded cases)',
        'extra_files': ['crates/tekes-ui-controls/tests/action_contract.rs'],
        'include_registry': True,
        'command': [
            'cargo', 'test', '-p', 'tekes-ui-controls', '--features', 'token-audit',
            '--test', 'action_contract', '--locked',
        ],
        'env_var': 'TEKESUI_ACTION_AUDIT_OUT',
        'public_api_test': 'crates/tekes-ui-controls/tests/action_contract.rs',
        'validator': _check_action,
        'metadata': {
            'assertions': [
                'all 31 required states and 17 registered combinations via public render/event APIs',
                'stable frame, controlled value and accessible icon name',
                'disabled/loading block activation',
                'Link Space remains unconsumed',
                'latest model flags before pointer/keyboard activation',
                'observed disabled interval cancels stale Space arming',
            ],
        },
        'summary': '576 bounded cases; complete=False; native acceptance unchanged',
    },
    'navigation': {
        'id': 'navigation',
        'description': 'Public Disclosure, Breadcrumb & Tabs probes (288 bounded cases)',
        'extra_files': ['crates/tekes-ui-controls/tests/navigation_contract.rs'],
        'include_registry': True,
        'command': [
            'cargo', 'test', '-p', 'tekes-ui-controls', '--features', 'token-audit',
            '--test', 'navigation_contract', '--locked',
        ],
        'env_var': 'TEKESUI_NAVIGATION_AUDIT_OUT',
        'public_api_test': 'crates/tekes-ui-controls/tests/navigation_contract.rs',
        'validator': _check_navigation,
        'metadata': {
            'assertions': [
                '17 header states and7 combinations via production public APIs',
                'latest Disclosure availability cancels Space arm',
                'Disclosure radius-row lookup',
                'Breadcrumb overall availability preserves path and Space remains unconsumed',
                'current Breadcrumb item noninteractive; invalid indices rejected',
                'Tabs controlled selection preserved across paints',
            ],
            'native_acceptance': 'not inferred from this probe',
        },
        'summary': '288 bounded cases; complete=False; native acceptance unchanged',
    },
    'input': {
        'id': 'input',
        'description': 'Public TextField, TextArea & SearchField lookups (612 bounded cases)',
        'extra_files': ['crates/tekes-ui-controls/tests/input_contract.rs'],
        'command': [
            'cargo', 'test', '-p', 'tekes-ui-controls', '--features', 'token-audit',
            '--test', 'input_contract', '--locked',
        ],
        'env_var': 'TEKESUI_INPUT_AUDIT_OUT',
        'public_api_test': 'crates/tekes-ui-controls/tests/input_contract.rs',
        'validator': _check_input,
        'metadata': {
            'assertions': [
                'existing input/error boundary roles',
                'stable frame and preserved draft/selection',
                'disabled/read-only reject edits',
                'disabled/read-only caret suppression',
                'UTF-16 grapheme selection and native-empty preedit cancellation',
                'IME commit and undo',
                'existing pointer selection and capture cancellation',
            ],
        },
        'summary': '612 bounded cases; complete=False; native acceptance unchanged',
    },
    'selection': {
        'id': 'selection',
        'description': 'Public Checkbox, RadioGroup & Switch lookups (2016 bounded cases)',
        'extra_files': ['crates/tekes-ui-controls/tests/selection_contract.rs'],
        'command': [
            'cargo', 'test', '-p', 'tekes-ui-controls', '--features', 'token-audit',
            '--test', 'selection_contract', '--locked',
        ],
        'env_var': 'TEKESUI_SELECTION_AUDIT_OUT',
        'public_api_test': 'crates/tekes-ui-controls/tests/selection_contract.rs',
        'validator': _check_selection,
        'metadata': {
            'assertions': [
                'existing semantic boundary role',
                'stable layout and hit frame',
                'selected value preserved by render',
                'focus ring independent of error and disabled',
            ],
        },
        'summary': '2016 bounded cases; complete=False; native acceptance unchanged',
    },
    'choice-selection': {
        'id': 'choice-selection',
        'description': 'Select & ComboBox View state probes (204 bounded cases)',
        'inherit_inputs': ['navigation'],
        'extra_files': ['crates/tekes-ui-controls/tests/choice_selection_states.rs'],
        'command': [
            'cargo', 'test', '-p', 'tekes-ui-controls', '--features', 'token-audit',
            '--test', 'choice_selection_states', '--locked',
        ],
        'env_var': 'TEKESUI_CHOICE_SELECTION_AUDIT_OUT',
        'public_api_test': 'crates/tekes-ui-controls/tests/choice_selection_states.rs',
        'validator': _check_choice_selection,
        'metadata': {
            'scope': (
                'Select9 exact production View states; ComboBox8 caller-owned TextSurface '
                'and original ComboBoxView states; four themes/three brands; '
                'CoreText layout and no-op glyph painter'
            ),
            'excluded': [
                'Select loading/caller async failure; ComboBox loading/error composition',
                'complete state combinations/dependency extraction',
                'native AX/VoiceOver/physical IME',
            ],
        },
        'summary': '204 exact public choice cases; complete=False; native/experience separate',
    },
    'choice-trigger': {
        'id': 'choice-trigger',
        'description': 'CommandPalette trigger action states (84 bounded cases)',
        'inherit_inputs': ['navigation'],
        'extra_files': ['crates/tekes-ui-controls/tests/choice_trigger_states.rs'],
        'command': [
            'cargo', 'test', '-p', 'tekes-ui-controls', '--features', 'token-audit',
            '--test', 'choice_trigger_states', '--locked',
        ],
        'env_var': 'TEKESUI_CHOICE_TRIGGER_AUDIT_OUT',
        'public_api_test': 'crates/tekes-ui-controls/tests/choice_trigger_states.rs',
        'validator': _check_choice_trigger,
        'metadata': {
            'scope': 'CommandPalette trigger only; five exact action states and two pointer cancellation cases; four themes/three brands',
            'excluded': [
                'query surface and all candidate states',
                'loading/error/selected/open/empty',
                'native AX/VoiceOver',
                'complete dependency extraction',
            ],
        },
        'summary': '84 trigger cases; complete=False; popup/query states excluded',
    },
    'command-results': {
        'id': 'command-results',
        'description': 'CommandPalette trigger + result states (120 bounded cases)',
        'base': 'choice-trigger',
        'extra_files': ['crates/tekes-ui-controls/tests/command_result_states.rs'],
        'command': [
            'cargo', 'test', '-p', 'tekes-ui-controls', '--features', 'token-audit',
            '--test', 'command_result_states', '--locked',
        ],
        'env_var': 'TEKESUI_COMMAND_RESULTS_AUDIT_OUT',
        'public_api_test': 'crates/tekes-ui-controls/tests/command_result_states.rs',
        'merger': _merge_command_results,
        'metadata': {
            'scope': 'Original CommandPaletteView five trigger states and three result states; caller TextSurface; four themes/three brands; real repeated activation; CoreText layout with no-op glyph painter',
            'excluded': [
                'loading/error composition',
                'complete combinations/dependency extraction',
                'native AX/VoiceOver/physical IME',
            ],
        },
        'summary': '120 bounded trigger/result cases; complete=False',
    },
    'command-async': {
        'id': 'command-async',
        'description': 'CommandPalette async caller composition (168 bounded cases)',
        'base': 'command-results',
        'include_consumer_src': True,
        'extra_files': ['examples/composition-consumer/tests/command_status.rs'],
        'command': [
            'cargo', 'test', '-p', 'tekes-ui-composition-consumer', '--features', 'token-audit',
            '--test', 'command_status', '--locked',
        ],
        'env_var': 'TEKESUI_COMMAND_ASYNC_AUDIT_OUT',
        'public_api_test': 'examples/composition-consumer/tests/command_status.rs',
        'merger': _merge_command_async,
        'metadata': {
            'scope': 'Original public CommandPaletteView trigger/results plus retained and initial-empty async caller; original TextFieldContent/Status/Feedback; four themes/three brands; existing host IME/capture routing; CoreText/no-op glyph painter',
            'excluded': [
                'complete state combinations/dependency extraction',
                'native live async transitions/physicalIME/controlsAX/VoiceOver',
                'complete motion',
            ],
        },
        'summary': '168 bounded command cases; complete=False',
    },
    'select-async': {
        'id': 'select-async',
        'description': 'Select/ComboBox + async background refresh (228 bounded cases)',
        'base': 'choice-selection',
        'include_consumer_src': True,
        'include_root_lock': True,
        'extra_files': [
            'examples/composition-consumer/tests/choice_status.rs',
            'examples/composition-consumer/Cargo.toml',
        ],
        'command': [
            'cargo', 'test', '-p', 'tekes-ui-composition-consumer', '--features', 'token-audit',
            '--test', 'choice_status', '--locked',
        ],
        'env_var': 'TEKESUI_SELECT_ASYNC_AUDIT_OUT',
        'public_api_test': 'examples/composition-consumer/tests/choice_status.rs',
        'merger': _merge_select_async,
        'metadata': {
            'scope': 'Select9/ComboBox8 production View states plus independent Select background refresh loading/failure: ContentHost/Stack/Progress/Banner, cached choice/focus/candidate preservation and separate validation semantics; four themes/three brands; CoreText with no-op glyph painter',
            'excluded': [
                'ComboBox/CommandPalette async composition',
                'complete state combinations/dependency extraction',
                'native AX/VoiceOver/physical IME',
                'native live async interaction and complete motion',
            ],
        },
        'summary': '228 exact bounded choice/Select-refresh cases; complete=False',
    },
    'combo-async': {
        'id': 'combo-async',
        'description': 'Retained ComboBox query, IME & validation composition (264 bounded cases)',
        'base': 'select-async',
        'extra_files': ['examples/composition-consumer/tests/combo_status.rs'],
        'command': [
            'cargo', 'test', '-p', 'tekes-ui-composition-consumer', '--features', 'token-audit',
            '--test', 'combo_status', '--locked',
        ],
        'env_var': 'TEKESUI_COMBO_ASYNC_AUDIT_OUT',
        'public_api_test': 'examples/composition-consumer/tests/combo_status.rs',
        'merger': _merge_combo_async,
        'metadata': {
            'scope': 'Existing production choice states plus independent Select refresh and ComboBox retained TextFieldContent/ComboBoxView refresh/validation; four themes/three brands; ContentHost IME-before-overlay arbitration and marked response rejection; CoreText with no-op glyph painter',
            'excluded': [
                'CommandPalette async composition',
                'complete state combinations/dependency extraction',
                'native live async transitions/physicalIME/AX/VoiceOver',
                'complete motion',
            ],
        },
        'summary': '264 bounded choice and retained editable refresh cases; complete=False',
    },
    'content-status': {
        'id': 'content-status',
        'description': 'Navigation header + caller-owned content status (408 bounded cases)',
        'base': 'navigation',
        'include_consumer_src': True,
        'extra_files': [
            'examples/composition-consumer/Cargo.toml',
            'examples/composition-consumer/tests/scenarios.rs',
        ],
        'command': [
            'cargo', 'test', '-p', 'tekes-ui-composition-consumer', '--features', 'token-audit',
            '--locked', '--test', 'scenarios', 'content_status',
        ],
        'env_var': 'TEKESUI_CONTENT_STATUS_AUDIT_OUT',
        'public_api_test': 'examples/composition-consumer/tests/scenarios.rs',
        'merger': _merge_content_status,
        'metadata': {
            'header_only_deferred': None,  # preserved from _merge_content_status output
            'content_status_coverage': 'caller-owned public composition verified; no new builtin adapter or broad migration',
            'native_acceptance': 'not inferred from API probes',
        },
        'summary': '408 bounded cases; complete=False; native acceptance separate',
    },
    'layout-surface': {
        'id': 'layout-surface',
        'description': 'Stack geometry vs Surface paint lookups (192 bounded cases)',
        'inherit_inputs': ['navigation'],
        'include_root_lock': True,
        'extra_files': ['crates/tekes-ui-controls/tests/layout_surface_tokens.rs'],
        'command': [
            'cargo', 'test', '--locked', '-p', 'tekes-ui-controls', '--features', 'token-audit',
            '--test', 'layout_surface_tokens',
        ],
        'env_var': 'TEKESUI_LAYOUT_SURFACE_AUDIT_OUT',
        'public_api_test': 'crates/tekes-ui-controls/tests/layout_surface_tokens.rs',
        'validator': _check_layout_surface,
        'metadata': {'native_acceptance': 'separate/unverified'},
        'summary': '192 actual scoped observations; geometry and paint dependencies distinct; complete=False',
    },
    'menu-content': {
        'id': 'menu-content',
        'description': 'MenuContent production lookups through consumer (288 bounded cases)',
        'inherit_inputs': ['navigation'],
        'include_consumer_src': True,
        'include_root_lock': True,
        'extra_files': [
            'examples/composition-consumer/tests/menu_content_tokens.rs',
            'examples/composition-consumer/Cargo.toml',
        ],
        'command': [
            'cargo', 'test', '-p', 'tekes-ui-composition-consumer', '--features', 'token-audit',
            '--test', 'menu_content_tokens', '--locked',
        ],
        'env_var': 'TEKESUI_MENU_CONTENT_AUDIT_OUT',
        'public_api_test': 'examples/composition-consumer/tests/menu_content_tokens.rs',
        'validator': _check_menu_content,
        'metadata': {'native_acceptance': 'separate'},
        'summary': '288 bounded actual MenuContent observations; complete=False',
    },
    'passive-feedback': {
        'id': 'passive-feedback',
        'description': 'Progress, StatusBadge & Skeleton paint contracts (216 bounded cases)',
        'inherit_inputs': ['navigation'],
        'include_root_lock': True,
        'extra_files': ['crates/tekes-ui-controls/tests/passive_feedback_contract.rs'],
        'command': [
            'cargo', 'test', '--locked', '-p', 'tekes-ui-controls', '--features', 'token-audit',
            '--test', 'passive_feedback_contract',
        ],
        'env_var': 'TEKESUI_PASSIVE_FEEDBACK_AUDIT_OUT',
        'public_api_test': 'crates/tekes-ui-controls/tests/passive_feedback_contract.rs',
        'validator': _check_passive_feedback,
        'metadata': {'native_acceptance': 'separate/unverified'},
        'summary': '216 actual public-view observations; complete=False',
    },
    'popover-content': {
        'id': 'popover-content',
        'description': 'Popover shell and owned subtree lookups (384 bounded cases)',
        'inherit_inputs': ['navigation'],
        'include_consumer_src': True,
        'include_root_lock': True,
        'extra_files': [
            'examples/composition-consumer/tests/popover_tokens.rs',
            'examples/composition-consumer/Cargo.toml',
        ],
        'command': [
            'cargo', 'test', '-p', 'tekes-ui-composition-consumer', '--features', 'token-audit',
            '--test', 'popover_tokens', '--locked',
        ],
        'env_var': 'TEKESUI_POPOVER_AUDIT_OUT',
        'public_api_test': 'examples/composition-consumer/tests/popover_tokens.rs',
        'validator': _check_popover_content,
        'metadata': {'native_acceptance': 'separate'},
        'summary': '384 bounded separate shell/subtree observations; complete=False',
    },
    'popup': {
        'id': 'popup',
        'description': 'Popup placement geometry lookups (192 bounded cases)',
        'inherit_inputs': ['navigation'],
        'extra_files': ['crates/tekes-ui-controls/tests/popup_placement.rs'],
        'command': [
            'cargo', 'test', '-p', 'tekes-ui-controls', '--features', 'token-audit',
            '--test', 'popup_placement', 'production_popup_geometry', '--locked',
        ],
        'env_var': 'TEKESUI_POPUP_AUDIT_OUT',
        'public_api_test': 'crates/tekes-ui-controls/tests/popup_placement.rs',
        'validator': _check_popup,
        'metadata': {'native_acceptance': 'separate'},
        'summary': '192 geometry cases; complete=False; required states not promoted',
    },
    'slider': {
        'id': 'slider',
        'description': 'SliderView state/geometry/event probes (216 bounded cases)',
        'inherit_inputs': ['selection'],
        'extra_files': [
            'examples/composition-consumer/tests/slider_states.rs',
            'examples/composition-consumer/Cargo.toml',
            'crates/tekes-ui-controls/tests/selection_availability.rs',
        ],
        'command': [
            'cargo', 'test', '-p', 'tekes-ui-composition-consumer', '--features', 'token-audit',
            '--test', 'slider_states', '--locked',
        ],
        'env_var': 'TEKESUI_SLIDER_AUDIT_OUT',
        'public_api_test': 'examples/composition-consumer/tests/slider_states.rs',
        'validator': _check_slider,
        'metadata': {
            'scope': 'Original public SliderView six states; real pointer/key/drag/release, values0/50/100, four themes/three brands, Scene primitive geometry/color; no native inference',
            'excluded': [
                'complete state combinations/dependency extraction',
                'native AX/VoiceOver',
                'performance',
                'complete native interaction acceptance',
            ],
        },
        'summary': '216 bounded slider cases; complete=False',
    },
    'slider-composition': {
        'id': 'slider-composition',
        'description': 'SliderView + SliderContent composition (264 bounded cases)',
        'base': 'slider',
        'include_consumer_src': True,
        'extra_files': ['examples/composition-consumer/tests/slider_content.rs'],
        'command': [
            'cargo', 'test', '-p', 'tekes-ui-composition-consumer', '--features', 'token-audit',
            '--test', 'slider_content', '--locked',
        ],
        'env_var': 'TEKESUI_SLIDER_COMPOSITION_AUDIT_OUT',
        'public_api_test': 'examples/composition-consumer/tests/slider_content.rs',
        'merger': _merge_slider_composition,
        'metadata': {
            'scope': 'Original SliderView six states plus public SliderContent in independent ContentHost/Stack with TextFieldContent sibling; real ownership/range/actions; four themes/three brands; CoreText/no-op glyph painter; native AX separate',
            'excluded': [
                'native AX role/actions/VoiceOver operation',
                'complete state combinations/dependency extraction',
                'physicalIME/live async/full motion',
                'performance',
            ],
        },
        'summary': '264 bounded slider cases; complete=False',
    },
    'tooltip': {
        'id': 'tooltip',
        'description': 'Tooltip lifecycle & closed zero-read probes (240 bounded cases)',
        'inherit_inputs': ['navigation'],
        'include_root_lock': True,
        'extra_files': [
            'examples/composition-consumer/tests/tooltip_tokens.rs',
            'examples/composition-consumer/Cargo.toml',
            'scripts/generate-component-capabilities.py',
        ],
        'command': [
            'cargo', 'test', '-p', 'tekes-ui-composition-consumer', '--features', 'token-audit',
            '--test', 'tooltip_tokens', '--locked',
        ],
        'env_var': 'TEKESUI_TOOLTIP_AUDIT_OUT',
        'public_api_test': 'examples/composition-consumer/tests/tooltip_tokens.rs',
        'validator': _check_tooltip,
        'metadata': {'native_acceptance': 'separate/unverified'},
        'summary': '240 bounded Tooltip observations; explicit closed zero reads, complete=False',
    },
    'collection-focus': {
        'id': 'collection-focus',
        'description': 'Historical review host collection focus/availability (requires --host)',
        'inherit_inputs': ['navigation'],
        'include_root_lock': True,
        'extra_files': ['crates/tekes-ui-gallery/src/gallery/workbench/extended/collections_adapter.rs'],
        'optional_host_files': [
            'design/reviews/collection-focus-availability-20261020/host/src/fixture.rs',
            'design/reviews/collection-focus-availability-20261020/host/src/text_backend.rs',
        ],
        'default_host': Path('design/reviews/collection-focus-ink-20261021/host'),
        'host_test': 'collection_tokens',
        'env_var': 'TEKESUI_COLLECTION_FOCUS_AUDIT_OUT',
        'validator': _check_collection_focus,
        'metadata': {'native_acceptance': 'separate/unverified'},
        'summary': '504 asserted collection focus/availability observations; complete=False',
    },
    'grid-layout': {
        'id': 'grid-layout',
        'description': 'Historical review host GridView zero-lookup geometry probe (requires --host)',
        'inherit_inputs': ['navigation'],
        'include_root_lock': True,
        'extra_files': [
            'crates/tekes-ui-controls/tests/grid_layout_availability.rs',
            'crates/tekes-ui-gallery/src/gallery/workbench/extended.rs',
        ],
        'optional_host_files': [
            'design/reviews/collection-focus-native-20261021/host/src/text_backend.rs',
        ],
        'default_host': Path('design/reviews/grid-layout-availability-20261021/host'),
        'run_mode': 'grid_bin',
        'validator': _check_grid_layout,
        'metadata': {'native_acceptance': 'separate/unverified'},
        'summary': '96 asserted zero-lookup GridView observations; child paint outside capture; complete=False',
    },
    'separator': {
        'id': 'separator',
        'description': 'Historical review host SeparatorView probe (requires --host)',
        'inherit_inputs': ['navigation'],
        'include_root_lock': True,
        'extra_files': [
            'crates/tekes-ui-controls/tests/separator_scale.rs',
            'crates/tekes-ui-gallery/src/gallery/workbench/extended.rs',
        ],
        'default_host': Path('design/reviews/separator-scale-20261021/host'),
        'host_test': 'tokens',
        'env_var': 'TEKESUI_SEPARATOR_TOKEN_OUT',
        'validator': _check_separator,
        'metadata': {'native_acceptance': 'separate/unverified'},
        'summary': '144 bounded actual Separator observations; complete=False',
    },
}


def execute_suite(suite_id: str, host_override: Path | None = None, current_only: bool = False, root=ROOT):
    spec = SUITES[suite_id]
    before = suite_inputs(suite_id, host_override=host_override, root=root)
    base_data = None
    if spec.get('base'):
        base_data = execute_suite(spec['base'], host_override=host_override, current_only=current_only, root=root)

    if 'default_host' in spec:
        host = _require_host(spec, host_override, root=root)
        manifest_path = str((host if host.is_absolute() else host) / 'Cargo.toml')
        if spec.get('run_mode') == 'grid_bin':
            command = ['cargo', 'run', '--manifest-path', manifest_path, '--features', 'token-audit', '--locked']
            host_parent = (root / host).parent if not host.is_absolute() else host.parent
            env = dict(
                os.environ,
                TEKESUI_GRID_REVIEW_OUT=str(host_parent / 'token-gpu'),
                TEKESUI_GRID_REVIEW_PHASE='after',
            )
            if current_only:
                env['TEKESUI_GRID_REVIEW_CURRENT_ONLY'] = '1'
            subprocess.run(command, cwd=root, env=env, check=True)
            raw_data = json.loads((host_parent / 'grid-token-raw.json').read_text())
            public_api_test = str(host / 'src/main.rs') if not host.is_absolute() else str(host / 'src/main.rs')
        else:
            command = [
                'cargo', 'test', '--manifest-path', manifest_path,
                '--features', 'token-audit', '--test', spec['host_test'], '--locked',
            ]
            with tempfile.TemporaryDirectory(prefix=f'tekesui-{suite_id}-') as folder:
                raw = Path(folder) / 'reads.json'
                subprocess.run(command, cwd=root, env=dict(os.environ, **{spec['env_var']: str(raw)}), check=True)
                raw_data = json.loads(raw.read_text())
            test_rel = spec['host_test'] + '.rs'
            public_api_test = (host / 'tests' / test_rel).as_posix()
    else:
        command = list(spec['command'])
        with tempfile.TemporaryDirectory(prefix=f'tekesui-{suite_id}-') as folder:
            raw = Path(folder) / 'reads.json'
            env = dict(os.environ, **{spec['env_var']: str(raw)})
            subprocess.run(command, cwd=root, env=env, check=True)
            raw_data = json.loads(raw.read_text())
        public_api_test = spec['public_api_test']

    if suite_inputs(suite_id, host_override=host_override, root=root) != before:
        raise RuntimeError(f"source changed during probe '{suite_id}'; no evidence written")

    if base_data is not None:
        data = spec['merger'](base_data, raw_data)
    else:
        data = raw_data
        spec['validator'](data, raw_data)

    meta = {k: v for k, v in spec.get('metadata', {}).items() if v is not None}
    data.update(
        inputs_sha256=before,
        probe_command=command,
        public_api_test=public_api_test,
        **meta,
    )
    return data


def write_evidence(output: Path, data: dict):
    output.parent.mkdir(parents=True, exist_ok=True)
    with output.open('x') as handle:
        json.dump(data, handle, indent=2)
        handle.write('\n')


def active_suite_ids():
    return [sid for sid, spec in SUITES.items() if 'default_host' not in spec]


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--list', action='store_true', help='list all supported audit suites and exit')
    parser.add_argument('--suite', choices=sorted(SUITES), help='single component token audit suite to run')
    parser.add_argument('--output', type=Path, help='new, non-existing JSON path for --suite output')
    parser.add_argument('--all', action='store_true', help='run all active in-repo suites into --output-dir')
    parser.add_argument('--output-dir', type=Path, help='new or non-conflicting directory for --all outputs')
    parser.add_argument('--host', type=Path, help='override host crate directory for historical review-host suites')
    parser.add_argument('--current-only', action='store_true', help='for grid-layout: assert current geometry without historical PNGs')
    args = parser.parse_args(argv)

    if args.list:
        for sid in sorted(SUITES):
            print(f"{sid}\t{SUITES[sid]['description']}")
        return 0

    if args.suite and args.all:
        parser.error('choose either --suite or --all, not both')
    if not args.suite and not args.all:
        parser.error('pass --list, --suite <name> --output <path>, or --all --output-dir <dir>')

    if args.suite:
        if not args.output:
            parser.error('--output is required with --suite')
        if args.output.exists():
            parser.error('output exists; choose a fresh evidence path to protect prior results')
        try:
            data = execute_suite(args.suite, host_override=args.host, current_only=args.current_only)
        except FileNotFoundError as exc:
            raise SystemExit(str(exc))
        write_evidence(args.output, data)
        print(f"Wrote {args.output}: {SUITES[args.suite]['summary']}")
        return 0

    if not args.output_dir:
        parser.error('--output-dir is required with --all')
    args.output_dir.mkdir(parents=True, exist_ok=True)
    targets = [args.output_dir / f'{sid}.json' for sid in active_suite_ids()]
    existing = [p for p in targets if p.exists()]
    if existing:
        parser.error(f'output file already exists: {existing[0]}')
    for sid in active_suite_ids():
        out = args.output_dir / f'{sid}.json'
        data = execute_suite(sid, host_override=args.host, current_only=args.current_only)
        write_evidence(out, data)
        print(f"Wrote {out}: {SUITES[sid]['summary']}")
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
