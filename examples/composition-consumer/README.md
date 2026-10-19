# Experimental composition consumer

For local component remediation, use the existing native cases through
`python3 scripts/component-review.py build <list|dialog|slider> <round-id>` from
the repository root. This reuses the compact public build cache and retains
separate acceptance receipts before cleaning round binaries. See
[the component review workflow](../../docs/component-review.md). Cases follow
the component currently under repair; the workflow does not require adding
unrelated scenarios or broad migration.

This is a separate Cargo workspace. It owns the icon adapter, a custom Badge,
UI data and composite structure, and imports only public TekesUI APIs.

```sh
cargo test --manifest-path examples/composition-consumer/Cargo.toml --locked
```

The host supplies its existing ButtonTextRenderer and uploads the approved
Settings SVG through Renderer::register_image. IconImages maps the resulting
images to palette-derived foregrounds. The fixture uses the production IconView,
Button state/paint and shared Stack layout. It declares one action and explicit
accessible name; its contents are decorative to accessibility and input.

The library remains an uncommitted protocol experiment. The standalone test is
public-API evidence, not native interaction or VoiceOver evidence. Gallery's
composition_acceptance test embeds this same consumer fixture with actual text,
registered icon pixels and GPU rendering, but does not migrate Gallery routes.

The updated #14 requires three minimum consumer scenes before broad migration.
The Button, heterogeneous List and Rich Dialog fixtures are implemented; full
native IME/accessibility/experience acceptance remains pending.

List and Rich Dialog minimum scenes now live in `scenarios.rs`, including actual
Field/TextSurface validation and stable focus/state. Run the standalone native
consumer on macOS with `cargo run --manifest-path examples/composition-consumer/Cargo.toml --example native -- list`
(or `-- dialog`). It uses public platform/CoreText/Metal APIs, never Gallery types.
The new scene adapters remain experimental: composed Dialog presence transitions,
physical IME/candidate acceptance and native control accessibility are pending.
See `docs/design/composition-list-dialog-20261017.md` in the repository for the
separate API, GPU, native and Gold7 evidence.
