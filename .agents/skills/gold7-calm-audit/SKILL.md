---
name: gold7-calm-audit
description: >-
  Audits and calibrates TekesUI themes, color tokens, and UI contrast under
  Gold7 (G1 warm craft, G2 four-tier micro-contrast surfaces, G3 hairline
  boundaries) with strict low-interference contrast ceilings. Use when
  designing, reviewing, or tuning theme palettes, base tokens, brand policy,
  or diagnosing harsh/high-contrast UI visuals in TekesUI.
---

# Gold7 Calm-UI Color & Contrast Audit

Evaluates and calibrates TekesUI color tokens (`light` and `dark` ordinary modes) against **Gold7 G1–G3** so interfaces stay warm, hierarchical, and free of visual glare while preserving uncompromised accessibility in `light-ic` and `dark-ic`.

## Core Principle: Band-Pass Calm UI (Plan B)

Ordinary reading modes (`light`, `dark`) must pass **both** minimum legibility floors **and** maximum glare ceilings. High-contrast modes (`light-ic`, `dark-ic`) strictly retain `7.0:1` text and `3.0:1` non-text guarantees without lowering thresholds.

- **Floors prevent illegibility** (see `design/source/contrast-pairs.json` and `design/theme/data-plot-pairs.json`).
- **Ceilings prevent visual fatigue** (`primary-text` capped at `<= 13.5:1`, `secondary-text <= 6.6:1`, `tertiary-text` distinct and receding at `3.5:1–4.8:1` on resting surfaces, dark `surface-0` above OLED-black glare `L_oklch >= 0.185`, restrained `border`/`separator` and desaturated status ink).

Detailed numeric windows and regeneration commands are documented in [calm-budget.md](./references/calm-budget.md).

## Strict Edit Boundaries

- **Permitted token-tuning inputs**:
  - `design/source/base.tokens.json` (ordinary `light` / `dark` primitive values)
  - `design/brand/policy.json` (ordinary `light` / `dark` brand derivation targets)
  - `design/source/contrast-pairs.json` (only if an ordinary contrast threshold needs explicit adjustment; keep `light-ic` / `dark-ic` untouched)
  - Official derived projections and hashes: `design/system.json` (`pinned_inputs`), `design/gallery/brands.json`, `design/tokens.json`, `design/proposal-mapping.json`, `design/theme/generated/*`, `design/theme/artifact.lock.json`, `design/gallery/capabilities.json`, and `design/reviews/reference-alignment-20261028/report.json`
- **Forbidden changes without explicit user approval**:
  - Never add, remove, or rename any `*.schema.json` file, Token ID, Rust `ColorRole` / `Theme` variant, or `Surface` binding in `design/theme/rules.json`.
  - Never modify `design/imports/v0.3.0-proposal1/`, `design/baseline/`, `scripts/fixtures/theme-v1.6.1-color-baseline.json`, or component Rust source files (`crates/`).
  - Never soften `light-ic` or `dark-ic` values or thresholds.

## Audit & Calibration Workflow

1. Run the deterministic band-pass audit on the repository:
   ```bash
   python3 .agents/skills/gold7-calm-audit/scripts/audit_gold7_calm.py --check
   ```
   *(Pass `--json` for machine-readable violations and per-role metrics.)*

2. Scan component surface bindings and Rust controls for chrome-noise over-use (**read-only advisory** — report findings to the user without editing `rules.json` or component code):
   ```bash
   python3 .agents/skills/gold7-calm-audit/scripts/audit_gold7_calm.py --scan-components
   ```

3. When calibrating ordinary `light` / `dark` values, update `design/source/base.tokens.json` and/or `design/brand/policy.json`, refresh `pinned_inputs` SHA-256 digests in `design/system.json`, run the 5 official generators listed in [calm-budget.md](./references/calm-budget.md), and verify with `--check` plus `scripts/test-gold7-calm-audit.py`, `scripts/check-gold7.py`, `scripts/check-reference-alignment.py`, and `scripts/check-design-system.py`.
