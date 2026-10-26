//! Public original Host + real original text editor; no native window or GPU.
use std::rc::Rc;
use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, Insets, TextSurfaceHost, TextSurfaceTextRenderer,
    composition::*,
    extended::{Axis, Stack},
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
#[test]
fn original_route_gap_capture_identity_and_atomic_fragment() {
    let mut h = ContentHost::new(Content::new(
        ROOT,
        panel(
            Content::new(CHILD, ButtonContent::new("real button"))
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
            .component::<ButtonContent>()
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
            .component::<ButtonContent>()
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
        panel(Content::new(CHILD, ButtonContent::new("button")), false),
    ))
    .unwrap();
    assert!(!h.focus(Some(CHILD), true));
    let mut t = Text::new(true);
    render(&mut h, &mut t, Rect::new(0., 0., 100., 100.));
    assert!(h.focus(Some(CHILD), true));
    h.focus(None, false);
    let _ = h.get_mut(ROOT);
    assert!(!h.focus(Some(CHILD), true));
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
                Content::new(button, ButtonContent::new("original action")),
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

#[test]
fn legacy_rect_native_adapter_refuses_actual_curve_authority() {
    use tekes_ui_platform_macos::content_accessibility::{content_action, content_key, content_nodes};
    let root = ComponentId(8490);
    let child = ComponentId(8491);
    let content = panel(Content::new(child, ButtonContent::new("Curve action")), false);
    let mut host = ContentHost::new(Content::new(root, content)).unwrap();
    render(&mut host, &mut Text::new(true), Rect::new(0., 0., 240., 180.));
    let key = content_key(&host, child, None).unwrap();
    assert!(host.requires_curve_geometry());
    assert!(!host.semantics().is_empty(), "real current Content tree remains available");
    assert!(content_nodes(&host).is_empty(), "legacy Rect publication cannot grant curve point authority");
    let response = content_action(&mut host, key, tekes_ui_platform_macos::AccessibilityAction::Press);
    assert!(!response.consumed && !response.activated);
    assert_eq!(host.get(child).unwrap().component::<ButtonContent>().unwrap().activations, 0);
}
