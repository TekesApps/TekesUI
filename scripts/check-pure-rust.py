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

try:
    import tomllib
except ModuleNotFoundError:
    tomllib = None


def parse_toml(text: str) -> dict:
    """Minimal self-contained TOML parser supporting Cargo.toml syntax on Python 3.9+."""
    src = text.lstrip("\ufeff").replace("\r\n", "\n")
    n = len(src)

    def skip_hspace_and_comment(pos: int, skip_nl: bool) -> int:
        while pos < n:
            ch = src[pos]
            if ch in (" ", "\t"):
                pos += 1
            elif ch == "#":
                while pos < n and src[pos] != "\n":
                    pos += 1
            elif skip_nl and ch == "\n":
                pos += 1
            else:
                break
        return pos

    def parse_basic_string(pos: int, multiline: bool) -> tuple[str, int]:
        out = []
        if multiline and pos < n and src[pos] == "\n":
            pos += 1
        escapes = {'"': '"', "\\": "\\", "n": "\n", "r": "\r", "t": "\t", "b": "\b", "f": "\f"}
        while pos < n:
            if multiline:
                if src.startswith('"""', pos):
                    end = pos + 3
                    while end < n and src[end] == '"' and (end - (pos + 3)) < 2:
                        out.append('"')
                        end += 1
                    return "".join(out), end
            elif src[pos] == '"':
                return "".join(out), pos + 1
            elif src[pos] == "\n":
                raise ValueError("unterminated basic string in TOML")
            ch = src[pos]
            if ch == "\\":
                pos += 1
                if pos >= n:
                    raise ValueError("unterminated escape in TOML string")
                esc = src[pos]
                if esc in escapes:
                    out.append(escapes[esc])
                    pos += 1
                elif multiline and esc in (" ", "\t", "\n"):
                    while pos < n and src[pos] in (" ", "\t", "\n"):
                        pos += 1
                elif esc == "u" and pos + 5 <= n:
                    out.append(chr(int(src[pos + 1:pos + 5], 16)))
                    pos += 5
                elif esc == "U" and pos + 9 <= n:
                    out.append(chr(int(src[pos + 1:pos + 9], 16)))
                    pos += 9
                else:
                    raise ValueError(f"unsupported escape \\{esc} in TOML")
            else:
                out.append(ch)
                pos += 1
        raise ValueError("unterminated string in TOML")

    def parse_literal_string(pos: int, multiline: bool) -> tuple[str, int]:
        if multiline and pos < n and src[pos] == "\n":
            pos += 1
        start = pos
        if multiline:
            end = src.find("'''", pos)
            if end < 0:
                raise ValueError("unterminated multiline literal string in TOML")
            val = src[start:end]
            end += 3
            while end < n and src[end] == "'" and len(val) - len(val.rstrip("'")) >= 0 and (src[end - 1] == "'"):
                if end < n and src[end] == "'":
                    val += "'"
                    end += 1
                else:
                    break
            return val, end
        while pos < n:
            if src[pos] == "'":
                return src[start:pos], pos + 1
            if src[pos] == "\n":
                raise ValueError("unterminated literal string in TOML")
            pos += 1
        raise ValueError("unterminated literal string in TOML")

    def parse_key_part(pos: int) -> tuple[str, int]:
        pos = skip_hspace_and_comment(pos, False)
        if pos >= n:
            raise ValueError("expected key in TOML")
        if src[pos] == '"':
            return parse_basic_string(pos + 1, False)
        if src[pos] == "'":
            return parse_literal_string(pos + 1, False)
        start = pos
        while pos < n and (src[pos].isalnum() or src[pos] in ("_", "-")):
            pos += 1
        if pos == start:
            raise ValueError(f"unexpected key character {src[pos]!r} in TOML")
        return src[start:pos], pos

    def parse_dotted_key(pos: int) -> tuple[list[str], int]:
        parts = []
        while True:
            part, pos = parse_key_part(pos)
            parts.append(part)
            pos = skip_hspace_and_comment(pos, False)
            if pos < n and src[pos] == ".":
                pos += 1
                continue
            return parts, pos

    def parse_value(pos: int) -> tuple[object, int]:
        pos = skip_hspace_and_comment(pos, False)
        if pos >= n:
            raise ValueError("missing value in TOML")
        if src.startswith('"""', pos):
            return parse_basic_string(pos + 3, True)
        if src.startswith("'''", pos):
            return parse_literal_string(pos + 3, True)
        ch = src[pos]
        if ch == '"':
            return parse_basic_string(pos + 1, False)
        if ch == "'":
            return parse_literal_string(pos + 1, False)
        if ch == "[":
            pos += 1
            items = []
            while True:
                pos = skip_hspace_and_comment(pos, True)
                if pos >= n:
                    raise ValueError("unterminated array in TOML")
                if src[pos] == "]":
                    return items, pos + 1
                val, pos = parse_value(pos)
                items.append(val)
                pos = skip_hspace_and_comment(pos, True)
                if pos < n and src[pos] == ",":
                    pos += 1
                    continue
                if pos < n and src[pos] == "]":
                    return items, pos + 1
                raise ValueError("expected ',' or ']' in TOML array")
        if ch == "{":
            pos += 1
            tbl = {}
            while True:
                pos = skip_hspace_and_comment(pos, True)
                if pos >= n:
                    raise ValueError("unterminated inline table in TOML")
                if src[pos] == "}":
                    return tbl, pos + 1
                key_parts, pos = parse_dotted_key(pos)
                pos = skip_hspace_and_comment(pos, False)
                if pos >= n or src[pos] != "=":
                    raise ValueError("expected '=' in inline table")
                val, pos = parse_value(pos + 1)
                set_path(tbl, key_parts, val)
                pos = skip_hspace_and_comment(pos, True)
                if pos < n and src[pos] == ",":
                    pos += 1
                    continue
                if pos < n and src[pos] == "}":
                    return tbl, pos + 1
                raise ValueError("expected ',' or '}' in inline table")
        start = pos
        while pos < n and src[pos] not in ("\n", "#", ",", "]", "}", " ", "\t"):
            pos += 1
        token = src[start:pos]
        if token == "true":
            return True, pos
        if token == "false":
            return False, pos
        if re.fullmatch(r"[+-]?\d(?:_?\d)*", token):
            return int(token.replace("_", "")), pos
        return token, pos

    def set_path(target: dict, parts: list[str], value: object) -> None:
        node = target
        for part in parts[:-1]:
            nxt = node.setdefault(part, {})
            if not isinstance(nxt, dict):
                raise ValueError(f"TOML key conflict at {part!r}")
            node = nxt
        node[parts[-1]] = value

    def ensure_table(root_dict: dict, parts: list[str], array_item: bool) -> dict:
        node = root_dict
        for idx, part in enumerate(parts):
            is_last = idx == len(parts) - 1
            if is_last and array_item:
                arr = node.setdefault(part, [])
                if not isinstance(arr, list):
                    raise ValueError(f"TOML array-of-tables conflict at {part!r}")
                entry = {}
                arr.append(entry)
                return entry
            existing = node.get(part)
            if existing is None:
                existing = {}
                node[part] = existing
            elif isinstance(existing, list):
                existing = existing[-1]
            if not isinstance(existing, dict):
                raise ValueError(f"TOML table conflict at {part!r}")
            node = existing
        return node

    root_doc: dict = {}
    current = root_doc
    pos = 0
    while True:
        pos = skip_hspace_and_comment(pos, True)
        if pos >= n:
            return root_doc
        if src.startswith("[[", pos):
            parts, pos = parse_dotted_key(pos + 2)
            pos = skip_hspace_and_comment(pos, False)
            if not src.startswith("]]", pos):
                raise ValueError("expected ']]' closing array table header")
            pos = skip_hspace_and_comment(pos + 2, False)
            if pos < n and src[pos] != "\n":
                raise ValueError("unexpected trailing tokens after table header")
            current = ensure_table(root_doc, parts, True)
        elif src[pos] == "[":
            parts, pos = parse_dotted_key(pos + 1)
            pos = skip_hspace_and_comment(pos, False)
            if pos >= n or src[pos] != "]":
                raise ValueError("expected ']' closing table header")
            pos = skip_hspace_and_comment(pos + 1, False)
            if pos < n and src[pos] != "\n":
                raise ValueError("unexpected trailing tokens after table header")
            current = ensure_table(root_doc, parts, False)
        else:
            parts, pos = parse_dotted_key(pos)
            pos = skip_hspace_and_comment(pos, False)
            if pos >= n or src[pos] != "=":
                raise ValueError("expected '=' after key in TOML")
            val, pos = parse_value(pos + 1)
            pos = skip_hspace_and_comment(pos, False)
            if pos < n and src[pos] != "\n":
                raise ValueError("unexpected trailing tokens after value in TOML")
            set_path(current, parts, val)


def load_toml(text: str) -> dict:
    return tomllib.loads(text) if tomllib is not None else parse_toml(text)

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
                    document = load_toml(candidate.read_text())
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
                manifest = load_toml(path.read_text())
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
                            target = load_toml((resolved / "Cargo.toml").read_text()).get("package", {}).get("name")
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

    def test_fallback_toml_parser_handles_workspace_and_inline_tables(self):
        sample = (
            '[workspace]\n'
            'members = [\n'
            '  "crates/*",\n'
            '  "examples/brand-consumer", # trailing comment\n'
            ']\n'
            '[workspace.package]\n'
            'version = "0.3.34"\n'
            '[workspace.dependencies]\n'
            'probe = { package = "tekes-ui-compare", path = "crates/compare", default-features = false }\n'
            '[target.\'cfg(target_os = "macos")\'.dependencies.objc2]\n'
            'version = "=0.6.5"\n'
            'workspace = true\n'
        )
        parsed = parse_toml(sample)
        self.assertEqual(parsed["workspace"]["package"]["version"], "0.3.34")
        self.assertEqual(parsed["workspace"]["members"], ["crates/*", "examples/brand-consumer"])
        self.assertEqual(parsed["workspace"]["dependencies"]["probe"]["default-features"], False)
        self.assertTrue(parsed["target"]['cfg(target_os = "macos")']["dependencies"]["objc2"]["workspace"])
        real_manifest = (Path(__file__).resolve().parents[1] / "Cargo.toml").read_text()
        parsed_real = parse_toml(real_manifest)
        self.assertIn("workspace", parsed_real)
        if tomllib is not None:
            self.assertEqual(parsed_real, tomllib.loads(real_manifest))


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
