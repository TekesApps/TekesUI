#!/usr/bin/env python3
"""Theme completeness, deterministic outputs and semantic rejection regressions."""
import copy
import importlib.util
import json
from pathlib import Path
import sys
import hashlib
import unittest
from unittest.mock import patch
sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('theme_system', ROOT / 'scripts/generate-theme-system.py')
g = importlib.util.module_from_spec(spec)
spec.loader.exec_module(g)

class ThemeSystemTests(unittest.TestCase):
    def rules(self): return g.merge.load(ROOT / 'design/theme/rules.json')
    def test_link_rejects_invented_marker_helper_in_every_original_state(self):
        for state in ['rest','hover','pressed','disabled']:
            for channel in ['marker','helper']:
                r=self.rules();r['profiles']['link'][state][channel]='primary-text'
                with self.assertRaisesRegex(ValueError,'Link has no marker/helper anatomy'):g.bindings(r)
    def test_passive_status_content_complete_shared_profiles(self):
        r=self.rules(); p=g.validate(r)
        for tone in ['info','running','succeeded','failed']:
            u='content-status-'+tone
            self.assertEqual(set(p[u]),{'rest','disabled'})
            self.assertEqual(p[u]['rest']['background'],'$surface')
            self.assertEqual(p[u]['rest']['text'],'status-'+tone)
            self.assertEqual(p[u]['rest']['icon'],'status-'+tone)
            self.assertEqual(p[u]['disabled'],p['content']['disabled'])
            for d in r['surfaces'].values():self.assertEqual(d['usages'][u],u)
    def test_passive_status_content_rejects_missing_uses_and_action_states(self):
        for tone in ['info','running','succeeded','failed']:
            u='content-status-'+tone
            r=self.rules(); del r['surfaces']['canvas']['usages'][u]
            with self.assertRaises(ValueError):g.validate(r)
            r=self.rules();r['surfaces']['canvas']['usages'][u]='quiet'
            with self.assertRaisesRegex(ValueError,'incomplete or unsupported states'):g.validate(r)
    def test_passive_status_content_rejects_own_ground_and_indicators(self):
        for k,v in [('background','surface-0'),('boundary','separator'),('focus','border-focused'),('marker','primary-text'),('helper','primary-text')]:
            r=self.rules();r['profiles']['content-status-info']['state_overrides']['rest'][k]=v
            with self.assertRaisesRegex(ValueError,'passive status content requires parent ground'):g.validate(r)
    def test_unused_shared_profile_unknown_role_rejected_by_normal_generation(self):
        original_load = g.merge.load
        for channel in ['text', 'boundary']:
            rules = self.rules()
            rules['profiles']['not-yet-used'] = {'extends': 'content', 'state_overrides': {'rest': {channel: 'definitely-no-such-color-role'}}}
            def load(path):
                return copy.deepcopy(rules) if path == ROOT / 'design/theme/rules.json' else original_load(path)
            with patch.object(g.merge, 'load', side_effect=load):
                with self.assertRaisesRegex(ValueError, 'unknown color role: definitely-no-such-color-role'):
                    g.generate()

    def test_known_unused_shared_profile_is_allowed_without_changing_any_default_color(self):
        original_load = g.merge.load
        baseline = json.loads(g.generate()['design/theme/generated/instances.json'])
        rules = self.rules()
        rules['profiles']['not-yet-used'] = {'extends': 'content'}
        def load(path):
            return copy.deepcopy(rules) if path == ROOT / 'design/theme/rules.json' else original_load(path)
        with patch.object(g.merge, 'load', side_effect=load):
            output = json.loads(g.generate()['design/theme/generated/instances.json'])
        self.assertEqual([i['modes'] for i in baseline['instances']], [i['modes'] for i in output['instances']])

    def test_empty_or_non_array_theme_definitions_rejected(self):
        load = g.merge.load
        for definitions in [[], {}, None]:
            def with_definitions(path):
                return definitions if path == ROOT / 'design/theme/themes.json' else load(path)
            with patch.object(g.merge, 'load', side_effect=with_definitions):
                with self.assertRaisesRegex(ValueError, 'non-empty array'):
                    g.generate()
    def test_complete_generation_is_deterministic_and_fresh(self):
        a, b = g.generate(), g.generate()
        self.assertEqual(a,b)
        for path,data in a.items(): self.assertEqual((ROOT/path).read_bytes(),data,path)

    def test_artifact_lock_pins_exact_source_closure_and_candidate_artifacts(self):
        outputs=g.generate()
        self.assertNotIn(g.ARTIFACT_LOCK,outputs)
        expected=g.artifact_lock_bytes(outputs)
        lock=json.loads(expected)
        current=(ROOT/g.ARTIFACT_LOCK).read_bytes()
        self.assertEqual(current,expected)
        generated=json.loads(outputs['design/theme/generated/instances.json'])
        contract=generated['theme_input_contract']
        expected_paths=set(contract['pinned_inputs'])|{'design/system.json','design/theme/themes.json'}
        expected_paths|={item['brand_config'] for item in generated['instances']}
        self.assertEqual(set(generated['inputs_sha256']),expected_paths)
        self.assertEqual(set(lock['source_inputs_sha256']),expected_paths)
        self.assertEqual(lock['source_inputs_sha256'],generated['inputs_sha256'])
        self.assertEqual(lock['definition_source_sha256'],generated['inputs_sha256']['design/theme/themes.json'])
        self.assertEqual(lock['themes'],[{'id':i['id'],'version':i['version'],'schema_version':i['schema_version'],'brand_config':i['brand_config']} for i in generated['instances']])
    def test_missing_state_channel_surface_use_and_unknown_profile_rejected(self):
        for mutate in [
            lambda r:r['profiles']['quiet'].pop('hover'),
            lambda r:r['profiles']['quiet']['hover'].pop('icon'),
            lambda r:r['surfaces']['panel']['usages'].pop('input'),
            lambda r:r['surfaces']['panel']['usages'].update(input='unknown'),
            lambda r:r['profiles']['quiet'].update(typo=r['profiles']['quiet']['rest']),
            lambda r:r['profiles']['quiet']['disabled'].update(focus='border-focused'),
            lambda r:r['profiles']['quiet']['rest'].update(text=None),
            lambda r:r.update(schema_version='99'),
            lambda r:r.update(rules_version='99'),
            lambda r:r.update(modes=['light']),
            lambda r:r['surfaces']['panel'].update(guess_rgb='#ffffff'),
        ]:
            r=self.rules();mutate(r)
            with self.assertRaises(Exception): g.bindings(r)
    def test_additive_surface_uses_same_shared_profiles_and_complete_states(self):
        r=self.rules();r['surfaces']['inspector']=copy.deepcopy(r['surfaces']['panel'])
        rows=g.bindings(r)
        self.assertEqual(len([x for x in rows if x[0]=='inspector']),sum(len(v) for v in g.USES.values()))
        self.assertIn('("inspector", "action-quiet", "hover")',g.rust_source(r))
    def test_default_preserved_and_brand_semantics_stay_isolated(self):
        output=json.loads(g.generate()['design/theme/generated/instances.json'])
        instances=output['instances'];self.assertEqual([i['id'] for i in instances],['default','blue','clay','blue-inputs'])
        fixture=json.loads((ROOT/'scripts/fixtures/theme-v1.6.1-color-baseline.json').read_text())
        canonical=lambda x:json.dumps(x,sort_keys=True,separators=(',',':'),ensure_ascii=False).encode()
        self.assertEqual(sum(x['surface_packet_count'] for x in fixture['themes']),15792)
        for record in fixture['themes']:
            packet=next(i for i in instances if i['id']==record['theme_id'])['modes'][record['mode']]
            self.assertEqual(hashlib.sha256(canonical(packet)).hexdigest(),record['canonical_sha256'],(record['theme_id'],record['mode']))
        for mode in g.MODES:
            default=instances[0]['modes'][mode]
            for instance in instances[1:3]:
                candidate=instance['modes'][mode]
                for surface in default:
                    for usage in ['content','action-quiet','input','selection']:
                        a=copy.deepcopy(default[surface][usage]);b=copy.deepcopy(candidate[surface][usage]);
                        for state in a:a[state].pop('marker');b[state].pop('marker')
                        self.assertEqual(a,b)
                self.assertNotEqual(default['panel']['action-primary']['rest']['background'],candidate['panel']['action-primary']['rest']['background'])
            changed=instances[3]
            self.assertEqual(changed['brand'],instances[1]['brand'])
            if mode=='light':
                self.assertNotEqual(changed['compiled_color_roles'][mode]['p-surface-0'],instances[1]['compiled_color_roles'][mode]['p-surface-0'])
                self.assertNotEqual(changed['compiled_color_roles'][mode]['p-primary-text'],instances[1]['compiled_color_roles'][mode]['p-primary-text'])
                self.assertNotEqual(changed['compiled_color_roles'][mode]['p-status-info'],instances[1]['compiled_color_roles'][mode]['p-status-info'])
                self.assertNotEqual(changed['modes'][mode],instances[1]['modes'][mode])
        self.assertNotEqual(instances[3]['compiled_color_roles']['light']['p-accent'],instances[1]['compiled_color_roles']['light']['p-accent'])

    def test_invalid_root_contract_and_contrast_rejection_leave_generated_files_unchanged(self):
        original_load=g.merge.load
        system_path=ROOT/'design/system.json'
        output_paths=['design/theme/generated/instances.json','design/theme/generated/surface_rules.rs']
        before={p:(ROOT/p).read_bytes() for p in output_paths}
        mutations=[
            lambda c:c['pinned_inputs'].pop('design/theme/rules.json'),
            lambda c:c['pinned_inputs'].update({'../outside.json':'0'*64}),
            lambda c:c.update(unknown_contract_field=True),
            lambda c:c.update(contract_version='99.0.0'),
            lambda c:c.pop('constraints_ref'),
            lambda c:c.update(supported_modes=['light']),
            lambda c:c['pinned_inputs'].update({'design/theme/rules.json':'0'*64}),
        ]
        for mutation in mutations:
            system=json.loads(json.dumps(original_load(system_path)))
            mutation(system['theme_system']['theme_input_contract'])
            def load(path, system=system):return system if path==system_path else original_load(path)
            with patch.object(g.merge,'load',side_effect=load):
                with self.assertRaises(Exception):g.generate()
            self.assertEqual(before,{p:(ROOT/p).read_bytes() for p in output_paths})
        definitions=json.loads(json.dumps(original_load(ROOT/'design/theme/themes.json')))
        definitions[0]['inputs']['overrides']['neutral_base']={'p-primary-text':{m:'#FFFFFF' for m in g.MODES}}
        def bad_contrast(path):return definitions if path==ROOT/'design/theme/themes.json' else original_load(path)
        with patch.object(g.merge,'load',side_effect=bad_contrast):
            with self.assertRaisesRegex(ValueError,'BRAND_CONTRAST'):g.generate()
        self.assertEqual(before,{p:(ROOT/p).read_bytes() for p in output_paths})

    def test_theme_input_overrides_reject_unknown_semantic_generated_missing_mode_and_alpha(self):
        original_load=g.merge.load
        invalid=[
            {'neutral_base':{'no-such-role':{m:'#111111' for m in g.MODES}},'status_base':{}},
            {'neutral_base':{'surface-0':{m:'#111111' for m in g.MODES}},'status_base':{}},
            {'neutral_base':{'action-primary-ground':{m:'#111111' for m in g.MODES}},'status_base':{}},
            {'neutral_base':{'p-surface-0':{'light':'#111111'}},'status_base':{}},
            {'neutral_base':{},'status_base':{'p-status-info':{m:'#80FFFFFF' for m in g.MODES}}},
        ]
        for overrides in invalid:
            defs=json.loads(json.dumps(original_load(ROOT/'design/theme/themes.json')))
            defs[0]['inputs']['overrides']=overrides
            def load(path, defs=defs):return defs if path==ROOT/'design/theme/themes.json' else original_load(path)
            with patch.object(g.merge,'load',side_effect=load):
                with self.assertRaises(Exception):g.generate()
    def test_hover_foregrounds_and_disabled_focus_are_explicit(self):
        for _,usage,state,pair in g.bindings(self.rules()):
            self.assertEqual(set(pair),set(g.CHANNELS))
            if 'disabled' in state:self.assertIsNone(pair['focus'])
            if usage=='action-quiet' and state=='hover':
                self.assertEqual(pair['text'],'primary-text');self.assertEqual(pair['icon'],'primary-text')

    def test_shared_profile_cycles_unknown_bases_and_overrides_are_rejected(self):
        for mutation in [
            lambda r:r['profiles']['readonly'].update(extends='readonly'),
            lambda r:r['profiles']['readonly'].update(extends='missing'),
            lambda r:r['profiles']['readonly']['overrides'].update(unknown='primary-text'),
            lambda r:r['profiles']['readonly'].update(state_overrides={'missing':{'text':'primary-text'}}),
            lambda r:r['profiles']['readonly'].update(state_overrides={'disabled':{'focus':'border-focused'}}),
        ]:
            r=self.rules();mutation(r)
            with self.assertRaises(Exception):g.bindings(r)
    def test_all_thirteen_brands_and_custom_configs_use_same_complete_pairings(self):
        import importlib
        sys.path.insert(0,str(ROOT/g.merge.UPSTREAM/'tools'))
        brand=importlib.import_module('brand')
        base=g.merge.load(ROOT/'design/source/base.tokens.json')
        policy=g.merge.load(ROOT/'design/brand/policy.json')
        metadata=g.merge.load(ROOT/'design/source/token-metadata.json')
        configs=[g.merge.load(ROOT/p['config']) for p in g.merge.load(ROOT/'design/brand/gallery.json')['presets']]
        for name,seed in [('custom-apricot','#B57251'),('custom-seaglass','#367F7B')]:
            configs.append(dict(schema_version='1.0',brand_id=name,version='1.0.0',seed_srgb=seed,overrides={}))
        rows=g.bindings(self.rules())
        for config in configs:
            bundle,_=brand.compile_config(config,base=base,policy=policy,metadata=metadata)
            for mode in g.MODES:
                for surface,usage,state,roles in rows:
                    colors={k:None if v is None else g.merge.token_value(bundle,'color',v,mode) for k,v in roles.items()}
                    if 'disabled' not in state:
                        for fg in ['text','icon','marker']:
                            if colors[fg] is not None:
                                threshold=3.0 if fg=='marker' else 4.5
                                self.assertGreaterEqual(g.contrast(colors[fg],colors['background']),threshold,(config['brand_id'],mode,surface,usage,state,fg))
                        if usage == 'range-track':
                            parent = g.merge.token_value(bundle,'color',self.rules()['surfaces'][surface]['background'],mode)
                            self.assertGreaterEqual(g.contrast(colors['boundary'],parent),3.0)
                            self.assertGreaterEqual(g.contrast(colors['boundary'],colors['background']),3.0)
                        if colors['focus'] is not None:
                            parent = g.merge.token_value(bundle,'color',self.rules()['surfaces'][surface]['background'],mode)
                            self.assertGreaterEqual(g.contrast(colors['focus'],parent),3.0)
    def test_readonly_error_marker_selection_and_label_roles_remain_distinct(self):
        profiles=g.validate(self.rules())
        self.assertEqual(profiles['readonly-error']['rest']['background'],'surface-1')
        self.assertEqual(profiles['readonly-error']['rest']['helper'],'status-failed')
        self.assertEqual(profiles['readonly-error']['rest']['focus'],'border-focused')
        self.assertEqual(profiles['check-checked']['hover']['marker'],'control-checked-mark')
        self.assertEqual(profiles['content']['rest']['text'],'primary-text')
        self.assertEqual(profiles['text-selection']['rest']['background'],'text-selection')
        self.assertEqual(profiles['text-selection']['rest']['text'],'selected-text')

    def test_selectable_content_has_its_own_complete_focus_contract(self):
        profiles=g.validate(self.rules())
        self.assertEqual(set(profiles['content-selectable']),{'rest','disabled'})
        self.assertEqual(profiles['content-selectable']['rest']['focus'],'border-focused')
        self.assertIsNone(profiles['content-selectable']['disabled']['focus'])
        for state in ['rest','disabled']:
            for channel in g.CHANNELS:
                if channel!='focus':self.assertEqual(profiles['content-selectable'][state][channel],profiles['content'][state][channel])

    def test_toggle_parts_and_outside_label_are_complete_without_changing_check_markers(self):
        profiles=g.validate(self.rules())
        track,thumb,label=(profiles[k] for k in ['toggle-track','toggle-thumb','control-label'])
        for part in [track,thumb,label]:self.assertEqual(set(part),set(g.STATES))
        self.assertEqual(track['rest']['background'],'surface-3')
        self.assertEqual(thumb['rest']['background'],'surface-0')
        self.assertEqual(track['selected']['background'],'c-switch-on-ground')
        self.assertEqual(thumb['selected']['background'],'c-switch-on-thumb')
        self.assertEqual(profiles['toggle-track-error']['disabled']['boundary'],'status-failed')
        self.assertEqual(thumb['disabled']['boundary'],'disabled-text')
        self.assertEqual(thumb['rest']['boundary'],'border')
        self.assertIsNone(profiles['check-unchecked']['rest']['marker'])
        self.assertEqual(profiles['check-checked']['rest']['marker'],'control-checked-mark')
        for state in g.STATES:
            self.assertIsNone(thumb[state]['focus'])
            self.assertIsNone(label[state]['focus'])
            self.assertIsNone(label[state]['boundary'])
            self.assertEqual(label[state]['background'],'$surface')
            self.assertEqual(label[state]['text'],'tertiary-text' if 'disabled' in state else 'primary-text')
            self.assertEqual(label[state]['icon'],label[state]['text'])

    def test_missing_toggle_part_or_illegal_shared_overrides_are_rejected(self):
        for mutation in [
            lambda r:r['surfaces']['panel']['usages'].pop('toggle-thumb'),
            lambda r:r['profiles']['toggle-thumb'].update(state_overrides={'missing':{'background':'surface-0'}}),
            lambda r:r['profiles']['toggle-track-error'].update(overrides={'boundary':None,'unknown':'border'}),
            lambda r:r['profiles']['control-label'].update(state_overrides={'disabled':{'focus':'border-focused'}}),
        ]:
            r=self.rules();mutation(r)
            with self.assertRaises(Exception):g.bindings(r)

    def test_range_parts_keep_original_anatomy_and_only_four_states(self):
        p=g.validate(self.rules());track,thumb=(p[k] for k in ['range-track','range-thumb'])
        for part in [track,thumb]:self.assertEqual(set(part),set(g.STATES[:4]))
        self.assertEqual(track['rest']['background'],'surface-3')
        self.assertEqual(track['hover']['marker'],'accent-hover')
        self.assertEqual(track['pressed']['marker'],'accent-pressed')
        self.assertEqual(track['disabled']['marker'],'disabled-text')
        self.assertEqual(thumb['rest']['background'],'surface-0')
        self.assertEqual(thumb['disabled']['background'],'surface-2')
        self.assertEqual(thumb['disabled']['boundary'],'disabled-text')
        self.assertEqual(thumb['rest']['focus'],'border-focused')
        self.assertIsNone(thumb['disabled']['focus'])
        self.assertTrue(all(v['focus'] is None for v in track.values()))

    def test_range_selected_or_missing_part_definitions_are_rejected(self):
        for mutation in [
            lambda r:r['surfaces']['canvas']['usages'].pop('range-thumb'),
            lambda r:r['profiles']['range-track'].update(extends='selection'),
            lambda r:r['profiles']['range-thumb'].update(state_overrides={'selected':{'background':'surface-0'}}),
        ]:
            r=self.rules();mutation(r)
            with self.assertRaises(Exception):g.bindings(r)

    def test_enabled_range_missing_boundary_is_rejected(self):
        r=self.rules();r['profiles']['range-track']['overrides']['boundary']=None
        with self.assertRaisesRegex(ValueError,'requires a visible boundary'):g.bindings(r)

    def test_regenerated_low_contrast_range_boundary_is_rejected(self):
        r=self.rules();r['profiles']['range-track']['overrides']['boundary']='surface-3'
        load=g.merge.load
        def with_rules(path):return r if path == ROOT/'design/theme/rules.json' else load(path)
        with patch.object(g.merge,'load',side_effect=with_rules):
            with self.assertRaisesRegex(ValueError,'insufficient range boundary contrast'):g.generate()

    def test_frame_inherits_content_with_only_decorative_boundary(self):
        p=g.validate(self.rules())
        self.assertEqual(set(p['surface-frame']),{'rest','disabled'})
        for state,pair in p['surface-frame'].items():
            self.assertEqual(pair['boundary'],'separator')
            self.assertEqual({k:v for k,v in pair.items() if k!='boundary'},{k:v for k,v in p['content'][state].items() if k!='boundary'})

    def test_missing_or_interactive_frame_definitions_are_rejected(self):
        for mutation in [
            lambda r:r['surfaces']['canvas']['usages'].pop('surface-frame'),
            lambda r:r['surfaces']['canvas']['usages'].update({'surface-frame':'quiet'}),
            lambda r:r['profiles']['surface-frame']['overrides'].update(boundary=None),
            lambda r:r['profiles']['surface-frame']['overrides'].update(focus='border-focused'),
            lambda r:r['profiles']['surface-frame']['overrides'].update(marker='primary-text'),
            lambda r:r['profiles']['surface-frame']['overrides'].update(helper='primary-text'),
        ]:
            r=self.rules();mutation(r)
            with self.assertRaises(Exception):g.bindings(r)

    def test_known_low_contrast_foreground_role_is_not_a_valid_theme(self):
        r=self.rules();r['profiles']['content']['rest']['text']='document-canvas'
        load=g.merge.load
        def with_rules(path):return r if path == ROOT/'design/theme/rules.json' else load(path)
        with patch.object(g.merge,'load',side_effect=with_rules):
            with self.assertRaisesRegex(ValueError,'insufficient foreground contrast'):g.generate()

    def test_declared_focus_owners_require_focus_and_other_parts_reject_it(self):
        for usage,profile in [('action-quiet','quiet'),('input-readonly','readonly'),('check-checked','check-checked'),('toggle-track','toggle-track'),('range-thumb','range-thumb'),('link','link'),('content-selectable','content-selectable'),('selection','selection')]:
            r=self.rules();p=r['profiles'][profile]
            if 'extends' in p:p.setdefault('state_overrides',{}).setdefault('rest',{})['focus']=None
            else:p['rest']['focus']=None
            with self.assertRaisesRegex(ValueError,'focus'):g.bindings(r)
        for profile in ['content','text-selection','control-label','toggle-thumb','range-track']:
            r=self.rules();p=r['profiles'][profile]
            if 'extends' in p:p.setdefault('state_overrides',{}).setdefault('rest',{})['focus']='border-focused'
            else:p['rest']['focus']='border-focused'
            with self.assertRaisesRegex(ValueError,'focus'):g.bindings(r)

    def test_required_part_markers_and_input_helpers_cannot_be_null(self):
        for usage,channel in [('range-track','marker'),('check-checked','marker'),('check-mixed','marker'),('input','helper'),('input-error','helper'),('input-readonly','helper')]:
            r=self.rules();base=r['surfaces']['canvas']['usages'][usage]
            r['profiles']['missing-part']={'extends':base,'state_overrides':{'rest':{channel:None}}}
            r['surfaces']['canvas']['usages'][usage]='missing-part'
            with self.assertRaisesRegex(ValueError,'missing required'):g.bindings(r)

    def test_focus_must_contrast_parent_surface(self):
        r=self.rules();r['profiles']['quiet']['rest']['focus']='$surface';load=g.merge.load
        def with_rules(path):return r if path==ROOT/'design/theme/rules.json'else load(path)
        with patch.object(g.merge,'load',side_effect=with_rules):
            with self.assertRaisesRegex(ValueError,'insufficient focus contrast'):g.generate()

if __name__=='__main__':unittest.main()
