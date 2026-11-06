#!/usr/bin/env python3
"""Reject misleading native-scroll acceptance reports."""
import copy
import contextlib
import io
import json
import sys
import tempfile
from types import SimpleNamespace
from unittest import mock
import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location('scroll_summary', Path(__file__).with_name('f1-scroll-summary.py'))
summary = importlib.util.module_from_spec(spec)
spec.loader.exec_module(summary)


spec = importlib.util.spec_from_file_location('scroll_measure', Path(__file__).with_name('measure-scroll-pages-quiet.py'))
measure = importlib.util.module_from_spec(spec)
spec.loader.exec_module(measure)


def fixture():
    return {
        'completed': True, 'config': {'seconds': 2/120},
        'stats': {'frames': 3, 'rendered_frames': 3, 'refresh_interval_ms': 1000/120,
                  'cpu_ms': {'max': 1}, 'drops': {'dropped_refreshes': 0}},
        'visibility': {'occluded_callbacks': 0, 'non_key_callbacks': 0},
        'presentation': {'missing_receipts': 0, 'not_presented': 0, 'duplicate_drawable_serials': 0,
                         'samples': [[0, 1+i/120, 0] for i in range(3)]},
        'idle_after': {'display_link_callbacks': 0, 'wait_s': 5.0}}


class ScrollAcceptance(unittest.TestCase):
    def test_complete_visible_timeline(self):
        self.assertEqual(summary.strict_verdict(fixture()), [])

    def test_single_actual_drop_rejects_good_cpu_and_callback_metrics(self):
        report = fixture()
        report['presentation']['samples'][-1][1] += 1/120
        self.assertIn('actual_presentation_gap_or_invalid_timeline', summary.strict_verdict(report))

    def test_requested_refresh_does_not_certify_actual_100hz(self):
        report = fixture()
        report['presentation']['samples'] = [[0, 1+i/100, 0] for i in range(3)]
        self.assertIn('actual_refresh_not_120hz', summary.strict_verdict(report))

    def test_receipts_visibility_duration_and_old_metadata_are_required(self):
        mutations = [lambda r: r['presentation'].update(missing_receipts=1),
                     lambda r: r['presentation'].update(not_presented=1),
                     lambda r: r['presentation'].update(duplicate_drawable_serials=1),
                     lambda r: r['presentation']['samples'].pop(),
                     lambda r: r['visibility'].update(non_key_callbacks=1),
                     lambda r: r['visibility'].update(occluded_callbacks=1),
                     lambda r: r.pop('visibility'),
                     lambda r: r['config'].update(seconds=60),
                     lambda r: r['stats'].update(rendered_frames=2),
                     lambda r: r['stats']['drops'].update(dropped_refreshes=1),
                     lambda r: r['idle_after'].update(display_link_callbacks=1)]
        for mutation in mutations:
            report = copy.deepcopy(fixture())
            mutation(report)
            self.assertTrue(summary.strict_verdict(report))

    def test_late_receipts_do_not_erase_a_failed_drain(self):
        for drain in ({'finished': True, 'timed_out': True, 'unpresented': 0},
                      {'finished': False, 'timed_out': False, 'unpresented': 0},
                      {'finished': True, 'timed_out': False, 'unpresented': 1}):
            report = fixture()
            report['presentation_drain'] = drain
            self.assertIn('presentation_drain_failed_or_unfinished', summary.strict_verdict(report))

    def test_timer_only_liveness_proof_cannot_certify_performance(self):
        report = fixture()
        report['presentation_drain'] = {'finished': True, 'timed_out': False,
                                        'unpresented': 0, 'timer_only_diagnostic': True}
        self.assertIn('drain_timer_only_diagnostic', summary.strict_verdict(report))

    def test_early_close_rejects_otherwise_complete_report(self):
        for wait in (4.999, None, float('nan')):
            report = fixture()
            report['idle_after']['wait_s'] = wait
            self.assertIn('idle_observation_short_or_unknown', summary.strict_verdict(report))

    def test_nonfinite_and_hidden_callback_costs_reject(self):
        for value in (9, float('nan'), float('inf')):
            report = fixture()
            report['samples'] = {'columns': ['cpu_ms'], 'rows': [[1], [value]]}
            self.assertTrue(summary.strict_verdict(report))
        report = fixture()
        report['presentation']['samples'][1][1] = float('nan')
        self.assertTrue(summary.strict_verdict(report))


class QuietMethodAcceptance(unittest.TestCase):
    @staticmethod
    def sample(ws_cpu=45, external_cpu=20):
        # Real usage() classification with equivalent cumulative CPU deltas;
        # no ps snapshot, native process, or elapsed measurement is executed.
        return measure.quiet.usage((0, {}), (3, {
            402: (ws_cpu*3/100, '/System/WindowServer'),
            999: (external_cpu*3/100, '/other/task'),
        }), '/fake/app', set(), 30, 100)

    def run_case(self, before, during, options=()):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            binary = root/'not-executable'
            binary.write_bytes(b'offline fake child, never executed')
            out = root/'reports'
            report = fixture()
            report['process'] = {'cpu_percent_while_scrolling': 1.0}
            clock = [0.0]
            rows = iter([copy.deepcopy(before)]+[copy.deepcopy(r) for r in during])
            def sleep(seconds): clock[0] += seconds
            def launch(_argv, *, env, **_kwargs):
                Path(env['TEKES_PERF_OUT']).write_text(json.dumps(report))
                polls = iter([None]*len(during)+[0])
                return SimpleNamespace(pid=1000, returncode=0, poll=lambda: next(polls))
            argv = ['measure', '--binary', str(binary), '--out', str(out),
                    '--prefix', 'case', '--content', 'scroll-area',
                    '--seconds', str(2/120), '--stable-seconds', '3',
                    '--wait-seconds', '3', '--interval', '3', *options]
            with mock.patch.object(sys, 'argv', argv), \
                 mock.patch.object(measure.time, 'sleep', sleep), \
                 mock.patch.object(measure.time, 'monotonic', lambda: clock[0]), \
                 mock.patch.object(measure.quiet, 'snapshot', return_value=None), \
                 mock.patch.object(measure.quiet, 'usage', side_effect=lambda *a: next(rows)), \
                 mock.patch.object(measure.subprocess, 'Popen', side_effect=launch) as child, \
                 contextlib.redirect_stdout(io.StringIO()):
                code = 0
                try: measure.main()
                except SystemExit as error: code = error.code
            audit = json.loads((out/'case-scroll-area-ambient.json').read_text())
            summary_path, receipt_path = out/'case-summary.json', out/'case-scroll-area-run.json'
            result = json.loads(summary_path.read_text())[0] if summary_path.exists() else None
            receipt = json.loads(receipt_path.read_text()) if receipt_path.exists() else None
            return code, child.call_count, audit, result, receipt

    def test_ws_only_new_method_records_both_results_without_rewriting_raw(self):
        row = self.sample()
        self.assertFalse(row['quiet'])
        self.assertTrue(row['cpu_attribution']['known_external']['below_limits'])
        code, calls, audit, result, receipt = self.run_case(row, [row], ['--quiet-method', 'known-external'])
        self.assertEqual((code, calls), (0, 1))
        self.assertFalse(audit['raw_gate_passed'])
        self.assertTrue(audit['selected_gate_passed'])
        self.assertFalse(result['raw_ambient_cpu_below_limits'])
        self.assertTrue(result['selected_ambient_cpu_below_limits'])
        self.assertTrue(result['quiet_gate_and_zero_drop_pass'])
        self.assertEqual(receipt['quiet_method_id'], measure.QUIET_METHODS['known-external'])
        self.assertFalse(receipt['raw_gate_passed'])
        self.assertTrue(receipt['selected_ambient_cpu_below_limits'])
        for key, value in row.items(): self.assertEqual(audit['during'][0][key], value)

    def test_independent_external_excess_rejects_admission_and_during(self):
        bad, good = self.sample(external_cpu=40), self.sample()
        code, calls, audit, result, _ = self.run_case(bad, [], ['--quiet-method', 'known-external'])
        self.assertEqual((code, calls), (3, 0))
        self.assertFalse(audit['gate_passed'])
        self.assertIsNone(result)
        _, _, _, result, _ = self.run_case(good, [bad], ['--quiet-method', 'known-external'])
        self.assertFalse(result['selected_ambient_cpu_below_limits'])
        self.assertFalse(result['quiet_gate_and_zero_drop_pass'])

    def test_empty_during_never_passes(self):
        _, _, audit, result, receipt = self.run_case(self.sample(), [], ['--quiet-method', 'known-external'])
        self.assertEqual(audit['during'], [])
        self.assertFalse(result['ambient_cpu_below_limits'])
        self.assertFalse(result['quiet_gate_and_zero_drop_pass'])
        self.assertFalse(receipt['selected_ambient_cpu_below_limits'])

    def test_shared_desktop_diagnostic_never_passes_either_gate(self):
        row = self.sample(ws_cpu=0)
        _, _, audit, result, _ = self.run_case(row, [row], ['--quiet-method', 'known-external', '--shared-desktop-diagnostic'])
        self.assertFalse(audit['raw_gate_passed'])
        self.assertFalse(audit['selected_gate_passed'])
        self.assertFalse(result['quiet_gate_and_zero_drop_pass'])
        self.assertFalse(result['raw_quiet_gate_and_zero_drop_pass'])

    def test_old_default_is_raw_and_keeps_old_outcomes(self):
        code, calls, audit, result, _ = self.run_case(self.sample(), [])
        self.assertEqual((code, calls), (3, 0))
        self.assertEqual(audit['quiet_method'], 'raw')
        self.assertFalse(audit['gate_passed'])
        self.assertIsNone(result)
        row = self.sample(ws_cpu=0)
        _, _, audit, result, receipt = self.run_case(row, [row])
        self.assertTrue(result['quiet_gate_and_zero_drop_pass'])
        self.assertEqual(result['raw_ambient_cpu_below_limits'], result['ambient_cpu_below_limits'])
        self.assertEqual(receipt['quiet_method'], 'raw')
        self.assertTrue(audit['external_process_cpu_below_threshold_during_all_samples'])


    def test_known_external_reuses_total_and_heavy_process_blockers(self):
        for processes in ({pid: (.28*3, '/other/task') for pid in range(10,14)},
                          {10: (.02*3, '/tools/cargo')}):
            row = measure.quiet.usage((0, {}), (3, processes), '/fake/app', set(), 30, 100)
            self.assertFalse(measure.quiet_sample_passed(row, 'known-external'))


if __name__ == '__main__':
    unittest.main()
