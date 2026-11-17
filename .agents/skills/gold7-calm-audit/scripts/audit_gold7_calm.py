#!/usr/bin/env python3
"""Gold7 Calm-UI Band-Pass Auditor (Plan B).

Evaluates TekesUI color tokens and brand policy against BOTH readability floors
and glare ceilings for ordinary `light` and `dark` themes, while verifying that
`light-ic` and `dark-ic` retain uncompromised high-contrast guarantees.

Read-only tool: never mutates tokens, schemas, rules, or component sources.
"""
from __future__ import annotations

import argparse
import json
import math
from pathlib import Path
import re
import sys
from typing import Any

ROOT = Path(__file__).resolve().parents[4]
ORDINARY_MODES = ("light", "dark")
IC_MODES = ("light-ic", "dark-ic")
ALL_MODES = ORDINARY_MODES + IC_MODES
SURFACES = ("surface-0", "surface-1", "surface-2", "surface-3")
TRANSIENT_GROUNDS = (
    "button-hover",
    "button-pressed",
    "row-selection",
    "row-selection-hover",
    "diff-added-surface",
    "diff-removed-surface",
)
WARM_NEUTRAL_ROLES = (
    "surface-0",
    "surface-1",
    "primary-text",
    "secondary-text",
    "tertiary-text",
    "border",
    "separator",
)
STATUS_ROLES = (
    "status-info",
    "status-running",
    "status-succeeded",
    "status-failed",
    "warning-foreground",
)


def parse_hex(value: str) -> tuple[float, float, float]:
    if not isinstance(value, str) or not re.fullmatch(r"#[0-9A-Fa-f]{6}", value):
        raise ValueError(f"Expected opaque #RRGGBB hex color, got {value!r}")
    return tuple(int(value[i : i + 2], 16) / 255.0 for i in (1, 3, 5))


def parse_bytes(value: str) -> tuple[int, int, int]:
    if not isinstance(value, str) or not re.fullmatch(r"#[0-9A-Fa-f]{6}", value):
        raise ValueError(f"Expected opaque #RRGGBB hex color, got {value!r}")
    return (int(value[1:3], 16), int(value[3:5], 16), int(value[5:7], 16))


def linearize(channel: float) -> float:
    return channel / 12.92 if channel <= 0.04045 else ((channel + 0.055) / 1.055) ** 2.4


def relative_luminance(hex_color: str) -> float:
    r, g, b = (linearize(c) for c in parse_hex(hex_color))
    return 0.2126 * r + 0.7152 * g + 0.0722 * b


def contrast_ratio(a: str, b: str) -> float:
    lo, hi = sorted((relative_luminance(a), relative_luminance(b)))
    return (hi + 0.05) / (lo + 0.05)


def to_oklab(hex_color: str) -> tuple[float, float, float]:
    r, g, b = (linearize(c) for c in parse_hex(hex_color))
    l_ = (0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b) ** (1.0 / 3.0)
    m_ = (0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b) ** (1.0 / 3.0)
    s_ = (0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b) ** (1.0 / 3.0)
    return (
        0.2104542553 * l_ + 0.7936177850 * m_ - 0.0040720468 * s_,
        1.9779984951 * l_ - 2.4285922050 * m_ + 0.4505937099 * s_,
        0.0259040371 * l_ + 0.7827717662 * m_ - 0.8086757660 * s_,
    )


def to_oklch(hex_color: str) -> tuple[float, float, float]:
    l_val, a_val, b_val = to_oklab(hex_color)
    chroma = math.hypot(a_val, b_val)
    if chroma < 1e-5:
        return l_val, 0.0, 0.0
    return l_val, chroma, math.atan2(b_val, a_val)


def delta_ok(a: str, b: str) -> float:
    return math.sqrt(sum((x - y) ** 2 for x, y in zip(to_oklab(a), to_oklab(b))))


def resolve_token(color_tokens: dict[str, dict[str, Any]], name: str, mode: str, stack: tuple[str, ...] = ()) -> str:
    if name in stack:
        raise ValueError(f"Cyclic color token alias: {stack + (name,)}")
    if name not in color_tokens:
        raise KeyError(f"Missing color token: {name}")
    value = color_tokens[name]["value"]
    if isinstance(value, str) and value.startswith("{") and value.endswith("}"):
        return resolve_token(color_tokens, value[1:-1], mode, stack + (name,))
    if isinstance(value, dict):
        return str(value[mode])
    return str(value)


def _record(
    violations: list[dict[str, Any]],
    code: str,
    mode: str,
    subject: str,
    message: str,
    actual: Any,
    budget: str,
) -> None:
    violations.append(
        {
            "code": code,
            "mode": mode,
            "subject": subject,
            "message": message,
            "actual": actual,
            "budget": budget,
        }
    )


def audit(root: Path = ROOT) -> dict[str, Any]:
    base = json.loads((root / "design/source/base.tokens.json").read_text(encoding="utf-8"))
    policy = json.loads((root / "design/brand/policy.json").read_text(encoding="utf-8"))
    contrast_pairs = json.loads((root / "design/source/contrast-pairs.json").read_text(encoding="utf-8"))

    tokens = {entry["name"]: entry for entry in base["color"]["tokens"]}
    violations: list[dict[str, Any]] = []
    metrics: dict[str, Any] = {mode: {} for mode in ALL_MODES}

    def color(role: str, mode: str) -> str:
        return resolve_token(tokens, role, mode)

    # Verify localized contrast-pairs keeps IC untouched.
    for pair in contrast_pairs["pairs"]:
        if pair["kind"] == "text" and pair["ic"] < 7.0:
            _record(
                violations,
                "IC_THRESHOLD_WEAKENED",
                "ic",
                f"{pair['foreground']}/{pair['background']}",
                "Increased-contrast text threshold must remain >= 7.0",
                pair["ic"],
                ">= 7.0",
            )
        if pair["kind"] == "non-text" and pair["ic"] < 3.0:
            _record(
                violations,
                "IC_THRESHOLD_WEAKENED",
                "ic",
                f"{pair['foreground']}/{pair['background']}",
                "Increased-contrast non-text threshold must remain >= 3.0",
                pair["ic"],
                ">= 3.0",
            )

    for mode in ORDINARY_MODES:
        s1 = color("surface-1", mode)
        mode_metrics: dict[str, Any] = {}

        # 1. Ordinary primary reading window
        for surface in SURFACES:
            bg = color(surface, mode)
            ratio = contrast_ratio(color("primary-text", mode), bg)
            if ratio > 13.5:
                _record(
                    violations,
                    "GLARE_PRIMARY_TEXT",
                    mode,
                    f"primary-text/{surface}",
                    f"Primary text glare exceeds ceiling on {surface}",
                    round(ratio, 3),
                    "7.5 <= ratio <= 13.5",
                )
            elif ratio < 7.5:
                _record(
                    violations,
                    "LOW_PRIMARY_TEXT",
                    mode,
                    f"primary-text/{surface}",
                    f"Primary text contrast below calm reading floor on {surface}",
                    round(ratio, 3),
                    "7.5 <= ratio <= 13.5",
                )

        # 2. Ordinary secondary calm window
        for surface in SURFACES:
            bg = color(surface, mode)
            ratio = contrast_ratio(color("secondary-text", mode), bg)
            if ratio > 6.6:
                _record(
                    violations,
                    "OVER_CONTRAST_SECONDARY",
                    mode,
                    f"secondary-text/{surface}",
                    f"Secondary text contrast exceeds calm ceiling on {surface}",
                    round(ratio, 3),
                    "4.5 <= ratio <= 6.6",
                )
            elif ratio < 4.5:
                _record(
                    violations,
                    "LOW_SECONDARY_TEXT",
                    mode,
                    f"secondary-text/{surface}",
                    f"Secondary text contrast below 4.5 floor on {surface}",
                    round(ratio, 3),
                    "4.5 <= ratio <= 6.6",
                )

        # 3. Ordinary tertiary calm window (Plan B)
        for surface in SURFACES:
            bg = color(surface, mode)
            ratio = contrast_ratio(color("tertiary-text", mode), bg)
            if ratio > 4.8:
                _record(
                    violations,
                    "OVER_CONTRAST_TERTIARY",
                    mode,
                    f"tertiary-text/{surface}",
                    f"Tertiary text contrast exceeds calm ceiling on {surface}",
                    round(ratio, 3),
                    "3.5 <= ratio <= 4.8",
                )
            elif ratio < 3.5:
                _record(
                    violations,
                    "LOW_TERTIARY_TEXT",
                    mode,
                    f"tertiary-text/{surface}",
                    f"Tertiary text contrast below 3.5 Plan B floor on {surface}",
                    round(ratio, 3),
                    "3.5 <= ratio <= 4.8",
                )

        for ground_name in TRANSIENT_GROUNDS:
            bg = color(ground_name, mode)
            ratio = contrast_ratio(color("tertiary-text", mode), bg)
            if ratio < 3.0:
                _record(
                    violations,
                    "LOW_TERTIARY_TRANSIENT",
                    mode,
                    f"tertiary-text/{ground_name}",
                    f"Tertiary text below 3.0 on transient/diff surface {ground_name}",
                    round(ratio, 3),
                    ">= 3.0",
                )

        # 4. 4-tier typographic hierarchy & perceptual separation
        tiers = ["primary-text", "secondary-text", "tertiary-text", "disabled-text"]
        tier_ratios = {role: contrast_ratio(color(role, mode), s1) for role in tiers}
        tier_l = {role: to_oklch(color(role, mode))[0] for role in tiers}
        mode_metrics["text_ratios_on_s1"] = {k: round(v, 3) for k, v in tier_ratios.items()}
        mode_metrics["text_oklch_l"] = {k: round(v, 4) for k, v in tier_l.items()}

        if not (
            tier_ratios["primary-text"]
            > tier_ratios["secondary-text"]
            > tier_ratios["tertiary-text"]
            > tier_ratios["disabled-text"]
        ):
            _record(
                violations,
                "TYPOGRAPHY_ORDER",
                mode,
                "text-tiers",
                "Typographic contrast order must satisfy primary > secondary > tertiary > disabled",
                mode_metrics["text_ratios_on_s1"],
                "strictly descending",
            )

        for (upper, lower), min_delta in (
            (("primary-text", "secondary-text"), 0.12),
            (("secondary-text", "tertiary-text"), 0.06),
            (("tertiary-text", "disabled-text"), 0.06),
        ):
            diff = abs(tier_l[upper] - tier_l[lower])
            if diff < min_delta:
                _record(
                    violations,
                    "TYPOGRAPHY_COLLAPSE",
                    mode,
                    f"{upper}/{lower}",
                    f"OKLCH lightness step between {upper} and {lower} collapsed",
                    round(diff, 4),
                    f">= {min_delta}",
                )

        disabled_ratio = tier_ratios["disabled-text"]
        if disabled_ratio > 3.4:
            _record(
                violations,
                "OVER_CONTRAST_DISABLED",
                mode,
                "disabled-text/surface-1",
                "Disabled text looks overly active on surface-1",
                round(disabled_ratio, 3),
                "2.0 <= ratio <= 3.4",
            )
        elif disabled_ratio < 2.0:
            _record(
                violations,
                "LOW_DISABLED_TEXT",
                mode,
                "disabled-text/surface-1",
                "Disabled text contrast too faint on surface-1",
                round(disabled_ratio, 3),
                "2.0 <= ratio <= 3.4",
            )

        # 5. Warm calm surfaces (G1 & G2)
        if mode == "dark":
            for sname in ("surface-0", "code-surface"):
                l_val = to_oklch(color(sname, mode))[0]
                if l_val < 0.185:
                    _record(
                        violations,
                        "BLACK_HOLE_SURFACE",
                        mode,
                        sname,
                        f"Dark {sname} is too close to pure OLED black",
                        round(l_val, 4),
                        "L_oklch >= 0.185",
                    )

        for left, right in zip(SURFACES, SURFACES[1:]):
            step_ratio = contrast_ratio(color(left, mode), color(right, mode))
            if step_ratio > 1.12:
                _record(
                    violations,
                    "SURFACE_STEP_EXCESS",
                    mode,
                    f"{left}->{right}",
                    f"Adjacent surface tier contrast step exceeds micro-contrast budget",
                    round(step_ratio, 3),
                    "1.00 <= ratio <= 1.12",
                )

        for role in WARM_NEUTRAL_ROLES:
            r_b, _, b_b = parse_bytes(color(role, mode))
            if r_b < b_b:
                _record(
                    violations,
                    "COLD_NEUTRAL_DRIFT",
                    mode,
                    role,
                    f"Neutral role {role} drifted cool (R < B)",
                    color(role, mode),
                    "R >= B",
                )

        # 6. Quiet boundaries & focus restraint (G3)
        sep_ratio = contrast_ratio(color("separator", mode), s1)
        if sep_ratio > 1.35:
            _record(
                violations,
                "NOISY_SEPARATOR",
                mode,
                "separator/surface-1",
                "Decorative separator is too prominent",
                round(sep_ratio, 3),
                "1.08 <= ratio <= 1.35",
            )
        elif sep_ratio < 1.08:
            _record(
                violations,
                "INVISIBLE_SEPARATOR",
                mode,
                "separator/surface-1",
                "Decorative separator vanished below 1.08:1",
                round(sep_ratio, 3),
                "1.08 <= ratio <= 1.35",
            )

        border_hex = color("border", mode)
        border_s1 = contrast_ratio(border_hex, s1)
        if border_s1 > 4.0:
            _record(
                violations,
                "HARSH_BORDER",
                mode,
                "border/surface-1",
                "Required control border exceeds calm ceiling on surface-1",
                round(border_s1, 3),
                "3.0 <= ratio <= 4.0",
            )
        elif border_s1 < 3.0:
            _record(
                violations,
                "LOW_BORDER",
                mode,
                "border/surface-1",
                "Required control border below 3.0:1 on surface-1",
                round(border_s1, 3),
                "3.0 <= ratio <= 4.0",
            )

        border_grounds = list(policy["control_backgrounds"]) + ["row-selection-hover", "segmented-thumb"]
        for bg_name in border_grounds:
            ratio = contrast_ratio(border_hex, color(bg_name, mode))
            if ratio < 3.0:
                _record(
                    violations,
                    "LOW_BORDER",
                    mode,
                    f"border/{bg_name}",
                    f"Required border below 3.0:1 on {bg_name}",
                    round(ratio, 3),
                    ">= 3.0",
                )

        focus_hex = color("border-focused", mode)
        focus_s1 = contrast_ratio(focus_hex, s1)
        if focus_s1 > 4.4:
            _record(
                violations,
                "HARSH_FOCUS",
                mode,
                "border-focused/surface-1",
                "Focus ring contrast exceeds calm ceiling on surface-1",
                round(focus_s1, 3),
                "3.2 <= ratio <= 4.4",
            )
        elif focus_s1 < 3.2:
            _record(
                violations,
                "LOW_FOCUS",
                mode,
                "border-focused/surface-1",
                "Focus ring contrast below 3.2:1 on surface-1",
                round(focus_s1, 3),
                "3.2 <= ratio <= 4.4",
            )

        focus_delta = delta_ok(border_hex, focus_hex)
        if focus_delta < 0.012:
            _record(
                violations,
                "COLLAPSED_FOCUS",
                mode,
                "border-focused/border",
                "Focus color collapsed onto resting border color",
                round(focus_delta, 4),
                ">= 0.012",
            )

        mode_metrics["boundaries_on_s1"] = {
            "separator": round(sep_ratio, 3),
            "border": round(border_s1, 3),
            "border-focused": round(focus_s1, 3),
            "focus_delta_ok": round(focus_delta, 4),
        }

        # 7. Calm status & brand policy ceilings (G1)
        status_metrics: dict[str, Any] = {}
        for role in STATUS_ROLES:
            hex_val = color(role, mode)
            ratio = contrast_ratio(hex_val, s1)
            _, chroma, _ = to_oklch(hex_val)
            status_metrics[role] = {"ratio_s1": round(ratio, 3), "chroma": round(chroma, 4)}
            if ratio > 8.0:
                _record(
                    violations,
                    "OVEREXPOSED_STATUS",
                    mode,
                    f"{role}/surface-1",
                    f"Status ink {role} is overly bright/harsh on surface-1",
                    round(ratio, 3),
                    "4.5 <= ratio <= 8.0",
                )
            elif ratio < 4.5:
                _record(
                    violations,
                    "LOW_STATUS_CONTRAST",
                    mode,
                    f"{role}/surface-1",
                    f"Status ink {role} below 4.5:1 on surface-1",
                    round(ratio, 3),
                    "4.5 <= ratio <= 8.0",
                )
            if chroma > 0.15:
                _record(
                    violations,
                    "HIGH_CHROMA_STATUS",
                    mode,
                    role,
                    f"Status ink {role} chroma exceeds calm saturation ceiling",
                    round(chroma, 4),
                    "C_oklch <= 0.15",
                )
        mode_metrics["status"] = status_metrics
        metrics[mode] = mode_metrics

    # Brand policy ordinary target checks
    dark_t = policy["targets"]["dark"]
    light_t = policy["targets"]["light"]
    for field, limit in (("link", 0.76), ("outline", 0.72), ("subtle_text", 0.80)):
        if dark_t[field] > limit:
            _record(
                violations,
                "HARSH_BRAND_POLICY",
                "dark",
                f"policy.targets.dark.{field}",
                f"Dark brand target {field} is overly bright",
                dark_t[field],
                f"<= {limit}",
            )
    for field, floor in (("action", 0.50), ("link", 0.44)):
        if light_t[field] < floor:
            _record(
                violations,
                "HARSH_BRAND_POLICY",
                "light",
                f"policy.targets.light.{field}",
                f"Light brand target {field} is overly dark/harsh",
                light_t[field],
                f">= {floor}",
            )

    # 8. Increased-contrast uncompromised guarantees
    for ic_mode, ord_mode in zip(IC_MODES, ORDINARY_MODES):
        ic_s1 = color("surface-1", ic_mode)
        ord_s1 = color("surface-1", ord_mode)
        for role in ("primary-text", "secondary-text", "tertiary-text"):
            ic_r = contrast_ratio(color(role, ic_mode), ic_s1)
            ord_r = contrast_ratio(color(role, ord_mode), ord_s1)
            for surface in SURFACES:
                sr = contrast_ratio(color(role, ic_mode), color(surface, ic_mode))
                if sr < 7.0:
                    _record(
                        violations,
                        "IC_TEXT_FLOOR",
                        ic_mode,
                        f"{role}/{surface}",
                        f"Increased-contrast {role} dropped below 7.0:1 on {surface}",
                        round(sr, 3),
                        ">= 7.0",
                    )
            if ic_r <= ord_r:
                _record(
                    violations,
                    "IC_MONOTONICITY",
                    ic_mode,
                    role,
                    f"Increased-contrast {role} is not strictly stronger than ordinary {ord_mode}",
                    {"ic": round(ic_r, 3), "ordinary": round(ord_r, 3)},
                    "ic > ordinary",
                )
        for role in ("border", "border-focused"):
            ic_r = contrast_ratio(color(role, ic_mode), ic_s1)
            ord_r = contrast_ratio(color(role, ord_mode), ord_s1)
            for surface in SURFACES:
                sr = contrast_ratio(color(role, ic_mode), color(surface, ic_mode))
                if sr < 3.0:
                    _record(
                        violations,
                        "IC_BOUNDARY_FLOOR",
                        ic_mode,
                        f"{role}/{surface}",
                        f"Increased-contrast {role} dropped below 3.0:1 on {surface}",
                        round(sr, 3),
                        ">= 3.0",
                    )
            if ic_r <= ord_r:
                _record(
                    violations,
                    "IC_MONOTONICITY",
                    ic_mode,
                    role,
                    f"Increased-contrast {role} is not strictly stronger than ordinary {ord_mode}",
                    {"ic": round(ic_r, 3), "ordinary": round(ord_r, 3)},
                    "ic > ordinary",
                )
        metrics[ic_mode] = {
            "primary_s1": round(contrast_ratio(color("primary-text", ic_mode), ic_s1), 3),
            "secondary_s1": round(contrast_ratio(color("secondary-text", ic_mode), ic_s1), 3),
            "tertiary_s1": round(contrast_ratio(color("tertiary-text", ic_mode), ic_s1), 3),
            "border_s1": round(contrast_ratio(color("border", ic_mode), ic_s1), 3),
        }

    return {
        "format": "tekesui-gold7-calm-audit/1",
        "status": "fail" if violations else "pass",
        "violation_count": len(violations),
        "violations": violations,
        "metrics": metrics,
    }


EXEMPT_INPUT_PREFIXES = ("input", "check-", "toggle-", "range-", "selection", "action-primary")
ALLOWED_NON_INPUT_BORDER_PROFILES: frozenset[str] = frozenset()
ALLOWED_NON_INPUT_SECONDARY_TEXT_PROFILES: frozenset[str] = frozenset(
    {"quiet", "data-category-label"}
)
ALLOWED_RUST_CONTROL_FILES: frozenset[str] = frozenset()


def scan_components(root: Path = ROOT) -> dict[str, Any]:
    """Read-only regression gate scanning theme rules & Rust controls for chrome noise."""
    rules = json.loads((root / "design/theme/rules.json").read_text(encoding="utf-8"))
    advisory_profiles: list[dict[str, Any]] = []
    violations: list[dict[str, Any]] = []

    for profile_name, body in sorted(rules.get("profiles", {}).items()):
        if profile_name.startswith(EXEMPT_INPUT_PREFIXES):
            continue
        # Resolve direct dictionary fields (including overrides) for gate check.
        serialized = json.dumps(body)
        uses_border = '"border"' in serialized
        uses_secondary = '"secondary-text"' in serialized
        if uses_border or uses_secondary:
            advisory_profiles.append(
                {
                    "profile": profile_name,
                    "uses_border": uses_border,
                    "uses_secondary_text": uses_secondary,
                }
            )
            if uses_border and profile_name not in ALLOWED_NON_INPUT_BORDER_PROFILES:
                violations.append(
                    {
                        "code": "NON_INPUT_PROFILE_BORDER",
                        "subject": f"profile:{profile_name}",
                        "message": "Non-input profile references high-contrast 'border' role instead of 'separator'",
                    }
                )
            if uses_secondary and profile_name not in ALLOWED_NON_INPUT_SECONDARY_TEXT_PROFILES:
                violations.append(
                    {
                        "code": "NON_INPUT_PROFILE_SECONDARY_TEXT",
                        "subject": f"profile:{profile_name}",
                        "message": "Non-input profile references 'secondary-text' outside explicit allowlist",
                    }
                )

    rust_hits: list[dict[str, Any]] = []
    controls_src = root / "crates/tekes-ui-controls/src"
    if controls_src.is_dir():
        for path in sorted(controls_src.rglob("*.rs")):
            text = path.read_text(encoding="utf-8")
            border_refs = len(re.findall(r'"border"', text))
            secondary_refs = len(re.findall(r'"secondary-text"', text))
            if border_refs or secondary_refs:
                rel = path.relative_to(root).as_posix()
                rust_hits.append(
                    {
                        "file": rel,
                        "border_refs": border_refs,
                        "secondary_text_refs": secondary_refs,
                    }
                )
                if rel not in ALLOWED_RUST_CONTROL_FILES:
                    violations.append(
                        {
                            "code": "CONTROLS_RAW_PALETTE_BYPASS",
                            "subject": rel,
                            "message": (
                                f"Direct palette string lookup (border={border_refs}, "
                                f"secondary-text={secondary_refs}) bypasses shared Surface contract"
                            ),
                        }
                    )

    return {
        "mode": "regression-gate",
        "status": "fail" if violations else "pass",
        "violation_count": len(violations),
        "violations": violations,
        "allowlisted_profiles": {
            "border": sorted(ALLOWED_NON_INPUT_BORDER_PROFILES),
            "secondary_text": sorted(ALLOWED_NON_INPUT_SECONDARY_TEXT_PROFILES),
        },
        "allowlisted_control_files": sorted(ALLOWED_RUST_CONTROL_FILES),
        "non_input_profiles_with_border_or_secondary": advisory_profiles,
        "rust_control_files_with_border_or_secondary": rust_hits,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=ROOT, help="TekesUI repository root")
    parser.add_argument("--check", action="store_true", help="Exit 1 if any band-pass or component violation is found")
    parser.add_argument("--json", action="store_true", help="Emit JSON output")
    parser.add_argument(
        "--scan-components",
        action="store_true",
        help="Read-only regression gate of rules.json and controls for chrome noise / Surface bypasses",
    )
    args = parser.parse_args()
    root = args.root.resolve()

    if args.scan_components:
        scan = scan_components(root)
        if args.json:
            print(json.dumps(scan, ensure_ascii=False, indent=2))
        else:
            if scan["violations"]:
                for v in scan["violations"]:
                    print(
                        f"FAIL [{v['code']}] {v['subject']}: {v['message']}",
                        file=sys.stderr,
                    )
            else:
                print(
                    "PASS: Component Surface contract & non-input profile gate "
                    f"(allowlisted secondary-text profiles={scan['allowlisted_profiles']['secondary_text']}, raw control bypasses=0)."
                )
        return 1 if scan["violations"] else 0

    report = audit(root)
    if args.json:
        print(json.dumps(report, ensure_ascii=False, indent=2))
    else:
        if report["violations"]:
            for v in report["violations"]:
                print(
                    f"FAIL [{v['code']}] {v['mode']} {v['subject']}: {v['message']} "
                    f"(actual={v['actual']}, budget={v['budget']})",
                    file=sys.stderr,
                )
        else:
            print("PASS: Gold7 calm-UI band-pass contrast, OKLCH hierarchy, warmth, and IC checks.")
            for mode in ORDINARY_MODES:
                m = report["metrics"][mode]
                print(
                    f"  {mode:5s} text={m['text_ratios_on_s1']} L={m['text_oklch_l']} bounds={m['boundaries_on_s1']}"
                )
    if args.check and report["violations"]:
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
