#!/usr/bin/env python3
"""Read-only Gallery DSv1 guard. Never generates or updates golden files.

Default: token source, visual source metadata, and checked-in PNG integrity.
Visual source changes require --check-snapshots, with byte-identical PNGs in the
recorded environment. Actual candidate build provenance remains a reviewer duty.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import math
from pathlib import Path, PurePosixPath
import platform
import re
import struct
import subprocess
import sys
import tempfile
import unittest
from unittest import mock
import zlib

MANIFEST = "design/baseline/manifest.json"
PAGES = {"primitives", "tokens", "static-text"}
THEMES = {"light", "dark", "light-ic", "dark-ic"}
SCALES = {1, 2}
MIGRATION = "design/source-migration.json"
# This approval pin is populated after the reviewed source-only migration receipt.
# It is deliberately outside the receipt; changing the receipt is not self-approval.
MIGRATION_APPROVAL_SHA256 = "84bba0e3890c6c6c2eff821dfca0e14b3135f521352f0322eb5a96e50d9182b0"
APPROVED_NUMERIC_SHA256 = "f08777d278d713b2a2549facf8b40804da900251ff886bda3204a6497dbe8c76"


def semantic_digest(value) -> str:
    return hashlib.sha256(json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False, allow_nan=False).encode()).hexdigest()


def numeric_contract(document: dict) -> tuple[int, str]:
    channels = []
    def collect(value, pointer):
        if isinstance(value, dict):
            for key, child in value.items():
                collect(child, pointer + "/" + key.replace("~", "~0").replace("/", "~1"))
        elif isinstance(value, list):
            for index, child in enumerate(value):
                collect(child, pointer + "/" + str(index))
        elif isinstance(value, (int, float)) and not isinstance(value, bool):
            if not math.isfinite(value):
                raise ValueError("non-finite token channel")
            channels.append({"pointer": pointer, "f32_be_bits": struct.pack(">f", float(value)).hex()})
    def visit(value, pointer=""):
        if isinstance(value, dict):
            for key, child in value.items():
                if key == "$value":
                    collect(child, pointer + "/$value")
                else:
                    visit(child, pointer + "/" + key)
        elif isinstance(value, list):
            for index, child in enumerate(value):
                visit(child, pointer + "/" + str(index))
    visit(document)
    channels.sort(key=lambda row: row["pointer"])
    return len(channels), semantic_digest(channels)


def source_migration(root: Path, manifest: dict, actual_tokens: dict[str, str], generated_rust: Path | None = None, resolved_contract: Path | None = None) -> dict[str, str] | None:
    receipt_path = safe_path(root, MIGRATION)
    if not receipt_path.exists():
        if generated_rust is not None or resolved_contract is not None:
            raise ValueError("generated/parsed token review requires the approved migration receipt")
        return None
    if digest(receipt_path) != MIGRATION_APPROVAL_SHA256:
        raise ValueError("migration receipt changed or is not approved")
    receipt = json.loads(receipt_path.read_text())
    if receipt["schema"] != "tekesui-token-source-migration/1" or receipt["status"] != "approved-source-only-migration":
        raise ValueError("invalid migration approval status")
    if receipt["authorization"]["baseline_commit"] != manifest["approval"]["source_commit"] or not receipt["authorization"]["user_instruction"]:
        raise ValueError("migration approval does not refer to the original baseline")
    if receipt["original_baseline"]["manifest_sha256"] != digest(safe_path(root, MANIFEST)):
        raise ValueError("migration receipt original manifest binding changed")
    source = receipt["json_source"]
    if source["path"] != "design/tokens.json":
        raise ValueError("migration source must be design/tokens.json")
    json_path = safe_path(root, source["path"])
    document = json.loads(json_path.read_text())
    if source["file_sha256"] != digest(json_path) or source["semantic_sha256"] != semantic_digest(document):
        raise ValueError("JSON authority bytes or semantics changed")
    approved_contract = {"count": 508, "f32_sha256": APPROVED_NUMERIC_SHA256}
    if receipt["approved_numeric_contract"] != approved_contract:
        raise ValueError("migration does not preserve the original 508-channel f32 contract")
    # Production JSON may contain aliases/derived expressions. Resolve these with
    # the Rust generator, never a second implementation in this Python guard.
    if resolved_contract is not None:
        if resolved_contract.is_symlink() or not resolved_contract.is_file():
            raise ValueError("resolved token contract is missing or a symlink")
        count, numeric_sha = numeric_contract(json.loads(resolved_contract.read_text()))
        if {"count": count, "f32_sha256": numeric_sha} != approved_contract:
            raise ValueError("resolved JSON differs from the original 508-channel f32 contract")
    if receipt["token_bindings"] != actual_tokens:
        raise ValueError("token adapter/build source inventory changed")
    generator = receipt["generator"]
    if generator["root"] != "crates/tekes-ui-design-export" or generator["files"] != files_under(root, generator["root"]):
        raise ValueError("generator source inventory changed")
    expected_generated = generator["generated_rust_sha256"]
    if not re.fullmatch(r"[0-9a-f]{64}", expected_generated):
        raise ValueError("generated Rust content hash missing")
    if generated_rust is not None and (generated_rust.is_symlink() or not generated_rust.is_file() or digest(generated_rust) != expected_generated):
        raise ValueError("generated Rust is stale, missing, or differs from the approved JSON generation")
    overrides = receipt["visual_source_overrides"]
    if set(overrides) != {"Cargo.lock"} or any(digest(safe_path(root, name)) != sha for name, sha in overrides.items()):
        raise ValueError("migration permits only its exact Cargo.lock source transition")
    return overrides


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def safe_path(root: Path, relative: str) -> Path:
    pure = PurePosixPath(relative)
    if not relative or pure.is_absolute() or ".." in pure.parts or "\\" in relative:
        raise ValueError(f"invalid repository-relative path: {relative!r}")
    path = root.joinpath(*pure.parts)
    if any(part.is_symlink() for part in [path, *path.parents] if part != root.parent):
        raise ValueError(f"symlink is not a baseline input: {relative}")
    return path


def files_under(root: Path, directory: str) -> dict[str, str]:
    path = safe_path(root, directory)
    if not path.is_dir():
        raise ValueError(f"source directory missing: {directory}")
    files = {}
    for item in path.rglob("*"):
        if item.name == ".DS_Store" or "__pycache__" in item.parts:
            continue
        if item.is_symlink():
            raise ValueError(f"source symlink requires review: {item.relative_to(root)}")
        if item.is_file():
            files[item.relative_to(root).as_posix()] = digest(item)
    return files


def changed_files(expected: dict[str, str], actual: dict[str, str]) -> list[str]:
    return sorted(name for name in expected.keys() | actual.keys() if expected.get(name) != actual.get(name))


def png_size(path: Path) -> tuple[int, int]:
    with path.open("rb") as image:
        header = image.read(24)
    if len(header) != 24 or header[:8] != b"\x89PNG\r\n\x1a\n" or header[12:16] != b"IHDR":
        raise ValueError(f"not a PNG with an IHDR header: {path.name}")
    return struct.unpack(">II", header[16:24])


def observe_environment() -> dict:
    if platform.system() != "Darwin":
        return {"platform": platform.system()}
    def run(*arguments):
        return subprocess.check_output(arguments, text=True).strip()
    languages = run("defaults", "read", "-g", "AppleLanguages")
    language_list = re.findall(r'"([^"\n]+)"', languages)
    if not language_list:
        language_list = [line.strip().rstrip(",") for line in languages.splitlines() if line.strip() not in {"(", ")"}]
    return {
        "platform": "Darwin",
        "machine_model": run("sysctl", "-n", "hw.model"),
        "os_version": run("sw_vers", "-productVersion"),
        "os_build": run("sw_vers", "-buildVersion"),
        "apple_languages": language_list,
    }


def check(root: Path, manifest: dict, candidate: Path | None = None, environment: dict | None = None, generated_rust: Path | None = None, resolved_contract: Path | None = None) -> tuple[list[str], list[str]]:
    failures, pending = [], []
    try:
        if manifest["schema"] != "tekesui-gallery-design-baseline/1":
            raise ValueError("unrecognized baseline manifest schema")
        if not re.fullmatch(r"[0-9a-f]{40}", manifest["approval"]["source_commit"]):
            raise ValueError("approved source commit must be a full SHA-1")
        if not re.fullmatch(r"[0-9a-f]{64}", manifest["approval"]["release_sha256"]):
            raise ValueError("approved original release SHA-256 missing")
        if not manifest["approval"]["user_instruction"]:
            raise ValueError("explicit design approval instruction missing")
        if manifest["status"] != "approved-visual-starting-point":
            raise ValueError("baseline is a visual starting point, not a complete component system")
        tokens = manifest["token_sources"]
        if tokens["root"] != "crates/tekes-ui-tokens" or not tokens["files"]:
            raise ValueError("token crate inventory missing")
        for inventory in (tokens["files"], manifest["visual_sources"]["files"]):
            if not isinstance(inventory, dict):
                raise ValueError("source hash inventory must be a map")
            for filename, sha256 in inventory.items():
                safe_path(root, filename)
                if not isinstance(sha256, str) or not re.fullmatch(r"[0-9a-f]{64}", sha256):
                    raise ValueError("invalid source SHA-256: " + filename)
        actual_tokens = files_under(root, tokens["root"])
        approved_overrides = source_migration(root, manifest, actual_tokens, generated_rust, resolved_contract)
        token_changes = changed_files(tokens["files"], actual_tokens)
        if token_changes and approved_overrides is None:
            failures.append("TOKEN BASELINE CHANGED; explicit design baseline approval/update required: " + ", ".join(token_changes))
        sources = manifest["visual_sources"]
        if not sources["roots"] or "crates/tekes-ui-gallery/src" not in sources["roots"] or not sources["files"]:
            raise ValueError("Gallery visual source provenance missing")
        actual = {}
        for directory in sources["roots"]:
            actual.update(files_under(root, directory))
        for filename in sources["extra_files"]:
            path = safe_path(root, filename)
            if path.is_file():
                actual[filename] = digest(path)
        source_changes = changed_files(sources["files"], actual)
        if approved_overrides is not None:
            source_changes = [name for name in source_changes if name not in approved_overrides]
        if source_changes and candidate is None:
            pending.append("VISUAL SOURCE CHANGED; requires --check-snapshots from the candidate build: " + ", ".join(source_changes))
        snapshots = manifest["snapshots"]
        if set(snapshots["pages"]) != PAGES or set(snapshots["themes"]) != THEMES or set(snapshots["scales"]) != SCALES:
            raise ValueError("baseline must retain 3 original pages x 4 themes x 2 scales")
        expected_names = {f"{page}-{theme}@{scale}x.png" for page in PAGES for theme in THEMES for scale in SCALES}
        if snapshots["canvas_pt"] != [1200, 800]:
            raise ValueError("original 1200x800 point calibration canvas changed")
        expected_environment = snapshots["environment"]["match"]
        if set(expected_environment) != {"platform", "machine_model", "os_version", "os_build", "apple_languages"} or not all(expected_environment.values()):
            raise ValueError("original machine/OS/build/language environment metadata incomplete")
        entries = snapshots["files"]
        names = [item["file"] for item in entries]
        if len(names) != 24 or set(names) != expected_names:
            raise ValueError("snapshot entries missing, duplicate, or outside the original 24-image grid")
        golden = safe_path(root, snapshots["directory"])
        if set(path.name for path in golden.glob("*.png")) != expected_names:
            failures.append("checked-in golden PNG inventory differs from the 24-image manifest")
        for item in entries:
            path = safe_path(root, snapshots["directory"] + "/" + item["file"])
            if not path.is_file():
                failures.append("golden missing: " + item["file"])
                continue
            scale = int(item["file"].split("@")[-1][:-5])
            size = (1200 * scale, 800 * scale)
            if item["size_px"] != list(size) or png_size(path) != size or digest(path) != item["sha256"]:
                failures.append("golden bytes/header changed: " + item["file"])
            if candidate is not None:
                other = candidate / item["file"]
                if other.is_symlink() or not other.is_file():
                    failures.append("candidate missing or symlink: " + item["file"])
                elif other.read_bytes() != path.read_bytes():
                    failures.append("candidate differs byte-for-byte: " + item["file"])
        if candidate is not None:
            if set(path.name for path in candidate.glob("*.png")) != expected_names:
                failures.append("candidate PNG inventory differs from the original 24 images")
            current = environment if environment is not None else observe_environment()
            expected_environment = snapshots["environment"]["match"]
            mismatches = [key for key, value in expected_environment.items() if current.get(key) != value]
            if mismatches:
                pending.append("ENVIRONMENT UNVERIFIED; byte comparison is not same-environment acceptance: " + ", ".join(mismatches))
    except (KeyError, TypeError, ValueError, OverflowError, struct.error, OSError, subprocess.SubprocessError) as error:
        failures.append("cannot validate design baseline: " + str(error))
    return failures, pending


def test_png(scale: int) -> bytes:
    width, height = 1200 * scale, 800 * scale
    def chunk(kind, data):
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))
    raw = (b"\0" + b"\0" * width * 4) * height
    return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0)) + chunk(b"IDAT", zlib.compress(raw)) + chunk(b"IEND", b"")


class BaselineTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="tekes-design-baseline-")
        self.root = Path(self.temp.name).resolve()
        self.addCleanup(self.temp.cleanup)
        for name in ["crates/tekes-ui-tokens/src/lib.rs", "crates/tekes-ui-gallery/src/main.rs"]:
            path = self.root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text("// original Rust source\n")
        self.environment = {"platform": "Darwin", "machine_model": "test", "os_version": "test", "os_build": "test", "apple_languages": ["en-JP", "ja-JP"]}
        images = self.root / "design/baseline/screenshots"
        images.mkdir(parents=True)
        self.candidate = self.root / "candidate"
        self.candidate.mkdir()
        encoded = {scale: test_png(scale) for scale in SCALES}
        entries = []
        for page in sorted(PAGES):
            for theme in sorted(THEMES):
                for scale in sorted(SCALES):
                    name = f"{page}-{theme}@{scale}x.png"
                    (images / name).write_bytes(encoded[scale])
                    (self.candidate / name).write_bytes(encoded[scale])
                    entries.append({"file": name, "size_px": [1200 * scale, 800 * scale], "sha256": digest(images / name)})
        self.manifest = {
            "schema": "tekesui-gallery-design-baseline/1", "status": "approved-visual-starting-point",
            "approval": {"source_commit": "a" * 40, "release_sha256": "b" * 64, "user_instruction": "Keep this Gallery design"},
            "token_sources": {"root": "crates/tekes-ui-tokens", "files": files_under(self.root, "crates/tekes-ui-tokens")},
            "visual_sources": {"roots": ["crates/tekes-ui-gallery/src"], "extra_files": [], "files": files_under(self.root, "crates/tekes-ui-gallery/src")},
            "snapshots": {"directory": "design/baseline/screenshots", "pages": sorted(PAGES), "themes": sorted(THEMES), "scales": sorted(SCALES), "canvas_pt": [1200, 800], "files": entries, "environment": {"match": self.environment}},
        }

    def test_original_metadata_and_candidate_pass_without_writing(self):
        before = (self.candidate / self.manifest["snapshots"]["files"][0]["file"]).read_bytes()
        self.assertEqual(check(self.root, self.manifest), ([], []))
        self.assertEqual(check(self.root, self.manifest, self.candidate, self.environment), ([], []))
        self.assertEqual((self.candidate / self.manifest["snapshots"]["files"][0]["file"]).read_bytes(), before)

    def test_changed_or_added_token_source_always_fails(self):
        (self.root / "crates/tekes-ui-tokens/src/lib.rs").write_text("// changed token\n")
        self.assertTrue(check(self.root, self.manifest, self.candidate, self.environment)[0])
        (self.root / "crates/tekes-ui-tokens/src/new.rs").write_text("// extra token\n")
        self.assertTrue(check(self.root, self.manifest)[0])

    def test_visual_source_change_requires_candidate_but_same_pixels_can_pass(self):
        (self.root / "crates/tekes-ui-gallery/src/main.rs").write_text("// non-visual behavior change\n")
        failures, pending = check(self.root, self.manifest)
        self.assertFalse(failures)
        self.assertTrue(pending)
        self.assertEqual(check(self.root, self.manifest, self.candidate, self.environment), ([], []))

    def test_different_missing_or_extra_candidate_fails(self):
        name = self.manifest["snapshots"]["files"][0]["file"]
        (self.candidate / name).write_bytes(b"different")
        self.assertTrue(check(self.root, self.manifest, self.candidate, self.environment)[0])
        (self.candidate / name).unlink()
        self.assertTrue(check(self.root, self.manifest, self.candidate, self.environment)[0])
        (self.candidate / "unexpected.png").write_bytes(b"extra")
        self.assertTrue(check(self.root, self.manifest, self.candidate, self.environment)[0])

    def test_changed_golden_or_duplicate_entry_fails(self):
        name = self.manifest["snapshots"]["files"][0]["file"]
        (self.root / "design/baseline/screenshots" / name).write_bytes(b"changed golden")
        self.assertTrue(check(self.root, self.manifest)[0])
        self.manifest["snapshots"]["files"][-1] = self.manifest["snapshots"]["files"][0]
        self.assertTrue(check(self.root, self.manifest)[0])

    def approve_test_migration(self):
        manifest_path = self.root / MANIFEST
        manifest_path.write_text(json.dumps(self.manifest))
        token = self.root / "design/tokens.json"
        token.write_text(json.dumps({"tokens": {"fixture": {"$value": [0.0] * 508}}}))
        (self.root / "crates/tekes-ui-tokens/src/lib.rs").write_text("// approved generated adapter\n")
        generator = self.root / "crates/tekes-ui-design-export"
        generator.mkdir(parents=True)
        (generator / "lib.rs").write_text("// generator source\n")
        lock = self.root / "Cargo.lock"
        lock.write_text("// exact source-only lock transition\n")
        generated = self.root / "generated.rs"
        generated.write_text("// deterministic generated content\n")
        count, numeric_sha = numeric_contract(json.loads(token.read_text()))
        receipt = {
            "schema": "tekesui-token-source-migration/1", "status": "approved-source-only-migration",
            "authorization": {"baseline_commit": self.manifest["approval"]["source_commit"], "user_instruction": "JSON authority, preserve original appearance"},
            "original_baseline": {"manifest_sha256": digest(manifest_path)},
            "json_source": {"path": "design/tokens.json", "file_sha256": digest(token), "semantic_sha256": semantic_digest(json.loads(token.read_text()))},
            "approved_numeric_contract": {"count": count, "f32_sha256": numeric_sha},
            "token_bindings": files_under(self.root, "crates/tekes-ui-tokens"),
            "generator": {"root": "crates/tekes-ui-design-export", "files": files_under(self.root, "crates/tekes-ui-design-export"), "generated_rust_sha256": digest(generated)},
            "visual_source_overrides": {"Cargo.lock": digest(lock)},
        }
        path = self.root / MIGRATION
        path.write_text(json.dumps(receipt))
        self.enterContext(mock.patch(__name__ + ".MIGRATION_APPROVAL_SHA256", digest(path)))
        self.enterContext(mock.patch(__name__ + ".APPROVED_NUMERIC_SHA256", numeric_sha))
        return generated

    def test_exact_source_migration_and_fresh_generated_content_pass(self):
        generated = self.approve_test_migration()
        self.assertEqual(check(self.root, self.manifest, generated_rust=generated), ([], []))
        self.assertEqual(check(self.root, self.manifest, self.candidate, self.environment, generated), ([], []))
        generated.write_text("// stale generated content\n")
        self.assertTrue(check(self.root, self.manifest, generated_rust=generated)[0])

    def test_resolved_contract_preserves_original_f32_bits(self):
        generated = self.approve_test_migration()
        resolved = self.root / "resolved.json"
        resolved.write_bytes((self.root / "design/tokens.json").read_bytes())
        self.assertEqual(check(self.root, self.manifest, generated_rust=generated, resolved_contract=resolved), ([], []))
        document = json.loads(resolved.read_text())
        document["tokens"]["fixture"]["$value"][0] = 1.0
        resolved.write_text(json.dumps(document))
        self.assertTrue(check(self.root, self.manifest, resolved_contract=resolved)[0])

    def test_migration_json_numeric_change_is_not_baseline_approval(self):
        self.approve_test_migration()
        path = self.root / "design/tokens.json"
        document = json.loads(path.read_text())
        document["tokens"]["fixture"]["$value"][0] = 1.0
        path.write_text(json.dumps(document))
        self.assertTrue(check(self.root, self.manifest, self.candidate, self.environment)[0])

    def test_migration_receipt_and_generator_drift_are_rejected(self):
        self.approve_test_migration()
        path = self.root / MIGRATION
        original = path.read_bytes()
        path.write_bytes(original + b"\n")
        self.assertTrue(check(self.root, self.manifest)[0])
        path.write_bytes(original)
        (self.root / "crates/tekes-ui-design-export/lib.rs").write_text("// changed generator\n")
        self.assertTrue(check(self.root, self.manifest)[0])

    def test_source_migration_does_not_approve_other_visual_changes(self):
        self.approve_test_migration()
        (self.root / "crates/tekes-ui-gallery/src/main.rs").write_text("// unreviewed UI change\n")
        failures, pending = check(self.root, self.manifest)
        self.assertFalse(failures)
        self.assertTrue(pending)

    def test_other_environment_requires_review_even_when_pngs_match(self):
        failures, pending = check(self.root, self.manifest, self.candidate, {"platform": "Linux"})
        self.assertFalse(failures)
        self.assertTrue(pending)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument("--check-snapshots", type=Path, metavar="CANDIDATE_DIRECTORY")
    parser.add_argument("--check-generated-rust", type=Path, metavar="GENERATED_RUST_FILE")
    parser.add_argument("--check-resolved-contract", type=Path, metavar="RESOLVED_TOKENS_JSON")
    arguments = parser.parse_args()
    if arguments.self_test:
        result = unittest.TextTestRunner(verbosity=2).run(unittest.defaultTestLoader.loadTestsFromTestCase(BaselineTests))
        return 0 if result.wasSuccessful() else 1
    root = Path(__file__).resolve().parents[1]
    try:
        manifest = json.loads((root / MANIFEST).read_text())
    except (OSError, ValueError) as error:
        print(f"Design baseline FAILED: {error}", file=sys.stderr)
        return 1
    failures, pending = check(root, manifest, arguments.check_snapshots, generated_rust=arguments.check_generated_rust, resolved_contract=arguments.check_resolved_contract)
    for issue in failures:
        print("FAILED: " + issue, file=sys.stderr)
    for issue in pending:
        print("NEEDS REVIEW / UNVERIFIED: " + issue, file=sys.stderr)
    if failures:
        return 1
    if pending:
        return 2
    if arguments.check_snapshots is None:
        print("DSv1 metadata, approved token source/migration bindings, visual source hashes and 24 golden files passed. No candidate was rendered or visually revalidated.")
    else:
        print("DSv1 candidate: all 24 PNGs are byte-identical, and checker environment matches. Reviewer must separately verify candidate build/source provenance.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
