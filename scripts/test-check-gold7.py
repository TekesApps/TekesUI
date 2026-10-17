from __future__ import annotations

import importlib.util
import hashlib
import json
import subprocess
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location("check_gold7", Path(__file__).with_name("check-gold7.py"))
check_gold7 = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(check_gold7)


class Gold7CheckerTests(unittest.TestCase):
    def read(self, path: str):
        import json

        return json.loads((ROOT / path).read_text(encoding="utf-8"))

    def test_alignment_worklist_cannot_omit_or_duplicate_requirements(self):
        worklist = self.read("design/behavior/gold7-alignment.json")
        worklist["requirements"].pop()
        self.assertTrue(any("cover each Gold7 requirement exactly once" in error
            for error in check_gold7.check(ROOT, {"design/behavior/gold7-alignment.json": worklist})))
        worklist = self.read("design/behavior/gold7-alignment.json")
        worklist["requirements"].append(worklist["requirements"][0])
        self.assertTrue(any("cover each Gold7 requirement exactly once" in error
            for error in check_gold7.check(ROOT, {"design/behavior/gold7-alignment.json": worklist})))

    def test_alignment_worklist_cannot_claim_experience_pass(self):
        worklist = self.read("design/behavior/gold7-alignment.json")
        worklist["requirements"][0]["experience"] = "pass"
        self.assertTrue(any("cannot promote experience" in error for error in
            check_gold7.check(ROOT, {"design/behavior/gold7-alignment.json": worklist})))

    def test_focus_cannot_fall_back_to_a_single_border(self):
        base = self.read("design/source/base.tokens.json")
        next(token for token in base["stroke"]["tokens"] if token["name"] == "c-focus-width")["value"] = "{stroke-rule}"
        self.assertTrue(any("independent two-point ring" in error for error in
            check_gold7.check(ROOT, {"design/source/base.tokens.json": base})))

    def test_current_gold7_sources_and_roles_are_consistent(self):
        self.assertEqual(check_gold7.check(ROOT), [])

    def test_missing_or_duplicate_gold7_item_fails(self):
        gold = self.read("design/behavior/gold7.json")
        gold["items"].pop()
        errors = check_gold7.check(ROOT, {"design/behavior/gold7.json": gold})
        self.assertTrue(any("ordered, unique set G1 through G7" in error for error in errors))

    def test_invalid_status_and_unsubstantiated_pass_fail(self):
        gold = self.read("design/behavior/gold7.json")
        gold["items"][0]["assessment"]["code"]["status"] = "implemented"
        errors = check_gold7.check(ROOT, {"design/behavior/gold7.json": gold})
        self.assertTrue(any("invalid assessment status" in error for error in errors))

        gold = self.read("design/behavior/gold7.json")
        gold["items"][0]["assessment"]["experience"]["status"] = "pass"
        errors = check_gold7.check(ROOT, {"design/behavior/gold7.json": gold})
        self.assertTrue(any("pass requires verifiable evidence artifacts" in error for error in errors))

    def test_core_gold7_requirements_cannot_be_removed(self):
        gold = self.read("design/behavior/gold7.json")
        gold["items"][0]["requirements"] = [
            requirement for requirement in gold["items"][0]["requirements"]
            if requirement["id"] != "whole-page-reading-composition"
        ]
        errors = check_gold7.check(ROOT, {"design/behavior/gold7.json": gold})
        self.assertTrue(any("missing core requirements whole-page-reading-composition" in error for error in errors))

    def test_missing_explicit_media_gap_is_not_reported_as_unverified(self):
        gold = self.read("design/behavior/gold7.json")
        gold["items"][5]["assessment"]["code"]["status"] = "unverified"
        errors = check_gold7.check(ROOT, {"design/behavior/gold7.json": gold})
        self.assertTrue(any("explicitly missing media runtime as fail" in error for error in errors))

    def test_pass_requires_artifact_and_immutable_input_manifest(self):
        gold = self.read("design/behavior/gold7.json")
        gold["items"][0]["assessment"]["code"] = {
            "status": "pass",
            "evidence_refs": [
                {
                    "artifact": {"$ref": "../../docs/design/gold7.md"},
                    "method": "human-review",
                    "identity": {
                        "kind": "immutable_git_commit",
                        "manifest": {"$ref": "./gold7.json"},
                        "commit": "0" * 40,
                    },
                }
            ],
        }
        errors = check_gold7.check(ROOT, {"design/behavior/gold7.json": gold})
        self.assertTrue(any("Git verification failed" in error or "full 40-character" in error for error in errors))

    def test_pass_evidence_must_match_committed_manifest_artifact_and_input(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            for directory in ("design/behavior", "evidence", "input"):
                (root / directory).mkdir(parents=True, exist_ok=True)
            (root / "design/behavior/gold7.json").write_text("{}\n", encoding="utf-8")
            artifact_path = root / "evidence/review.txt"
            artifact_path.write_text("reviewed fixture\n", encoding="utf-8")
            source_path = root / "input/source.txt"
            source_path.write_text("reviewed input\n", encoding="utf-8")
            manifest_path = root / "evidence/manifest.json"
            manifest_path.write_text(json.dumps({
                "format": "tekesui-gold7-evidence-inputs/1",
                "inputs": [
                    {"path": "evidence/review.txt", "sha256": hashlib.sha256(artifact_path.read_bytes()).hexdigest()},
                    {"path": "input/source.txt", "sha256": hashlib.sha256(source_path.read_bytes()).hexdigest()},
                ],
            }) + "\n", encoding="utf-8")
            subprocess.run(["git", "init", "-q", str(root)], check=True)
            subprocess.run(["git", "-C", str(root), "-c", "user.name=Gold7 test", "-c", "user.email=gold7@example.invalid", "add", "."], check=True)
            subprocess.run(["git", "-C", str(root), "-c", "user.name=Gold7 test", "-c", "user.email=gold7@example.invalid", "commit", "-qm", "fixture"], check=True)
            commit = subprocess.run(["git", "-C", str(root), "rev-parse", "HEAD"], check=True, text=True, capture_output=True).stdout.strip()

            def resolve(source_relative, reference):
                target_text, separator, fragment = reference.partition("#")
                source = root / source_relative
                target = (source.parent / target_text).resolve() if target_text else source.resolve()
                if not target.is_file():
                    raise ValueError("missing test reference")
                return target, None if not separator else {}

            entry = {
                "artifact": {"$ref": "../../evidence/review.txt"},
                "method": "human-review",
                "identity": {
                    "kind": "immutable_git_commit",
                    "manifest": {"$ref": "../../evidence/manifest.json"},
                    "commit": commit,
                },
            }
            check_gold7._verify_pass_evidence(root, entry, resolve)

            source_path.write_text("changed after the cited review\n", encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "input bytes do not match"):
                check_gold7._verify_pass_evidence(root, entry, resolve)

    def test_frozen_worktree_manifest_accepts_uncommitted_exact_bytes(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            for directory in ("design/behavior", "evidence", "input"):
                (root / directory).mkdir(parents=True, exist_ok=True)
            (root / "design/behavior/gold7.json").write_text("{}\n", encoding="utf-8")
            artifact_path = root / "evidence/review.txt"
            source_path = root / "input/source.txt"
            artifact_path.write_text("frozen review bytes\n", encoding="utf-8")
            source_path.write_text("frozen source bytes\n", encoding="utf-8")
            subprocess.run(["git", "init", "-q", str(root)], check=True)
            subprocess.run(["git", "-C", str(root), "-c", "user.name=Gold7 test", "-c", "user.email=gold7@example.invalid", "add", "."], check=True)
            subprocess.run(["git", "-C", str(root), "-c", "user.name=Gold7 test", "-c", "user.email=gold7@example.invalid", "commit", "-qm", "baseline"], check=True)

            # The frozen evidence and input are newer working-tree bytes and intentionally uncommitted.
            artifact_path.write_text("new frozen review bytes\n", encoding="utf-8")
            source_path.write_text("new frozen source bytes\n", encoding="utf-8")
            manifest = {
                "format": "tekesui-gold7-evidence-inputs/1",
                "inputs": [
                    {"path": "evidence/review.txt", "sha256": hashlib.sha256(artifact_path.read_bytes()).hexdigest()},
                    {"path": "input/source.txt", "sha256": hashlib.sha256(source_path.read_bytes()).hexdigest()},
                ],
            }
            manifest_path = root / "evidence/manifest.json"
            manifest_path.write_text(json.dumps(manifest, sort_keys=True) + "\n", encoding="utf-8")
            manifest_digest = hashlib.sha256(manifest_path.read_bytes()).hexdigest()

            def resolve(source_relative, reference):
                target_text, separator, fragment = reference.partition("#")
                source = root / source_relative
                target = (source.parent / target_text).resolve() if target_text else source.resolve()
                if not target.is_file():
                    raise ValueError("missing test reference")
                return target, None if not separator else {}

            entry = {
                "artifact": {"$ref": "../../evidence/review.txt"},
                "method": "human-review",
                "identity": {
                    "kind": "frozen_worktree_manifest",
                    "manifest": {"$ref": "../../evidence/manifest.json", "sha256": manifest_digest},
                },
            }
            check_gold7._verify_pass_evidence(root, entry, resolve)
            git_status = subprocess.run(["git", "-C", str(root), "status", "--short"], check=True, text=True, capture_output=True).stdout
            self.assertTrue(git_status.strip(), "fixture must remain uncommitted to prove frozen-worktree acceptance")

    def test_frozen_worktree_manifest_rejects_stale_digest_missing_source_and_escaping_path(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            for directory in ("design/behavior", "evidence", "input"):
                (root / directory).mkdir(parents=True, exist_ok=True)
            (root / "design/behavior/gold7.json").write_text("{}\n", encoding="utf-8")
            artifact_path = root / "evidence/review.txt"
            source_path = root / "input/source.txt"
            artifact_path.write_text("review\n", encoding="utf-8")
            source_path.write_text("source\n", encoding="utf-8")
            manifest_path = root / "evidence/manifest.json"
            manifest = {
                "format": "tekesui-gold7-evidence-inputs/1",
                "inputs": [
                    {"path": "evidence/review.txt", "sha256": hashlib.sha256(artifact_path.read_bytes()).hexdigest()},
                    {"path": "input/source.txt", "sha256": hashlib.sha256(source_path.read_bytes()).hexdigest()},
                ],
            }

            def write_manifest(value):
                manifest_path.write_text(json.dumps(value, sort_keys=True) + "\n", encoding="utf-8")
                return hashlib.sha256(manifest_path.read_bytes()).hexdigest()

            def resolve(source_relative, reference):
                target_text, separator, fragment = reference.partition("#")
                source = root / source_relative
                target = (source.parent / target_text).resolve() if target_text else source.resolve()
                if not target.is_file():
                    raise ValueError("missing test reference")
                return target, None if not separator else {}

            def entry(digest):
                return {
                    "artifact": {"$ref": "../../evidence/review.txt"},
                    "method": "human-review",
                    "identity": {
                        "kind": "frozen_worktree_manifest",
                        "manifest": {"$ref": "../../evidence/manifest.json", "sha256": digest},
                    },
                }

            digest = write_manifest(manifest)
            with self.assertRaisesRegex(ValueError, "digest is stale or tampered"):
                check_gold7._verify_pass_evidence(root, entry("0" * 64), resolve)

            artifact_path.write_text("tampered review\n", encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "input bytes do not match manifest hash"):
                check_gold7._verify_pass_evidence(root, entry(digest), resolve)
            artifact_path.write_text("review\n", encoding="utf-8")
            source_path.write_text("tampered source\n", encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "input bytes do not match manifest hash"):
                check_gold7._verify_pass_evidence(root, entry(digest), resolve)
            source_path.write_text("source\n", encoding="utf-8")

            source_path.unlink()
            with self.assertRaisesRegex(ValueError, "does not exist as a file"):
                check_gold7._verify_pass_evidence(root, entry(digest), resolve)
            source_path.write_text("source\n", encoding="utf-8")

            escaped = json.loads(json.dumps(manifest))
            escaped["inputs"][1]["path"] = "../../outside.txt"
            escaped["inputs"][1]["sha256"] = "0" * 64
            digest = write_manifest(escaped)
            with self.assertRaisesRegex(ValueError, "escapes the repository"):
                check_gold7._verify_pass_evidence(root, entry(digest), resolve)

    def test_six_or_more_dynamic_brand_records_are_supported(self):
        gallery = self.read("design/brand/gallery.json")
        profiles = gallery["presets"]
        self.assertGreaterEqual(len(profiles), 6)
        self.assertEqual(check_gold7.check(ROOT), [])

    def test_brand_record_ids_and_runtime_indices_must_be_unique(self):
        gallery = self.read("design/brand/gallery.json")
        gallery["presets"][1]["runtime_index"] = gallery["presets"][0]["runtime_index"]
        errors = check_gold7.check(ROOT, {"design/brand/gallery.json": gallery})
        self.assertTrue(any("runtime_index values must be unique" in error for error in errors))

        gallery = self.read("design/brand/gallery.json")
        gallery["presets"][1]["id"] = gallery["presets"][0]["id"]
        errors = check_gold7.check(ROOT, {"design/brand/gallery.json": gallery})
        self.assertTrue(any("profile ids must be unique" in error for error in errors))

    def test_machine_and_review_assessments_are_separate(self):
        import json
        import subprocess

        result = subprocess.run(
            ["python3", str(Path(__file__).with_name("check-gold7.py")), "--json"],
            cwd=ROOT,
            text=True,
            capture_output=True,
            check=True,
        )
        report = json.loads(result.stdout)
        self.assertEqual(report["mechanical_checks"]["status"], "pass")
        self.assertEqual(report["review_assessments"]["status"], "separate-review-required")
        self.assertEqual(
            next(item for item in report["review_assessments"]["items"] if item["id"] == "G6")["code"]["status"],
            "fail",
        )

    def test_not_applicable_requires_a_scope_reason(self):
        gold = self.read("design/behavior/gold7.json")
        gold["items"][1]["assessment"]["code"]["status"] = "not-applicable"
        errors = check_gold7.check(ROOT, {"design/behavior/gold7.json": gold})
        self.assertTrue(any("not-applicable requires a reason" in error for error in errors))

    def test_dangling_pointer_fails(self):
        gold = self.read("design/behavior/gold7.json")
        gold["items"][0]["requirements"][0]["sources"][0]["$ref"] = "../source/layout.json#/missing/rule"
        errors = check_gold7.check(ROOT, {"design/behavior/gold7.json": gold})
        self.assertTrue(any("missing or invalid JSON pointer" in error for error in errors))

    def test_effective_interaction_cannot_drift_back_to_package_import(self):
        system = self.read("design/system.json")
        system["layers"]["interaction"]["$ref"] = "./imports/v0.3.0-proposal1/upstream/TekesUI-Design-System-v0.3.0-proposal1/design/interaction-contracts.json"
        errors = check_gold7.check(ROOT, {"design/system.json": system})
        self.assertTrue(any("expected effective local source" in error for error in errors))

    def test_surface_semantic_roles_must_alias_primitive_tiers(self):
        base = self.read("design/source/base.tokens.json")
        surface_2 = next(item for item in base["color"]["tokens"] if item["name"] == "surface-2")
        surface_2["value"] = "{p-surface-1}"
        errors = check_gold7.check(ROOT, {"design/source/base.tokens.json": base})
        self.assertTrue(any("surface-2: semantic role must alias p-surface-2" in error for error in errors))

    def test_surface_luminance_order_is_checked_per_theme(self):
        base = self.read("design/source/base.tokens.json")
        surface_0 = next(item for item in base["color"]["tokens"] if item["name"] == "p-surface-0")
        surface_0["value"]["light"] = "#FFFFFF"
        errors = check_gold7.check(ROOT, {"design/source/base.tokens.json": base})
        self.assertTrue(any("light: surface role luminance order" in error for error in errors))

    def test_malformed_surface_primitive_is_reported_without_crashing(self):
        base = self.read("design/source/base.tokens.json")
        surface_1 = next(item for item in base["color"]["tokens"] if item["name"] == "p-surface-1")
        surface_1["value"] = "{missing-surface}"
        errors = check_gold7.check(ROOT, {"design/source/base.tokens.json": base})
        self.assertTrue(any("p-surface-1: explicit values" in error for error in errors))

    def test_required_and_decorative_boundary_roles_must_remain_distinct(self):
        layout = self.read("design/source/layout.json")
        layout["boundary_styles"]["required_control"]["color"] = "separator"
        errors = check_gold7.check(ROOT, {"design/source/layout.json": layout})
        self.assertTrue(any("required and decorative boundaries must remain distinct" in error for error in errors))

    def test_equal_surface_colors_require_geometry_sources(self):
        gold = self.read("design/behavior/gold7.json")
        requirement = next(requirement for requirement in gold["items"][1]["requirements"] if "geometry_sources" in requirement)
        requirement.pop("geometry_sources")
        errors = check_gold7.check(ROOT, {"design/behavior/gold7.json": gold})
        self.assertTrue(any("coincident surface colors require geometry" in error for error in errors))


if __name__ == "__main__":
    unittest.main()
