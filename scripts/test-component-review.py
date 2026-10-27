#!/usr/bin/env python3
"""Safety boundaries of the real local build/cleanup entry point; no Cargo/UI."""
import argparse
import hashlib
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

    def test_same_workspace_default_is_nondestructive_but_explicit_rebind_cleans_local_only(self):
        cache = self.root / 'cache'
        workspace = self.root / 'workspace'
        workspace.mkdir()
        (workspace / 'Cargo.toml').write_text('[workspace]')
        metadata = json.dumps({'packages': [
            {'name': 'tekes-ui-controls', 'source': None},
            {'name': 'tekes-ui-design-export', 'source': None},
            {'name': 'serde', 'source': 'registry+crates.io'},
        ]})

        def output(command, **kwargs):
            return 'rustc test' if command[0] == 'rustc' else metadata

        with patch.object(shared, 'ROOT', self.root), patch.object(shared, 'CACHE', cache), \
             patch.object(shared.subprocess, 'check_output', side_effect=output), \
             patch.object(shared.subprocess, 'run', return_value=subprocess.CompletedProcess([], 0, 'cleaned local', '')) as run, \
             patch.object(shared, 'assert_unused') as unused:
            shared.prepare_cache(workspace_root=workspace)
            run.reset_mock(); unused.reset_mock()
            workspace_owner_before = (cache / '.tekesui-workspace.json').read_bytes()
            cache_owner_before = (cache / '.tekesui-cache.json').read_bytes()
            shared.prepare_cache(workspace_root=workspace)
            run.assert_not_called()
            unused.assert_not_called()
            self.assertEqual((cache / '.tekesui-workspace.json').read_bytes(), workspace_owner_before)

            shared.prepare_cache(workspace_root=workspace, rebind_local_packages=True)
            self.assertEqual(run.call_count, 1)
            unused.assert_called_once_with([cache])
            command = run.call_args.args[0]
            self.assertIn('--locked', command)
            self.assertIn('--offline', command)
            self.assertIn('tekes-ui-controls', command)
            self.assertIn('tekes-ui-design-export', command)
            self.assertNotIn('serde', command)
            self.assertEqual((cache / '.tekesui-cache.json').read_bytes(), cache_owner_before)
            owner = json.loads((cache / '.tekesui-workspace.json').read_text())
            self.assertEqual(owner['root'], str(workspace.resolve()))
            self.assertEqual(owner['previous_root'], str(workspace.resolve()))

    def test_explicit_rebind_refusal_is_nonmutating_for_bad_flag_owner_or_occupancy(self):
        cache = self.root / 'cache'
        workspace = self.root / 'workspace'
        other = self.root / 'other'
        for folder in (workspace, other):
            folder.mkdir()
            (folder / 'Cargo.toml').write_text('[workspace]')
        metadata = json.dumps({'packages': [{'name': 'tekes-ui-controls', 'source': None}]})

        def output(command, **kwargs):
            return 'rustc test' if command[0] == 'rustc' else metadata

        with patch.object(shared, 'ROOT', self.root), patch.object(shared, 'CACHE', cache), \
             patch.object(shared.subprocess, 'check_output', side_effect=output), \
             patch.object(shared.subprocess, 'run', return_value=subprocess.CompletedProcess([], 0, '', '')) as run, \
             patch.object(shared, 'assert_unused') as unused:
            shared.prepare_cache(workspace_root=workspace)
            run.reset_mock(); unused.reset_mock()
            before = {p.relative_to(cache).as_posix(): p.read_bytes() for p in cache.rglob('*') if p.is_file()}
            with self.assertRaisesRegex(SystemExit, 'explicit bool'):
                shared.prepare_cache(workspace_root=workspace, rebind_local_packages=1)
            run.assert_not_called(); unused.assert_not_called()
            with self.assertRaisesRegex(SystemExit, 'current workspace owner'):
                shared.prepare_cache(workspace_root=other, rebind_local_packages=True)
            run.assert_not_called(); unused.assert_not_called()
            self.assertEqual(before, {p.relative_to(cache).as_posix(): p.read_bytes() for p in cache.rglob('*') if p.is_file()})

            with patch.object(shared, 'assert_unused', side_effect=SystemExit('cache in use')):
                with self.assertRaisesRegex(SystemExit, 'cache in use'):
                    shared.prepare_cache(workspace_root=workspace, rebind_local_packages=True)
            run.assert_not_called()
            self.assertEqual(before, {p.relative_to(cache).as_posix(): p.read_bytes() for p in cache.rglob('*') if p.is_file()})

            (cache / '.tekesui-workspace.json').write_text('{bad json')
            malformed_before = {p.relative_to(cache).as_posix(): p.read_bytes() for p in cache.rglob('*') if p.is_file()}
            with self.assertRaisesRegex(SystemExit, 'Invalid workspace-cache ownership'):
                shared.prepare_cache(workspace_root=workspace, rebind_local_packages=True)
            self.assertEqual(malformed_before, {p.relative_to(cache).as_posix(): p.read_bytes() for p in cache.rglob('*') if p.is_file()})

            external = self.root / 'external-owner.json'
            external.write_text(json.dumps({'format': 'tekesui-cache-workspace/1', 'root': str(workspace.resolve())}))
            (cache / '.tekesui-workspace.json').unlink()
            (cache / '.tekesui-workspace.json').symlink_to(external)
            external_before = external.read_bytes()
            with self.assertRaisesRegex(SystemExit, 'Rebind requires existing cache'):
                shared.prepare_cache(workspace_root=workspace, rebind_local_packages=True)
            self.assertEqual(external.read_bytes(), external_before)
            self.assertTrue((cache / '.tekesui-workspace.json').is_symlink())

    def _registered_target_fixture(self, *, mapped_volume_uuid=None, target_link=True):
        fixture = self.root / f'storage-{len(list(self.root.iterdir()))}'
        host = fixture / 'host'
        target = host / 'target'
        physical = fixture / 'volume-target'
        registry = fixture / 'registry.json'
        receipt = fixture / 'receipt.json'
        host.mkdir(parents=True)
        physical.mkdir()
        if target_link:
            target.symlink_to(physical, target_is_directory=True)
        else:
            target.mkdir()
        uuid = mapped_volume_uuid or shared.EXPECTED_VOLUME_UUID
        receipt_data = {'format': shared.RECEIPT_FORMAT, 'status': 'complete', 'mappings': [
            {'logical_root': str(target), 'physical_root': str(physical),
             'verified': True, 'source_removed': True},
        ]}
        receipt.write_text(json.dumps(receipt_data))
        registry_data = {'format': shared.REGISTRY_FORMAT, 'owner_uid': shared.os.getuid(),
                         'receipt': str(receipt),
                         'receipt_sha256': hashlib.sha256(receipt.read_bytes()).hexdigest(),
                         'mappings': [{'logical_root': str(target), 'physical_root': str(physical),
                                       'volume_uuid': uuid}]}
        registry.write_text(json.dumps(registry_data))
        return host, target, physical, registry, receipt

    def test_registered_target_resolves_exact_mapping_and_direct_physical_path(self):
        host, target, physical, registry, receipt = self._registered_target_fixture()
        with patch.object(shared, 'STORAGE_RECEIPT', receipt):
            mapped = shared._resolve_target_path(
                target / 'review-cache', root=host, registry_path=registry,
                volume_uuid_lookup=lambda _path: shared.EXPECTED_VOLUME_UUID)
            self.assertEqual(mapped, physical / 'review-cache')
            direct = shared._resolve_target_path(
                physical / 'review-cache', root=host, registry_path=registry,
                volume_uuid_lookup=lambda _path: shared.EXPECTED_VOLUME_UUID)
            self.assertEqual(direct, physical / 'review-cache')
            with self.assertRaisesRegex(SystemExit, 'volume UUID is unavailable or incorrect'):
                shared._resolve_target_path(
                    target / 'review-cache', root=host, registry_path=registry,
                    volume_uuid_lookup=lambda _path: 'wrong-mounted-volume')

    def test_target_resolver_rejects_wrong_registered_volume_uuid(self):
        host, target, _, registry, receipt = self._registered_target_fixture(mapped_volume_uuid='wrong-volume')
        with patch.object(shared, 'STORAGE_RECEIPT', receipt):
            with self.assertRaisesRegex(SystemExit, 'volume UUID mismatch'):
                shared._resolve_target_path(target / 'review-cache', root=host, registry_path=registry)

    def test_target_resolver_rejects_unregistered_target_symlink(self):
        host = self.root / 'host'; host.mkdir()
        physical = self.root / 'physical'; physical.mkdir()
        (host / 'target').symlink_to(physical, target_is_directory=True)
        registry = self.root / 'missing-registry.json'
        with self.assertRaisesRegex(SystemExit, 'no storage registry authorization'):
            shared._resolve_target_path(host / 'target' / 'review-cache', root=host, registry_path=registry)

    def test_target_resolver_rejects_symlink_mapping_mismatch_and_nested_symlink(self):
        host, target, physical, registry, receipt = self._registered_target_fixture()
        alternate = self.root / 'alternate'; alternate.mkdir()
        target.unlink(); target.symlink_to(alternate, target_is_directory=True)
        with patch.object(shared, 'STORAGE_RECEIPT', receipt):
            with self.assertRaisesRegex(SystemExit, 'destination changed'):
                shared._resolve_target_path(target / 'review-cache', root=host, registry_path=registry)

            target.unlink(); target.symlink_to(physical, target_is_directory=True)
            external = self.root / 'external'; external.mkdir()
            (physical / 'review-cache').symlink_to(external, target_is_directory=True)
            with self.assertRaisesRegex(SystemExit, 'symlink within mapped target'):
                shared._resolve_target_path(target / 'review-cache' / 'debug', root=host, registry_path=registry,
                                            volume_uuid_lookup=lambda _path: shared.EXPECTED_VOLUME_UUID)

    def test_target_resolver_rejects_registered_target_replaced_by_native_directory(self):
        host, target, _, registry, receipt = self._registered_target_fixture(target_link=False)
        with patch.object(shared, 'STORAGE_RECEIPT', receipt):
            with self.assertRaisesRegex(SystemExit, 'replaced by a native path'):
                shared._resolve_target_path(target / 'review-cache', root=host, registry_path=registry)

    def test_target_registry_rejects_overlapping_and_bad_receipt_mapping(self):
        host, target, physical, registry, receipt = self._registered_target_fixture()
        data = json.loads(registry.read_text())
        data['mappings'].append({'logical_root': str(target / 'nested'),
                                 'physical_root': str(self.root / 'other-physical'),
                                 'volume_uuid': shared.EXPECTED_VOLUME_UUID})
        registry.write_text(json.dumps(data))
        with patch.object(shared, 'STORAGE_RECEIPT', receipt):
            with self.assertRaisesRegex(SystemExit, 'Overlapping storage mapping root'):
                shared._load_target_registry(host, registry)

        host, target, physical, registry, receipt = self._registered_target_fixture()
        receipt_data = json.loads(receipt.read_text())
        receipt_data['mappings'][0]['physical_root'] = str(self.root / 'wrong-physical')
        receipt.write_text(json.dumps(receipt_data))
        data = json.loads(registry.read_text())
        data['receipt_sha256'] = hashlib.sha256(receipt.read_bytes()).hexdigest()
        registry.write_text(json.dumps(data))
        with patch.object(shared, 'STORAGE_RECEIPT', receipt):
            with self.assertRaisesRegex(SystemExit, 'routes do not exactly match'):
                shared._load_target_registry(host, registry)

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
