"""Local review build policy. No UI launch, source changes, or Git writes."""
from contextlib import contextmanager
import fcntl
import json
import os
from pathlib import Path
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[1]
CACHE = ROOT / 'target/review-cache'
POLICY = {'CARGO_PROFILE_DEV_DEBUG': '0', 'CARGO_PROFILE_TEST_DEBUG': '0',
          'CARGO_PROFILE_DEV_INCREMENTAL': 'false', 'CARGO_PROFILE_TEST_INCREMENTAL': 'false'}


@contextmanager
def build_lock():
    path = ROOT / 'target/.component-review.lock'
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open('a') as stream:
        try:
            fcntl.flock(stream, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            raise SystemExit('Another review build/cleanup owns the shared cache')
        yield


def environment():
    return dict(os.environ, **POLICY, CARGO_TARGET_DIR=str(CACHE))


def prepare_cache(workspace_root=None):
    if CACHE.is_symlink() or CACHE.resolve() != CACHE.absolute():
        raise SystemExit('Refusing symlinked cache')
    identity = {'owner': 'tekesui-component-review-v1', 'root': str(ROOT), 'policy': POLICY,
                'rustc': subprocess.check_output(['rustc', '-vV'], text=True, cwd=ROOT),
                'overrides': {key: os.environ.get(key) for key in
                              ('RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'TEKESUI_DESIGN_ARTIFACT',
                               'CARGO_BUILD_TARGET', 'RUSTC_WRAPPER')}}
    marker = CACHE / '.tekesui-cache.json'
    if marker.exists():
        if json.loads(marker.read_text()) != identity:
            raise SystemExit('Cache configuration changed; finish rounds, then run cache-reset')
    else:
        if CACHE.exists() and any(CACHE.iterdir()):
            raise SystemExit('Refusing to adopt an unowned build cache')
        CACHE.mkdir(parents=True, exist_ok=True)
        marker.write_text(json.dumps(identity, indent=2) + '\n')
    # Cargo may reuse identically named path-package artifacts when two
    # checkouts share a target directory and the returning sources are older.
    # Keep one latest workspace lane; retain compiled registry dependencies.
    workspace = Path(workspace_root or ROOT).resolve()
    if (workspace / 'Cargo.toml').is_file():
        owner = CACHE / '.tekesui-workspace.json'
        previous = json.loads(owner.read_text()) if owner.exists() else None
        if previous is not None and previous.get('format') != 'tekesui-cache-workspace/1':
            raise SystemExit('Unknown workspace-cache ownership; cleanup refused')
        if previous is None or previous.get('root') != str(workspace):
            assert_unused([CACHE])
            scopes = [workspace]
            consumer = workspace / 'examples/composition-consumer'
            if (consumer / 'Cargo.toml').is_file():
                scopes.append(consumer)
            packages, cleanup_output = set(), []
            for scope in scopes:
                metadata = json.loads(subprocess.check_output(
                    ['cargo', 'metadata', '--no-deps', '--format-version', '1', '--locked', '--offline'],
                    cwd=scope, text=True))
                local = sorted({p['name'] for p in metadata['packages'] if p.get('source') is None})
                if not local:
                    raise SystemExit('No local workspace packages; cache ownership unchanged')
                command = ['cargo', 'clean', '--locked', '--offline', '--target-dir', str(CACHE)]
                for package in local:
                    command.extend(['--package', package])
                cleanup = subprocess.run(command, cwd=scope, env=environment(),
                                         capture_output=True, text=True)
                if cleanup.returncode:
                    raise SystemExit('Local package cache reset failed: ' + cleanup.stderr)
                packages.update(local)
                cleanup_output.append(cleanup.stdout + cleanup.stderr)
            owner.write_text(json.dumps({'format': 'tekesui-cache-workspace/1',
                                         'root': str(workspace), 'packages': sorted(packages),
                                         'previous_root': previous.get('root') if previous else None,
                                         'cleanup': ''.join(cleanup_output)}, indent=2) + '\n')
    return CACHE


def assert_unused(paths):
    """Fail closed; check executable mappings and cwd as well as open files."""
    paths = [Path(p) for p in paths]
    if any(p.is_symlink() or p.resolve() != p.absolute() for p in paths):
        raise SystemExit('Refusing symlinked cleanup paths')
    result = subprocess.run(['lsof', '-nP', '-Fpn'], capture_output=True, text=True)
    if result.returncode:
        raise SystemExit('Cannot establish complete open-file inventory; cleanup refused')
    for line in result.stdout.splitlines():
        if not line.startswith('n/'):
            continue
        opened = line[1:]
        for path in paths:
            if opened == str(path) or opened.startswith(str(path) + '/'):
                raise SystemExit('Cleanup target is in use: ' + str(path))


def remove_owned(path, parent):
    path, parent = Path(path), Path(parent)
    if path == parent or parent not in path.parents or path.resolve() != path.absolute():
        raise SystemExit('Cleanup path escaped its owned directory')
    if path.is_symlink():
        raise SystemExit('Refusing symlink cleanup')
    if path.is_dir():
        shutil.rmtree(path)
    elif path.exists():
        path.unlink()
