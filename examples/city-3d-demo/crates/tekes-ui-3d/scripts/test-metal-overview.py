#!/usr/bin/env python3
"""Regression tests for native Metal counter weighting and benchmark rejection."""
import importlib.util
from pathlib import Path
import tempfile
import unittest
import xml.etree.ElementTree as ET

spec = importlib.util.spec_from_file_location('overview', Path(__file__).with_name('summarize-metal-overview.py'))
overview = importlib.util.module_from_spec(spec)
spec.loader.exec_module(overview)


def export(path, records):
    root = ET.Element('trace-query-result')
    schema = ET.SubElement(root, 'schema')
    for key in records[0]:
        ET.SubElement(ET.SubElement(schema, 'col'), 'mnemonic').text = key
    for index, record in enumerate(records):
        row = ET.SubElement(root, 'row')
        for key, value in record.items():
            cell = ET.SubElement(row, key, fmt=str(value))
            if key == 'process':
                cell.set('id', f'process-{index}')
                ET.SubElement(cell, 'pid').text = str(value)
            else:
                cell.text = str(value)
    ET.ElementTree(root).write(path)


class OverviewTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.duration = Path(self.temp.name)/'duration.xml'
        self.fps = Path(self.temp.name)/'fps.xml'
        self.ds = [dict(process=7, **{'metric-variant': 0, 'layer-id': 1,
            'name': 'GPU Active Time', 'start': start, 'duration': 1_000_000_000,
            'number-of-frames': frames, 'value-in-ms': ms})
            for start, frames, ms in [(0, 60, 2), (1_000_000_000, 120, 8)]]
        # A second process with a larger frame count must never become our layer.
        self.ds.append(dict(self.ds[0], process=99, **{'number-of-frames': 9999, 'value-in-ms': 900}))
        self.cs = [dict(process=7, **{'layer-id': 1, 'name': 'FPS', 'start': start,
            'duration': 1_000_000_000, 'metric-value': fps})
            for start, fps in [(0, 60), (1_000_000_000, 120)]]

    def run_summary(self, seconds=None):
        export(self.duration, self.ds)
        export(self.fps, self.cs)
        return overview.summarize(self.duration, self.fps, None, 7, seconds)

    def test_frame_weighting_and_pid_isolation(self):
        result = self.run_summary()
        self.assertEqual(result['gpu_metrics']['GPU Active Time']['frame_weighted_mean_ms'], 6)
        self.assertEqual(result['layer_counters']['FPS']['time_weighted_mean'], 90)
        self.assertEqual(result['analysis_seconds'], 2)

    def test_central_window_and_short_coverage_rejection(self):
        result = self.run_summary(1)
        self.assertEqual(result['gpu_metrics']['GPU Active Time']['weighted_frames'], 90)
        with self.assertRaisesRegex(ValueError, 'shorter than requested'):
            self.run_summary(60)

    def test_occluded_recording_rejection(self):
        for row in self.cs:
            row['metric-value'] = 0
        with self.assertRaisesRegex(ValueError, 'No sustained presentation'):
            self.run_summary()


if __name__ == '__main__':
    unittest.main()
