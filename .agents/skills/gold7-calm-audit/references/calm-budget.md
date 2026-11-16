# Gold7 Calm-UI Band-Pass Budget (Plan B)

## 1. Band-Pass Windows (`light` & `dark` Ordinary Themes)

All ratios use unrounded WCAG 2.2 relative luminance contrast; perceptual lightness `L`, chroma `C`, and `delta_ok` use CSS Color 4 OKLab/OKLCH (`brand_color.py` equations).

| Role / Metric | Ordinary Target (`light`, `dark`) | Guarded Violation |
|---|---|---|
| `primary-text` vs `surface-0..3` | `7.5 <= ratio <= 13.5` | `GLARE_PRIMARY_TEXT` (`> 13.5`) or `LOW_PRIMARY_TEXT` (`< 7.5`) |
| `secondary-text` vs `surface-0..3` | `4.5 <= ratio <= 6.6` | `OVER_CONTRAST_SECONDARY` (`> 6.6`) or `LOW_SECONDARY_TEXT` (`< 4.5`) |
| `tertiary-text` vs `surface-0..3` | `3.5 <= ratio <= 4.8` | `OVER_CONTRAST_TERTIARY` (`> 4.8`) or `LOW_TERTIARY_TEXT` (`< 3.5`) |
| `tertiary-text` vs transient/selection/diff grounds | `ratio >= 3.0` | `LOW_TERTIARY_TRANSIENT` (`< 3.0`) |
| `disabled-text` vs `surface-1` | `2.0 <= ratio <= 3.4` | `OVER_CONTRAST_DISABLED` (`> 3.4`) or `LOW_DISABLED_TEXT` (`< 2.0`) |
| Typographic order on `surface-1` | `primary > secondary > tertiary > disabled` | `TYPOGRAPHY_ORDER` |
| Perceptual step (`\|L(primary)-L(secondary)\|`, `\|L(sec)-L(ter)\|`, `\|L(ter)-L(dis)\|`) | `>= 0.12`, `>= 0.06`, `>= 0.06` | `TYPOGRAPHY_COLLAPSE` |
| `dark` `surface-0` & `code-surface` floor | `L_oklch >= 0.185` (avoids `#0B0B0A` OLED black) | `BLACK_HOLE_SURFACE` |
| Adjacent `surface-0..3` tier step | `1.00 <= ratio <= 1.12` | `SURFACE_STEP_EXCESS` |
| Warm-neutral guard (`surface-0`, `surface-1`, text roles, `border`, `separator`) | `R_byte >= B_byte` | `COLD_NEUTRAL_DRIFT` |
| `separator` vs `surface-1` | `1.08 <= ratio <= 1.35` | `NOISY_SEPARATOR` / `INVISIBLE_SEPARATOR` |
| `border` vs `surface-1` & `control_backgrounds` + `row-selection-hover` + `segmented-thumb` | `3.0 <= ratio <= 4.0` on `surface-1`; `>= 3.0` on control bgs | `HARSH_BORDER` / `LOW_BORDER` |
| `border-focused` vs `surface-1` | `3.2 <= ratio <= 4.4`, `delta_ok(border, border-focused) >= 0.012` | `HARSH_FOCUS` / `COLLAPSED_FOCUS` |
| `status-*` & `warning-foreground` vs `surface-1` | `4.5 <= ratio <= 8.0`, `C_oklch <= 0.15` | `OVEREXPOSED_STATUS` / `HIGH_CHROMA_STATUS` |
| `policy.json` ordinary targets | `dark.link <= 0.76`, `dark.outline <= 0.72`, `dark.subtle_text <= 0.80`, `light.action >= 0.50`, `light.link >= 0.44` | `HARSH_BRAND_POLICY` |

## 2. Increased-Contrast Invariants (`light-ic`, `dark-ic`)

- `primary-text`, `secondary-text`, `tertiary-text` vs `surface-0..3`: `ratio >= 7.0`, and strictly higher on `surface-1` than ordinary mode.
- `border`, `border-focused` vs `surface-0..3`: `ratio >= 3.0`, and strictly higher on `surface-1` than ordinary mode.
- Never lower `-ic` thresholds in `design/source/contrast-pairs.json` or `design/brand/policy.json`.

## 3. Official Token Regeneration Sequence

After editing `design/source/base.tokens.json`, `design/brand/policy.json`, or `design/source/contrast-pairs.json`:

1. Refresh modified file SHA-256 digests in `design/system.json#/theme_system/theme_input_contract/pinned_inputs`.
2. Regenerate all derived projections in order (using `target/design-tools-venv/bin/python`):
   ```bash
   target/design-tools-venv/bin/python scripts/generate-gallery-brands.py
   target/design-tools-venv/bin/python scripts/merge-design-system.py
   target/design-tools-venv/bin/python scripts/generate-theme-system.py --write-artifact-lock
   target/design-tools-venv/bin/python scripts/generate-component-capabilities.py
   target/design-tools-venv/bin/python scripts/check-reference-alignment.py --output design/reviews/reference-alignment-20261028/report.json
   ```
3. Re-run `audit_gold7_calm.py --check` and the repository test suite.
