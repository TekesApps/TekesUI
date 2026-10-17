#!/usr/bin/env python3
"""Validate trace reference decoding and missing-data boundaries."""
import importlib.util
import gzip
import hashlib
import json
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location(
    'metal_analysis', Path(__file__).with_name('analyze-metal-trace.py'))
analysis = importlib.util.module_from_spec(spec)
spec.loader.exec_module(analysis)


class TraceAnalysisTests(unittest.TestCase):
    def parse(self, body):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'table.xml'
            path.write_text(body)
            return analysis.table(path)

    def test_references_preserve_numeric_units_process_and_labels(self):
        rows = self.parse('''<trace-query-result><node><schema>
          <col><mnemonic>start</mnemonic></col><col><mnemonic>duration</mnemonic></col>
          <col><mnemonic>thread</mnemonic></col><col><mnemonic>label</mnemonic></col>
          </schema><row><start-time id="1" fmt="00:00.001">1000000</start-time>
          <duration id="2" fmt="8.33 ms">8333333</duration>
          <thread id="3"><process id="4"><pid id="5">42</pid></process></thread>
          <narrative id="6" fmt="render frame"><string>nested text</string></narrative>
          </row><row><start-time ref="1"/><duration ref="2"/>
          <thread ref="3"/><narrative ref="6"/></row></node></trace-query-result>''')
        self.assertEqual(rows, [dict(start=1000000, duration=8333333, thread=42,
                                     label='render frame')] * 2)

    def test_unknown_value_is_missing_not_zero(self):
        self.assertEqual(self.parse('''<root><schema><col><mnemonic>duration</mnemonic>
          </col></schema><row><sentinel/></row></root>'''), [{'duration': None}])
        self.assertIsNone(analysis.distribution_ns([]))

    def test_dangling_reference_fails(self):
        with self.assertRaises(KeyError):
            self.parse('''<root><schema><col><mnemonic>start</mnemonic></col></schema>
              <row><start-time ref="missing"/></row></root>''')

    def test_nanoseconds_convert_once_without_clamping(self):
        stats = analysis.distribution_ns([-1000000, 0, 9000000])
        self.assertEqual(stats['count'], 3)
        self.assertEqual(stats['mean_ms'], 8 / 3)
        self.assertEqual(stats['max_ms'], 9)
        self.assertEqual(stats['p50_ms'], 0)

    def test_target_filter_id_join_and_unmatched_frame(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory)
            (path / 'capture-receipt.json').write_text(json.dumps({'complete': True, 'pid': 42}))
            tables = {
                'metal-application-command-buffer-submissions': [
                    dict(process=42, start=1000000, duration=100000,
                         **{'event-label': 'tekes frame pass', 'cmdbuffer-id': 100}),
                    dict(process=42, start=5000000, duration=100000,
                         **{'event-label': 'tekes frame pass', 'cmdbuffer-id': 101}),
                    dict(process=999, start=1, duration=1,
                         **{'event-label': 'tekes frame pass', 'cmdbuffer-id': 200})],
                'metal-gpu-intervals': [
                    dict(process=42, state='Active', start=2000000, duration=1000000,
                         **{'cmdbuffer-id': 100, 'channel-name': 'Fragment'}),
                    dict(process=999, state='Active', start=1, duration=999999999,
                         **{'cmdbuffer-id': 200, 'channel-name': 'Fragment'}),
                    dict(process=42, state='Waiting', start=1, duration=999999999,
                         **{'cmdbuffer-id': 100, 'channel-name': 'Fragment'})],
                'metal-command-buffer-completed': [dict(timestamp=4000000, **{'cmdbuffer-id': 100})],
                'ca-client-buffer-wait-interval': [dict(thread=42, start=1, duration=9000000),
                                                  dict(thread=999, start=1, duration=999999999)]}
            raw = gzip.compress(json.dumps({'format': 'tekesui-target-metal-tables/1',
                                            'pid': 42, 'tables': tables}).encode())
            (path / 'target-tables.json.gz').write_bytes(raw)
            (path / 'manifest.json').write_text(json.dumps({
                'target-tables.json.gz': {'sha256': hashlib.sha256(raw).hexdigest()}}))
            result = analysis.analyze(path)
            self.assertEqual(result['target_submissions'], 2)
            self.assertEqual(result['joined_render_buffers'], 1)
            self.assertEqual(result['render_submissions_without_gpu_match'], 1)
            self.assertEqual(result['excluded_non_active_gpu_intervals'], 1)
            self.assertEqual(result['render_gpu_span']['max_ms'], 1)
            self.assertEqual(result['render_commit_to_completion']['max_ms'], 2.9)
            self.assertEqual(result['drawable_waits_over_8_33ms'], 1)
            (path / 'target-tables.json.gz').write_bytes(raw + b'corruption')
            with self.assertRaisesRegex(ValueError, 'checksum mismatch'):
                analysis.analyze(path)

    def test_incomplete_capture_is_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory)
            (path / 'receipt.json').write_text('{"complete": false}')
            with self.assertRaisesRegex(ValueError, 'did not complete'):
                analysis.analyze(path)


if __name__ == '__main__':
    unittest.main()
