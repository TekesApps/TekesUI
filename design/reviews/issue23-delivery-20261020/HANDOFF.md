# Issue23 delivery candidate - local acceptance complete, unpublished

Source identity: 122ae31df7e506ff85b492608550dc0bc0d9fa8cb803382b5d66e2e276511cd4
Public base: 431093786de47b48cd566d0d9c0eafa088b10d68
14 changed files. Cargo.lock is byte-exact public baseline.

Root cause: TextFieldContent previously coupled form decoration with reusable TextSurface services. The public read_only_content constructor opts into unadorned content, reusing UTF16 selection, controlled revision updates, AX and native text geometry. Parent ScrollContent remains the only scroll owner. Ordinary fields keep their default presentation. No Agent domain component or new editor/focus/scroll state machine.

Passed on this exact source:3 public API tests,233 core tests,134 consumer tests,strict consumer all-target Clippy,96 real GPU frames,pure Rust and design projection/system gates. The96 regenerated images are byte-identical to the individually inspected predecessor images.

Native host is live on U2790B. Active selection/copy/append/focus/readonly replay remains pending; Gold7 native actions are not accepted yet. The older2f090 independent consumer acceptance retains its old source identity.

Reproduction:
  cd source
  python3 scripts/verify-agentkit-public-pin.py docs/agentkit/public-source-issue23.json
  cargo test --locked -p tekes-ui-controls --test plain_text_content
  cargo test --locked --manifest-path examples/composition-consumer/Cargo.toml --tests
  cd ../gpu-harness
  TEKESUI_PLAIN_GPU_OUT=../gpu cargo test --locked independent_plain_content_has_no_form_pixels_and_keeps_real_selection_in_four_themes -- --exact

Use one shared CARGO_TARGET_DIR with debug=0 and incremental=false for dev/test; dependencies must be available before offline use. GPU harness imports these source packages and uses the verbatim native TextBackend; the existing GPU assertions are unchanged.

Predecessor9db9 is preserved. This source changes only CI timing-test exclusions and its identity manifest relative to9db9; all Rust/Cargo/design/consumer bytes are identical. The root build-cache cleanup fix is separate and is not mixed into this14file package.

No commit,push,tag,release or Issue closure performed. Next: finish native replay, obtain scoped publication authorization, then final public-commit consumer replay.

Portable review-bundle extraction replay also passed: public3/consumer134/GPU96, source manifest unchanged and all pixels byte-identical. The extracted directory and local Cargo artifacts were cleaned; original source/cache ownership restored. See portable-replay.json and portable-cache-cleanup.json. The pre-native bundle remains an immutable tested input; this later receipt is a companion evidence file.

Final native replay PASSED on source122ae31d:8 active U2790B checkpoints, real pointer selection/copy, controlled append+focus return, readonly isolation,360pt reflow and parent-only scrolling, active native document1790/0 navigation. Native and bounded Gold7 receipts are separate. The native round is accepted/cleaned, runtime absent, VoiceOver was not enabled. Initial inactive narrow-navigation capture is excluded and its active retry is retained.

All local acceptance layers are complete. Publication and final public-commit consumer acceptance remain separate and require explicit scoped authorization. The next bundle includes final companion evidence; prior pre-native bundle remains immutable.

User-authorized publication completed: public commit 61ddf7c68fccd819e20a01ba3104a8a0f8d7e64f on codex/agentkit-public-pin-issue23-20261020. Anonymous API verified14files and exact source122ae31d. Original960 source/config/docs files and HEAD unchanged. AgentKit session01a14920-0d6e-7652-9a60-c74ef1a1f70f received the exact public SHA for independent review; formal pin upgrade remains separate. CI39288833653 is in progress, not accepted as green.
