#!/usr/bin/env python3
"""Validate the proposed module without changing shared exports or consumers."""
from pathlib import Path
import re
import subprocess
import tempfile


def main():
    root = Path(__file__).resolve().parents[1]
    source = root / 'crates/tekes-ui-motion/src'
    output = root / 'target/motion-state-transition-check'
    output.mkdir(parents=True, exist_ok=True)
    # Keep normal Rust module resolution but use the actual source files, not
    # copies of implementations. Shared lib.rs is only read, never edited.
    library = re.sub(
        r'(?m)^((?:pub )?mod (\w+);)$',
        lambda m: '#[path = "' + str(source / (m[2] + '.rs')) + '"]\n' + m[1],
        (source / 'lib.rs').read_text())
    if not re.search(r'(?m)^pub mod state_transition;', library):
        library += '\n#[path = "' + str(source / 'state_transition.rs') + '"]\npub mod state_transition;\n'
    with tempfile.TemporaryDirectory(prefix='tekesui-motion-transition-') as directory:
        package = Path(directory)
        (package / 'standalone.rs').write_text(library)
        manifest = '[package]\nname = "tekes-ui-motion"\nversion = "0.0.0"\nedition = "2024"\n[workspace]\n[lib]\npath = "standalone.rs"\n'
        for test in sorted((source.parent / 'tests').glob('*.rs')):
            manifest += '\n[[test]]\nname = "' + test.stem + '"\npath = "' + str(test) + '"\n'
        manifest_path = package / 'Cargo.toml'
        manifest_path.write_text(manifest)
        for command, extra in [('test', []), ('clippy', ['--all-targets', '--', '-D', 'warnings'])]:
            args = ['cargo', '+1.95.0', command, '--offline', '--manifest-path', str(manifest_path),
                    '--target-dir', str(output / 'cargo')] + extra
            with (output / (command + '.log')).open('w') as log:
                result = subprocess.run(args, cwd=root, stdout=log, stderr=subprocess.STDOUT)
            if result.returncode:
                raise SystemExit(f'{command} failed; inspect {output / (command + ".log")}')
            print(f'{command} passed: {output / (command + ".log")}', flush=True)


if __name__ == '__main__':
    main()
