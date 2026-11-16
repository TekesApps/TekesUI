# DiffView

Use DiffView::between for a line-level comparison and RichDocument::from_diff with RichContent for selectable reading.

## Creating the content

```rust
use tekes_ui_controls::{composition::{RichContent, RichDocument}, extended::DiffView};
let diff = DiffView::between("old\nkeep", "new\nkeep");
let content = RichContent::new("diff-1", 1, RichDocument::from_diff(&diff)).unwrap();
```

## State and events

Each DiffLine contains Equal, Added or Removed and its text. RichContent keeps selection, source mapping and expected-version updates on the shared text path. Added and removed lines use shared status colors.

## Composition and limitations

The bounded LCS algorithm falls back to a removed block followed by an added block above its work budget; a minimal edit sequence is not guaranteed. Parent layout owns scrolling. There is no inline-character diff, conflict resolution, patch application or file writing. The application supplies both source versions.

## Related APIs

[Code Block](code-block.md)
