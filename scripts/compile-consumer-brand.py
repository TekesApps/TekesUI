#!/usr/bin/env python3
"""Compile caller-owned BrandConfig into an offline artifact; never modify TekesUI.

Use Python with the checked-in proposal requirements. Rust builds consume the
result through TEKESUI_DESIGN_ARTIFACT, without running Python at build/runtime.
"""
import argparse
import importlib
import importlib.util
from pathlib import Path
import shutil
import sys
import tempfile

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('tekes_consumer_merge', ROOT / 'scripts/merge-design-system.py')
merge = importlib.util.module_from_spec(spec)
spec.loader.exec_module(merge)


def compile_artifact(config_path, destination):
    config_path = Path(config_path).resolve(strict=True)
    destination = Path(destination).absolute()
    if destination.exists() or destination.is_symlink():
        raise ValueError('output already exists; choose a new artifact directory')
    if destination.resolve().is_relative_to(ROOT):
        raise ValueError('consumer artifact output must be outside the TekesUI checkout')
    config = merge.load(config_path)
    # Keep imported validator resources rooted in the immutable dependency, not
    # a temporary staging path that disappears between repeated calls.
    sys.path.insert(0, str(ROOT / merge.UPSTREAM / "tools"))
    importlib.import_module("brand")
    # Stage the exact existing author/compiler inputs. Only the staged caller
    # config differs; the dependency checkout and imported package stay untouched.
    with tempfile.TemporaryDirectory(prefix='tekesui-brand-', dir=destination.parent) as temporary:
        staged = Path(temporary) / 'artifact'
        for relative in [*map(Path, merge.SOURCE_FILES), *(merge.UPSTREAM / p for p in merge.UPSTREAM_INPUTS)]:
            target = staged / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(ROOT / relative, target)
        (staged / 'design/brand/config.json').write_bytes(merge.bytes_json(config))
        document, report = merge.merge(staged)
        (staged / 'design/tokens.json').write_bytes(merge.bytes_json(document))
        (staged / 'design/proposal-mapping.json').write_bytes(merge.bytes_json(report))
        staged.rename(destination)
    return destination


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--config', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    print(compile_artifact(args.config, args.output))


if __name__ == '__main__':
    try:
        main()
    except (OSError, ValueError, KeyError, TypeError, ImportError) as error:
        print('CONSUMER_BRAND_ERROR: ' + str(error), file=sys.stderr)
        sys.exit(1)
