#!/usr/bin/env python3
"""Check Gold7's authoritative references and mechanically testable rules.

This is a source-integrity check only. It never marks code integration or user
experience as passed; those assessments require separate review evidence.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import subprocess
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
GOLD7 = "design/behavior/gold7.json"
RESULTS = {"pass", "fail", "unverified", "not-applicable"}
LEVELS = {"contract", "code", "experience"}
ITEM_IDS = [f"G{i}" for i in range(1, 8)]
REQUIRED_REQUIREMENTS = {
    "G1": {"warm-neutral-reading-foundation", "consumer-brand-derivation", "whole-page-reading-composition"},
    "G2": {"four-semantic-surface-roles", "same-color-geometric-distinction"},
    "G3": {"hairline-and-required-boundary-roles", "component-role-radius-map", "nested-radius-geometry", "real-scale-rendering"},
    "G4": {"named-press-expansion-snap-presets", "real-component-motion-integration", "direct-manipulation-and-interruption", "reduced-motion-path"},
    "G5": {"ime-and-suggestion-priority", "durable-draft-recovery", "manual-scroll-reading-anchor", "explicit-return-to-latest"},
    "G6": {"zoom-preserves-reading-anchor", "bounded-split-and-pan-geometry", "cancel-and-non-gesture-alternatives"},
    "G7": {"reusable-real-component-contract", "component-documentation-and-runnable-preview", "business-combinations-are-real"},
}
MECHANICAL_CHECK_IDS = {
    "authoritative-reference-integrity", "gold7-requirement-catalog", "semantic-surface-order",
    "boundary-role-separation", "radius-contract", "interaction-contract-invariants",
    "consumer-brand-registry", "pass-evidence-integrity",
}


def _pointer(value: Any, fragment: str) -> Any:
    if not fragment:
        return value
    if not fragment.startswith("/"):
        raise ValueError(f"unsupported JSON fragment #{fragment}")
    node = value
    for encoded in fragment[1:].split("/"):
        part = encoded.replace("~1", "/").replace("~0", "~")
        if isinstance(node, list):
            node = node[int(part)]
        else:
            node = node[part]
    return node


def _verify_pass_evidence(root: Path, entry: Any, resolve: Any) -> None:
    """Bind a pass claim to exact artifact/input bytes under one explicit identity policy."""
    if not isinstance(entry, dict):
        raise ValueError("each evidence entry must be an object")
    artifact_ref = entry.get("artifact", {}).get("$ref") if isinstance(entry.get("artifact"), dict) else None
    identity = entry.get("identity")
    method = entry.get("method")
    if not artifact_ref or not isinstance(identity, dict) or method not in {
        "automated-test", "native-capture", "human-review", "accessibility-review"
    }:
        raise ValueError("expected artifact $ref, explicit identity policy, and a supported review method")
    artifact_path, artifact_value = resolve(GOLD7, artifact_ref)
    manifest_block = identity.get("manifest")
    manifest_ref = manifest_block.get("$ref") if isinstance(manifest_block, dict) else None
    if not manifest_ref:
        raise ValueError("identity must reference a frozen input manifest")
    manifest_path, manifest_value = resolve(GOLD7, manifest_ref)
    if not artifact_path.is_file() or not manifest_path.is_file() or artifact_value is not None or manifest_value is not None:
        raise ValueError("evidence and manifest refs must target existing whole files")

    def repo_relative(path: Path) -> str:
        try:
            return path.resolve().relative_to(root.resolve()).as_posix()
        except ValueError as error:
            raise ValueError("evidence path escapes the repository") from error

    manifest_relative = repo_relative(manifest_path)
    manifest_bytes = manifest_path.read_bytes()
    kind = identity.get("kind")
    commit: str | None = None
    if kind == "frozen_worktree_manifest":
        expected_manifest_digest = manifest_block.get("sha256")
        if not isinstance(expected_manifest_digest, str) or len(expected_manifest_digest) != 64:
            raise ValueError("frozen worktree manifest reference needs its SHA-256 digest")
        if hashlib.sha256(manifest_bytes).hexdigest() != expected_manifest_digest.lower():
            raise ValueError("frozen worktree manifest digest is stale or tampered")
    elif kind == "immutable_git_commit":
        revision = identity.get("commit")
        if not isinstance(revision, str) or len(revision) != 40 or any(char not in "0123456789abcdef" for char in revision.lower()):
            raise ValueError("immutable Git identity needs a full 40-character commit hash")

        def git(*args: str) -> bytes:
            result = subprocess.run(
                ["git", "-C", str(root), *args],
                check=False,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            )
            if result.returncode:
                raise ValueError(f"Git verification failed for {' '.join(args)}")
            return result.stdout

        commit = git("rev-parse", "--verify", f"{revision}^{{commit}}").decode().strip()
        if commit != revision.lower():
            raise ValueError("identity revision is not the exact immutable Git commit")

        def committed_bytes(relative: str) -> bytes:
            return git("show", f"{commit}:{relative}")

        if manifest_bytes != committed_bytes(manifest_relative):
            raise ValueError("input manifest working copy differs from the cited immutable commit")
    else:
        raise ValueError("identity kind must be immutable_git_commit or frozen_worktree_manifest")

    manifest = json.loads(manifest_bytes.decode("utf-8"))
    if manifest.get("format") != "tekesui-gold7-evidence-inputs/1":
        raise ValueError("unsupported evidence input manifest format")
    inputs = manifest.get("inputs")
    if not isinstance(inputs, list) or not inputs:
        raise ValueError("input manifest must contain at least one hashed input")
    normalized: dict[str, str] = {}
    for item in inputs:
        if not isinstance(item, dict) or not isinstance(item.get("path"), str) or not isinstance(item.get("sha256"), str):
            raise ValueError("every manifest input needs path and sha256")
        declared_path = Path(item["path"])
        if declared_path.is_absolute():
            raise ValueError("manifest input paths must be repository-relative")
        path = (root / declared_path).resolve()
        relative = repo_relative(path)
        if relative in normalized:
            raise ValueError(f"duplicate manifest input path: {relative}")
        if not path.is_file():
            raise ValueError(f"manifest input does not exist as a file: {relative}")
        if len(item["sha256"]) != 64:
            raise ValueError(f"invalid SHA-256 for manifest input {relative}")
        current = path.read_bytes()
        expected = item["sha256"].lower()
        if hashlib.sha256(current).hexdigest() != expected:
            raise ValueError(f"input bytes do not match manifest hash: {relative}")
        if commit is not None:
            committed = committed_bytes(relative)
            if hashlib.sha256(committed).hexdigest() != expected:
                raise ValueError(f"input commit bytes do not match manifest hash: {relative}")
        normalized[relative] = expected
    artifact_relative = repo_relative(artifact_path)
    if artifact_relative not in normalized:
        raise ValueError("manifest must include the evidence artifact itself among hashed inputs")
    if len(normalized) < 2:
        raise ValueError("manifest must bind the evidence artifact to at least one distinct source input")
    if commit is not None and artifact_path.read_bytes() != committed_bytes(artifact_relative):
        raise ValueError("evidence artifact working copy differs from the cited immutable commit")


def check(root: Path = ROOT, overrides: dict[str, Any] | None = None) -> list[str]:
    overrides = overrides or {}
    errors: list[str] = []

    def load(relative: str) -> Any:
        normalized = Path(relative).as_posix()
        if normalized in overrides:
            return overrides[normalized]
        return json.loads((root / normalized).read_text(encoding="utf-8"))

    def resolve(source_relative: str, reference: str) -> tuple[Path, Any | None]:
        if reference.startswith(("https://", "http://", "urn:")):
            return Path(""), None
        path_text, sep, fragment = reference.partition("#")
        source_path = root / source_relative
        target = (source_path.parent / path_text).resolve() if path_text else source_path.resolve()
        try:
            target.relative_to(root.resolve())
        except ValueError as error:
            raise ValueError("reference escapes repository") from error
        if not target.exists():
            raise ValueError(f"missing reference target: {reference}")
        if not sep or not fragment:
            return target, None
        if target.suffix.lower() not in {".json", ".schema"}:
            raise ValueError(f"JSON pointer targets a non-JSON file: {reference}")
        target_relative = target.relative_to(root.resolve()).as_posix()
        value = load(target_relative)
        try:
            return target, _pointer(value, fragment)
        except (KeyError, IndexError, ValueError) as error:
            raise ValueError(f"missing or invalid JSON pointer in reference: {reference}") from error

    def inspect_refs(value: Any, source_relative: str, location: str) -> None:
        if isinstance(value, dict):
            reference = value.get("$ref")
            if "$ref" in value and not isinstance(reference, str):
                errors.append(f"{location}: $ref must be a string")
            elif isinstance(reference, str):
                try:
                    resolve(source_relative, reference)
                except (OSError, ValueError, KeyError, IndexError, json.JSONDecodeError) as error:
                    errors.append(f"{location}: {error}")
            for key, item in value.items():
                inspect_refs(item, source_relative, f"{location}/{key}")
        elif isinstance(value, list):
            for index, item in enumerate(value):
                inspect_refs(item, source_relative, f"{location}/{index}")

    try:
        system = load("design/system.json")
        schema = load("design/system.schema.json")
        integration = load("design/integration.json")
        gold7 = load(GOLD7)
        layout = load("design/source/layout.json")
        interaction = load("design/source/interaction-contracts.json")
        motion = load("design/source/motion.json")
        base = load("design/source/base.tokens.json")
        brand_config = load("design/brand/config.json")
        brand_policy = load("design/brand/policy.json")
        brand_gallery = load("design/brand/gallery.json")
    except (OSError, json.JSONDecodeError) as error:
        return [f"cannot load Gold7 input: {error}"]

    inspect_refs(system, "design/system.json", "system")
    inspect_refs(integration, "design/integration.json", "integration")
    inspect_refs(gold7, GOLD7, "gold7")

    expected_effective = {
        "design/system.json#/layers/interaction": system.get("layers", {}).get("interaction", {}).get("$ref"),
        "design/integration.json#/effective_specifications/layout": integration.get("effective_specifications", {}).get("layout", {}).get("$ref"),
        "design/integration.json#/effective_specifications/interaction": integration.get("effective_specifications", {}).get("interaction", {}).get("$ref"),
        "design/integration.json#/effective_specifications/motion": integration.get("effective_specifications", {}).get("motion", {}).get("$ref"),
        "design/system.json#/interaction/proposal": system.get("interaction", {}).get("proposal", {}).get("$ref"),
        "design/system.json#/interaction/motion/source": system.get("interaction", {}).get("motion", {}).get("source", {}).get("$ref"),
        "design/system.json#/specs/calibration_layout/proposal": system.get("specs", {}).get("calibration_layout", {}).get("proposal", {}).get("$ref"),
    }
    expected_values = {
        "design/system.json#/layers/interaction": "./source/interaction-contracts.json",
        "design/integration.json#/effective_specifications/layout": "./source/layout.json",
        "design/integration.json#/effective_specifications/interaction": "./source/interaction-contracts.json",
        "design/integration.json#/effective_specifications/motion": "./source/motion.json",
        "design/system.json#/interaction/proposal": "./source/interaction-contracts.json",
        "design/system.json#/interaction/motion/source": "./source/motion.json",
        "design/system.json#/specs/calibration_layout/proposal": "./source/layout.json",
    }
    for key, expected in expected_values.items():
        if expected_effective[key] != expected:
            errors.append(f"{key}: expected effective local source {expected}")

    package_refs = {
        "design/integration.json#/effective_specifications/layout": integration.get("effective_specifications", {}).get("layout", {}).get("package_source", {}).get("$ref"),
        "design/integration.json#/effective_specifications/interaction": integration.get("effective_specifications", {}).get("interaction", {}).get("package_source", {}).get("$ref"),
        "design/integration.json#/effective_specifications/motion": integration.get("effective_specifications", {}).get("motion", {}).get("package_source", {}).get("$ref"),
        "design/system.json#/specs/calibration_layout/package_source": system.get("specs", {}).get("calibration_layout", {}).get("package_source", {}).get("$ref"),
        "design/system.json#/interaction/package_source": system.get("interaction", {}).get("package_source", {}).get("$ref"),
        "design/system.json#/interaction/motion/package_source": system.get("interaction", {}).get("motion", {}).get("package_source", {}).get("$ref"),
    }
    package_reference_values = {
        "design/integration.json#/effective_specifications/layout": "./imports/v0.3.0-proposal1/upstream/TekesUI-Design-System-v0.3.0-proposal1/design/layout.json",
        "design/integration.json#/effective_specifications/interaction": "./imports/v0.3.0-proposal1/upstream/TekesUI-Design-System-v0.3.0-proposal1/design/interaction-contracts.json",
        "design/integration.json#/effective_specifications/motion": "./imports/v0.3.0-proposal1/upstream/TekesUI-Design-System-v0.3.0-proposal1/design/tokens/motion.toml",
        "design/system.json#/specs/calibration_layout/package_source": "./imports/v0.3.0-proposal1/upstream/TekesUI-Design-System-v0.3.0-proposal1/design/layout.json",
        "design/system.json#/interaction/package_source": "./imports/v0.3.0-proposal1/upstream/TekesUI-Design-System-v0.3.0-proposal1/design/interaction-contracts.json",
        "design/system.json#/interaction/motion/package_source": "./imports/v0.3.0-proposal1/upstream/TekesUI-Design-System-v0.3.0-proposal1/design/tokens/motion.toml",
    }
    for key, expected in package_reference_values.items():
        if package_refs[key] != expected:
            errors.append(f"{key}: must retain original package source {expected}")

    authority = system.get("authority", {})
    authority = authority if isinstance(authority, dict) else {}
    gold7_authority = authority.get("gold7", {})
    gold7_authority = gold7_authority if isinstance(gold7_authority, dict) else {}
    if gold7_authority.get("$ref") != "./behavior/gold7.json":
        errors.append("system authority must reference behavior/gold7.json")
    if "gold7" not in schema.get("properties", {}).get("authority", {}).get("required", []):
        errors.append("system schema authority.required must include gold7")

    if gold7.get("format") != "tekesui-gold7/1":
        errors.append("Gold7 format must be tekesui-gold7/1")
    items = gold7.get("items")
    if not isinstance(items, list) or [item.get("id") for item in items] != ITEM_IDS:
        errors.append("Gold7 items must be the ordered, unique set G1 through G7")
        items = items if isinstance(items, list) else []
    policy = gold7.get("assessment_policy", {})
    policy = policy if isinstance(policy, dict) else {}
    levels = policy.get("levels", [])
    results = policy.get("results", [])
    if not isinstance(levels, list) or not isinstance(results, list) or set(levels) != LEVELS or set(results) != RESULTS:
        errors.append("Gold7 assessment levels/results do not match the required vocabulary")
    if not isinstance(policy.get("static_checker_limit"), str) or not all(
        phrase in policy["static_checker_limit"] for phrase in ("design-contract approval", "production code integration", "actual experience")
    ):
        errors.append("static checker limit must explicitly disclaim contract, code, and experience acceptance")
    valid_items = [item for item in items if isinstance(item, dict)]
    if len(valid_items) != len(items):
        errors.append("Gold7 items must contain objects")
    items = valid_items
    for item in items:
        item_id = item.get("id")
        assessment = item.get("assessment", {})
        assessment = assessment if isinstance(assessment, dict) else {}
        if set(assessment) != LEVELS:
            errors.append(f"{item.get('id')}: assessment must separately cover contract, code, experience")
            continue
        for level, result in assessment.items():
            if not isinstance(result, dict):
                errors.append(f"{item.get('id')}/{level}: assessment entry must be an object")
                continue
            status = result.get("status")
            evidence = result.get("evidence_refs", [])
            if status not in RESULTS:
                errors.append(f"{item.get('id')}/{level}: invalid assessment status {status!r}")
            if status == "not-applicable" and not result.get("reason"):
                errors.append(f"{item.get('id')}/{level}: not-applicable requires a reason")
            if status == "pass":
                if not isinstance(evidence, list) or not evidence:
                    errors.append(f"{item.get('id')}/{level}: pass requires verifiable evidence artifacts and an immutable input manifest")
                else:
                    for evidence_entry in evidence:
                        try:
                            _verify_pass_evidence(root, evidence_entry, resolve)
                        except (OSError, ValueError, KeyError, TypeError, json.JSONDecodeError) as error:
                            errors.append(f"{item.get('id')}/{level}: invalid pass evidence: {error}")
        requirements = item.get("requirements", [])
        if not isinstance(requirements, list) or not requirements:
            errors.append(f"{item.get('id')}: at least one sourced requirement is required")
            requirements = []
        requirement_ids = {
            requirement.get("id") for requirement in requirements
            if isinstance(requirement, dict) and isinstance(requirement.get("id"), str)
        }
        missing_requirements = REQUIRED_REQUIREMENTS.get(item_id, set()) - requirement_ids
        if missing_requirements:
            errors.append(f"{item_id}: missing core requirements {', '.join(sorted(missing_requirements))}")
        for requirement in requirements:
            if not isinstance(requirement, dict):
                errors.append(f"{item.get('id')}: requirement entry must be an object")
                continue
            if not requirement.get("id") or not requirement.get("rule"):
                errors.append(f"{item.get('id')}: requirement needs id and rule")
            if not requirement.get("sources"):
                errors.append(f"{item.get('id')}/{requirement.get('id')}: source references are required")

    # The work queue must not silently omit a Gold7 requirement or invent a
    # completed experience status from implementation progress.
    try:
        worklist = load("design/behavior/gold7-alignment.json")
        rows = worklist.get("requirements", [])
        expected_ids = {f"{item['id']}/{requirement['id']}"
                        for item in items for requirement in item.get("requirements", [])}
        actual_ids = [row.get("id") for row in rows]
        if set(actual_ids) != expected_ids or len(actual_ids) != len(expected_ids):
            errors.append("alignment worklist must cover each Gold7 requirement exactly once")
        for row in rows:
            if row.get("implementation") not in {"missing", "partial", "implemented-unverified"}:
                errors.append("alignment worklist needs an explicit implementation-gap status")
            if row.get("experience") != "unverified":
                errors.append("alignment worklist cannot promote experience without Gold7 evidence")
            if not row.get("observation") or not row.get("next_action"):
                errors.append("alignment worklist requires an observation and next action")
    except (OSError, ValueError, KeyError, TypeError) as error:
        errors.append(f"alignment worklist unavailable: {error}")

    checks = gold7.get("mechanical_checks")
    check_ids = [entry.get("id") for entry in checks if isinstance(entry, dict)] if isinstance(checks, list) else []
    if set(check_ids) != MECHANICAL_CHECK_IDS or len(check_ids) != len(MECHANICAL_CHECK_IDS):
        errors.append("mechanical_checks must enumerate each unique executable source-integrity check")
    reviews = gold7.get("review_assessments", {})
    if not isinstance(reviews, dict) or reviews.get("dimensions") != ["contract", "code", "experience"]:
        errors.append("review_assessments must remain a separate contract/code/experience assessment section")
    if any(
        isinstance(item.get("assessment"), dict)
        and isinstance(item["assessment"].get("experience"), dict)
        and item["assessment"]["experience"].get("status") == "pass"
        for item in items
    ):
        if policy.get("static_checker_limit") is None:
            errors.append("static checker cannot imply actual experience acceptance")

    media_contract = None
    try:
        _, media_contract = resolve(GOLD7, "./reference-alignment.json#/media_viewer")
    except (OSError, ValueError, KeyError, IndexError) as error:
        errors.append(f"G6 media runtime gap reference is unavailable: {error}")
    g6 = next((item for item in items if item.get("id") == "G6"), {})
    g6_code = g6.get("assessment", {}).get("code", {}) if isinstance(g6.get("assessment"), dict) else {}
    if isinstance(media_contract, dict) and media_contract.get("status") == "specified-not-implemented":
        reference_alignment = load("design/behavior/reference-alignment.json")
        media_recipe = next((entry for entry in reference_alignment.get("components", []) if entry.get("id") == "media"), {})
        if media_recipe.get("implementation") != "missing-runtime":
            errors.append("G6 reference must expose the media viewer's missing runtime implementation")
        if g6_code.get("status") != "fail":
            errors.append("G6 code assessment must mark the explicitly missing media runtime as fail, not unverified")
        g6_experience = g6.get("assessment", {}).get("experience", {})
        if g6_experience.get("status") != "unverified":
            errors.append("G6 media experience must remain unverified until runtime experience is evaluated")
    if g6_code.get("status") == "pass":
        errors.append("G6 cannot pass code integration while the referenced media runtime is specified-not-implemented")

    color_tokens = _token_map(base.get("color", {}).get("tokens", []))
    radius_tokens = _token_map(base.get("radius", {}).get("tokens", []))
    stroke_tokens = _token_map(base.get("stroke", {}).get("tokens", []))
    color_group = base.get("color", {})
    color_group = color_group if isinstance(color_group, dict) else {}
    themes = color_group.get("themes", [])
    themes = set(themes) if isinstance(themes, list) else set()
    required_themes = {"light", "dark", "light-ic", "dark-ic"}
    if themes != required_themes:
        errors.append("surface role validation requires the four declared light/dark contrast themes")
    for tier in range(4):
        semantic = color_tokens.get(f"surface-{tier}", {}).get("value")
        primitive = color_tokens.get(f"p-surface-{tier}", {}).get("value")
        if semantic != f"{{p-surface-{tier}}}":
            errors.append(f"surface-{tier}: semantic role must alias p-surface-{tier}")
        if not isinstance(primitive, dict) or set(primitive) != required_themes:
            errors.append(f"p-surface-{tier}: explicit values for all four themes are required")
    for theme in sorted(required_themes):
        luminances = []
        colors = []
        for tier in range(4):
            primitive_value = color_tokens.get(f"p-surface-{tier}", {}).get("value")
            value = primitive_value.get(theme) if isinstance(primitive_value, dict) else None
            try:
                luminances.append(_relative_luminance(value))
                colors.append(value)
            except (TypeError, ValueError):
                errors.append(f"p-surface-{tier}/{theme}: expected a valid opaque #RRGGBB value")
                luminances.append(0.0)
                colors.append(None)
        if theme.startswith("dark"):
            ordered = all(left <= right for left, right in zip(luminances, luminances[1:]))
        else:
            ordered = all(left <= right for left, right in zip(luminances, luminances[1:]))
        if not ordered:
            errors.append(f"{theme}: surface role luminance order must not reverse from surface-0 to surface-3")
        if any(left == right for left, right in zip(colors, colors[1:])):
            g2 = next((item for item in items if item.get("id") == "G2"), {})
            g2_requirement = next(
                (entry for entry in g2.get("requirements", []) if isinstance(entry, dict) and entry.get("geometry_sources")),
                {},
            )
            geometry_refs = [
                source.get("$ref", "")
                for source in g2_requirement.get("geometry_sources", [])
            ]
            if not any(reference.endswith(".rs") for reference in geometry_refs):
                errors.append(f"{theme}: coincident surface colors require geometry implementation references")

    boundary = layout.get("boundary_styles", {})
    required_boundary = boundary.get("required_control", {})
    focus = boundary.get("focus", {})
    decorative = boundary.get("decorative", {})
    if required_boundary.get("color") not in color_tokens or required_boundary.get("stroke") not in stroke_tokens:
        errors.append("required control boundary must name defined semantic color and stroke roles")
    if focus.get("color") not in color_tokens or focus.get("stroke") not in stroke_tokens:
        errors.append("focus boundary must name defined semantic color and stroke roles")
    if decorative.get("color") not in color_tokens or decorative.get("increased_contrast_stroke") not in stroke_tokens:
        errors.append("decorative boundary must name defined separator color and increased-contrast stroke")
    if required_boundary.get("color") == decorative.get("color") or required_boundary.get("stroke") == decorative.get("stroke"):
        errors.append("required and decorative boundaries must remain distinct semantic roles")
    if layout.get("hairline_policy", {}).get("required_boundary") != required_boundary.get("stroke"):
        errors.append("hairline policy and boundary styles disagree on required boundary stroke")
    if not isinstance(required_boundary.get("minimum_contrast"), (int, float)) or required_boundary["minimum_contrast"] < 3:
        errors.append("required control boundary must retain a non-text contrast threshold of at least 3")

    spacing_tokens = _token_map(base.get("spacing", {}).get("tokens", []))
    if (stroke_tokens.get("c-focus-width", {}).get("value") != "{stroke-emphasis}"
            or spacing_tokens.get("c-focus-offset", {}).get("value") != "{space-micro}"
            or focus.get("offset") != "c-focus-offset"
            or focus.get("placement") != "outside-control"):
        errors.append("focus must use the independent two-point ring and two-point offset contract")

    nested = layout.get("nested_radius", {}).get("concentric", {})
    if nested.get("floor_token") != "radius-square":
        errors.append("concentric radius floor must be the existing radius-square role")
    if nested.get("formula") != "max(floor_token, outer_radius - max(inset, 0))":
        errors.append("concentric radius formula must subtract nonnegative inset and clamp to floor")
    square = radius_tokens.get("radius-square", {}).get("value")
    if not isinstance(square, str) or not square.startswith("{") or square[1:-1] not in radius_tokens:
        errors.append("radius-square must resolve through an existing base radius token")
    radius_spec = system.get("specs", {}).get("radius_derivation", {})
    if radius_spec.get("effective_rule", {}).get("$ref") != "./source/layout.json#/nested_radius/concentric":
        errors.append("system radius derivation must reference the effective nested concentric rule")

    drag = interaction.get("drag", {})
    reduce_motion = motion.get("accessibility", {}).get("reduce_motion", "")
    if drag.get("direct_tracking") is not True or drag.get("drag_alternative_required") is not True:
        errors.append("drag policy must preserve direct tracking and a non-gesture alternative")
    if "all-durations-zero" not in reduce_motion:
        errors.append("motion source must define an explicit reduced-motion path")
    ime = interaction.get("ime", {})
    if not ime.get("priority") or not ime.get("blocks_all_submit_paths"):
        errors.append("IME policy must prioritize composition and block submit while composing")
    if interaction.get("scroll", {}).get("retain_stable_anchor") is not True:
        errors.append("scroll policy must retain a stable reading anchor")

    if not brand_config.get("brand_id") or not brand_config.get("seed_srgb"):
        errors.append("consumer brand configuration must expose an explicit brand id and seed")
    if brand_policy.get("algorithm") != "tekes-oklch-constrained-v1":
        errors.append("brand colors must follow the existing semantic derivation pipeline")
    selection_policy = brand_gallery.get("selection")
    if selection_policy is not None and (not isinstance(selection_policy, str) or "framework-owned" in selection_policy.lower()):
        errors.append("brand selection must not imply a framework-owned brand default")
    # The registry's canonical spelling is `presets`; entries are object records.
    profiles = brand_gallery.get("presets", [])
    profiles = profiles if isinstance(profiles, list) else []
    ids: list[str] = []
    indices: list[int] = []
    reference_id = brand_gallery.get("reference_brand")
    reference_profile = None
    has_clay = False
    for profile in profiles:
        if not isinstance(profile, dict):
            errors.append("brand catalog entries must be profile objects with id, name, config, and runtime_index")
            continue
        profile_id = profile.get("id")
        profile_index = profile.get("runtime_index")
        config_path = profile.get("config")
        if not isinstance(profile_id, str) or not profile_id:
            errors.append("brand profile id must be a nonempty string")
        else:
            ids.append(profile_id)
            has_clay = has_clay or profile_id == "example-clay"
            if profile_id == reference_id:
                reference_profile = profile
        if not isinstance(profile.get("name"), str) or not profile.get("name"):
            errors.append(f"brand profile {profile_id!r} needs a display name")
        if isinstance(profile_index, bool) or not isinstance(profile_index, int) or profile_index < 0:
            errors.append(f"brand profile {profile_id!r} needs a nonnegative integer runtime_index")
        else:
            indices.append(profile_index)
        if not isinstance(config_path, str) or not (root / config_path).is_file():
            errors.append(f"brand profile config does not exist: {config_path}")
    if len(ids) != len(set(ids)):
        errors.append("brand profile ids must be unique")
    if len(indices) != len(set(indices)):
        errors.append("brand profile runtime_index values must be unique")
    if not has_clay:
        errors.append("brand profile catalog must retain a Clay profile by record id")
    if not isinstance(reference_id, str) or reference_profile is None:
        errors.append("Gallery reference_brand must identify a registered profile id")
    elif reference_profile.get("config") != "design/brand/config.json":
        errors.append("Gallery reference brand profile must use the consumer-owned brand config")

    return errors


def _token_map(tokens: list[dict[str, Any]]) -> dict[str, dict[str, Any]]:
    if not isinstance(tokens, list):
        return {}
    return {token.get("name"): token for token in tokens if isinstance(token, dict) and token.get("name")}


def _relative_luminance(hex_color: Any) -> float:
    if not isinstance(hex_color, str) or len(hex_color) != 7 or not hex_color.startswith("#"):
        raise ValueError("not #RRGGBB")
    rgb = [int(hex_color[index : index + 2], 16) / 255.0 for index in (1, 3, 5)]
    linear = [value / 12.92 if value <= 0.04045 else ((value + 0.055) / 1.055) ** 2.4 for value in rgb]
    return 0.2126 * linear[0] + 0.7152 * linear[1] + 0.0722 * linear[2]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=ROOT, help="repository root (primarily for tests)")
    parser.add_argument("--json", action="store_true", help="emit separate mechanical and review-assessment sections")
    args = parser.parse_args()
    root = args.root.resolve()
    errors = check(root)
    gold7 = json.loads((root / GOLD7).read_text(encoding="utf-8"))
    review_assessments = [
        {"id": item.get("id"), **item.get("assessment", {})}
        for item in gold7.get("items", []) if isinstance(item, dict)
    ]
    report = {
        "mechanical_checks": {
            "status": "fail" if errors else "pass",
            "checks": sorted(MECHANICAL_CHECK_IDS),
            "errors": errors,
            "scope": "source integrity and mechanical relationships only",
        },
        "review_assessments": {
            "status": "separate-review-required",
            "items": review_assessments,
            "scope": "contract, code, and experience claims; not promoted by mechanical checks",
        },
    }
    if args.json:
        print(json.dumps(report, ensure_ascii=False, indent=2))
        return 1 if errors else 0
    if errors:
        for error in errors:
            print(f"FAIL: {error}")
        return 1
    print("PASS: Gold7 mechanical source/reference checks only.")
    print("Review assessments are a separate section; inspect with --json. This run does not establish code or experience acceptance.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
