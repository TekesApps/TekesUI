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

The initial protocol fixtures remain historical evidence; current delivered scope
and immutable checkpoints are recorded in their linked Issues. The standalone test is
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

## Ordinary decorative Separator (#54)

`tests/support/separator_fixture.rs::build` is the minimal public consumer: a
normal `StackContent` holds Text → `SeparatorContent` → Text. The parent owns
business labels, axis and constraints; the original SDK View owns Theme ink and
backing-scale hairlines. The separator creates no action, focus or AX owner.

Run the `separator_content` integration target to exercise ordinary mounting,
Theme/scale/constraints, unchanged reading/marked Composer and checked mount and
retirement. The explicitly ignored `separator_content_gpu` target is the original
8-frame/256-case full-ROI/clip comparison; it requires a fresh
`TEKESUI_SEPARATOR_GPU_OUT` evidence directory and the repository's coordinated
build lane. It opens no window. Source tests do not claim physical IME or
VoiceOver acceptance. AgentKit pin adoption remains #54's separate delivery step.


## Single-line readonly labels (#31)

The public API and the minimal mixed-language fixture are available at immutable
revision [`b6e4755b6cbd90c94e896c1f93f9afc96116202f`](https://github.com/TekesApps/TekesUI/tree/b6e4755b6cbd90c94e896c1f93f9afc96116202f).
See [`src/label_content.rs`](src/label_content.rs): original `TextContent`, a text
`Button`, and a `Button` holding a decorative original TextContent slot share
`Content::with_label_layout(LabelOptions::single_line(LabelOverflow::TailEllipsis))`.
The custom action supplies its complete accessible name. Source text stays intact.
This opt-in does not alter the default natural multiline path or editable text.

The existing native example selects this fixture with `native label`. Public API
checks are `label_content` and `label_layout_baseline`; the current real Metal
consumer check is `label_content_gpu`. Use the repository's coordinated build
lane, Rust 1.95.0 and locked/offline dependencies. The GPU check opens no window;
`TEKESUI_LABEL_CONTENT_GPU_OUT` selects the coordinated SSD evidence directory.
It records actual mixed-font runs, measured-layout identity, retained reflow,
Theme ink and visible glyph pixels. Independent native CTLine truncation and
Unicode cluster oracles remain in the text package's existing label tests.
See [the label contract and evidence](../../docs/design/label-content-integration-20261021.md)
for the exact verification identity and boundaries.


## Controlled original Menu (#36)

The fixed public SDK/consumer base is
[`7ba2da34ed9846a7726899d612a7136787dd77e0`](https://github.com/TekesApps/TekesUI/tree/7ba2da34ed9846a7726899d612a7136787dd77e0).
[`src/declared_menu.rs`](src/declared_menu.rs) declares the original compact
Menu trigger, Sort/Group submenus, checked/disabled items, separator and editor.
`MenuContent::set_entries` owns no business preference state. Consume
`ContentHost::drain_action_records()` and each owned
`ContentActionPayload::MenuSelection` value path, source, model revision and
mount; do not reconstruct an earlier choice from mutable `selected`.

The normal native entry is `native menu-tree`. In the coordinated acceptance
lane, `TEKES_COMPOSITION_SELFTEST=menu-tree` enables fixture-only phase commands
and `TEKESUI_COMPONENT_EVIDENCE` selects its assigned SSD output directory.
The existing AppKit selftest bridge sends pointer/key/scroll and real native AX
selectors; MenuContent still owns placement, hit testing, capture and painting.
The phase journal distinguishes original public semantic old-token rejection,
programmatic MenuView opening, actual native AX Press, and synthetic marked-text
input. These do not certify physical IME or VoiceOver speech. The coordinator
schedules native execution using the existing desktop occupancy/idle guards.
See [the Menu contract and evidence](../../docs/design/menu-declarations-20261021.md).
