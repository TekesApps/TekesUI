# Readonly rich document consumer

The `rich_content` module independently composes public SDK components. `build()` returns the complete ContentHost with one continuous Markdown message, a code document, a diff and an independent composer. `append()` and `prepend()` show existing stable-item reading transactions. No Agent types, custom text shaping, hit mapping or scroll compensation live in the consumer.

```rust
use tekes_ui_controls::composition::{Content, ComponentId, RichContent, RichDocument};
let document = RichDocument::from_markdown("# Hello\n\n中文😀 e\u{301} [docs](https://example.com)");
let mut rich = RichContent::new("message-42", 1, document)?;
let expected = rich.version();
rich.update(&expected, 2, RichDocument::from_markdown("# Hello\n\n中文😀 e\u{301} [docs](https://example.com)\n\nTail"))?;
let content = Content::new(ComponentId(42), rich);
```

The native host reuses the original key, pointer, CoreText, AX and selftest adapters. Its normal launch is:

```sh
cargo +1.95.0 run --manifest-path examples/composition-consumer/Cargo.toml --example rich_content --locked --offline
```

Plain pointer gestures select text. Cmd-click and Cmd-Enter at a link emit a typed host callback; published AX Link parts emit the same callback with Accessibility provenance. Drain `RichContent::drain_link_actions()` after dispatch and apply host URL policy. The SDK does not open the target. Copy uses the original selected display text and the host clipboard bridge. The document stays readonly for commits, preedit and AX replacement.

See [the contract](../../docs/design/selectable-rich-content-20261030.md) for source/display UTF16 mapping, version rejection, Markdown literal support, code/diff copy semantics and layered acceptance. A prepared native script does not constitute native or physical-IME acceptance.

## Physical-IME manual controls

Use the newly built `rich_content` example with `TEKESUI_RICH_NATIVE_SELFTEST` **unset** and `TEKESUI_RICH_NATIVE_EVIDENCE` pointing to the granted attempt's fresh evidence directory. The normal macOS menu **Rich content manual** provides two independent commands: **Append rich document** invokes the existing `append()` reading transaction; **Toggle light / dark** changes the host theme and redraws. Neither command runs a selftest script, sends Preedit/Unmark/Commit, moves focus, or synthesizes keyboard/pointer events. Width changes remain real window resizing by the operator. These fixture controls do not change SDK interfaces.

Before asking the user to begin, the owner must have the actual new program SHA/source binding and Main's fresh native grant. CPU107's old executable lacks these menu commands and cannot be used for this step. Do not launch via the old automatic 11-phase runner.

1. Show the two menu commands before the physical case. The user selects/copies `中文😀` and checks the readonly document using the original instructions.
2. The user clicks **Independent draft**, types with the actual Chinese input source, and leaves a visible candidate unconfirmed. The owner records the real input-source identity, operator action and native Preedit/marked/focus baseline; no scripted baseline is installed.
3. Invoke **Append rich document**, then **Toggle light / dark**, independently while the original candidate is still unconfirmed. Each command writes `manual-append-before/after` or `manual-theme-before/after` to `events.jsonl` and checks the Composer's original focus, mount and external edit baseline after layout. The user may then physically narrow the window and continue/confirm the candidate normally. Append can be repeated through the existing increasing document version.
4. The owner compares the last actual Preedit baseline before menu interaction with the command's before/after records and observed candidate. If menu tracking itself commits, cancels or loses the candidate before command dispatch, the physical preservation case has failed or was not reached: do not use equality inside the command alone as proof. Do not re-inject Preedit to complete it. Keep the original input history and stop dependent actions on the first failure.

The fixture's existing `physical_IME:false` / `software_AppKit:true` log fields remain historical software provenance, not a verdict on the operator's physical input. Keep a separate human-action/input-source record and actual candidate observation. Native runtime, physical IME, VoiceOver and performance remain `not_run` until separately executed and reviewed. The original automatic 11-phase path is unchanged and is not part of this manual preparation rerun.
