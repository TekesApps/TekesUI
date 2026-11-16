# Skeleton

Use SkeletonContent for a static placeholder while your application prepares content.

## Creating the content

```rust
use tekes_ui_controls::composition::SkeletonContent;
let mut placeholder = SkeletonContent::default();
placeholder.set_accessibility(Some("Loading preview".into()), None).unwrap();
```

## State and events

The parent must provide finite width and height; Skeleton has no intrinsic height. layout reads geometry. set_accessibility switches between decorative output and a named Group; invalid empty names leave the declaration unchanged.

## Composition and limitations

Skeleton has no input, activation, timer, shimmer or retrieval behavior. Add ordinary status text when needed and replace the placeholder when data arrives or fails. It does not infer dimensions from future content.

## Related APIs

[Progress](progress.md)
