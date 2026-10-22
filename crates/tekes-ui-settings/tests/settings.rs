use std::path::PathBuf;
use tekes_ui_settings::SelectableListContent;
use tekes_ui_settings::{Draft, FileProvider, Schema, SettingsForm, Snapshot, Value};
fn schema() -> Schema {
    Schema::parse(include_str!("fixtures/layout.schema.toml")).unwrap()
}
fn temp() -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    std::env::temp_dir()
        .join(format!(
            "tekes-settings-test-{}-{}-{}",
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
        .join("settings.toml")
}
#[test]
fn schema_reports_unsupported_and_duplicate_or_overlapping_definitions() {
    let source = include_str!("fixtures/layout.schema.toml");
    assert!(Schema::parse(&source.replace("schema_version = 1", "schema_version = 99")).is_err());
    assert!(
        Schema::parse(&source.replace("id = \"autosave\"", "id = \"workspace-name\"")).is_err()
    );
    assert!(Schema::parse(&source.replace("type = \"integer\"", "type = \"action\"")).is_err());
    assert!(
        Schema::parse(&source.replace("[\"workspace\", \"autosave\"]", "[\"workspace\"]")).is_err()
    );
}
#[test]
fn range_validation_preserves_invalid_draft_and_cancel_is_local() {
    let s = schema();
    let mut draft = Draft::new(Snapshot::default());
    draft.edit(&s.settings[3], "999".into());
    assert!(draft.submit(&s, "bad".into()).is_err());
    assert_eq!(draft.display(&s.settings[3]), "999");
    draft.cancel();
    assert!(!draft.dirty());
    assert_eq!(draft.display(&s.settings[3]), "14");
    assert_eq!(draft.base.revision, 0);
}
#[test]
fn nested_typed_round_trip_reset_unsets_and_preserves_unknown_values() {
    let path = temp();
    let s = schema();
    let provider = FileProvider::open(path.clone(), s.namespace.clone()).unwrap();
    let mut base = Snapshot::default();
    base.overrides
        .insert("future".into(), Value::String("preserve".into()));
    let mut draft = Draft::new(base);
    for (f, value) in s
        .settings
        .iter()
        .zip(["研究工作区", "false", "en", "18", "1.75"])
    {
        draft.edit(f, value.into());
    }
    let request = draft.submit(&s, "one".into()).unwrap();
    let saved = provider.commit(&request).unwrap();
    draft.accept(&s, &request, saved.clone()).unwrap();
    assert!(!draft.dirty());
    assert_eq!(
        saved.overrides["reading"]["font_size"].as_integer(),
        Some(18)
    );
    assert_eq!(
        saved.overrides["reading"]["advanced"]["line_spacing"].as_float(),
        Some(1.75)
    );
    assert_eq!(
        saved.overrides["workspace"]["autosave"].as_bool(),
        Some(false)
    );
    draft.reset(&s);
    assert!(draft.dirty());
    assert_eq!(provider.load().unwrap(), saved);
    let reset = draft.submit(&s, "reset".into()).unwrap();
    let receipt = provider.commit(&reset).unwrap();
    assert_eq!(receipt.overrides.len(), 1);
    assert_eq!(receipt.overrides["future"].as_str(), Some("preserve"));
    draft.accept(&s, &reset, receipt).unwrap();
    assert!(!draft.dirty());
    drop(provider);
    std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
}
#[test]
fn lost_receipt_duplicate_precedes_revision_check_and_survives_provider_restart() {
    let path = temp();
    let s = schema();
    let mut draft = Draft::new(Snapshot::default());
    draft.edit(&s.settings[0], "Saved".into());
    let request = draft.submit(&s, "lost-receipt".into()).unwrap();
    let provider = FileProvider::open(path.clone(), s.namespace.clone()).unwrap();
    let first = provider.commit(&request).unwrap();
    drop(provider);
    let provider = FileProvider::open(path.clone(), s.namespace.clone()).unwrap();
    assert_eq!(provider.query(&request).unwrap(), Some(first.clone()));
    assert_eq!(provider.commit(&request).unwrap(), first);
    draft.accept(&s, &request, first).unwrap();
    assert!(!draft.dirty());
    drop(provider);
    std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
}
#[test]
fn successful_batch_cannot_erase_newer_typing_even_when_reverted_to_original() {
    let s = schema();
    for newer in ["Newer", "My Workspace"] {
        let mut draft = Draft::new(Snapshot::default());
        draft.edit(&s.settings[0], "Sent".into());
        let request = draft.submit(&s, "in-flight".into()).unwrap();
        draft.edit(&s.settings[0], newer.into());
        let snapshot = Snapshot {
            revision: 1,
            last_commit_id: request.commit_id.clone(),
            overrides: request.overrides.clone(),
        };
        draft.accept(&s, &request, snapshot).unwrap();
        assert!(draft.dirty());
        assert_eq!(draft.display(&s.settings[0]), newer);
    }
}
#[test]
fn provider_rejects_second_writer_stale_commit_and_corruption_without_overwrite() {
    let path = temp();
    let s = schema();
    let provider = FileProvider::open(path.clone(), s.namespace.clone()).unwrap();
    assert!(FileProvider::open(path.clone(), s.namespace.clone()).is_err());
    let mut draft = Draft::new(Snapshot::default());
    draft.edit(&s.settings[0], "First".into());
    let one = draft.submit(&s, "one".into()).unwrap();
    provider.commit(&one).unwrap();
    draft.edit(&s.settings[0], "Second".into());
    let stale = draft.submit(&s, "stale".into()).unwrap();
    let before = std::fs::read(&path).unwrap();
    assert!(provider.commit(&stale).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), before);
    std::fs::write(&path, "malformed = [").unwrap();
    assert!(provider.load().is_err());
    assert!(provider.commit(&one).is_err());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "malformed = [");
    drop(provider);
    std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
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
fn group_item_field_identity_and_item_filter_do_not_depend_on_labels() {
    let source = include_str!("fixtures/layout.schema.toml");
    let s = schema();
    assert_eq!(s.items().count(), 3);
    let item = s.for_item("reading/text").unwrap();
    assert_eq!(item.settings.len(), 2);
    assert!(item.settings.iter().all(|f| f.item == "reading/text"));
    let renamed = Schema::parse(&source.replace("基本设置", "General settings")).unwrap();
    assert_eq!(s.settings[0].id, renamed.settings[0].id);
    assert_eq!(s.settings[0].path, renamed.settings[0].path);
    let minimal = source
        .replace("[[pages]]\nid = \"general\"\nlabel = \"通用\"", "")
        .replace("page = \"general\"", "");
    assert_eq!(Schema::parse(&minimal).unwrap().items().count(), 3);
}
#[test]
fn discard_before_late_receipt_never_resurrects_cancelled_editor() {
    let s = schema();
    let mut draft = Draft::new(Snapshot::default());
    draft.edit(&s.settings[0], "Sent".into());
    let request = draft.submit(&s, "late".into()).unwrap();
    draft.cancel();
    let snapshot = Snapshot {
        revision: 1,
        last_commit_id: request.commit_id.clone(),
        overrides: request.overrides.clone(),
    };
    draft.accept(&s, &request, snapshot).unwrap();
    assert!(!draft.dirty());
    assert_eq!(draft.display(&s.settings[0]), "Sent");
}
#[test]
fn known_write_failure_is_not_unknown_and_does_not_replace_original() {
    let path = temp();
    let s = schema();
    let provider = FileProvider::open(path.clone(), s.namespace.clone()).unwrap();
    let mut draft = Draft::new(Snapshot::default());
    draft.edit(&s.settings[0], "Test".into());
    let request = draft.submit(&s, "failure".into()).unwrap();
    std::fs::create_dir(path.with_extension("pending")).unwrap();
    let error = provider.commit_checked(&request).unwrap_err();
    assert!(!error.unknown);
    assert!(!path.exists());
    assert!(draft.dirty());
    drop(provider);
    std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
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
fn apphost_general_defaults_only_include_appearance_and_text_zoom() {
    let s = Schema::parse(include_str!(
        "../../../examples/app-host-demo/settings.schema.toml"
    ))
    .unwrap();
    let draft = Draft::new(Snapshot::default());
    let field = |id: &str| s.settings.iter().find(|f| f.id.ends_with(id)).unwrap();
    assert_eq!(draft.display(field("/appearance")), "system");
    assert_eq!(draft.display(field("/text-size")), "100");
    assert_eq!(field("/text-size").min, Some(50.));
    assert_eq!(field("/text-size").max, Some(300.));
    assert_eq!(s.settings.len(), 2);
    assert!(
        s.settings
            .iter()
            .all(|f| f.id.ends_with("/appearance") || f.id.ends_with("/text-size"))
    );
    assert_eq!(
        s.items().map(|(id, _)| id).collect::<Vec<_>>(),
        vec!["general/appearance"]
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
