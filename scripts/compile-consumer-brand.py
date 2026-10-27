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
import subprocess
import sys
import tempfile

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('tekes_consumer_merge', ROOT / 'scripts/merge-design-system.py')
merge = importlib.util.module_from_spec(spec)
spec.loader.exec_module(merge)
theme_spec = importlib.util.spec_from_file_location('consumer_theme_generator', ROOT / 'scripts/generate-theme-system.py')
theme_generator = importlib.util.module_from_spec(theme_spec)
theme_spec.loader.exec_module(theme_generator)


def _copy_input(source_root, staged_root, relative):
    source = (source_root / relative).resolve(strict=True)
    if not source.is_relative_to(source_root.resolve()):
        raise ValueError('consumer Theme input escaped its declared source root: ' + str(relative))
    target = staged_root / relative
    target.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(source, target)


def _prepare_theme_inputs(staged, definition_path):
    definitions_path = Path(definition_path).resolve(strict=True)
    definitions = merge.load(definitions_path)
    if not isinstance(definitions, list) or not definitions:
        raise ValueError('consumer Theme definitions must be a non-empty array')
    # Preserve the existing config artifact input while copying each explicitly
    # named Theme BrandConfig into a stable, checkout-relative staged path.
    rewritten = []
    staged_configs = {}
    definition_root = definitions_path.parent
    for item in definitions:
        if not isinstance(item, dict) or not isinstance(item.get('brand_config'), str):
            raise ValueError('each consumer Theme definition must name a BrandConfig file')
        external = Path(item['brand_config'])
        if external.is_absolute():
            raise ValueError('consumer Theme BrandConfig paths must be relative to the definitions file')
        config = (definition_root / external).resolve(strict=True)
        if not config.is_relative_to(definition_root.resolve()):
            raise ValueError('consumer Theme BrandConfig escaped the definitions directory')
        if not config.is_file():
            raise ValueError('consumer Theme BrandConfig is not a file: ' + item['brand_config'])
        # Schema validation enforces the Theme id grammar before generation;
        # use the id only as a filename after a strict character check.
        theme_id = item.get('id', '')
        if not theme_id or not theme_id[0].islower() or any(c not in 'abcdefghijklmnopqrstuvwxyz0123456789-' for c in theme_id):
            raise ValueError('invalid consumer Theme identity')
        staged_config = staged_configs.get(config)
        if staged_config is None:
            staged_config = 'design/theme/consumer-configs/' + theme_id + '.json'
            target = staged / staged_config
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(config, target)
            staged_configs[config] = staged_config
        copied = dict(item)
        copied['brand_config'] = staged_config
        rewritten.append(copied)
    theme_dir = staged / 'design/theme'
    theme_dir.mkdir(parents=True, exist_ok=True)
    (theme_dir / 'themes.json').write_bytes(merge.bytes_json(rewritten))

    # Copy the exact compiler closure named by the shared Theme contract.
    system = merge.load(ROOT / 'design/system.json')
    contract = system['theme_system']['theme_input_contract']
    fixed = {'design/system.json', 'design/system.schema.json',
             'design/theme/rules.json', 'design/theme/rules.schema.json', 'design/theme/theme.schema.json',
             theme_generator.ARTIFACT_LOCK_SCHEMA, 'design/theme/artifact.lock.json',
             'design/source/base.tokens.json', 'design/source/token-metadata.json', 'design/brand/policy.json',
             'scripts/generate-theme-system.py', 'scripts/merge-design-system.py'}
    closure = fixed | set(contract['pinned_inputs'])
    # The source lock is carried as an artifact file for compatibility, while
    # the independently supplied copy below remains the verifier's trust input.
    closure.add(theme_generator.ARTIFACT_LOCK)
    # This input is the caller-selected projection BrandConfig staged by
    # compile_artifact; copying the package default here would sever the
    # generated projection from its recorded source bytes.
    closure.add('design/brand/config.json')
    for relative in sorted(closure - {'design/theme/themes.json', 'design/brand/config.json'}):
        _copy_input(ROOT, staged, relative)
    return rewritten


def compile_artifact(config_path, destination, theme_definitions_path=None, theme_lock_output=None):
    config_path = Path(config_path).resolve(strict=True)
    destination = Path(destination).absolute()
    if destination.exists() or destination.is_symlink():
        raise ValueError('output already exists; choose a new artifact directory')
    if destination.resolve().is_relative_to(ROOT):
        raise ValueError('consumer artifact output must be outside the TekesUI checkout')
    if theme_definitions_path is None and theme_lock_output is not None:
        raise ValueError('--theme-lock-output requires --theme-definitions')
    if theme_definitions_path is not None:
        if theme_lock_output is None:
            raise ValueError('consumer Theme definitions require an independent --theme-lock-output')
        theme_lock_output = Path(theme_lock_output).absolute()
        if theme_lock_output.exists() or theme_lock_output.is_symlink():
            raise ValueError('consumer Theme lock output already exists; choose a new file')
        if theme_lock_output.resolve().is_relative_to(ROOT) or theme_lock_output.resolve().is_relative_to(destination.resolve()):
            raise ValueError('consumer Theme lock output must be outside the TekesUI checkout and artifact directory')
        if theme_lock_output == destination:
            raise ValueError('consumer Theme lock output must be a separate file')
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
        lock_bytes = None
        if theme_definitions_path is not None:
            _prepare_theme_inputs(staged, theme_definitions_path)
            result = subprocess.run([sys.executable, str(staged / 'scripts/generate-theme-system.py'), '--write-artifact-lock'],
                                    cwd=staged, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
            if result.returncode:
                raise ValueError('consumer Theme generation failed: ' + result.stdout.strip())
            lock_bytes = (staged / 'design/theme/artifact.lock.json').read_bytes()
        staged.rename(destination)
        if lock_bytes is not None:
            try:
                theme_lock_output.parent.mkdir(parents=True, exist_ok=True)
                theme_lock_output.write_bytes(lock_bytes)
            except OSError:
                shutil.rmtree(destination)
                raise
    return destination


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--config', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--theme-definitions', type=Path,
                        help='optional consumer-owned Theme definition array using relative BrandConfig paths')
    parser.add_argument('--theme-lock-output', type=Path,
                        help='required independent output for the consumer Theme source lock')
    args = parser.parse_args()
    print(compile_artifact(args.config, args.output, args.theme_definitions, args.theme_lock_output))


if __name__ == '__main__':
    try:
        main()
    except (OSError, ValueError, KeyError, TypeError, ImportError) as error:
        print('CONSUMER_BRAND_ERROR: ' + str(error), file=sys.stderr)
        sys.exit(1)
