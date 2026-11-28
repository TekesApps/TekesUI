# TextArea

Use TextFieldContent with a multiline TextSurface for notes, drafts and messages. There is no separate TextAreaContent type.

## Creating the content

```rust
use tekes_ui_controls::composition::*;

let mut area = TextFieldContent::multiline("Message", "");
area.viewport = TextViewport::Grow { min: 56., max: 160. };
area.tab_policy = TextTabPolicy::Insert;
area.submit_policy = TextSubmitPolicy::CommandEnter;
let mut host = ContentHost::new(Content::new(ComponentId(5), area)).unwrap();
host.set_action_events(true);
```

## State and events

The final TextSurface constructor flags mean multiline and read-only. Generic multiline inputs (`TextFieldContent::multiline`, `TextSurface::new(..., true, _)`) default to `TextRole::MultilineBody` (`"multiline-body"`, 14 pt / 20 pt / 400); explicit composer styling keeps `.with_role(TextRole::ComposerBody)` (`"composer-body"`). surface.input holds text, selection and marked text. Fixed, Grow and Fill viewport modes control height; Fill also needs remaining-space allocation from the parent Stack. Use apply_external_value for versioned external drafts.

## Migrating exhaustive `TextRole` matches (`0.4.x` → `0.5.0`)

`0.5.0` adds `TextRole::MultilineBody` as a distinct variant alongside `TextRole::ComposerBody`. Downstream callers matching exhaustively on `TextRole` without `_` should add the new variant while keeping `"composer-body"` overrides keyed on `ComposerBody` (or `role.id() == "composer-body"`):

```rust
use tekes_ui_tokens::proposal::TextRole;

fn role_point_size(role: TextRole) -> f32 {
    match role {
        TextRole::UiBody => role.style().size,
        TextRole::MultilineBody => role.style().size, // 14 pt neutral multiline default
        TextRole::ComposerBody => 18.0,               // explicit composer-body override
        TextRole::UiLabel
        | TextRole::UiSecondary
        | TextRole::UiCaption
        | TextRole::UiHeading
        | TextRole::Code
        | TextRole::ToolbarLabel
        | TextRole::DocBody
        | TextRole::DocH1
        | TextRole::DocH2
        | TextRole::DocH3
        | TextRole::ChatBody
        | TextRole::ButtonXs
        | TextRole::ButtonSm
        | TextRole::ButtonMd
        | TextRole::ButtonLg => role.style().size,
    }
}
```

## Composition and limitations

Navigate is the default Tab policy. Insert permits tabs; Backtab is consumed without unindent. CommandEnter submission preserves ordinary Enter for newlines. Submission is an intent, not proof that a message was sent. TextSurface owns editing scroll and caret reveal; ScrollContent owns page scrolling. Use read_only_content or RichContent for reading.

## Related APIs

[Text Field](text-field.md)
