#!/usr/bin/env python3
"""Reject misleading native-scroll acceptance reports."""
import copy
import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location('scroll_summary', Path(__file__).with_name('f1-scroll-summary.py'))
summary = importlib.util.module_from_spec(spec)
spec.loader.exec_module(summary)


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


if __name__ == '__main__':
    unittest.main()
