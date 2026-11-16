# CodeBlock

Use CodeBlock to store a language and source, then RichDocument::from_code with RichContent for selectable code. CodeBlockView remains a lightweight drawing entry.

## Creating the content

```rust
use tekes_ui_controls::{composition::{RichContent, RichDocument}, extended::CodeBlock};
let code = CodeBlock { language: "rust".into(), source: "let count = 1;".into() };
let content = RichContent::new("source-1", 1, RichDocument::from_code(&code)).unwrap();
```

## State and events

lines reads source lines. RichContent recognizes bundled language names and extensions and applies shared keyword, string, number and comment colors. Unknown or omitted languages use plain text. Construction and versioned update report parser errors before changing content.

## Composition and limitations

Selection, copy and source mapping use the original text owner. Color does not change shaping, glyph positions or cluster boundaries. Parent ScrollContent supplies scrolling. This is a read-only reader with no execution, editing, breakpoints, semantic type analysis or compiler diagnostics. Compose a Button explicitly if a copy action is needed.

## Related APIs

[Markdown](markdown.md)
