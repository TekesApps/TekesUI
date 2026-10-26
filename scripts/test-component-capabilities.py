#!/usr/bin/env python3
"""Tests for evidence-bound current capability inventory."""
import copy
import json
import shutil
import subprocess
import sys
import tempfile
from contextlib import contextmanager
import importlib.util
import unittest
from unittest.mock import patch
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('capabilities', ROOT / 'scripts/generate-component-capabilities.py')
capabilities = importlib.util.module_from_spec(spec)
spec.loader.exec_module(capabilities)

class CapabilityTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.data = capabilities.generate()

    @contextmanager
    def context_fixture(self):
        # Exercise the real CLI against pinned source bytes, without changing
        # active WIP or an accepted historical capability inventory.
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            paths = set(self.data['inputs_sha256'])
            for relative in list(paths):
                if relative.endswith('.json'):
                    document = json.loads((ROOT / relative).read_text())
                    if document.get('format') == 'tekesui-runtime-token-observation/1':
                        paths.update(document['inputs_sha256'])
            for relative in paths:
                target = root / relative
                target.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(ROOT / relative, target)
            target = root / capabilities.OUTPUT
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_text(json.dumps(self.data, indent=2, ensure_ascii=False, sort_keys=True) + '\n')
            yield root

    def context_command(self, root, *args):
        integration = json.loads((root / 'design/integration.json').read_text())
        script = (root / 'design' / integration['tools']['context']['$ref']).resolve()
        return subprocess.run([sys.executable, str(script),
                               *args], cwd=root, capture_output=True, text=True)

    def test_context_emits_current_inventory_without_rewriting_it(self):
        with self.context_fixture() as root:
            path = root / capabilities.OUTPUT
            before = (path.read_bytes(), path.stat().st_mtime_ns)
            result = self.context_command(root)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(json.loads(result.stdout), self.data)
            self.assertEqual((path.read_bytes(), path.stat().st_mtime_ns), before)

    def test_context_selects_current_component_by_stable_id_or_route(self):
        with self.context_fixture() as root:
            for query in ['tui.button', 'component-button']:
                result = self.context_command(root, '--component', query)
                self.assertEqual(result.returncode, 0, result.stderr)
                data = json.loads(result.stdout)
                self.assertEqual(data['component']['id'], 'tui.button')
                self.assertEqual(data['component']['route'], 'component-button')
                self.assertEqual(data['component']['maturity'], 'experimental')
                self.assertEqual(data['component']['contract_code_experience']['experience'], 'unverified')
                self.assertEqual(data['inventory'], capabilities.OUTPUT.as_posix())
                self.assertIn('coverage_limits', data)

    def test_context_projects_current_inputs_when_inventory_is_stale_without_rewriting(self):
        for relative, addition in [(capabilities.OUTPUT, '\n '),
                                   (Path('crates/tekes-ui-controls/src/lib.rs'), '\n// changed source\n')]:
            with self.subTest(path=str(relative)), self.context_fixture() as root:
                path = root / relative
                path.write_text(path.read_text() + addition)
                before = {p: (p.read_bytes(), p.stat().st_mtime_ns) for p in
                          [path, root / capabilities.OUTPUT, root / capabilities.BINDING_OUTPUT]}
                result = self.context_command(root)
                self.assertEqual(result.returncode, 0, result.stderr)
                projection = json.loads(result.stdout)['context_projection']
                self.assertEqual(projection['status'], 'current-input-projection-differs-from-checked-in')
                self.assertFalse(projection['persisted'])
                self.assertNotEqual(projection['generated_projection_sha256'], projection['checked_in_projection_sha256'])
                self.assertEqual({p: (p.read_bytes(), p.stat().st_mtime_ns) for p in before}, before)
                check = self.context_command(root, '--check')
                self.assertNotEqual(check.returncode, 0)
                self.assertEqual(check.stdout, '')
        with self.context_fixture() as root:
            path = root / capabilities.BINDING_OUTPUT
            path.write_text(path.read_text() + '\n// stale binding\n')
            before = {p: (p.read_bytes(), p.stat().st_mtime_ns) for p in
                      [path, root / capabilities.OUTPUT]}
            result = self.context_command(root)
            self.assertNotEqual(result.returncode, 0)
            self.assertEqual(result.stdout, '')
            self.assertEqual({p: (p.read_bytes(), p.stat().st_mtime_ns) for p in before}, before)

    def test_context_unknown_component_does_not_fall_back_to_historical_planned_data(self):
        with self.context_fixture() as root:
            result = self.context_command(root, '--component', 'tui.not-a-current-component')
            self.assertNotEqual(result.returncode, 0)
            self.assertEqual(result.stdout, '')
            self.assertIn('Unknown current component', result.stderr)

    def test_range_receipt_requires_exact_state_and_explicit_assertion(self):
        self.assertTrue(capabilities.state_observed('dragging', {'range_state':'dragging','state_asserted':True}))
        for case in [{'range_state':'dragging'}, {'range_state':'dragging','state_asserted':'true'}, {'range_state':'dragging+focus','state_asserted':True}, {'range_state':'pressed','state_asserted':True}]:
            self.assertFalse(capabilities.state_observed('dragging',case))

    def test_action_receipt_requires_exact_state_and_explicit_public_assertion(self):
        valid={'action_state':'focus','state_asserted':True}
        self.assertTrue(capabilities.state_observed('focus',valid))
        for case in [{'action_state':'focus'}, {'action_state':'focus','state_asserted':False}, {'action_state':'focus','state_asserted':'true'}, {'action_state':'loading+focus','state_asserted':True}]:
            self.assertFalse(capabilities.state_observed('focus',case))
        self.assertFalse(capabilities.state_observed('pressed',valid))

    def test_choice_receipt_requires_exact_state_and_explicit_public_assertion(self):
        valid={'choice_state':'selected','state_asserted':True}
        self.assertTrue(capabilities.state_observed('selected',valid))
        self.assertFalse(capabilities.state_observed('open',valid))
        for case in [{'choice_state':'selected'}, {'choice_state':'selected','state_asserted':False}, {'choice_state':'selected','state_asserted':'true'}, {'choice_state':'selected+focus','state_asserted':True}]:
            self.assertFalse(capabilities.state_observed('selected',case))

    def test_navigation_receipt_cannot_promote_deferred_body_states(self):
        valid={'navigation_state':'expanded','state_asserted':True}
        self.assertTrue(capabilities.state_observed('expanded',valid))
        self.assertFalse(capabilities.state_observed('loading',valid))
        self.assertFalse(capabilities.state_observed('focus',{'navigation_state':'loading+focus','state_asserted':True}))
        self.assertFalse(capabilities.state_observed('expanded',{'navigation_state':'expanded'}))

    def test_content_receipt_keeps_body_and_header_combination_claims_exact(self):
        valid={'content_state':'error','state_asserted':True}
        self.assertTrue(capabilities.state_observed('error',valid))
        self.assertFalse(capabilities.state_observed('focus',valid))
        self.assertFalse(capabilities.state_observed('error',{'content_state':'error+focus','state_asserted':True}))
        self.assertFalse(capabilities.state_observed('error',{'content_state':'error','state_asserted':'true'}))

    def test_current_inventory_has_expected_registered_counts(self):
        entries = json.loads((ROOT / 'design/gallery/workbench.json').read_text())['entries']
        expected = {}
        for kind, bucket in [('component', 'components'), ('pattern', 'patterns'), ('example', 'examples')]:
            registered = {entry['slug'] for entry in entries if entry['kind'] == kind}
            expected[bucket] = len(registered)
            self.assertEqual(len(self.data[bucket]), len(registered))
            self.assertEqual({entry['route'] for entry in self.data[bucket]}, registered)
        self.assertEqual(self.data['counts'], expected)
        capabilities.validate(self.data)

    def test_inventory_identity_is_content_addressed_and_deterministic(self):
        first = capabilities.generate()
        second = capabilities.generate()
        self.assertEqual(first, second)
        self.assertEqual(first['source_state'], 'content-addressed by input SHA-256 hashes; Git HEAD is intentionally excluded')
        self.assertNotIn('source_revision', first)

    def test_inputs_are_hashed_and_experience_is_not_inferred(self):
        for path in ['design/gallery/workbench.json', 'design/behavior/gold7-alignment.json',
                     'design/imports/v0.3.0-proposal1/upstream/TekesUI-Design-System-v0.3.0-proposal1/design/registry/components.json']:
            self.assertIn(path, self.data['inputs_sha256'])
        self.assertTrue(all(c['contract_code_experience']['experience'] == 'unverified' for c in self.data['components']))
        self.assertTrue(all(c['maturity'] == 'experimental' for c in self.data['components']))

    def test_duplicate_component_and_route_are_rejected(self):
        for key in ['id', 'route']:
            bad = copy.deepcopy(self.data)
            bad['components'][1][key] = bad['components'][0][key]
            with self.assertRaisesRegex(ValueError, 'duplicate'):
                capabilities.validate(bad)

    def test_unknown_source_and_mismatched_api_symbol_are_rejected(self):
        bad = copy.deepcopy(self.data)
        bad['components'][0]['source'] = 'crates/no-such-crate/src/lib.rs'
        with self.assertRaisesRegex(ValueError, 'unknown source'):
            capabilities.validate(bad)
        bad = copy.deepcopy(self.data)
        bad['components'][0]['source_symbol']['symbol'] = 'ImaginaryComponent'
        with self.assertRaisesRegex(ValueError, 'mismatch'):
            capabilities.validate(bad)

    def test_missing_route_is_rejected(self):
        bad = copy.deepcopy(self.data)
        bad['components'][0]['route'] = ''
        with self.assertRaisesRegex(ValueError, 'missing or duplicate'):
            capabilities.validate(bad)

    def test_public_reexports_and_aliases_resolve_to_the_actual_source(self):
        self.assertEqual(capabilities.resolve_api('tekes_ui_controls::Button'),
                         ('crates/tekes-ui-controls/src/composition/button.rs', 'Button'))
        self.assertEqual(capabilities.resolve_api('tekes_ui_controls::ScrollArea'),
                         ('crates/tekes-ui-controls/src/scroll_view.rs', 'ScrollView'))
        self.assertEqual(capabilities.resolve_api('tekes_ui_controls::layout_view::IconView'),
                         ('crates/tekes-ui-controls/src/layout_view.rs', 'IconView'))

    def test_a_same_named_declaration_does_not_prove_a_public_path(self):
        for api in ['tekes_ui_controls::widgets::ButtonModel',
                    'tekes_ui_controls::extended::Button',
                    'tekes_ui_controls::MissingComponent',
                    'tekes_ui_controls::scroll_view::ScrollView']:
            with self.subTest(api=api), self.assertRaisesRegex(ValueError, 'no public path'):
                capabilities.resolve_api(api)

    def test_every_component_has_explicit_compiled_model_view_bindings(self):
        registry = json.loads((ROOT / capabilities.REGISTRY).read_text())['entries']
        expected = {entry['component']: entry['api_bindings']
                    for entry in registry if entry['kind'] == 'component'}
        actual = {component['id']: component['public_api_bindings']['paths']
                  for component in self.data['components']}
        self.assertEqual(actual, expected)
        self.assertIn('tekes_ui_controls::ToolbarControl', actual['tui.toolbar'])
        for c in self.data['components']:
            self.assertEqual(c['public_api_bindings']['paths'][0], c['api'])
            self.assertEqual(c['public_api_bindings']['status'], 'compiler-check-required')
        bad = copy.deepcopy(self.data)
        bad['components'][0]['public_api_bindings']['paths'] = []
        with self.assertRaisesRegex(ValueError, 'compile paths'):
            capabilities.validate(bad)

    def test_historical_identity_map_explains_40_to_45_without_hiding_a_gap(self):
        mapping = self.data['historical_registry']['identity_mapping']
        self.assertEqual(len(mapping['entries']), 40)
        self.assertEqual(len(mapping['current_additions']), 6)
        self.assertEqual(sum(e['relationship'] == 'same-id' for e in mapping['entries']), 39)
        selection = next(e for e in mapping['entries'] if e['historical'] == 'tui.selection-surface')
        self.assertEqual(selection['relationship'], 'partial-overlap')
        self.assertIn('unimplemented', selection['note'])

    def test_historical_missing_duplicate_or_unexplained_ids_fail_closed(self):
        registry = capabilities.read_json(capabilities.REGISTRY)
        historical = capabilities.read_json(capabilities.HISTORICAL_REGISTRY)['components']
        good = capabilities.read_json(capabilities.HISTORICAL_MAP)
        mutations = []
        bad = copy.deepcopy(good); bad['entries'].pop(); mutations.append(bad)
        bad = copy.deepcopy(good); bad['entries'].append(bad['entries'][0]); mutations.append(bad)
        bad = copy.deepcopy(good); bad['current_additions'].pop(); mutations.append(bad)
        bad = copy.deepcopy(good); bad['entries'][0]['current'] = ['tui.imaginary']; mutations.append(bad)
        for bad in mutations:
            with patch.object(capabilities, 'read_json', return_value=bad), self.assertRaises(ValueError):
                capabilities.historical_mapping(registry, historical)

    def test_missing_or_duplicate_binding_is_rejected_with_the_route(self):
        registry = capabilities.read_json(capabilities.REGISTRY)
        original = next(e for e in registry['entries'] if e['kind'] == 'component')
        for bindings in [[], [original['api'], original['api']]]:
            bad = copy.deepcopy(original); bad['api_bindings'] = bindings
            with self.assertRaisesRegex(ValueError, original['slug']):
                capabilities.component_bindings(bad)

    @contextmanager
    def source_fixture(self):
        # Mutations run against a temporary source tree, never the active WIP.
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            shutil.copytree(ROOT / 'crates/tekes-ui-controls/src', root / 'crates/tekes-ui-controls/src')
            shutil.copy2(ROOT / 'crates/tekes-ui-controls/build.rs', root / 'crates/tekes-ui-controls/build.rs')
            for path in [capabilities.REGISTRY, capabilities.PUBLIC_SURFACE]:
                (root / path).parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(ROOT / path, root / path)
            with patch.object(capabilities, 'ROOT', root):
                yield root

    def test_public_surface_classifies_all_paths_and_preserves_component_count(self):
        surface = self.data['public_type_surface']['types']
        self.assertEqual(len(surface), len(capabilities.public_type_surface()))
        self.assertEqual(len({e['api'] for e in surface}), len(surface))
        self.assertEqual({e['category'] for e in surface},
                         {'component', 'component-support', 'component-alias', 'component-adapter', 'composition', 'shared-support'})
        button = next(e for e in surface if e['api'] == 'tekes_ui_controls::composition::Button')
        self.assertEqual(button['components'], ['tui.button', 'tui.icon-button'])
        self.assertEqual(button['category'], 'component-adapter')
        removed = {'ButtonContent', 'CompoundButton', 'IconButton', 'IconButtonView'}
        self.assertFalse(any(e['api'].split('::')[-1] in removed for e in surface))
        self.assertEqual(self.data['counts']['components'], 45)

    def test_new_public_types_and_reexport_aliases_report_the_exact_missing_path(self):
        for file, declaration, missing in [
            ('lib.rs', 'pub struct UnregisteredControl;', 'tekes_ui_controls::UnregisteredControl'),
            ('layout_view.rs', 'pub struct UnregisteredView;', 'tekes_ui_controls::layout_view::UnregisteredView'),
            ('lib.rs', 'pub use widgets::ButtonModel as AlternateButton;', 'tekes_ui_controls::AlternateButton')]:
            with self.source_fixture() as root:
                path = root / 'crates/tekes-ui-controls/src' / file
                path.write_text(path.read_text() + '\n' + declaration + '\n')
                with self.assertRaisesRegex(ValueError, missing):
                    capabilities.reverse_coverage(capabilities.read_json(capabilities.REGISTRY))

    def test_private_test_nested_and_literal_types_do_not_expand_public_surface(self):
        with self.source_fixture() as root:
            before = capabilities.public_type_surface()
            path = root / 'crates/tekes-ui-controls/src/lib.rs'
            path.write_text(path.read_text() + '''
mod private_support { pub struct Hidden; }
pub(crate) struct CrateOnly;
fn local_helper() { pub struct LocalOnly; }
// pub struct CommentOnly;
/* nested /* comment */ pub struct BlockOnly; */
const RAW: &str = r#"pub struct LiteralOnly; { [ ] }"#;
''')
            self.assertEqual(capabilities.public_type_surface(), before)
            with self.assertRaisesRegex(ValueError, 'no public path'):
                capabilities.resolve_api('tekes_ui_controls::private_support::Hidden')

    def test_unsupported_export_forms_fail_closed(self):
        for declaration in ['pub mod inline { pub struct Missed; }',
                            'pub use widgets::*;', 'pub use {widgets::ButtonModel};',
                            'expose_types!();', 'macro_rules! expose { () => { pub struct Hidden; } }']:
            with self.source_fixture() as root:
                path = root / 'crates/tekes-ui-controls/src/lib.rs'
                path.write_text(path.read_text() + '\n' + declaration + '\n')
                with self.assertRaisesRegex(ValueError, 'unsupported'):
                    capabilities.public_type_surface()

    def test_missing_stale_duplicate_and_helper_laundering_are_rejected(self):
        registry = capabilities.read_json(capabilities.REGISTRY)
        good = capabilities.read_json(capabilities.PUBLIC_SURFACE)
        mutations = []
        bad = copy.deepcopy(good); bad['types'].pop(); mutations.append((bad, 'missing='))
        bad = copy.deepcopy(good); bad['types'][0]['api'] = 'tekes_ui_controls::Private'; mutations.append((bad, 'stale/private='))
        bad = copy.deepcopy(good); bad['types'].append(bad['types'][0]); mutations.append((bad, 'duplicate'))
        bad = copy.deepcopy(good); item = next(e for e in bad['types'] if e['category'] == 'component'); item['category'] = 'shared-support'; item['components'] = []; mutations.append((bad, 'cannot be classified as helper'))
        bad = copy.deepcopy(good); item = next(e for e in bad['types'] if e['category'] == 'component-adapter'); item['components'] = []; mutations.append((bad, 'needs an existing owner'))
        bad = copy.deepcopy(good); item = next(e for e in bad['types'] if e['category'] == 'component-alias'); item['components'] = ['tui.button']; mutations.append((bad, 'alias must reference'))
        bad = copy.deepcopy(good); bad['types'][0]['reason'] = ''; mutations.append((bad, 'reason required'))
        for bad, message in mutations:
            with self.source_fixture() as root:
                (root / capabilities.PUBLIC_SURFACE).write_text(json.dumps(bad))
                with self.assertRaisesRegex(ValueError, message):
                    capabilities.reverse_coverage(registry)

    def test_registered_component_cannot_lose_its_binding_or_owner_route(self):
        registry = capabilities.read_json(capabilities.REGISTRY)
        bad = copy.deepcopy(registry)
        item = next(e for e in bad['entries'] if e.get('component') == 'tui.button')
        item['api_bindings'].pop()
        with self.assertRaisesRegex(ValueError, 'component-button: missing/duplicate public API bindings'):
            capabilities.reverse_coverage(bad)
        bad_route = copy.deepcopy(registry)
        next(e for e in bad_route['entries'] if e.get('component') == 'tui.button')['slug'] = ''
        with self.assertRaisesRegex(ValueError, 'tui.button: missing component route'):
            capabilities.reverse_coverage(bad_route)
        bad = copy.deepcopy(registry)
        bad['entries'] = [e for e in bad['entries'] if e.get('component') != 'tui.button']
        with self.assertRaisesRegex(ValueError, 'unknown/duplicate component identity'):
            capabilities.reverse_coverage(bad)

    def test_generated_surface_cannot_drop_classified_types(self):
        bad = copy.deepcopy(self.data)
        bad['public_type_surface']['types'].pop()
        with self.assertRaisesRegex(ValueError, 'classification/source coverage mismatch'):
            capabilities.validate(bad)

    def test_type_free_generated_include_requires_unchanged_reviewed_generator(self):
        with self.source_fixture() as root:
            path = root / 'crates/tekes-ui-controls/build.rs'
            path.write_text(path.read_text() + '\n// Changed generator requires another review.\n')
            with self.assertRaisesRegex(ValueError, 'type-free generator changed'):
                capabilities.public_type_surface()


    @contextmanager
    def contract_fixture(self):
        with self.source_fixture() as root:
            shutil.copytree(ROOT / 'crates/tekes-ui-gallery/src', root / 'crates/tekes-ui-gallery/src')
            button_test = Path('crates/tekes-ui-controls/tests/composition_button_alignment.rs')
            (root / button_test).parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(ROOT / button_test, root / button_test)
            for path in [capabilities.COVERAGE, capabilities.STATE_PROFILES,
                         capabilities.COMPONENT_CONTRACTS, capabilities.HISTORICAL_REGISTRY,
                         capabilities.TOKENS]:
                (root / path).parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(ROOT / path, root / path)
            for row in capabilities.read_json(capabilities.COVERAGE)['components']:
                artifact = row.get('runtime_token_observation')
                if artifact:
                    paths = [artifact, *json.loads((ROOT / artifact).read_text())['inputs_sha256']]
                    for path in paths:
                        (root / path).parent.mkdir(parents=True, exist_ok=True)
                        shutil.copy2(ROOT / path, root / path)
            yield root

    def test_required_states_are_accounted_without_promoting_fixture_or_native_acceptance(self):
        components = self.data['components']
        self.assertEqual(len(components), 45)
        for component in components:
            coverage = component['state_coverage']
            self.assertEqual(set(coverage['required']), {s['state'] for s in coverage['reviews']})
            self.assertTrue(all(s['runtime'] in {'unverified', 'public-api-verified'} and s['experience'] == 'unverified' for s in coverage['reviews']))
        text = next(c for c in components if c['id'] == 'tui.text-field')
        self.assertEqual(text['state_coverage']['fixture_unverified'], [])
        self.assertEqual(text['state_coverage']['runtime_observation_status'], 'historical-stale')
        self.assertIn('composing', text['state_coverage']['historical_public_api_verified'])
        self.assertNotIn('composing', text['state_coverage']['public_api_verified'])
        checkbox = next(c for c in components if c['id'] == 'tui.checkbox')
        self.assertIn('mixed', checkbox['state_coverage']['required'])

    def test_input_case_names_do_not_promote_hover_pressed_or_disabled_focus(self):
        for name in ['rest', 'focus', 'disabled', 'error', 'read-only', 'composing', 'selection']:
            self.assertTrue(capabilities.state_observed(name, {'input_state': name}))
            for unsupported in ['hover', 'pressed']:
                self.assertFalse(capabilities.state_observed(unsupported, {'input_state': name}))
        self.assertFalse(capabilities.state_observed('focus', {'input_state': 'disabled-focus'}))
        for state, flag in [('hover', 'pointer_hovered'), ('pressed', 'pointer_pressed')]:
            self.assertFalse(capabilities.state_observed(state, {'input_state':state}))
            self.assertFalse(capabilities.state_observed(state, {'input_state':state,flag:False}))
            self.assertTrue(capabilities.state_observed(state, {'input_state':state,flag:True}))


    def test_omitted_required_state_and_new_profile_requirement_fail_precisely(self):
        with self.contract_fixture() as root:
            data = capabilities.read_json(capabilities.COVERAGE)
            item = next(c for c in data['components'] if c['id'] == 'tui.text-field')
            item['states'] = [r for r in item['states'] if r['state'] != 'composing']
            (root / capabilities.COVERAGE).write_text(json.dumps(data))
            with self.assertRaisesRegex(ValueError, 'component-text-field: required state coverage missing=.*composing'):
                capabilities.component_coverage(capabilities.read_json(capabilities.REGISTRY))
        with self.contract_fixture() as root:
            data = capabilities.read_json(capabilities.STATE_PROFILES)
            data['profiles']['button'].append('contract-added-state')
            (root / capabilities.STATE_PROFILES).write_text(json.dumps(data))
            with self.assertRaisesRegex(ValueError, 'contract-added-state'):
                capabilities.component_coverage(capabilities.read_json(capabilities.REGISTRY))

    def test_duplicate_state_missing_component_and_historical_override_are_rejected(self):
        for mutation, message in [
            ('duplicate', 'duplicate state'), ('missing', 'component coverage missing='),
            ('override', 'cannot be silently overridden'), ('exception', 'invalid state-profile exceptions')]:
            with self.contract_fixture() as root:
                data = capabilities.read_json(capabilities.COVERAGE)
                item = next(c for c in data['components'] if c['id'] == 'tui.button')
                if mutation == 'duplicate': item['states'].append(item['states'][0])
                elif mutation == 'missing': data['components'].remove(item)
                elif mutation == 'override': item['profile_exceptions'] = {'loading': 'silently drop a required state'}
                else: next(c for c in data['components'] if c['id'] == 'tui.toolbar')['profile_exceptions'] = {'invented': 'invalid exception'}
                (root / capabilities.COVERAGE).write_text(json.dumps(data))
                with self.assertRaisesRegex(ValueError, message):
                    capabilities.component_coverage(capabilities.read_json(capabilities.REGISTRY))

    def test_fixture_anchor_and_native_acceptance_cannot_be_fabricated(self):
        for mutation, message in [('anchor', 'fixture source/anchor/story/review note missing'),
                                  ('native', 'cannot promote runtime/native acceptance')]:
            with self.contract_fixture() as root:
                data = capabilities.read_json(capabilities.COVERAGE)
                review = next(c for c in data['components'] if c['id'] == 'tui.button')['states'][0]
                if mutation == 'anchor': review['evidence']['anchor'] = 'NONEXISTENT_FIXTURE_ANCHOR'
                else: review['experience'] = 'pass'
                (root / capabilities.COVERAGE).write_text(json.dumps(data))
                with self.assertRaisesRegex(ValueError, message):
                    capabilities.component_coverage(capabilities.read_json(capabilities.REGISTRY))

    def test_contract_token_index_retains_provenance_and_runtime_unknown(self):
        button = next(c for c in self.data['components'] if c['id'] == 'tui.button')['token_dependencies']
        self.assertTrue(button['contract_present'])
        self.assertTrue(button['contract_references'])
        self.assertTrue(all(r['pointers'] for r in button['contract_references']))
        self.assertIn('color:c-button-primary-ground', button['contract_alias_closure'])
        self.assertEqual(button['status'], 'runtime-unverified')
        self.assertEqual(button['tokens'], [])
        surface = next(c for c in self.data['components'] if c['id'] == 'tui.surface')['token_dependencies']
        self.assertFalse(surface['contract_present'])
        self.assertEqual(surface['contract_references'], [])
        area = next(c for c in self.data['components'] if c['id'] == 'tui.text-area')['token_dependencies']
        self.assertEqual(area['contract_owner'], 'tui.text-field')
        self.assertTrue(area['contract_inheritance_reason'])

    def test_unknown_contract_token_and_broken_or_cyclic_aliases_fail(self):
        with self.contract_fixture() as root:
            data = capabilities.read_json(capabilities.COMPONENT_CONTRACTS)
            data['components']['tui.button']['primary']['rest'] = 'c-nonexistent-button-token'
            (root / capabilities.COMPONENT_CONTRACTS).write_text(json.dumps(data))
            with self.assertRaisesRegex(ValueError, 'tui.button: unknown contract token c-nonexistent-button-token'):
                capabilities.contract_token_references('tui.button')
        for target, message in [({'light': '{nonexistent-token}'}, 'unknown token alias'),
                                ('{nonexistent-token}', 'unknown token alias'),
                                ('{c-button-primary-ground}', 'cyclic token dependency')]:
            with self.contract_fixture() as root:
                data = capabilities.read_json(capabilities.TOKENS)
                token = next(t for t in data['proposal']['tokens']['color']['tokens'] if t['name'] == 'c-button-primary-ground')
                token['value'] = target
                (root / capabilities.TOKENS).write_text(json.dumps(data))
                with self.assertRaisesRegex(ValueError, message):
                    capabilities.contract_token_references('tui.button')

    def test_inherited_token_contract_requires_real_owner_and_explicit_reason(self):
        with self.contract_fixture() as root:
            data = capabilities.read_json(capabilities.COVERAGE)
            row = next(c for c in data['components'] if c['id'] == 'tui.text-area')
            row['token_contract']['owner'] = 'tui.nonexistent-editor'
            (root / capabilities.COVERAGE).write_text(json.dumps(data))
            with self.assertRaisesRegex(ValueError, 'invalid inherited token contract'):
                capabilities.contract_token_references('tui.text-area')

    def test_generated_state_and_token_results_cannot_be_silently_changed(self):
        for key in ['state_coverage', 'token_dependencies']:
            bad = copy.deepcopy(self.data)
            bad['components'][0][key] = {}
            with self.assertRaisesRegex(ValueError, 'state/token coverage mismatch'):
                capabilities.validate(bad)


    def test_observed_selection_tokens_are_bounded_and_state_evidence_is_not_native_pass(self):
        for identity in ['tui.checkbox', 'tui.radio-group', 'tui.switch']:
            component = next(c for c in self.data['components'] if c['id'] == identity)
            observation = component['token_dependencies']['runtime_observation']
            self.assertFalse(observation['complete'])
            self.assertGreater(observation['case_count'], 0)
            self.assertIn('color:status-failed', observation['tokens'])
            self.assertIn('stroke:stroke-rule', observation['tokens'])
            self.assertEqual(component['state_coverage']['runtime_observation_status'], 'historical-stale')
            self.assertEqual(component['state_coverage']['public_api_verified'], [])
            self.assertEqual(set(component['state_coverage']['historical_public_api_verified']), set(component['state_coverage']['required']))
            self.assertTrue(all(s['runtime'] == 'unverified' and s.get('historical_runtime') == 'public-api-verified' for s in component['state_coverage']['reviews']))
            self.assertTrue(all(s['experience'] == 'unverified' for s in component['state_coverage']['reviews']))

    def test_observed_action_states_are_bounded_and_link_inset_is_canonical(self):
        for identity in ['tui.button','tui.icon-button','tui.link','tui.segmented-control']:
            component=next(c for c in self.data['components'] if c['id']==identity)
            observation=component['token_dependencies']['runtime_observation']
            self.assertFalse(observation['complete'])
            self.assertGreater(observation['case_count'],0)
            self.assertEqual(component['state_coverage']['runtime_observation_status'], 'historical-stale')
            self.assertEqual(component['state_coverage']['public_api_verified'], [])
            self.assertEqual(set(component['state_coverage']['historical_public_api_verified']), set(component['state_coverage']['required']))
            self.assertTrue(all(s['runtime'] == 'unverified' and s.get('historical_runtime') == 'public-api-verified' for s in component['state_coverage']['reviews']))
            self.assertTrue(all(s['experience']=='unverified' for s in component['state_coverage']['reviews']))
        link=next(c for c in self.data['components'] if c['id']=='tui.link')
        self.assertIn('spacing:space-regular',link['token_dependencies']['runtime_observation']['tokens'])

    def test_changed_production_source_downgrades_observation_with_exact_diff(self):
        with self.contract_fixture() as root:
            path = root / 'crates/tekes-ui-controls/src/selection.rs'
            path.write_text(path.read_text() + '\n// modified production implementation\n')
            row = next(c for c in capabilities.read_json(capabilities.COVERAGE)['components'] if c['id'] == 'tui.switch')
            with patch.object(capabilities, 'ROOT', root):
                observation = capabilities.runtime_observation(row)
            self.assertEqual(observation['status'], 'historical-stale')
            self.assertIn('crates/tekes-ui-controls/src/selection.rs', {item['path'] for item in observation['identity_differences']})
            self.assertTrue(observation['inputs_sha256'])
            self.assertFalse(observation['complete'])

    def test_stale_identity_is_not_current_public_api_verified_and_keeps_partial_scope(self):
        stack = next(c for c in self.data['components'] if c['id'] == 'tui.stack')
        chart = next(c for c in self.data['components'] if c['id'] == 'tui.chart')
        for component in [stack, chart]:
            coverage = component['state_coverage']
            self.assertEqual(coverage['runtime_observation_status'], 'historical-stale')
            self.assertEqual(coverage['public_api_verified'], [])
            self.assertEqual(coverage['historical_public_api_verified'], ['rest'] if component['id'] == 'tui.stack' else ['rest', 'focus', 'selected'])
            self.assertEqual(component['maturity'], 'experimental')
        self.assertEqual(len(chart['state_coverage']['required']), 8)
        self.assertTrue(chart['state_coverage']['fixture_unverified'])
        self.assertEqual(chart['token_dependencies']['runtime_observation']['status'], 'historical-stale')
        self.assertIn('crates/tekes-ui-controls/src/chart.rs', {item['path'] for item in chart['token_dependencies']['runtime_observation']['identity_differences']})

    def test_bounded_button_and_chart_receipts_project_without_completing_roots(self):
        button = next(c for c in self.data['components'] if c['id'] == 'tui.button')
        chart = next(c for c in self.data['components'] if c['id'] == 'tui.chart')
        for component in (button, chart):
            intake = component['state_coverage']['practical_intake']
            differences = intake['source_identity']['differences']
            if differences:
                self.assertEqual(intake['status'], 'accepted-source-subset-stale')
                for source, difference in differences.items():
                    self.assertEqual(difference['expected'], intake['source_identity']['sha256'][source])
                    self.assertEqual(difference['actual'], capabilities.digest((ROOT / source).read_bytes()))
                    self.assertNotEqual(difference['expected'], difference['actual'])
            else:
                self.assertEqual(intake['status'], 'accepted-source-subset-matched')
            self.assertTrue(intake['claims'])
            self.assertTrue(intake['exceptions'])
            self.assertTrue(intake['gaps'])
            self.assertEqual(intake['consumer_entry']['script'], 'scripts/component-context.py')
            self.assertEqual(component['maturity'], 'experimental')
            self.assertEqual(component['state_coverage']['public_api_verified'], [])
        self.assertEqual(len(button['state_coverage']['practical_intake']['evidence']), 2)
        self.assertEqual(len(chart['state_coverage']['practical_intake']['evidence']), 2)
        self.assertEqual(len(chart['state_coverage']['required']), 8)
        self.assertEqual(chart['state_coverage']['runtime_observation_status'], 'historical-stale')

    def test_practical_intake_rejects_cross_identity_missing_path_and_wrong_state(self):
        cases = [
            ('cross-identity', lambda intake: intake['evidence'][0].update(component='tui.chart'), 'cross-component'),
            ('missing-source', lambda intake: intake['source_identity']['sha256'].update({'crates/missing.rs': '0' * 64}), 'missing practical intake source'),
            ('wrong-status', lambda intake: intake.update(status='complete'), 'invalid practical intake status'),
        ]
        for name, mutate, expected in cases:
            with self.subTest(name=name), self.contract_fixture() as root:
                data = capabilities.read_json(capabilities.COVERAGE)
                row = next(item for item in data['components'] if item['id'] == 'tui.button')
                mutate(row['practical_intake'])
                (root / capabilities.COVERAGE).write_text(json.dumps(data))
                with patch.object(capabilities, 'ROOT', root):
                    with self.assertRaisesRegex(ValueError, expected):
                        capabilities.component_coverage(capabilities.read_json(capabilities.REGISTRY))

    def test_runtime_observation_rejects_cross_identity_missing_sources_and_malformed_identity(self):
        with self.contract_fixture() as root:
            coverage = capabilities.read_json(capabilities.COVERAGE)
            stack = next(c for c in coverage['components'] if c['id'] == 'tui.stack')
            wrong_owner = dict(stack, id='tui.button')
            with patch.object(capabilities, 'ROOT', root):
                with self.assertRaisesRegex(ValueError, 'missing actual runtime lookup cases for this component identity'):
                    capabilities.runtime_observation(wrong_owner)
                artifact = root / stack['runtime_token_observation']
                original = json.loads(artifact.read_text())
                bad = copy.deepcopy(original)
                bad['inputs_sha256']['crates/does-not-exist.rs'] = '0' * 64
                artifact.write_text(json.dumps(bad))
                with self.assertRaisesRegex(ValueError, 'missing runtime observation source path'):
                    capabilities.runtime_observation(stack)
                bad['inputs_sha256'] = dict(original['inputs_sha256'])
                bad['inputs_sha256']['crates/tekes-ui-controls/src/chart.rs'] = 'not-a-sha'
                artifact.write_text(json.dumps(bad))
                with self.assertRaisesRegex(ValueError, 'malformed runtime observation identity'):
                    capabilities.runtime_observation(stack)
                bad['inputs_sha256'] = dict(original['inputs_sha256'])
                bad['inputs_sha256']['../../outside.rs'] = '0' * 64
                artifact.write_text(json.dumps(bad))
                with self.assertRaisesRegex(ValueError, 'invalid runtime observation source path'):
                    capabilities.runtime_observation(stack)

    def test_public_api_state_claim_requires_executed_cases_in_four_themes(self):
        with self.contract_fixture() as root:
            data = capabilities.read_json(capabilities.COVERAGE)
            row = next(c for c in data['components'] if c['id'] == 'tui.button')
            row.pop('runtime_token_observation', None)
            row['states'][0]['runtime'] = 'public-api-verified'
            (root / capabilities.COVERAGE).write_text(json.dumps(data))
            with self.assertRaisesRegex(ValueError, 'component-button/rest: missing four-theme public API evidence'):
                capabilities.component_coverage(capabilities.read_json(capabilities.REGISTRY))


class ZeroLookupEvidenceTests(unittest.TestCase):
    @contextmanager
    def observation(self, case):
        # A closed no-op render may execute no dynamic lookup. It still needs
        # asserted state, an explicit reason and the same source provenance.
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            source = root / 'fixture.rs'
            source.write_text('// isolated observation source\n')
            relative = 'design/reviews/zero-lookups.json'
            path = root / relative
            path.parent.mkdir(parents=True)
            path.write_text(json.dumps({
                'format': 'tekesui-runtime-token-observation/1', 'complete': False,
                'scope': 'one asserted closed render; no aliases/constants/native claims',
                'excluded': ['unexecuted branches', 'constants', 'native experience'],
                'inputs_sha256': {'fixture.rs': capabilities.digest(source.read_bytes())},
                'cases': [case],
            }))
            with patch.object(capabilities, 'ROOT', root):
                yield {'id': 'tui.tooltip', 'runtime_token_observation': relative}

    def test_asserted_empty_render_retains_real_zero_reads_without_inventing_tokens(self):
        case = {'component': 'tui.tooltip', 'content_state': 'rest', 'state_asserted': True,
                'theme': 'light', 'lookups': [], 'no_dynamic_lookups': True,
                'zero_lookup_reason': 'Closed Tooltip produces no semantic paint or dynamic token calls.'}
        with self.observation(case) as row:
            result = capabilities.runtime_observation(row)
        self.assertFalse(result['complete'])
        self.assertEqual(result['cases'], [case])
        self.assertEqual(result['cases'][0]['lookups'], [])

    def test_missing_unasserted_or_contradictory_empty_reads_still_fail_closed(self):
        valid = {'component': 'tui.tooltip', 'content_state': 'rest', 'state_asserted': True,
                 'theme': 'light', 'lookups': [], 'no_dynamic_lookups': True,
                 'zero_lookup_reason': 'No dynamic calls executed.'}
        malformed = []
        for field in ['lookups', 'no_dynamic_lookups', 'zero_lookup_reason', 'state_asserted']:
            case = dict(valid)
            case.pop(field)
            malformed.append(case)
        malformed += [dict(valid, state_asserted=False), dict(valid, zero_lookup_reason=' '),
                      dict(valid, lookups=[['color', 'surface-3']]),
                      dict(valid, lookups='color:surface-3'), dict(valid, lookups=[['color']])]
        for case in malformed:
            with self.subTest(case=case), self.observation(case) as row:
                with self.assertRaisesRegex(ValueError, 'runtime lookup'):
                    capabilities.runtime_observation(row)


if __name__ == '__main__':
    unittest.main()
