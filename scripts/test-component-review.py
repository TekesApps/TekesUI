#!/usr/bin/env python3
"""Safety boundaries of the real local build/cleanup entry point; no Cargo/UI."""
import argparse
import importlib.util
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch
import tekesui_build as shared

spec = importlib.util.spec_from_file_location('component_review', Path(__file__).with_name('component-review.py'))
review = importlib.util.module_from_spec(spec)
spec.loader.exec_module(review)


class ReviewSafetyTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name).resolve()

    def test_cleanup_rejects_parent_escape_and_symlink_targets(self):
        owned = self.root / 'owned'; owned.mkdir()
        source = self.root / 'source.rs'; source.write_text('preserve WIP')
        link = owned / 'link'; link.symlink_to(source)
        for path in (owned, source, link, owned / '..' / 'source.rs'):
            with self.assertRaises(SystemExit):
                shared.remove_owned(path, owned)
        self.assertEqual(source.read_text(), 'preserve WIP')
        artifact = owned / 'binary'; artifact.write_bytes(b'compiled')
        shared.remove_owned(artifact, owned)
        self.assertFalse(artifact.exists())

    def test_open_mapping_or_cwd_blocks_cleanup_and_inventory_failure_is_closed(self):
        path = self.root / 'app'; path.mkdir()
        for opened in (str(path), str(path / 'Contents/MacOS/core')):
            result = subprocess.CompletedProcess([], 0, 'p42\nn' + opened + '\n', '')
            with patch.object(shared.subprocess, 'run', return_value=result):
                with self.assertRaises(SystemExit): shared.assert_unused([path])
        with patch.object(shared.subprocess, 'run', return_value=subprocess.CompletedProcess([], 1, '', 'denied')):
            with self.assertRaises(SystemExit): shared.assert_unused([path])

    def test_unowned_cache_and_configuration_drift_are_not_adopted(self):
        cache = self.root / 'cache'; cache.mkdir()
        with patch.object(shared, 'CACHE', cache), patch.object(shared, 'ROOT', self.root), patch.object(shared.subprocess, 'check_output', return_value='rustc test'):
            (cache / 'unrelated').write_text('keep')
            with self.assertRaises(SystemExit): shared.prepare_cache()
            (cache / 'unrelated').unlink()
            shared.prepare_cache()
            marker = cache / '.tekesui-cache.json'
            original = marker.read_bytes()
            with patch.dict(shared.os.environ, {'RUSTFLAGS': 'different'}):
                with self.assertRaises(SystemExit): shared.prepare_cache()
            self.assertEqual(marker.read_bytes(), original)

    def test_switching_checkout_invalidates_local_packages_but_retains_registry_cache(self):
        cache = self.root / 'cache'
        first = self.root / 'first'; second = self.root / 'second'
        for folder in (first, second):
            folder.mkdir(); (folder / 'Cargo.toml').write_text('[workspace]')
        metadata = json.dumps({'packages': [
            {'name': 'tekes-ui-controls', 'source': None},
            {'name': 'tekes-ui-design-export', 'source': None},
            {'name': 'wgpu', 'source': 'registry+crates.io'},
        ]})
        def output(command, **kwargs):
            return 'rustc test' if command[0] == 'rustc' else metadata
        with patch.object(shared, 'ROOT', self.root), patch.object(shared, 'CACHE', cache), \
             patch.object(shared.subprocess, 'check_output', side_effect=output), \
             patch.object(shared.subprocess, 'run', return_value=subprocess.CompletedProcess([], 0, 'removed local artifacts', '')) as run, \
             patch.object(shared, 'assert_unused') as unused:
            shared.prepare_cache(workspace_root=first)
            shared.prepare_cache(workspace_root=first)
            self.assertEqual(run.call_count, 1)
            shared.prepare_cache(workspace_root=second)
            self.assertEqual(run.call_count, 2)
            args = run.call_args.args[0]
            self.assertIn('tekes-ui-controls', args)
            self.assertIn('tekes-ui-design-export', args)
            self.assertIn('--locked', args)
            self.assertIn('--offline', args)
            self.assertNotIn('wgpu', args)
            self.assertEqual(unused.call_count, 2)
            marker = cache / '.tekesui-workspace.json'
            self.assertEqual(json.loads(marker.read_text())['root'], str(second))
            with patch.object(shared, 'assert_unused', side_effect=SystemExit('in use')):
                with self.assertRaisesRegex(SystemExit, 'in use'):
                    shared.prepare_cache(workspace_root=first)
            self.assertEqual(json.loads(marker.read_text())['root'], str(second))
            self.assertEqual(run.call_count, 2)

    def test_excluded_consumer_is_cleaned_in_its_own_workspace_before_ownership_moves(self):
        workspace = self.root / 'workspace'
        consumer = workspace / 'examples/composition-consumer'
        consumer.mkdir(parents=True)
        for folder in (workspace, consumer):
            (folder / 'Cargo.toml').write_text('[workspace]')
        cache = self.root / 'cache'
        def output(command, **kwargs):
            if command[0] == 'rustc':
                return 'rustc test'
            name = 'consumer' if kwargs['cwd'] == consumer else 'controls'
            return json.dumps({'packages': [{'name': name, 'source': None}]})
        def clean(command, **kwargs):
            failed = kwargs['cwd'] == consumer and fail_consumer
            return subprocess.CompletedProcess(command, int(failed), '', 'consumer failed' if failed else '')
        with patch.object(shared, 'ROOT', self.root), patch.object(shared, 'CACHE', cache), \
             patch.object(shared.subprocess, 'check_output', side_effect=output), \
             patch.object(shared.subprocess, 'run', side_effect=clean) as run, \
             patch.object(shared, 'assert_unused'):
            fail_consumer = True
            with self.assertRaisesRegex(SystemExit, 'consumer failed'):
                shared.prepare_cache(workspace_root=workspace)
            self.assertFalse((cache / '.tekesui-workspace.json').exists())
            fail_consumer = False
            run.reset_mock()
            shared.prepare_cache(workspace_root=workspace)
            self.assertEqual([call.kwargs['cwd'] for call in run.call_args_list], [workspace, consumer])
            self.assertIn('controls', run.call_args_list[0].args[0])
            self.assertNotIn('consumer', run.call_args_list[0].args[0])
            self.assertIn('consumer', run.call_args_list[1].args[0])
            for call in run.call_args_list:
                self.assertIn('--locked', call.args[0])
                self.assertIn('--offline', call.args[0])
            marker = json.loads((cache / '.tekesui-workspace.json').read_text())
            self.assertEqual(marker['packages'], ['consumer', 'controls'])

    def test_pending_acceptance_never_cleans_or_claims_passed(self):
        receipt = self.root / 'r/receipt.json'; receipt.parent.mkdir()
        data = {'status': 'open', 'inputs': {}, 'build': {'result': 'passed'}, 'required': list(review.LAYERS), 'layers': {}}
        receipt.write_text(json.dumps(data))
        args = type('Args', (), {'round': 'r', 'discard': False})()
        with patch.object(review, 'RECEIPTS', self.root), patch.object(review, 'inputs', return_value={}):
            with self.assertRaisesRegex(SystemExit, 'pending/failed'): review.finish(args)
        self.assertEqual(json.loads(receipt.read_text())['status'], 'open')

    def test_source_drift_invalidates_previous_results(self):
        receipt = self.root / 'r/receipt.json'; receipt.parent.mkdir()
        receipt.write_text(json.dumps({'status': 'open', 'inputs': {'source.rs': 'before'}}))
        with patch.object(review, 'RECEIPTS', self.root), patch.object(review, 'inputs', return_value={'source.rs': 'after'}):
            with self.assertRaisesRegex(SystemExit, 'Source/WIP changed'): review.load('r')

    def test_replaced_binary_is_preserved_even_when_round_is_discarded(self):
        receipts = self.root / 'receipts'; runtime = self.root / 'runtime'; cache = self.root / 'cache'
        receipt = receipts / 'r/receipt.json'; receipt.parent.mkdir(parents=True)
        app = runtime / 'r'; app.mkdir(parents=True)
        (app / '.owned.json').write_text(json.dumps({'round': 'r', 'root': str(self.root)}))
        cache.mkdir(); binary = cache / 'native'; binary.write_text('new build owned elsewhere')
        data = {'status': 'open', 'artifacts': [{'path': str(binary), 'sha256': 'old hash'}]}
        receipt.write_text(json.dumps(data))
        args = type('Args', (), {'round': 'r', 'discard': True})()
        with patch.object(review, 'RECEIPTS', receipts), patch.object(review, 'RUNTIME', runtime), patch.object(review, 'CACHE', cache), patch.object(review, 'ROOT', self.root), patch.object(review, 'assert_unused'):
            with self.assertRaisesRegex(SystemExit, 'replaced by another build'): review.finish(args)
        self.assertEqual(binary.read_text(), 'new build owned elsewhere')
        self.assertTrue(app.exists())

    def test_build_observed_source_drift_cannot_be_accepted_even_if_source_was_restored(self):
        receipt = self.root / 'r/receipt.json'; receipt.parent.mkdir()
        receipt.write_text(json.dumps({'status': 'open', 'inputs': {}, 'build': {'result': 'passed'},
                                       'source_changed_during_build': True, 'required': [], 'layers': {}}))
        args = type('Args', (), {'round': 'r', 'discard': False})()
        with patch.object(review, 'RECEIPTS', self.root), patch.object(review, 'inputs', return_value={}):
            with self.assertRaises(SystemExit): review.finish(args)

    def test_motion_gate_refuses_to_overwrite_an_existing_evidence_directory(self):
        marker = self.root / 'test.log'; marker.write_text('previous evidence')
        result = subprocess.run(['python3', str(Path(__file__).with_name('check-motion-transition.py')),
                                 '--output', str(self.root)], capture_output=True, text=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(marker.read_text(), 'previous evidence')

    def test_round_name_cannot_escape_evidence_directory(self):
        for name in ('../r', '/tmp/r', 'a/b', '.hidden'):
            with self.assertRaises(argparse.ArgumentTypeError): review.identifier(name)


if __name__ == '__main__':
    unittest.main()
