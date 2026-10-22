# NumberFieldContent

`tekes_ui_controls::composition::NumberFieldContent` is an editable bounded numeric control with a text field and decrement/increment buttons. It reuses `TextFieldContent`, `TextSurface`, `NumberField` and `ButtonContent` for editing, layout and activation.

```rust
use tekes_ui_controls::{TextSurfaceAlignment, composition::*};

let mut field = NumberFieldContent::new(
    "Percentage", 100., 50., 300., 10.,
    DecimalNumberFormat { suffix: "%".into() },
).unwrap();
field.set_input_digits(3); // fixed input-box width for three digits and "%"
field.set_alignment(TextSurfaceAlignment::Trailing);
field.set_borderless(false);
let content = Content::new(ComponentId(42), field)
    .with_help("Enter commits; Escape cancels");
```

## Values, parsing and layout

- `new(name, value, min, max, step, format)` returns `None` for non-finite configuration, invalid bounds, a non-positive step, an out-of-range initial value or a codec that cannot round-trip the value.
- `NumberFormat` pairs `format(f64)` with `parse(&str)`. Formatted values must parse back to the same finite value; lossy formats report `FormatMismatch` rather than silently changing the number. Consumers may implement a codec for their own decimal precision or notation.
- `DecimalNumberFormat { suffix }` accepts ASCII decimal digits, an optional leading sign and decimal point, and the declared optional suffix, with surrounding whitespace. It rejects exponent notation, words, localized digits, undeclared units and non-finite values. `%` is a display suffix: `125` and `125%` both mean 125, and a step of 10 means ten percentage points.
- `set_input_digits(n)` fixes the **input box** width using the current font's widest measured digit samples, the format symbols, field padding and caret allowance. It accepts 1–1024 and returns `false` without changing configuration otherwise. Width does not depend on the current value or draft. Font/scale changes remeasure it automatically. This is layout capacity, not a text-length limit: longer drafts scroll, and narrow parent constraints can reduce the available width. Buttons add their own measured widths.
- `input_digits()` reads the setting; `clear_input_digits()` restores the default **auto content width**. In auto mode the field follows its draft text (with room for at least one formatted digit); neither mode stretches to fill extra parent space. Custom codecs may override `width_sample(digits)` when their zero representation needs a different sizing sample. The default substitutes the digit run into the formatted zero; the decimal codec preserves its suffix.
- `set_alignment(Leading | Center | Trailing)`, `set_borderless(bool)` and `set_disabled(bool)` use the standard Design System presentation. The default has field chrome and Quiet buttons. Alignment uses the same origin for painting, caret, selection and hit testing.

## Editing and committed values

`value()` returns the last committed valid number. `edit_state()` separately returns the draft, focus/composition state, dirty flag, validation and committed value. `validation()` is `Valid(f64)`, `Incomplete`, `Invalid`, `OutOfRange` or `FormatMismatch`.

| Operation | Behavior |
| --- | --- |
| Type, paste, select, text Undo/Redo | Changes the original text editor only; does not reformat or publish a numeric value. |
| Enter or blur | Commits a valid draft and normalizes its display; retains invalid/incomplete drafts without publishing. |
| Escape | Restores the committed value; while composing, delegates cancellation to the text editor. |
| Buttons or unmodified Up/Down | Steps from the current valid draft, then commits once. Step does not constrain typed values to multiples. Steps saturate at the bounds. |
| Invalid/incomplete draft | Retains its text and disables both buttons; stepping does not change it. |
| IME composition | Does not commit or step. Blur ends the original text marking but does not publish a numeric value in that blur. |

Tab/Backtab move focus through the control's single text-editor stop. Left/Right and selection modifiers retain their text meanings. Native key provenance prevents candidate-confirming keys or duplicate callbacks from submitting/stepping twice. Pointer cancellation and unmount cancel interaction without saving a draft.

`drain_value_changes()` returns once-only `{ previous, value, reason }` records, with reason `Submit`, `Blur` or `Step`. Only changed committed values enter this queue. Drain after host input, accessibility actions and focus changes, including blur; `ContentResponse.changed` also includes ordinary editing and must not be treated as a save signal. Persistence and configuration Undo/Redo belong to the consumer.

`set_value(value)` synchronizes authoritative consumer state without echoing a numeric change. It rejects invalid values and rejects different values while a draft is dirty or composing. An identical-value echo leaves the draft alone. Explicitly discard a non-composing draft with `cancel_edit()` before replacing it. `commit(reason)`, `step(increment)` and `can_step(increment)` are also public and respect disabled/composition state.

## Accessibility and help

The control exposes a named `TextInput`, current draft, invalid state, range description and numeric `value_range()`. Its named Decrease/Increase semantic buttons expose matching geometry and disabled state. Buttons support Press; the owner also supports Increment/Decrement, while text actions delegate to the original editor. `description`, `decrement_label` and `increment_label` are consumer-localizable. `Content.with_help()` supplies owner help; each button uses its action label as hover help.

Native adapters must preserve the editable text role when a numeric range is present. The macOS example uses AXTextField plus named action buttons and range description. Numeric text selection, caret and IME geometry continue through the original text service.

## Example and tests

The independent public-API example is `examples/composition-consumer/src/editable_number.rs`; it includes three-digit fixed percentage, auto-width plain number and four-digit fixed borderless decimal inputs.

```sh
cargo run --manifest-path examples/composition-consumer/Cargo.toml --example native -- editable-number
cargo test --manifest-path examples/composition-consumer/Cargo.toml --test editable_number --test editor_keys --test number_field --test content_help
cargo test -p tekes-ui-controls
```

Tests cover paired codecs, input restrictions, step/bounds, draft retention, commit/cancel/blur, IME/key chains, external value conflicts, text Undo/Redo and selection, keyboard focus, accessibility actions, hover help, fixed digit width, font scaling, alignment and narrow geometry. The native example forwards Copy/Cut/Paste through the original text transactions.
