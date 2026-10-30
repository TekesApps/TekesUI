#!/usr/bin/env python3
"""Replay archived preview guard evidence offline; never call the native launcher."""
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import sys
import unittest

sys.dont_write_bytecode = True


def reports_from(attempt):
    """Authenticate only the raw files consumed by this evidence replay."""
    result = json.loads((attempt / "native-result.json").read_text())
    paths = sorted((attempt / "evidence").glob("checkpoint-*.json"))
    for path in paths + [attempt / "evidence" / name for name in
                         ("native-events.jsonl", "journal.jsonl")]:
        expected = result["evidence_sha256"][str(path.relative_to(attempt))]
        if hashlib.sha256(path.read_bytes()).hexdigest() != expected:
            raise ValueError("Archived input differs from native result: " + str(path))
    reports = {report["phase"]: report for report in
               (json.loads(path.read_text()) for path in paths)}
    return result, reports


class GuardEvidenceRegression(unittest.TestCase):
    def test_later_abort_cannot_hide_a_reported_pass_with_wrong_plain_text(self):
        result, reports = self.failed
        self.assertNotEqual(result["exit_code"], 0)
        self.assertFalse(result["timeout"])
        self.assertTrue(reports[46005]["passed"])
        self.assertIn(46005, result["native_software_assertions"]["passed_checkpoints"])
        self.assertEqual(reports[46005]["state"]["text"],
                         "中Original composer draft文 composer draft")
        errors = self.guard.reached_setup_errors(reports)
        self.assertEqual(len(errors), 1)
        self.assertIn("46005", errors[0])
        self.assertEqual(result["native_software_assertions"]["not_run"], [46025, 46026])
        self.assertFalse(any(str(phase) in error for phase in (46025, 46026)
                             for error in errors))

    def test_actual_marked_prefix_does_not_invent_an_unreached_plain_failure(self):
        _, reports = self.failed
        prefix = {phase: report for phase, report in reports.items() if phase <= 46004}
        self.assertEqual(set(prefix), set(range(46001, 46005)))
        self.assertEqual(self.guard.reached_setup_errors(prefix), [])

    def test_accepted_run_keeps_exact_setup_and_original_full_guard_acceptance(self):
        result, reports = self.accepted
        self.assertEqual(result["exit_code"], 0)
        self.assertFalse(result["timeout"])
        self.assertEqual(set(reports), set(range(46001, 46027)))
        self.assertEqual(self.guard.reached_setup_errors(reports), [])
        self.assertEqual(reports[46001]["state"]["text"], "中文 composer draft")
        self.assertEqual(reports[46005]["state"]["text"], "Original composer draft")
        assertions = self.guard.native_assertions(self.accepted_path)
        self.assertTrue(assertions["passed"], assertions["errors"])
        self.assertEqual(assertions["checkpoints"], 26)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--runner-dir", type=Path, required=True,
                        help="Trusted archived original26 runner with binding.json")
    parser.add_argument("--failed-attempt", type=Path, required=True,
                        help="Original late-abort attempt with wrong plain setup")
    parser.add_argument("--accepted-attempt", type=Path, required=True,
                        help="Accepted original26 attempt")
    args = parser.parse_args()
    runner = args.runner_dir.resolve(strict=True)
    binding = json.loads((runner / "binding.json").read_text())
    path = runner / "run_native.py"
    if hashlib.sha256(path.read_bytes()).hexdigest() != binding["runner_sha256"]:
        raise ValueError("Runner differs from its archived binding")
    spec = importlib.util.spec_from_file_location("archived_preview_guard", path)
    guard = importlib.util.module_from_spec(spec)
    # This trusted module defines helpers; its __main__ launcher is never invoked.
    spec.loader.exec_module(guard)
    GuardEvidenceRegression.guard = guard
    GuardEvidenceRegression.failed = reports_from(args.failed_attempt.resolve(strict=True))
    GuardEvidenceRegression.accepted_path = args.accepted_attempt.resolve(strict=True)
    GuardEvidenceRegression.accepted = reports_from(GuardEvidenceRegression.accepted_path)
    print(json.dumps({"scope": "offline archived evidence replay only",
                      "runner_sha256": binding["runner_sha256"],
                      "failed_attempt": str(args.failed_attempt),
                      "accepted_attempt": str(args.accepted_attempt),
                      "native_launch": False}, ensure_ascii=False), flush=True)
    result = unittest.TextTestRunner(verbosity=2).run(
        unittest.defaultTestLoader.loadTestsFromTestCase(GuardEvidenceRegression))
    return 0 if result.wasSuccessful() else 1


if __name__ == "__main__":
    raise SystemExit(main())
