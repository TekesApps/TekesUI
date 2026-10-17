#!/usr/bin/env python3
import importlib.util
from pathlib import Path
import unittest
s=importlib.util.spec_from_file_location('finalize',Path(__file__).with_name('finalize-symbol-performance.py'))
m=importlib.util.module_from_spec(s);s.loader.exec_module(m)


def fixture(interval=1000/120):
    return ({'strict_zero_drop_pass':True,'not_presented':0,
             'presentation_interval_ms':{'count':2,'mean':interval},
             'gpu_timing':False,'static_diagnostic':False,
             'low_background_gate_passed':True,
             'external_process_cpu_below_threshold_during_all_samples':True},
            {'idle_after':{'wait_s':5.0}, 'presentation':{'samples':[[1,1.041,0],[1+interval/1000,1.041+interval/1000,0]]},
             'samples':{'columns':['target_s','cpu_ms','budget_ms'],
                        'rows':[[1,1,8],[1+interval/1000,1,8]]}})


class FinalGates(unittest.TestCase):
    def test_100hz_cannot_pass_even_without_a_12_5ms_gap(self):
        row,report=fixture(10)
        self.assertFalse(m.finalize_row(row,report)['strict_120hz_pass'])

    def test_120hz_requires_environment_evidence_for_quiet_acceptance(self):
        row,report=fixture()
        self.assertTrue(m.finalize_row(row,report)['quiet_120hz_acceptance_pass'])
        row['low_background_gate_passed']=False
        self.assertTrue(m.finalize_row(row,report)['strict_120hz_pass'])
        self.assertFalse(m.finalize_row(row,report)['quiet_120hz_acceptance_pass'])

    def test_drain_failures_reject_performance_acceptance(self):
        row,report=fixture()
        good={'finished':True,'timed_out':False,'unpresented':0,'timer_only_diagnostic':False}
        report['presentation_drain']=good.copy()
        self.assertTrue(m.finalize_row(row,report)['strict_120hz_pass'])
        for field,value in [('finished',False),('timed_out',True),('unpresented',1),('timer_only_diagnostic',True)]:
            with self.subTest(field=field):
                report['presentation_drain']={**good,field:value}
                self.assertFalse(m.finalize_row(row,report)['strict_120hz_pass'])

    def test_legacy_reports_keep_drain_unknown(self):
        row,report=fixture()
        self.assertIsNone(m.finalize_row(row,report)['presentation_drain_pass'])

    def test_early_or_unknown_idle_observation_cannot_pass(self):
        row,report=fixture()
        for idle in [{},{'wait_s':4.99},{'wait_s':None}]:
            with self.subTest(idle=idle):
                report['idle_after']=idle
                self.assertFalse(m.finalize_row(row,report)['strict_120hz_pass'])
        del report['idle_after']
        self.assertFalse(m.finalize_row(row,report)['strict_120hz_pass'])

    def test_terminal_failure_is_not_exempted_and_input_stays_intact(self):
        row,report=fixture();row['not_presented']=1
        report['presentation']['samples'][-1][1]=0
        result=m.finalize_row(row,report)
        self.assertTrue(result['last_measured_frame_not_presented'])
        self.assertFalse(result['strict_120hz_pass'])
        self.assertNotIn('last_measured_frame_not_presented',row)


if __name__=='__main__':unittest.main()
