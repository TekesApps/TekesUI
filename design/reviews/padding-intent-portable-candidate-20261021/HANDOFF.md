# Issue 34 — independent padding focus intent increment

Input identity: `a30b18db1a1df1f5a780c245080b22221a63cb20556f7274b76786d4d896584e`.

Archive: `padding-intent-a30b18db1a1d.tar.gz`.
SHA256: `54b217691907389a0f9bc51bc8cafae4d54151dd2c491f32df7af5eaf25ded9e`.
Base: unchanged `95b140c278a2257c11fb956eaf860313c38bc4829bae70e022cc3d8f33bb38ac`.
497 source/lock files; exactly five increment paths listed in `portable-manifest.json`. No other live WIP copied. Existing token/set/API values unchanged. No Git publication, tag or freeze promotion.

## Shared contract and receiver use

`ContentHost::padding_focus_target(&ContentEvent) -> Option<ComponentId>` reads the original registered padding route. It shares classification with real dispatch: primary down only, actual child bounds shield padding even when unavailable/unconsumed, current scope/ancestry/editability/semantic clip/mount are retained. Open overlays and capture do not report padding. A modal may query its own registered subtree, not its background.

In the actual composing barrier, allow the original event only when this query returns the **same composing original editor**, in addition to the receiver's existing direct-editor and explicit nonfocusing business-navigation policies. `None` grants no permission. Do not allow the whole band or copy private hit/capture/overlay conditions.

No event, focus, reveal, caret/edit/marked/selection/undo, action, capture or key/IME epoch side effect. Same-turn policy hint only; not an authorization or asynchronous reservation. Dispatch rechecks the actual current state. Pending stale ownership may conservatively return None until ordinary reconciliation.

An existing shared hole was reproduced: a custom open overlay returning unconsumed allowed the old padding fallback to focus the background editor. Dispatch now uses the same query guard. Ordinary action/AX/style/token behavior stays unchanged. Prior immutable packets remain unchanged.

## Sender evidence and reproducible build

Root WIP:21 targeted tests,426 controls/253 consumer; strict controls/consumer/native-host Clippy;24 real GPU cases/120 functional frames;17 actual token probes/6432 bounded cases (`complete=false`). DS/pure-Rust/Gold7-contract/composition/capability checks passed.

Exact archive was clean-extracted and validated independently of the live root:50 text/388 controls/250 consumer; all three strict Clippy gates; original native example and review-native example build;24 GPU cases/120 frames. Every497 source/lock digest stayed unchanged; every local dependency resolves inside the extraction. See `portable-acceptance.json` and `portable-checks.json`.

Extract the archive into a new source folder (not a new target/cache). Use Cargo1.95 and an existing receiver target. Run:

```sh
cargo test --locked -p tekes-ui-controls
cargo test --locked --manifest-path examples/composition-consumer/Cargo.toml
cargo clippy --locked --manifest-path examples/composition-consumer/Cargo.toml --all-targets -- -D warnings
```

`review-harness/host` is outside the fixed497-source identity; its own exact hashes are in `review-harness-manifest.json`. To reproduce sender GPU/native harness builds, copy it to `source/design/reviews/padding-focus-intent-20261021/host`, matching its relative dependency layout. `replay.py` is the sender clean-extract recipe with shared-target locking, local-dependency/source hash checks and cleanup.

Root's signed native source identity `53c4be13e1ff838162be7d116edcf1f8a798796175fbfe615f861015e23305a7` passed10 real active U2790B@2 checkpoints: query+original focus, original selection preserved, actual Send once/never padding, read-only/disabled rejection, recovery, same-ID replacement rejection and explicit rebind. Receipt: `../padding-focus-intent-20261021/progress.json`. This is a separate **Root WIP** identity, not an isolated archive or actual AgentKit Composer native pass. Synthetic preedit API/GPU evidence does not substitute for physical IME.

## Receiver remaining acceptance

1. Verify identity/archive/files and build only this input; integrate public query into the actual Composer IME barrier.
2. On original main/SideChat inputs, test preedit+selection preserved by actual band padding while original Send/Stop/attachment and outside/overlay/capture routes remain fenced by the existing barrier.
3. Keep query and real dispatch on the same UI turn. Recheck invalidated mount, hidden/read-only/disabled, active overlay/modal and capture; do not treat a past ID as permission.
4. Record actual consumer results and whether this private development input is eligible to advance the development frozen candidate. Physical IME, matched ClientKit/Gold7 experience, public pin/CI and Issue closure remain separate explicit decisions.

Producer desktop is released and VoiceOver off. Original review executable/logs/receipts/screenshots/source/immutable packages retained; unused compilation outputs cleaned after process/lsof checks,453.2MiB reclaimed. Shared target remains about3.21GiB. No performance measurement or commit/push/close.
