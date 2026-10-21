#!/usr/bin/env python3
"""Guard completeness and truthful source binding of the manually reviewed audit."""
import copy
import importlib.util
import json
from pathlib import Path
import unittest
import tempfile
import shutil
ROOT=Path(__file__).resolve().parents[1]
spec=importlib.util.spec_from_file_location('composition_audit',ROOT/'scripts/check-composition-audit.py')
audit=importlib.util.module_from_spec(spec);spec.loader.exec_module(audit)


class AuditTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):cls.data=json.loads((ROOT/audit.MATRIX).read_text())
    def test_current_full_matrix_has_exact_source_evidence(self):self.assertEqual(audit.validate(self.data),45)
    def test_unchanged_audited_export_is_valid_without_git_head(self):
        # A commit/export changes repository identity, not reviewed source bytes.
        # Source edits still fail even when the historical baseline is unchanged.
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            for name in [*self.data['source_sha256'], 'design/gallery/capabilities.json']:
                target = root / name
                target.parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(ROOT / name, target)
            self.assertEqual(audit.validate(self.data, root=root), 45)
            name = next(iter(self.data['source_sha256']))
            with (root / name).open('a') as stream:
                stream.write('\nchanged after audit\n')
            with self.assertRaisesRegex(ValueError, 'stale composition audit'):
                audit.validate(self.data, root=root)

    def test_baseline_still_requires_an_immutable_commit_identity(self):
        data = copy.deepcopy(self.data)
        data['head'] = 'main'
        with self.assertRaisesRegex(ValueError, 'immutable audit baseline'):
            audit.validate(data)

    def test_omitted_or_duplicate_components_fail(self):
        for mutate in [lambda d:d['components'].pop(),lambda d:d['components'].append(d['components'][0])]:
            d=copy.deepcopy(self.data);mutate(d)
            with self.assertRaisesRegex(ValueError,'exactly once'):audit.validate(d)
    def test_stale_source_or_fabricated_line_fails(self):
        d=copy.deepcopy(self.data);key=next(iter(d['source_sha256']));d['source_sha256'][key]='0'*64
        with self.assertRaisesRegex(ValueError,'stale composition audit'):audit.validate(d)
        d=copy.deepcopy(self.data);d['components'][0]['evidence'][0]['line']=1
        with self.assertRaisesRegex(ValueError,'evidence'):audit.validate(d)
    def test_leaf_does_not_acquire_a_children_requirement(self):
        d=copy.deepcopy(self.data);leaf=next(c for c in d['components'] if c['kind']=='leaf');leaf['content_capacity']='缺失'
        with self.assertRaisesRegex(ValueError,'leaf must not'):audit.validate(d)
    def test_registered_public_binding_cannot_be_omitted(self):
        d=copy.deepcopy(self.data);d['components'][0]['evidence'].pop(0)
        with self.assertRaisesRegex(ValueError,'registered public API'):audit.validate(d)
    def test_source_audit_does_not_promote_native_acceptance(self):
        d=copy.deepcopy(self.data);d['components'][0]['native_acceptance']='已完成'
        with self.assertRaisesRegex(ValueError,'cannot promote native'):audit.validate(d)


if __name__=='__main__':unittest.main()
