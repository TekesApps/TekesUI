use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer,
    composition::{Component, ComponentId, ContentEvent, NumberFieldContent, NumberValidation},
};
use tekes_ui_scene::{Rect, Scene, Size};
use tekes_ui_settings::{Draft, FileProvider, Schema, SettingsForm, Snapshot};
use tekes_ui_tokens::{Rgba, proposal};
fn schema() -> Schema {
    Schema::parse(include_str!(
        "../../../examples/app-host-demo/settings.schema.toml"
    ))
    .unwrap()
}
fn number(form: &mut SettingsForm) -> &mut NumberFieldContent {
    form.host
        .get_mut(ComponentId(101))
        .unwrap()
        .component_mut::<NumberFieldContent>()
        .unwrap()
}
fn type_value(form: &mut SettingsForm, text: &str) {
    let n = number(form);
    n.set_focus(true, true);
    n.event(&ContentEvent::TextCommand("selectAll:".into()));
    n.event(&ContentEvent::Commit {
        text: text.into(),
        replacement: None,
    });
}
#[test]
fn only_committed_numbers_reach_save_and_cancel_keeps_saved_value() {
    let s = schema();
    let field = &s.settings[1];
    let mut draft = Draft::new(Snapshot::default());
    let mut form = SettingsForm::new_autosave(&s, &draft).unwrap();
    assert_eq!(number(&mut form).input_digits(), Some(3));
    type_value(&mut form, "125%");
    assert_eq!(
        number(&mut form).validation(),
        NumberValidation::Valid(125.)
    );
    form.collect(&s, &mut draft);
    assert!(!draft.dirty());
    number(&mut form).event(&ContentEvent::TextCommand("insertNewline:".into()));
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
    number(&mut form).event(&ContentEvent::TextCommand("cancelOperation:".into()));
    form.collect(&s, &mut draft);
    assert!(!draft.dirty());
    assert_eq!(number(&mut form).value(), 125.);
    assert!(number(&mut form).step(true));
    form.collect(&s, &mut draft);
    assert_eq!(draft.display(field), "135");
    assert!(number(&mut form).step(false));
    form.collect(&s, &mut draft);
    assert_eq!(draft.display(field), "125");
    type_value(&mut form, "301%");
    assert!(!form.flush_numbers(&s, &mut draft));
    assert_eq!(number(&mut form).value(), 125.);
    assert!(!number(&mut form).step(true));
    number(&mut form).cancel_edit();
    number(&mut form).set_value(300.).unwrap();
    assert!(!number(&mut form).can_step(true));
    assert!(!number(&mut form).step(true));
    number(&mut form).set_value(50.).unwrap();
    assert!(!number(&mut form).step(false));
    drop(provider);
    std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
}
struct Metrics;
impl ButtonTextRenderer for Metrics {
    fn measure(&mut self, s: &str, role: proposal::TextRole, _: f32) -> Size {
        Size::new(s.chars().count() as f32 * 8., role.style().line_height)
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
    let n = number(&mut form);
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
}
