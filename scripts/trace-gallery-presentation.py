#!/usr/bin/env python3
"""Capture Metal queue/presentation diagnostics, never a quiet performance pass.

Attach after launch and finish recording before the harness exits. A successful
record alone is insufficient: export the TOC and individual event tables too.
Launch with an allowlisted environment so traces do not capture shell secrets.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import time
import xml.etree.ElementTree as ET


TABLES = (
    'metal-application-command-buffer-submissions',
    'metal-command-buffer-completed',
    'metal-gpu-intervals',
    'ca-client-buffer-wait-interval',
    'displayed-surfaces-interval',
    'display-surface-swap',
    'display-vsyncs-interval',
    'metal-object-label',
)


def usable_queue_capture(receipt):
    """Export success and nonempty queue/GPU data are separate requirements."""
    return receipt.get('complete', False) and all(
        receipt['tables'].get(schema, {}).get('rows', 0) > 0 for schema in (
            'metal-application-command-buffer-submissions',
            'metal-command-buffer-completed', 'metal-gpu-intervals'))


def environment(out, driver, seconds):
    env = {key: os.environ[key] for key in
           ('HOME', 'USER', 'LOGNAME', 'TMPDIR', 'LANG') if key in os.environ}
    env.update(PATH='/usr/bin:/bin:/usr/sbin:/sbin',
               TEKES_PERF=f'scroll:{seconds}', TEKES_PERF_CONTENT='scroll-area',
               TEKES_PERF_WORKBENCH='1', TEKES_PERF_GPU_TIMING='0',
               TEKES_PERF_FRAME_LATENCY='2',
               TEKES_METAL_DISPLAY_LINK='1' if driver == 'metal' else '0',
               TEKESUI_GALLERY_LOCALE='en', TEKESUI_WINDOW_PLACEMENT='builtin',
               TEKES_PERF_OUT=str(out / 'perf.json'))
    return env


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--out', type=Path, required=True)
    parser.add_argument('--driver', choices=('cad', 'metal'), default='cad')
    parser.add_argument('--trace-seconds', type=int, default=40)
    args = parser.parse_args()
    if not 5 <= args.trace_seconds <= 120:
        parser.error('trace duration must be 5..120 seconds')
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=False)
    binary = args.binary.resolve(strict=True)
    checksum = hashlib.sha256(binary.read_bytes()).hexdigest()
    receipt = {'format': 'tekesui-metal-trace/1', 'diagnostic_only': True,
               'quiet_certified': False, 'driver': args.driver,
               'binary': str(binary), 'binary_sha256': checksum,
               'tables': {}, 'complete': False}

    def save():
        (out / 'receipt.json').write_text(json.dumps(receipt, indent=2) + '\n')

    def run(command, log, timeout):
        with (out / log).open('w') as stream:
            result = subprocess.run(command, stdout=stream, stderr=subprocess.STDOUT,
                                    env=environment(out, args.driver, 0), timeout=timeout)
        return result.returncode

    save()
    with (out / 'gallery.log').open('w') as log:
        app = subprocess.Popen([str(binary)], stdout=log, stderr=subprocess.STDOUT,
                               env=environment(out, args.driver, args.trace_seconds + 25))
        receipt['pid'] = app.pid
        try:
            time.sleep(4)
            if app.poll() is not None:
                raise RuntimeError('Gallery exited before attach')
            trace = out / 'metal.trace'
            command = ['xcrun', 'xctrace', 'record', '--template', 'Metal System Trace',
                       '--time-limit', f'{args.trace_seconds}s', '--output', str(trace),
                       '--attach', str(app.pid)]
            receipt['record_command'] = command
            # Metal System Trace finalizes several GB after the timed interval.
            # A short subprocess timeout can kill a healthy collector mid-save.
            receipt['record_exit'] = run(command, 'record.log', args.trace_seconds + 600)
            receipt['gallery_alive_after_record'] = app.poll() is None
            receipt['gallery_exit'] = app.wait(timeout=150)
            toc = out / 'toc.xml'
            receipt['toc_exit'] = run(['xcrun', 'xctrace', 'export', '--input', str(trace),
                                       '--toc', '--output', str(toc)], 'toc.log', 120)
            if receipt['toc_exit'] == 0:
                root = ET.parse(toc)
                # Never retain environment values in reviewable exported metadata.
                for env in root.findall('.//environment'):
                    env.clear()
                root.write(toc, encoding='utf-8', xml_declaration=True)
                schemas = {t.get('schema') for t in root.findall('.//table')}
                for schema in TABLES:
                    if schema not in schemas:
                        receipt['tables'][schema] = {'missing_schema': True}
                        continue
                    target = out / f'{schema}.xml'
                    xpath = f'/trace-toc/run[@number="1"]/data/table[@schema="{schema}"]'
                    code = run(['xcrun', 'xctrace', 'export', '--input', str(trace),
                                '--xpath', xpath, '--output', str(target)],
                               f'{schema}.log', 120)
                    entry = {'exit': code}
                    if code == 0:
                        entry['rows'] = sum(1 for _, el in ET.iterparse(target)
                                            if el.tag == 'row')
                        entry['sha256'] = hashlib.sha256(target.read_bytes()).hexdigest()
                    receipt['tables'][schema] = entry
                    save()
            receipt['binary_unchanged'] = hashlib.sha256(binary.read_bytes()).hexdigest() == checksum
            receipt['complete'] = (receipt['record_exit'] == 0 and receipt['toc_exit'] == 0
                                   and receipt['gallery_exit'] == 0 and receipt['binary_unchanged']
                                   and all(t.get('exit') == 0 for t in receipt['tables'].values())
                                   and len(receipt['tables']) == len(TABLES))
            receipt['usable_queue_capture'] = usable_queue_capture(receipt)
        except Exception as error:
            receipt['error'] = str(error)
            raise
        finally:
            if app.poll() is None:
                app.terminate()
                try:
                    app.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    app.kill()
                    app.wait()
            save()
    print(json.dumps(receipt, indent=2))
    return 0 if receipt.get('usable_queue_capture') else 1


if __name__ == '__main__':
    raise SystemExit(main())
