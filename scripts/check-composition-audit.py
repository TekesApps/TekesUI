#!/usr/bin/env python3
"""Validate the current, explicitly reviewed composition matrix; never infer acceptance.

No automatic refresh: changed source requires reviewing affected conclusions and
rebinding evidence. Historical frozen receipts are intentionally not updated.
The recorded Git baseline is provenance; exact source bytes and API anchors are
the freshness authority, so committing unchanged reviewed bytes remains valid.
"""
import argparse
import hashlib
import json
from pathlib import Path, PurePosixPath
import re

ROOT = Path(__file__).resolve().parents[1]
MATRIX = 'design/remediation/composition-audit.json'


def validate(data, root=ROOT):
    if data.get('format') != 'tekesui-composition-audit/1':
        raise ValueError('unsupported composition audit format')
    components = data.get('components', [])
    actual = [c['id'] for c in components]
    inventory = json.loads((root / 'design/gallery/capabilities.json').read_text())['components']
    expected = [c['id'] for c in inventory]
    bindings = {c['id']:c['public_api_bindings']['resolved_sources'] for c in inventory}
    if len(actual) != len(set(actual)) or set(actual) != set(expected):
        raise ValueError('composition matrix must cover every current component exactly once')
    if not isinstance(data.get('head'), str) or not re.fullmatch(r'[0-9a-f]{40}', data['head']):
        raise ValueError('immutable audit baseline commit SHA required')
    sources = data.get('source_sha256', {})
    if not sources:
        raise ValueError('audited source digests required')
    for relative, digest in sources.items():
        path = PurePosixPath(relative)
        if path.is_absolute() or '..' in path.parts:
            raise ValueError('unsafe audit source path')
        source = root / relative
        if not source.is_file() or hashlib.sha256(source.read_bytes()).hexdigest() != digest:
            raise ValueError('stale composition audit: ' + relative)
    statuses = {'已实现','部分实现','缺失','未验证','不适用'}
    profiles = data.get('profiles', {})
    for c in components:
        if c.get('kind') not in {'leaf','slots','leaf-and-slots'}:
            raise ValueError(c['id'] + ': invalid content dimension')
        if c.get('runtime_profile') not in profiles:
            raise ValueError(c['id'] + ': missing runtime collaboration profile')
        if any(c.get(key) not in statuses for key in ['content_capacity','heterogeneous_consumer_content']):
            raise ValueError(c['id'] + ': invalid capability status')
        if c.get('protocol',{}).get('status') not in statuses:
            raise ValueError(c['id'] + ': invalid protocol status')
        if c['kind']=='leaf' and (c.get('slots') or c['content_capacity']!='不适用'):
            raise ValueError(c['id'] + ': leaf must not be required to own content')
        for field in ['legacy_content','state_identity','limits','native_acceptance','accessibility','requires_internal_source_edit','compatibility']:
            if not isinstance(c.get(field),str) or not c[field].strip():
                raise ValueError(c['id'] + ': missing ' + field)
        if not c.get('evidence'):
            raise ValueError(c['id'] + ': missing public API evidence')
        if not set(bindings[c['id']]).issubset({e['api'] for e in c['evidence']}):
            raise ValueError(c['id'] + ': missing registered public API binding')
        if c['native_acceptance'] != '未验证':
            raise ValueError(c['id'] + ': source audit cannot promote native acceptance')
        for evidence in c['evidence']:
            registered = bindings[c['id']].get(evidence['api'])
            if registered and (registered['source'] != evidence['path'] or registered['line'] != evidence['line']):
                raise ValueError(c['id'] + ': registered API evidence mismatch')
            if evidence['path'] not in sources:
                raise ValueError(c['id'] + ': unpinned source evidence')
            lines = (root / evidence['path']).read_text().splitlines()
            number = evidence['line']
            if not isinstance(number,int) or number<1 or number>len(lines) or lines[number-1].strip()!=evidence['anchor']:
                raise ValueError(c['id'] + ': stale evidence line/anchor')
    if not data.get('shared_evidence'):
        raise ValueError('shared runtime collaboration evidence required')
    for evidence in data['shared_evidence']:
        if evidence['path'] not in sources:
            raise ValueError('unpinned shared runtime evidence')
        lines = (root / evidence['path']).read_text().splitlines()
        number = evidence['line']
        if not isinstance(number,int) or number<1 or number>len(lines) or lines[number-1].strip()!=evidence['anchor']:
            raise ValueError('stale shared runtime evidence line/anchor')
    return len(components)


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--matrix',type=Path,default=ROOT / MATRIX)
    args=parser.parse_args()
    count=validate(json.loads(args.matrix.read_text()))
    print(f'PASS: {count} current components, explicit content dimensions and exact source/line evidence. Native/experience acceptance is separate.')


if __name__=='__main__': main()
