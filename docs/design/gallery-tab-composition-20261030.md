# Gallery Tab traversal while composing — Refs #3

## Scope and existing behavior

Gallery already normalizes native `insertTab:` / `insertBacktab:` selectors to
its original Tab traversal, which owns shell, story, leaf focus and reveal.
Existing regressions cover consecutive selectors, page boundaries, roving groups,
disabled controls and reveal of mounted offscreen fields. Those paths remain in
place; offscreen-but-mounted controls are not hidden controls.

The Issue's earlier `process-container-focus-traversal-20261022.md` reference is
available in the preserved gallery-design-baseline checkout, but is absent from
this candidate's committed baseline. That document describes the independent
#14/#48 ProcessContainer/Toolbar traversal contract and its headless evidence.
This change does not recreate that document or modify the remote focus owner.

## Concrete gap and correction

Before this change, a marked editor prevented selector normalization in
`Gallery::handle_input`, but the original `TextCommand` still reached
`TextDemo::command`. Its Tab fallback could unmark the editor and advance the
inner focus. Raw Tab could also reach Gallery's scope traversal while marked.
Skipping normalization alone therefore did not preserve the composition owner.

At the existing Gallery input boundary, both Tab key-down and the two selectors
now return unchanged while the active published text state is marked. Native
input interpretation has already had its opportunity to handle the key before
Gallery receives this event. Gallery does not commit, unmark, insert characters,
or move focus on behalf of that still-active composition. After an explicit
native commit/unmark, selectors use the unchanged one-step Tab route. No extra
focus index, timer, retry, state machine or input owner is introduced.

Gallery's multiline TextArea keeps its existing navigation policy: unmarked Tab
and Backtab leave/return through the original story order without inserting a
tab. This does not change the separate public `TextTabPolicy::Insert` option for
consumers that explicitly select insertion.

## Verification contract

Two regressions in the existing Gallery test module cover:

- Actual selector events, both directions, in standalone TextEditing and its
  internal-text story: marked text, selection, focus frame and viewport offset
  remain unchanged; raw Tab observes the same barrier. Unmark then permits the
  normal selector transition.
- Actual multiline TextArea story: selector-only entry, forward/reverse exit and
  return preserve its text and selection, documenting navigation rather than
  insertion.

Reuse existing shell/leaf traversal, disabled-target and reveal regressions as
applicable. Rust compilation and these behavioral tests require the next
coordinator-controlled shared build window; source parsing/format checks alone
are not test passes. AppKit selector delivery, physical Tab/Shift+Tab, CJK IME,
VoiceOver and cross-process native acceptance remain separate and unverified.
The #41 HID-refused attempt is preserved and is not retried by this batch.

## 集成验证（2026-10-30）

实际输入为 MAIN `c439cc8` 加本候选及保留 WIP；修复前精确 selector 回归已编译并实际运行 1 项，以 marked 状态返回 redraw 而非 `Change::NONE` 失败。修复后两条新 selector 回归及五条既有焦点／reveal／leaf／disabled 回归均实际各执行 1 项并通过，Gallery binary/tests check 和 Clippy `-D warnings` 通过。原 handoff 的 disabled 测试路径少了 `interaction_tests`，其零测试日志保留；已按真实路径 `gallery::workbench::interaction_tests::disabled_logical_target_is_skipped` 精确补跑通过。

原始输入、日志、失败与修复结果见 SSD `artifacts/delivery/integration-executor-20261030-07/{red01,green03}/`。物理键盘、AppKit selector／IME、VoiceOver 和跨进程 native 未验，原 AC 保持。
