#!/usr/bin/env python3
"""Script-level #57 regressions. Uses package fixtures, never native/Rust tools."""
import copy
import importlib.util
import json
from pathlib import Path
import plistlib
import sys
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
sys.path.insert(0, str(ROOT / 'scripts'))


def load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


probe = load('icon57_probe', HERE / 'verify_package.py')
package = load('icon57_package', ROOT / 'scripts/package-gallery.py')


class CatalogEvidence(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.manifest = json.loads((ROOT / 'crates/tekes-ui-icons/catalog.json').read_text())
        cls.report = {'schema': 'tekesui-icon-catalog/1', 'mode': 'canonical', 'count': 177,
                      'notices': [], 'entries': [
                          {'name': row['name'], 'svg': (ROOT / 'crates/tekes-ui-icons' / row['file']).read_text(),
                           'canonical_match': True, 'before': False}
                          for row in cls.manifest['icons']]}

    def test_source_inventory_reproduces_the_original_29_overlaps(self):
        inventory = probe.source_inventory(ROOT)
        self.assertEqual((inventory['canonical_count'], inventory['proposal_overlap_count'],
                          inventory['changed_overlaps']), (177, 51, 29))
        changed = [row for row in inventory['overlaps'] if row['different']]
        self.assertEqual(sum(row['reference_kind'] == 'shared_current'
                             for row in inventory['overlaps']), 8)
        self.assertTrue(all(row['historical_strokes'] == ['1.5'] for row in changed))
        self.assertTrue(all(row['canonical_strokes'] == ['1.25'] for row in changed))

    def test_verifier_accepts_exact_default_and_never_claims_pixels(self):
        result = probe.verify_report(self.report, self.manifest)
        self.assertEqual(result['count'], 177)
        self.assertEqual(result['pixel_acceptance'], 'not_performed_by_this_probe')

    def test_count_only_or_self_reported_canonical_flag_cannot_hide_wrong_artwork(self):
        wrong = copy.deepcopy(self.report)
        wrong['entries'][0]['svg'] = '<svg/>'
        with self.assertRaisesRegex(ValueError, 'artwork'):
            probe.verify_report(wrong, self.manifest)

    def test_duplicate_identity_cannot_replace_a_missing_identity(self):
        wrong = copy.deepcopy(self.report)
        wrong['entries'][0] = wrong['entries'][1]
        with self.assertRaisesRegex(ValueError, 'names'):
            probe.verify_report(wrong, self.manifest)

    def test_partial_historical_or_error_results_do_not_pass_default_acceptance(self):
        for change in ({'count': 51}, {'mode': 'historical'}, {'notices': ['missing directory']}):
            with self.subTest(change=change), self.assertRaises(ValueError):
                probe.verify_report({**self.report, **change}, self.manifest)
        wrong = copy.deepcopy(self.report)
        wrong['entries'][0]['before'] = True
        with self.assertRaisesRegex(ValueError, 'comparison'):
            probe.verify_report(wrong, self.manifest)

    def test_source_denial_profile_names_each_checkout_without_renaming_it(self):
        roots = [Path('/some source/TekesUI'), Path('/other/quote"root')]
        profile = probe.sandbox_profile(roots)
        self.assertIn('(deny file-read* (subpath "/some source/TekesUI"))', profile)
        self.assertIn(json.dumps(str(roots[1])), profile)

    def test_probe_refuses_output_within_a_source_checkout_before_copy_or_launch(self):
        with tempfile.TemporaryDirectory() as temporary:
            source = Path(temporary)
            app = source / 'Gallery.app'
            app.mkdir()
            with patch.object(probe.subprocess, 'Popen') as launch:
                with self.assertRaisesRegex(ValueError, 'outside every blocked checkout'):
                    probe.run_package(app, [source], source / 'evidence', 1)
                launch.assert_not_called()

    def test_no_checkout_denial_means_no_launch_and_a_retained_failed_result(self):
        with tempfile.TemporaryDirectory() as temporary:
            fixture = Path(temporary)
            source = fixture / 'source'
            source.mkdir()
            (source / 'Cargo.toml').write_text('[workspace]')
            app = source / 'Gallery.app'
            contents = app / 'Contents'
            (contents / 'Resources').mkdir(parents=True)
            (contents / 'MacOS').mkdir()
            (contents / 'MacOS/gallery').write_bytes(b'fixture only')
            (contents / 'Info.plist').write_bytes(plistlib.dumps({'CFBundleExecutable': 'gallery'}))
            (contents / 'Resources/IconCatalog.json').write_text(json.dumps(self.manifest))
            output = fixture / 'outside'
            with patch.object(probe.subprocess, 'run', return_value=SimpleNamespace(
                    returncode=0, stdout=b'[workspace]', stderr=b'')), \
                    patch.object(probe.subprocess, 'Popen') as launch:
                with self.assertRaisesRegex(RuntimeError, 'read-denial preflight failed'):
                    probe.run_package(app, [source], output, 1)
                launch.assert_not_called()
            result = json.loads((output / 'result.json').read_text())
            self.assertEqual(result['status'], 'failed')
            self.assertIn('read-denial preflight failed', result['error'])
            self.assertEqual((source / 'Cargo.toml').read_text(), '[workspace]')


class PackageStructure(unittest.TestCase):
    def test_existing_binary_bundle_contains_public_manifest_and_original_app_resources(self):
        # Mock only the platform tooling: this is package-structure evidence,
        # not a signed app or a real package-outside-checkout execution.
        with tempfile.TemporaryDirectory() as temporary:
            fixture = Path(temporary)
            binary = fixture / 'gallery-binary'
            binary.write_bytes(b'fixture executable; not a real Gallery')
            app = fixture / 'TekesUI Gallery.app'
            commands = []

            def platform_tool(command, **kwargs):
                commands.append(command)
                if command[0] == 'xcrun':
                    compiled = Path(command[command.index('--compile') + 1])
                    (compiled / 'Assets.car').write_bytes(b'fixture app icon')
                    (compiled / 'icon-info.plist').write_bytes(
                        plistlib.dumps({'CFBundleIconName': 'Gallery'}))

            with patch.object(package, 'APP', app), patch.object(package.sys, 'platform', 'darwin'), \
                    patch.object(package.sys, 'argv', ['package-gallery.py', '--binary', str(binary)]), \
                    patch.object(package.subprocess, 'run', side_effect=platform_tool):
                package.main()
            contents = app / 'Contents'
            info = plistlib.loads((contents / 'Info.plist').read_bytes())
            self.assertEqual((contents / 'MacOS' / info['CFBundleExecutable']).read_bytes(), binary.read_bytes())
            self.assertEqual((contents / 'Resources/IconCatalog.json').read_bytes(),
                             (ROOT / 'crates/tekes-ui-icons/catalog.json').read_bytes())
            self.assertEqual((contents / 'Resources/Gallery.icns').read_bytes(),
                             (package.ASSETS / 'Gallery.icns').read_bytes())
            self.assertTrue((contents / 'Resources/Assets.car').is_file())
            self.assertEqual([command[0] for command in commands], ['xcrun', 'codesign', 'codesign'])
            self.assertEqual(info['CFBundleIdentifier'], 'com.tekesui.gallery.dev')


if __name__ == '__main__':
    unittest.main()
