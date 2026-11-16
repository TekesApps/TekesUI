# Markdown

Use RichDocument::from_markdown with RichContent for a selectable, read-only Markdown document. Markdown::parse and MarkdownView remain lower-level parsing and drawing entries.

## Creating the content

```rust
use tekes_ui_controls::composition::{RichContent, RichDocument};
let document = RichDocument::from_markdown("# Title\n\nA short paragraph.");
let content = RichContent::new("manual", 1, document).unwrap();
```

## State and events

The bounded parser supports headings, paragraphs, bullets, simple links and fenced code. text, blocks and links expose the projection; display_to_source and source_to_display use UTF-16 grapheme boundaries. RichContent.new and update return errors. update requires the expected version and an increasing revision for the same identity; create a new component to switch documents.

## Composition and limitations

Fenced code appears on a separate themed code surface with inset text and decorative line numbers. The same padded text layout drives selection and hit testing. Copy includes the displayed code body, without fences or line numbers.

RichContent colors recognized fenced languages using shared keyword, string, number and comment colors. Unknown or omitted languages remain plain. Syntax errors reject construction or update. Original glyph clusters determine color boundaries without reshaping. drain_link_actions reports targets and source ranges; the application decides navigation. ScrollContent owns document scrolling. Full CommonMark, tables, inline emphasis/code font shaping, HTML, remote images and scripts are unsupported.

## Related APIs

[Code Block](code-block.md)
