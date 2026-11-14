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
        .component::<tekes_ui_controls::composition::KeyedSelectableListContent>()
        .unwrap();
    assert_eq!(
        group.selected_key(),
        Some(tekes_ui_controls::extended::ListItemKey(2))
    );
    let labels: Vec<_> = group
        .model()
        .snapshot()
        .items
        .iter()
        .map(|item| item.name.clone())
        .collect();
    let selected_part = group
        .semantic_part_for_key(tekes_ui_controls::extended::ListItemKey(2))
        .unwrap();
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
        Some(selected_part),
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
            .component::<tekes_ui_controls::composition::KeyedSelectableListContent>()
            .unwrap()
            .selected_key(),
        None
    );
    assert_eq!(
        sidebar
            .host
            .get(ComponentId(101))
            .unwrap()
            .component::<tekes_ui_controls::composition::KeyedSelectableListContent>()
            .unwrap()
            .selected_key(),
        Some(tekes_ui_controls::extended::ListItemKey(1))
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
        Button, Component, ComponentId, ContentEvent, IconImage, IconRenderError, ScrollContent,
        StackContent, TextFieldContent, WithIconRenderer,
    };
    use tekes_ui_scene::{ImageId, Rect, Scene};
    use tekes_ui_tokens::{Rgba, Theme};
    struct Icons;
    impl tekes_ui_controls::layout_view::IconRenderer for Icons {
        fn prepare_icon(
            &mut self,
            name: &str,
            _: f32,
            _: f32,
            _: Rgba,
        ) -> Result<IconImage, IconRenderError> {
            assert_eq!(name, "tekes-search");
            Ok(IconImage::new(ImageId(99), std::rc::Rc::new(())))
        }
        fn draw_icon(&mut self, _: &mut Scene, _: &str, _: Rect, _: Rgba) {
            panic!("header icon must retain its prepared image lease");
        }
    }
    let mut sidebar = tekes_ui_settings::SettingsSidebar::new_with_back(&schema()).unwrap();
    let root = sidebar
        .host
        .get(ComponentId(3))
        .unwrap()
        .component::<StackContent>()
        .unwrap();
    assert_eq!(root.children().len(), 2);
    assert!(root.children()[1].component::<ScrollContent>().is_some());
    let header = root.children()[0].component::<StackContent>().unwrap();
    assert_eq!(
        header.model.axis,
        tekes_ui_controls::extended::Axis::Horizontal
    );
    assert_eq!(header.children().len(), 2);
    assert!(header.children()[0].component::<Button>().is_some());
    assert!(header.children()[1].component::<Button>().is_some());
    assert!(sidebar.host.get(ComponentId(6)).is_none());
    let frame = Rect::new(0., 0., 260., 500.);
    sidebar
        .render(
            &mut Scene::new(),
            &mut WithIconRenderer::new(&mut ListText, &mut Icons),
            frame,
            Theme::LIGHT.into(),
        )
        .unwrap();
    let back_mount = sidebar.host.semantic_mount(ComponentId(4));
    let scroll_mount = sidebar.host.semantic_mount(ComponentId(1));
    assert!(back_mount.is_some() && scroll_mount.is_some());
    let search = sidebar.host.get(ComponentId(10)).unwrap().frame();
    let back = sidebar.host.get(ComponentId(4)).unwrap().frame();
    assert_eq!(search.center().y, back.center().y);
    assert_eq!(back.min_x(), frame.min_x());
    assert_eq!(search.max_x(), frame.max_x(), "search belongs at trailing");
    sidebar.event(&ContentEvent::PointerDown(search.center()));
    sidebar.event(&ContentEvent::PointerUp(search.center()));
    let header = sidebar
        .host
        .get(ComponentId(5))
        .unwrap()
        .component::<StackContent>()
        .unwrap();
    assert_eq!(header.children().len(), 1);
    assert!(
        header.children()[0]
            .component::<TextFieldContent>()
            .is_some()
    );
    assert!(sidebar.host.get(ComponentId(10)).is_none());
    assert_eq!(sidebar.host.focused(), Some(ComponentId(6)));
    assert!(sidebar.host.get(ComponentId(4)).is_none());
    assert_eq!(sidebar.host.semantic_mount(ComponentId(1)), scroll_mount);
    assert!(sidebar.cancel_search().unwrap());
    assert!(sidebar.host.get(ComponentId(6)).is_none());
    assert_eq!(sidebar.host.focused(), Some(ComponentId(10)));
    assert_eq!(sidebar.host.semantic_mount(ComponentId(1)), scroll_mount);
    let restored_back_mount = sidebar.host.semantic_mount(ComponentId(4));
    assert!(restored_back_mount.is_some());
    for width in [180., 360.] {
        let frame = Rect::new(12., 16., width, 500.);
        sidebar
            .render(
                &mut Scene::new(),
                &mut WithIconRenderer::new(&mut ListText, &mut Icons),
                frame,
                Theme::LIGHT.into(),
            )
            .unwrap();
        let search = sidebar.host.get(ComponentId(10)).unwrap().frame();
        let back = sidebar.host.get(ComponentId(4)).unwrap().frame();
        assert_eq!(back.min_x(), frame.min_x());
        assert_eq!(search.max_x(), frame.max_x());
        assert_eq!(search.center().y, back.center().y);
        assert_eq!(
            sidebar.host.semantic_mount(ComponentId(4)),
            restored_back_mount
        );
    }
}

#[test]
fn radio_presentation_rejects_non_enum_fields_at_schema_boundary() {
    let source = include_str!("fixtures/layout.schema.toml").replace(
        "type = \"string\"",
        "type = \"string\"\npresentation = \"radio\"",
    );
    assert!(
        Schema::parse(&source)
            .unwrap_err()
            .to_string()
            .contains("radio presentation requires enum")
    );
}

#[test]
fn sidebar_queued_selection_keeps_business_key_after_programmatic_sync() {
    use tekes_ui_controls::composition::{ComponentId, ContentEvent, KeyedSelectableListContent};
    use tekes_ui_controls::extended::ListItemKey;
    use tekes_ui_scene::{Rect, Scene};
    use tekes_ui_tokens::Theme;
    let mut definition = schema();
    // Labels are presentation only, including duplicate labels in one group.
    definition.groups[1].items[0].label = "Same".into();
    definition.groups[1].items[1].label = "Same".into();
    let mut sidebar = tekes_ui_settings::SettingsSidebar::new(&definition).unwrap();
    sidebar.select("reading/text");
    sidebar
        .render(
            &mut Scene::new(),
            &mut ListText,
            Rect::new(0., 0., 260., 500.),
            Theme::LIGHT.into(),
        )
        .unwrap();
    let list = sidebar
        .host
        .get(ComponentId(103))
        .unwrap()
        .component::<KeyedSelectableListContent>()
        .unwrap();
    let part = list.semantic_part_for_key(ListItemKey(3)).unwrap();
    use tekes_ui_controls::composition::Component;
    let point = list
        .semantic_parts()
        .into_iter()
        .find(|row| row.index == part)
        .unwrap()
        .frame
        .center();
    sidebar.host.event(&ContentEvent::PointerDown(point));
    sidebar.host.event(&ContentEvent::PointerUp(point));
    sidebar.select("workspace/general");
    assert_eq!(
        sidebar.take_selection().as_deref(),
        Some("reading/advanced"),
        "queued intent retains its original key, not the later selected index"
    );
    assert_eq!(sidebar.take_selection(), None);
    sidebar.select("reading/text");
    assert_eq!(
        sidebar.take_selection(),
        None,
        "programmatic publication is not user navigation"
    );
}

#[test]
fn sidebar_hidden_item_revokes_queued_selection_instead_of_retargeting() {
    use tekes_ui_controls::composition::{
        AccessibilityAction, ComponentId, KeyedSelectableListContent, ListDisplayUpdate,
        ListRowDisplay,
    };
    use tekes_ui_controls::extended::ListItemKey;
    use tekes_ui_scene::{Rect, Scene};
    use tekes_ui_tokens::Theme;
    let mut sidebar = tekes_ui_settings::SettingsSidebar::new(&schema()).unwrap();
    sidebar.select("workspace/general");
    sidebar
        .render(
            &mut Scene::new(),
            &mut ListText,
            Rect::new(0., 0., 260., 500.),
            Theme::LIGHT.into(),
        )
        .unwrap();
    let list = sidebar
        .host
        .get(ComponentId(103))
        .unwrap()
        .component::<KeyedSelectableListContent>()
        .unwrap();
    let part = list.semantic_part_for_key(ListItemKey(2)).unwrap();
    let mut snapshot = list.model().snapshot().clone();
    use tekes_ui_controls::composition::Component;
    let ids: Vec<_> = list.children().iter().map(|row| row.id()).collect();
    let mut rows: Vec<_> = snapshot
        .items
        .iter()
        .zip(ids)
        .map(|(item, row_id)| ListRowDisplay {
            key: item.key,
            row_id,
            display: ListDisplayUpdate::Keep,
        })
        .collect();
    sidebar
        .host
        .accessibility_action(ComponentId(103), Some(part), AccessibilityAction::Press);
    snapshot.display_items = vec![ListItemKey(3)];
    sidebar
        .host
        .replace_keyed_list(ComponentId(103), &snapshot, &mut rows, None)
        .unwrap();
    assert_eq!(
        sidebar.take_selection(),
        None,
        "hidden route cannot become the item now at its old display position"
    );
}

#[test]
fn grouped_form_keeps_rows_contiguous_and_separates_schema_groups() {
    use tekes_ui_controls::composition::ComponentId;
    use tekes_ui_scene::{CornerRadii, Primitive, Rect, Scene};
    use tekes_ui_tokens::{Palette, Spacing, Theme};
    // Three real controls: two in one schema group, one in the next.
    let definition = schema();
    let mut schema = definition.clone();
    let field = definition.settings[1].clone();
    schema.settings = (0..3)
        .map(|i| {
            let mut f = field.clone();
            f.id = format!("toggle-{i}");
            f.group = schema.groups[usize::from(i == 2)].id.clone();
            f
        })
        .collect();
    let draft = Draft::new(Snapshot::default());
    let mut form = SettingsForm::new_autosave(&schema, &draft).unwrap();
    // This geometry fixture uses measured labels, not a font-shaping service.
    // Keep original rows and control mounts; real wrapping is checked by the
    // SettingsRow multiline test and the native appearance candidate.
    for i in 0..3 {
        let id = ComponentId((1_u64 << 32) + i * 4 + 1);
        *form.host.get_mut(id).unwrap() = tekes_ui_controls::composition::Content::new(
            id,
            tekes_ui_controls::composition::TextContent::new("Setting"),
        );
    }
    let mount = form.host.semantic_mount(ComponentId(100));
    for width in [260., 900.] {
        for theme in [Theme::LIGHT, Theme::DARK] {
            let mut scene = Scene::new();
            let palette = Palette::from(theme);
            form.render(
                &mut scene,
                &mut ListText,
                Rect::new(0., 0., width, 500.),
                palette,
            )
            .unwrap();
            let rows: Vec<_> = (0..3)
                .map(|i| {
                    form.host
                        .get(ComponentId((1_u64 << 32) + i * 4))
                        .unwrap()
                        .frame()
                })
                .collect();
            assert_eq!(
                rows[0].max_y(),
                rows[1].min_y(),
                "one continuous group ground"
            );
            assert_eq!(rows[2].min_y() - rows[1].max_y(), Spacing::Relaxed.value());
            let body = form.host.get(ComponentId(11)).unwrap().frame();
            assert_eq!(body.size.width, width.min(560.));
            assert_eq!(body.min_x(), (width - body.size.width) * 0.5);
            let content = form.host.get(ComponentId(20)).unwrap().frame();
            assert_eq!(rows[0].size.width, content.size.width);
            assert_eq!(rows[0].min_x(), content.min_x());
            let grounds: Vec<_> = scene
                .primitives()
                .iter()
                .filter_map(|p| match p {
                    Primitive::Quad(q)
                        if q.fill == palette.color("c-settings-group-ground").unwrap() =>
                    {
                        Some(q)
                    }
                    _ => None,
                })
                .collect();
            assert_eq!(grounds.len(), 3);
            assert_eq!(grounds[0].corner_radii.bottom_left, 0.);
            assert!(grounds[0].corner_radii.top_left > 0.);
            assert_eq!(grounds[1].corner_radii.top_left, 0.);
            assert!(grounds[1].corner_radii.bottom_left > 0.);
            assert_eq!(
                grounds[2].corner_radii,
                CornerRadii::uniform(grounds[0].corner_radii.top_left)
            );
            let separator = scene
                .primitives()
                .iter()
                .filter_map(|p| match p {
                    Primitive::Quad(q)
                        if q.fill == palette.color("separator").unwrap()
                            && q.rect.min_y() == rows[1].min_y() =>
                    {
                        Some(q)
                    }
                    _ => None,
                })
                .next()
                .expect("inset separator within first group");
            assert_eq!(
                separator.rect.min_x(),
                rows[1].min_x() + Spacing::Loose.value()
            );
            assert_eq!(
                separator.rect.max_x(),
                rows[1].max_x() - Spacing::Loose.value()
            );
            assert_eq!(form.host.semantic_mount(ComponentId(100)), mount);
        }
    }
}
