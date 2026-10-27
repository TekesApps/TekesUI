use tekes_ui_settings::{Draft, Schema, SelectableListContent, SettingsForm, Snapshot};
fn schema() -> Schema {
    Schema::parse(include_str!("fixtures/layout.schema.toml")).unwrap()
}

#[derive(Default)]
struct ListText;
impl tekes_ui_controls::ButtonTextRenderer for ListText {
    fn measure(
        &mut self,
        text: &str,
        _: tekes_ui_tokens::proposal::TextRole,
        width: f32,
    ) -> tekes_ui_scene::Size {
        tekes_ui_scene::Size::new((text.chars().count() as f32 * 8.).min(width), 16.)
    }
    fn draw(
        &mut self,
        _: &mut tekes_ui_scene::Scene,
        _: &str,
        _: tekes_ui_tokens::proposal::TextRole,
        _: tekes_ui_scene::Rect,
        _: tekes_ui_controls::ButtonTextAlign,
        _: tekes_ui_tokens::Rgba,
    ) {
    }
}

#[test]
fn shared_form_uses_real_controls_and_keeps_mounts_across_value_updates() {
    use tekes_ui_controls::composition::{
        ComponentId, MenuContent, SwitchContent, TextFieldContent,
    };
    let s = schema();
    let mut draft = Draft::new(Snapshot::default());
    let mut form = SettingsForm::new(&s, &draft).unwrap();
    assert!(
        form.host
            .get(ComponentId(100))
            .unwrap()
            .component::<TextFieldContent>()
            .is_some()
    );
    assert!(
        form.host
            .get(ComponentId(101))
            .unwrap()
            .component::<SwitchContent>()
            .is_some()
    );
    assert!(
        form.host
            .get(ComponentId(102))
            .unwrap()
            .component::<MenuContent>()
            .is_some()
    );
    let mount = form.host.semantic_mount(ComponentId(100));
    draft.edit(&s.settings[0], "Changed".into());
    form.replace_values(&s, &draft);
    assert_eq!(form.host.semantic_mount(ComponentId(100)), mount);
    assert_eq!(
        form.host
            .get(ComponentId(100))
            .unwrap()
            .component::<TextFieldContent>()
            .unwrap()
            .surface
            .input
            .text,
        "Changed"
    );
}

#[test]
fn resetting_a_menu_clears_prior_selection_semantics() {
    use tekes_ui_controls::composition::{Component, ComponentId, MenuContent};
    let s = schema();
    let mut draft = Draft::new(Snapshot::default());
    draft.edit(&s.settings[2], "en".into());
    let mut form = SettingsForm::new(&s, &draft).unwrap();
    draft.cancel();
    form.replace_values(&s, &draft);
    let menu = form
        .host
        .get(ComponentId(102))
        .unwrap()
        .component::<MenuContent>()
        .unwrap();
    assert!(menu.trigger.label.contains("跟随系统"));
    assert_ne!(menu.semantics().value, Some("English".into()));
}

#[test]
fn sidebar_uses_standard_list_paint_and_selection_routes_without_button_styles() {
    use tekes_ui_controls::{
        ControlKey,
        collections_view::ListView,
        composition::{Component, ComponentId, ContentEvent, ContentTheme},
        extended::List,
    };
    use tekes_ui_scene::{Rect, Scene};
    use tekes_ui_tokens::{Palette, Theme};
    let mut sidebar = tekes_ui_settings::SettingsSidebar::new(&schema()).unwrap();
    sidebar.select("reading/text");
    let frame = Rect::new(0., 0., 260., 500.);
    let palette = Palette::from(Theme::LIGHT);
    sidebar
        .render(&mut Scene::new(), &mut ListText, frame, palette)
        .unwrap();
    let group = sidebar
        .host
        .get(ComponentId(103))
        .unwrap()
        .component::<SelectableListContent>()
        .unwrap();
    assert_eq!(group.model.selected, Some(0));
    let labels = group.labels.clone();
    let mut adapted = SelectableListContent::new("阅读", labels.clone());
    adapted.model.selected = Some(1);
    let mut painted = Scene::new();
    adapted.paint(
        &mut painted,
        &mut ListText,
        frame,
        frame,
        ContentTheme::from(palette),
    );
    let mut standard = Scene::new();
    let mut model = List::new(labels.len());
    model.selected = Some(1);
    ListView::new().render(
        &mut standard,
        &mut ListText,
        &model,
        &labels,
        frame,
        Some(frame),
        palette,
    );
    assert_eq!(
        painted, standard,
        "adapter must preserve standard ListView ink exactly"
    );
    sidebar.host.accessibility_action(
        ComponentId(103),
        Some(0),
        tekes_ui_controls::composition::AccessibilityAction::Press,
    );
    assert_eq!(
        sidebar.take_selection(),
        None,
        "pressing the current item must not add another history visit"
    );
    sidebar.host.focus(Some(ComponentId(103)), true);
    assert_eq!(
        sidebar
            .event(&ContentEvent::Key {
                key: ControlKey::Down,
                down: true
            })
            .as_deref(),
        Some("reading/advanced")
    );
    assert_eq!(
        sidebar.take_selection(),
        None,
        "one input creates one route request"
    );
    sidebar.select("workspace/general");
    assert_eq!(
        sidebar
            .host
            .get(ComponentId(103))
            .unwrap()
            .component::<SelectableListContent>()
            .unwrap()
            .model
            .selected,
        None
    );
    assert_eq!(
        sidebar
            .host
            .get(ComponentId(101))
            .unwrap()
            .component::<SelectableListContent>()
            .unwrap()
            .model
            .selected,
        Some(0)
    );
}

#[test]
fn autosave_form_has_no_footer_and_preserves_original_text_mount_during_errors() {
    use tekes_ui_controls::{
        TextSurfaceAlignment, TextSurfacePresentation,
        composition::{ComponentId, TextFieldContent},
    };
    let schema = schema();
    let draft = Draft::new(Snapshot::default());
    let mut form = SettingsForm::new_autosave(&schema, &draft).unwrap();
    for id in [2, 3, 4, 5, 12] {
        assert!(form.host.get(ComponentId(id)).is_none());
    }
    let mount = form.host.semantic_mount(ComponentId(100));
    let field = form.host.get(ComponentId(100)).unwrap();
    let surface = &field.component::<TextFieldContent>().unwrap().surface;
    assert_eq!(surface.alignment, TextSurfaceAlignment::Trailing);
    assert_eq!(surface.presentation, TextSurfacePresentation::Content);
    form.save_error(Some("write failed"), false);
    assert!(form.host.get(ComponentId(5)).is_some());
    assert_eq!(form.host.semantic_mount(ComponentId(100)), mount);
    form.save_error(None, false);
    assert!(form.host.get(ComponentId(5)).is_none());
    form.replace_values(&schema, &draft);
    let field = form.host.get(ComponentId(100)).unwrap();
    assert_eq!(form.host.semantic_mount(ComponentId(100)), mount);
    assert_eq!(
        field
            .component::<TextFieldContent>()
            .unwrap()
            .surface
            .alignment,
        TextSurfaceAlignment::Trailing
    );
}

#[test]
fn descriptions_use_original_help_and_accessibility_without_extra_label_rows() {
    use tekes_ui_controls::composition::{ComponentId, TextContent};
    let schema = Schema::parse(include_str!(
        "../../../examples/app-host-demo/settings.schema.toml"
    ))
    .unwrap();
    let draft = Draft::new(Snapshot::default());
    let mut form = SettingsForm::new_autosave(&schema, &draft).unwrap();
    for (index, field) in schema.settings.iter().enumerate() {
        let base = (1_u64 << 32) + index as u64 * 4;
        let control = form.host.get(ComponentId(100 + index as u64)).unwrap();
        let label = form.host.get(ComponentId(base + 1)).unwrap();
        assert_eq!(control.help_text(), Some(field.description.as_str()));
        assert_eq!(label.help_text(), Some(field.description.as_str()));
        assert_eq!(label.component::<TextContent>().unwrap().text, field.label);
        assert!(form.host.get(ComponentId(base + 2)).is_none());
    }
    // Replacing values must not lose the original owner's help or mount.
    let mount = form.host.semantic_mount(ComponentId(100));
    form.replace_values(&schema, &draft);
    assert_eq!(form.host.semantic_mount(ComponentId(100)), mount);
    assert_eq!(
        form.host.get(ComponentId(100)).unwrap().help_text(),
        Some(schema.settings[0].description.as_str())
    );
    // Validation errors are still visible Text, not tooltip-only metadata.
    form.save_error(Some("文字大小: value outside allowed range"), false);
    assert_eq!(
        form.host
            .get(ComponentId(2))
            .unwrap()
            .component::<TextContent>()
            .unwrap()
            .text,
        "文字大小: value outside allowed range"
    );
}

#[test]
fn pinned_header_composes_original_back_and_editor_outside_scroll_owner() {
    use tekes_ui_controls::composition::{
        Button, Component, ComponentId, ScrollContent, StackContent, TextFieldContent,
    };
    let sidebar = tekes_ui_settings::SettingsSidebar::new_with_back(&schema()).unwrap();
    let root = sidebar
        .host
        .get(ComponentId(3))
        .unwrap()
        .component::<StackContent>()
        .unwrap();
    assert_eq!(root.children().len(), 2);
    assert!(root.children()[1].component::<ScrollContent>().is_some());
    let header = root.children()[0].component::<StackContent>().unwrap();
    assert_eq!(header.children().len(), 2);
    assert!(header.children()[0].component::<Button>().is_some());
    assert!(
        header.children()[1]
            .component::<TextFieldContent>()
            .is_some()
    );
}
