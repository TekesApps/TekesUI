#!/usr/bin/env python3
"""Guard Rust implementation sources and private-reference build boundaries.

Shell/Python tooling and WGSL are intentional. This static guard catches explicit
foreign sources, Cargo declarations and compiler invocations; it is not arbitrary
program/data-flow analysis. Run --self-test for isolated negative fixtures.
"""
from __future__ import annotations

import argparse
import ast
import json
from pathlib import Path
import re
import shlex
import subprocess
import sys
import tempfile
import unittest

if sys.version_info < (3, 11):
    raise SystemExit("check-pure-rust.py requires Python 3.11+ (tomllib); choose a newer python3 interpreter.")
import tomllib

FOREIGN_SUFFIXES = {".swift", ".m", ".mm", ".c", ".cc", ".cpp", ".cxx", ".h", ".hh", ".hpp", ".hxx"}
FOREIGN_TOOLS = {"swift", "swiftc", "xcodebuild", "clang", "clang++", "gcc", "g++"}
PRIVATE_PACKAGES = {"tekesmetaltextkit", "tekesclientkit", "tekesappleshim"}
RUST_COMPARE = "tekes-ui-compare"


def normalized(value: str) -> str:
    return re.sub(r"[-_]", "", value).lower()


def managed_files(root: Path) -> list[Path]:
    output = subprocess.check_output(
        ["git", "-C", str(root), "ls-files", "--cached", "--others", "--exclude-standard", "-z"]
    )
    files = set()
    for item in output.split(b"\0"):
        if not item:
            continue
        name = Path(item.decode("utf-8", errors="surrogateescape"))
        if any(part in {"target", ".git"} for part in name.parts) or "worktrees" in name.parts:
            continue
        path = root / name
        if path.is_file():
            files.add(path)
    return sorted(files)


def foreign_tool(program: str) -> bool:
    return Path(program).name in FOREIGN_TOOLS


def python_commands(source: str) -> list[str]:
    commands = []
    tree = ast.parse(source)
    for node in ast.walk(tree):
        if not isinstance(node, ast.Call) or not node.args:
            continue
        function = node.func
        if not isinstance(function, ast.Attribute) or function.attr not in {"run", "Popen", "call", "check_call", "check_output", "system", "popen"}:
            continue
        arg = node.args[0]
        if isinstance(arg, (ast.List, ast.Tuple)) and arg.elts and isinstance(arg.elts[0], ast.Constant) and isinstance(arg.elts[0].value, str):
            commands.append(arg.elts[0].value)
            if arg.elts[0].value == "xcrun" and len(arg.elts) > 1 and isinstance(arg.elts[1], ast.Constant):
                commands.append(str(arg.elts[1].value))
        elif isinstance(arg, ast.Constant) and isinstance(arg.value, str):
            commands.extend(shell_commands(arg.value))
    return commands


def shell_commands(source: str) -> list[str]:
    commands = []
    for line in source.splitlines():
        # Also handles GitHub Actions run: lines and block-shell bodies.
        line = re.sub(r"^\s*(?:-\s*)?run:\s*", "", line)
        lexer = shlex.shlex(line, posix=True, punctuation_chars=";&|")
        lexer.whitespace_split = True
        try:
            words = list(lexer)
        except ValueError:
            continue  # Multiline shell strings are outside this direct-call guard.
        start = True
        via_xcrun = False
        for word in words:
            if re.fullmatch(r"[;&|]+", word):
                start, via_xcrun = True, False
            elif start:
                if re.match(r"[A-Za-z_][A-Za-z_0-9]*=", word) or word in {"env", "exec", "command", "sudo", "then", "do"}:
                    continue
                commands.append(word)
                via_xcrun = Path(word).name == "xcrun"
                start = False
            elif via_xcrun and not word.startswith("-"):
                commands.append(word)
                via_xcrun = False
    return commands


def rust_commands(source: str) -> list[str]:
    # Tokenize comments and string literals separately, so docs and fixture strings
    # containing Command::new(...) cannot impersonate an actual invocation.
    pattern = re.compile(r'//[^\n]*|/\*.*?\*/|r\#*".*?"\#*|"(?:\\.|[^"\\])*"|[A-Za-z_][A-Za-z_0-9]*|::|[^\s]', re.S)
    tokens = [m.group() for m in pattern.finditer(source) if not m.group().startswith(("//", "/*"))]
    commands = []
    for i in range(len(tokens) - 4):
        if tokens[i:i + 4] != ["Command", "::", "new", "("]:
            continue
        literal = tokens[i + 4]
        if literal.startswith('"'):
            try:
                commands.append(json.loads(literal))
            except json.JSONDecodeError:
                pass
        elif literal.startswith("r") and '"' in literal:
            commands.append(literal[literal.index('"') + 1:literal.rindex('"')])
    return commands


def dependency_tables(table: dict):
    for key, value in table.items():
        if key in {"dependencies", "dev-dependencies", "build-dependencies"} and isinstance(value, dict):
            yield value
        elif isinstance(value, dict):
            yield from dependency_tables(value)


def violations(root: Path, files: list[Path]) -> list[str]:
    original_root = root.absolute()
    root = root.resolve()
    errors = []
    local_graph = {}
    def workspace_for(manifest_path):
        # Review evidence contains complete independent Cargo workspaces. Never
        # interpret their workspace=true dependencies against the outer repo.
        for directory in manifest_path.parent.parents:
            if directory != root and root not in directory.parents:
                break
            candidate = directory / "Cargo.toml"
            if candidate.is_file():
                try:
                    document = tomllib.loads(candidate.read_text())
                except (OSError, ValueError):
                    continue  # The managed manifest pass reports syntax errors.
                if "workspace" in document:
                    return directory, document["workspace"].get("dependencies", {})
        return root, {}

    for path in files:
        path = root / path.relative_to(original_root)
        name = path.relative_to(root).as_posix()
        suffix = path.suffix.lower()
        if suffix in FOREIGN_SUFFIXES or path.name in {"Package.swift", "Package.resolved"} or any(part.endswith((".xcodeproj", ".xcworkspace")) for part in path.parts):
            errors.append(f"{name}: foreign source or SwiftPM/Xcode input")
        if path.is_symlink():
            errors.append(f"{name}: managed source symlink requires explicit review")
            continue
        if path.name == "Cargo.toml":
            try:
                manifest = tomllib.loads(path.read_text())
            except (OSError, ValueError) as error:
                errors.append(f"{name}: invalid Cargo manifest: {error}")
                continue
            package = manifest.get("package", {}).get("name")
            if package:
                local_graph[package] = set()
            build = manifest.get("package", {}).get("build")
            if isinstance(build, str) and Path(build).suffix.lower() in FOREIGN_SUFFIXES:
                errors.append(f"{name}: foreign build entry {build}")
            if "workspace" in manifest:
                workspace_root = path.parent
                workspace_dependencies = manifest["workspace"].get("dependencies", {})
            else:
                workspace_root, workspace_dependencies = workspace_for(path)
            for table in dependency_tables(manifest):
                for alias, declaration in table.items():
                    dep = declaration if isinstance(declaration, dict) else {}
                    dependency_base = path.parent
                    if dep.get("workspace"):
                        inherited = workspace_dependencies.get(alias)
                        if inherited is None:
                            errors.append(f"{name}: unresolved inherited dependency {alias}")
                        dep = inherited if isinstance(inherited, dict) else {}
                        dependency_base = workspace_root
                    dependency = dep.get("package", alias)
                    if package:
                        local_graph[package].add(dependency)
                    if normalized(dependency) in PRIVATE_PACKAGES:
                        errors.append(f"{name}: private reference dependency {dependency}")
                    for field in ("git", "path"):
                        value = dep.get(field, "")
                        if any(private in normalized(value) for private in PRIVATE_PACKAGES) or (field == "git" and normalized("TekesUI-compare") in normalized(value)):
                            errors.append(f"{name}: private reference {field} {value}")
                    if "path" in dep:
                        resolved = (dependency_base / dep["path"]).resolve()
                        if not resolved.is_relative_to(root):
                            errors.append(f"{name}: dependency path escapes public repository: {dep['path']}")
                        elif (resolved / "Cargo.toml").is_file():
                            target = tomllib.loads((resolved / "Cargo.toml").read_text()).get("package", {}).get("name")
                            if package and target:
                                local_graph[package].add(target)
        if suffix not in {".rs", ".py", ".sh", ".yml", ".yaml"}:
            continue
        try:
            source = path.read_text()
            commands = python_commands(source) if suffix == ".py" else rust_commands(source) if suffix == ".rs" else shell_commands(source)
            for program in commands:
                if foreign_tool(program):
                    errors.append(f"{name}: direct foreign compiler invocation {program}")
        except (OSError, SyntaxError, UnicodeError) as error:
            errors.append(f"{name}: cannot inspect source: {error}")
    pending = ["tekes-ui-gallery"]
    seen = set()
    while pending:
        package = pending.pop()
        if package in seen:
            continue
        seen.add(package)
        pending.extend(local_graph.get(package, ()))
    if RUST_COMPARE in seen:
        errors.append("tekes-ui-gallery: dependency graph reaches independent Rust compare tool")
    return sorted(set(errors))


class GuardTests(unittest.TestCase):
    def check(self, sources):
        with tempfile.TemporaryDirectory(prefix="tekes-pure-rust-") as temp:
            root = Path(temp)
            files = []
            for name, source in sources.items():
                path = root / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(source)
                files.append(path)
            return violations(root, files)

    def test_rust_system_bindings_and_tools_are_allowed(self):
        self.assertEqual(self.check({"src/main.rs": 'use objc2_core_text::CTFont; // reference Swift package only\n', "src/glyph.wgsl": "fn main() {}", "scripts/check.py": 'import subprocess\nsubprocess.run(["git", "status"])\n', "scripts/check.sh": "#!/bin/bash\ncargo check\n", "Cargo.toml": '[package]\nname="tekes-ui-gallery"\n[dependencies]\nobjc2="0.6"\n'}), [])

    def test_foreign_sources_and_projects_fail(self):
        for path in ["src/bad.swift", "src/bad.m", "src/bad.mm", "src/bad.c", "src/bad.cpp", "Package.swift", "Package.resolved", "App.xcodeproj/project.pbxproj"]:
            with self.subTest(path=path):
                self.assertTrue(self.check({path: ""}))

    def test_direct_compiler_calls_fail(self):
        cases = {"src/bad.rs": 'fn f(){std::process::Command::new("swift").arg("build");}', "scripts/bad.py": 'import subprocess\nsubprocess.run(["swiftc", "bad.swift"])', "scripts/bad.sh": "xcrun swift build", ".github/workflows/bad.yml": "- run: xcodebuild -project Bad"}
        for path, source in cases.items():
            with self.subTest(path=path):
                self.assertTrue(self.check({path: source}))

    def test_comments_and_fixtures_are_not_invocations(self):
        self.assertEqual(self.check({"src/good.rs": '// Command::new("swift")\nfn f(){let s=r#"Command::new("swift")"#;}', "scripts/good.py": 'fixture = \'subprocess.run(["swift", "build"])\'\n', "scripts/good.sh": "# swift build\necho 'swift build'\n"}), [])

    def test_private_dependency_and_aliased_escape_fail(self):
        for dependency in ['private={package="TekesMetalTextKit",version="1"}', 'private={path="../TekesUI-compare"}', 'private={git="https://example.invalid/TekesClientKit"}']:
            with self.subTest(dependency=dependency):
                self.assertTrue(self.check({"Cargo.toml": '[package]\nname="tekes-ui-gallery"\n[dependencies]\n' + dependency}))

    def test_independent_rust_compare_is_allowed_but_gallery_edge_fails(self):
        sources = {"Cargo.toml": '[workspace]\nmembers=["crates/*"]\n[workspace.dependencies]\ntekes-ui-compare={path="crates/tekes-ui-compare"}', "crates/tekes-ui-gallery/Cargo.toml": '[package]\nname="tekes-ui-gallery"', "crates/tekes-ui-compare/Cargo.toml": '[package]\nname="tekes-ui-compare"\n[dependencies]\ntekes-ui-text="1"'}
        self.assertEqual(self.check(sources), [])
        sources["crates/tekes-ui-gallery/Cargo.toml"] += '\n[dependencies]\nprobe={package="tekes-ui-compare",path="../tekes-ui-compare"}'
        self.assertTrue(self.check(sources))

    def test_workspace_alias_cannot_hide_gallery_compare_edge(self):
        self.assertTrue(self.check({"Cargo.toml": '[workspace]\n[workspace.dependencies]\nprobe={package="tekes-ui-compare",path="crates/compare"}', "crates/gallery/Cargo.toml": '[package]\nname="tekes-ui-gallery"\n[dependencies]\nprobe.workspace=true', "crates/compare/Cargo.toml": '[package]\nname="tekes-ui-compare"'}))

    def test_nested_review_workspace_resolves_its_own_inheritance(self):
        self.assertEqual(self.check({
            "Cargo.toml": '[workspace]\n[workspace.dependencies]\nouter="1"',
            "reviews/consumer/Cargo.toml": '[workspace]\n[workspace.dependencies]\nshared={package="public-text",path="crates/text"}',
            "reviews/consumer/crates/demo/Cargo.toml": '[package]\nname="demo"\n[dependencies]\nshared.workspace=true',
        }), [])

    def test_nested_review_workspace_still_rejects_private_and_escape(self):
        for declaration in ['{package="TekesClientKit",version="1"}', '{path="../../../escape"}']:
            with self.subTest(declaration=declaration):
                self.assertTrue(self.check({
                    "Cargo.toml": '[workspace]',
                    "reviews/consumer/Cargo.toml": '[workspace]\n[workspace.dependencies]\nshared='+declaration,
                    "reviews/consumer/crates/demo/Cargo.toml": '[package]\nname="demo"\n[dependencies]\nshared.workspace=true',
                }))

    def test_indirect_gallery_compare_edge_fails(self):
        self.assertTrue(self.check({"Cargo.toml": '[package]\nname="tekes-ui-gallery"\n[dependencies]\ntekes-ui-text="1"', "crates/text/Cargo.toml": '[package]\nname="tekes-ui-text"\n[dependencies]\ntekes-ui-compare="1"'}))


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        result = unittest.TextTestRunner(verbosity=2).run(unittest.defaultTestLoader.loadTestsFromTestCase(GuardTests))
        return 0 if result.wasSuccessful() else 1
    root = Path(__file__).resolve().parents[1]
    files = managed_files(root)
    errors = violations(root, files)
    if errors:
        print("Pure Rust source boundary FAILED", file=sys.stderr)
        for error in errors:
            print(f"- {error}", file=sys.stderr)
        return 1
    print(f"Pure Rust source boundary passed: {len(files)} managed/untracked files; Rust + WGSL implementation, non-Swift auxiliary scripts allowed; no private reference build dependency.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
