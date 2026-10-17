#!/usr/bin/env python3
"""Checks for the diagnostic runner; these do not certify performance."""
import importlib.util
from pathlib import Path
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location(
    'trace_gallery', Path(__file__).with_name('trace-gallery-presentation.py'))
trace = importlib.util.module_from_spec(spec)
spec.loader.exec_module(trace)


class TraceEnvironmentTests(unittest.TestCase):
    def test_no_inherited_secrets_or_unrelated_harness_options(self):
        with patch.dict(trace.os.environ, {
            'HOME': '/tmp/test-user', 'PATH': '/unexpected', 'EXAMPLE_API_KEY': 'fixture',
            'TEKES_SELFTEST': 'gallery', 'TEKES_PERF_SPEED': '999999',
        }, clear=True):
            env = trace.environment(Path('/tmp/report'), 'cad', 65)
        self.assertNotIn('EXAMPLE_API_KEY', env)
        self.assertNotIn('TEKES_SELFTEST', env)
        self.assertNotIn('TEKES_PERF_SPEED', env)
        self.assertEqual(env['PATH'], '/usr/bin:/bin:/usr/sbin:/sbin')
        self.assertEqual(env['TEKES_PERF'], 'scroll:65')
        self.assertEqual(env['TEKES_PERF_GPU_TIMING'], '0')
        self.assertEqual(env['TEKES_PERF_OUT'], '/tmp/report/perf.json')
        self.assertEqual(env['TEKES_METAL_DISPLAY_LINK'], '0')

    def test_driver_is_explicit_and_workbench_is_real(self):
        env = trace.environment(Path('/tmp/report'), 'metal', 30)
        self.assertEqual(env['TEKES_METAL_DISPLAY_LINK'], '1')
        self.assertEqual(env['TEKES_PERF_WORKBENCH'], '1')
        self.assertEqual(env['TEKES_PERF_CONTENT'], 'scroll-area')
        self.assertEqual(env['TEKES_PERF_FRAME_LATENCY'], '2')

    def test_empty_or_failed_exports_cannot_be_usable(self):
        tables = {schema: {'exit': 0, 'rows': 1} for schema in trace.TABLES}
        receipt = {'complete': True, 'tables': tables}
        self.assertTrue(trace.usable_queue_capture(receipt))
        for schema in ('metal-application-command-buffer-submissions',
                       'metal-command-buffer-completed', 'metal-gpu-intervals'):
            with self.subTest(schema=schema):
                modified = {**tables, schema: {'exit': 0, 'rows': 0}}
                self.assertFalse(trace.usable_queue_capture({**receipt, 'tables': modified}))
        self.assertFalse(trace.usable_queue_capture({**receipt, 'complete': False}))


if __name__ == '__main__':
    unittest.main()
