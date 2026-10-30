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
