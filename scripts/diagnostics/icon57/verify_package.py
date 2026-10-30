#!/usr/bin/env python3
"""#57 source inventory and allocated native package probe. No builds or cache changes."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import plistlib
import re
import shutil
import subprocess
import time
import xml.etree.ElementTree as ET


def digest(data):
    return hashlib.sha256(data).hexdigest()


def source_inventory(root):
    icons = root / 'crates/tekes-ui-icons'
    manifest = json.loads((icons / 'catalog.json').read_text())
    expected = {item['name']: item['sha256'] for item in manifest['icons']}
    if len(expected) != 177 or len(manifest['icons']) != 177:
        raise ValueError('expected 177 unique canonical identities')
    glyphs = re.findall(r'name:\s*"([^"]+)",\s*svg:\s*include_str!\("([^"]+)"\)',
                        (icons / 'src/catalog.rs').read_text())
    actual = {name: digest((icons / 'src' / path).read_bytes()) for name, path in glyphs}
    if len(glyphs) != 177 or actual != expected:
        raise ValueError('Glyph::ALL bytes differ from the public catalog manifest')
    proposal_file = root / 'crates/tekes-ui-gallery/src/gallery/proposal_assets.rs'
    proposals = re.findall(
        r'\(\s*"([^"]+)",\s*"[^"]*",\s*"[^"]*",\s*'
        r'(include_str!\(\s*"[^"]+"\s*\)|tekes_ui_icons::Icon::\w+\.svg\(\))',
        proposal_file.read_text())
    if len(proposals) != 51:
        raise ValueError('proposal inventory syntax/count changed; inspect before interpreting')
    overlaps = []
    for name, expression in proposals:
        path = re.search(r'include_str!\(\s*"([^"]+)"', expression)
        previous = ((proposal_file.parent / path[1]).read_bytes() if path
                    else (icons / 'assets' / f'{name}.svg').read_bytes())
        canonical = (icons / 'assets' / f'{name}.svg').read_bytes()
        strokes = lambda data: sorted({node.attrib['stroke-width']
                                      for node in ET.fromstring(data).iter()
                                      if 'stroke-width' in node.attrib})
        overlaps.append({'name': name, 'historical_sha256': digest(previous),
                         'reference_kind': 'retained_proposal' if path else 'shared_current',
                         'canonical_sha256': digest(canonical),
                         'different': previous != canonical,
                         'historical_strokes': strokes(previous),
                         'canonical_strokes': strokes(canonical)})
    return {'layer': 'source_inventory_only', 'canonical_count': len(actual),
            'canonical': expected, 'proposal_overlap_count': len(overlaps),
            'changed_overlaps': sum(row['different'] for row in overlaps),
            'overlaps': overlaps}


def verify_report(report, manifest):
    """Validate actual runtime-resolved SVG bytes against the packaged public manifest."""
    if report.get('schema') != 'tekesui-icon-catalog/1' or report.get('mode') != 'canonical':
        raise ValueError('runtime probe did not use default canonical mode')
    if report.get('notices') != []:
        raise ValueError('runtime catalog has loading errors')
    entries = report['entries']
    expected = {row['name']: row['sha256'] for row in manifest['icons']}
    actual = {row['name']: digest(row['svg'].encode()) for row in entries}
    if (len(expected) != 177 or len(manifest['icons']) != 177 or len(entries) != 177
            or report.get('count') != 177 or actual != expected):
        raise ValueError('runtime resolved count, names or artwork differ from packaged canonical manifest')
    if any(row['canonical_match'] is not True or row['before'] is not False for row in entries):
        raise ValueError('default runtime has a noncanonical or comparison entry')
    return {'layer': 'runtime_catalog_resolution', 'count': 177, 'exact_artwork': True,
            'pixel_acceptance': 'not_performed_by_this_probe'}


def sandbox_profile(source_roots):
    return '(version 1)\n(allow default)\n' + ''.join(
        f'(deny file-read* (subpath {json.dumps(str(root))}))\n' for root in source_roots)


def run_package(app, source_roots, output, timeout):
    """Run only in the integrator's allocated native window. Never rename a checkout."""
    app = app.resolve(strict=True)
    source_roots = [root.resolve(strict=True) for root in source_roots]
    output = output.resolve()
    if any(output == root or root in output.parents for root in source_roots):
        raise ValueError('native evidence/package copy must be outside every blocked checkout')
    output.mkdir(parents=True, exist_ok=False)
    copied = output / app.name
    shutil.copytree(app, copied, symlinks=True)
    contents = copied / 'Contents'
    manifest = json.loads((contents / 'Resources/IconCatalog.json').read_text())
    info = plistlib.loads((contents / 'Info.plist').read_bytes())
    binary = contents / 'MacOS' / info['CFBundleExecutable']
    profile = sandbox_profile(source_roots)
    (output / 'no-source.sb').write_text(profile)
    result = {'layer': 'real_package_outside_checkout', 'status': 'failed',
              'app': str(copied), 'binary_sha256': digest(binary.read_bytes()),
              'blocked_source_roots': [str(root) for root in source_roots],
              'source_denial': [], 'native_pixels': 'unverified'}
    process = None
    try:
        for root in source_roots:
            sentinel = root / 'Cargo.toml'
            sentinel.read_bytes()  # The sentinel must exist and be readable without the sandbox.
            blocked = subprocess.run(['/usr/bin/sandbox-exec', '-p', profile, '/bin/cat', str(sentinel)],
                                     stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False)
            stderr = blocked.stderr.decode(errors='replace')
            result['source_denial'].append({'path': str(sentinel), 'exit': blocked.returncode,
                                            'stderr': stderr})
            if blocked.returncode == 0 or 'Operation not permitted' not in stderr:
                raise RuntimeError('checkout read-denial preflight failed; no app launch')
        report_path = output / 'runtime-catalog.json'
        environment = {key: value for key, value in os.environ.items()
                       if not key.startswith(('TEKESUI_', 'TEKES_SELFTEST', 'TEKES_PERF'))}
        environment.update(TEKESUI_ICON_CATALOG_REPORT=str(report_path), TEKESUI_GALLERY_PAGE='icons')
        with (output / 'native.log').open('wb') as log:
            process = subprocess.Popen(['/usr/bin/sandbox-exec', '-p', profile, str(binary)],
                                       cwd=output, env=environment, stdout=log, stderr=subprocess.STDOUT)
            deadline = time.monotonic() + timeout
            while not report_path.exists():
                if process.poll() is not None:
                    raise RuntimeError(f'Gallery exited before its report: {process.returncode}')
                if time.monotonic() >= deadline:
                    raise TimeoutError('Gallery catalog probe timed out')
                time.sleep(0.1)
            # Keep the real launch alive beyond the early catalog construction.
            time.sleep(3)
            if process.poll() is not None:
                raise RuntimeError(f'Gallery exited after catalog construction: {process.returncode}')
            result['catalog'] = verify_report(json.loads(report_path.read_text()), manifest)
            result['status'] = 'passed'
    except Exception as error:
        result['error'] = str(error)
        raise
    finally:
        if process is not None and process.poll() is None:
            process.terminate()  # Only this probe's own child process.
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()
        (output / 'result.json').write_text(json.dumps(result, indent=2) + '\n')
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest='command', required=True)
    inventory = commands.add_parser('inventory', help='source only; no native launch')
    inventory.add_argument('--root', type=Path, required=True)
    inventory.add_argument('--out', type=Path, required=True)
    native = commands.add_parser('run', help='requires allocated native window; launches one app')
    native.add_argument('--app', type=Path, required=True)
    native.add_argument('--source-root', type=Path, action='append', required=True,
                        help='include the actual compiled Cargo manifest root; repeat for relevant checkouts')
    native.add_argument('--out', type=Path, required=True)
    native.add_argument('--timeout', type=float, default=30)
    args = parser.parse_args()
    if args.command == 'inventory':
        result = source_inventory(args.root.resolve(strict=True))
        args.out.parent.mkdir(parents=True, exist_ok=True)
        with args.out.open('x') as stream:
            stream.write(json.dumps(result, indent=2) + '\n')
    else:
        result = run_package(args.app, args.source_root, args.out, args.timeout)
    print(json.dumps({key: value for key, value in result.items() if key not in ('canonical', 'overlaps')}))


if __name__ == '__main__':
    main()
