use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, TextEditCommand,
    composition::{Component, ComponentId, ContentEvent, NumberFieldContent, NumberValidation},
};
use tekes_ui_scene::{Rect, Scene, Size};
use tekes_ui_settings::{Draft, FileProvider, Schema, SettingsForm, Snapshot};
use tekes_ui_tokens::{Rgba, proposal};
fn schema() -> Schema {
    Schema::parse(include_str!("fixtures/app-host-demo.settings.schema.toml")).unwrap()
}
fn with_number<R>(form: &mut SettingsForm, f: impl FnOnce(&mut NumberFieldContent) -> R) -> R {
    form.with_host_mut(|host| {
        f(host
            .get_mut(ComponentId(101))
            .unwrap()
            .component_mut::<NumberFieldContent>()
            .unwrap())
    })
}
fn type_value(form: &mut SettingsForm, text: &str) {
    with_number(form, |n| {
        n.set_focus(true, true);
        n.event(&ContentEvent::TextCommand(TextEditCommand::SelectAll));
        n.event(&ContentEvent::Commit {
            text: text.into(),
            replacement: None,
        });
    });
}
#[test]
fn only_committed_numbers_reach_save_and_cancel_keeps_saved_value() {
    let s = schema();
    let field = &s.settings[1];
    let mut draft = Draft::new(Snapshot::default());
    let mut form = SettingsForm::new_autosave(&s, &draft).unwrap();
    assert_eq!(with_number(&mut form, |n| n.input_digits()), Some(3));
    type_value(&mut form, "125%");
    assert_eq!(
        with_number(&mut form, |n| n.validation()),
        NumberValidation::Valid(125.)
    );
    form.collect(&s, &mut draft);
    assert!(!draft.dirty());
    with_number(&mut form, |n| {
        n.event(&ContentEvent::TextCommand(TextEditCommand::InsertNewline));
    });
    form.collect(&s, &mut draft);
    assert_eq!(draft.display(field), "125");
    let path = std::env::temp_dir()
        .join(format!(
            "settings-number-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
        .join("settings.toml");
    let provider = FileProvider::open(path.clone(), s.namespace.clone()).unwrap();
    let request = draft.submit(&s, "numeric-submit".into()).unwrap();
    let saved = provider.commit(&request).unwrap();
    draft.accept(&s, &request, saved).unwrap();
    assert_eq!(
        provider.load().unwrap().overrides["general"]["text_size_percent"].as_integer(),
        Some(125)
    );
    type_value(&mut form, "200");
    with_number(&mut form, |n| {
        n.event(&ContentEvent::TextCommand(TextEditCommand::CancelOperation));
    });
    form.collect(&s, &mut draft);
    assert!(!draft.dirty());
    assert_eq!(with_number(&mut form, |n| n.value()), 125.);
    assert!(with_number(&mut form, |n| n.step(true)));
    form.collect(&s, &mut draft);
    assert_eq!(draft.display(field), "135");
    assert!(with_number(&mut form, |n| n.step(false)));
    form.collect(&s, &mut draft);
    assert_eq!(draft.display(field), "125");
    type_value(&mut form, "301%");
    assert!(!form.flush_numbers(&s, &mut draft));
    assert_eq!(with_number(&mut form, |n| n.value()), 125.);
    assert!(!with_number(&mut form, |n| n.step(true)));
    with_number(&mut form, |n| n.cancel_edit());
    with_number(&mut form, |n| n.set_value(300.)).unwrap();
    assert!(!with_number(&mut form, |n| n.can_step(true)));
    assert!(!with_number(&mut form, |n| n.step(true)));
    with_number(&mut form, |n| n.set_value(50.)).unwrap();
    assert!(!with_number(&mut form, |n| n.step(false)));
    drop(provider);
    std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
}
struct Metrics;
impl Metrics {
    fn shape_paragraph(
        text: &str,
        role: proposal::TextRole,
        width: f32,
    ) -> tekes_ui_text::Paragraph {
        let char_w = 8.;
        let line_h = f64::from(role.style().line_height);
        let natural = text.chars().count() as f32 * char_w;
        let line_count = if width <= 0. || natural <= width {
            1
        } else {
            (natural / width.max(char_w)).ceil().max(1.) as usize
        };
        let para_w = f64::from(natural.min(width));
        let u16_len = text.encode_utf16().count();
        let lines = (0..line_count)
            .map(|idx| tekes_ui_text::Line {
                index: idx,
                utf16: 0..u16_len,
                bytes: 0..text.len(),
                ascent: line_h * 0.8,
                descent: line_h * 0.2,
                leading: 0.,
                height: line_h,
                top: idx as f64 * line_h,
                baseline_y: idx as f64 * line_h + line_h * 0.8,
                width: para_w,
                width_trimmed: para_w,
                runs: vec![],
                carets: (0..=u16_len)
                    .map(|i| (i, i as f64 * f64::from(char_w)))
                    .collect(),
                graphemes: vec![],
                has_newline: false,
            })
            .collect();
        tekes_ui_text::Paragraph {
            lines,
            height: line_count as f64 * line_h,
            width: para_w,
            default_line_height: line_h,
            fonts: vec![],
            utf16_len: u16_len,
        }
    }
}
impl tekes_ui_controls::TextSurfaceTextRenderer for Metrics {
    fn push_visible(
        &mut self,
        _: &mut Scene,
        _: &tekes_ui_text::Paragraph,
        _: tekes_ui_scene::Point,
        _: Rgba,
        _: Rect,
    ) {
    }
}
impl tekes_ui_controls::TextSurfaceHost for Metrics {
    fn shape_surface(
        &mut self,
        text: &str,
        role: proposal::TextRole,
        width: Option<f32>,
    ) -> std::rc::Rc<tekes_ui_text::Paragraph> {
        std::rc::Rc::new(Self::shape_paragraph(text, role, width.unwrap_or(f32::MAX)))
    }
    fn shape_label(
        &mut self,
        text: &str,
        role: proposal::TextRole,
        width: f32,
        _: tekes_ui_text::LabelOptions,
    ) -> Option<std::rc::Rc<tekes_ui_text::LabelLayout>> {
        Some(std::rc::Rc::new(tekes_ui_text::LabelLayout::new(
            text,
            Self::shape_paragraph(text, role, width),
            f64::from(width),
            0.,
            vec![],
        )))
    }
}
impl ButtonTextRenderer for Metrics {
    fn text_surface_host(&mut self) -> Option<&mut dyn tekes_ui_controls::TextSurfaceHost> {
        Some(self)
    }
    fn measure(&mut self, s: &str, role: proposal::TextRole, width: f32) -> Size {
        let p = Self::shape_paragraph(s, role, width);
        Size::new(p.width as f32, p.height as f32)
    }
    fn draw(
        &mut self,
        _: &mut Scene,
        _: &str,
        _: proposal::TextRole,
        _: Rect,
        _: ButtonTextAlign,
        _: Rgba,
    ) {
    }
}
#[test]
fn three_digit_percent_editor_has_intrinsic_stable_width() {
    let s = schema();
    let draft = Draft::new(Snapshot::default());
    let mut form = SettingsForm::new_autosave(&s, &draft).unwrap();
    with_number(&mut form, |n| {
        let width = n.measure(&mut Metrics, Size::new(600., 100.)).size.width;
        assert!(width < 250.);
        n.set_value(50.).unwrap();
        assert_eq!(
            n.measure(&mut Metrics, Size::new(600., 100.)).size.width,
            width
        );
        n.set_value(300.).unwrap();
        assert_eq!(
            n.measure(&mut Metrics, Size::new(1000., 100.)).size.width,
            width
        );
    });
}

#[test]
fn nested_for_item_dirty_drafts_integer_validation_trailing_space_suffix_and_save_error_focus_are_safe()
 {
    // Direct schema with item only in child group whose parent is reading.
    let nested_schema = Schema::parse(
        r#"
schema_version = 1
namespace = "test.nested"
provider = "test.nested.settings"

[[pages]]
id = "reading"
label = "Reading"

[[groups]]
id = "reading"
page = "reading"
label = "Reading"

[[groups]]
id = "reading.advanced"
page = "reading"
parent = "reading"
label = "Advanced"

[[groups.items]]
id = "spacing"
label = "Spacing"

[[groups.items.fields]]
id = "pad"
path = ["reading", "pad"]
label = "Pad"
type = "decimal"
default = 12.0
min = 0.0
max = 100.0
step = 1.0
suffix = "px "

[[groups.items.fields]]
id = "count"
path = ["reading", "count"]
label = "Count"
type = "integer"
default = 10
min = 1
max = 100
step = 1
"#,
    )
    .unwrap();
    let item = nested_schema.for_item("reading.advanced/spacing").unwrap();
    assert_eq!(item.groups.len(), 2);

    // Dirty non-numeric draft builds without panic!
    let mut draft = Draft::new(Snapshot::default());
    draft.edit(&item.settings[0], "abc".into());
    draft.edit(&item.settings[1], "999".into());
    let mut form = SettingsForm::new_autosave(&item, &draft).unwrap();
    // Collecting preserves valid controls and never panics on unmounted/nested rows.
    form.collect(&item, &mut draft);

    // Invalid disk snapshot fails closed in SettingsForm instead of panicking.
    let bad_base = Snapshot {
        revision: 1,
        last_commit_id: "bad".into(),
        overrides: [(
            "reading".into(),
            tekes_ui_settings::Value::Table(
                [(
                    "count".into(),
                    tekes_ui_settings::Value::String("bad".into()),
                )]
                .into_iter()
                .collect(),
            ),
        )]
        .into_iter()
        .collect(),
    };
    assert!(SettingsForm::new_autosave(&item, &Draft::new(bad_base)).is_err());

    // FieldType::Integer rejects fractional decimal "1.5" in UI validation.
    let s = schema();
    let mut int_draft = Draft::new(Snapshot::default());
    let mut int_form = SettingsForm::new_autosave(&s, &int_draft).unwrap();
    type_value(&mut int_form, "1.5%");
    assert!(!int_form.flush_numbers(&s, &mut int_draft));

    // replace_values drains stale blur changes without retaining old dirty edits.
    with_number(&mut int_form, |n| n.cancel_edit());
    with_number(&mut int_form, |n| n.set_value(120.)).unwrap();
    type_value(&mut int_form, "130%");
    with_number(&mut int_form, |n| {
        n.commit(tekes_ui_controls::composition::NumberCommitReason::Blur)
    });
    assert!(with_number(&mut int_form, |n| n.has_value_changes()));
    int_form.replace_values(&s, &Draft::new(Snapshot::default()));
    assert_eq!(with_number(&mut int_form, |n| n.value()), 100.);
    let mut empty_draft = Draft::new(Snapshot::default());
    int_form.collect(&s, &mut empty_draft);
    assert!(!empty_draft.dirty());

    // save_error renders APPLY/DISCARD and removes focused APPLY safely when cleared.
    let toggle_schema = Schema::parse(
        r#"
schema_version = 1
namespace = "test.save_error"
provider = "test.save_error.settings"

[[pages]]
id = "general"
label = "General"

[[groups]]
id = "general"
page = "general"
label = "General"

[[groups.items]]
id = "flag"
label = "Flag"

[[groups.items.fields]]
id = "enabled"
path = ["general", "enabled"]
label = "Enabled"
type = "boolean"
default = false
"#,
    )
    .unwrap();
    let mut toggle_form =
        SettingsForm::new_autosave(&toggle_schema, &Draft::new(Snapshot::default())).unwrap();
    toggle_form.save_error(Some("Readonly"), false);
    toggle_form
        .render(
            &mut Scene::new(),
            &mut Metrics,
            Rect::new(0., 0., 480., 400.),
            tekes_ui_tokens::Theme::LIGHT.into(),
        )
        .unwrap();
    let retry = toggle_form.host().get(ComponentId(5)).unwrap().frame();
    let discard = toggle_form.host().get(ComponentId(6)).unwrap().frame();
    assert_eq!(retry.min_y(), discard.min_y());
    assert!(retry.max_x() < discard.min_x());
    assert!(toggle_form.focus(Some(ComponentId(5)), true));
    assert_eq!(toggle_form.host().focused(), Some(ComponentId(5)));
    toggle_form.save_error(None, false);
    assert!(toggle_form.host().get(ComponentId(5)).is_none());
    assert_eq!(toggle_form.host().focused(), None);
}
