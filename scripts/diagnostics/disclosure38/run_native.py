"""One Disclosure attempt using the existing coordinator guards. Never builds or retries."""
import argparse
import datetime
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import traceback
import uuid

sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent


class Refusal(RuntimeError):
    """The requested native window cannot safely start this attempt."""


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def write_json(path, value):
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n")


def load_module(name, spec):
    path = Path(spec["path"])
    if sha(path) != spec["sha256"]:
        raise Refusal("Existing guard/helper changed: " + str(path))
    module_spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(module_spec)
    module_spec.loader.exec_module(module)
    return module


def read_displays():
    """Read-only CoreGraphics inventory; same query as #25 read_displays.py.

    Mirror predicates use the local SDK CGDisplayConfiguration.h signatures.
    No AppKit application, display configuration or window operation is used.
    """
    import ctypes

    class Point(ctypes.Structure):
        _fields_ = [("x", ctypes.c_double), ("y", ctypes.c_double)]

    class Size(ctypes.Structure):
        _fields_ = [("width", ctypes.c_double), ("height", ctypes.c_double)]

    class Rect(ctypes.Structure):
        _fields_ = [("origin", Point), ("size", Size)]

    api = ctypes.CDLL("/System/Library/Frameworks/CoreGraphics.framework/CoreGraphics")
    api.CGGetActiveDisplayList.argtypes = [ctypes.c_uint32, ctypes.POINTER(ctypes.c_uint32), ctypes.POINTER(ctypes.c_uint32)]
    api.CGGetActiveDisplayList.restype = ctypes.c_int32
    for name in ("CGDisplayIsBuiltin", "CGDisplayIsInMirrorSet", "CGDisplayMirrorsDisplay"):
        function = getattr(api, name)
        function.argtypes = [ctypes.c_uint32]
        function.restype = ctypes.c_uint32
    api.CGDisplayBounds.argtypes = [ctypes.c_uint32]
    api.CGDisplayBounds.restype = Rect
    ids = (ctypes.c_uint32 * 32)()
    count = ctypes.c_uint32()
    if api.CGGetActiveDisplayList(32, ids, ctypes.byref(count)) != 0:
        raise Refusal("CoreGraphics active display query failed")
    return [{"display_id": display, "builtin": bool(api.CGDisplayIsBuiltin(display)),
             "mirrored": bool(api.CGDisplayIsInMirrorSet(display)),
             "mirrors_display": api.CGDisplayMirrorsDisplay(display),
             "bounds": {"x": bounds.origin.x, "y": bounds.origin.y,
                        "width": bounds.size.width, "height": bounds.size.height}}
            for display in ids[:count.value] for bounds in [api.CGDisplayBounds(display)]]


def require_external_display(displays, target):
    """Check the coordinator-bound target without selecting or moving a window."""
    if type(target) is not int or not 0 < target <= 0xffffffff:
        raise Refusal("A valid external_display_id is required in the prepared binding")
    if not any(not display["builtin"] for display in displays):
        raise Refusal("No active external display")
    if any(display["mirrored"] or display["mirrors_display"] for display in displays):
        raise Refusal("Mirrored displays are outside the external-only native window")
    selected = next((display for display in displays if display["display_id"] == target), None)
    if selected is None:
        raise Refusal("Bound external display target is not active")
    if selected["builtin"]:
        raise Refusal("Bound display target is builtin")


def verify_inputs(binding):
    main = Path(binding["main"])
    actual = {path: sha(main / path) for path in binding["sources_and_manifests"]}
    if actual != binding["sources_and_manifests"]:
        changed = [path for path in actual if actual[path] != binding["sources_and_manifests"][path]]
        raise Refusal("Related source/manifest changed: " + ", ".join(changed))
    program = Path(binding["program"]["path"])
    if (sha(program) != binding["program"]["sha256"] or not os.access(program, os.X_OK)):
        raise Refusal("Archived native program identity or executable mode changed")
    return {"program_sha256": sha(program), "sources_and_manifests": actual}


def stop_owned_process(process):
    # Only this attempt's Popen child; never search for or terminate other tasks.
    if process is not None and process.poll() is None:
        process.terminate()
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait()


IMAGES = ("collapsed-light.png", "expanded-focus-light.png", "marked-collapse-refused.png",
          "disabled-light.png", "narrow-rtl-dark.png", "collapsed-rtl-dark.png")


def native_assertions(attempt):
    """Independent raw journal checks, including reached setup on late abort."""
    evidence = Path(attempt) / "evidence"
    journal = evidence / "disclosure38-journal.jsonl"
    reports = [json.loads(line) for line in journal.read_text().splitlines() if line.strip()] if journal.is_file() else []
    errors = []
    phases = [r.get("phase") for r in reports]
    if phases != list(range(38001, 38016)):
        errors.append("native checkpoints missing, duplicated or out of order")
    counts = [0, 1, 2, 3, 3, 4, 5, 6, 6, 6, 6, 6, 6, 7, 7]
    expanded = [False, True, True, False, False, True, True, True, True, True, True, True, True, False, False]
    mounts = None
    for report in reports:
        phase = report.get("phase")
        if phase not in range(38001, 38016):
            errors.append("unknown phase")
            continue
        i = phase - 38001
        if report.get("passed") is not True or report.get("window_active") is not True or report.get("programmatic_AppKit") is not True:
            errors.append(f"checkpoint {phase} did not pass in active window")
        if report.get("expanded") is not expanded[i] or len(report.get("records", [])) != counts[i]:
            errors.append(f"checkpoint {phase} controlled state/action count differs")
        actual_mounts = (report.get("trigger_mount"), report.get("editor_mount"), report.get("outside_mount"))
        if mounts is None:
            mounts = actual_mounts
        if actual_mounts != mounts or None in actual_mounts:
            errors.append(f"checkpoint {phase} original mount differs")
        if report.get("outside") != {"text":"Keep A😀B", "selection":{"start":0,"end":0}, "marked":None}:
            errors.append(f"checkpoint {phase} unrelated editor changed")
        records = report.get("records", [])
        if [record.get("source") for record in records] != (["Pointer", "Keyboard", "Keyboard"] + ["Accessibility"] * 4)[:counts[i]]:
            errors.append(f"checkpoint {phase} action source prefix differs")
        if any(record.get("target") != 38002 or record.get("mount") != report.get("trigger_mount") for record in records):
            errors.append(f"checkpoint {phase} action owner differs")
        if phase >= 38009 and report.get("public_sdk_marked_ax_refused") is not True:
            errors.append(f"checkpoint {phase} public SDK marked AX refusal missing")
        editor = report.get("editor", {})
        if phase <= 38008 and editor.get("text") != "Original body draft":
            errors.append(f"checkpoint {phase} original draft differs")
        if phase in (38009, 38010) and editor != {"text":"中文 body draft", "selection":{"start":1,"end":1}, "marked":{"start":0,"end":2}}:
            errors.append(f"checkpoint {phase} fixed marked setup differs")
        if phase >= 38011 and editor != {"text":"中文 body draft", "selection":{"start":1,"end":1}, "marked":None}:
            errors.append(f"checkpoint {phase} unmarked body state differs")
    if reports:
        final = reports[-1]
        records = final.get("records", [])
        if [r.get("source") for r in records] != ["Pointer", "Keyboard", "Keyboard"] + ["Accessibility"] * 4:
            errors.append("pointer/Enter/Space/AX source sequence differs")
        if any(r.get("target") != 38002 or r.get("mount") != final.get("trigger_mount") for r in records):
            errors.append("action has a second owner or mount")
        if len({r.get("generation") for r in records}) != len(records):
            errors.append("generation repeated")
        if final.get("phase") != 38015 or final.get("complete") is not True:
            errors.append("final completion missing")
    for name in IMAGES:
        path = evidence / name
        if not path.is_file() or path.read_bytes()[:8] != b"\x89PNG\r\n\x1a\n":
            errors.append("native screenshot missing or invalid: " + name)
    stderr = Path(attempt) / "stderr.log"
    log = stderr.read_text() if stderr.is_file() else ""
    for token in ("composition-event mode=disclosure event=PointerDown", "composition-event mode=disclosure event=PointerUp",
                  "composition-event mode=disclosure event=Preedit", "SELFTEST PASS AXPress component=38002"):
        if token not in log:
            errors.append("native callback evidence missing: " + token)
    return {"passed": not errors, "errors": errors, "checkpoints":len(reports),
            "scope":"programmatic AppKit only; screenshot existence is not pixel review; no physical IME or VoiceOver"}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binding", type=Path, required=True)
    parser.add_argument("--output-root", type=Path, required=True)
    parser.add_argument("--reduced-motion", choices=("0", "1"), required=True)
    args = parser.parse_args()
    attempt = args.output_root / (datetime.datetime.now(datetime.timezone.utc).strftime("run-%Y%m%dT%H%M%S.%fZ-") + uuid.uuid4().hex[:8])
    attempt.mkdir(parents=True, exist_ok=False)
    process = None
    events = []
    result = {"started_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
              "status": "not_run", "launched": False, "behavior_pass": False,
              "physical_input": False, "physical_ime": "not_run", "voiceover": "not_run",
              "gpu_pixel_acceptance": "not_run", "attempt": str(attempt)}
    exit_code = 2
    try:
        binding_path = args.binding
        binding = json.loads(binding_path.read_text())
        result["binding_sha256"] = sha(binding_path)
        if sha(__file__) != binding["runner_sha256"]:
            raise Refusal("Runner differs from prepared binding")
        build_result = Path(binding["build_result"]["path"])
        if sha(build_result) != binding["build_result"]["sha256"]:
            raise Refusal("Original successful build result changed")
        base = load_module("native_existing_helper_loader", binding["base_loader"])
        guard = load_module("native_existing_gpu_guard", binding["gpu_guard"])
        helper, helper_sha = base.load_build_helper()
        if helper_sha != binding["build_helper_sha256"]:
            raise Refusal("Original shared build helper changed")
        result["build_helper_sha256"] = helper_sha
        if not base.physically_within(args.output_root.resolve(), base.SSD_ROOT):
            raise Refusal("Native evidence output is not on the authorized SSD")
        with helper.build_lock():
            with guard.gpu_lock(helper, events):
                cache, owner = base.verify_cache_owner(helper)
                helper.assert_unused([helper.CACHE])
                result["cache"] = str(cache)
                result["cache_owner"] = owner["root"]
                # One occupancy snapshot and one physical idle snapshot: no wait/retry loop.
                check = subprocess.run(["pgrep", "-fl", binding["native_process_pattern"]], capture_output=True, text=True)
                (attempt / "pgrep-stdout.log").write_text(check.stdout)
                (attempt / "pgrep-stderr.log").write_text(check.stderr)
                result["pgrep_exit_code"] = check.returncode
                if check.returncode != 1:
                    raise Refusal("Native target occupied" if check.returncode == 0 else "Native occupancy query failed")
                hid = subprocess.run(["ioreg", "-c", "IOHIDSystem"], capture_output=True, text=True)
                (attempt / "ioreg-stdout.log").write_text(hid.stdout)
                (attempt / "ioreg-stderr.log").write_text(hid.stderr)
                match = re.search(r'"HIDIdleTime"\s*=\s*(\d+)', hid.stdout)
                if hid.returncode != 0 or match is None:
                    raise Refusal("Physical HID idle identity unavailable")
                idle_seconds = int(match.group(1)) / 1e9
                result["prelaunch_idle_seconds"] = idle_seconds
                if idle_seconds < 60:
                    raise Refusal("Desktop input active within the last 60 seconds")
                result["display_preflight"] = {"api": "CoreGraphics", "read_only": True,
                    "target_display_id": binding.get("external_display_id"), "displays": read_displays()}
                write_json(attempt / "display-preflight.json", result["display_preflight"])
                require_external_display(result["display_preflight"]["displays"], binding.get("external_display_id"))
                result["inputs_before"] = verify_inputs(binding)
                result["program"] = binding["program"]
                result["command"] = [binding["program"]["path"], "disclosure"]
                result["cwd"] = binding["main"]
                result["timeout_seconds"] = 45
                result["compiled_base"] = binding["compiled_base"]
                env = helper.environment()
                evidence = attempt / "evidence"
                evidence.mkdir()
                temporary = attempt / "tmp"
                temporary.mkdir()
                env.update(TEKESUI_WINDOW_PLACEMENT="external", TEKES_COMPOSITION_SELFTEST="disclosure", TEKES_COMPOSITION_REDUCED_MOTION=args.reduced_motion,
                           TEKESUI_COMPONENT_EVIDENCE=str(evidence), TMPDIR=str(temporary))
                result["runtime_environment"] = {name: env.get(name) for name in
                    ("TEKESUI_WINDOW_PLACEMENT", "TEKES_COMPOSITION_SELFTEST", "TEKES_COMPOSITION_REDUCED_MOTION", "TEKESUI_COMPONENT_EVIDENCE", "TMPDIR", "CARGO_TARGET_DIR", "SDKROOT", "TEKESUI_DESIGN_ARTIFACT")}
                write_json(attempt / "launch.json", result)
                with (attempt / "stdout.log").open("w") as stdout, (attempt / "stderr.log").open("w") as stderr:
                    process = subprocess.Popen([binding["program"]["path"], "disclosure"], cwd=binding["main"], env=env, stdout=stdout, stderr=stderr)
                    result["launched"] = True
                    result["pid"] = process.pid
                    try:
                        result["exit_code"] = process.wait(timeout=45)
                        result["timeout"] = False
                    except subprocess.TimeoutExpired:
                        result["timeout"] = True
                        stop_owned_process(process)
                        result["exit_code"] = process.returncode
                    except BaseException:
                        stop_owned_process(process)
                        raise
                result["inputs_after"] = verify_inputs(binding)
                result["native_software_assertions"] = native_assertions(attempt)
                passed = (result["exit_code"] == 0 and not result["timeout"]
                          and result["native_software_assertions"]["passed"])
                result["status"] = "native_software_checks_passed" if passed else "failed"
                result["behavior_pass"] = "pending coordinator review" if passed else False
                result["root_acceptance"] = "pending coordinator journal/native review; no physical IME/VoiceOver/pixel claim"
                exit_code = 0 if passed else 1
    except BaseException as error:
        # Preserve a refusal separately from a launched failure; propagate failure via exit code.
        result["reason"] = str(error)
        result["error_type"] = type(error).__name__
        (attempt / "runner-error.log").write_text(traceback.format_exc())
        result["status"] = "failed" if result["launched"] else "not_run"
        exit_code = 1 if result["launched"] else 2
    finally:
        stop_owned_process(process)
        result["gpu_lock_events"] = events
        result["finished_utc"] = datetime.datetime.now(datetime.timezone.utc).isoformat()
        result["guards_exited"] = True
        result["evidence_sha256"] = {str(path.relative_to(attempt)): sha(path)
            for path in attempt.rglob("*") if path.is_file() and "tmp" not in path.relative_to(attempt).parts}
        # Each invocation owns a new directory; prior attempts and not_run evidence are untouched.
        write_json(attempt / ("native-result.json" if result["launched"] else "not-run.json"), result)
        print(json.dumps({"status": result["status"], "attempt": str(attempt), "launched": result["launched"]}), flush=True)
    return exit_code


if __name__ == "__main__":
    raise SystemExit(main())
