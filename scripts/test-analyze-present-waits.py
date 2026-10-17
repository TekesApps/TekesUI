#!/usr/bin/env python3
import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location(
    'present_analysis', Path(__file__).with_name('analyze-present-waits.py'))
analysis = importlib.util.module_from_spec(spec)
spec.loader.exec_module(analysis)


class BudgetComparisonTests(unittest.TestCase):
    def test_measured_work_and_reported_budget_remain_distinct(self):
        result = analysis.budget_comparison([
            {'budget_ms': 8, 'cpu_ms': 1}, {'budget_ms': 4, 'cpu_ms': 5}])
        self.assertEqual(result['known_samples'], 2)
        self.assertEqual(result['handler_work_over_reported_budget'], 1)
        self.assertEqual(result['reported_budget_minus_handler_work_ms']['mean'], 3)

    def test_missing_or_invalid_times_do_not_become_zero_cost(self):
        result = analysis.budget_comparison([
            {}, {'cpu_ms': None, 'budget_ms': 8}, {'cpu_ms': float('nan'), 'budget_ms': 8}])
        self.assertEqual(result['unknown_samples'], 3)
        self.assertEqual(result['known_samples'], 0)
        self.assertIsNone(result['reported_budget_minus_handler_work_ms'])
        self.assertIsNone(analysis.budget_comparison([])['reported_budget_minus_handler_work_ms'])


if __name__ == '__main__':
    unittest.main()
