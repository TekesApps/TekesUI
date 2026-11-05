#!/usr/bin/env python3
"""Compare retained #41 logs. A source declaration alone is not source evidence."""
import argparse
import hashlib
import json
from pathlib import Path

SAMPLES = {'ascii': 'Ascii-A7', 'cjk': '原生草稿保留',
           'mixed': '原生A草稿保留', 'pinyin-first': '你好'}
SOURCES = {'cua-typeText', 'pid-hardware-key', 'local-key-characters', 'paste', 'physical-keyboard'}


def read(path):
    rows = [json.loads(line.removeprefix('UNICODE41 ')) for line in path.read_text().splitlines()
            if line.startswith('UNICODE41 ')]
    if not rows or [r['seq'] for r in rows] != list(range(1, len(rows) + 1)):
        raise ValueError('missing or unordered diagnostic records')
    return rows


def summarize(rows, control, source, case):
    if not rows or any((r['control'], r['declared_source'], r['case']) != (control, source, case) for r in rows):
        raise ValueError('control/source/case changed within a run')
    ready = [r for r in rows if r['kind'] == 'ready']
    result = [r for r in rows if r['kind'] == 'result']
    if len(ready) != 1 or len(result) != 1 or rows[-1] is not result[0]:
        raise ValueError('require one ready and one final result; preserve incomplete logs separately')
    if not ready[0]['active'] or not ready[0]['focused'] or ready[0]['value'] or ready[0]['marked'] is not None or ready[0]['selection'] != [0, 0]:
        raise ValueError('run did not start in an active, focused, empty, unmarked editor')
    text = [r for r in rows if r['kind'] in ('commit', 'preedit', 'unmark')]
    keys = [r['detail'] for r in rows if r['kind'] == 'key_context']
    stack = []
    for k in keys:
        if k['phase'] == 'start':
            stack.append(k['context_id'])
        elif k['phase'] == 'end' and stack and stack[-1] == k['context_id']:
            stack.pop()
        else:
            raise ValueError('unbalanced native key context')
    if stack:
        raise ValueError('unfinished native key context')
    final = result[0]
    detail = final['detail']
    end = len(SAMPLES[case].encode('utf-16-le')) // 2
    passed = (final['selection'] == [end, end] and final['value'] == SAMPLES[case] and final['marked'] is None and final['actions'] == 0
              and detail['expected'] == SAMPLES[case] and detail['matches']
              and detail['no_marked'] and detail['no_submit'] and detail['ready'])
    first = next((r['kind'] for r in text if r['kind'] != 'unmark'), None)
    if case == 'pinyin-first':
        passed = passed and first == 'preedit'
    return {'passed': bool(passed), 'value': final['value'], 'selection': final['selection'],
            'first_text_callback': first, 'text_callback_order': [r['kind'] for r in text],
            'text_callbacks': [r['detail'] for r in text], 'key_contexts': keys,
            'key_characters': [k['characters'] for k in keys if k['phase'] == 'start'],
            'pid': ready[0]['pid']}


def compare(appkit_path, sdk_path, receipt):
    source, case = receipt['source'], receipt['case']
    if source not in SOURCES or case not in SAMPLES:
        raise ValueError('unknown fixed source/case')
    if source == 'local-key-characters' and case == 'pinyin-first':
        raise ValueError('Unicode NSEvent is not Pinyin key input')
    results = {}
    for control, path in [('appkit', appkit_path), ('sdk', sdk_path)]:
        run = receipt['runs'][control]
        digest = hashlib.sha256(path.read_bytes()).hexdigest()
        if run['log_sha256'] != digest or not run['program_sha256'] or not run['source_evidence']:
            raise ValueError('missing source/program evidence or changed log')
        rows = read(path)
        results[control] = summarize(rows, control, source, case)
        if results[control]['pid'] != run['pid']:
            raise ValueError('source receipt refers to another process')
    a, s = results['appkit'], results['sdk']
    same_keys = a['key_characters'] == s['key_characters']
    if a['passed'] and s['passed']:
        finding = 'both_pass_for_recorded_source'
    elif a['passed'] and not s['passed'] and same_keys:
        finding = 'sdk_divergence_requires_investigation'
    else:
        finding = 'retained_failure_not_uniquely_attributed'
    return {'source': source, 'case': case, 'finding': finding, 'same_observed_key_characters': same_keys,
            'passed': a['passed'] and s['passed'], 'runs': results,
            'limits': ['Declared source is corroborated by the linked operator/driver evidence, not inferred from final text.',
                       'AppKit superclass handleEvent return is not exposed; callbacks and standard editing are observed.',
                       'Local KeyCharacters, paste and software PID keys do not prove CUA typeText or physical IME.',
                       'One passing pair does not establish stability; the first failure remains retained.']}


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--appkit', required=True, type=Path)
    p.add_argument('--sdk', required=True, type=Path)
    p.add_argument('--source-receipt', required=True, type=Path)
    p.add_argument('--output', required=True, type=Path)
    args = p.parse_args()
    result = compare(args.appkit, args.sdk, json.loads(args.source_receipt.read_text()))
    with args.output.open('x') as stream:
        json.dump(result, stream, ensure_ascii=False, indent=2)
        stream.write('\n')
    print(result['finding'])
    return 0 if result['passed'] else 1


if __name__ == '__main__':
    raise SystemExit(main())
