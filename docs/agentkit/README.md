# AgentKit public foundation handoff (#15–#21)

This delivery makes the independently accepted `03721866a362b1d31261ebbbc86140eb07e9e23320871c325234a5212591213d` development inputs available through an immutable Git commit. The authoritative digest is in `public-source-037.json` (188 exact files; verify it rather than copying the prose). Pin the full Git commit SHA supplied with this branch, not a moving branch HEAD. This is an experimental UI foundation, not a stable-version release. No tag or registry publication is required for this handoff.

## Scope

| Issue | Public implementation | Accepted behavior |
| --- | --- | --- |
| #15 | `platform-macos/src/wake.rs`, window host | Targeted background wake, coalescing, close safety; no continuous data polling |
| #16 | `controls/src/reading.rs` | Prepend reading anchor, wrap/reflow and session identity |
| #17 | `platform-macos/src/text_key.rs`, `controls/src/composition/key.rs` | Native text commands, modifiers, IME consumption and submit intent |
| #18 | `controls/src/text_input/external.rs` | Controlled external value transactions and composition conflict handling |
| #19 | `controls/src/composition/pointer.rs`, native event metadata | Pointer selection and cancellation without stuck capture |
| #20 | `controls/src/text_surface.rs`, native text accessibility | Viewport constraints, candidate geometry and independent text navigation |
| #21 | `controls/src/composition/actions.rs` | Ordered, scoped actions carrying target and input source |

The reviewed input set includes the existing composition adapters, shared token author inputs and generated projection, platform bridge and public tests on which these APIs depend. Keeping those coupled modules byte-identical avoids creating a different implementation from the one AgentKit accepted. The later Gallery private lifecycle fix and other Gallery remediation WIP are excluded. The retained repository Gallery is compiled and tested for compatibility; it is not the newer WIP Gallery acceptance target.

The seven runtime-token JSON receipts are included only because the existing public registry tests require their metadata inputs. Component maturity is not promoted. No new tokens, UI framework, Agent presentation state, or Kernel behavior is introduced by packaging.

## Reproduce

Requirements: macOS with Metal, Rust 1.95.0 (the repository toolchain), Python 3.13 and the design package's pinned Python requirements. Use one reusable `CARGO_TARGET_DIR`, disable incremental compilation and debug symbols for review builds if storage is constrained. Registry dependencies may be downloaded normally; there are no sibling-checkout dependencies.

```sh
python3 scripts/verify-agentkit-public-pin.py
python3 -m pip install -r design/imports/v0.3.0-proposal1/upstream/TekesUI-Design-System-v0.3.0-proposal1/requirements.txt
python3 -m unittest discover -s scripts/tests
python3 scripts/test-merge-design-system.py
python3 scripts/merge-design-system.py --check
python3 scripts/generate-gallery-brands.py --check
cargo fmt --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo clippy --manifest-path examples/composition-consumer/Cargo.toml --all-targets --locked -- -D warnings
cargo test --manifest-path examples/composition-consumer/Cargo.toml --all-targets --locked
cargo build --manifest-path examples/composition-consumer/Cargo.toml --example native --locked
```

The small native consumer supports individual scenarios; use it for relevant native checks rather than building another full Gallery application. Inspect its existing README and CLI options. Close it after acceptance and remove only unused outputs attributable to that build, preserving the shared dependency cache and evidence.

## Consumer handoff

AgentKit should fetch the exact public commit, verify the 188-file digest, preserve its older e4 snapshot, and apply the already reviewed `native.rs` and `text_viewport.rs` consumer adaptations. Additional repository files beyond the 188 reviewed inputs are inventoried in `delivery-changes.json`; they do not silently replace those public inputs. The second consumer includes exact test-only key/pointer translation support, so it builds independently of Gallery private modules.

Re-run AgentKit UI, public-consumer and headless checks, its native demo build, updated tooling and the two agreed failure regressions. Record the fetched commit SHA and the resolved dependency paths, and report a separate close verdict for each Issue. The immutable pin alone does not prove consumer acceptance.

## Evidence boundaries

`validation.json` records the isolated delivery's tests. `development-consumer-review.json` is the receiver's earlier independent acceptance of the identical 037 inputs. `acceptance-matrix.json` and the referenced reports are historical source-bound receipts, including their then-pending public pin; those historical statements are retained rather than rewritten as new native runs.

Automatic tests, real GPU rendering, native interaction, physical Chinese IME and human VoiceOver acceptance remain separate layers. The native/IME/VoiceOver receipts retain their original byte identities; a new Git SHA with unchanged public files does not claim a new physical run. Performance is not measured. Complete component/page/brand/theme acceptance is outside #15–#21 and remains open under the remediation ledger.

The final consumer pin report supersedes historical delivery blockers only after it passes. Issue closure requires all original criteria, not just these local gates. No automatic GitHub close keywords are used in the delivery commit.
