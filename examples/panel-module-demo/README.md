# Native Panel module consumer

Neutral public-API consumer with three fixed module factories in independent processes. Rendering stays in each worker; IPC carries frame metadata, OS surface rights and toolbar descriptions.

All visible text goes through TekesUI controls: `TextFieldContent::read_only_content` for the selectable multiline document, `TextContent` for status, and the production `Button` label renderer for the counter. `src/text.rs` implements the controls' native font-service interfaces using CoreText, the default semantic TextRole metrics, and the worker glyph atlas. The fixture does not manually position body glyphs or use the display-only `DisplayParagraph` shortcut. TextSurface owns wrap width, paragraph caching, selection geometry and clipping; full Paragraph caret data is retained.

The columns contain different documents, each repeated four times: Chinese prose and punctuation; English prose and long words; mixed CJK/English, emoji/ZWJ/skin tones, combining accents, Japanese/Korean, Arabic/Hebrew and unbroken identifiers. The Host and module use default palette backgrounds without colored cards or panel frames. Fonts and line heights follow default UiBody tokens.

Displayed milliseconds measure the component's changed-layout CPU time, not input-to-display latency. Full editable-paragraph shaping costs more than the former display-only path; previous fixture timing figures do not establish this path's performance. Native IME, accessibility and complete editable-text IPC are still outside this prototype's acceptance; the body is read-only.

Reproduce the real-component CPU resize benchmark with:

```sh
cargo +1.95.0 run --manifest-path examples/panel-module-demo/Cargo.toml --example component_reflow_bench --target-dir target/apphost-native
```

TextSurface reuses a matching intrinsic measurement for arrangement; PreparedText retains bounded full-caret geometry for identical UTF-16 line ranges. Both optimizations preserve the production component path and editable paragraph data. The benchmark reports new widths separately from reverse dragging; it is not a display-latency measurement.
