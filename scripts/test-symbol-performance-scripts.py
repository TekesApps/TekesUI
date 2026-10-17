#!/usr/bin/env python3
"""Regression checks for measurement integrity and low-background gating."""
import copy
import importlib.util
from pathlib import Path
import unittest


def module(name, filename):
    spec = importlib.util.spec_from_file_location(name, Path(__file__).with_name(filename))
    result = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(result)
    return result


summary = module('symbol_summary', 'summarize-symbol-perf.py')
quiet = module('symbol_quiet', 'measure-symbols-quiet.py')


def fixture():
    dist = {'count': 3, 'max': 1, 'mean': 1, 'p99': 1}
    return {
        'stats': {'frames': 3, 'rendered_frames': 3, 'refresh_interval_ms': 1000/120,
                  'cpu_ms': dist, 'cpu_active_ms': dist, 'build_ms': dist,
                  'acquire_ms': dist, 'gpu_ms': None, 'drops': {'dropped_refreshes': 0}},
        'symbols_workload': {'sources':8,'animated_views':24,'mode':'steady',
                            'all_models_stopped':True,'errors':[], 'model_and_control_ms':dist,
                            'raster_and_upload_ms':dist,'parameter_edits_including_warmup':0,
                            'target_changes_including_warmup':0},
        'presentation': {'samples':[[0,1,0],[0,1+1/120,0],[0,1+2/120,0]],
                         'missing_receipts':0,'not_presented':0,'duplicate_drawable_serials':0,
                         'interval_ms':dist,'gaps_over_1_5_refresh':0},
        'completed':True,'config':{'seconds':2/120},'gpu_timing':{'enabled':False},
        'display':{},'visibility':{'occluded_callbacks':0,'non_key_callbacks':0},
        'process':{'cpu_percent_while_scrolling':10,'cpu_rate_peak_250ms_percent':20},
        'idle_after':{'display_link_callbacks':0},'scene_per_frame_median':{'images':24}}


class Integrity(unittest.TestCase):
    def test_complete_visible_timeline_can_pass(self):
        self.assertTrue(summary.summarize(fixture(),'ok','x')['strict_zero_drop_pass'])

    def test_skipped_occluded_duplicate_missing_or_legacy_cannot_pass(self):
        mutations = [lambda r:r['stats'].update(rendered_frames=2),
                     lambda r:r['visibility'].update(occluded_callbacks=1),
                     lambda r:r['visibility'].update(non_key_callbacks=1),
                     lambda r:r['presentation'].update(duplicate_drawable_serials=1),
                     lambda r:r['presentation'].update(missing_receipts=1),
                     lambda r:r['presentation'].update(not_presented=1),
                     lambda r:r.pop('visibility'),
                     lambda r:r['presentation'].pop('duplicate_drawable_serials')]
        for mutation in mutations:
            r = copy.deepcopy(fixture());mutation(r)
            self.assertFalse(summary.summarize(r,'bad','x')['strict_zero_drop_pass'])

    def test_unpresented_receipt_cannot_hide_display_gap(self):
        r=fixture();r['presentation']['samples']=[[0,1,0],[0,0,0],[0,1+3/120,0]]
        row=summary.summarize(r,'gap','x')
        self.assertEqual(row['presentation_gap_events'],1)
        self.assertAlmostEqual(row['presentation_interval_ms']['max'],25)
        self.assertFalse(row['strict_zero_drop_pass'])

    def test_short_receipt_fragment_cannot_pass_full_duration(self):
        r=fixture();r['config']['seconds']=60
        result=summary.summarize(r,'fragment','x')
        self.assertIn('presented_timeline_shorter_than_configured_duration', result['measurement_integrity_issues'])
        self.assertFalse(result['strict_zero_drop_pass'])

    def test_unrendered_callback_cost_is_still_reported(self):
        r=fixture();r['samples']={'columns':['cpu_ms','build_ms','rendered'],
                                 'rows':[[1,.5,True],[100,.7,False]]}
        result=summary.summarize(r,'skipped-cost','x')
        self.assertEqual(result['all_callback_cpu_ms']['max'],100)
        self.assertEqual(result['all_callback_build_ms']['max'],.7)

    def test_cpu_budget_and_pause_are_required(self):
        r=fixture();r['stats']['cpu_ms']={'max':9}
        self.assertFalse(summary.summarize(r,'budget','x')['strict_zero_drop_pass'])
        r=fixture();r['idle_after']['display_link_callbacks']=1
        self.assertFalse(summary.summarize(r,'pause','x')['strict_zero_drop_pass'])

    def test_cumulative_cpu_delta_excludes_only_known_benchmark(self):
        before=(10,{1:(1,'rustc'),2:(2,'bench'),3:(5,'idle')})
        after=(12,{1:(2,'rustc'),2:(4,'bench'),3:(5,'idle')})
        row=quiet.usage(before,after,'bench',set(),30,100)
        self.assertFalse(row['quiet']);self.assertEqual(row['cpu_percent_total'],50)
        self.assertTrue(quiet.usage(before,after,'bench',{1},30,100)['quiet'])
        after=(12,{1:(1.1,'rustc'),2:(4,'bench'),3:(5,'idle')})
        self.assertFalse(quiet.usage(before,after,'bench',set(),30,100)['quiet'])

    def test_cpu_time_formats_include_days(self):
        self.assertEqual(quiet.cpu_seconds('01:02.50'),62.5)
        self.assertEqual(quiet.cpu_seconds('1-02:03:04'),93784)


if __name__ == '__main__':
    unittest.main()
