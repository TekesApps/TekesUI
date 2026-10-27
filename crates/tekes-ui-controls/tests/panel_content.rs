//! Public original Host + real original text editor; no native window or GPU.
use std::rc::Rc;
use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, Insets, TextSurfaceHost, TextSurfaceTextRenderer,
    composition::*,
    extended::{Axis, Stack},
    layout::MeasuredChild,
};
use tekes_ui_scene::{Point, Rect, Scene, Size};
use tekes_ui_text::{
    Paragraph, ShapeOptions, Shaper, TextStyle,
    coretext::{CoreTextFonts, CoreTextShaper},
};
use tekes_ui_tokens::{Rgba, Theme, proposal};
struct Text {
    enabled: bool,
    now: std::time::Duration,
    reduced: bool,
    clocked: bool,
    frames: usize,
    shaper: CoreTextShaper,
    widths: Vec<Option<f32>>,
}
impl Text {
    fn new(enabled: bool) -> Self {
        Self {
            enabled,
            now: std::time::Duration::ZERO,
            reduced: false,
            clocked: true,
            frames: 0,
            shaper: CoreTextShaper::new(CoreTextFonts::new()),
            widths: Vec::new(),
        }
    }
}
impl ButtonTextRenderer for Text {
    fn text_surface_host(&mut self) -> Option<&mut dyn TextSurfaceHost> {
        if self.enabled { Some(self) } else { None }
    }
    fn motion_context(&self) -> Option<tekes_ui_controls::ControlMotionContext> {
        self.clocked
            .then_some(tekes_ui_controls::ControlMotionContext {
                now: self.now,
                reduced_motion: self.reduced,
            })
    }
    fn request_animation_frame(&mut self) {
        self.frames += 1;
    }
    fn measure(&mut self, s: &str, r: proposal::TextRole, w: f32) -> Size {
        assert!(w > 0., "inactive pane must not measure zero-width text");
        let paragraph = self.shape_surface(s, r, Some(w));
        Size::new(paragraph.width as f32, paragraph.height as f32)
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
impl TextSurfaceHost for Text {
    fn shape_surface(&mut self, s: &str, r: proposal::TextRole, w: Option<f32>) -> Rc<Paragraph> {
        assert!(w.is_none_or(|w| w > 0.), "zero-width editor shaping");
        self.widths.push(w);
        Rc::new(self.shaper.shape(
            s,
            &TextStyle::system(r.style().size).with_weight(r.style().weight),
            &ShapeOptions {
                width: w.map(f64::from),
                line_spacing: 0.,
            },
        ))
    }
}
impl TextSurfaceTextRenderer for Text {
    fn push_visible(&mut self, _: &mut Scene, _: &Paragraph, _: Point, _: Rgba, _: Rect) {}
}

const ROOT: ComponentId = ComponentId(8100);
const CHILD: ComponentId = ComponentId(8101);
const FIELD: ComponentId = ComponentId(8102);
fn render(h: &mut ContentHost, t: &mut Text, clip: Rect) -> Scene {
    let mut s = Scene::new();
    h.render(
        &mut s,
        t,
        Rect::new(0., 0., 100., 100.),
        clip,
        Theme::LIGHT.into(),
    )
    .unwrap();
    s
}
fn panel(child: Content, zero: bool) -> PanelContent {
    let mut p = PanelContent::new(child).unwrap();
    if zero {
        p.set_padding(Some(Insets::default())).unwrap();
    }
    p
}

fn split_input(leading: Content, trailing: Content) -> SplitViewConstruction {
    SplitViewConstruction {
        config: SplitViewConfig::default(),
        slots: SplitViewSlotIds {
            leading: ComponentId(8290),
            trailing: ComponentId(8291),
            divider: ComponentId(8292),
        },
        leading,
        trailing,
        budget: 64 * 1024 * 1024,
    }
}

fn stack(children: Vec<Content>) -> StackContent {
    StackContent {
        model: Stack {
            axis: Axis::Vertical,
            gap: 0.,
        },
        flexible_child: None,
        children,
    }
}

#[derive(Debug)]
struct UnsupportedPreparedEffects;
impl Component for UnsupportedPreparedEffects {
    fn table_transaction_compatibility(&self) -> TableTransactionCompatibility {
        TableTransactionCompatibility::StableHooks
    }
    fn curve_subtree_capability(&self) -> CurveSubtreeCapability {
        CurveSubtreeCapability::Recursive
    }
    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, _: Size) -> MeasuredChild {
        MeasuredChild::new(Size::new(8., 8.))
    }
}

#[test]
fn split_view_accepts_nested_panels_with_local_prepared_effects() {
    let nested = Content::new(
        ComponentId(8280),
        panel(
            Content::new(ComponentId(8281), TextContent::new("nested")),
            true,
        ),
    );
    let leading = Content::new(
        ComponentId(8270),
        panel(Content::new(ComponentId(8271), stack(vec![nested])), true),
    );
    let trailing = Content::new(
        ComponentId(8260),
        panel(
            Content::new(ComponentId(8261), Button::new("trailing")),
            true,
        ),
    );

    assert!(SplitViewContent::try_new(split_input(leading, trailing)).is_ok());
}

#[test]
fn split_view_rejects_unsupported_nested_panel_descendant_and_returns_full_input() {
    let unsupported_id = ComponentId(8252);
    let nested = Content::new(unsupported_id, UnsupportedPreparedEffects);
    let leading = Content::new(
        ComponentId(8250),
        panel(Content::new(ComponentId(8251), stack(vec![nested])), true),
    );
    let trailing = Content::new(
        ComponentId(8240),
        panel(
            Content::new(ComponentId(8241), Button::new("trailing")),
            true,
        ),
    );
    let input = split_input(leading, trailing);
    let expected_slots = input.slots;
    let expected_config = input.config;

    let rejected =
        SplitViewContent::try_new(input).expect_err("unknown descendant is not authorized");
    assert_eq!(
        rejected.error,
        SplitViewError::Effects(PreparedEffectsError::Unavailable)
    );
    assert_eq!(rejected.input.slots.leading, expected_slots.leading);
    assert_eq!(rejected.input.slots.trailing, expected_slots.trailing);
    assert_eq!(rejected.input.slots.divider, expected_slots.divider);
    assert_eq!(rejected.input.config, expected_config);
    assert_eq!(rejected.input.budget, 64 * 1024 * 1024);
    assert_eq!(rejected.input.leading.id(), ComponentId(8250));
    assert_eq!(rejected.input.trailing.id(), ComponentId(8240));
    let retained = rejected
        .input
        .leading
        .component::<PanelContent>()
        .unwrap()
        .child()
        .component::<StackContent>()
        .unwrap();
    assert_eq!(retained.children[0].id(), unsupported_id);
    assert!(
        retained.children[0]
            .component::<UnsupportedPreparedEffects>()
            .is_some()
    );
}
#[test]
fn original_route_gap_capture_identity_and_atomic_fragment() {
    let mut h = ContentHost::new(Content::new(
        ROOT,
        panel(
            Content::new(CHILD, Button::new("real button"))
                .preserving_focus()
                .bubbling_handled_events(true),
            true,
        ),
    ))
    .unwrap();
    let mut t = Text::new(true);
    let mut s = Scene::new();
    s.push_quad(tekes_ui_scene::Quad::new(
        Rect::new(110., 0., 10., 10.),
        Rgba::gray(0.5),
    ));
    let original = s.primitives()[0].clone();
    h.render(
        &mut s,
        &mut t,
        Rect::new(0., 0., 100., 100.),
        Rect::new(0., 0., 100., 100.),
        Theme::LIGHT.into(),
    )
    .unwrap();
    assert_eq!(s.primitives()[0], original);
    assert!(
        s.primitives()
            .iter()
            .any(|p| matches!(p, tekes_ui_scene::Primitive::PushShapeClip(_)))
    );
    assert_eq!(
        h.try_pointer_regions(),
        Err(CompositionError::CurvePointerRegionsUnsupported)
    );
    assert!(h.pointer_regions().is_empty());
    assert!(h.pointer_hit_test(Point::new(0.5, 0.5)).is_none());
    assert_eq!(h.pointer_hit_test(Point::new(50., 50.)), Some(CHILD));
    let mount = h.semantic_mount(CHILD).unwrap();
    let snapshot = h.geometry_snapshot().unwrap();
    assert!(snapshot.hit_test(Point::new(0.5, 0.5)).is_none());
    assert_eq!(snapshot.hit_test(Point::new(50., 50.)).unwrap().0, CHILD);
    assert!(
        !h.event(&ContentEvent::PointerDown(Point::new(0.5, 0.5)))
            .consumed
    );
    render(&mut h, &mut t, Rect::new(0., 0., 100., 100.));
    assert!(
        h.event(&ContentEvent::PointerDown(Point::new(50., 50.)))
            .consumed
    );
    assert!(h.pointer_capture().is_some());
    assert!(
        h.event(&ContentEvent::PointerMove(Point::new(0.5, 0.5)))
            .changed
    );
    assert!(
        !h.get(CHILD)
            .unwrap()
            .component::<Button>()
            .unwrap()
            .button
            .state
            .hovered
    );
    let r = h.event(&ContentEvent::PointerUp(Point::new(0.5, 0.5)));
    assert!(r.consumed);
    assert!(!r.activated);
    assert!(h.pointer_capture().is_none());
    assert_eq!(
        h.get(CHILD)
            .unwrap()
            .component::<Button>()
            .unwrap()
            .activations,
        0
    );
    assert_eq!(h.semantic_mount(CHILD), Some(mount));
    assert!(!snapshot.is_current());
    assert!(snapshot.hit_test(Point::new(50., 50.)).is_none());
}
#[derive(Debug)]
struct Opaque(std::rc::Rc<std::cell::Cell<usize>>);
impl Component for Opaque {
    fn measure(
        &mut self,
        _: &mut dyn ButtonTextRenderer,
        a: Size,
    ) -> tekes_ui_controls::layout::MeasuredChild {
        self.0.set(self.0.get() + 1);
        tekes_ui_controls::layout::MeasuredChild::new(a)
    }
    fn lifecycle(&mut self, _: Lifecycle) {
        self.0.set(self.0.get() + 1);
    }
}
#[test]
fn unknown_child_refused_before_lifecycle_or_layout_and_insert_preserves_original_tree() {
    let count = std::rc::Rc::new(std::cell::Cell::new(0));
    assert!(matches!(
        PanelContent::new(Content::new(CHILD, Opaque(count.clone()))),
        Err(CompositionError::UnsupportedCurveChild(CHILD))
    ));
    assert_eq!(count.get(), 0);
    let children = Content::new(
        CHILD,
        StackContent {
            model: Stack {
                axis: Axis::Vertical,
                gap: 0.,
            },
            flexible_child: None,
            children: vec![],
        },
    );
    let mut h = ContentHost::new(Content::new(ROOT, panel(children, false))).unwrap();
    assert_eq!(
        h.insert(CHILD, Content::new(FIELD, Opaque(count.clone()))),
        Err(CompositionError::UnsupportedCurveChild(FIELD))
    );
    assert_eq!(count.get(), 0);
    assert!(h.get(FIELD).is_none());
}
#[test]
fn real_original_editor_marked_selection_survives_full_clip() {
    let mut composer = TextFieldContent::new("Composer", "kept 😀 draft");
    composer.surface.input.multiline = true;
    composer.surface.presentation = tekes_ui_controls::TextSurfacePresentation::Content;
    composer.viewport = TextViewport::Fill;
    composer.show_metadata = false;

    let mut h = ContentHost::new(Content::new(
        ROOT,
        panel(Content::new(FIELD, composer), false),
    ))
    .unwrap();
    let mut t = Text::new(true);
    render(&mut h, &mut t, Rect::new(0., 0., 100., 100.));
    assert!(h.focus(Some(FIELD), true));
    h.event_with_text(
        &ContentEvent::Preedit {
            text: "中文".into(),
            selection: 0..2,
            clauses: std::iter::once(0..2).collect(),
            replacement: None,
        },
        &mut t,
    )
    .unwrap();
    let before = h.text_input_snapshot().unwrap();
    let native_binding = h.native_key_binding().unwrap();
    let mount = h.semantic_mount(FIELD).unwrap();
    assert!(before.marked.is_some());
    render(&mut h, &mut t, Rect::new(300., 300., 100., 100.));
    let after = h.text_input_snapshot().unwrap();
    assert_eq!(before.binding, after.binding);
    assert_eq!(h.native_key_binding(), Some(native_binding));
    assert_eq!(before.text, after.text);
    assert_eq!(before.marked, after.marked);
    assert_eq!(before.selection, after.selection);
    assert_eq!(h.focused(), Some(FIELD));
    assert_eq!(h.semantic_mount(FIELD), Some(mount));
    let before_snapshot = h.geometry_snapshot().unwrap();
    assert!(before_snapshot.hit_test(Point::new(50., 50.)).is_none());
    let _ = h.get_mut(FIELD);
    assert!(!before_snapshot.is_current());
    assert!(h.geometry_snapshot().is_none());
}
#[test]
fn strict_curve_never_downgrades_before_layout_or_after_geometry_mutation() {
    let mut h = ContentHost::new(Content::new(
        ROOT,
        panel(Content::new(CHILD, Button::new("button")), false),
    ))
    .unwrap();
    assert!(!h.focus(Some(CHILD), true));
    let mut t = Text::new(true);
    render(&mut h, &mut t, Rect::new(0., 0., 100., 100.));
    assert!(h.focus(Some(CHILD), true));
    h.focus(None, false);
    let _ = h.get_mut(ROOT);
    assert!(!h.focus(Some(CHILD), true));
    assert!(h.geometry_snapshot().is_none());
    assert!(h.pointer_hit_test(Point::new(50., 50.)).is_none());
    assert_eq!(h.focused(), None);
}

#[test]
fn original_reading_reveal_consumes_curve_plan_and_stale_plan_cannot_change_user_offset() {
    use tekes_ui_controls::{ScrollArea, ScrollRevealOptions, ScrollRevealPlacement};
    let scroll_id = ComponentId(8120);
    let rows_id = ComponentId(8121);
    let target = ComponentId(8149);
    let rows = Content::new(
        rows_id,
        StackContent {
            model: Stack {
                axis: Axis::Vertical,
                gap: 0.,
            },
            flexible_child: None,
            children: (0..30)
                .map(|i| Content::new(ComponentId(8122 + i), TextContent::new(format!("line {i}"))))
                .collect(),
        },
    );
    let mut scroll = ScrollContent::new(ScrollArea::default(), rows);
    scroll.viewport_height = Some(100.);
    let mut h = ContentHost::new(Content::new(
        ROOT,
        panel(Content::new(scroll_id, scroll), true),
    ))
    .unwrap();
    h.set_scroll_reading_document(scroll_id, "panel-document", 1, vec![target], |_| {})
        .unwrap();
    let mut t = Text::new(true);
    render(&mut h, &mut t, Rect::new(0., 0., 100., 100.));
    let mount = h.semantic_mount(target).unwrap();
    let version = h
        .get(scroll_id)
        .unwrap()
        .component::<ScrollContent>()
        .unwrap()
        .reading_document_version()
        .unwrap()
        .clone();
    let options = ScrollRevealOptions {
        placement: ScrollRevealPlacement::Start(Axis::Vertical),
        animated: false,
        ..Default::default()
    };
    let plan = h
        .plan_host_reveal(target, options, &[], HostRevealBudget::default())
        .unwrap();
    let before = h
        .get(scroll_id)
        .unwrap()
        .component::<ScrollContent>()
        .unwrap()
        .scroll
        .offset;
    assert_eq!(before, Point::default());
    let outcome = h.commit_host_reveal(plan).unwrap();
    assert!(outcome.revealed);
    assert!(outcome.changed);
    render(&mut h, &mut t, Rect::new(0., 0., 100., 100.));
    let current = h
        .get(scroll_id)
        .unwrap()
        .component::<ScrollContent>()
        .unwrap();
    assert_eq!(current.reading_document_version(), Some(&version));
    assert_eq!(h.semantic_mount(target), Some(mount));
    assert!(current.scroll.offset.y > 0.);
    let stale = h
        .plan_host_reveal(
            target,
            ScrollRevealOptions {
                animated: false,
                ..Default::default()
            },
            &[],
            HostRevealBudget::default(),
        )
        .unwrap();
    let before = h
        .get(scroll_id)
        .unwrap()
        .component::<ScrollContent>()
        .unwrap()
        .scroll
        .offset;
    let _ = h.get_mut(scroll_id);
    assert_eq!(h.commit_host_reveal(stale), Err(HostRevealError::StalePlan));
    assert_eq!(
        h.get(scroll_id)
            .unwrap()
            .component::<ScrollContent>()
            .unwrap()
            .scroll
            .offset,
        before
    );
}

#[test]
fn prepared_primary_down_in_panel_commits_focus_before_capture() {
    let mut host = ContentHost::new(Content::new(
        ROOT,
        panel(Content::new(CHILD, Button::new("prepared button")), false),
    ))
    .unwrap();
    let mut text = Text::new(true);
    render(&mut host, &mut text, Rect::new(0., 0., 100., 100.));
    let response = host.event(&ContentEvent::PointerDown(Point::new(50., 50.)));
    assert!(response.consumed);
    assert_eq!(host.focused(), Some(CHILD));
    assert!(host.pointer_capture().is_some());
    assert_eq!(
        host.get(CHILD)
            .unwrap()
            .component::<Button>()
            .unwrap()
            .activations,
        0
    );
}

#[test]
fn same_id_remount_replaces_frozen_curve_mount_identity() {
    let stack_id = ComponentId(8154);
    let stack = Content::new(
        stack_id,
        StackContent {
            model: Stack {
                axis: Axis::Vertical,
                gap: 0.,
            },
            flexible_child: None,
            children: vec![Content::new(CHILD, Button::new("first mount"))],
        },
    );
    let mut host = ContentHost::new(Content::new(ROOT, panel(stack, false))).unwrap();
    let mut text = Text::new(true);
    render(&mut host, &mut text, Rect::new(0., 0., 100., 100.));
    let old_mount = host.semantic_mount(CHILD).unwrap();
    let old_snapshot = host.geometry_snapshot().unwrap();
    assert_eq!(old_snapshot.identity(CHILD, None), Some(old_mount));

    host.remove(CHILD).unwrap();
    host.insert(
        stack_id,
        Content::new(CHILD, Button::new("replacement mount")),
    )
    .unwrap();
    assert!(!old_snapshot.is_current());
    assert!(host.geometry_snapshot().is_none());
    assert!(host.pointer_hit_test(Point::new(50., 50.)).is_none());
    render(&mut host, &mut text, Rect::new(0., 0., 100., 100.));

    let new_mount = host.semantic_mount(CHILD).unwrap();
    assert_ne!(old_mount, new_mount);
    assert_eq!(old_snapshot.identity(CHILD, None), None);
    assert_eq!(
        host.geometry_snapshot().unwrap().identity(CHILD, None),
        Some(new_mount)
    );
}

#[test]
fn captured_button_uses_current_curve_after_legal_relayout() {
    let mut host = ContentHost::new(Content::new(
        ROOT,
        panel(Content::new(CHILD, Button::new("relayout capture")), false),
    ))
    .unwrap();
    let mut text = Text::new(true);
    render(&mut host, &mut text, Rect::new(0., 0., 100., 100.));
    assert!(
        host.event(&ContentEvent::PointerDown(Point::new(50., 50.)))
            .consumed
    );
    let capture = host.pointer_capture().unwrap();
    let mount = host.semantic_mount(CHILD).unwrap();
    assert_eq!(capture.0, CHILD);
    assert_eq!(capture.1, mount);

    let mut scene = Scene::new();
    host.render(
        &mut scene,
        &mut text,
        Rect::new(10., 0., 100., 100.),
        Rect::new(10., 0., 100., 100.),
        Theme::LIGHT.into(),
    )
    .unwrap();
    assert_eq!(host.pointer_capture(), Some(capture));
    assert_eq!(host.pointer_hit_test(Point::new(60., 50.)), Some(CHILD));
    assert!(
        host.event(&ContentEvent::PointerMove(Point::new(60., 50.)))
            .changed
    );
    assert!(
        host.get(CHILD)
            .unwrap()
            .component::<Button>()
            .unwrap()
            .button
            .state
            .hovered
    );

    assert!(host.pointer_hit_test(Point::new(10., 0.)).is_none());
    assert!(
        host.event(&ContentEvent::PointerMove(Point::new(10., 0.)))
            .changed
    );
    assert!(
        !host
            .get(CHILD)
            .unwrap()
            .component::<Button>()
            .unwrap()
            .button
            .state
            .hovered
    );
    assert_eq!(host.pointer_capture(), Some(capture));
}

#[test]
fn curve_input_budget_refuses_before_host_mount_or_layout() {
    let children = (0..HostRevealBudget::default().max_path_nodes)
        .map(|i| Content::new(ComponentId(8200 + i as u64), TextContent::new("bounded")))
        .collect();
    let tree = Content::new(
        ComponentId(8190),
        StackContent {
            model: Stack {
                axis: Axis::Vertical,
                gap: 0.,
            },
            flexible_child: None,
            children,
        },
    );
    assert!(matches!(
        PanelContent::new(tree),
        Err(CompositionError::CurveGeometry(
            tekes_ui_scene::ClipError::InputBudgetExceeded
        ))
    ));
}

#[test]
fn one_original_tree_keeps_composer_and_reading_owners_through_streaming_and_reveal() {
    use tekes_ui_controls::{ScrollArea, ScrollRevealOptions, TextSurfacePresentation};
    let root = ComponentId(8240);
    let stack = ComponentId(8241);
    let button = ComponentId(8242);
    let editor = ComponentId(8243);
    let scroll = ComponentId(8244);
    let document = ComponentId(8245);
    let text = ComponentId(8246);
    let last = ComponentId(8279);
    let incoming = ComponentId(8280);
    let mut composer = TextFieldContent::new("Composer", "one retained 😀 draft");
    composer.surface.input.multiline = true;
    composer.surface.presentation = TextSurfacePresentation::Content;
    composer.show_metadata = false;
    // Synthetic consumer allocations, not product defaults or design token replacements.
    composer.viewport = TextViewport::Fixed(40.);
    let rows = Content::new(
        document,
        StackContent {
            model: Stack {
                axis: Axis::Vertical,
                gap: 0.,
            },
            flexible_child: None,
            children: (8260..8280)
                .map(|id| Content::new(ComponentId(id), TextContent::new(format!("row {id}"))))
                .collect(),
        },
    );
    let mut reading = ScrollContent::new(ScrollArea::default(), rows);
    reading.viewport_height = Some(64.);
    let body = Content::new(
        stack,
        StackContent {
            model: Stack {
                axis: Axis::Vertical,
                gap: 0.,
            },
            flexible_child: None,
            children: vec![
                Content::new(text, TextContent::new("original text")),
                Content::new(button, Button::new("original action")),
                Content::new(editor, composer),
                Content::new(scroll, reading),
            ],
        },
    );
    let mut host = ContentHost::new(Content::new(root, PanelContent::new(body).unwrap())).unwrap();
    host.set_action_events(true);
    let (version, ()) = host
        .set_scroll_reading_document(scroll, "one-original-tree", 1, vec![last], |_| {})
        .unwrap();
    let mut t = Text::new(true);
    let frame = Rect::new(0., 0., 320., 300.);
    let paint = |h: &mut ContentHost, t: &mut Text, clip: Rect| {
        h.render(&mut Scene::new(), t, frame, clip, Theme::LIGHT.into())
            .unwrap();
    };
    paint(&mut host, &mut t, frame);
    let mounts: Vec<_> = [text, button, editor, scroll, last]
        .into_iter()
        .map(|id| (id, host.semantic_mount(id).unwrap()))
        .collect();
    assert!(host.focus(Some(editor), true));
    host.event_with_text(
        &ContentEvent::Preedit {
            text: "中文".into(),
            selection: 0..2,
            clauses: std::iter::once(0..2).collect(),
            replacement: None,
        },
        &mut t,
    )
    .unwrap();
    paint(&mut host, &mut t, frame);
    let original = host.text_input_snapshot().unwrap();
    assert!(original.marked.is_some());
    let native = host.native_key_binding().unwrap();
    assert!(
        !host
            .accessibility_action(button, None, AccessibilityAction::Focus)
            .consumed
    );
    assert_eq!(host.focused(), Some(editor));
    let (next, ()) = host
        .update_scroll_reading_document(scroll, &version, 2, vec![last, incoming], |h| {
            h.insert(
                document,
                Content::new(incoming, TextContent::new("streamed original item")),
            )
            .unwrap()
        })
        .unwrap();
    paint(&mut host, &mut t, frame);
    let plan = host
        .plan_host_reveal(
            last,
            ScrollRevealOptions {
                animated: false,
                ..Default::default()
            },
            &[],
            HostRevealBudget::default(),
        )
        .unwrap();
    assert!(host.commit_host_reveal(plan).unwrap().revealed);
    paint(&mut host, &mut t, frame);
    paint(&mut host, &mut t, Rect::new(500., 500., 320., 300.));
    let after = host.text_input_snapshot().unwrap();
    assert_eq!(host.focused(), Some(editor));
    assert_eq!(original.binding, after.binding);
    assert_eq!(original.text, after.text);
    assert_eq!(original.marked, after.marked);
    assert_eq!(original.selection, after.selection);
    assert_eq!(host.native_key_binding(), Some(native));
    for (id, mount) in mounts {
        assert_eq!(host.semantic_mount(id), Some(mount));
    }
    assert_eq!(
        host.get(scroll)
            .unwrap()
            .component::<ScrollContent>()
            .unwrap()
            .reading_document_version(),
        Some(&next)
    );
    assert!(
        !host
            .drain_action_records()
            .iter()
            .any(|r| r.action.kind == ContentActionKind::Submit)
    );
}

#[derive(Debug)]
struct TallRevealProbe;
impl Component for TallRevealProbe {
    fn measure(
        &mut self,
        _: &mut dyn ButtonTextRenderer,
        available: Size,
    ) -> tekes_ui_controls::layout::MeasuredChild {
        tekes_ui_controls::layout::MeasuredChild::new(Size::new(available.width, 180.))
    }
}

#[test]
fn ordinary_scroll_keeps_partial_oversized_reveal_with_margins() {
    use tekes_ui_controls::{ScrollArea, ScrollRevealOptions, ScrollRevealPlacement};
    let target = ComponentId(8151);
    for (placement, expect_limit_clamp) in [
        (ScrollRevealPlacement::Nearest, false),
        (ScrollRevealPlacement::Start(Axis::Vertical), true),
    ] {
        let mut scroll =
            ScrollContent::new(ScrollArea::default(), Content::new(target, TallRevealProbe));
        scroll.viewport_height = Some(60.);
        let mut host = ContentHost::new(Content::new(ROOT, scroll)).unwrap();
        let mut text = Text::new(true);
        render(&mut host, &mut text, Rect::new(0., 0., 100., 60.));
        assert!(host.get(target).unwrap().frame().size.height > 60.);
        let plan = host
            .plan_host_reveal(
                target,
                ScrollRevealOptions {
                    placement,
                    margins: Insets::new(8., 0., 8., 0.),
                    animated: false,
                },
                &[],
                HostRevealBudget::default(),
            )
            .unwrap();
        let outcome = host.commit_host_reveal(plan).unwrap();
        assert!(outcome.revealed && outcome.partial);
        assert_eq!(outcome.limit_clamped, expect_limit_clamp);
    }
}

#[derive(Debug)]
struct FocusSpaceProbe {
    space: std::rc::Rc<std::cell::RefCell<RequiredFocusSpace>>,
    focused: std::rc::Rc<std::cell::Cell<bool>>,
    content_height: Option<f32>,
}
impl Component for FocusSpaceProbe {
    fn curve_subtree_capability(&self) -> CurveSubtreeCapability {
        CurveSubtreeCapability::Local
    }
    fn measure(
        &mut self,
        _: &mut dyn ButtonTextRenderer,
        available: Size,
    ) -> tekes_ui_controls::layout::MeasuredChild {
        tekes_ui_controls::layout::MeasuredChild::new(Size::new(
            available.width,
            self.content_height.unwrap_or(available.height),
        ))
    }
    fn focusable(&self) -> bool {
        true
    }
    fn set_focus(&mut self, focused: bool, _: bool) {
        self.focused.set(focused);
    }
    fn requires_focus_reveal(&self) -> bool {
        true
    }
    fn required_focus_space(&self) -> RequiredFocusSpace {
        self.space.borrow().clone()
    }
}

fn focus_space_host(
    space: RequiredFocusSpace,
) -> (
    ContentHost,
    std::rc::Rc<std::cell::RefCell<RequiredFocusSpace>>,
    std::rc::Rc<std::cell::Cell<bool>>,
) {
    let space = std::rc::Rc::new(std::cell::RefCell::new(space));
    let focused = std::rc::Rc::new(std::cell::Cell::new(false));
    let target = FocusSpaceProbe {
        space: space.clone(),
        focused: focused.clone(),
        content_height: None,
    };
    let host =
        ContentHost::new(Content::new(ROOT, panel(Content::new(CHILD, target), true))).unwrap();
    (host, space, focused)
}

#[test]
fn visible_required_witness_survives_clamped_scroll_margins_without_curve() {
    use tekes_ui_controls::{ScrollArea, ScrollRevealOptions, ScrollRevealPlacement};
    let target_id = ComponentId(8153);
    let space = std::rc::Rc::new(std::cell::RefCell::new(RequiredFocusSpace::Unmeasured));
    let focused = std::rc::Rc::new(std::cell::Cell::new(false));
    let probe = FocusSpaceProbe {
        space: space.clone(),
        focused,
        content_height: Some(180.),
    };
    let mut scroll = ScrollContent::new(ScrollArea::default(), Content::new(target_id, probe));
    scroll.viewport_height = Some(100.);
    let mut host = ContentHost::new(Content::new(ROOT, scroll)).unwrap();
    let mut text = Text::new(true);
    let frame = Rect::new(0., 0., 120., 100.);
    render(&mut host, &mut text, frame);
    let target = host.get(target_id).unwrap().frame();
    // The rendered target starts at y=5, so this witness begins at y=55.
    // With a 60 px leading margin, Start requests -5 px and clamps to zero;
    // the 30 px witness remains fully visible but its 110 px expanded height
    // exceeds the 100 px viewport, exercising the original partial flag.
    let witness = Rect::new(target.min_x() + 20., target.min_y() + 50., 10., 30.);
    assert_eq!(target.intersection(&witness), Some(witness));
    assert!(frame.intersection(&witness).is_some());
    *space.borrow_mut() = RequiredFocusSpace::Sufficient {
        witness,
        evidence: RequiredFocusEvidence::new(),
    };
    let plan = host
        .plan_host_reveal(
            target_id,
            ScrollRevealOptions {
                placement: ScrollRevealPlacement::Start(Axis::Vertical),
                margins: Insets::new(60., 0., 20., 0.),
                animated: false,
            },
            &[],
            HostRevealBudget::default(),
        )
        .unwrap();
    let result = host.commit_host_reveal(plan).unwrap();
    assert!(result.revealed && result.partial && result.limit_clamped);
    assert!(
        host.focused().is_none(),
        "explicit reveal does not transfer focus"
    );
}

#[test]
fn impossible_curved_scroll_witness_leaves_focus_and_offset_unchanged() {
    use tekes_ui_controls::ScrollArea;
    let scroll_id = ComponentId(8152);
    let space = std::rc::Rc::new(std::cell::RefCell::new(RequiredFocusSpace::Unmeasured));
    let focused = std::rc::Rc::new(std::cell::Cell::new(false));
    let probe = FocusSpaceProbe {
        space: space.clone(),
        focused: focused.clone(),
        content_height: Some(180.),
    };
    let mut scroll = ScrollContent::new(ScrollArea::default(), Content::new(CHILD, probe));
    scroll.viewport_height = Some(80.);
    let mut host = ContentHost::new(Content::new(
        ROOT,
        panel(Content::new(scroll_id, scroll), true),
    ))
    .unwrap();
    let mut text = Text::new(true);
    render(&mut host, &mut text, Rect::new(0., 0., 120., 100.));
    let target = host.get(CHILD).unwrap().frame();
    let shape = host
        .get(ROOT)
        .unwrap()
        .component::<PanelContent>()
        .unwrap()
        .geometry()
        .unwrap()
        .shape;
    // The witness fills the viewport's full width and exactly 100 px vertically.
    // Its only vertical landing inside the Panel's 100 px outer bounds is y=0;
    // at that landing both x=5/95 top corners are outside the rounded domain.
    let witness = Rect::new(target.min_x(), target.min_y(), target.size.width, 100.);
    assert_eq!(target.intersection(&witness), Some(witness));
    assert_eq!(shape.contains([f64::from(witness.min_x()), 0.]), Ok(false));
    assert_eq!(shape.contains([f64::from(witness.max_x()), 0.]), Ok(false));
    *space.borrow_mut() = RequiredFocusSpace::Sufficient {
        witness,
        evidence: RequiredFocusEvidence::new(),
    };
    let before = host
        .get(scroll_id)
        .unwrap()
        .component::<ScrollContent>()
        .unwrap()
        .scroll
        .offset;
    assert!(
        host.plan_host_reveal(CHILD, Default::default(), &[], HostRevealBudget::default())
            .is_err()
    );
    assert!(!host.focus(Some(CHILD), true));
    assert_eq!(
        host.get(scroll_id)
            .unwrap()
            .component::<ScrollContent>()
            .unwrap()
            .scroll
            .offset,
        before
    );
    assert_eq!(host.focused(), None);
    assert!(!focused.get());
}

#[test]
fn target_focus_evidence_remains_independent_from_certified_panel_boundary() {
    use tekes_ui_controls::ScrollRevealOptions;
    let (mut host, space, _) = focus_space_host(RequiredFocusSpace::Unmeasured);
    let mut text = Text::new(true);
    render(&mut host, &mut text, Rect::new(0., 0., 120., 100.));
    let target = host.get(CHILD).unwrap().frame();
    let witness = Rect::new(target.min_x() + 40., target.min_y() + 25., 12., 10.);
    *space.borrow_mut() = RequiredFocusSpace::Sufficient {
        witness,
        evidence: RequiredFocusEvidence::new(),
    };
    let plan = host.plan_host_reveal(
        CHILD,
        ScrollRevealOptions::default(),
        &[],
        HostRevealBudget::default(),
    );
    assert!(
        plan.is_ok(),
        "target supplies its original witness; it does not supply Panel geometry"
    );

    for status in [
        RequiredFocusSpace::Insufficient,
        RequiredFocusSpace::Unmeasured,
    ] {
        *space.borrow_mut() = status;
        assert_eq!(
            host.plan_host_reveal(
                CHILD,
                ScrollRevealOptions::default(),
                &[],
                HostRevealBudget::default()
            )
            .unwrap_err(),
            HostRevealError::NoFeasibleRegion,
        );
    }
}

#[test]
fn rounded_panel_corner_rejects_required_witness_inside_bbox_but_outside_curve() {
    use tekes_ui_controls::ScrollRevealOptions;
    let (mut host, space, _) = focus_space_host(RequiredFocusSpace::Unmeasured);
    let mut text = Text::new(true);
    render(&mut host, &mut text, Rect::new(0., 0., 120., 100.));
    let shape = host
        .get(ROOT)
        .unwrap()
        .component::<PanelContent>()
        .unwrap()
        .geometry()
        .unwrap()
        .shape;
    let outer = shape.rect();
    let radius = shape.radii().top_left;
    assert!(radius > 0.);
    let witness = Rect::new(
        outer.min_x() + radius * 0.05,
        outer.min_y() + radius * 0.05,
        1.,
        1.,
    );
    assert!(
        outer.intersection(&witness).is_some(),
        "witness is in the rectangular bbox"
    );
    *space.borrow_mut() = RequiredFocusSpace::Sufficient {
        witness,
        evidence: RequiredFocusEvidence::new(),
    };
    assert_eq!(
        host.plan_host_reveal(
            CHILD,
            ScrollRevealOptions::default(),
            &[],
            HostRevealBudget::default()
        )
        .unwrap_err(),
        HostRevealError::NoFeasibleRegion,
    );
}

#[test]
fn changed_target_focus_evidence_rejects_plan_before_focus_or_scroll_mutation() {
    use tekes_ui_controls::ScrollRevealOptions;
    let (mut host, space, focused) = focus_space_host(RequiredFocusSpace::Unmeasured);
    let mut text = Text::new(true);
    render(&mut host, &mut text, Rect::new(0., 0., 120., 100.));
    let target = host.get(CHILD).unwrap().frame();
    let witness = Rect::new(target.min_x() + 40., target.min_y() + 25., 12., 10.);
    *space.borrow_mut() = RequiredFocusSpace::Sufficient {
        witness,
        evidence: RequiredFocusEvidence::new(),
    };
    let plan = host
        .plan_host_reveal(
            CHILD,
            ScrollRevealOptions::default(),
            &[],
            HostRevealBudget::default(),
        )
        .unwrap();
    *space.borrow_mut() = RequiredFocusSpace::Sufficient {
        witness,
        evidence: RequiredFocusEvidence::new(),
    };
    assert_eq!(
        host.commit_host_reveal(plan),
        Err(HostRevealError::StalePlan)
    );
    assert!(!focused.get());
    assert_eq!(host.focused(), None);
}
