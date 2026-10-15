#!/usr/bin/env python3
"""Offline integration tests for authored inputs -> brand compiler -> Rust projection."""
import importlib.util
import json
from pathlib import Path
import shutil
import sys
import tempfile
import unittest
sys.dont_write_bytecode=True
ROOT=Path(__file__).resolve().parents[1]
spec=importlib.util.spec_from_file_location('merge_design',ROOT/'scripts/merge-design-system.py')
merge=importlib.util.module_from_spec(spec);spec.loader.exec_module(merge)

class MergeTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.reference,cls.report=merge.merge()
    def fixture(self):
        temporary=tempfile.TemporaryDirectory(prefix='tekes-ds-merge-')
        self.addCleanup(temporary.cleanup);root=Path(temporary.name)
        for file in merge.SOURCE_FILES:
            path=root/file;path.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(ROOT/file,path)
        imported=root/merge.UPSTREAM;imported.parent.mkdir(parents=True,exist_ok=True);imported.symlink_to(ROOT/merge.UPSTREAM,target_is_directory=True)
        return root
    def test_selected_neutral_exactly_matches_full_package_bundle(self):
        expected=merge.load(ROOT/merge.UPSTREAM/'design/brand/generated/example-neutral/tokens.json')
        self.assertEqual(self.reference['proposal']['tokens'],expected)
        self.assertEqual(self.reference['proposal']['brand']['config']['brand_id'],'example-neutral')
        self.assertEqual(len(self.report['text_profiles']),16)
    def test_generated_projection_is_deterministic_and_all_169_mappings_resolve(self):
        again,report=merge.merge();self.assertEqual(merge.bytes_json(again),merge.bytes_json(self.reference));self.assertEqual(report,self.report)
        self.assertEqual(len(report['bindings']),169)
        for binding in report['bindings']:
            self.assertEqual(merge.pointer(again,binding['legacy'])['$value'],binding['resolved_value'])
    def test_five_unmatched_text_roles_and_motion_rules_are_retained_or_adopted(self):
        legacy=merge.load(ROOT/'design/source/legacy.tokens.json')
        for role in self.report['unmatched_text_roles']:
            self.assertEqual(legacy['tokens']['text'][role],self.reference['tokens']['text'][role])
        self.assertEqual(merge.pointer(self.reference,'/tokens/motion/standard/state-change')['$value']['value'],merge.f32(.15))
        self.assertEqual(merge.pointer(self.reference,'/tokens/motion/standard/geometry')['$value']['value'],merge.f32(.12))
    def test_consumer_blue_brand_propagates_only_through_permitted_graph(self):
        root=self.fixture();config=merge.load(ROOT/merge.UPSTREAM/'design/brand/examples/example-blue.json')
        (root/'design/brand/config.json').write_bytes(merge.bytes_json(config));blue,_=merge.merge(root)
        expected=merge.load(ROOT/merge.UPSTREAM/'design/brand/generated/example-blue/tokens.json')
        self.assertEqual(blue['proposal']['tokens'],expected)
        self.assertNotEqual(blue['sets']['light']['color']['accent'],self.reference['sets']['light']['color']['accent'])
        for role in ['primary-text','button-hover','row-selection','border-focused','text-selection','status-failed','diff-added-surface']:
            for theme in merge.THEMES:self.assertEqual(blue['sets'][theme]['color'][role],self.reference['sets'][theme]['color'][role])
        self.assertEqual(blue['tokens'],self.reference['tokens'])
    def test_unknown_brand_invalid_overrides_and_duplicate_value_authority_are_rejected(self):
        for change in [lambda c:c.update(seed_srgb='blue'),lambda c:c.update(overrides={'accent':{'reason':'insufficient themes','values':{'light':'#123456'}}})]:
            root=self.fixture();config=merge.load(root/'design/brand/config.json');change(config);(root/'design/brand/config.json').write_bytes(merge.bytes_json(config))
            with self.assertRaises(ValueError):merge.merge(root)
        root=self.fixture();legacy=merge.load(root/'design/source/legacy.tokens.json');legacy['tokens']['radius']['control']['$value']={'value':28,'unit':'pt'};(root/'design/source/legacy.tokens.json').write_bytes(merge.bytes_json(legacy))
        with self.assertRaisesRegex(ValueError,'authored twice'):merge.merge(root)
    def test_unknown_mapping_target_is_not_silently_retained(self):
        root=self.fixture();mapping=merge.load(root/'design/source/rust-mapping.json');mapping['bindings'][0]['target']='unknown-token';(root/'design/source/rust-mapping.json').write_bytes(merge.bytes_json(mapping))
        with self.assertRaisesRegex(ValueError,'unknown/duplicate'):merge.merge(root)
    def test_immutable_archive_is_not_written_by_import(self):
        self.assertFalse(any((ROOT/merge.UPSTREAM).rglob('__pycache__')))

if __name__=='__main__':unittest.main()
