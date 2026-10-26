//! Public original Host + real original text editor; no native window or GPU.
use std::rc::Rc;
use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, Insets, ScrollArea, ScrollRevealOptions,
    ScrollRevealPlacement, TextSurfaceHost, TextSurfaceTextRenderer,
    composition::*,
    extended::{Axis, Stack},
};
use tekes_ui_scene::{ClipBudget, ClipRegion, ClipShape, CornerRadii, Point, Rect, Scene, Size};
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

const ROOT: ComponentId = ComponentId(9800);
const SCROLL: ComponentId = ComponentId(9801);
const ITEMS: ComponentId = ComponentId(9802);
const FIELD: ComponentId = ComponentId(9803);
const TARGET: ComponentId = ComponentId(9804);
fn column(id: ComponentId, children: Vec<Content>) -> Content {
    Content::new(
        id,
        StackContent {
            model: Stack {
                axis: Axis::Vertical,
                gap: 0.,
            },
            flexible_child: None,
            children,
        },
    )
}
fn setup() -> (ContentHost, Text) {
    let rows = column(
        ITEMS,
        (0..20)
            .map(|i| Content::new(ComponentId(9850 + i), TextContent::new(format!("row {i}"))))
            .chain([Content::new(TARGET, ButtonContent::new("Target"))])
            .collect(),
    );
    let mut scroll = ScrollContent::new(ScrollArea::default(), rows);
    scroll.viewport_height = Some(100.);
    let mut h = ContentHost::new(column(
        ROOT,
        vec![
            Content::new(SCROLL, scroll),
            Content::new(FIELD, TextFieldContent::new("Composer", "kept 😀 draft")),
        ],
    ))
    .unwrap();
    h.set_scroll_reading_document(SCROLL, "original", 1, vec![TARGET], |_| {})
        .unwrap();
    h.set_action_events(true);
    let mut t = Text::new(true);
    paint(&mut h, &mut t);
    assert!(h.focus(Some(FIELD), true));
    paint(&mut h, &mut t);
    h.event_with_text(&ContentEvent::TextCommand("selectAll:".into()), &mut t)
        .unwrap();
    paint(&mut h, &mut t);
    let input = &h.text_surface(FIELD).unwrap().input;
    assert_ne!(input.anchor, input.caret);
    assert!(h.focused_text_binding().is_some());
    (h, t)
}
fn paint(h: &mut ContentHost, t: &mut Text) {
    let f = Rect::new(0., 0., 300., 160.);
    h.render(&mut Scene::new(), t, f, f, Theme::LIGHT.into())
        .unwrap();
}
fn options() -> ScrollRevealOptions {
    ScrollRevealOptions {
        placement: ScrollRevealPlacement::Nearest,
        margins: Insets::ZERO,
        animated: false,
    }
}
fn offset(h: &ContentHost) -> Point {
    h.get(SCROLL)
        .unwrap()
        .component::<ScrollContent>()
        .unwrap()
        .scroll
        .offset
}
#[derive(Debug, PartialEq)]
struct RetainedState {
    focused: Option<ComponentId>,
    focus_epoch: u64,
    offset: Point,
    text: String,
    marked: Option<std::ops::Range<usize>>,
    selection: (usize, usize),
    binding: Option<(u64, u64)>,
}
fn state(h: &ContentHost) -> RetainedState {
    let s = h.text_surface(FIELD).unwrap();
    RetainedState {
        focused: h.focused(),
        focus_epoch: h.focus_epoch(),
        offset: offset(h),
        text: s.input.text.clone(),
        marked: s.input.marked.clone(),
        selection: (s.input.anchor, s.input.caret),
        binding: h.focused_text_binding(),
    }
}

#[test]
fn readonly_plan_commit_and_real_editor_preserved() {
    let (mut h, mut t) = setup();
    let before = state(&h);
    let mount = h.semantic_mount(FIELD);
    let plan = h
        .plan_host_reveal(TARGET, options(), &[], HostRevealBudget::default())
        .unwrap();
    assert_eq!(state(&h), before);
    let out = h.commit_host_reveal(plan).unwrap();
    assert!(out.changed);
    assert_eq!(h.focused(), before.focused);
    assert_eq!(h.semantic_mount(FIELD), mount);
    assert_eq!(h.text_surface(FIELD).unwrap().input.text, before.text);
    let immediate = state(&h);
    assert_eq!(immediate.selection, before.selection);
    assert_eq!(immediate.binding, before.binding);
    paint(&mut h, &mut t);
    assert!(h.get(TARGET).unwrap().visible().is_some());
    let input = &h.text_surface(FIELD).unwrap().input;
    assert_eq!((input.anchor, input.caret), before.selection);
    assert_eq!(h.focused_text_binding(), before.binding);
}
#[test]
fn stale_input_geometry_mount_version_offset_and_foreign_host_reject_atomically() {
    for change in 0..6 {
        let (mut h, mut t) = setup();
        let plan = h
            .plan_host_reveal(TARGET, options(), &[], HostRevealBudget::default())
            .unwrap();
        match change {
            0 => {
                h.event(&ContentEvent::PointerMove(Point::new(-1., -1.)));
            }
            1 => paint(&mut h, &mut t),
            2 => {
                let _ = h.remove(TARGET).unwrap();
                h.insert(ITEMS, Content::new(TARGET, ButtonContent::new("replacement")))
                    .unwrap();
            }
            3 => {
                h.set_scroll_reading_document(SCROLL, "original", 2, vec![TARGET], |_| {})
                    .unwrap();
            }
            4 => {
                h.get_mut(SCROLL)
                    .unwrap()
                    .component_mut::<ScrollContent>()
                    .unwrap()
                    .scroll
                    .scroll_by(0., 3.);
            }
            _ => {
                let (mut foreign, _) = setup();
                let before = state(&foreign);
                assert_eq!(
                    foreign.commit_host_reveal(plan),
                    Err(HostRevealError::StalePlan)
                );
                assert_eq!(state(&foreign), before);
                continue;
            }
        }
        let before = state(&h);
        assert_eq!(h.commit_host_reveal(plan), Err(HostRevealError::StalePlan));
        assert_eq!(state(&h), before);
    }
}
#[test]
fn unsupported_budget_and_curve_impossibility_do_not_mutate() {
    let (h, _) = setup();
    let before = state(&h);
    let shape = ClipShape::normalize(
        Rect::new(-100., -100., 1., 1.),
        CornerRadii::ZERO,
        Point::default(),
    )
    .unwrap();
    let region = ClipRegion::new(
        &[shape],
        ClipBudget {
            max_shapes: 1,
            max_work: 10000,
        },
    )
    .unwrap();
    assert!(matches!(
        h.plan_host_reveal(
            TARGET,
            options(),
            &[(SCROLL, region)],
            HostRevealBudget::default()
        ),
        Err(HostRevealError::NoFeasibleRegion)
    ));
    let budget = HostRevealBudget {
        max_path_nodes: usize::MAX,
        max_passes: usize::MAX,
        max_queries: 0,
        max_geometry_work: usize::MAX,
    };
    assert!(matches!(
        h.plan_host_reveal(TARGET, options(), &[], budget),
        Err(HostRevealError::PlanBudgetExceeded)
    ));
    assert_eq!(state(&h), before);
}
#[test]
fn ime_transfer_barrier_keeps_marked_owner_and_scroll() {
    let (mut h, mut t) = setup();
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
    let before = state(&h);
    assert!(h.is_composing());
    assert!(!h.enter_focus(false));
    assert_eq!(state(&h), before);
    assert!(
        !h.accessibility_action(TARGET, None, AccessibilityAction::Focus)
            .changed
    );
    assert_eq!(state(&h), before);
    let version = h
        .get(SCROLL)
        .unwrap()
        .component::<ScrollContent>()
        .unwrap()
        .reading_document_version()
        .unwrap()
        .clone();
    h.reveal_scroll_reading_item(SCROLL, &version, TARGET, options(), &mut t)
        .unwrap();
    assert_eq!(h.focused(), before.focused);
    assert_eq!(h.text_surface(FIELD).unwrap().input.marked, before.marked);
    let input = &h.text_surface(FIELD).unwrap().input;
    assert_eq!((input.anchor, input.caret), before.selection);
    assert_eq!(h.focused_text_binding(), before.binding);
}
#[test]
fn nested_original_scrolls_plan_without_layout_or_trial_mutation() {
    let mut inner = ScrollContent::new(
        ScrollArea::default(),
        column(
            ITEMS,
            (0..15)
                .map(|i| {
                    Content::new(
                        ComponentId(9900 + i),
                        TextContent::new(format!("inner {i}")),
                    )
                })
                .chain([Content::new(TARGET, ButtonContent::new("Nested target"))])
                .collect(),
        ),
    );
    inner.viewport_height = Some(70.);
    let rows = column(
        ComponentId(9990),
        (0..12)
            .map(|i| {
                Content::new(
                    ComponentId(9950 + i),
                    TextContent::new(format!("outer {i}")),
                )
            })
            .chain([Content::new(SCROLL, inner)])
            .collect(),
    );
    let mut outer = ScrollContent::new(ScrollArea::default(), rows);
    outer.viewport_height = Some(110.);
    let mut h = ContentHost::new(Content::new(ROOT, outer)).unwrap();
    let mut t = Text::new(true);
    paint(&mut h, &mut t);
    let before = (
        h.get(ROOT)
            .unwrap()
            .component::<ScrollContent>()
            .unwrap()
            .scroll
            .offset,
        offset(&h),
        h.get(TARGET).unwrap().frame(),
    );
    let p = h
        .plan_host_reveal(TARGET, options(), &[], HostRevealBudget::default())
        .unwrap();
    assert_eq!(
        before,
        (
            h.get(ROOT)
                .unwrap()
                .component::<ScrollContent>()
                .unwrap()
                .scroll
                .offset,
            offset(&h),
            h.get(TARGET).unwrap().frame()
        )
    );
    assert!(h.commit_host_reveal(p).unwrap().changed);
    paint(&mut h, &mut t);
    assert!(h.get(TARGET).unwrap().visible().is_some());
}
#[test]
fn late_plan_failure_does_not_commit_first_nested_offset() {
    let (mut h, _) = setup();
    let before = state(&h);
    let budget = HostRevealBudget {
        max_path_nodes: usize::MAX,
        max_passes: 1,
        max_queries: usize::MAX,
        max_geometry_work: usize::MAX,
    };
    assert!(matches!(
        h.plan_host_reveal(TARGET, options(), &[], budget),
        Err(HostRevealError::PlanBudgetExceeded)
    ));
    assert_eq!(state(&h), before);
    assert!(h.drain_action_records().is_empty());
}

#[test]
fn real_curve_constraint_lands_original_target_and_preserves_editor() {
    let (mut h, mut t) = setup();
    h.insert(
        ITEMS,
        Content::new(ComponentId(9998), TextContent::new("trailing room")),
    )
    .unwrap();
    h.insert(
        ITEMS,
        Content::new(ComponentId(9999), TextContent::new("trailing room")),
    )
    .unwrap();
    paint(&mut h, &mut t);
    let before = state(&h);
    let region = ClipRegion::new(
        &[ClipShape::normalize(
            Rect::new(4., 0., 292., 100.),
            CornerRadii::uniform(20.),
            Point::default(),
        )
        .unwrap()],
        ClipBudget {
            max_shapes: 1,
            max_work: 10000,
        },
    )
    .unwrap();
    let p = h
        .plan_host_reveal(
            TARGET,
            options(),
            &[(SCROLL, region.clone())],
            HostRevealBudget::default(),
        )
        .unwrap();
    let out = h.commit_host_reveal(p).unwrap();
    assert!(out.shape_clamped);
    assert!(!out.partial && !out.limit_clamped);
    paint(&mut h, &mut t);
    let r = h.get(TARGET).unwrap().frame();
    for p in [
        [r.min_x(), r.min_y()],
        [r.max_x(), r.min_y()],
        [r.min_x(), r.max_y()],
        [r.max_x(), r.max_y()],
    ] {
        assert!(region.contains(p.map(f64::from)).unwrap());
    }
    assert_eq!(h.focused(), before.focused);
    assert_eq!(h.text_surface(FIELD).unwrap().input.text, before.text);
}

#[test]
fn curved_document_end_is_explicitly_partial_and_limit_clamped() {
    let (mut h, _) = setup();
    let region = ClipRegion::new(
        &[ClipShape::normalize(
            Rect::new(4., 0., 292., 100.),
            CornerRadii::uniform(20.),
            Point::default(),
        )
        .unwrap()],
        ClipBudget {
            max_shapes: 1,
            max_work: 10000,
        },
    )
    .unwrap();
    let p = h
        .plan_host_reveal(
            TARGET,
            options(),
            &[(SCROLL, region)],
            HostRevealBudget::default(),
        )
        .unwrap();
    let out = h.commit_host_reveal(p).unwrap();
    assert!(out.partial && out.limit_clamped && out.revealed);
}
#[derive(Debug)]
struct DeclaredCurve {
    children: Vec<Content>,
}
impl Component for DeclaredCurve {
    fn children(&self) -> &[Content] {
        &self.children
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.children)
    }
    fn requires_focus_reveal(&self) -> bool {
        true
    }
    fn measure(
        &mut self,
        t: &mut dyn ButtonTextRenderer,
        s: Size,
    ) -> tekes_ui_controls::layout::MeasuredChild {
        self.children[0].measure(t, s)
    }
    fn arrange(&mut self, t: &mut dyn ButtonTextRenderer, f: Rect, c: Rect) {
        self.children[0].arrange(t, f, c);
    }
}
#[test]
fn structural_curve_requirement_cannot_fall_into_legacy_focus_even_before_layout() {
    let mut h = ContentHost::new(Content::new(
        ROOT,
        DeclaredCurve {
            children: vec![Content::new(TARGET, ButtonContent::new("target"))],
        },
    ))
    .unwrap();
    assert!(!h.focus(Some(TARGET), true));
    assert_eq!(h.focused(), None);
    let mut t = Text::new(true);
    paint(&mut h, &mut t);
    assert!(matches!(
        h.plan_host_reveal(TARGET, options(), &[], HostRevealBudget::default()),
        Err(HostRevealError::UnsupportedLayout)
    ));
    assert!(!h.focus(Some(TARGET), true));
    assert_eq!(h.focused(), None);
    h.get_mut(TARGET).unwrap();
    assert!(!h.focus(Some(TARGET), true));
    assert_eq!(h.focused(), None);
}

#[derive(Debug)]
struct OpaqueReveal {
    children: Vec<Content>,
    arranges: Rc<std::cell::Cell<usize>>,
    reveals: Rc<std::cell::Cell<usize>>,
}
impl Component for OpaqueReveal {
    fn children(&self) -> &[Content] {
        &self.children
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.children)
    }
    fn measure(
        &mut self,
        t: &mut dyn ButtonTextRenderer,
        s: Size,
    ) -> tekes_ui_controls::layout::MeasuredChild {
        self.children[0].measure(t, s)
    }
    fn arrange(&mut self, t: &mut dyn ButtonTextRenderer, f: Rect, c: Rect) {
        self.arranges.set(self.arranges.get() + 1);
        self.children[0].arrange(t, f, c);
    }
    fn reveal(&mut self, r: Rect) -> Rect {
        self.reveals.set(self.reveals.get() + 1);
        r
    }
}
#[test]
fn opaque_custom_reveal_is_never_invoked_as_a_readonly_probe_or_error_fallback() {
    let arranges = Rc::new(std::cell::Cell::new(0));
    let reveals = Rc::new(std::cell::Cell::new(0));
    let rows = column(
        ITEMS,
        (0..20)
            .map(|i| Content::new(ComponentId(9850 + i), TextContent::new("original row")))
            .chain([Content::new(TARGET, ButtonContent::new("Target"))])
            .collect(),
    );
    let opaque = Content::new(
        ComponentId(9840),
        OpaqueReveal {
            children: vec![rows],
            arranges: arranges.clone(),
            reveals: reveals.clone(),
        },
    );
    let mut scroll = ScrollContent::new(ScrollArea::default(), opaque);
    scroll.viewport_height = Some(100.);
    let mut h = ContentHost::new(column(
        ROOT,
        vec![
            Content::new(SCROLL, scroll),
            Content::new(FIELD, TextFieldContent::new("Composer", "retained")),
        ],
    ))
    .unwrap();
    let mut t = Text::new(true);
    paint(&mut h, &mut t);
    assert!(h.focus(Some(FIELD), true));
    let before = state(&h);
    let arranged = arranges.get();
    assert!(matches!(
        h.plan_host_reveal(TARGET, options(), &[], HostRevealBudget::default()),
        Err(HostRevealError::UnsupportedLayout)
    ));
    assert!(!h.focus(Some(TARGET), true));
    assert_eq!(state(&h), before);
    assert_eq!(arranges.get(), arranged);
    assert_eq!(reveals.get(), 0);
}
#[test]
fn original_host_invalid_margins_never_mutate_any_owner_or_offset() {
    let (h, _) = setup();
    let before = state(&h);
    for margins in [
        Insets::new(-1., 0., 0., 0.),
        Insets::new(f32::NAN, 0., 0., 0.),
        Insets::new(f32::MAX, 0., f32::MAX, 0.),
    ] {
        let result = h.plan_host_reveal(
            TARGET,
            ScrollRevealOptions {
                margins,
                ..options()
            },
            &[],
            HostRevealBudget::default(),
        );
        assert!(matches!(
            result,
            Err(HostRevealError::InvalidOptions(
                tekes_ui_controls::ScrollRevealError::InvalidMargins
            ))
        ));
        assert_eq!(state(&h), before);
    }
}

#[test]
fn original_reading_entry_retains_its_invalid_options_error_contract() {
    let (mut h, mut t) = setup();
    let before = state(&h);
    let version = h
        .get(SCROLL)
        .unwrap()
        .component::<ScrollContent>()
        .unwrap()
        .reading_document_version()
        .unwrap()
        .clone();
    let result = h.reveal_scroll_reading_item(
        SCROLL,
        &version,
        TARGET,
        ScrollRevealOptions {
            margins: Insets::new(f32::NAN, 0., 0., 0.),
            ..options()
        },
        &mut t,
    );
    assert_eq!(
        result,
        Err(ContentRevealError::InvalidOptions(
            tekes_ui_controls::ScrollRevealError::InvalidMargins
        ))
    );
    assert_eq!(state(&h), before);
}

#[derive(Debug)]
struct MutableGetterProbe {
    children: Vec<Content>,
    armed: Rc<std::cell::Cell<bool>>,
    calls: Rc<std::cell::Cell<usize>>,
}
impl Component for MutableGetterProbe {
    fn children(&self) -> &[Content] {
        &self.children
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        if self.armed.get() {
            let count = self.calls.get() + 1;
            self.calls.set(count);
            // A legitimate mutable getter really changes the owned tree and
            // allocates a real original Content/mount, rather than a fake flag.
            self.children.push(Content::new(
                ComponentId(200_000 + count as u64),
                TextContent::new("minted by unplanned mutable getter"),
            ));
        }
        Some(&mut self.children)
    }
    fn measure(
        &mut self,
        _: &mut dyn ButtonTextRenderer,
        s: Size,
    ) -> tekes_ui_controls::layout::MeasuredChild {
        tekes_ui_controls::layout::MeasuredChild::new(Size::new(s.width, 0.))
    }
}
fn getter_probe() -> (
    Content,
    Rc<std::cell::Cell<bool>>,
    Rc<std::cell::Cell<usize>>,
) {
    let armed = Rc::new(std::cell::Cell::new(false));
    let calls = Rc::new(std::cell::Cell::new(0));
    (
        Content::new(
            ComponentId(9849),
            MutableGetterProbe {
                children: Vec::new(),
                armed: armed.clone(),
                calls: calls.clone(),
            },
        ),
        armed,
        calls,
    )
}
fn assert_probe_untouched(h: &ContentHost, calls: &std::cell::Cell<usize>) {
    assert_eq!(
        calls.get(),
        0,
        "unplanned sibling mutable getter was invoked"
    );
    assert!(
        h.get(ComponentId(9849))
            .unwrap()
            .component::<MutableGetterProbe>()
            .unwrap()
            .children
            .is_empty(),
        "unplanned Content was created in the original tree"
    );
}
#[test]
fn explicit_reveal_ignores_unplanned_mutable_getter_siblings() {
    let (mut h, mut t) = setup();
    let (probe, armed, calls) = getter_probe();
    h.insert(ROOT, probe).unwrap();
    h.reorder(ROOT, &[ComponentId(9849), SCROLL, FIELD])
        .unwrap();
    paint(&mut h, &mut t);
    let before = state(&h);
    let version = h
        .get(SCROLL)
        .unwrap()
        .component::<ScrollContent>()
        .unwrap()
        .reading_document_version()
        .unwrap()
        .clone();
    armed.set(true);
    h.reveal_scroll_reading_item(SCROLL, &version, TARGET, options(), &mut t)
        .unwrap();
    assert_probe_untouched(&h, &calls);
    // Normal rendering is outside the transaction. Do not arm its intentionally
    // mutable lifecycle/layout traversal as a false commit-path probe.
    armed.set(false);
    paint(&mut h, &mut t);
    armed.set(true);
    h.reveal_scroll_reading_item(SCROLL, &version, TARGET, options(), &mut t)
        .unwrap();
    assert_probe_untouched(&h, &calls);
    let after = state(&h);
    assert_eq!(after.focused, before.focused);
    assert_eq!(after.selection, before.selection);
    assert_eq!(after.binding, before.binding);
}
#[test]
fn nested_scroll_commit_ignores_unplanned_mutable_getter_between_owners() {
    let (probe, armed, calls) = getter_probe();
    let mut inner = ScrollContent::new(
        ScrollArea::default(),
        column(
            ITEMS,
            (0..15)
                .map(|i| Content::new(ComponentId(9900 + i), TextContent::new("inner row")))
                .chain([Content::new(TARGET, ButtonContent::new("Nested target"))])
                .collect(),
        ),
    );
    inner.viewport_height = Some(70.);
    let rows = column(
        ComponentId(9990),
        (0..12)
            .map(|i| Content::new(ComponentId(9950 + i), TextContent::new("outer row")))
            .chain([probe, Content::new(SCROLL, inner)])
            .collect(),
    );
    let mut outer = ScrollContent::new(ScrollArea::default(), rows);
    outer.viewport_height = Some(110.);
    let mut h = ContentHost::new(Content::new(ROOT, outer)).unwrap();
    let mut t = Text::new(true);
    paint(&mut h, &mut t);
    armed.set(true);
    let p = h
        .plan_host_reveal(TARGET, options(), &[], HostRevealBudget::default())
        .unwrap();
    assert!(h.commit_host_reveal(p).unwrap().changed);
    assert!(
        h.get(ROOT)
            .unwrap()
            .component::<ScrollContent>()
            .unwrap()
            .scroll
            .offset
            .y
            > 0.
    );
    assert!(offset(&h).y > 0.);
    assert_probe_untouched(&h, &calls);
}
#[test]
fn focus_old_and_new_owner_paths_ignore_unplanned_mutable_getter_siblings() {
    for before_old in [true, false] {
        let (probe, armed, calls) = getter_probe();
        let target = Content::new(TARGET, ButtonContent::new("New owner"));
        let editor = Content::new(
            FIELD,
            TextFieldContent::new("Old owner", "kept original draft"),
        );
        let children = if before_old {
            vec![target, probe, editor]
        } else {
            vec![editor, probe, target]
        };
        let mut h = ContentHost::new(column(ROOT, children)).unwrap();
        let mut t = Text::new(true);
        paint(&mut h, &mut t);
        assert!(h.focus(Some(FIELD), true));
        h.event_with_text(&ContentEvent::TextCommand("selectAll:".into()), &mut t)
            .unwrap();
        paint(&mut h, &mut t);
        let mount = h.semantic_mount(FIELD);
        let value = h.text_surface(FIELD).unwrap().input.text.clone();
        let selection = (
            h.text_surface(FIELD).unwrap().input.anchor,
            h.text_surface(FIELD).unwrap().input.caret,
        );
        armed.set(true);
        assert!(h.focus(Some(TARGET), true));
        assert_eq!(h.focused(), Some(TARGET));
        assert_probe_untouched(&h, &calls);
        assert_eq!(h.semantic_mount(FIELD), mount);
        assert_eq!(h.text_surface(FIELD).unwrap().input.text, value);
        assert_eq!(
            (
                h.text_surface(FIELD).unwrap().input.anchor,
                h.text_surface(FIELD).unwrap().input.caret
            ),
            selection
        );
    }
}
