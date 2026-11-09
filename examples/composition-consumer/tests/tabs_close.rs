//! Controlled tab close uses original Content, action delivery and AX routing.
#[path = "support/coretext.rs"]
mod coretext;
use std::{cell::Cell, rc::Rc};
use tekes_ui_controls::{
    ButtonTextRenderer, ControlKey, composition::*, layout::MeasuredChild,
    layout_view::IconRenderer,
};
use tekes_ui_scene::{ImageId, Point, Primitive, Rect, Scene, Size};
use tekes_ui_tokens::{Rgba, Theme, proposal};

#[derive(Default)]
struct Icons {
    requests: Vec<(String, f32, Rgba)>,
    fail: bool,
}
impl IconRenderer for Icons {
    fn prepare_icon(
        &mut self,
        name: &str,
        size: f32,
        _: f32,
        tint: Rgba,
    ) -> Result<IconImage, IconRenderError> {
        let glyph = tekes_ui_icons::Glyph::named(name).expect("existing canonical close asset");
        assert_eq!(glyph.name, "tekes-remove-one");
        assert!(!glyph.svg.contains("<circle") && glyph.svg.contains("<path"));
        self.requests.push((name.into(), size, tint));
        if self.fail {
            return Err(IconRenderError::Upload);
        }
        Ok(IconImage::new(ImageId(229_900), Rc::new(())))
    }
    fn draw_icon(&mut self, _: &mut Scene, _: &str, _: Rect, _: Rgba) {
        panic!("Tabs must retain a prepared icon resource");
    }
}

const TABS: ComponentId = ComponentId(229_000);
fn label(index: usize) -> ComponentId {
    ComponentId(229_010 + index as u64 * 2)
}
fn panel(index: usize) -> ComponentId {
    ComponentId(229_011 + index as u64 * 2)
}
fn host(selected: usize) -> ContentHost {
    let items = ["One", "Two", "List"]
        .into_iter()
        .enumerate()
        .map(|(index, name)| {
            TabItem::new(
                Content::new(label(index), TextContent::new(name)),
                Content::new(panel(index), TextFieldContent::new(name, "retained draft")),
            )
            .with_closable(index != 2)
        })
        .collect();
    let mut host = ContentHost::new(Content::new(TABS, TabsContent::new(items, selected))).unwrap();
    host.set_action_events(true);
    host
}
fn render(host: &mut ContentHost) {
    render_icons(host, &mut Icons::default());
}
fn render_icons(host: &mut ContentHost, icons: &mut Icons) -> Scene {
    render_icons_on_theme(host, icons, Theme::LIGHT)
}
fn render_icons_on_theme(host: &mut ContentHost, icons: &mut Icons, theme: Theme) -> Scene {
    let mut text = coretext::Text::new(1.);
    text.services = true;
    text.reduced_motion = true;
    let frame = Rect::new(10., 10., 460., 200.);
    let mut scene = Scene::new();
    host.render(
        &mut scene,
        &mut WithIconRenderer::new(&mut text, icons),
        frame,
        frame,
        theme.into(),
    )
    .unwrap();
    scene
}
fn tabs(host: &mut ContentHost) -> &mut TabsContent {
    host.get_mut(TABS).unwrap().component_mut().unwrap()
}
fn close_point(host: &ContentHost, index: usize) -> Point {
    host.semantics()
        .into_iter()
        .find(|node| node.id == TABS)
        .unwrap()
        .parts
        .into_iter()
        .find(|part| part.index == 3 + index)
        .unwrap()
        .frame
        .center()
}
fn expected_close(index: usize) -> Option<ContentActionPayload> {
    Some(ContentActionPayload::TabCloseRequest {
        index,
        label: label(index),
        panel: panel(index),
    })
}

#[test]
fn transparent_cross_uses_canonical_artwork_centered_inside_the_original_hit_target() {
    let mut h = host(0);
    let mut icons = Icons::default();
    let scene = render_icons(&mut h, &mut icons);
    assert_eq!(icons.requests.len(), 2);
    assert!(
        icons
            .requests
            .iter()
            .all(|(name, size, _)| name == "tekes-remove-one"
                && *size == proposal::metric("size", "size-icon-standard").unwrap())
    );
    let frames: Vec<_> = scene
        .primitives()
        .iter()
        .filter_map(|primitive| match primitive {
            Primitive::Image(image) if image.image == ImageId(229_900) => Some(image.rect),
            _ => None,
        })
        .collect();
    assert_eq!(frames.len(), 2);
    let parts = h
        .semantics()
        .into_iter()
        .find(|node| node.id == TABS)
        .unwrap()
        .parts;
    for (index, artboard) in frames.iter().enumerate() {
        let hit = parts
            .iter()
            .find(|part| part.index == 3 + index)
            .unwrap()
            .frame;
        assert_eq!(artboard.center(), hit.center());
        assert_eq!(artboard.size.width, artboard.size.height);
        assert!(hit.size.width > artboard.size.width && hit.size.height > artboard.size.height);
    }
    let hit = parts.iter().find(|part| part.index == 4).unwrap().frame;
    let outside_art_inside_hit = Point::new(hit.min_x() + 0.5, hit.center().y);
    assert!(!frames[1].contains(outside_art_inside_hit));
    assert!(
        h.event(&ContentEvent::PointerDown(outside_art_inside_hit))
            .capture_pointer
    );
    assert!(
        h.event(&ContentEvent::PointerUp(outside_art_inside_hit))
            .activated
    );
    assert_eq!(h.drain_action_records()[0].payload, expected_close(1));
    assert_eq!(tabs(&mut h).selected(), 0);
    tabs(&mut h).set_tab_disabled(1, true);
    let mut disabled_icons = Icons::default();
    render_icons(&mut h, &mut disabled_icons);
    assert_ne!(icons.requests[1].2, disabled_icons.requests[1].2);
}

#[test]
fn close_cross_has_no_background_in_pointer_states_and_keeps_keyboard_focus() {
    use tekes_ui_tokens::{Palette, surface_palette::resolve_surface};
    for theme in Theme::ALL {
        let palette = Palette::from(theme);
        for state in ["rest", "hover", "pressed", "disabled"] {
            let mut h = host(0);
            render_icons_on_theme(&mut h, &mut Icons::default(), theme);
            let point = close_point(&h, 1);
            match state {
                "hover" => {
                    h.event(&ContentEvent::PointerMove(point));
                }
                "pressed" => {
                    h.event(&ContentEvent::PointerDown(point));
                }
                "disabled" => {
                    tabs(&mut h).set_tab_disabled(1, true);
                }
                _ => {}
            }
            let mut icons = Icons::default();
            let scene = render_icons_on_theme(&mut h, &mut icons, theme);
            let hit = h
                .semantics()
                .into_iter()
                .find(|node| node.id == TABS)
                .unwrap()
                .parts
                .into_iter()
                .find(|part| part.index == 4)
                .unwrap()
                .frame;
            assert!(
                !scene.primitives().iter().any(|p| matches!(p,
                    Primitive::Quad(q) if q.rect == hit
                )),
                "close has no frame background or border: {theme:?}/{state}"
            );
            assert!(
                scene.primitives().iter().any(|p| matches!(p,
                    Primitive::Quad(q) if q.fill == palette.color("row-selection").unwrap()
                        && q.rect.contains(close_point(&h, 0))
                )),
                "the selected Tab still owns its fill: {theme:?}/{state}"
            );
            assert_eq!(
                icons.requests[1].2,
                resolve_surface(palette, "canvas", "action-quiet", state)
                    .unwrap()
                    .icon,
                "close retains its original semantic foreground: {theme:?}/{state}"
            );
        }
        let mut h = host(0);
        render_icons_on_theme(&mut h, &mut Icons::default(), theme);
        assert!(
            h.accessibility_action(TABS, Some(4), AccessibilityAction::Focus)
                .consumed
        );
        let scene = render_icons_on_theme(&mut h, &mut Icons::default(), theme);
        let point = close_point(&h, 1);
        assert!(
            scene.primitives().iter().any(|p| matches!(p,
                Primitive::Quad(q) if q.rect.contains(point) && q.fill.a == 0.
                    && q.border_color == palette.color("border-focused").unwrap()
                    && q.border_width == proposal::metric("stroke", "c-focus-width").unwrap()
            )),
            "the transparent close keeps a visible keyboard focus: {theme:?}"
        );
    }
}

#[test]
fn only_closable_tabs_require_icons_and_upload_failure_is_reported_by_the_original_host() {
    let frame = Rect::new(10., 10., 460., 200.);
    let mut text = coretext::Text::new(1.);
    text.services = true;
    let mut closable = host(0);
    assert_eq!(
        closable.render(
            &mut Scene::new(),
            &mut text,
            frame,
            frame,
            Theme::LIGHT.into()
        ),
        Err(CompositionError::MissingIconServices(TABS))
    );
    let mut plain = ContentHost::new(Content::new(
        TABS,
        TabsContent::new(
            vec![TabItem::new(
                Content::new(label(0), TextContent::new("Plain")),
                Content::new(panel(0), TextContent::new("Plain panel")),
            )],
            0,
        ),
    ))
    .unwrap();
    assert_eq!(
        plain.render(
            &mut Scene::new(),
            &mut text,
            frame,
            frame,
            Theme::LIGHT.into()
        ),
        Ok(())
    );
    let mut failing = Icons {
        fail: true,
        ..Default::default()
    };
    assert_eq!(
        closable.render(
            &mut Scene::new(),
            &mut WithIconRenderer::new(&mut text, &mut failing),
            frame,
            frame,
            Theme::LIGHT.into()
        ),
        Err(CompositionError::IconResource {
            owner: TABS,
            reason: IconRenderError::Upload
        })
    );
    render(&mut closable);
    assert!(closable.drain_action_records().is_empty());
}

#[test]
fn pointer_close_is_a_once_only_request_and_does_not_select_or_remove_content() {
    let mut h = host(0);
    render(&mut h);
    let node = h
        .semantics()
        .into_iter()
        .find(|node| node.id == TABS)
        .unwrap();
    let close = node.parts.iter().find(|part| part.index == 4).unwrap();
    assert_eq!(close.semantics.role, ContentRole::Button);
    assert_eq!(close.semantics.name, "Close Two");
    assert_eq!(h.semantic_part_parent(TABS, 4), Some(1));
    assert!(
        node.parts.iter().all(|part| part.index != 5),
        "List is not closable"
    );
    let point = close_point(&h, 1);
    assert!(h.event(&ContentEvent::PointerDown(point)).capture_pointer);
    assert!(h.event(&ContentEvent::PointerUp(point)).activated);
    let records = h.drain_action_records();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].action.source, ContentActionSource::Pointer);
    assert_eq!(records[0].payload, expected_close(1));
    assert_eq!(tabs(&mut h).selected(), 0);
    assert!(tabs(&mut h).is_tab_open(1));
    assert!(h.get(panel(1)).is_some());
    assert!(tabs(&mut h).set_tab_open(1, false));
    assert_eq!(
        tabs(&mut h).selected(),
        0,
        "approving an inactive close keeps selection"
    );
    h.event(&ContentEvent::PointerUp(point));
    assert!(h.drain_action_records().is_empty());
}

#[test]
fn ax_and_keyboard_close_share_the_original_identity_without_a_selection_action() {
    let mut h = host(0);
    render(&mut h);
    assert!(
        h.accessibility_action(TABS, Some(4), AccessibilityAction::Press)
            .activated
    );
    let records = h.drain_action_records();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].action.source, ContentActionSource::Accessibility);
    assert_eq!(records[0].payload, expected_close(1));
    assert_eq!(tabs(&mut h).selected(), 0);
    assert!(
        h.accessibility_action(TABS, Some(4), AccessibilityAction::Focus)
            .consumed
    );
    h.event(&ContentEvent::Key {
        key: ControlKey::Space,
        down: true,
    });
    assert!(h.drain_action_records().is_empty());
    h.event(&ContentEvent::Key {
        key: ControlKey::Space,
        down: false,
    });
    let records = h.drain_action_records();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].action.source, ContentActionSource::Keyboard);
    assert_eq!(records[0].payload, expected_close(1));
    assert_eq!(tabs(&mut h).selected(), 0);
}

#[test]
fn queued_close_keeps_original_ids_across_a_later_reorder_and_skips_disabled_fallback() {
    let mut h = host(0);
    render(&mut h);
    assert!(
        h.accessibility_action(TABS, Some(4), AccessibilityAction::Press)
            .activated
    );
    assert!(tabs(&mut h).reorder(&[1, 0, 2]));
    let records = h.drain_action_records();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].payload, expected_close(1));
    assert_eq!(
        tabs(&mut h).selected(),
        1,
        "selected original One follows its identity"
    );
    tabs(&mut h).set_tab_disabled(2, true);
    assert!(tabs(&mut h).set_tab_open(1, false));
    assert_eq!(
        tabs(&mut h).selected(),
        0,
        "disabled right neighbor yields to the left"
    );
}

#[test]
fn approved_close_falls_right_then_left_and_reopen_preserves_mount_and_draft() {
    let mut h = host(1);
    render(&mut h);
    let mount = h.semantic_mount(panel(1));
    assert!(mount.is_some());
    assert!(tabs(&mut h).set_tab_open(1, false));
    assert_eq!(tabs(&mut h).selected(), 2);
    assert_eq!(h.semantic_mount(panel(1)), None);
    assert!(h.get(panel(1)).is_some());
    assert!(tabs(&mut h).set_tab_open(2, false));
    assert_eq!(tabs(&mut h).selected(), 0);
    assert!(tabs(&mut h).set_tab_open(0, false));
    render(&mut h);
    let nodes = h.semantics();
    assert!(
        nodes
            .iter()
            .all(|node| ![panel(0), panel(1), panel(2)].contains(&node.id))
    );
    assert!(
        nodes
            .iter()
            .filter(|node| node.id == TABS)
            .all(|node| node.parts.is_empty())
    );
    assert!(h.focus_targets().is_empty());
    assert!(
        !h.accessibility_action(TABS, Some(4), AccessibilityAction::Press)
            .activated
    );
    assert!(tabs(&mut h).set_tab_open(1, true));
    render(&mut h);
    assert_eq!(tabs(&mut h).selected(), 1);
    assert_eq!(h.semantic_mount(panel(1)), mount);
    assert_eq!(
        h.get(panel(1))
            .unwrap()
            .component::<TextFieldContent>()
            .unwrap()
            .surface
            .input
            .text,
        "retained draft"
    );
    assert!(
        h.drain_action_records().is_empty(),
        "approval is not a new request"
    );
}

#[test]
fn disabled_and_composing_tabs_reject_close_and_keep_original_input() {
    let mut h = host(0);
    render(&mut h);
    tabs(&mut h).set_tab_disabled(1, true);
    assert!(
        !h.accessibility_action(TABS, Some(4), AccessibilityAction::Press)
            .activated
    );
    let point = close_point(&h, 1);
    assert!(!h.event(&ContentEvent::PointerDown(point)).capture_pointer);
    h.event(&ContentEvent::PointerUp(point));
    h.focus(Some(panel(0)), false);
    h.event(&ContentEvent::Preedit {
        text: "にほん".into(),
        selection: 0..3,
        clauses: std::iter::once(0..3).collect(),
        replacement: None,
    });
    render(&mut h);
    assert!(h.is_composing());
    assert!(
        !h.accessibility_action(TABS, Some(3), AccessibilityAction::Press)
            .activated
    );
    assert!(!tabs(&mut h).set_tab_open(0, false));
    assert!(tabs(&mut h).is_tab_open(0));
    assert!(h.is_composing());
    assert_eq!(h.focused(), Some(panel(0)));
    assert!(h.drain_action_records().is_empty());
    tabs(&mut h).set_tab_disabled(1, false);
    render(&mut h);
    assert!(
        h.accessibility_action(TABS, Some(4), AccessibilityAction::Press)
            .activated
    );
    assert_eq!(h.drain_action_records()[0].payload, expected_close(1));
    assert!(tabs(&mut h).set_tab_open(1, false));
    assert!(
        h.is_composing(),
        "closing another panel must not end the selected composition"
    );
    assert_eq!(h.focused(), Some(panel(0)));
}

#[test]
fn close_arms_are_revoked_by_reorder_hiding_and_window_deactivation() {
    let mut h = host(0);
    render(&mut h);
    let old = close_point(&h, 1);
    assert!(h.event(&ContentEvent::PointerDown(old)).capture_pointer);
    assert!(tabs(&mut h).reorder(&[1, 0, 2]));
    render(&mut h);
    assert!(!h.event(&ContentEvent::PointerUp(old)).activated);
    assert!(h.drain_action_records().is_empty());
    let point = close_point(&h, 0);
    assert!(h.event(&ContentEvent::PointerDown(point)).capture_pointer);
    assert!(tabs(&mut h).set_tab_open(0, false));
    render(&mut h);
    assert!(!h.event(&ContentEvent::PointerUp(point)).activated);
    assert!(
        !h.accessibility_action(TABS, Some(3), AccessibilityAction::Press)
            .activated
    );
    assert!(h.drain_action_records().is_empty());
    tabs(&mut h).set_tab_open(0, true);
    render(&mut h);
    let point = close_point(&h, 0);
    assert!(h.event(&ContentEvent::PointerDown(point)).capture_pointer);
    tabs(&mut h).set_window_active(false);
    assert!(!h.event(&ContentEvent::PointerUp(point)).activated);
    assert!(h.drain_action_records().is_empty());
}

#[derive(Debug)]
struct FakeAuxiliary {
    declared: bool,
    parent_role: ContentRole,
    parent_expanded: bool,
    parent_text: bool,
    parent_value: bool,
    calls: Rc<Cell<usize>>,
}
impl Component for FakeAuxiliary {
    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        MeasuredChild::new(available)
    }
    fn semantics(&self) -> ContentSemantics {
        ContentSemantics::new(ContentRole::TabList, "Fake tabs")
    }
    fn semantic_parts(&self) -> Vec<SemanticPart> {
        [
            (
                0,
                self.parent_role,
                "Original",
                Rect::new(10., 10., 100., 30.),
            ),
            (
                1,
                ContentRole::Button,
                "Close fake",
                Rect::new(80., 10., 20., 30.),
            ),
        ]
        .into_iter()
        .map(|(index, role, name, frame)| {
            let mut semantics = ContentSemantics::new(role, name);
            if index == 0 && self.parent_expanded {
                semantics.expanded = Some(true);
            }
            SemanticPart {
                index,
                frame,
                focused: false,
                semantics,
            }
        })
        .collect()
    }
    fn semantic_part_parent(&self, part: usize) -> Option<usize> {
        (part == 1).then_some(0)
    }
    fn semantic_part_is_auxiliary_action(&self, _: usize) -> bool {
        self.declared
    }
    fn semantic_part_text_layout(&self, part: usize) -> Option<ContentTextLayout> {
        (part == 0 && self.parent_text).then(|| ContentTextLayout {
            input: ContentTextInput::from_surface(
                &TextFieldContent::new("invalid Tab text", "").surface,
            ),
            multiline: false,
            lines: Vec::new(),
            carets: Vec::new(),
        })
    }
    fn semantic_part_value_range(&self, part: usize) -> Option<std::ops::RangeInclusive<f64>> {
        (part == 0 && self.parent_value).then_some(0.0..=1.0)
    }
    fn accessibility_action(
        &mut self,
        _: Option<usize>,
        _: &AccessibilityAction,
    ) -> ContentResponse {
        self.calls.set(self.calls.get() + 1);
        ContentResponse {
            consumed: true,
            activated: true,
            ..Default::default()
        }
    }
}
#[test]
fn ax_gate_rejects_an_undeclared_button_and_an_auxiliary_with_no_original_tab_parent() {
    for (declared, parent_role) in [(false, ContentRole::Tab), (true, ContentRole::Button)] {
        let calls = Rc::new(Cell::new(0));
        let mut h = ContentHost::new(Content::new(
            TABS,
            FakeAuxiliary {
                declared,
                parent_role,
                parent_expanded: false,
                parent_text: false,
                parent_value: false,
                calls: calls.clone(),
            },
        ))
        .unwrap();
        render(&mut h);
        assert!(
            !h.accessibility_action(TABS, Some(1), AccessibilityAction::Press)
                .activated
        );
        assert_eq!(calls.get(), 0);
    }
}

#[test]
fn auxiliary_close_cannot_bypass_an_invalid_or_text_value_owning_parent_tab() {
    for invalid in 0..3 {
        let calls = Rc::new(Cell::new(0));
        let mut h = ContentHost::new(Content::new(
            TABS,
            FakeAuxiliary {
                declared: true,
                parent_role: ContentRole::Tab,
                parent_expanded: invalid == 0,
                parent_text: invalid == 1,
                parent_value: invalid == 2,
                calls: calls.clone(),
            },
        ))
        .unwrap();
        render(&mut h);
        assert!(
            !h.accessibility_action(TABS, Some(0), AccessibilityAction::Press)
                .consumed
        );
        assert!(
            !h.accessibility_action(TABS, Some(1), AccessibilityAction::Press)
                .consumed
        );
        assert_eq!(calls.get(), 0);
    }
}

#[test]
fn auxiliary_close_only_admits_original_focus_and_press_actions() {
    let calls = Rc::new(Cell::new(0));
    let mut h = ContentHost::new(Content::new(
        TABS,
        FakeAuxiliary {
            declared: true,
            parent_role: ContentRole::Tab,
            parent_expanded: false,
            parent_text: false,
            parent_value: false,
            calls: calls.clone(),
        },
    ))
    .unwrap();
    render(&mut h);
    for action in [
        AccessibilityAction::Increment,
        AccessibilityAction::Decrement,
        AccessibilityAction::SetExpanded(true),
        AccessibilityAction::SetValue("x".into()),
        AccessibilityAction::SetSelection(0..0),
        AccessibilityAction::ReplaceSelectedText("x".into()),
    ] {
        assert!(!h.accessibility_action(TABS, Some(1), action).consumed);
        assert_eq!(calls.get(), 0);
    }
}
