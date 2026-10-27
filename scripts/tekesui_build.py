"""Local review build policy. No UI launch, source changes, or Git writes."""
from contextlib import contextmanager
import fcntl
import hashlib
import json
import os
from pathlib import Path
import shutil
import stat
import subprocess

ROOT = Path(__file__).resolve().parents[1]
CACHE = ROOT / 'target/review-cache'
TARGET_ROOT = ROOT / 'target'
STORAGE_REGISTRY = Path('/Users/frankqin/.codex/storage-migrations/tekesui-targets.json')
STORAGE_RECEIPT = Path('/Users/frankqin/.codex/storage-migrations/tekesui-20261024-build-artifacts.json')
EXPECTED_VOLUME_UUID = '4289CF6D-F09D-4831-8558-4AB223E4EE04'
REGISTRY_FORMAT = 'tekesui-target-mappings/1'
RECEIPT_FORMAT = 'tekesui-target-migration-receipt/1'
POLICY = {'CARGO_PROFILE_DEV_DEBUG': '0', 'CARGO_PROFILE_TEST_DEBUG': '0',
          'CARGO_PROFILE_DEV_INCREMENTAL': 'false', 'CARGO_PROFILE_TEST_INCREMENTAL': 'false'}


def _find_volume_mountpoint(path, is_mount=None):
    """Find the nearest existing mounted ancestor without inventing a UUID."""
    path = Path(path)
    mount_test = is_mount or Path.is_mount
    if not path.exists() or not path.is_dir():
        return None
    for candidate in (path, *path.parents):
        if candidate == Path(candidate.anchor):
            break
        if candidate.exists() and candidate.is_dir() and mount_test(candidate):
            return candidate
    return None


def _disk_volume_uuid(path, *, mountpoint_lookup=None, check_output=None):
    """Query diskutil for the volume containing path, failing closed if absent."""
    find_mountpoint = mountpoint_lookup or _find_volume_mountpoint
    mountpoint = find_mountpoint(path)
    if mountpoint is None or Path(mountpoint) == Path(Path(mountpoint).anchor):
        return None
    run = check_output or subprocess.check_output
    try:
        raw = run(['diskutil', 'info', '-plist', str(mountpoint)])
        info = __import__('plistlib').loads(raw)
        return info.get('VolumeUUID') or info.get('APFSVolumeUUID')
    except (OSError, subprocess.CalledProcessError, ValueError):
        return None


def _checked_regular_file(path, uid, description):
    flags = os.O_RDONLY | getattr(os, 'O_NOFOLLOW', 0)
    try:
        descriptor = os.open(path, flags)
    except OSError as exc:
        raise SystemExit(f'{description} is unavailable or symlinked: {exc}')
    try:
        info = os.fstat(descriptor)
        if not stat.S_ISREG(info.st_mode) or info.st_uid != uid:
            raise SystemExit(f'{description} must be a regular file owned by uid {uid}')
        with os.fdopen(descriptor, 'rb', closefd=False) as stream:
            return stream.read()
    finally:
        os.close(descriptor)


def _canonical_absolute(value, description):
    if not isinstance(value, str):
        raise SystemExit(f'Invalid {description}')
    path = Path(value)
    if not path.is_absolute() or '..' in path.parts or str(path) != value:
        raise SystemExit(f'Invalid {description}: {value}')
    return path


def _load_target_registry(root=None, registry_path=None):
    root = Path(root if root is not None else ROOT)
    registry_path = Path(registry_path if registry_path is not None else STORAGE_REGISTRY)
    target = root / 'target'
    if not registry_path.exists() and not registry_path.is_symlink():
        if target.is_symlink():
            raise SystemExit('Target symlink has no storage registry authorization')
        return None
    if registry_path.is_symlink():
        raise SystemExit('Storage registry must not be a symlink')
    uid = os.getuid()
    raw = _checked_regular_file(registry_path, uid, 'Storage registry')
    try:
        registry = json.loads(raw)
    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise SystemExit('Invalid storage registry JSON: ' + str(exc))
    if registry.get('format') != REGISTRY_FORMAT or registry.get('owner_uid') != uid:
        raise SystemExit('Storage registry format or owner mismatch')
    mappings = registry.get('mappings')
    if not isinstance(mappings, list) or not mappings:
        raise SystemExit('Storage registry mappings must be a nonempty list')
    logicals, physicals = [], []
    normalized = []
    for entry in mappings:
        if not isinstance(entry, dict):
            raise SystemExit('Invalid storage mapping')
        logical = _canonical_absolute(entry.get('logical_root'), 'logical root')
        physical = _canonical_absolute(entry.get('physical_root'), 'physical root')
        if entry.get('volume_uuid') != EXPECTED_VOLUME_UUID:
            raise SystemExit('Storage mapping volume UUID mismatch')
        if logical in logicals or physical in physicals:
            raise SystemExit('Duplicate storage mapping root')
        logicals.append(logical)
        physicals.append(physical)
        normalized.append((logical, physical, entry['volume_uuid']))
    for roots in (logicals, physicals, logicals + physicals):
        for i, left in enumerate(roots):
            if any(left in right.parents or right in left.parents for right in roots[i + 1:]):
                raise SystemExit('Overlapping storage mapping roots')
    receipt_path = _canonical_absolute(registry.get('receipt'), 'receipt path')
    if receipt_path != STORAGE_RECEIPT:
        raise SystemExit('Migration receipt path differs from the fixed receipt location')
    receipt_raw = _checked_regular_file(receipt_path, uid, 'Migration receipt')
    if hashlib.sha256(receipt_raw).hexdigest() != registry.get('receipt_sha256'):
        raise SystemExit('Migration receipt SHA256 mismatch')
    try:
        receipt = json.loads(receipt_raw)
    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise SystemExit('Invalid migration receipt JSON: ' + str(exc))
    if receipt.get('format') != RECEIPT_FORMAT or receipt.get('status') != 'complete':
        raise SystemExit('Migration receipt is not complete')
    receipt_mappings = receipt.get('mappings')
    if not isinstance(receipt_mappings, list) or len(receipt_mappings) != len(normalized):
        raise SystemExit('Migration receipt mapping set mismatch')
    receipt_routes = []
    for item in receipt_mappings:
        if not isinstance(item, dict) or item.get('verified') is not True or item.get('source_removed') is not True:
            raise SystemExit('Migration receipt lacks verified completion evidence')
        logical = _canonical_absolute(item.get('logical_root'), 'receipt logical root')
        physical = _canonical_absolute(item.get('physical_root'), 'receipt physical root')
        receipt_routes.append((logical, physical))
    expected_routes = [(logical, physical) for logical, physical, _ in normalized]
    if len(set(receipt_routes)) != len(receipt_routes) or set(receipt_routes) != set(expected_routes):
        raise SystemExit('Migration receipt routes do not exactly match registry')
    return normalized


def _reject_parent_segments(path, description):
    if '..' in Path(path).parts:
        raise SystemExit(f'{description} contains a parent traversal component')


def _is_within(path, parent):
    try:
        Path(path).relative_to(Path(parent))
        return True
    except ValueError:
        return False


def _resolve_target_path(path, *, root=None, registry_path=None, volume_uuid_lookup=None):
    """Resolve only the exact ROOT/target symlink authorized by the registry."""
    root = Path(root if root is not None else ROOT)
    _reject_parent_segments(root, 'Target root')
    _reject_parent_segments(path, 'Target path')
    registry_path = registry_path if registry_path is not None else STORAGE_REGISTRY
    target = root / 'target'
    path = Path(os.path.abspath(path))
    target = Path(os.path.abspath(target))
    if not _is_within(path, target):
        return path
    registry = _load_target_registry(root, registry_path)
    routes = [item for item in (registry or []) if item[0] == target]
    if target.is_symlink():
        if len(routes) != 1:
            raise SystemExit('Target symlink lacks one exact registry mapping')
        _, physical, volume_uuid = routes[0]
        if os.readlink(target) != str(physical):
            raise SystemExit('Target symlink destination changed from registry')
        cursor = Path(physical.anchor)
        for component in physical.parts[1:]:
            cursor = cursor / component
            if cursor.is_symlink():
                raise SystemExit('Mapped physical target contains a symlink component')
        if not physical.is_dir():
            raise SystemExit('Mapped target volume is missing')
        lookup = volume_uuid_lookup or _disk_volume_uuid
        if lookup(physical) != volume_uuid:
            raise SystemExit('Mapped target volume UUID is unavailable or incorrect')
        mapped = physical / path.relative_to(target)
        cursor = physical
        for part in path.relative_to(target).parts:
            cursor = cursor / part
            if cursor.is_symlink():
                raise SystemExit('Refusing symlink within mapped target')
        return mapped
    if routes:
        raise SystemExit('Registered target symlink was replaced by a native path')
    cursor = target
    for part in path.relative_to(target).parts:
        cursor = cursor / part
        if cursor.is_symlink():
            raise SystemExit('Refusing symlink within target')
    return path


@contextmanager
def build_lock():
    logical_path = TARGET_ROOT / '.component-review.lock'
    path = _resolve_target_path(logical_path)
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open('a') as stream:
        try:
            fcntl.flock(stream, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            raise SystemExit('Another review build/cleanup owns the shared cache')
        yield


def environment():
    _resolve_target_path(CACHE)
    return dict(os.environ, **POLICY, CARGO_TARGET_DIR=str(CACHE))

def prepare_cache(workspace_root=None, *, rebind_local_packages=False):
    """Prepare this workspace lane; explicit rebind refreshes only its local packages.

    Rebinding is intentionally limited to an existing, correctly owned cache whose
    current workspace marker already names the requested workspace. It leaves the
    cache configuration owner and registry dependencies untouched.
    """
    if type(rebind_local_packages) is not bool:
        raise SystemExit('rebind_local_packages must be an explicit bool')
    workspace = Path(workspace_root or ROOT).resolve()
    if rebind_local_packages and not (workspace / 'Cargo.toml').is_file():
        raise SystemExit('Cannot rebind local packages for a missing workspace')

    cache_path = _resolve_target_path(CACHE)
    marker = cache_path / '.tekesui-cache.json'
    owner = cache_path / '.tekesui-workspace.json'
    if rebind_local_packages and (marker.is_symlink() or owner.is_symlink() or
                                 not marker.is_file() or not owner.is_file()):
        raise SystemExit('Rebind requires existing cache and workspace ownership markers')

    identity = {'owner': 'tekesui-component-review-v1', 'root': str(ROOT), 'policy': POLICY,
                'rustc': subprocess.check_output(['rustc', '-vV'], text=True, cwd=ROOT),
                'overrides': {key: os.environ.get(key) for key in
                              ('RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'TEKESUI_DESIGN_ARTIFACT',
                               'CARGO_BUILD_TARGET', 'RUSTC_WRAPPER')}}
    if marker.exists():
        if json.loads(marker.read_text()) != identity:
            raise SystemExit('Cache configuration changed; finish rounds, then run cache-reset')
    else:
        if cache_path.exists() and any(cache_path.iterdir()):
            raise SystemExit('Refusing to adopt an unowned build cache')
        cache_path.mkdir(parents=True, exist_ok=True)
        marker.write_text(json.dumps(identity, indent=2) + '\n')

    # Cargo may reuse identically named path-package artifacts when two
    # checkouts share a target directory and the returning sources are older.
    # Keep one latest workspace lane; retain compiled registry dependencies.
    if (workspace / 'Cargo.toml').is_file():
        previous = None
        if owner.exists():
            try:
                previous = json.loads(owner.read_text())
            except (OSError, json.JSONDecodeError) as exc:
                raise SystemExit('Invalid workspace-cache ownership; cleanup refused: ' + str(exc))
            if not isinstance(previous, dict) or previous.get('format') != 'tekesui-cache-workspace/1':
                raise SystemExit('Unknown workspace-cache ownership; cleanup refused')
        if rebind_local_packages and (previous is None or previous.get('root') != str(workspace)):
            raise SystemExit('Rebind is allowed only for the current workspace owner')
        same_owner_rebind = rebind_local_packages
        if previous is None or previous.get('root') != str(workspace) or same_owner_rebind:
            # Validate occupancy before metadata queries or any cache mutation.
            assert_unused([CACHE])
            scopes = [workspace]
            consumer = workspace / 'examples/composition-consumer'
            if (consumer / 'Cargo.toml').is_file():
                scopes.append(consumer)

            # Resolve every local package set and command before cleaning any of
            # them, so malformed/empty metadata cannot leave a partial rebind.
            plans = []
            packages = set()
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
                plans.append((scope, command))
                packages.update(local)

            cleanup_output = []
            for scope, command in plans:
                cleanup = subprocess.run(command, cwd=scope, env=environment(),
                                         capture_output=True, text=True)
                if cleanup.returncode:
                    raise SystemExit('Local package cache reset failed: ' + cleanup.stderr)
                cleanup_output.append(cleanup.stdout + cleanup.stderr)
            owner.write_text(json.dumps({'format': 'tekesui-cache-workspace/1',
                                         'root': str(workspace), 'packages': sorted(packages),
                                         'previous_root': previous.get('root') if previous else None,
                                         'cleanup': ''.join(cleanup_output)}, indent=2) + '\n')
    elif rebind_local_packages:
        raise SystemExit('Cannot rebind local packages for a missing workspace')
    return CACHE


def assert_unused(paths):
    """Fail closed; check executable mappings and cwd as well as open files."""
    logical_paths = [Path(p) for p in paths]
    checked_paths = []
    for path in logical_paths:
        _reject_parent_segments(path, 'Cleanup path')
        if _is_within(Path(os.path.abspath(path)), Path(os.path.abspath(TARGET_ROOT))):
            physical = _resolve_target_path(path)
            checked_paths.extend([Path(os.path.abspath(path)), physical])
        else:
            if path.is_symlink() or path.resolve() != path.absolute():
                raise SystemExit('Refusing symlinked cleanup paths')
            checked_paths.append(path)
    unique_paths = list(dict.fromkeys(checked_paths))
    result = subprocess.run(['lsof', '-nP', '-Fpn'], capture_output=True, text=True)
    if result.returncode:
        raise SystemExit('Cannot establish complete open-file inventory; cleanup refused')
    for line in result.stdout.splitlines():
        if not line.startswith('n/'):
            continue
        opened = line[1:]
        for path in unique_paths:
            if opened == str(path) or opened.startswith(str(path) + '/'):
                raise SystemExit('Cleanup target is in use: ' + str(path))


def remove_owned(path, parent):
    path, parent = Path(path), Path(parent)
    _reject_parent_segments(path, 'Cleanup path')
    _reject_parent_segments(parent, 'Cleanup parent')
    path_in_target = _is_within(Path(os.path.abspath(path)), Path(os.path.abspath(TARGET_ROOT)))
    parent_in_target = _is_within(Path(os.path.abspath(parent)), Path(os.path.abspath(TARGET_ROOT)))
    if path_in_target != parent_in_target:
        raise SystemExit('Cleanup path escaped its owned directory')
    if path_in_target:
        path = _resolve_target_path(path)
        parent = _resolve_target_path(parent)
        if path == parent or parent not in path.parents:
            raise SystemExit('Cleanup path escaped its owned directory')
    elif path == parent or parent not in path.parents or path.resolve() != path.absolute():
        raise SystemExit('Cleanup path escaped its owned directory')
    if path.is_symlink():
        raise SystemExit('Refusing symlink cleanup')
    if path.is_dir():
        shutil.rmtree(path)
    elif path.exists():
        path.unlink()
