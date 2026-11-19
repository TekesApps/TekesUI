#!/usr/bin/env python3
"""Unit tests for the consolidated scripts/audit-component-tokens.py runner."""
import importlib.util
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / 'scripts/audit-component-tokens.py'
spec = importlib.util.spec_from_file_location('audit_component_tokens', SCRIPT)
mod = importlib.util.module_from_spec(spec)
spec.loader.exec_module(mod)


class AuditComponentTokensTests(unittest.TestCase):
    EXPECTED_SUITES = {
        'action',
        'choice-selection',
        'choice-trigger',
        'collection-focus',
        'combo-async',
        'command-async',
        'command-results',
        'content-status',
        'grid-layout',
        'input',
        'layout-surface',
        'menu-content',
        'navigation',
        'passive-feedback',
        'popover-content',
        'popup',
        'select-async',
        'selection',
        'separator',
        'slider',
        'slider-composition',
        'tooltip',
    }

    def test_list_outputs_all_22_suites(self):
        run = subprocess.run(
            [sys.executable, str(SCRIPT), '--list'],
            cwd=ROOT,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=True,
        )
        listed = {line.split('\t', 1)[0] for line in run.stdout.splitlines() if line.strip()}
        self.assertEqual(listed, self.EXPECTED_SUITES)
        self.assertEqual(len(mod.SUITES), 22)

    def test_refuses_existing_output_and_unknown_suite(self):
        with tempfile.TemporaryDirectory(prefix='tekesui-audit-cli-') as folder:
            existing = Path(folder) / 'existing.json'
            existing.write_text('preserve-me\n')
            overwrite = subprocess.run(
                [sys.executable, str(SCRIPT), '--suite', 'navigation', '--output', str(existing)],
                cwd=ROOT,
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            )
            self.assertNotEqual(overwrite.returncode, 0)
            self.assertEqual(existing.read_text(), 'preserve-me\n')

            unknown = subprocess.run(
                [sys.executable, str(SCRIPT), '--suite', 'nonexistent', '--output', str(Path(folder) / 'out.json')],
                cwd=ROOT,
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            )
            self.assertNotEqual(unknown.returncode, 0)

    def test_active_suite_input_hashing_against_converged_workspace(self):
        for sid in mod.active_suite_ids():
            with self.subTest(suite=sid):
                digests = mod.suite_inputs(sid)
                self.assertIn('scripts/audit-component-tokens.py', digests)
                self.assertIn('design/tokens.json', digests)
                self.assertFalse(any('examples/composition-consumer/Cargo.lock' == k for k in digests))

    def test_removed_review_host_suites_fail_with_actionable_message(self):
        for sid in ('separator', 'grid-layout', 'collection-focus'):
            with self.subTest(suite=sid), tempfile.TemporaryDirectory(prefix='tekesui-host-err-') as folder:
                out = Path(folder) / f'{sid}.json'
                run = subprocess.run(
                    [sys.executable, str(SCRIPT), '--suite', sid, '--output', str(out)],
                    cwd=ROOT,
                    text=True,
                    stdout=subprocess.PIPE,
                    stderr=subprocess.PIPE,
                )
                self.assertNotEqual(run.returncode, 0)
                self.assertIn('--host', run.stderr)
                self.assertFalse(out.exists())


if __name__ == '__main__':
    unittest.main()
