#!/usr/bin/env python3
"""Unit tests for the Gold7 Calm-UI Band-Pass Auditor skill."""
import copy
import hashlib
import importlib.util
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / ".agents/skills/gold7-calm-audit/scripts/audit_gold7_calm.py"
spec = importlib.util.spec_from_file_location("audit_gold7_calm", SCRIPT)
AUDITOR = importlib.util.module_from_spec(spec)
spec.loader.exec_module(AUDITOR)


class Gold7CalmAuditTests(unittest.TestCase):
    def staged_root(self):
        tmp = tempfile.TemporaryDirectory(prefix="tekes-gold7-calm-")
        self.addCleanup(tmp.cleanup)
        root = Path(tmp.name)
        for rel in (
            "design/source/base.tokens.json",
            "design/brand/policy.json",
            "design/source/contrast-pairs.json",
            "design/theme/rules.json",
        ):
            dst = root / rel
            dst.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(ROOT / rel, dst)
        return root

    def mutate_base(self, root: Path, updates: dict[str, dict[str, str]]) -> None:
        path = root / "design/source/base.tokens.json"
        doc = json.loads(path.read_text(encoding="utf-8"))
        for token in doc["color"]["tokens"]:
            if token["name"] in updates:
                for mode, val in updates[token["name"]].items():
                    token["value"][mode] = val
        path.write_text(json.dumps(doc, indent=2), encoding="utf-8")

    def test_current_repo_passes_calm_band_pass_audit(self):
        report = AUDITOR.audit(ROOT)
        self.assertEqual(report["status"], "pass", report["violations"])
        self.assertEqual(report["violations"], [])

    def test_rejects_glare_primary_text(self):
        root = self.staged_root()
        self.mutate_base(root, {"p-primary-text": {"light": "#151515", "dark": "#F5F5F5"}})
        codes = {v["code"] for v in AUDITOR.audit(root)["violations"]}
        self.assertIn("GLARE_PRIMARY_TEXT", codes)

    def test_rejects_collapsed_secondary_tertiary_hierarchy(self):
        root = self.staged_root()
        self.mutate_base(
            root,
            {
                "p-secondary-text": {"dark": "#A3A3A3"},
                "p-tertiary-text": {"dark": "#9F9F9B"},
            },
        )
        codes = {v["code"] for v in AUDITOR.audit(root)["violations"]}
        self.assertIn("OVER_CONTRAST_SECONDARY", codes)
        self.assertIn("OVER_CONTRAST_TERTIARY", codes)
        self.assertIn("TYPOGRAPHY_COLLAPSE", codes)

    def test_rejects_dark_black_hole_surface(self):
        root = self.staged_root()
        self.mutate_base(root, {"p-surface-0": {"dark": "#0B0B0A"}})
        codes = {v["code"] for v in AUDITOR.audit(root)["violations"]}
        self.assertIn("BLACK_HOLE_SURFACE", codes)

    def test_rejects_overexposed_status_color(self):
        root = self.staged_root()
        self.mutate_base(root, {"p-status-info": {"dark": "#A8D1FF"}})
        codes = {v["code"] for v in AUDITOR.audit(root)["violations"]}
        self.assertIn("OVEREXPOSED_STATUS", codes)

    def test_rejects_weakened_ic_tertiary(self):
        root = self.staged_root()
        self.mutate_base(root, {"p-tertiary-text": {"dark-ic": "#707070"}})
        codes = {v["code"] for v in AUDITOR.audit(root)["violations"]}
        self.assertIn("IC_TEXT_FLOOR", codes)

    def test_scan_components_is_read_only(self):
        tracked = [
            ROOT / "design/theme/rules.json",
            ROOT / "design/source/base.tokens.json",
            ROOT / "design/brand/policy.json",
        ]
        before = {p: hashlib.sha256(p.read_bytes()).hexdigest() for p in tracked}
        proc = subprocess.run(
            [sys.executable, str(SCRIPT), "--root", str(ROOT), "--scan-components", "--json"],
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(proc.returncode, 0, proc.stderr)
        payload = json.loads(proc.stdout)
        self.assertEqual(payload["mode"], "regression-gate")
        self.assertEqual(payload["status"], "pass")
        self.assertEqual(payload["violations"], [])
        self.assertEqual(
            payload["allowlisted_profiles"]["secondary_text"],
            ["data-category-label", "quiet"],
        )
        after = {p: hashlib.sha256(p.read_bytes()).hexdigest() for p in tracked}
        self.assertEqual(before, after)

    def test_scan_components_rejects_non_input_border_and_controls_bypass(self):
        root = self.staged_root()
        rules_path = root / "design/theme/rules.json"
        rules = json.loads(rules_path.read_text(encoding="utf-8"))
        rules["profiles"]["skeleton"]["rest"]["boundary"] = "border"
        rules["profiles"]["progress-track"]["rest"]["text"] = "secondary-text"
        rules_path.write_text(json.dumps(rules, indent=2), encoding="utf-8")

        fake_ctrl = root / "crates/tekes-ui-controls/src/fake_control.rs"
        fake_ctrl.parent.mkdir(parents=True, exist_ok=True)
        fake_ctrl.write_text('let _ = palette.color("border");\n', encoding="utf-8")

        proc = subprocess.run(
            [sys.executable, str(SCRIPT), "--root", str(root), "--scan-components", "--json"],
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(proc.returncode, 1)
        payload = json.loads(proc.stdout)
        self.assertEqual(payload["status"], "fail")
        codes = {v["code"] for v in payload["violations"]}
        self.assertEqual(
            codes,
            {
                "NON_INPUT_PROFILE_BORDER",
                "NON_INPUT_PROFILE_SECONDARY_TEXT",
                "CONTROLS_RAW_PALETTE_BYPASS",
            },
        )


if __name__ == "__main__":
    unittest.main()
