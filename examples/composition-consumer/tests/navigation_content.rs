//! Navigation adapters through their public Content API and real ContentHost.
use std::{cell::Cell, rc::Rc};
use tekes_ui_composition_consumer::navigation_content::{
    self, BACKGROUND_ACTION, BREADCRUMB, COMMAND, COMMAND_QUERY, COMPOSER, FIRST_PANEL, ROOT,
    SECOND_PANEL, TABS,
};
use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, ControlKey, TextSurfaceHost, TextSurfaceTextRenderer,
    composition::*,
    extended::{Axis, Stack},
    layout::MeasuredChild,
};
use tekes_ui_scene::{Point, Rect, Scene, Size};
use tekes_ui_text::{
    coretext::{CoreTextFonts, CoreTextShaper},
    model::{Paragraph, ShapeOptions},
    style::TextStyle,
    traits::Shaper,
};
use tekes_ui_tokens::{Palette, Rgba, Theme, proposal};

struct Text {
    shaper: CoreTextShaper,
    drawn: Vec<String>,
    records: Vec<(String, Rect, Rgba)>,
    scale: f32,
}
impl Default for Text {
    fn default() -> Self {
        Self {
            shaper: CoreTextShaper::new(CoreTextFonts::new()),
            drawn: Vec::new(),
            records: Vec::new(),
            scale: 1.,
        }
    }
}
impl ButtonTextRenderer for Text {
    fn resolved_font_size(&mut self, role: proposal::TextRole) -> f32 {
        role.style().size * self.scale
    }
    fn text_surface_host(&mut self) -> Option<&mut dyn TextSurfaceHost> {
        Some(self)
    }
    fn measure(&mut self, value: &str, role: proposal::TextRole, width: f32) -> Size {
        if width <= 0.0 || !width.is_finite() {
            return Size::default();
        }
        let p = self.shaper.shape(
            value,
            &TextStyle::system(role.style().size * self.scale).with_weight(role.style().weight),
            &ShapeOptions {
                width: Some(f64::from(width)),
                line_spacing: 0.,
            },
        );
        Size::new(p.width as f32, p.height as f32)
    }
    fn draw(
        &mut self,
        _: &mut Scene,
        value: &str,
        _: proposal::TextRole,
        frame: Rect,
        _: ButtonTextAlign,
        color: Rgba,
    ) {
        self.drawn.push(value.to_owned());
        self.records.push((value.to_owned(), frame, color));
    }
}
impl TextSurfaceHost for Text {
    fn shape_surface(
        &mut self,
        value: &str,
        role: proposal::TextRole,
        width: Option<f32>,
    ) -> Rc<Paragraph> {
        Rc::new(self.shaper.shape(
            value,
            &TextStyle::system(role.style().size * self.scale).with_weight(role.style().weight),
            &ShapeOptions {
                width: width.map(f64::from),
                line_spacing: 0.0,
            },
        ))
    }
}
impl TextSurfaceTextRenderer for Text {
    fn push_visible(&mut self, _: &mut Scene, _: &Paragraph, _: Point, _: Rgba, _: Rect) {}
}

fn render(host: &mut ContentHost, width: f32) -> Scene {
    let frame = Rect::new(20.0, 20.0, width, 180.0);
    let mut scene = Scene::new();
    host.render(
        &mut scene,
        &mut Text::default(),
        frame,
        frame,
        Theme::LIGHT.into(),
    )
    .unwrap();
    scene
}

fn breadcrumb_parts(host: &ContentHost) -> Vec<SemanticPart> {
    host.semantics()
        .into_iter()
        .find(|node| node.id == BREADCRUMB)
        .expect("breadcrumb owns one semantic group")
        .parts
}

#[test]
fn breadcrumb_publishes_ancestor_links_and_captures_original_path_for_each_input_source() {
    let mut host = navigation_content::breadcrumb(&["Home", "Library", "Current"]).unwrap();
    render(&mut host, 520.0);

    let parts = breadcrumb_parts(&host);
    assert_eq!(parts.len(), 3);
    assert_eq!(parts[0].semantics.role, ContentRole::Link);
    assert_eq!(parts[0].semantics.name, "Home");
    assert!(!parts[0].semantics.disabled);
    assert_eq!(parts[2].semantics.role, ContentRole::Text);
    assert!(parts[2].semantics.disabled);
    assert!(parts[2].semantics.selected);

    assert!(
        host.accessibility_action(BREADCRUMB, Some(1), AccessibilityAction::Press)
            .activated
    );
    let action = host.drain_action_records().pop().expect("one AX action");
    assert_eq!(action.action.source, ContentActionSource::Accessibility);
    assert_eq!(
        action.payload,
        Some(ContentActionPayload::BreadcrumbNavigation {
            index: 1,
            path: vec!["Home".into(), "Library".into(), "Current".into()],
        })
    );
    assert!(
        !host
            .accessibility_action(BREADCRUMB, Some(2), AccessibilityAction::Press)
            .activated
    );
    assert!(host.drain_action_records().is_empty());

    let point = breadcrumb_parts(&host)[0].frame.center();
    assert!(
        host.event(&ContentEvent::PointerDown(point))
            .capture_pointer
    );
    assert!(host.event(&ContentEvent::PointerUp(point)).activated);
    let action = host
        .drain_action_records()
        .pop()
        .expect("one pointer action");
    assert_eq!(action.action.source, ContentActionSource::Pointer);
    assert_eq!(
        action.payload,
        Some(ContentActionPayload::BreadcrumbNavigation {
            index: 0,
            path: vec!["Home".into(), "Library".into(), "Current".into()],
        })
    );

    host.focus(Some(BREADCRUMB), true);
    assert!(
        host.event(&ContentEvent::Key {
            key: tekes_ui_controls::ControlKey::Enter,
            down: true
        })
        .activated
    );
    let action = host
        .drain_action_records()
        .pop()
        .expect("one keyboard action");
    assert_eq!(action.action.source, ContentActionSource::Keyboard);
    assert!(matches!(
        action.payload,
        Some(ContentActionPayload::BreadcrumbNavigation { .. })
    ));
    host.focus(Some(BREADCRUMB), true);
    assert_eq!(host.focused(), Some(BREADCRUMB));
    host.set_focus_wrap(false);
    assert!(
        host.event(&ContentEvent::Key {
            key: ControlKey::Tab { reverse: false },
            down: true
        })
        .consumed,
        "Tab first advances within the breadcrumb links"
    );
    assert!(
        !host
            .event(&ContentEvent::Key {
                key: ControlKey::Tab { reverse: false },
                down: true
            })
            .consumed,
        "the last breadcrumb link yields ordinary Tab traversal"
    );
}

#[test]
fn breadcrumb_rejects_stale_path_gestures_and_accessibility_parts() {
    let mut host = navigation_content::breadcrumb(&["Home", "Library", "Current"]).unwrap();
    render(&mut host, 520.0);
    let point = breadcrumb_parts(&host)[0].frame.center();
    assert!(
        host.event(&ContentEvent::PointerDown(point))
            .capture_pointer
    );
    host.get_mut(BREADCRUMB)
        .unwrap()
        .component_mut::<BreadcrumbContent>()
        .unwrap()
        .model
        .items = vec!["Home".into(), "New branch".into(), "Current".into()];
    assert!(!host.event(&ContentEvent::PointerUp(point)).activated);
    assert!(host.drain_action_records().is_empty());
    assert!(
        breadcrumb_parts(&host).is_empty(),
        "old frames cannot publish semantics for edited path"
    );
    assert!(
        !host
            .accessibility_action(BREADCRUMB, Some(0), AccessibilityAction::Press)
            .activated
    );
    assert!(host.drain_action_records().is_empty());
    render(&mut host, 520.0);
    assert_eq!(breadcrumb_parts(&host).len(), 3);
    assert!(
        host.accessibility_action(BREADCRUMB, Some(0), AccessibilityAction::Press)
            .activated
    );
    assert!(matches!(
        host.drain_action_records()[0].payload,
        Some(ContentActionPayload::BreadcrumbNavigation { ref path, .. }) if path[1] == "New branch"
    ));
}

#[test]
fn breadcrumb_geometry_remains_bounded_and_visible_when_labels_wrap_in_a_narrow_host() {
    let mut host = navigation_content::breadcrumb(&[
        "A very long workspace name",
        "A nested project with a long title",
        "Current",
    ])
    .unwrap();
    render(&mut host, 110.0);
    let node = host
        .semantics()
        .into_iter()
        .find(|node| node.id == BREADCRUMB)
        .unwrap();
    assert_eq!(node.parts.len(), 3);
    assert!(node.parts.iter().all(|part| !part.frame.is_empty()));
    assert!(node.parts.iter().all(|part| part.frame.size.height >= 20.0));
    assert!(node.visible.is_some());
    assert!(host.focus_targets().iter().any(|(id, _)| *id == BREADCRUMB));
}

#[test]
fn tabs_render_real_nested_labels_and_only_the_selected_retained_panel() {
    let mut host = navigation_content::tabs().unwrap();
    let scene = render(&mut host, 460.0);
    assert!(!scene.is_empty());
    assert!(host.get(FIRST_PANEL).is_some());
    assert!(host.get(SECOND_PANEL).is_some());
    assert!(host.get(COMPOSER).is_some());
    assert_eq!(
        host.get(TABS)
            .unwrap()
            .component::<TabsContent>()
            .unwrap()
            .selected(),
        0
    );

    let nodes = host.semantics();
    let tabs = nodes.iter().find(|node| node.id == TABS).unwrap();
    assert_eq!(tabs.semantics.role, ContentRole::TabList);
    assert_eq!(tabs.parts.len(), 2);
    assert_eq!(tabs.parts[0].semantics.name, "Overview Welcome");
    assert_eq!(tabs.parts[0].semantics.role, ContentRole::Tab);
    assert!(tabs.parts[0].semantics.selected);
    assert_eq!(tabs.parts[1].semantics.name, "Settings");
    assert!(!tabs.parts[1].semantics.selected);
    assert!(
        nodes.iter().all(|node| node.id != ComponentId(9_811)
            && node.id != ComponentId(9_812)
            && node.id != ComponentId(9_813)),
        "nested label Content is decorative"
    );
    assert!(
        nodes.iter().any(|node| node.id == COMPOSER),
        "selected panel remains in the real semantic tree"
    );

    assert!(
        host.accessibility_action(TABS, Some(1), AccessibilityAction::Press)
            .activated
    );
    let record = host.drain_action_records().pop().unwrap();
    assert_eq!(record.action.source, ContentActionSource::Accessibility);
    assert_eq!(
        record.payload,
        Some(ContentActionPayload::TabSelection {
            index: 1,
            label: ComponentId(9_813),
            panel: SECOND_PANEL,
        })
    );
    let active_mount = host
        .semantic_mount(SECOND_PANEL)
        .expect("selected panel is active");
    assert!(
        host.get_mut(TABS)
            .unwrap()
            .component_mut::<TabsContent>()
            .unwrap()
            .select(0)
    );
    assert_eq!(
        host.semantic_mount(SECOND_PANEL),
        None,
        "inactive panel is outside semantic traversal"
    );
    assert!(
        host.get_mut(TABS)
            .unwrap()
            .component_mut::<TabsContent>()
            .unwrap()
            .select(1)
    );
    assert_eq!(
        host.semantic_mount(SECOND_PANEL),
        Some(active_mount),
        "the same panel mount returns when reactivated"
    );
    render(&mut host, 460.0);
    let visible = host.semantics();
    assert!(visible.iter().any(|node| node.id == SECOND_PANEL));
    assert!(visible.iter().all(|node| node.id != COMPOSER));
    assert_eq!(
        host.focus_targets()
            .iter()
            .filter(|(id, _)| *id == COMPOSER)
            .count(),
        0
    );
    host.focus(Some(TABS), true);
    host.event(&ContentEvent::Key {
        key: ControlKey::Tab { reverse: false },
        down: true,
    });
    assert_ne!(
        host.focused(),
        Some(COMPOSER),
        "Tab cannot reach an inactive panel editor"
    );
}

#[test]
fn tabs_pointer_keyboard_and_ax_share_selection_and_action_identity() {
    let mut host = navigation_content::tabs().unwrap();
    render(&mut host, 460.0);
    let second = host
        .semantics()
        .into_iter()
        .find(|node| node.id == TABS)
        .unwrap()
        .parts[1]
        .frame
        .center();
    assert!(
        host.event(&ContentEvent::PointerDown(second))
            .capture_pointer
    );
    assert!(host.event(&ContentEvent::PointerUp(second)).activated);
    let pointer = host.drain_action_records().pop().unwrap();
    assert_eq!(pointer.action.source, ContentActionSource::Pointer);
    assert_eq!(
        pointer.payload,
        Some(ContentActionPayload::TabSelection {
            index: 1,
            label: ComponentId(9_813),
            panel: SECOND_PANEL,
        })
    );

    host.focus(Some(TABS), true);
    assert!(
        host.event(&ContentEvent::Key {
            key: ControlKey::Left,
            down: true
        })
        .activated
    );
    let keyboard = host.drain_action_records().pop().unwrap();
    assert_eq!(keyboard.action.source, ContentActionSource::Keyboard);
    assert_eq!(
        keyboard.payload,
        Some(ContentActionPayload::TabSelection {
            index: 0,
            label: ComponentId(9_810),
            panel: FIRST_PANEL,
        })
    );
    assert_eq!(
        host.get(TABS)
            .unwrap()
            .component::<TabsContent>()
            .unwrap()
            .selected(),
        0
    );
}

#[test]
fn tabs_keep_inactive_panel_state_and_reject_selection_while_its_editor_composes() {
    let mut host = navigation_content::tabs().unwrap();
    render(&mut host, 460.0);
    host.get_mut(FIRST_PANEL)
        .unwrap()
        .set_surface(Some("panel"))
        .unwrap();
    host.focus(Some(COMPOSER), false);
    host.event(&ContentEvent::Preedit {
        text: "draft".into(),
        selection: 0..5,
        clauses: vec![0..5],
        replacement: None,
    });
    render(&mut host, 460.0);
    assert!(host.is_composing());
    let mount = host.semantic_mount(FIRST_PANEL);
    assert!(
        !host
            .get_mut(TABS)
            .unwrap()
            .component_mut::<TabsContent>()
            .unwrap()
            .select(1)
    );
    assert_eq!(
        host.get(TABS)
            .unwrap()
            .component::<TabsContent>()
            .unwrap()
            .selected(),
        0
    );
    assert!(host.is_composing());
    assert_eq!(host.semantic_mount(FIRST_PANEL), mount);
    assert!(host.focus_targets().iter().any(|(id, _)| *id == COMPOSER));
}

#[test]
fn tabs_disabled_entries_reject_ax_pointer_and_programmatic_selection() {
    let mut host = navigation_content::tabs().unwrap();
    render(&mut host, 460.0);
    assert!(
        host.get_mut(TABS)
            .unwrap()
            .component_mut::<TabsContent>()
            .unwrap()
            .set_tab_disabled(1, true)
    );
    let tabs = host
        .semantics()
        .into_iter()
        .find(|node| node.id == TABS)
        .unwrap();
    assert!(tabs.parts[1].semantics.disabled);
    assert!(
        !host
            .accessibility_action(TABS, Some(1), AccessibilityAction::Press)
            .activated
    );
    let point = tabs.parts[1].frame.center();
    assert!(
        !host
            .event(&ContentEvent::PointerDown(point))
            .capture_pointer
    );
    assert!(
        !host
            .get_mut(TABS)
            .unwrap()
            .component_mut::<TabsContent>()
            .unwrap()
            .select(1)
    );
    assert_eq!(
        host.get(TABS)
            .unwrap()
            .component::<TabsContent>()
            .unwrap()
            .selected(),
        0
    );
    assert!(host.drain_action_records().is_empty());
    host.get_mut(TABS)
        .unwrap()
        .component_mut::<TabsContent>()
        .unwrap()
        .set_disabled(true);
    assert!(
        host.semantics()
            .iter()
            .find(|node| node.id == TABS)
            .unwrap()
            .semantics
            .disabled
    );
}

#[test]
fn tabs_reorder_moves_complete_content_pairs_without_changing_mounts() {
    let mut host = navigation_content::tabs().unwrap();
    render(&mut host, 460.0);
    assert!(
        host.get_mut(TABS)
            .unwrap()
            .component_mut::<TabsContent>()
            .unwrap()
            .select(1)
    );
    render(&mut host, 460.0);
    let label_mount = host.semantic_mount(ComponentId(9_813));
    let panel_mount = host
        .semantic_mount(SECOND_PANEL)
        .expect("selected panel has an active mount");
    assert!(
        !host
            .get_mut(TABS)
            .unwrap()
            .component_mut::<TabsContent>()
            .unwrap()
            .reorder(&[0, 0])
    );
    assert!(
        host.get_mut(TABS)
            .unwrap()
            .component_mut::<TabsContent>()
            .unwrap()
            .reorder(&[1, 0])
    );
    let tabs = host.get(TABS).unwrap().component::<TabsContent>().unwrap();
    assert_eq!(
        tabs.selected(),
        0,
        "the originally selected panel follows its item"
    );
    assert_eq!(host.semantic_mount(ComponentId(9_813)), label_mount);
    assert_eq!(host.semantic_mount(SECOND_PANEL), Some(panel_mount));
    assert!(host.get(SECOND_PANEL).is_some());
    assert!(host.get(FIRST_PANEL).is_some());
}

#[derive(Debug)]
struct OverlayProbe {
    paints: Rc<Cell<usize>>,
    overlay_paints: Rc<Cell<usize>>,
}
impl Component for OverlayProbe {
    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, _: Size) -> MeasuredChild {
        MeasuredChild::new(Size::new(80.0, 28.0))
    }
    fn semantics(&self) -> ContentSemantics {
        ContentSemantics::new(ContentRole::Group, "Overlay probe")
    }
    fn paint(
        &mut self,
        _: &mut Scene,
        _: &mut dyn ButtonTextRenderer,
        _: Rect,
        _: Rect,
        _: ContentTheme,
    ) {
        self.paints.set(self.paints.get() + 1);
    }
    fn overlay_presented(&self) -> bool {
        true
    }
    fn paints_children(&self) -> bool {
        true
    }
    fn paint_overlay(
        &mut self,
        _: &mut Scene,
        _: &mut dyn ButtonTextRenderer,
        _: Rect,
        _: Rect,
        _: ContentTheme,
    ) {
        self.overlay_paints.set(self.overlay_paints.get() + 1);
    }
}

#[test]
fn inactive_tabs_panel_cannot_leak_retained_overlay_paint() {
    let panel_paints = Rc::new(Cell::new(0));
    let overlay_paints = Rc::new(Cell::new(0));
    let first = TabItem::new(
        Content::new(ComponentId(9_840), TextContent::new("One")),
        Content::new(
            ComponentId(9_841),
            OverlayProbe {
                paints: panel_paints.clone(),
                overlay_paints: overlay_paints.clone(),
            },
        ),
    );
    let second = TabItem::new(
        Content::new(ComponentId(9_842), TextContent::new("Two")),
        Content::new(ComponentId(9_843), TextContent::new("Other panel")),
    );
    let mut host = ContentHost::new(Content::new(
        ROOT,
        StackContent {
            model: Stack {
                axis: Axis::Vertical,
                gap: 0.0,
            },
            children: vec![Content::new(TABS, TabsContent::new(vec![first, second], 1))],
            flexible_child: None,
        },
    ))
    .unwrap();
    render(&mut host, 400.0);
    assert_eq!(
        panel_paints.get(),
        0,
        "inactive panel Content must not paint"
    );
    assert_eq!(overlay_paints.get(), 0);
    let mut scene = Scene::new();
    host.get_mut(ROOT).unwrap().paint_overlays(
        &mut scene,
        &mut Text::default(),
        ContentTheme::from(Palette::from(Theme::LIGHT)),
    );
    assert_eq!(
        overlay_paints.get(),
        0,
        "inactive retained overlay bypassed the selection gate"
    );
    assert_eq!(panel_paints.get(), 0);
    assert!(
        !host
            .semantics()
            .iter()
            .any(|node| node.id == ComponentId(9_841))
    );
}

#[test]
fn command_palette_keeps_original_query_mounted_and_uses_filtered_menu_semantics() {
    let mut host = navigation_content::command_palette().unwrap();
    render(&mut host, 420.0);
    let closed = host.semantics();
    let command = closed.iter().find(|node| node.id == COMMAND).unwrap();
    assert_eq!(command.semantics.expanded, Some(false));
    assert!(
        command
            .parts
            .iter()
            .any(|part| part.semantics.role == ContentRole::MenuButton)
    );
    assert!(closed.iter().all(|node| node.id != COMMAND_QUERY));
    assert!(
        host.focus_targets()
            .iter()
            .all(|(id, _)| *id != COMMAND_QUERY)
    );

    assert!(host.focus(Some(BACKGROUND_ACTION), true));
    assert!(
        host.focus(Some(COMMAND), true),
        "palette trigger owner can receive focus before opening"
    );
    assert!(
        host.get_mut(COMMAND)
            .unwrap()
            .component_mut::<CommandPaletteContent>()
            .unwrap()
            .open()
    );
    render(&mut host, 420.0);
    let query_mount = host
        .semantic_mount(COMMAND_QUERY)
        .expect("opening activates the original query");
    assert!(host.semantics().iter().any(|node| node.id == COMMAND_QUERY));
    assert!(
        host.focus_targets()
            .iter()
            .any(|(id, _)| *id == COMMAND_QUERY)
    );
    assert_eq!(host.focused(), Some(COMMAND_QUERY));
    assert!(!host.focus(Some(BACKGROUND_ACTION), true));
    host.focus(Some(COMMAND_QUERY), false);
    assert_eq!(host.focused(), Some(COMMAND_QUERY));
    assert!(
        host.event(&ContentEvent::Preedit {
            text: "pref".into(),
            selection: 0..4,
            clauses: vec![0..4],
            replacement: None,
        })
        .changed
    );
    render(&mut host, 420.0);
    let command = host
        .get(COMMAND)
        .unwrap()
        .component::<CommandPaletteContent>()
        .unwrap();
    assert_eq!(command.model.combo.query, "pref");
    let menu_items = host
        .semantics()
        .into_iter()
        .find(|node| node.id == COMMAND)
        .unwrap()
        .parts
        .into_iter()
        .filter(|part| part.semantics.role == ContentRole::MenuItem)
        .collect::<Vec<_>>();
    assert_eq!(menu_items.len(), 1);
    assert_eq!(
        menu_items[0].index, 2,
        "filtered semantics retain original command index"
    );
    assert_eq!(menu_items[0].semantics.name, "Preferences");
    assert!(
        !host
            .get_mut(COMMAND)
            .unwrap()
            .component_mut::<CommandPaletteContent>()
            .unwrap()
            .close()
    );
    assert!(host.is_composing());
    assert!(
        !host
            .accessibility_action(COMMAND, Some(2), AccessibilityAction::Press)
            .activated
    );
    assert!(
        !host
            .event(&ContentEvent::Key {
                key: ControlKey::Enter,
                down: true
            })
            .activated
    );
    assert!(host.event(&ContentEvent::Unmark).changed);
    assert!(
        host.accessibility_action(COMMAND, Some(2), AccessibilityAction::Press)
            .activated
    );
    let actions = host.drain_action_records();
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0].action.source, ContentActionSource::Accessibility);
    assert_eq!(
        actions[0].payload,
        Some(ContentActionPayload::CommandSelection {
            index: 2,
            label: "Preferences".into()
        })
    );
    let command = host
        .get(COMMAND)
        .unwrap()
        .component::<CommandPaletteContent>()
        .unwrap();
    assert_eq!(command.model.combo.selected, Some(2));
    assert!(!command.is_open());
    assert_eq!(
        host.semantic_mount(COMMAND_QUERY),
        None,
        "closed query is outside active semantic scope"
    );
    assert!(
        host.get_mut(COMMAND)
            .unwrap()
            .component_mut::<CommandPaletteContent>()
            .unwrap()
            .open()
    );
    render(&mut host, 420.0);
    assert_eq!(
        host.semantic_mount(COMMAND_QUERY),
        Some(query_mount),
        "reopen retains the original editor mount"
    );
    assert!(
        host.get_mut(COMMAND)
            .unwrap()
            .component_mut::<CommandPaletteContent>()
            .unwrap()
            .close()
    );
    render(&mut host, 420.0);
    assert_eq!(host.semantic_mount(COMMAND_QUERY), None);
    assert!(
        host.focus_targets()
            .iter()
            .all(|(id, _)| *id != COMMAND_QUERY)
    );
    assert_eq!(
        host.focused(),
        Some(COMMAND),
        "closing restores the palette trigger owner"
    );
}

#[derive(Debug)]
struct SurfaceProbe(Rc<std::cell::RefCell<Vec<&'static str>>>);
impl Component for SurfaceProbe {
    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, _: Size) -> MeasuredChild {
        MeasuredChild::new(Size::new(120.0, 28.0))
    }
    fn paint(
        &mut self,
        _: &mut Scene,
        _: &mut dyn ButtonTextRenderer,
        _: Rect,
        _: Rect,
        theme: ContentTheme,
    ) {
        self.0.borrow_mut().push(theme.surface());
    }
}

#[test]
fn tabs_panel_inherits_and_clears_surface_without_remounting_content() {
    let surfaces = Rc::new(std::cell::RefCell::new(Vec::new()));
    let host_id = ComponentId(9_860);
    let panel_id = ComponentId(9_861);
    let tabs = Content::new(
        TABS,
        TabsContent::new(
            vec![TabItem::new(
                Content::new(ComponentId(9_862), TextContent::new("Panel")),
                Content::new(panel_id, SurfaceProbe(surfaces.clone())),
            )],
            0,
        ),
    );
    let mut host = ContentHost::new(Content::new(
        host_id,
        StackContent {
            model: Stack {
                axis: Axis::Vertical,
                gap: 0.0,
            },
            children: vec![tabs],
            flexible_child: None,
        },
    ))
    .unwrap();
    let mount = host.semantic_mount(panel_id);
    host.set_inherited_surface("sidebar").unwrap();
    render(&mut host, 420.0);
    assert_eq!(surfaces.borrow().last(), Some(&"sidebar"));
    host.get_mut(panel_id)
        .unwrap()
        .set_surface(Some("panel"))
        .unwrap();
    render(&mut host, 420.0);
    assert_eq!(surfaces.borrow().last(), Some(&"panel"));
    host.get_mut(panel_id).unwrap().set_surface(None).unwrap();
    render(&mut host, 420.0);
    assert_eq!(surfaces.borrow().last(), Some(&"sidebar"));
    assert_eq!(host.semantic_mount(panel_id), mount);
}

#[test]
fn command_trigger_and_popup_pointer_capture_close_once_without_background_actions() {
    let mut host = navigation_content::command_palette().unwrap();
    render(&mut host, 420.);
    let trigger = host
        .get(COMMAND)
        .unwrap()
        .component::<CommandPaletteContent>()
        .unwrap()
        .view
        .trigger_layout()
        .unwrap()
        .frame
        .center();
    assert!(
        host.event(&ContentEvent::PointerDown(trigger))
            .capture_pointer
    );
    assert_eq!(host.pointer_capture().map(|c| c.0), Some(COMMAND));
    assert!(host.event(&ContentEvent::PointerUp(trigger)).changed);
    assert!(
        host.drain_actions().is_empty(),
        "opening does not execute a command"
    );
    render(&mut host, 420.);
    assert_eq!(host.focused(), Some(COMMAND_QUERY));
    let option = host
        .get(COMMAND)
        .unwrap()
        .component::<CommandPaletteContent>()
        .unwrap()
        .options()
        .layout()
        .unwrap()
        .items[1]
        .visible
        .unwrap()
        .center();
    assert!(
        host.event(&ContentEvent::PointerDown(option))
            .capture_pointer
    );
    assert_eq!(host.pointer_capture().map(|c| c.0), Some(COMMAND));
    assert!(host.event(&ContentEvent::PointerUp(option)).activated);
    let records = host.drain_action_records();
    assert_eq!(records.len(), 1);
    assert_eq!(
        records[0].payload,
        Some(ContentActionPayload::CommandSelection {
            index: 1,
            label: "Open Folder".into()
        })
    );
    assert_eq!(host.focused(), Some(COMMAND));
    assert!(
        host.event(&ContentEvent::Key {
            key: ControlKey::Enter,
            down: true
        })
        .changed
    );
    render(&mut host, 420.);
    assert!(host.drain_actions().is_empty());
    let background = host.get(BACKGROUND_ACTION).unwrap().frame().center();
    // The original query/menu may cover the background action; dismiss at an
    // actual point outside the popup, then release over the background.
    assert!(
        host.event(&ContentEvent::PointerDown(Point::new(-100., -100.)))
            .consumed
    );
    assert!(
        !host
            .get(COMMAND)
            .unwrap()
            .component::<CommandPaletteContent>()
            .unwrap()
            .is_open()
    );
    host.event(&ContentEvent::PointerUp(background));
    assert!(host.drain_actions().is_empty());
    assert_eq!(host.focused(), Some(COMMAND));
    assert!(
        !host
            .event(&ContentEvent::Key {
                key: ControlKey::Enter,
                down: false
            })
            .activated
    );
}

#[test]
fn command_geometry_capture_cancel_retains_query_and_window_loss_retires_it() {
    let mut host = navigation_content::command_palette().unwrap();
    render(&mut host, 420.);
    host.focus(Some(COMMAND), true);
    host.get_mut(COMMAND)
        .unwrap()
        .component_mut::<CommandPaletteContent>()
        .unwrap()
        .open();
    render(&mut host, 420.);
    let option = host
        .get(COMMAND)
        .unwrap()
        .component::<CommandPaletteContent>()
        .unwrap()
        .options()
        .layout()
        .unwrap()
        .items[0]
        .visible
        .unwrap()
        .center();
    host.event(&ContentEvent::PointerDown(option));
    assert!(host.pointer_capture().is_some());
    assert!(host.cancel_pointer_capture());
    assert!(
        host.get(COMMAND)
            .unwrap()
            .component::<CommandPaletteContent>()
            .unwrap()
            .is_open()
    );
    assert!(!host.event(&ContentEvent::PointerUp(option)).activated);
    assert!(host.drain_actions().is_empty());
    assert_eq!(host.focused(), Some(COMMAND_QUERY));
    let query_binding = host.native_key_binding().expect("original query binding");
    assert!(!query_binding.is_empty());
    assert!(
        host.event(&ContentEvent::Preedit {
            text: "Open".into(),
            selection: 1..3,
            clauses: vec![0..4],
            replacement: None,
        })
        .changed
    );
    assert!(host.is_composing(), "preedit must reach the original query");
    let input = &host.focused_text().unwrap().input;
    let marked = input
        .marked
        .clone()
        .expect("query must contain marked text");
    let utf16: Vec<_> = input.text.encode_utf16().collect();
    assert_eq!(String::from_utf16(&utf16[marked.clone()]).unwrap(), "Open");
    assert_eq!(input.selection(), 1..3);
    assert_eq!(
        host.native_key_binding().as_deref(),
        Some(query_binding.as_slice())
    );
    host.get_mut(COMMAND)
        .unwrap()
        .component_mut::<CommandPaletteContent>()
        .unwrap()
        .model
        .combo
        .items
        .push("Open Recent".into());
    render(&mut host, 420.);
    assert!(
        host.is_composing(),
        "candidate refresh must preserve preedit"
    );
    let input = &host.focused_text().unwrap().input;
    assert_eq!(input.marked, Some(marked.clone()));
    let utf16: Vec<_> = input.text.encode_utf16().collect();
    assert_eq!(String::from_utf16(&utf16[marked]).unwrap(), "Open");
    assert_eq!(input.selection(), 1..3);
    assert_eq!(
        host.native_key_binding().as_deref(),
        Some(query_binding.as_slice())
    );
    host.set_window_active(false);
    assert!(
        !host
            .get(COMMAND)
            .unwrap()
            .component::<CommandPaletteContent>()
            .unwrap()
            .is_open()
    );
    assert!(host.semantics().iter().all(|node| node.id != COMMAND_QUERY));
    host.set_window_active(true);
    render(&mut host, 420.);
    assert!(
        !host
            .get(COMMAND)
            .unwrap()
            .component::<CommandPaletteContent>()
            .unwrap()
            .is_open()
    );
}

#[test]
fn real_content_navigation_surfaces_inherit_override_and_clear() {
    use tekes_ui_tokens::surface_palette::resolve_surface;
    for theme in [Theme::LIGHT, Theme::DARK] {
        let palette = Palette::from(theme);
        let frame = Rect::new(20., 20., 480., 320.);
        let mut breadcrumb = navigation_content::breadcrumb(&["Home", "Current"]).unwrap();
        breadcrumb.set_inherited_surface("sidebar").unwrap();
        for surface in [None, Some("panel"), None] {
            breadcrumb
                .get_mut(BREADCRUMB)
                .unwrap()
                .set_surface(surface)
                .unwrap();
            let mut text = Text::default();
            breadcrumb
                .render(&mut Scene::new(), &mut text, frame, frame, palette)
                .unwrap();
            let expected = resolve_surface(
                palette,
                surface.unwrap_or("sidebar"),
                "action-quiet",
                "rest",
            )
            .unwrap()
            .text;
            assert_eq!(
                text.records.iter().find(|(s, _, _)| s == "Home").unwrap().2,
                expected
            );
        }
        let mut command = navigation_content::command_palette().unwrap();
        command.set_inherited_surface("sidebar").unwrap();
        for surface in [None, Some("panel"), None] {
            command
                .get_mut(COMMAND)
                .unwrap()
                .set_surface(surface)
                .unwrap();
            let mut text = Text::default();
            command
                .render(&mut Scene::new(), &mut text, frame, frame, palette)
                .unwrap();
            let expected = resolve_surface(
                palette,
                surface.unwrap_or("sidebar"),
                "action-primary",
                "rest",
            )
            .unwrap()
            .text;
            assert_eq!(
                text.records
                    .iter()
                    .find(|(s, _, _)| s == "Commands")
                    .unwrap()
                    .2,
                expected
            );
        }
        command.focus(Some(COMMAND), true);
        command
            .get_mut(COMMAND)
            .unwrap()
            .component_mut::<CommandPaletteContent>()
            .unwrap()
            .open();
        for surface in [None, Some("panel"), None] {
            command
                .get_mut(COMMAND_QUERY)
                .unwrap()
                .set_surface(surface)
                .unwrap();
            let mut text = Text::default();
            command
                .render(&mut Scene::new(), &mut text, frame, frame, palette)
                .unwrap();
            let expected = resolve_surface(
                palette,
                surface.unwrap_or("popover"),
                "control-label",
                "rest",
            )
            .unwrap()
            .text;
            assert_eq!(
                text.records
                    .iter()
                    .rfind(|(s, _, _)| s == "Filter commands")
                    .unwrap()
                    .2,
                expected,
                "theme={theme:?} query-surface={surface:?}"
            );
            assert_eq!(command.get(COMMAND_QUERY).unwrap().surface(), surface);
        }
    }
}

#[test]
fn tabs_large_type_and_narrow_geometry_keep_real_labels_above_the_panel() {
    let mut host = navigation_content::tabs().unwrap();
    let frame = Rect::new(20., 20., 240., 600.);
    let mut text = Text {
        scale: 2.,
        ..Default::default()
    };
    host.render(
        &mut Scene::new(),
        &mut text,
        frame,
        frame,
        Theme::LIGHT.into(),
    )
    .unwrap();
    let tabs = host.semantics().into_iter().find(|n| n.id == TABS).unwrap();
    assert_eq!(tabs.parts.len(), 2);
    assert!(tabs.parts[0].frame.size.height > 34.);
    let panel = host.get(FIRST_PANEL).unwrap().frame();
    assert!(
        tabs.parts
            .iter()
            .all(|part| part.frame.max_y() <= panel.min_y() + 0.01)
    );
    for (label, rect, _) in &text.records {
        if ["Overview", "Welcome", "Settings"].contains(&label.as_str()) {
            assert!(rect.max_y() <= panel.min_y() + 0.01);
            assert!(rect.min_x() >= frame.min_x() && rect.max_x() <= frame.max_x());
        }
    }
}

#[test]
fn navigation_owns_fixed_slots_and_rejects_interactive_tab_labels() {
    let mut host = navigation_content::command_palette().unwrap();
    assert!(host.remove(COMMAND_QUERY).is_err());
    assert!(host.get(COMMAND_QUERY).is_some());
    let invalid = Content::new(
        TABS,
        TabsContent::new(
            vec![TabItem::new(
                Content::new(ComponentId(9_990), Button::new("Nested action")),
                Content::new(ComponentId(9_991), TextContent::new("Panel")),
            )],
            0,
        ),
    );
    assert!(matches!(
        ContentHost::new(invalid),
        Err(CompositionError::NestedInteractive { .. })
    ));
}

#[test]
fn tabs_switch_retires_original_panel_button_capture_and_late_release() {
    const PANEL_ACTION: ComponentId = ComponentId(9_970);
    let mut host = ContentHost::new(Content::new(
        TABS,
        TabsContent::new(
            vec![
                TabItem::new(
                    Content::new(ComponentId(9_971), TextContent::new("Panel A")),
                    Content::new(
                        FIRST_PANEL,
                        StackContent {
                            model: Stack {
                                axis: Axis::Vertical,
                                gap: 0.,
                            },
                            children: vec![Content::new(
                                PANEL_ACTION,
                                Button::new("Original panel action"),
                            )],
                            flexible_child: None,
                        },
                    ),
                ),
                TabItem::new(
                    Content::new(ComponentId(9_972), TextContent::new("Panel B")),
                    Content::new(SECOND_PANEL, TextContent::new("Other panel")),
                ),
            ],
            0,
        ),
    ))
    .unwrap();
    host.set_action_events(true);
    render(&mut host, 460.);
    let panel_mount = host
        .semantic_mount(FIRST_PANEL)
        .expect("original panel mount");
    let action_mount = host
        .semantic_mount(PANEL_ACTION)
        .expect("original Button mount");
    let pressed_at = host.get(PANEL_ACTION).unwrap().frame().center();
    assert!(
        host.event(&ContentEvent::PointerDown(pressed_at))
            .capture_pointer
    );
    let capture = host
        .pointer_capture()
        .expect("original Button captures pointer-down");
    assert_eq!((capture.0, capture.1), (PANEL_ACTION, action_mount));
    assert_eq!(host.focused(), Some(PANEL_ACTION));
    assert!(
        host.get(PANEL_ACTION)
            .unwrap()
            .component::<Button>()
            .unwrap()
            .button
            .state
            .pressed
    );
    assert!(!host.is_composing());

    assert!(
        host.get_mut(TABS)
            .unwrap()
            .component_mut::<TabsContent>()
            .unwrap()
            .select(1)
    );
    render(&mut host, 460.);
    assert!(host.pointer_capture().is_none());
    assert_ne!(host.focused(), Some(PANEL_ACTION));
    assert!(
        host.focus_targets()
            .iter()
            .all(|(id, _)| *id != PANEL_ACTION)
    );
    assert!(
        host.semantics()
            .iter()
            .all(|node| node.id != FIRST_PANEL && node.id != PANEL_ACTION)
    );
    assert!(!host.focus(Some(PANEL_ACTION), true));
    assert!(
        !host
            .accessibility_action(PANEL_ACTION, None, AccessibilityAction::Press)
            .activated
    );
    assert!(
        !host
            .get(PANEL_ACTION)
            .unwrap()
            .component::<Button>()
            .unwrap()
            .button
            .state
            .pressed
    );
    assert!(!host.event(&ContentEvent::PointerUp(pressed_at)).activated);
    assert_eq!(
        host.get(PANEL_ACTION)
            .unwrap()
            .component::<Button>()
            .unwrap()
            .activations,
        0
    );
    assert!(host.drain_action_records().is_empty());

    assert!(
        host.get_mut(TABS)
            .unwrap()
            .component_mut::<TabsContent>()
            .unwrap()
            .select(0)
    );
    render(&mut host, 460.);
    assert_eq!(host.semantic_mount(FIRST_PANEL), Some(panel_mount));
    assert_eq!(host.semantic_mount(PANEL_ACTION), Some(action_mount));
    let returned_at = host.get(PANEL_ACTION).unwrap().frame().center();
    assert!(
        !host.event(&ContentEvent::PointerUp(returned_at)).activated,
        "returning to the panel must not revive its old arm"
    );
    assert!(host.drain_action_records().is_empty());
    assert!(
        host.event(&ContentEvent::PointerDown(returned_at))
            .capture_pointer
    );
    assert!(host.event(&ContentEvent::PointerUp(returned_at)).activated);
    assert!(host.pointer_capture().is_none());
    let records = host.drain_action_records();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].action.target, PANEL_ACTION);
    assert_eq!(records[0].action.source, ContentActionSource::Pointer);
    assert_eq!(records[0].mount, action_mount);
    assert_eq!(
        host.get(PANEL_ACTION)
            .unwrap()
            .component::<Button>()
            .unwrap()
            .activations,
        1
    );
    assert!(host.drain_action_records().is_empty());
}
