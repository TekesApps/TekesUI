use tekes_ui_controls::composition::SelectableListContent;
use tekes_ui_settings::{Draft, Schema, SettingsForm, Snapshot};
fn schema() -> Schema {
    Schema::parse(include_str!("fixtures/layout.schema.toml")).unwrap()
}

#[derive(Default)]
struct ListText {
    type_scale: f32,
    draws: Vec<(String, tekes_ui_tokens::Rgba)>,
}
impl ListText {
    fn scale(&self) -> f32 {
        if self.type_scale > 0. {
            self.type_scale
        } else {
            1.
        }
    }
    fn shape_paragraph(
        &self,
        text: &str,
        role: tekes_ui_tokens::proposal::TextRole,
        width: Option<f32>,
    ) -> tekes_ui_text::Paragraph {
        let scale = self.scale();
        let char_w = 8. * scale;
        let line_h = f64::from(role.style().line_height * scale);
        let natural = text.chars().count() as f32 * char_w;
        let max_w = width.filter(|w| w.is_finite()).unwrap_or(f32::MAX);
        let line_count = if max_w <= 0. || natural <= max_w {
            1
        } else {
            (natural / max_w.max(char_w)).ceil().max(1.) as usize
        };
        let para_w = f64::from(natural.min(max_w));
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
            .collect::<Vec<_>>();
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
impl tekes_ui_controls::TextSurfaceTextRenderer for ListText {
    fn push_visible(
        &mut self,
        _: &mut tekes_ui_scene::Scene,
        _: &tekes_ui_text::Paragraph,
        _: tekes_ui_scene::Point,
        _: tekes_ui_tokens::Rgba,
        _: tekes_ui_scene::Rect,
    ) {
    }
}
impl tekes_ui_controls::TextSurfaceHost for ListText {
    fn type_scale(&self) -> f32 {
        self.scale()
    }
    fn shape_surface(
        &mut self,
        text: &str,
        role: tekes_ui_tokens::proposal::TextRole,
        width: Option<f32>,
    ) -> std::rc::Rc<tekes_ui_text::Paragraph> {
        std::rc::Rc::new(self.shape_paragraph(text, role, width))
    }
    fn shape_label(
        &mut self,
        text: &str,
        role: tekes_ui_tokens::proposal::TextRole,
        width: f32,
        options: tekes_ui_text::LabelOptions,
    ) -> Option<std::rc::Rc<tekes_ui_text::LabelLayout>> {
        let mut p = self.shape_paragraph(text, role, Some(width));
        if let Some(max_lines) = options.max_lines
            && p.lines.len() > max_lines.get()
        {
            p.lines.truncate(max_lines.get());
            let line_h = p.default_line_height;
            p.height = max_lines.get() as f64 * line_h;
        }
        Some(std::rc::Rc::new(tekes_ui_text::LabelLayout::new(
            text,
            p,
            f64::from(width),
            0.,
            vec![],
        )))
    }
}
impl tekes_ui_controls::ButtonTextRenderer for ListText {
    fn text_surface_host(&mut self) -> Option<&mut dyn tekes_ui_controls::TextSurfaceHost> {
        Some(self)
    }
    fn resolved_font_size(&mut self, role: tekes_ui_tokens::proposal::TextRole) -> f32 {
        role.style().size * self.scale()
    }
    fn measure(
        &mut self,
        text: &str,
        role: tekes_ui_tokens::proposal::TextRole,
        width: f32,
    ) -> tekes_ui_scene::Size {
        let p = self.shape_paragraph(text, role, Some(width));
        tekes_ui_scene::Size::new(p.width as f32, p.height as f32)
    }
    fn draw(
        &mut self,
        _: &mut tekes_ui_scene::Scene,
        text: &str,
        _: tekes_ui_tokens::proposal::TextRole,
        _: tekes_ui_scene::Rect,
        _: tekes_ui_controls::ButtonTextAlign,
        ink: tekes_ui_tokens::Rgba,
    ) {
        self.draws.push((text.to_owned(), ink));
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
        form.host()
            .get(ComponentId(100))
            .unwrap()
            .component::<TextFieldContent>()
            .is_some()
    );
    assert!(
        form.host()
            .get(ComponentId(101))
            .unwrap()
            .component::<SwitchContent>()
            .is_some()
    );
    assert!(
        form.host()
            .get(ComponentId(102))
            .unwrap()
            .component::<MenuContent>()
            .is_some()
    );
    let mount = form.host().semantic_mount(ComponentId(100));
    draft.edit(&s.settings[0], "Changed".into());
    form.replace_values(&s, &draft);
    assert_eq!(form.host().semantic_mount(ComponentId(100)), mount);
    assert_eq!(
        form.host()
            .get(ComponentId(100))
            .unwrap()
            .component::<TextFieldContent>()
            .unwrap()
            .text(),
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
        .host()
        .get(ComponentId(102))
        .unwrap()
        .component::<MenuContent>()
        .unwrap();
    assert!(menu.trigger().label().contains("跟随系统"));
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
        .render(&mut Scene::new(), &mut ListText::default(), frame, palette)
        .unwrap();
    let group = sidebar
        .host()
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
        &mut ListText::default(),
        frame,
        frame,
        ContentTheme::from(palette),
    );
    let mut standard = Scene::new();
    let mut model = List::new(labels.len());
    model.selected = Some(1);
    ListView::new().render(
        &mut standard,
        &mut ListText::default(),
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
    sidebar.with_host_mut(|host| {
        host.accessibility_action(
            ComponentId(103),
            Some(selected_part),
            tekes_ui_controls::composition::AccessibilityAction::Press,
        );
    });
    assert_eq!(
        sidebar.take_selection(),
        None,
        "pressing the current item must not add another history visit"
    );
    sidebar.focus(Some(ComponentId(103)), true);
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
            .host()
            .get(ComponentId(103))
            .unwrap()
            .component::<tekes_ui_controls::composition::KeyedSelectableListContent>()
            .unwrap()
            .selected_key(),
        None
    );
    assert_eq!(
        sidebar
            .host()
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
        assert!(form.host().get(ComponentId(id)).is_none());
    }
    let mount = form.host().semantic_mount(ComponentId(100));
    let field = form.host().get(ComponentId(100)).unwrap();
    let tf = field.component::<TextFieldContent>().unwrap();
    assert_eq!(tf.alignment(), TextSurfaceAlignment::Trailing);
    assert_eq!(tf.surface_presentation(), TextSurfacePresentation::Content);
    form.save_error(Some("write failed"), false);
    form.render(
        &mut tekes_ui_scene::Scene::new(),
        &mut ListText::default(),
        tekes_ui_scene::Rect::new(0., 0., 560., 400.),
        tekes_ui_tokens::Palette::from(tekes_ui_tokens::Theme::LIGHT),
    )
    .unwrap();
    let apply = form.host().get(ComponentId(5)).unwrap().frame();
    let discard = form.host().get(ComponentId(6)).unwrap().frame();
    assert_eq!(apply.min_y(), discard.min_y());
    assert!(apply.max_x() < discard.min_x());
    assert_eq!(form.host().semantic_mount(ComponentId(100)), mount);
    form.save_error(None, false);
    assert!(form.host().get(ComponentId(5)).is_none());
    form.replace_values(&schema, &draft);
    let field = form.host().get(ComponentId(100)).unwrap();
    assert_eq!(form.host().semantic_mount(ComponentId(100)), mount);
    assert_eq!(
        field.component::<TextFieldContent>().unwrap().alignment(),
        TextSurfaceAlignment::Trailing
    );
}

#[test]
fn descriptions_use_original_help_and_accessibility_without_extra_label_rows() {
    use tekes_ui_controls::composition::{ComponentId, TextContent};
    let schema =
        Schema::parse(include_str!("fixtures/app-host-demo.settings.schema.toml")).unwrap();
    let draft = Draft::new(Snapshot::default());
    let mut form = SettingsForm::new_autosave(&schema, &draft).unwrap();
    for (index, field) in schema.settings.iter().enumerate() {
        let base = (1_u64 << 32) + index as u64 * 4;
        let control = form.host().get(ComponentId(100 + index as u64)).unwrap();
        let label = form.host().get(ComponentId(base + 1)).unwrap();
        assert_eq!(control.help_text(), Some(field.description.as_str()));
        assert_eq!(label.help_text(), Some(field.description.as_str()));
        assert_eq!(
            label.component::<TextContent>().unwrap().text(),
            field.label
        );
        assert!(form.host().get(ComponentId(base + 2)).is_none());
    }
    // Replacing values must not lose the original owner's help or mount.
    let mount = form.host().semantic_mount(ComponentId(100));
    form.replace_values(&schema, &draft);
    assert_eq!(form.host().semantic_mount(ComponentId(100)), mount);
    assert_eq!(
        form.host().get(ComponentId(100)).unwrap().help_text(),
        Some(schema.settings[0].description.as_str())
    );
    // Validation errors are still visible Text, not tooltip-only metadata.
    form.save_error(Some("文字大小: value outside allowed range"), false);
    assert_eq!(
        form.host()
            .get(ComponentId(2))
            .unwrap()
            .component::<TextContent>()
            .unwrap()
            .text(),
        "文字大小: value outside allowed range"
    );
}

#[test]
fn pinned_header_composes_original_back_and_editor_outside_scroll_owner() {
    use tekes_ui_controls::composition::{
        Button, ComponentId, ContentEvent, IconImage, IconRenderError, ScrollContent,
        SearchFieldContent, StackContent, WithIconRenderer,
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
        .host()
        .get(ComponentId(3))
        .unwrap()
        .component::<StackContent>()
        .unwrap();
    assert_eq!(root.children().len(), 2);
    assert!(root.children()[1].component::<ScrollContent>().is_some());
    let header = sidebar
        .host()
        .get(ComponentId(5))
        .unwrap()
        .component::<StackContent>()
        .unwrap();
    assert_eq!(header.axis(), tekes_ui_controls::extended::Axis::Horizontal);
    assert_eq!(header.children().len(), 2);
    assert!(header.children()[0].component::<Button>().is_some());
    assert!(header.children()[1].component::<Button>().is_some());
    assert!(sidebar.host().get(ComponentId(6)).is_none());
    let frame = Rect::new(0., 0., 260., 500.);
    sidebar
        .render(
            &mut Scene::new(),
            &mut WithIconRenderer::new(&mut ListText::default(), &mut Icons),
            frame,
            Theme::LIGHT.into(),
        )
        .unwrap();
    let back_mount = sidebar.back_button_mount();
    let scroll_mount = sidebar.host().semantic_mount(ComponentId(1));
    assert!(back_mount.is_some() && scroll_mount.is_some());
    let search = sidebar.host().get(ComponentId(10)).unwrap().frame();
    let back = sidebar.back_button_frame().unwrap();
    let header_frame = sidebar.host().get(ComponentId(5)).unwrap().frame();
    let scroll_frame = sidebar.host().get(ComponentId(1)).unwrap().frame();
    let first_heading = sidebar.host().get(ComponentId(100)).unwrap().frame();
    let first_list = sidebar.host().get(ComponentId(101)).unwrap().frame();
    let second_heading = sidebar.host().get(ComponentId(102)).unwrap().frame();
    assert_eq!(search.center().y, back.center().y);
    assert_eq!(
        back.min_x(),
        frame.min_x() + tekes_ui_tokens::Spacing::Loose.value()
    );
    assert_eq!(
        header_frame.min_y(),
        frame.min_y() + tekes_ui_tokens::Spacing::Loose.value()
    );
    assert_eq!(
        scroll_frame.min_y() - header_frame.max_y(),
        tekes_ui_tokens::Spacing::Regular.value(),
        "pinned header keeps 8pt clip clearance above scrolling list"
    );
    assert_eq!(scroll_frame.max_x(), frame.max_x());
    assert_eq!(
        first_heading.min_x(),
        back.min_x(),
        "sidebar header and scrolled group headings align on leading edge"
    );
    assert_eq!(
        first_heading.min_y() - header_frame.max_y(),
        tekes_ui_tokens::Spacing::Loose.value(),
        "symmetric 16pt header-to-first-heading gap instead of fractured 28pt"
    );
    assert_eq!(
        second_heading.min_y() - first_list.max_y(),
        tekes_ui_controls::composition::ListContent::SECTION_GAP
    );
    assert_eq!(
        first_list.min_y() - first_heading.max_y(),
        tekes_ui_controls::composition::ListContent::SECTION_GAP
    );
    assert_eq!(
        search.max_x(),
        frame.max_x() - tekes_ui_tokens::Spacing::Loose.value(),
        "search belongs at padded trailing"
    );
    sidebar.event(&ContentEvent::PointerDown(search.center()));
    sidebar.event(&ContentEvent::PointerUp(search.center()));
    let header = sidebar
        .host()
        .get(ComponentId(5))
        .unwrap()
        .component::<StackContent>()
        .unwrap();
    assert_eq!(header.children().len(), 1);
    assert!(
        header.children()[0]
            .component::<SearchFieldContent>()
            .is_some()
    );
    assert!(sidebar.host().get(ComponentId(10)).is_none());
    assert_eq!(sidebar.host().focused(), Some(ComponentId(6)));
    assert!(!sidebar.has_back_button());
    assert_eq!(sidebar.host().semantic_mount(ComponentId(1)), scroll_mount);
    assert!(sidebar.cancel_search().unwrap());
    assert!(sidebar.host().get(ComponentId(6)).is_none());
    assert_eq!(sidebar.host().focused(), Some(ComponentId(10)));
    assert_eq!(sidebar.host().semantic_mount(ComponentId(1)), scroll_mount);
    let restored_back_mount = sidebar.back_button_mount();
    assert!(restored_back_mount.is_some());
    for width in [180., 360.] {
        let frame = Rect::new(12., 16., width, 500.);
        sidebar
            .render(
                &mut Scene::new(),
                &mut WithIconRenderer::new(&mut ListText::default(), &mut Icons),
                frame,
                Theme::LIGHT.into(),
            )
            .unwrap();
        let search = sidebar.host().get(ComponentId(10)).unwrap().frame();
        let back = sidebar.back_button_frame().unwrap();
        assert_eq!(
            back.min_x(),
            frame.min_x() + tekes_ui_tokens::Spacing::Loose.value()
        );
        assert_eq!(
            search.max_x(),
            frame.max_x() - tekes_ui_tokens::Spacing::Loose.value()
        );
        assert_eq!(search.center().y, back.center().y);
        assert_eq!(sidebar.back_button_mount(), restored_back_mount);
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
            &mut ListText::default(),
            Rect::new(0., 0., 260., 500.),
            Theme::LIGHT.into(),
        )
        .unwrap();
    let list = sidebar
        .host()
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
    sidebar.with_host_mut(|host| {
        host.event(&ContentEvent::PointerDown(point));
        host.event(&ContentEvent::PointerUp(point));
    });
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
            &mut ListText::default(),
            Rect::new(0., 0., 260., 500.),
            Theme::LIGHT.into(),
        )
        .unwrap();
    let list = sidebar
        .host()
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
    sidebar.with_host_mut(|host| {
        host.accessibility_action(ComponentId(103), Some(part), AccessibilityAction::Press);
        snapshot.display_items = vec![ListItemKey(3)];
        host.replace_keyed_list(ComponentId(103), &snapshot, &mut rows, None)
            .unwrap();
    });
    assert_eq!(
        sidebar.take_selection(),
        None,
        "hidden route cannot become the item now at its old display position"
    );
}

#[test]
fn grouped_form_keeps_rows_contiguous_and_separates_schema_groups() {
    use tekes_ui_controls::composition::{ComponentId, StackContent};
    use tekes_ui_scene::{CornerRadii, Primitive, Rect, Scene};
    use tekes_ui_tokens::{Palette, Spacing, Theme};
    // Three real controls: two in one schema group, one in the next, keeping real labels intact.
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
    let mount = form.host().semantic_mount(ComponentId(100));
    let mut text = ListText::default();
    for width in [260., 900.] {
        for theme in [Theme::LIGHT, Theme::DARK] {
            let mut scene = Scene::new();
            let palette = Palette::from(theme);
            form.render(
                &mut scene,
                &mut text,
                Rect::new(0., 0., width, 700.),
                palette,
            )
            .unwrap();
            let rows: Vec<_> = (0..3)
                .map(|i| {
                    form.host()
                        .get(ComponentId((1_u64 << 32) + i * 4))
                        .unwrap()
                        .frame()
                })
                .collect();
            let first_heading = form.host().get(ComponentId(1_u64 << 50)).unwrap().frame();
            let second_heading = form
                .host()
                .get(ComponentId((1_u64 << 50) + 1))
                .unwrap()
                .frame();
            assert_eq!(
                rows[0].max_y(),
                rows[1].min_y(),
                "one continuous group ground"
            );
            assert_eq!(
                rows[0].min_y() - first_heading.max_y(),
                StackContent::SECTION_HEADER_GAP
            );
            assert_eq!(
                second_heading.min_y() - rows[1].max_y(),
                StackContent::PANEL_SECTION_GAP
            );
            assert_eq!(
                rows[2].min_y() - second_heading.max_y(),
                StackContent::SECTION_HEADER_GAP
            );
            let body = form.host().get(ComponentId(11)).unwrap().frame();
            assert_eq!(body.size.width, width.min(560.));
            assert_eq!(
                body.size.height, 700.,
                "ScrollContent owns document height without 390pt cap"
            );
            assert_eq!(body.min_x(), (width - body.size.width) * 0.5);
            let content = form.host().get(ComponentId(20)).unwrap().frame();
            let inset = Spacing::Loose.value();
            assert_eq!(rows[0].size.width, content.size.width);
            assert_eq!(rows[0].min_x(), body.min_x() + inset);
            assert_eq!(body.max_x() - rows[0].max_x(), inset);
            assert_eq!(first_heading.min_y(), body.min_y() + inset);
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
            assert_eq!(form.host().semantic_mount(ComponentId(100)), mount);
        }
    }
}

#[test]
fn full_production_settings_form_preserves_long_labels_scaling_and_horizontal_save_error() {
    use tekes_ui_controls::composition::{ComponentId, TextContent, TextFieldContent};
    use tekes_ui_scene::{Rect, Scene};
    use tekes_ui_tokens::{Palette, Spacing, Theme};
    let mut def = schema();
    def.settings[0].label =
        "Long wrapping production workspace title setting that must fit narrow panels cleanly"
            .into();
    let draft = Draft::new(Snapshot::default());
    let mut form = SettingsForm::new_autosave(&def, &draft).unwrap();
    form.save_error(Some("Failed to persist settings snapshot"), false);

    for scale in [1.0, 1.5] {
        let mut text = ListText {
            type_scale: scale,
            draws: Vec::new(),
        };
        for width in [260., 560.] {
            let viewport = Rect::new(0., 0., width, 700.);
            form.render(
                &mut Scene::new(),
                &mut text,
                viewport,
                Palette::from(Theme::LIGHT),
            )
            .unwrap();

            let heading0 = form.host().get(ComponentId(1_u64 << 50)).unwrap().frame();
            let heading1 = form
                .host()
                .get(ComponentId((1_u64 << 50) + 1))
                .unwrap()
                .frame();
            assert_eq!(
                form.host()
                    .get(ComponentId(1_u64 << 50))
                    .unwrap()
                    .component::<TextContent>()
                    .unwrap()
                    .text(),
                "工作区"
            );
            assert_eq!(
                form.host()
                    .get(ComponentId((1_u64 << 50) + 1))
                    .unwrap()
                    .component::<TextContent>()
                    .unwrap()
                    .text(),
                "阅读"
            );
            let row0 = form.host().get(ComponentId(1_u64 << 32)).unwrap().frame();
            let row1 = form
                .host()
                .get(ComponentId((1_u64 << 32) + 4))
                .unwrap()
                .frame();
            let row2 = form
                .host()
                .get(ComponentId((1_u64 << 32) + 8))
                .unwrap()
                .frame();
            let label0 = form
                .host()
                .get(ComponentId((1_u64 << 32) + 1))
                .unwrap()
                .frame();
            let control0 = form.host().get(ComponentId(100)).unwrap().frame();
            let block = Spacing::Tight.value();
            let intra_content_gap = (row1.min_y() + block) - (row0.max_y() - block);
            let inter_content_gap = (row2.min_y() + block) - (row1.max_y() - block);
            assert!(
                inter_content_gap > intra_content_gap,
                "inter-group content gap ({inter_content_gap}) must exceed intra-group ({intra_content_gap})"
            );
            assert!(label0.max_x() <= control0.min_x());
            assert!(label0.min_x() >= row0.min_x() + Spacing::Loose.value());
            assert!(control0.max_x() <= row0.max_x() - Spacing::Loose.value());
            assert!(label0.max_y() <= row0.max_y() - block);
            assert!(heading0.max_x() <= viewport.max_x() && heading1.max_x() <= viewport.max_x());

            let status_frame = form.host().get(ComponentId(2)).unwrap().frame();
            let apply_frame = form.host().get(ComponentId(5)).unwrap().frame();
            let discard_frame = form.host().get(ComponentId(6)).unwrap().frame();
            assert_eq!(apply_frame.min_y(), discard_frame.min_y());
            assert!(apply_frame.max_x() < discard_frame.min_x());
            assert!(status_frame.max_y() < apply_frame.min_y());
            let error_ink = tekes_ui_tokens::surface_palette::resolve_surface_with_options(
                Palette::from(Theme::LIGHT),
                "canvas",
                "input",
                "rest",
                tekes_ui_tokens::surface_palette::SurfaceOptions {
                    error: true,
                    read_only: false,
                },
            )
            .unwrap()
            .helper
            .unwrap();
            assert!(
                text.draws
                    .contains(&("Failed to persist settings snapshot".into(), error_ink)),
                "autosave error status must resolve canonical error helper ink"
            );
        }
    }

    // replace_values preserves help text, presentation, trailing alignment and mount, and refuses active IME.
    let mount = form.host().semantic_mount(ComponentId(100));
    assert!(form.focus(Some(ComponentId(100)), false));
    form.with_host_mut(|host| {
        let tf = host
            .get_mut(ComponentId(100))
            .unwrap()
            .component_mut::<TextFieldContent>()
            .unwrap();
        assert!(tf.set_composition_preview("中", 1..1, None, std::iter::once(0..1).collect()));
        let blocked = tf.replace_value(
            "Overwritten",
            tekes_ui_controls::ExternalSelection::Reset,
            tekes_ui_controls::ExternalUndo::Reset,
        );
        assert_eq!(
            blocked,
            tekes_ui_controls::ExternalValueResult::Conflict(
                tekes_ui_controls::ExternalValueConflict::Composition
            )
        );
    });
    form.replace_values(&def, &draft);
    let tf = form
        .host()
        .get(ComponentId(100))
        .unwrap()
        .component::<TextFieldContent>()
        .unwrap();
    assert_eq!(tf.text(), "My Workspace");
    assert_eq!(
        tf.surface_presentation(),
        tekes_ui_controls::TextSurfacePresentation::Content
    );
    assert_eq!(
        tf.alignment(),
        tekes_ui_controls::TextSurfaceAlignment::Trailing
    );
    assert_eq!(form.host().semantic_mount(ComponentId(100)), mount);
}
