# Issue #82 independent Panel consumer — local main intake

This six-file example receives the existing R19 consumer into `examples/panel-content-consumer/`, against local main `7ab8e3290d162d58e963011e8a969d5ed6bc57a0`. It uses the current repository crates through `../../crates`; no SDK is copied. The original R19 source remains at `/Volumes/HP P900/DEV/TekesUI/artifacts/issue-82/local-consumer-r19-prod-r2-main18-output-guard-20261026/consumer` and retains its historical identity and evidence.

## Preserved acceptance inputs

- `panel_content`: all three original CPU scenarios and assertions, including actual CoreText editor/IME state, retained history, zero-inset refusal, nested Panel and real Split ownership. Only the old `Button` type name is adapted to current `ButtonContent`.
- `panel_gpu`: the original 450 combinations (five primitive pipelines × five width/clip modes × three backing scales × three text scales × two themes) and 18 full-frame single-coverage oracles remain unchanged. The supplementary per-glyph/sentinel test remains separately named and does not replace the original matrix.
- `support/coretext.rs`: the original CoreText shaper, rasterizer, atlas and TextSurface renderer are preserved. No stub text or alternate geometry is introduced.
- The lockfile is current main's independent composition-consumer graph with only its root package name changed. Registry versions/checksums are unchanged. Obsolete `svg-icons` forwarding is removed; these sources do not require it. `token-audit` forwarding remains supported.

## Validation entry points

The main coordinator runs these from the final example directory, under the existing shared SSD cache/source-identity and build-lock rules. Reuse that target/cache; do not create a target beside this manifest. CPU, compile-only GPU, strict checks and actual GPU execution are separate evidence.

```sh
cargo test --locked --manifest-path examples/panel-content-consumer/Cargo.toml --test panel_content
cargo test --locked --manifest-path examples/panel-content-consumer/Cargo.toml --test panel_gpu --no-run
cargo clippy --locked --manifest-path examples/panel-content-consumer/Cargo.toml --test panel_content --test panel_gpu -- -D warnings
```

Actual GPU matrix execution requires macOS/Metal and a fresh `TEKESUI_PANEL82_GPU_ARTIFACT_DIR` beneath `/Volumes/HP P900/DEV/TekesUI/artifacts/issue-82/`. The source enforces that SSD Issue #82 root before creating outputs. The coordinator must verify its selected fresh run subdirectory does not exist and must not reuse the old R19 output.

```sh
cargo test --locked --manifest-path examples/panel-content-consumer/Cargo.toml --test panel_gpu actual_coretext_all_gpu_pipelines_and_full_frame_single_coverage_oracle -- --exact --nocapture
cargo test --locked --manifest-path examples/panel-content-consumer/Cargo.toml --test panel_gpu actual_coretext_each_glyph_run_scissor_matches_parent_fallback_and_restores_state -- --exact --nocapture
```

The supplementary test uses public production rendering and pixels. Its render-event metadata cannot expose or certify private prepared scissor/fallback state. Historical R2 diagnostic SDK logs remain separate historical evidence; current event ordinals must not be joined to those logs. No diagnostic cfg or private SDK API is required by this current consumer.

## Current status and original limits

The coordinator actually passed the original CPU3, current Controls312 (including three directed shared-query contracts), affected reading/Split/pointer regressions and both library/public-target strict checks. The original production450/18 matrix passed on actual Metal at main7ab plus the adapted consumer, with its exact source/program receipt retained; the later readonly query and accepted-wheel observation change no GPU rendering inputs. The required library dependency is local commit `22cb05e0032d47e031ab30c51a68132ae1f3c1a5`. Current supplementary per-run execution remains **not_run** and no historical private prepared-state diagnostic is transferred. Actual logs and original failures are under SSD `artifacts/issue-136/panel82-main7ab8e32-intake/`.

Remote B2, public immutable revision/release, native window/physical IME/VoiceOver and performance (where required by the original #82/#45 acceptance) remain separately unverified. This local consumer does not claim complete #82 or #45 acceptance. No current repository production code, Theme/global metadata, target/cache, release or remote state is changed by the six-file intake.
