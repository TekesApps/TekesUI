//! True nested mixed Content consumer; product code passes IDs, not offsets.
use std::rc::Rc;
use tekes_ui_composition_consumer::item_navigation::*;
use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, Insets, ReadingUpdateError, ScrollRevealOptions,
    ScrollRevealPlacement, TextSurfaceHost, TextSurfaceTextRenderer, composition::*,
};
use tekes_ui_scene::{Point, Rect, Scene, Size};
use tekes_ui_text::{
    Paragraph, ShapeOptions, Shaper, TextStyle,
    coretext::{CoreTextFonts, CoreTextShaper},
};
use tekes_ui_tokens::{Rgba, Spacing, Theme, proposal};
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

fn frame() -> Rect {
    Rect::new(0., 0., 520., 560.)
}
fn paint(f: &mut Fixture, t: &mut Text) {
    f.host
        .render(&mut Scene::new(), t, frame(), frame(), Theme::LIGHT.into())
        .unwrap();
}
fn setup() -> (Fixture, Text) {
    let mut f = Fixture::new().unwrap();
    let mut t = Text::new(true);
    paint(&mut f, &mut t);
    (f, t)
}
fn opts(placement: ScrollRevealPlacement) -> ScrollRevealOptions {
    ScrollRevealOptions {
        placement,
        margins: Insets::new(
            Spacing::Regular.value(),
            Spacing::Regular.value(),
            Spacing::Regular.value(),
            Spacing::Regular.value(),
        ),
        animated: false,
    }
}
fn offset(f: &Fixture, id: ComponentId) -> Point {
    f.host
        .get(id)
        .unwrap()
        .component::<ScrollContent>()
        .unwrap()
        .scroll
        .offset
}
fn mark_composer(f: &mut Fixture) -> tekes_ui_controls::TextEditBaseline {
    f.host.focus(Some(COMPOSER), true);
    f.host.event(&ContentEvent::Commit {
        text: "X".into(),
        replacement: None,
    });
    f.host.event(&ContentEvent::Preedit {
        text: "ni".into(),
        selection: 2..2,
        clauses: std::iter::once(0..2).collect(),
        replacement: None,
    });
    f.host.focused_text().unwrap().input.external_baseline()
}
fn composer_unchanged(f: &Fixture, b: &tekes_ui_controls::TextEditBaseline) {
    assert_eq!(f.host.focused(), Some(COMPOSER));
    assert_eq!(&f.host.focused_text().unwrap().input.external_baseline(), b);
}
#[test]
fn declared_nested_item_reveal_preserves_original_composer_and_only_moves_target_ancestors() {
    let (mut f, mut t) = setup();
    let b = mark_composer(&mut f);
    let other = offset(&f, OTHER);
    assert!(f.host.get(TARGET).unwrap().visible().is_none());
    let r = f
        .navigate(TARGET, opts(ScrollRevealPlacement::Nearest), &mut t)
        .unwrap();
    assert_eq!(r.status, ContentRevealStatus::Completed);
    assert!(r.changed);
    composer_unchanged(&f, &b);
    paint(&mut f, &mut t);
    composer_unchanged(&f, &b);
    assert!(f.host.get(TARGET).unwrap().visible().is_some());
    assert!(offset(&f, OWNER).y > 0. && offset(&f, INNER).y > 0.);
    assert_eq!(offset(&f, OTHER), other);
    assert_eq!(
        f.host
            .get(TARGET)
            .unwrap()
            .component::<Button>()
            .unwrap()
            .activations,
        0
    );
}
#[test]
fn nested_centered_placement_is_actual_item_geometry_not_a_focused_text_substitute() {
    let (mut f, mut t) = setup();
    let b = mark_composer(&mut f);
    let r = f
        .navigate(TARGET, opts(ScrollRevealPlacement::Centered), &mut t)
        .unwrap();
    assert_eq!(r.status, ContentRevealStatus::Completed);
    let target = f.host.get(TARGET).unwrap().frame().center();
    for owner in [INNER, OWNER] {
        let center = f.host.get(owner).unwrap().frame().center();
        assert!(
            (target.y - center.y).abs() < 0.02,
            "{owner:?}: target={target:?},center={center:?}"
        );
    }
    composer_unchanged(&f, &b);
}
#[test]
fn unlaid_out_removed_foreign_and_undeclared_items_return_explicit_errors() {
    for navigation in [opts(ScrollRevealPlacement::Nearest), start_options()] {
        let mut f = Fixture::new().unwrap();
        let mut t = Text::new(true);
        assert_eq!(
            f.navigate(TARGET, navigation, &mut t),
            Err(ContentRevealError::UnlaidOut)
        );
        paint(&mut f, &mut t);
        let initial = offset(&f, OWNER);
        assert_eq!(
            f.host
                .reveal_scroll_reading_item(OTHER, &f.version, TARGET, navigation, &mut t),
            Err(ContentRevealError::Reading(
                ReadingUpdateError::WrongDocument
            ))
        );
        assert_eq!(
            f.navigate(COMPOSER, navigation, &mut t),
            Err(ContentRevealError::UndeclaredItem)
        );
        let _retained = f.host.remove(TARGET).unwrap();
        assert_eq!(
            f.navigate(TARGET, navigation, &mut t),
            Err(ContentRevealError::MissingItem)
        );
        assert_eq!(offset(&f, OWNER), initial);
    }
}
#[test]
fn attachment_aba_and_old_revision_do_not_reveal_same_numeric_ids_in_another_document() {
    for navigation in [opts(ScrollRevealPlacement::Nearest), start_options()] {
        let (mut f, mut t) = setup();
        let old = f.version.clone();
        let (next, ()) = f
            .host
            .update_scroll_reading_document(OWNER, &old, 2, f.items.clone(), |_| {})
            .unwrap();
        assert_eq!(
            f.navigate(TARGET, navigation, &mut t),
            Err(ContentRevealError::Reading(
                ReadingUpdateError::StaleRevision
            ))
        );
        f.version = next;
        assert_eq!(
            f.navigate(TARGET, navigation, &mut t),
            Err(ContentRevealError::UnlaidOut)
        );
        paint(&mut f, &mut t);
        f.host
            .set_scroll_reading_document(OWNER, "document-B", 1, f.items.clone(), |_| {})
            .unwrap();
        let (current, ()) = f
            .host
            .set_scroll_reading_document(OWNER, "document-A", 2, f.items.clone(), |_| {})
            .unwrap();
        paint(&mut f, &mut t);
        assert_eq!(
            f.navigate(TARGET, navigation, &mut t),
            Err(ContentRevealError::Reading(
                ReadingUpdateError::WrongDocument
            ))
        );
        f.version = current;
        assert_eq!(
            f.navigate(TARGET, navigation, &mut t).unwrap().status,
            ContentRevealStatus::Completed
        );
    }
}
#[test]
fn replacing_a_mounted_item_needs_a_new_document_revision_even_after_layout() {
    for navigation in [opts(ScrollRevealPlacement::Nearest), start_options()] {
        let (mut f, mut t) = setup();
        let _old = f.host.remove(TARGET).unwrap();
        f.host
            .insert(
                INNER_ITEMS,
                Content::new(TARGET, Button::new("Replacement same numeric key")),
            )
            .unwrap();
        paint(&mut f, &mut t);
        assert_eq!(
            f.navigate(TARGET, navigation, &mut t),
            Err(ContentRevealError::TargetReplaced)
        );
        let (v, ()) = f
            .host
            .update_scroll_reading_document(OWNER, &f.version, 2, f.items.clone(), |_| {})
            .unwrap();
        f.version = v;
        paint(&mut f, &mut t);
        assert_eq!(
            f.navigate(TARGET, navigation, &mut t).unwrap().status,
            ContentRevealStatus::Completed
        );
    }
}
#[test]
fn intrinsic_clipping_rejects_and_restores_library_scroll_metadata_without_touching_editor() {
    for placement in [ScrollRevealPlacement::Centered, start_options().placement] {
        let (mut f, mut t) = setup();
        let b = mark_composer(&mut f);
        f.host
            .get_mut(PANEL)
            .unwrap()
            .component_mut::<Panel>()
            .unwrap()
            .clipped = true;
        paint(&mut f, &mut t);
        let before = [offset(&f, OWNER), offset(&f, INNER), offset(&f, OTHER)];
        assert_eq!(
            f.navigate(TARGET, opts(placement), &mut t),
            Err(ContentRevealError::Clipped)
        );
        assert_eq!(
            [offset(&f, OWNER), offset(&f, INNER), offset(&f, OTHER)],
            before
        );
        composer_unchanged(&f, &b);
        f.host
            .get_mut(PANEL)
            .unwrap()
            .component_mut::<Panel>()
            .unwrap()
            .clipped = false;
        paint(&mut f, &mut t);
        assert_eq!(
            f.navigate(TARGET, opts(placement), &mut t).unwrap().status,
            ContentRevealStatus::Completed
        );
        composer_unchanged(&f, &b);
    }
}
#[test]
fn original_motion_reduced_motion_and_unclocked_hosts_have_determinate_completion() {
    for reduced in [false, true] {
        let (mut f, mut t) = setup();
        t.reduced = reduced;
        paint(&mut f, &mut t);
        let b = mark_composer(&mut f);
        let r = f
            .navigate(
                TARGET,
                ScrollRevealOptions {
                    animated: true,
                    ..opts(ScrollRevealPlacement::Centered)
                },
                &mut t,
            )
            .unwrap();
        assert_eq!(
            r.status,
            if reduced {
                ContentRevealStatus::Completed
            } else {
                ContentRevealStatus::Animating
            }
        );
        for ms in [20, 40, 80, 160] {
            t.now = std::time::Duration::from_millis(ms);
            paint(&mut f, &mut t);
            composer_unchanged(&f, &b);
        }
        assert_eq!(
            f.host.scroll_reading_reveal_result().unwrap().status,
            ContentRevealStatus::Completed
        );
        assert!(f.host.get(TARGET).unwrap().visible().is_some());
    }
    let (mut f, mut t) = setup();
    t.clocked = false;
    assert_eq!(
        f.navigate(TARGET, Default::default(), &mut t)
            .unwrap()
            .status,
        ContentRevealStatus::Completed
    );
}
#[test]
fn fresh_wheel_interrupts_navigation_old_momentum_does_not_and_no_timer_reasserts_it() {
    let (mut f, mut t) = setup();
    let b = mark_composer(&mut f);
    assert_eq!(
        f.navigate(
            TARGET,
            ScrollRevealOptions {
                animated: true,
                ..opts(ScrollRevealPlacement::Centered)
            },
            &mut t
        )
        .unwrap()
        .status,
        ContentRevealStatus::Animating
    );
    let p = f.host.get(OWNER).unwrap().frame().center();
    let before = offset(&f, OWNER);
    f.host.event(&ContentEvent::Scroll {
        position: p,
        delta: Point::new(0., 30.),
        precise: true,
        begin: false,
        end: false,
        momentum: true,
        time: 1.,
    });
    assert_eq!(offset(&f, OWNER), before);
    f.host.event(&ContentEvent::Scroll {
        position: p,
        delta: Point::new(0., 30.),
        precise: true,
        begin: true,
        end: true,
        momentum: false,
        time: 2.,
    });
    let user = offset(&f, OWNER);
    t.now = std::time::Duration::from_secs(1);
    paint(&mut f, &mut t);
    assert_eq!(
        f.host.scroll_reading_reveal_result().unwrap().status,
        ContentRevealStatus::Interrupted
    );
    assert_eq!(offset(&f, OWNER), user);
    let inner = offset(&f, INNER);
    t.now = std::time::Duration::from_secs(2);
    paint(&mut f, &mut t);
    assert_eq!(offset(&f, OWNER), user);
    assert_eq!(offset(&f, INNER), inner);
    composer_unchanged(&f, &b);
}
#[test]
fn stale_document_stops_old_navigation_before_the_next_layout_tick() {
    let (mut f, mut t) = setup();
    f.navigate(TARGET, Default::default(), &mut t).unwrap();
    let before = [offset(&f, OWNER), offset(&f, INNER)];
    let (next, ()) = f
        .host
        .update_scroll_reading_document(OWNER, &f.version, 2, f.items.clone(), |_| {})
        .unwrap();
    assert_eq!(
        f.host.scroll_reading_reveal_result().unwrap().status,
        ContentRevealStatus::Rejected(ContentRevealError::Reading(
            ReadingUpdateError::StaleRevision
        ))
    );
    t.now = std::time::Duration::from_secs(1);
    paint(&mut f, &mut t);
    assert_eq!([offset(&f, OWNER), offset(&f, INNER)], before);
    f.version = next;
}
#[test]
fn already_visible_navigation_enters_history_and_document_edges_report_clamping() {
    let (mut f, mut t) = setup();
    f.host
        .get_mut(OWNER)
        .unwrap()
        .component_mut::<ScrollContent>()
        .unwrap()
        .follow_latest();
    paint(&mut f, &mut t);
    let r = f
        .navigate(
            LAST,
            ScrollRevealOptions {
                animated: false,
                ..Default::default()
            },
            &mut t,
        )
        .unwrap();
    assert!(!r.changed);
    assert!(
        !f.host
            .get(OWNER)
            .unwrap()
            .component::<ScrollContent>()
            .unwrap()
            .is_following_latest()
    );
    let r = f
        .navigate(TOP, opts(ScrollRevealPlacement::Centered), &mut t)
        .unwrap();
    assert!(r.clamped);
    assert_eq!(offset(&f, OWNER).y, 0.);
}
#[test]
fn layout_changes_interrupt_both_ancestors_and_do_not_take_composer_focus() {
    let (mut f, mut t) = setup();
    let b = mark_composer(&mut f);
    f.navigate(TARGET, Default::default(), &mut t).unwrap();
    t.now = std::time::Duration::from_millis(20);
    paint(&mut f, &mut t);
    let smaller = Rect::new(0., 0., 410., 500.);
    t.now = std::time::Duration::from_millis(40);
    f.host
        .render(
            &mut Scene::new(),
            &mut t,
            smaller,
            smaller,
            Theme::LIGHT.into(),
        )
        .unwrap();
    assert_eq!(
        f.host.scroll_reading_reveal_result().unwrap().status,
        ContentRevealStatus::Interrupted
    );
    let stopped = [offset(&f, OWNER), offset(&f, INNER)];
    t.now = std::time::Duration::from_secs(1);
    f.host
        .render(
            &mut Scene::new(),
            &mut t,
            smaller,
            smaller,
            Theme::LIGHT.into(),
        )
        .unwrap();
    assert_eq!([offset(&f, OWNER), offset(&f, INNER)], stopped);
    composer_unchanged(&f, &b);
}
#[test]
fn an_older_item_command_cannot_cancel_a_new_direct_navigation_on_the_same_scroll() {
    let (mut f, mut t) = setup();
    f.navigate(TARGET, Default::default(), &mut t).unwrap();
    let outer = &mut f
        .host
        .get_mut(OWNER)
        .unwrap()
        .component_mut::<ScrollContent>()
        .unwrap()
        .scroll;
    // Even replacing the public scroll value must not reuse the old motion ID.
    let viewport = outer.viewport_size();
    let document = outer.document_size();
    *outer = tekes_ui_controls::ScrollArea::default();
    outer.layout(viewport, document);
    outer
        .reveal_with(
            Rect::new(20., document.height - 60., 20., 20.),
            ScrollRevealOptions::default(),
        )
        .unwrap();
    t.now = std::time::Duration::from_millis(20);
    paint(&mut f, &mut t);
    assert_eq!(
        f.host.scroll_reading_reveal_result().unwrap().status,
        ContentRevealStatus::Interrupted
    );
    assert!(
        f.host
            .get(OWNER)
            .unwrap()
            .component::<ScrollContent>()
            .unwrap()
            .scroll
            .needs_animation()
    );
    let first = offset(&f, OWNER).y;
    t.now = std::time::Duration::from_millis(80);
    paint(&mut f, &mut t);
    assert!(offset(&f, OWNER).y > first);
}
#[test]
fn a_reading_transaction_rejects_navigation_before_calling_the_layout_backend() {
    let (mut f, mut t) = setup();
    let version = f.version.clone();
    let (_, result) = f
        .host
        .update_scroll_reading_document(OWNER, &version, 2, f.items.clone(), |host| {
            host.reveal_scroll_reading_item(OWNER, &version, TARGET, Default::default(), &mut t)
        })
        .unwrap();
    assert_eq!(
        result,
        Err(ContentRevealError::ReadingTransactionInProgress)
    );
}
#[test]
fn invalid_options_and_intrinsic_clip_restore_inflight_motion_instead_of_taking_ownership() {
    let (mut f, mut t) = setup();
    let active = f.navigate(TARGET, Default::default(), &mut t).unwrap();
    assert_eq!(active.status, ContentRevealStatus::Animating);
    let invalid = ScrollRevealOptions {
        margins: Insets::new(f32::NAN, 0., 0., 0.),
        ..Default::default()
    };
    assert_eq!(
        f.navigate(TARGET, invalid, &mut t),
        Err(ContentRevealError::InvalidOptions(
            tekes_ui_controls::ScrollRevealError::InvalidMargins
        ))
    );
    assert_eq!(
        f.host.scroll_reading_reveal_result().unwrap().status,
        ContentRevealStatus::Animating
    );
    t.now = std::time::Duration::from_millis(40);
    paint(&mut f, &mut t);
    assert!(offset(&f, OWNER).y > 0.);
    t.now = std::time::Duration::from_secs(1);
    paint(&mut f, &mut t);
    assert_eq!(
        f.host.scroll_reading_reveal_result().unwrap().status,
        ContentRevealStatus::Completed
    );
}
#[test]
fn a_real_editor_retains_undo_history_and_selection_after_item_navigation() {
    for (navigation, reverse) in [
        (opts(ScrollRevealPlacement::Nearest), false),
        (start_options(), false),
        (start_options(), true),
    ] {
        let (mut f, mut t) = setup();
        f.host.focus(Some(COMPOSER), true);
        let old = f.host.focused_text().unwrap().input.text.clone();
        f.host.event(&ContentEvent::Commit {
            text: "Retained edit".into(),
            replacement: None,
        });
        let input = &mut f
            .host
            .get_mut(COMPOSER)
            .unwrap()
            .component_mut::<TextFieldContent>()
            .unwrap()
            .surface
            .input;
        input.select(usize::from(reverse), false);
        input.select(usize::from(!reverse), true);
        let baseline = f.host.focused_text().unwrap().input.external_baseline();
        f.navigate(TARGET, navigation, &mut t).unwrap();
        composer_unchanged(&f, &baseline);
        f.host.event(&ContentEvent::TextCommand("undo:".into()));
        assert_eq!(f.host.focused_text().unwrap().input.text, old);
    }
}
#[test]
fn unmounting_and_remounting_owner_invalidates_original_attachment_and_bad_render_invalidates_layout()
 {
    let (mut f, mut t) = setup();
    let retained = f.host.remove(OWNER).unwrap();
    f.host.insert(ROOT, retained).unwrap();
    assert_eq!(
        f.navigate(TARGET, Default::default(), &mut t),
        Err(ContentRevealError::Reading(
            ReadingUpdateError::WrongDocument
        ))
    );
    f.version = f
        .host
        .get(OWNER)
        .unwrap()
        .component::<ScrollContent>()
        .unwrap()
        .reading_document_version()
        .unwrap()
        .clone();
    paint(&mut f, &mut t);
    t.enabled = false;
    assert!(matches!(
        f.host.render(
            &mut Scene::new(),
            &mut t,
            frame(),
            frame(),
            Theme::LIGHT.into()
        ),
        Err(CompositionError::MissingTextServices(_))
    ));
    t.enabled = true;
    assert_eq!(
        f.navigate(TARGET, Default::default(), &mut t),
        Err(ContentRevealError::UnlaidOut)
    );
    paint(&mut f, &mut t);
    assert_eq!(
        f.navigate(TARGET, opts(ScrollRevealPlacement::Nearest), &mut t)
            .unwrap()
            .status,
        ContentRevealStatus::Completed
    );
}
#[derive(Debug)]
struct MovingBox {
    shift: f32,
    arranged_shift: Option<f32>,
    children: Vec<Content>,
}
impl Component for MovingBox {
    fn host_reveal_passthrough(&self) -> bool {
        self.children.len() == 1 && self.shift.is_finite() && self.arranged_shift == Some(self.shift)
    }
    fn children(&self) -> &[Content] {
        &self.children
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.children)
    }
    fn measure(
        &mut self,
        _: &mut dyn ButtonTextRenderer,
        a: Size,
    ) -> tekes_ui_controls::layout::MeasuredChild {
        tekes_ui_controls::layout::MeasuredChild::new(Size::new(a.width, 200.))
    }
    fn arrange(&mut self, t: &mut dyn ButtonTextRenderer, f: Rect, c: Rect) {
        self.arranged_shift = None;
        let height = self.children[0].measure(t, f.size).size.height;
        self.children[0].arrange(
            t,
            Rect::new(f.min_x(), f.min_y() + self.shift, f.size.width, height),
            c,
        );
        self.arranged_shift = Some(self.shift);
    }
}
#[test]
fn target_position_change_with_identical_document_extent_interrupts_old_coordinate_motion() {
    for placement in [ScrollRevealPlacement::Centered, start_options().placement] {
        let (mut f, mut t) = setup();
        let order = f
            .host
            .get(ITEMS)
            .unwrap()
            .component::<ListContent>()
            .unwrap()
            .rows
            .children
            .iter()
            .map(Content::id)
            .collect::<Vec<_>>();
        let (next, ()) = f
            .host
            .update_scroll_reading_document(OWNER, &f.version, 2, f.items.clone(), |host| {
                let original = host.remove(TARGET).unwrap();
                host.remove(PANEL).unwrap();
                host.insert(
                    ITEMS,
                    Content::new(
                        PANEL,
                        MovingBox {
                            shift: 0.,
                            arranged_shift: None,
                            children: vec![original],
                        },
                    ),
                )
                .unwrap();
                host.reorder(ITEMS, &order).unwrap();
            })
            .unwrap();
        f.version = next;
        paint(&mut f, &mut t);
        let b = mark_composer(&mut f);
        let original_extent = f
            .host
            .get(OWNER)
            .unwrap()
            .component::<ScrollContent>()
            .unwrap()
            .scroll
            .document_size();
        assert_eq!(
            f.navigate(
                TARGET,
                ScrollRevealOptions {
                    animated: true,
                    ..opts(placement)
                },
                &mut t
            )
            .unwrap()
            .status,
            ContentRevealStatus::Animating
        );
        f.host
            .get_mut(PANEL)
            .unwrap()
            .component_mut::<MovingBox>()
            .unwrap()
            .shift = 70.;
        t.now = std::time::Duration::from_millis(20);
        paint(&mut f, &mut t);
        assert_eq!(
            f.host
                .get(OWNER)
                .unwrap()
                .component::<ScrollContent>()
                .unwrap()
                .scroll
                .document_size(),
            original_extent
        );
        assert_eq!(
            f.host.scroll_reading_reveal_result().unwrap().status,
            ContentRevealStatus::Interrupted
        );
        let stopped = offset(&f, OWNER);
        t.now = std::time::Duration::from_secs(1);
        paint(&mut f, &mut t);
        assert_eq!(offset(&f, OWNER), stopped);
        composer_unchanged(&f, &b);
        let recovered = f.navigate(TARGET, opts(placement), &mut t).unwrap();
        assert_eq!(recovered.status, ContentRevealStatus::Completed);
        paint(&mut f, &mut t);
        composer_unchanged(&f, &b);
        assert!(f.host.get(TARGET).unwrap().visible().is_some());
        if matches!(placement, ScrollRevealPlacement::Start(_)) && !recovered.clamped {
            assert!(
                (f.host.get(TARGET).unwrap().frame().min_y()
                    - f.host.get(OWNER).unwrap().visible().unwrap().min_y()
                    - opts(placement).margins.top)
                    .abs()
                    < 0.02
            );
        }
    }
}
fn start_options() -> ScrollRevealOptions {
    ScrollRevealOptions {
        placement: ScrollRevealPlacement::Start(tekes_ui_controls::extended::Axis::Vertical),
        margins: Insets::ZERO,
        animated: false,
    }
}
fn paint_header_clip(f: &mut Fixture, t: &mut Text) {
    f.host
        .render(
            &mut Scene::new(),
            t,
            frame(),
            Rect::new(0., 150., 520., 410.),
            Theme::LIGHT.into(),
        )
        .unwrap();
}
#[test]
fn start_uses_actual_ancestor_clip_without_consumer_margin_compensation() {
    for margin in [0., 8.] {
        let (mut f, mut t) = setup();
        paint_header_clip(&mut f, &mut t);
        let b = mark_composer(&mut f);
        let other = offset(&f, OTHER);
        let r = f
            .navigate(
                TARGET,
                ScrollRevealOptions {
                    margins: Insets::new(margin, 0., 0., 0.),
                    ..start_options()
                },
                &mut t,
            )
            .unwrap();
        assert_eq!(r.status, ContentRevealStatus::Completed);
        assert!(!r.clamped);
        paint_header_clip(&mut f, &mut t);
        let target = f.host.get(TARGET).unwrap().frame();
        for id in [OWNER, INNER] {
            let visible = f.host.get(id).unwrap().visible().unwrap();
            assert!(
                (target.min_y() - visible.min_y() - margin).abs() < 0.02,
                "actual visible start of {id:?}: target {target:?},visible {visible:?}"
            );
        }
        composer_unchanged(&f, &b);
        assert_eq!(offset(&f, OTHER), other);
    }
}
#[test]
fn start_moves_a_visible_middle_target_and_accepts_a_repeated_no_movement_command() {
    let (mut f, mut t) = setup();
    f.navigate(TARGET, opts(ScrollRevealPlacement::Centered), &mut t)
        .unwrap();
    paint(&mut f, &mut t);
    assert!(f.host.get(TARGET).unwrap().visible().is_some());
    let before = offset(&f, OWNER);
    let r = f.navigate(TARGET, start_options(), &mut t).unwrap();
    assert!(r.changed);
    assert!(!r.clamped);
    paint(&mut f, &mut t);
    assert_ne!(offset(&f, OWNER), before);
    let target = f.host.get(TARGET).unwrap().frame();
    assert!((target.min_y() - f.host.get(OWNER).unwrap().visible().unwrap().min_y()).abs() < 0.02);
    let repeat = f.navigate(TARGET, start_options(), &mut t).unwrap();
    assert!(!repeat.changed);
    assert_eq!(repeat.status, ContentRevealStatus::Completed);
}
#[test]
fn start_aligns_original_viewport_tall_panel_even_when_it_already_spans_the_view() {
    let (mut f, mut t) = setup();
    let mut items = f.items.clone();
    *items.iter_mut().find(|id| **id == TARGET).unwrap() = PANEL;
    f.version = f
        .host
        .update_scroll_reading_document(OWNER, &f.version, 2, items, |_| {})
        .unwrap()
        .0;
    let small = Rect::new(0., 0., 520., 300.);
    f.host
        .render(&mut Scene::new(), &mut t, small, small, Theme::LIGHT.into())
        .unwrap();
    let b = mark_composer(&mut f);
    f.navigate(PANEL, opts(ScrollRevealPlacement::Centered), &mut t)
        .unwrap();
    let before = offset(&f, OWNER);
    assert!(
        f.host.get(PANEL).unwrap().frame().size.height
            > f.host.get(OWNER).unwrap().frame().size.height
    );
    let r = f.navigate(PANEL, start_options(), &mut t).unwrap();
    assert!(r.changed);
    assert!(!r.clamped);
    assert_ne!(offset(&f, OWNER), before);
    let panel = f.host.get(PANEL).unwrap().frame();
    assert!((panel.min_y() - f.host.get(OWNER).unwrap().visible().unwrap().min_y()).abs() < 0.02);
    composer_unchanged(&f, &b);
    assert_eq!(offset(&f, INNER), Point::default());
}
#[test]
fn start_reports_unavoidable_head_and_tail_clamp_instead_of_faking_alignment() {
    let (mut f, mut t) = setup();
    let b = mark_composer(&mut f);
    let r = f
        .navigate(
            TOP,
            ScrollRevealOptions {
                margins: Insets::new(40., 0., 0., 0.),
                ..start_options()
            },
            &mut t,
        )
        .unwrap();
    assert!(r.clamped);
    assert_eq!(offset(&f, OWNER).y, 0.);
    assert!(f.host.get(TOP).unwrap().visible().is_some());
    let r = f.navigate(LAST, start_options(), &mut t).unwrap();
    assert!(r.clamped);
    let owner = f.host.get(OWNER).unwrap();
    assert_eq!(
        offset(&f, OWNER).y,
        owner.component::<ScrollContent>().unwrap().scroll.limit().y
    );
    assert!(f.host.get(LAST).unwrap().visible().is_some());
    assert!(!f.navigate(LAST, start_options(), &mut t).unwrap().changed);
    composer_unchanged(&f, &b);
}
#[test]
fn start_clip_motion_reduced_motion_unclocked_and_fresh_wheel_keep_original_preedit() {
    for (reduced, clocked) in [(false, true), (true, true), (false, false)] {
        let (mut f, mut t) = setup();
        t.reduced = reduced;
        t.clocked = clocked;
        paint_header_clip(&mut f, &mut t);
        let b = mark_composer(&mut f);
        let opts = ScrollRevealOptions {
            animated: true,
            ..start_options()
        };
        let r = f.navigate(TARGET, opts, &mut t).unwrap();
        assert_eq!(
            r.status,
            if !reduced && clocked {
                ContentRevealStatus::Animating
            } else {
                ContentRevealStatus::Completed
            }
        );
        if !reduced && clocked {
            t.now = std::time::Duration::from_millis(20);
            paint_header_clip(&mut f, &mut t);
            composer_unchanged(&f, &b);
            let p = f.host.get(OWNER).unwrap().visible().unwrap().center();
            f.host.event(&ContentEvent::Scroll {
                position: p,
                delta: Point::new(0., 4.),
                begin: false,
                end: false,
                time: 1.,
                momentum: false,
                precise: true,
            });
            let user = offset(&f, OWNER);
            t.now = std::time::Duration::from_secs(1);
            paint_header_clip(&mut f, &mut t);
            assert_eq!(offset(&f, OWNER), user);
            assert_eq!(
                f.host.scroll_reading_reveal_result().unwrap().status,
                ContentRevealStatus::Interrupted
            );
            f.navigate(TARGET, opts, &mut t).unwrap();
            t.now = std::time::Duration::from_secs(2);
        }
        paint_header_clip(&mut f, &mut t);
        assert_eq!(
            f.host.scroll_reading_reveal_result().unwrap().status,
            ContentRevealStatus::Completed
        );
        composer_unchanged(&f, &b);
        assert!(
            (f.host.get(TARGET).unwrap().frame().min_y()
                - f.host.get(OWNER).unwrap().visible().unwrap().min_y())
            .abs()
                < 0.02
        );
    }
}
#[test]
fn start_clip_change_interrupts_old_motion_and_explicit_retry_uses_new_visible_start() {
    let (mut f, mut t) = setup();
    paint_header_clip(&mut f, &mut t);
    let b = mark_composer(&mut f);
    f.navigate(
        TARGET,
        ScrollRevealOptions {
            animated: true,
            ..start_options()
        },
        &mut t,
    )
    .unwrap();
    t.now = std::time::Duration::from_millis(20);
    paint_header_clip(&mut f, &mut t);
    let clip = Rect::new(0., 180., 520., 380.);
    t.now = std::time::Duration::from_millis(40);
    f.host
        .render(
            &mut Scene::new(),
            &mut t,
            frame(),
            clip,
            Theme::LIGHT.into(),
        )
        .unwrap();
    assert_eq!(
        f.host.scroll_reading_reveal_result().unwrap().status,
        ContentRevealStatus::Interrupted
    );
    let r = f.navigate(TARGET, start_options(), &mut t).unwrap();
    assert_eq!(r.status, ContentRevealStatus::Completed);
    assert!(!r.clamped);
    composer_unchanged(&f, &b);
    assert!(
        (f.host.get(TARGET).unwrap().frame().min_y()
            - f.host.get(OWNER).unwrap().visible().unwrap().min_y())
        .abs()
            < 0.02
    );
}
#[test]
fn start_refines_an_initially_offscreen_nested_clip_when_parent_is_clamped_at_document_head() {
    for top in [150., 200.] {
        let (mut f, mut t) = setup();
        let items = f
            .items
            .iter()
            .copied()
            .filter(|id| !(11100..11112).contains(&id.0))
            .collect::<Vec<_>>();
        f.version = f
            .host
            .update_scroll_reading_document(OWNER, &f.version, 2, items.clone(), |h| {
                for id in 11100..11112 {
                    h.remove(ComponentId(id)).unwrap();
                }
            })
            .unwrap()
            .0;
        f.items = items;
        let clip = Rect::new(0., top, 520., 560. - top);
        f.host
            .render(
                &mut Scene::new(),
                &mut t,
                frame(),
                clip,
                Theme::LIGHT.into(),
            )
            .unwrap();
        let p = f.host.get(OWNER).unwrap().visible().unwrap().center();
        f.host.event(&ContentEvent::Scroll {
            position: p,
            delta: Point::new(0., 10000.),
            precise: true,
            begin: true,
            end: true,
            momentum: false,
            time: 1.,
        });
        f.host
            .render(
                &mut Scene::new(),
                &mut t,
                frame(),
                clip,
                Theme::LIGHT.into(),
            )
            .unwrap();
        let b = mark_composer(&mut f);
        assert!(f.host.get(TARGET).unwrap().visible().is_none());
        let opts = ScrollRevealOptions {
            margins: Insets::new(8., 0., 0., 0.),
            ..start_options()
        };
        let r = f.navigate(TARGET, opts, &mut t).unwrap();
        assert_eq!(r.status, ContentRevealStatus::Completed);
        assert!(!r.clamped);
        assert_eq!(offset(&f, OWNER).y, 0.);
        let target = f.host.get(TARGET).unwrap().frame();
        assert!((target.min_y() - top - 8.).abs() < 0.02);
        composer_unchanged(&f, &b);
    }
}
#[test]
fn start_can_reveal_a_currently_tiny_nested_visible_slice_before_applying_its_margin() {
    let (mut f, mut t) = setup();
    f.navigate(TARGET, opts(ScrollRevealPlacement::Centered), &mut t)
        .unwrap();
    paint(&mut f, &mut t);
    let owner = f.host.get(OWNER).unwrap().visible().unwrap();
    let inner = f.host.get(INNER).unwrap().frame();
    let delta = inner.min_y() - (owner.max_y() - 2.);
    f.host.event(&ContentEvent::Scroll {
        position: Point::new(owner.min_x() + 0.5, owner.center().y),
        delta: Point::new(0., delta),
        precise: true,
        begin: true,
        end: true,
        momentum: false,
        time: 1.,
    });
    paint(&mut f, &mut t);
    let actual = f.host.get(INNER).unwrap().visible().unwrap();
    assert!(actual.size.height <= 2.01);
    let b = mark_composer(&mut f);
    let r = f
        .navigate(
            TARGET,
            ScrollRevealOptions {
                margins: Insets::new(8., 0., 0., 0.),
                ..start_options()
            },
            &mut t,
        )
        .unwrap();
    assert_eq!(r.status, ContentRevealStatus::Completed);
    assert!(!r.clamped);
    let target = f.host.get(TARGET).unwrap().frame();
    for id in [OWNER, INNER] {
        assert!(
            (target.min_y() - f.host.get(id).unwrap().visible().unwrap().min_y() - 8.).abs() < 0.02
        );
    }
    composer_unchanged(&f, &b);
}
