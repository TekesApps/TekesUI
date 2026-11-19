# Markdown

Use `RichDocument::from_markdown` with `RichContent` for a selectable, strictly read-only Markdown document, or `MarkdownEditorContent` for interactive Live-Preview projected Markdown editing over one `TextFieldContent` / `TextSurface` owner. `Markdown::parse` and `MarkdownView` remain lower-level parsing and drawing entries.

## Creating the content

Read-only viewer:

```rust
use tekes_ui_controls::composition::{RichContent, RichDocument};
let document = RichDocument::from_markdown("# Title\n\nA short paragraph.");
let content = RichContent::new("manual", 1, document).unwrap();
```

Interactive Live-Preview editor (also suitable for `AppHost::others` panels):

```rust
use tekes_ui_controls::composition::{MarkdownEditorContent, TextViewport};
let editor = MarkdownEditorContent::new("notes", 1, "# Title\n\n- Item `code`")
    .unwrap()
    .with_viewport(TextViewport::Fixed(220.));
```

## State and events

`RichContent` remains read-only: its bounded parser supports headings, paragraphs, bullets, simple links and fenced code, with expected-version `update` requiring an increasing revision for the same identity.

`MarkdownEditorContent` keeps canonical LF-normalized Markdown (`source()`) and canonical source-level `Undo`/`Redo` over one retained `TextSurface`:
- **Live-Preview expansion**: unfocused blocks collapse delimiters; when focused, blocks intersecting `(source_anchor..source_caret)` (including `SelectAll`) reveal raw Markdown syntax inline while retaining block/syntax styling, and collapse when focus moves out.
- **IME & pointer drag freeze**: block re-projection freezes while `is_composing()` (`marked.is_some()`) or pointer drag capture is active, then reconciles on commit/unmark or pointer release.
- **Controlled streaming updates**: `check_external_source` and `apply_external_source` validate identity/revision/source-epoch conflicts (`ExternalValueConflict::Composition`, `ExternalValueConflict::StaleBaseline`) while preserving grapheme selection.

## Composition and limitations

Fenced code appears on a separate themed code surface with inset text and decorative line numbers (excluded from fences during editing and excluded from copy). Both `RichContent` and `MarkdownEditorContent` color recognized fenced languages using shared semantic tokens and report link activations via `drain_link_actions`. Inline code in `MarkdownEditorContent` uses `c-code-string`. Physical IME candidate positioning (#12) and VoiceOver acceptance (#5) remain tracked separately. Full CommonMark, GFM tables, HTML, remote images and scripts are unsupported.

## Related APIs

[Code Block](code-block.md) · [Diff View](diff-view.md)
