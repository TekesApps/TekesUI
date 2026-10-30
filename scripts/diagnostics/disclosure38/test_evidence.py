"""Offline corruption tests for Disclosure's evidence reader; no native launch."""
import copy
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location("disclosure_runner", Path(__file__).with_name("run_native.py"))
runner = importlib.util.module_from_spec(spec)
spec.loader.exec_module(runner)


class EvidenceTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.attempt = Path(self.temp.name)
        self.evidence = self.attempt / "evidence"
        self.evidence.mkdir()
        counts = [0, 1, 2, 3, 3, 4, 5, 6, 6, 6, 6, 6, 6, 7, 7]
        states = [False, True, True, False, False, True, True, True, True, True, True, True, True, False, False]
        actions = [{"source":source, "target":38002, "mount":10, "generation":i}
                   for i, source in enumerate(["Pointer", "Keyboard", "Keyboard"] + ["Accessibility"] * 4)]
        self.reports = []
        for i in range(15):
            marked = i in (8, 9)
            self.reports.append({"phase":38001+i, "passed":True, "window_active":True, "programmatic_AppKit":True,
                "public_sdk_marked_ax_refused":i>=8, "complete":i==14, "expanded":states[i], "records":copy.deepcopy(actions[:counts[i]]),
                "trigger_mount":10, "editor_mount":11, "outside_mount":12,
                "outside":{"text":"Keep A😀B", "selection":{"start":0,"end":0}, "marked":None},
                "editor":{"text":"Original body draft" if i<8 else "中文 body draft",
                          "selection":{"start":1,"end":1} if i>=8 else {"start":0,"end":0},
                          "marked":{"start":0,"end":2} if marked else None}})
        for name in runner.IMAGES:
            # Synthetic signature only. These tests do not produce screenshot evidence.
            (self.evidence / name).write_bytes(b"\x89PNG\r\n\x1a\n")
        (self.attempt / "stderr.log").write_text("\n".join([
            "composition-event mode=disclosure event=Pointer(PointerEvent { position: Point { x: 360.0, y: 34.0 }, phase: Down, button: Some(Primary) })",
            "composition-event mode=disclosure event=Pointer(PointerEvent { position: Point { x: 360.0, y: 34.0 }, phase: Up, button: Some(Primary) })",
            "composition-event mode=disclosure event=Preedit",
            "SELFTEST PASS AXPress component=38002"]))

    def tearDown(self):
        self.temp.cleanup()

    def check(self):
        (self.evidence / "disclosure38-journal.jsonl").write_text("\n".join(json.dumps(r) for r in self.reports))
        return runner.native_assertions(self.attempt)

    def test_complete_reader_contract(self):
        self.assertTrue(self.check()["passed"])

    def test_each_current_pointer_phase_is_required(self):
        path = self.attempt / "stderr.log"
        original = path.read_text()
        for phase in ("Down", "Up"):
            path.write_text("\n".join(line for line in original.splitlines() if "phase: " + phase not in line))
            self.assertFalse(self.check()["passed"])
        path.write_text(original)
        self.assertTrue(self.check()["passed"])

    def test_other_mode_or_nonpointer_phase_cannot_supply_pointer_evidence(self):
        path = self.attempt / "stderr.log"
        original = path.read_text()
        for replacement in (original.replace("mode=disclosure event=Pointer(", "mode=menu-tree event=Pointer("),
                            original.replace("event=Pointer(PointerEvent", "event=Other(PointerEvent")):
            path.write_text(replacement)
            self.assertFalse(self.check()["passed"])

    def test_late_abort_still_checks_reached_marked_setup(self):
        self.reports = self.reports[:10]
        self.reports[8]["editor"]["text"] = "中Original body draft文"
        result = self.check()
        self.assertFalse(result["passed"])
        self.assertIn("checkpoint 38009 fixed marked setup differs", result["errors"])

    def test_each_missing_phase_duplicate_action_owner_and_generation_is_rejected(self):
        original = copy.deepcopy(self.reports)
        mutations = [lambda: self.reports.pop(4),
                     lambda: self.reports[-1]["records"].append(self.reports[-1]["records"][0]),
                     lambda: self.reports[-1]["records"][0].update(target=38003),
                     lambda: self.reports[-1]["records"][1].update(generation=0),
                     lambda: self.reports[-1].update(complete=False),
                     lambda: self.reports[8].update(public_sdk_marked_ax_refused=False),
                     lambda: self.reports[2]["outside"].update(text="changed"),
                     lambda: self.reports[4].update(outside_mount=99),
                     lambda: self.reports[11]["editor"].update(text="changed"),
                     lambda: self.reports[12]["editor"].update(selection={"start":0,"end":0}),
                     lambda: self.reports[2]["records"][0].update(target=38003),
                     lambda: self.reports[3]["records"][1].update(source="Pointer")]
        for mutate in mutations:
            self.reports = copy.deepcopy(original)
            mutate()
            self.assertFalse(self.check()["passed"])

    def test_native_callbacks_and_images_are_required(self):
        (self.attempt / "stderr.log").write_text("")
        (self.evidence / runner.IMAGES[0]).unlink()
        self.assertFalse(self.check()["passed"])


class DisplayPreflightTests(unittest.TestCase):
    @staticmethod
    def displays():
        return [{"display_id": 1, "builtin": True, "mirrored": False, "mirrors_display": 0},
                {"display_id": 2, "builtin": False, "mirrored": False, "mirrors_display": 0}]

    def test_bound_unmirrored_external_is_allowed(self):
        runner.require_external_display(self.displays(), 2)

    def test_refusal_paths_never_launch_or_claim_native_success(self):
        from contextlib import nullcontext, redirect_stdout
        from io import StringIO
        from types import SimpleNamespace
        from unittest.mock import patch

        mirrored = self.displays()
        mirrored[1]["mirrored"] = True
        secondary = self.displays()
        secondary[1]["mirrors_display"] = 1
        cases = [(self.displays()[:1], 2, "No active external display"),
                 (mirrored, 2, "Mirrored displays"),
                 (secondary, 2, "Mirrored displays"),
                 (self.displays(), 99, "target is not active"),
                 (self.displays(), 1, "target is builtin"),
                 (self.displays(), None, "external_display_id is required")]
        for displays, target, reason in cases:
            with self.subTest(reason=reason), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                built = root / "build.json"
                built.write_text("{}")
                binding = root / "binding.json"
                binding.write_text(json.dumps({"runner_sha256": runner.sha(runner.__file__),
                    "build_result": {"path": str(built), "sha256": runner.sha(built)},
                    "base_loader": {}, "gpu_guard": {}, "build_helper_sha256": "helper",
                    "native_process_pattern": "fixture-only", "external_display_id": target}))
                helper = SimpleNamespace(CACHE=root, build_lock=nullcontext, assert_unused=lambda _: None)
                base = SimpleNamespace(SSD_ROOT=root, physically_within=lambda *_: True,
                    load_build_helper=lambda: (helper, "helper"), verify_cache_owner=lambda _: (root, {"root":str(root)}))
                guard = SimpleNamespace(gpu_lock=lambda *_: nullcontext())
                commands = [SimpleNamespace(returncode=1, stdout="", stderr=""),
                            SimpleNamespace(returncode=0, stdout='"HIDIdleTime" = 61000000000', stderr="")]
                with patch.object(runner.sys, "argv", ["run_native.py", "--binding", str(binding),
                         "--output-root", str(root / "attempts"), "--reduced-motion", "0"]), \
                     patch.object(runner, "load_module", side_effect=[base, guard]), \
                     patch.object(runner, "read_displays", return_value=displays), \
                     patch.object(runner.subprocess, "run", side_effect=commands), \
                     patch.object(runner.subprocess, "Popen") as launch, redirect_stdout(StringIO()):
                    self.assertEqual(runner.main(), 2)
                    launch.assert_not_called()
                result_path, = (root / "attempts").glob("*/not-run.json")
                result = json.loads(result_path.read_text())
                self.assertEqual(result["status"], "not_run")
                self.assertFalse(result["launched"])
                self.assertFalse(result["behavior_pass"])
                self.assertIn(reason, result["reason"])
                self.assertEqual(result["display_preflight"]["displays"], displays)


if __name__ == "__main__":
    unittest.main()
