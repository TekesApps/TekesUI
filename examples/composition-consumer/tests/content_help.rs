use tekes_ui_controls::{ButtonTextAlign, ButtonTextRenderer, composition::*};
use tekes_ui_scene::{Rect, Scene, Size};
use tekes_ui_tokens::{Rgba, Theme, proposal};
struct Text;
impl ButtonTextRenderer for Text {
    fn text_surface_host(&mut self) -> Option<&mut dyn tekes_ui_controls::TextSurfaceHost> {
        Some(self)
    }
    fn measure(&mut self, s: &str, r: proposal::TextRole, w: f32) -> Size {
        Size::new((s.len() as f32 * 8.).min(w), r.style().line_height)
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
impl tekes_ui_controls::TextSurfaceHost for Text {
    fn shape_surface(
        &mut self,
        text: &str,
        role: proposal::TextRole,
        width: Option<f32>,
    ) -> std::rc::Rc<tekes_ui_text::model::Paragraph> {
        use tekes_ui_text::traits::Shaper;
        let mut shaper = tekes_ui_text::coretext::CoreTextShaper::new(
            tekes_ui_text::coretext::CoreTextFonts::new(),
        );
        std::rc::Rc::new(shaper.shape(
            text,
            &tekes_ui_text::style::TextStyle::system(role.style().size),
            &tekes_ui_text::model::ShapeOptions {
                width: width.map(f64::from),
                line_spacing: 0.,
            },
        ))
    }
}
impl tekes_ui_controls::TextSurfaceTextRenderer for Text {
    fn push_visible(
        &mut self,
        _: &mut Scene,
        _: &tekes_ui_text::model::Paragraph,
        _: tekes_ui_scene::Point,
        _: Rgba,
        _: Rect,
    ) {
    }
}

use tekes_ui_composition_consumer::help_list::*;
#[test]
fn declaration_updates_original_semantic_owner_without_an_extra_node() {
    let mut h = build().unwrap();
    let f = Rect::new(0., 0., 400., 260.);
    h.render(&mut Scene::new(), &mut Text, f, f, Theme::LIGHT.into())
        .unwrap();
    let nodes = h.semantics();
    let row = nodes.iter().find(|n| n.id == FIRST).unwrap();
    assert_eq!(
        row.semantics.description.as_deref(),
        Some("query A · 第1项搜索摘要 /folder/1")
    );
    h.get_mut(FIRST)
        .unwrap()
        .set_help(Some("query B · 新摘要".into()));
    let updated = h.semantics();
    assert_eq!(updated.len(), nodes.len());
    assert_eq!(
        updated
            .iter()
            .find(|n| n.id == FIRST)
            .unwrap()
            .semantics
            .description
            .as_deref(),
        Some("query B · 新摘要")
    );
    h.get_mut(FIRST).unwrap().set_help(None);
    assert!(
        h.semantics()
            .iter()
            .find(|n| n.id == FIRST)
            .unwrap()
            .semantics
            .description
            .is_none()
    );
}

use std::time::Duration;
use tekes_ui_controls::{ControlKey, ControlMotionContext};
use tekes_ui_scene::Point;
struct Clock {
    text: Text,
    now: u64,
    reduced: bool,
    requested: bool,
    drawn: Vec<String>,
}
impl Default for Clock {
    fn default() -> Self {
        Self {
            text: Text,
            now: 0,
            reduced: false,
            requested: false,
            drawn: Vec::new(),
        }
    }
}
impl ButtonTextRenderer for Clock {
    fn measure(&mut self, s: &str, r: proposal::TextRole, w: f32) -> Size {
        self.text.measure(s, r, w)
    }
    fn draw(
        &mut self,
        s: &mut Scene,
        t: &str,
        r: proposal::TextRole,
        f: Rect,
        a: ButtonTextAlign,
        i: Rgba,
    ) {
        self.drawn.push(t.to_owned());
        self.text.draw(s, t, r, f, a, i)
    }
    fn text_surface_host(&mut self) -> Option<&mut dyn tekes_ui_controls::TextSurfaceHost> {
        Some(&mut self.text)
    }
    fn motion_context(&self) -> Option<ControlMotionContext> {
        Some(ControlMotionContext {
            now: Duration::from_millis(self.now),
            reduced_motion: self.reduced,
        })
    }
    fn request_animation_frame(&mut self) {
        self.requested = true;
    }
}
fn draw(h: &mut ContentHost, t: &mut Clock) {
    let f = Rect::new(0., 0., 400., 260.);
    h.render(&mut Scene::new(), t, f, f, Theme::LIGHT.into())
        .unwrap();
}
fn setup() -> (ContentHost, Clock) {
    let mut h = build().unwrap();
    let mut t = Clock::default();
    draw(&mut h, &mut t);
    (h, t)
}
fn row(h: &ContentHost) -> Point {
    h.get(FIRST).unwrap().visible().unwrap().center()
}
fn hover(h: &mut ContentHost, t: &mut Clock) {
    let p = row(h);
    h.event(&ContentEvent::PointerMove(p));
    draw(h, t);
    t.now += 500;
    draw(h, t);
    assert_eq!(h.help_tooltip().unwrap().0, FIRST);
}
fn click(h: &mut ContentHost, p: Point) {
    h.event(&ContentEvent::PointerDown(p));
    h.event(&ContentEvent::PointerUp(p));
}
#[test]
fn original_clock_dwell_does_not_change_focus_or_semantic_identity() {
    for reduced in [false, true] {
        let (mut h, mut t) = setup();
        t.reduced = reduced;
        let nodes = h.semantics();
        let p = row(&h);
        h.event(&ContentEvent::PointerMove(p));
        draw(&mut h, &mut t);
        assert!(h.help_tooltip().is_none());
        assert!(t.requested);
        assert!(h.needs_render());
        t.now = 499;
        draw(&mut h, &mut t);
        assert!(h.help_tooltip().is_none());
        t.now = 500;
        t.requested = false;
        draw(&mut h, &mut t);
        assert!(h.help_tooltip().is_some());
        assert!(!t.requested);
        assert!(!h.needs_render());
        assert_eq!(h.focused(), None);
        assert_eq!(h.semantics(), nodes);
        assert!(h.drain_actions().is_empty());
    }
}
#[test]
fn query_update_clear_and_equal_text_never_replay_old_help() {
    let (mut h, mut t) = setup();
    let p = row(&h);
    h.event(&ContentEvent::PointerMove(p));
    draw(&mut h, &mut t);
    t.now = 250;
    let same = h.get(FIRST).unwrap().help_text().unwrap().to_owned();
    h.get_mut(FIRST).unwrap().set_help(Some(same));
    draw(&mut h, &mut t);
    t.now = 500;
    draw(&mut h, &mut t);
    assert!(h.help_tooltip().is_some());
    h.get_mut(FIRST)
        .unwrap()
        .set_help(Some("query B · 唯一新说明".into()));
    assert!(h.help_tooltip().is_none());
    draw(&mut h, &mut t);
    t.now = 1000;
    draw(&mut h, &mut t);
    assert_eq!(h.help_tooltip().unwrap().1, "query B · 唯一新说明");
    h.get_mut(FIRST).unwrap().set_help(None);
    assert!(h.help_tooltip().is_none());
    t.now = 9000;
    draw(&mut h, &mut t);
    assert!(h.help_tooltip().is_none());
    assert!(h.get(FIRST).unwrap().help_text().is_none());
}
#[test]
fn hover_and_help_updates_keep_composer_preedit_selection_key_epoch_and_scroll() {
    let (mut h, mut t) = setup();
    h.focus(Some(COMPOSER), false);
    h.event(&ContentEvent::Commit {
        text: "draft".into(),
        replacement: None,
    });
    h.event(&ContentEvent::TextKeyStart {
        id: 9,
        composing: true,
        key: ContentKeyEvent {
            key: None,
            down: true,
            repeat: false,
            modifiers: Default::default(),
            native_code: 45,
            characters: "n".into(),
            characters_ignoring_modifiers: "n".into(),
        },
    });
    h.event(&ContentEvent::Preedit {
        text: "ni".into(),
        selection: 0..2,
        clauses: vec![],
        replacement: None,
    });
    let before = h.focused_text().unwrap().input.external_baseline();
    let focus = h.capture_focus();
    let offset = h
        .get(SCROLL)
        .unwrap()
        .component::<ScrollContent>()
        .unwrap()
        .scroll
        .offset;
    hover(&mut h, &mut t);
    h.get_mut(FIRST)
        .unwrap()
        .set_help(Some("updated snippet".into()));
    draw(&mut h, &mut t);
    t.now += 500;
    draw(&mut h, &mut t);
    assert_eq!(h.capture_focus(), focus);
    assert_eq!(h.focused_text().unwrap().input.external_baseline(), before);
    assert_eq!(
        h.get(SCROLL)
            .unwrap()
            .component::<ScrollContent>()
            .unwrap()
            .scroll
            .offset,
        offset
    );
    let key = h
        .event(&ContentEvent::TextKeyEnd {
            id: 9,
            input_context_consumed: false,
        })
        .text_key
        .unwrap();
    assert_eq!(key.target, Some(COMPOSER));
    assert_ne!(key.disposition, TextKeyDisposition::Cancelled);
    assert!(h.drain_actions().is_empty());
}
#[test]
fn original_pointer_keyboard_and_ax_actions_remain_once_and_close_help() {
    let (mut h, mut t) = setup();
    hover(&mut h, &mut t);
    let p = row(&h);
    click(&mut h, p);
    assert!(h.help_tooltip().is_none());
    t.now += 4000;
    draw(&mut h, &mut t);
    assert!(h.help_tooltip().is_none());
    assert_eq!(h.drain_actions().len(), 1);
    h.event(&ContentEvent::Key {
        key: ControlKey::Enter,
        down: true,
    });
    h.event(&ContentEvent::Key {
        key: ControlKey::Enter,
        down: false,
    });
    assert_eq!(h.drain_actions().len(), 1);
    assert!(
        h.accessibility_action(FIRST, None, AccessibilityAction::Press)
            .activated
    );
    assert_eq!(h.drain_actions().len(), 1);
    assert_eq!(
        h.get(FIRST)
            .unwrap()
            .component::<ButtonContent>()
            .unwrap()
            .activations,
        3
    );
    t.now += 4000;
    draw(&mut h, &mut t);
    t.now += 500;
    draw(&mut h, &mut t);
    assert!(
        h.help_tooltip().is_none(),
        "AX Press must not arm a future keyboard tooltip"
    );
    h.event(&ContentEvent::PointerMove(Point::new(-1., -1.)));
    hover(&mut h, &mut t);
    assert!(h.help_tooltip().is_some());
}
#[test]
fn keyboard_help_escape_and_duplicate_focus_do_not_activate_or_rearm() {
    let (mut h, mut t) = setup();
    h.focus(Some(FIRST), true);
    draw(&mut h, &mut t);
    t.now = 500;
    draw(&mut h, &mut t);
    assert!(h.help_tooltip().is_some());
    assert!(
        h.event(&ContentEvent::Key {
            key: ControlKey::Escape,
            down: true
        })
        .consumed
    );
    assert_eq!(h.focused(), Some(FIRST));
    assert!(h.help_tooltip().is_none());
    h.focus(Some(FIRST), true);
    t.now = 5000;
    draw(&mut h, &mut t);
    assert!(h.help_tooltip().is_none());
    assert!(h.drain_actions().is_empty());
    h.focus(Some(COMPOSER), true);
    h.focus(Some(FIRST), true);
    draw(&mut h, &mut t);
    t.now += 500;
    draw(&mut h, &mut t);
    assert!(h.help_tooltip().is_some());
}
#[test]
fn scroll_clip_and_cancel_hide_without_restoring_old_request() {
    for reason in 0..3 {
        let (mut h, mut t) = setup();
        hover(&mut h, &mut t);
        match reason {
            0 => {
                let p = row(&h);
                h.event(&ContentEvent::Scroll {
                    position: p,
                    delta: Point::new(0., 30.),
                    precise: true,
                    begin: false,
                    end: false,
                    momentum: false,
                    time: 0.,
                });
            }
            1 => {
                let f = Rect::new(0., 0., 400., 260.);
                h.render(
                    &mut Scene::new(),
                    &mut t,
                    f,
                    Rect::new(0., 90., 400., 170.),
                    Theme::LIGHT.into(),
                )
                .unwrap();
            }
            _ => {
                h.event(&ContentEvent::Cancel);
            }
        }
        assert!(h.help_tooltip().is_none());
        t.now += 2000;
        draw(&mut h, &mut t);
        assert!(h.help_tooltip().is_none(), "reason {reason}");
        assert!(h.drain_actions().is_empty());
    }
}
#[test]
fn removal_remount_revokes_original_declaration_until_explicit_new_binding() {
    let (mut h, mut t) = setup();
    hover(&mut h, &mut t);
    let old = h.remove(FIRST).unwrap();
    assert!(h.help_tooltip().is_none());
    assert!(old.help_text().is_none());
    h.insert(LIST, old).unwrap();
    draw(&mut h, &mut t);
    t.now += 1000;
    draw(&mut h, &mut t);
    assert!(h.help_tooltip().is_none());
    h.get_mut(FIRST).unwrap().set_help(Some("rebound".into()));
    h.reorder(
        LIST,
        &(0..8).map(|i| ComponentId(FIRST.0 + i)).collect::<Vec<_>>(),
    )
    .unwrap();
    draw(&mut h, &mut t);
    hover(&mut h, &mut t);
    assert_eq!(h.help_tooltip().unwrap().1, "rebound");
}
#[test]
fn real_overlay_and_editor_capture_take_priority_over_help() {
    let (mut h, mut t) = setup();
    hover(&mut h, &mut t);
    let menu = ComponentId(16100);
    h.insert(
        ROOT,
        Content::new(menu, MenuContent::new("Menu", vec!["First".into()])),
    )
    .unwrap();
    h.get_mut(menu)
        .unwrap()
        .component_mut::<MenuContent>()
        .unwrap()
        .menu
        .show(0);
    draw(&mut h, &mut t);
    assert!(h.help_tooltip().is_none());
    h.remove(menu).unwrap();
    draw(&mut h, &mut t);
    assert!(h.help_tooltip().is_none());
    let p = h.text_surface(COMPOSER).unwrap().outer.center();
    assert!(h.event(&ContentEvent::PointerDown(p)).capture_pointer);
    let row = row(&h);
    h.event(&ContentEvent::PointerMove(row));
    draw(&mut h, &mut t);
    t.now += 500;
    draw(&mut h, &mut t);
    assert!(h.help_tooltip().is_none());
    h.event(&ContentEvent::PointerUp(row));
    h.event(&ContentEvent::PointerMove(Point::new(-1., -1.)));
    hover(&mut h, &mut t);
}
#[test]
fn long_mixed_help_is_bounded_at_viewport_edges_in_all_themes() {
    for theme in Theme::ALL {
        for width in [96., 400.] {
            let (mut h, mut t) = setup();
            h.get_mut(FIRST).unwrap().set_help(Some(
                "长中文混合 A😀B é / very-long-folder/path/".repeat(40),
            ));
            let f = Rect::new(0., 0., width, 200.);
            h.render(&mut Scene::new(), &mut t, f, f, theme.into())
                .unwrap();
            let p = row(&h);
            h.event(&ContentEvent::PointerMove(p));
            h.render(&mut Scene::new(), &mut t, f, f, theme.into())
                .unwrap();
            t.now = 500;
            h.render(&mut Scene::new(), &mut t, f, f, theme.into())
                .unwrap();
            let (_, text, layout) = h.help_tooltip().unwrap();
            assert!(text.starts_with("长中文"));
            assert_eq!(layout.frame.intersection(&f), Some(layout.frame));
            assert!(layout.content.size.width <= width);
            assert_eq!(h.focused(), None);
        }
    }
}

#[test]
fn help_preserves_a_nonzero_original_reading_position() {
    let (mut h, mut t) = setup();
    h.get_mut(SCROLL)
        .unwrap()
        .component_mut::<ScrollContent>()
        .unwrap()
        .scroll
        .offset
        .y = 80.;
    draw(&mut h, &mut t);
    let id = ComponentId(FIRST.0 + 4);
    let p = h.get(id).unwrap().visible().unwrap().center();
    let before = h
        .get(SCROLL)
        .unwrap()
        .component::<ScrollContent>()
        .unwrap()
        .scroll
        .offset;
    h.event(&ContentEvent::PointerMove(p));
    draw(&mut h, &mut t);
    t.now = 500;
    draw(&mut h, &mut t);
    assert_eq!(h.help_tooltip().unwrap().0, id);
    assert_eq!(
        h.get(SCROLL)
            .unwrap()
            .component::<ScrollContent>()
            .unwrap()
            .scroll
            .offset,
        before
    );
    assert!(before.y > 0.);
}
#[test]
fn modal_owns_help_without_exposing_background_or_stealing_focus() {
    #[derive(Debug)]
    struct Layers(Vec<Content>);
    impl Component for Layers {
        fn children(&self) -> &[Content] {
            &self.0
        }
        fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
            Some(&mut self.0)
        }
        fn measure(
            &mut self,
            t: &mut dyn ButtonTextRenderer,
            a: Size,
        ) -> tekes_ui_controls::layout::MeasuredChild {
            for c in &mut self.0 {
                c.measure(t, a);
            }
            tekes_ui_controls::layout::MeasuredChild::new(a)
        }
        fn arrange(&mut self, t: &mut dyn ButtonTextRenderer, f: Rect, c: Rect) {
            for child in &mut self.0 {
                child.arrange(t, f, c);
            }
        }
        fn paint(
            &mut self,
            _: &mut Scene,
            _: &mut dyn ButtonTextRenderer,
            _: Rect,
            _: Rect,
            _: ContentTheme,
        ) {
        }
    }
    let modal = ComponentId(16200);
    let inside = ComponentId(16202);
    let mut h = ContentHost::new(Content::new(
        ROOT,
        Layers(vec![
            Content::new(FIRST, ButtonContent::new("Background")).with_help("background help"),
            Content::new(
                modal,
                DialogContent::new(
                    "Modal",
                    Content::new(ComponentId(16201), ButtonContent::new("Header")),
                    Content::new(inside, ButtonContent::new("Inside")).with_help("modal help"),
                    Content::new(ComponentId(16203), ButtonContent::new("Done")),
                ),
            ),
        ]),
    ))
    .unwrap();
    h.set_action_events(true);
    let mut t = Clock::default();
    draw(&mut h, &mut t);
    hover(&mut h, &mut t);
    h.show_dialog(modal).unwrap();
    draw(&mut h, &mut t);
    assert!(h.help_tooltip().is_none());
    h.focus(Some(inside), true);
    draw(&mut h, &mut t);
    t.now += 500;
    draw(&mut h, &mut t);
    assert_eq!(h.help_tooltip().unwrap().0, inside);
    assert_eq!(h.focused(), Some(inside));
    assert!(
        !h.accessibility_action(FIRST, None, AccessibilityAction::Press)
            .consumed
    );
    assert!(h.drain_actions().is_empty());
    h.close_dialog();
    draw(&mut h, &mut t);
    assert!(h.help_tooltip().is_none());
}
#[test]
fn missing_clock_and_backwards_clock_do_not_replay_a_visible_tooltip() {
    let (mut h, mut t) = setup();
    let p = row(&h);
    h.event(&ContentEvent::PointerMove(p));
    let f = Rect::new(0., 0., 400., 260.);
    h.render(&mut Scene::new(), &mut Text, f, f, Theme::LIGHT.into())
        .unwrap();
    assert!(h.help_tooltip().is_none());
    assert!(!h.needs_render());
    h.event(&ContentEvent::PointerMove(p));
    draw(&mut h, &mut t);
    t.now = 500;
    draw(&mut h, &mut t);
    assert!(h.help_tooltip().is_some());
    t.now = 100;
    draw(&mut h, &mut t);
    assert!(h.help_tooltip().is_none());
    t.now = 600;
    draw(&mut h, &mut t);
    assert!(h.help_tooltip().is_some());
}

#[test]
fn editor_only_text_service_falls_back_to_original_renderer_instead_of_blank_help() {
    let (mut h, mut t) = setup();
    hover(&mut h, &mut t);
    assert!(
        t.drawn
            .iter()
            .any(|s| s == "query A · 第1项搜索摘要 /folder/1"),
        "tooltip source must reach caller draw when shape_label is absent"
    );
}

#[test]
fn losing_host_clock_cannot_publish_an_old_visible_help_layout() {
    let (mut h, mut t) = setup();
    hover(&mut h, &mut t);
    let f = Rect::new(0., 0., 400., 260.);
    h.render(&mut Scene::new(), &mut Text, f, f, Theme::LIGHT.into())
        .unwrap();
    assert!(
        h.help_tooltip().is_none(),
        "no-clock frame did not paint help and cannot report old visibility"
    );
    t.now += 5000;
    draw(&mut h, &mut t);
    assert!(h.help_tooltip().is_none());
    hover(&mut h, &mut t);
}

#[test]
fn help_reuses_the_original_semantic_frame_not_unused_allocated_padding() {
    #[derive(Debug)]
    struct OriginalOwner;
    impl Component for OriginalOwner {
        fn interactive(&self) -> bool {
            true
        }
        fn semantic_frame(&self, _: Rect) -> Rect {
            Rect::new(0., 60., 100., 20.)
        }
        fn measure(
            &mut self,
            _: &mut dyn ButtonTextRenderer,
            a: Size,
        ) -> tekes_ui_controls::layout::MeasuredChild {
            tekes_ui_controls::layout::MeasuredChild::new(a)
        }
        fn paint(
            &mut self,
            _: &mut Scene,
            _: &mut dyn ButtonTextRenderer,
            _: Rect,
            _: Rect,
            _: ContentTheme,
        ) {
        }
    }
    let mut h =
        ContentHost::new(Content::new(FIRST, OriginalOwner).with_help("original body help"))
            .unwrap();
    let mut t = Clock::default();
    draw(&mut h, &mut t);
    h.event(&ContentEvent::PointerMove(Point::new(10., 10.)));
    draw(&mut h, &mut t);
    t.now = 500;
    draw(&mut h, &mut t);
    assert!(
        h.help_tooltip().is_none(),
        "allocated padding outside the original semantic owner must not trigger help"
    );
    h.event(&ContentEvent::PointerMove(Point::new(10., 70.)));
    draw(&mut h, &mut t);
    t.now = 1000;
    draw(&mut h, &mut t);
    assert_eq!(
        h.help_tooltip().unwrap().2.anchor,
        Rect::new(0., 60., 100., 20.)
    );
    let f = Rect::new(0., 0., 400., 260.);
    h.render(
        &mut Scene::new(),
        &mut t,
        f,
        Rect::new(0., 0., 400., 40.),
        Theme::LIGHT.into(),
    )
    .unwrap();
    assert!(h.help_tooltip().is_none());
}

#[test]
fn moving_into_the_original_tooltip_keeps_its_owner_until_leaving_both_regions() {
    let (mut h, mut t) = setup();
    hover(&mut h, &mut t);
    let tip = h.help_tooltip().unwrap().2;
    h.event(&ContentEvent::PointerMove(tip.visible.unwrap().center()));
    draw(&mut h, &mut t);
    assert_eq!(
        h.help_tooltip().unwrap().0,
        FIRST,
        "tooltip region retains the original owner instead of targeting a covered row"
    );
    h.event(&ContentEvent::PointerMove(Point::new(399., 259.)));
    draw(&mut h, &mut t);
    assert!(h.help_tooltip().is_none());
    assert!(h.drain_actions().is_empty());
}

#[test]
fn changed_help_does_not_cancel_an_existing_escape_or_click_dismissal() {
    for click_dismissal in [false, true] {
        let (mut h, mut t) = setup();
        hover(&mut h, &mut t);
        if click_dismissal {
            let p = row(&h);
            click(&mut h, p);
        } else {
            h.event(&ContentEvent::Key {
                key: ControlKey::Escape,
                down: true,
            });
        }
        assert!(h.help_tooltip().is_none());
        h.get_mut(FIRST)
            .unwrap()
            .set_help(Some("new query after dismissal".into()));
        draw(&mut h, &mut t);
        t.now += 2000;
        draw(&mut h, &mut t);
        assert!(
            h.help_tooltip().is_none(),
            "text is metadata, not a new owner input request"
        );
        h.event(&ContentEvent::PointerMove(Point::new(-1., -1.)));
        hover(&mut h, &mut t);
        assert_eq!(h.help_tooltip().unwrap().1, "new query after dismissal");
    }
}

const HELP_BAR_ROOT: ComponentId = ComponentId(26000);
const HELP_BAR: ComponentId = ComponentId(26001);
fn toolbar_help_fixture(deferred: bool) -> (ContentHost, Clock) {
    let mut bar = tekes_ui_controls::ToolbarControl::window();
    bar.set_actions(vec![
        tekes_ui_controls::ToolbarAction {
            id: 1,
            title: "One".into(),
            enabled: true,
        },
        tekes_ui_controls::ToolbarAction {
            id: 2,
            title: "Two".into(),
            enabled: false,
        },
    ]);
    assert!(bar.set_action_help(1, Some("Help for One".into())));
    assert!(bar.set_action_help(2, Some("Help for unavailable Two".into())));
    let mut host = ContentHost::new(Content::new(
        HELP_BAR_ROOT,
        ListContent::new("Commands", vec![Content::new(HELP_BAR, bar)]),
    ))
    .unwrap();
    host.set_action_events(true);
    host.set_help_overlay_deferred(deferred);
    let mut clock = Clock::default();
    draw_toolbar_help(&mut host, &mut clock, false);
    (host, clock)
}
fn draw_toolbar_help(h: &mut ContentHost, t: &mut Clock, overlay: bool) {
    let row = Rect::new(0., 0., 400., 42.);
    let viewport = Rect::new(0., 0., 400., 260.);
    h.render(&mut Scene::new(), t, row, viewport, Theme::LIGHT.into())
        .unwrap();
    if overlay {
        h.paint_help_overlay(&mut Scene::new(), t, viewport, Theme::LIGHT.into());
    }
}
fn help_part(h: &ContentHost, name: &str) -> SemanticPart {
    h.semantics()
        .into_iter()
        .find(|n| n.id == HELP_BAR)
        .unwrap()
        .parts
        .into_iter()
        .find(|p| p.semantics.name == name)
        .unwrap()
}
#[test]
fn toolbar_action_hints_reuse_dwell_and_disabled_semantics_without_new_owner() {
    let (mut h, mut t) = toolbar_help_fixture(false);
    let before = h.semantics();
    let first = help_part(&h, "One");
    let second = help_part(&h, "Two");
    assert_eq!(first.semantics.description.as_deref(), Some("Help for One"));
    assert!(second.semantics.disabled);
    h.event(&ContentEvent::PointerMove(first.frame.center()));
    draw_toolbar_help(&mut h, &mut t, false);
    assert!(h.help_tooltip().is_none());
    t.now = 500;
    draw_toolbar_help(&mut h, &mut t, false);
    assert_eq!(h.help_tooltip().unwrap().1, "Help for One");
    assert!(h.help_tooltip().unwrap().2.visible.unwrap().max_y() > 42.);
    h.event(&ContentEvent::PointerMove(second.frame.center()));
    draw_toolbar_help(&mut h, &mut t, false);
    assert!(
        h.help_tooltip().is_none(),
        "moving to another part must restart dwell"
    );
    t.now = 1000;
    draw_toolbar_help(&mut h, &mut t, false);
    assert_eq!(h.help_tooltip().unwrap().1, "Help for unavailable Two");
    assert_eq!(h.focused(), None);
    assert_eq!(h.semantics(), before);
    click(&mut h, second.frame.center());
    assert!(h.drain_actions().is_empty());
    click(&mut h, first.frame.center());
    assert_eq!(h.drain_actions().len(), 1);
    assert!(h.help_tooltip().is_none());
}
#[test]
fn toolbar_help_revisions_clear_and_unmount_cannot_replay_old_parts() {
    let (mut h, mut t) = toolbar_help_fixture(false);
    let first = help_part(&h, "One").frame.center();
    h.event(&ContentEvent::PointerMove(first));
    draw_toolbar_help(&mut h, &mut t, false);
    t.now = 500;
    draw_toolbar_help(&mut h, &mut t, false);
    assert!(h.help_tooltip().is_some());
    let bar = h
        .get_mut(HELP_BAR)
        .unwrap()
        .component_mut::<tekes_ui_controls::ToolbarControl>()
        .unwrap();
    assert!(!bar.set_action_help(1, Some("Help for One".into())));
    assert!(!bar.set_action_help(999, Some("unknown".into())));
    assert!(h.help_tooltip().is_some());
    assert!(
        h.get_mut(HELP_BAR)
            .unwrap()
            .component_mut::<tekes_ui_controls::ToolbarControl>()
            .unwrap()
            .set_action_help(2, Some("Updated other action".into()))
    );
    assert!(
        h.help_tooltip().is_some(),
        "another action's metadata must not dismiss this hint"
    );
    assert!(
        h.get_mut(HELP_BAR)
            .unwrap()
            .component_mut::<tekes_ui_controls::ToolbarControl>()
            .unwrap()
            .set_action_help(1, Some("Changed help".into()))
    );
    assert!(h.help_tooltip().is_none());
    draw_toolbar_help(&mut h, &mut t, false);
    t.now = 1000;
    draw_toolbar_help(&mut h, &mut t, false);
    assert_eq!(h.help_tooltip().unwrap().1, "Changed help");
    let retained = h.remove(HELP_BAR).unwrap();
    assert!(h.help_tooltip().is_none());
    h.insert(HELP_BAR_ROOT, retained).unwrap();
    draw_toolbar_help(&mut h, &mut t, false);
    h.event(&ContentEvent::PointerMove(
        help_part(&h, "One").frame.center(),
    ));
    draw_toolbar_help(&mut h, &mut t, false);
    t.now = 2000;
    draw_toolbar_help(&mut h, &mut t, false);
    assert!(
        h.help_tooltip().is_none(),
        "retained owner must explicitly rebind its help"
    );
}
#[test]
fn deferred_toolbar_help_paints_once_after_content_without_replaying_buttons() {
    let (mut h, mut t) = toolbar_help_fixture(true);
    let first = help_part(&h, "One").frame.center();
    h.event(&ContentEvent::PointerMove(first));
    draw_toolbar_help(&mut h, &mut t, false);
    assert!(!t.requested);
    let viewport = Rect::new(0., 0., 400., 260.);
    h.paint_help_overlay(&mut Scene::new(), &mut t, viewport, Theme::LIGHT.into());
    assert!(t.requested);
    assert!(h.needs_render());
    t.now = 500;
    draw_toolbar_help(&mut h, &mut t, false);
    t.drawn.clear();
    t.requested = false;
    h.paint_help_overlay(&mut Scene::new(), &mut t, viewport, Theme::LIGHT.into());
    assert_eq!(h.help_tooltip().unwrap().1, "Help for One");
    assert!(t.drawn.iter().all(|s| s != "One" && s != "Two"));
    assert!(!t.requested);
    assert!(!h.needs_render());
    assert!(h.drain_actions().is_empty());
    h.event(&ContentEvent::Key {
        key: ControlKey::Escape,
        down: true,
    });
    assert!(h.help_tooltip().is_none());
    t.now = 2000;
    draw_toolbar_help(&mut h, &mut t, true);
    assert!(h.help_tooltip().is_none());
}

#[test]
fn deferred_toolbar_help_requires_current_layout_and_active_window() {
    let (mut h, mut t) = toolbar_help_fixture(true);
    let first = help_part(&h, "One").frame.center();
    let viewport = Rect::new(0., 0., 400., 260.);
    h.event(&ContentEvent::PointerMove(first));
    draw_toolbar_help(&mut h, &mut t, true);
    t.now = 500;
    draw_toolbar_help(&mut h, &mut t, true);
    assert_eq!(h.help_tooltip().unwrap().1, "Help for One");
    assert!(h.geometry_snapshot().is_some());

    // The original tree mutation invalidates layout without rendering. Its old
    // retained anchor must not authorize a separate late help paint pass.
    h.reorder(HELP_BAR_ROOT, &[HELP_BAR]).unwrap();
    assert!(h.geometry_snapshot().is_none());
    t.drawn.clear();
    t.requested = false;
    let mut stale = Scene::new();
    h.paint_help_overlay(&mut stale, &mut t, viewport, Theme::LIGHT.into());
    assert!(stale.primitives().is_empty());
    assert!(t.drawn.is_empty());
    assert!(!t.requested);

    draw_toolbar_help(&mut h, &mut t, true);
    assert!(h.geometry_snapshot().is_some());
    assert_eq!(h.help_tooltip().unwrap().1, "Help for One");
    h.set_window_active(false);
    t.drawn.clear();
    t.requested = false;
    let mut inactive = Scene::new();
    h.paint_help_overlay(&mut inactive, &mut t, viewport, Theme::LIGHT.into());
    assert!(inactive.primitives().is_empty());
    assert!(t.drawn.is_empty());
    assert!(!t.requested);
    assert!(h.help_tooltip().is_none());

    h.set_window_active(true);
    draw_toolbar_help(&mut h, &mut t, true);
    h.event(&ContentEvent::PointerMove(first));
    draw_toolbar_help(&mut h, &mut t, true);
    t.now = 1000;
    draw_toolbar_help(&mut h, &mut t, true);
    assert_eq!(h.help_tooltip().unwrap().1, "Help for One");
    assert!(h.drain_actions().is_empty());
}
