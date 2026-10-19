# Issue 23 source candidate

Base: `431093786de47b48cd566d0d9c0eafa088b10d68`. This is an unpublished development candidate, not a promoted AgentKit pin.

The existing `TextFieldContent::read_only_content(name, value, TextRole)` opts into unadorned, parent-scrolled text. It reuses TextSurface editing, UTF-16 selection, accessibility, controlled value revisions and shaping. Default form fields retain their presentation. No new editor or scroll state machine is introduced.

The independent `plain_text` consumer composes a document, footnote, ordinary field and existing actions. Native mode `plain` exercises the same public API and copies selections through the existing platform clipboard adapter.

```sh
cargo test --locked -p tekes-ui-controls --test plain_text_content
cargo test --locked --manifest-path examples/composition-consumer/Cargo.toml --tests
python3 scripts/verify-agentkit-public-pin.py docs/agentkit/public-source-issue23.json
cargo run --locked --manifest-path examples/composition-consumer/Cargo.toml --example native -- plain
```

The historical 037 manifest is preserved as historical provenance; CI explicitly selects this successor's input manifest. Source hash verification proves identity, not acceptance. Final public-commit replay and AgentKit formal-pin approval remain separate.

Original accepted publisher source: `2f0902d9f90fb069696c52edc38d9b7e7f8f0d0c4a3f0141e21fe1ac3f3183d0`. Consumer review: https://github.com/TekesApps/TekesUI/issues/23#issuecomment-6271592686 . Its native/GPU/Gold7 evidence applies to that source, not automatically to this reduced candidate.
