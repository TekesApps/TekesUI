# Text

Use TextContent for a static label, TextFieldContent::read_only_content for selectable prose, and RichContent for structured documents.

## Creating the content

```rust
use tekes_ui_controls::composition::*;
use tekes_ui_tokens::proposal::TextRole;
let label = TextContent::new("Status");
let paragraph = TextFieldContent::read_only_content("Article", "Selectable text", TextRole::UiBody);
```

## State and events

TextContent exposes text and role without adding editing selection. read_only_content uses a borderless content presentation with selection and copy. External text updates use the versioned TextSurface contract so selection and composition remain consistent.

## Composition and limitations

The host supplies text shaping and reading focus; a parent supplies document scrolling. Static labels normally do not take an editing focus stop. Animated integer displays use TickingNumber.

## Related APIs

[Ticking Number](ticking-number.md)
