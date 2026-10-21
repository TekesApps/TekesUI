# Issue27 independent consumer candidate

Candidate `292f5fe65195af66624cb28413868eb46db6ab1a81ef1294ab19c8efbd45b0f4`.
Archive `layout-text-292f5fe65195.tar.gz`; SHA-256 `e9a816c5e34681f92d6632e1abe5995329f52a6d0757cf243df75aaf554fea76`.

479 source files, **8 changed paths** on untouched Issue31 candidate `545ef8d70d89404f11d916f05e417404277507b07dea808aac051339544431a1`. The initial packaging check expected nine paths, but proposal-mapping.json correctly remained unchanged; no archive was created before correcting that check. `scope-check-note.json` retains that distinction.

This is a separate optional Issue27 input. Finish validation of your current shared consumer change and 545 candidate first. Do not replace the current frozen/default/public pin or reinterpret 545 native/CI evidence as acceptance of this candidate.

## Change

ContentHost reuses its original text-service dependency scan, error and cancellation at entry, after measure, and after arrange. Layout-driven activation can no longer silently publish an active original text editor with missing TextSurfaceHost. Retry with a valid backend retains the original editor, value and reading attachment and does not steal focus. Application layout mutations are retained on failure. An entry-active subtree still needs a valid backend for its collapse frame; backend removal is allowed after it becomes inactive.

The independent Pane owns only layout activation and the original TextFieldContent. It introduces no Agent/Session state, editor clone or framework layer. Existing text shaping/native input are reused. The deliberately faulty native test adapter clears failed-frame native publication and fences fresh input while allowing pre-existing TextKey Finished acknowledgements to drain.

Only text_surface.layout_activation_preflight is added to the baseline JSON semantic contract. Existing tokens, sets and API token values are identical. The public SDK gains no new required error variant or backend trait method in this increment.

## Independent reproduction

Use Cargo 1.95 and the repository review-cache policy; keep debug info/incremental disabled. Extract the archive into a new directory; all local dependencies must resolve inside that extraction, and use --locked / --offline as available. Never point this package at the producer's WIP.

1. `cargo test --locked --manifest-path examples/composition-consumer/Cargo.toml --test layout_text_services`
2. `cargo test --locked -p tekes-ui-controls`
3. `cargo test --locked -p tekes-ui-text`
4. `cargo test --locked --manifest-path examples/composition-consumer/Cargo.toml`
5. Strict all-target Clippy for the three packages; build the consumer native example.
6. Independently replay your original Issue27 probe against this exact extraction. Check always-active parity, hidden missing-service frame, layout activation rejection and explicit valid renderer recovery.
7. If testing a native consumer, publish text input/AX only after successful render. Preserve the existing host's original input service; missing-backend mode is a diagnostic test, not a supported production renderer.

Sender exact-archive replay passed: 50 text / 388 controls / 181 consumer tests, strict all-target Clippy for all three, native example build and the existing432-case/864-frame core label GPU regression. All source/locks unchanged; all local dependencies remained inside the extraction. Owned outputs were cleaned and the Root cache lane restored. Receipts: portable-acceptance.json / portable-checks.json. Isolated292f native execution and receiver actual integration are still pending.

## Evidence boundaries

Producer WIP native identity `153090aaef37af951cfe35f0cee2cfafd28471c51d0cdf2ae6736fff3b4b1f39` is separate: 14 actual active U2790B@2 checkpoints, three native Tab entries, original baseline/value retained, failed-frame input/AX isolation and no stolen focus. This is not native acceptance of isolated292f.

Producer GPU proof: 48 cases/144 frames, editable and read-only reading-backed fields, four themes, three font scales, two width/backing combinations; recovery/reopen equals fresh valid original components. The preflight production source is exactly the same as this candidate. The Root receipts also include later unrelated WIP, so they are not substitute consumer receipts.

Gold7 evidence is scoped to unchanged valid rendering/recovery and original shared controls. Matched ClientKit product-page appearance, receiver actual integration/native review and exact public pin/CI remain separate. VoiceOver stayed off; performance was not measured.

Return per-item results bound to this full identity, source/lock preservation, actual consumer/native evidence, and whether you recommend a development freeze promotion. Do not promote or publish a pin automatically. No commit, push, tag or Issue closure is authorized by this handoff.
