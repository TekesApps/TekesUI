use tekes_ui_controls::composition::*;
use tekes_ui_controls::{ButtonTextAlign, ButtonTextRenderer, ControlKey};
use tekes_ui_scene::{Rect, Scene, Size};
use tekes_ui_tokens::{Rgba, Theme, proposal};
struct Labels;
impl ButtonTextRenderer for Labels {
    fn measure(&mut self, s: &str, r: proposal::TextRole, w: f32) -> Size {
        Size::new((s.len() as f32 * 7.).min(w), r.style().line_height)
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
fn render(h: &mut ContentHost) -> Result<(), CompositionError> {
    h.render(
        &mut Scene::new(),
        &mut Labels,
        Rect::new(0., 0., 500., 400.),
        Rect::new(0., 0., 500., 400.),
        Theme::LIGHT.into(),
    )
}
#[test]
fn composed_input_requires_an_explicit_host_text_capability() {
    let mut host = ContentHost::new(Content::new(
        ComponentId(1),
        TextFieldContent::new("Name", "value"),
    ))
    .unwrap();
    assert_eq!(
        render(&mut host),
        Err(CompositionError::MissingTextServices(ComponentId(1)))
    );
    assert!(host.focused_text().is_none());
}
#[test]
fn fixed_dialog_slots_are_protected_and_closed_content_is_not_semantically_exposed() {
    let mut host = ContentHost::new(Content::new(
        ComponentId(1),
        DialogContent::new(
            "Preferences",
            Content::new(
                ComponentId(2),
                TextContent {
                    text: "Preferences".into(),
                    role: proposal::TextRole::UiHeading,
                },
            ),
            Content::new(ComponentId(3), Button::new("Body action")),
            Content::new(ComponentId(4), Button::new("Close")),
        ),
    ))
    .unwrap();
    render(&mut host).unwrap();
    assert!(host.semantics().is_empty());
    assert!(host.focus_targets().is_empty());
    assert_eq!(
        host.insert(
            ComponentId(1),
            Content::new(ComponentId(5), Button::new("Wrong slot"))
        ),
        Err(CompositionError::NotContainer(ComponentId(1)))
    );
    assert_eq!(
        host.reorder(
            ComponentId(1),
            &[ComponentId(4), ComponentId(3), ComponentId(2)]
        ),
        Err(CompositionError::NotContainer(ComponentId(1)))
    );
    host.show_dialog(ComponentId(1)).unwrap();
    render(&mut host).unwrap();
    assert_eq!(host.semantics()[0].semantics.role, ContentRole::Dialog);
    assert_eq!(host.focused(), Some(ComponentId(3)));
    host.event(&ContentEvent::Cancel);
    render(&mut host).unwrap();
    assert_eq!(
        host.focused(),
        None,
        "render must not reacquire cancelled modal text focus on window deactivation"
    );
    host.event(&ContentEvent::Key {
        key: ControlKey::Tab { reverse: false },
        down: true,
    });
    assert_eq!(host.focused(), Some(ComponentId(3)));
    host.event(&ContentEvent::Key {
        key: ControlKey::Tab { reverse: false },
        down: true,
    });
    assert_eq!(host.focused(), Some(ComponentId(4)));
    host.event(&ContentEvent::Key {
        key: ControlKey::Tab { reverse: false },
        down: true,
    });
    assert_eq!(host.focused(), Some(ComponentId(3)));
    assert_eq!(
        host.show_dialog(ComponentId(1)),
        Err(CompositionError::ModalAlreadyOpen)
    );
    host.event(&ContentEvent::Key {
        key: ControlKey::Escape,
        down: true,
    });
    render(&mut host).unwrap();
    assert!(host.semantics().is_empty());
    assert!(host.focus_targets().is_empty());
}

#[derive(Debug)]
struct Marker;
impl Component for Marker {
    fn measure(
        &mut self,
        _: &mut dyn ButtonTextRenderer,
        a: Size,
    ) -> tekes_ui_controls::layout::MeasuredChild {
        tekes_ui_controls::layout::MeasuredChild::new(Size::new(a.width, 800.))
    }
    fn paint(
        &mut self,
        s: &mut Scene,
        _: &mut dyn ButtonTextRenderer,
        _: Rect,
        _: Rect,
        t: ContentTheme,
    ) {
        s.push_quad(tekes_ui_scene::Quad::new(
            Rect::new(9., 9., 11., 11.),
            t.palette.color("primary-text").unwrap(),
        ));
    }
}
#[derive(Debug)]
struct Siblings(Vec<Content>);
impl Component for Siblings {
    fn measure(
        &mut self,
        _: &mut dyn ButtonTextRenderer,
        a: Size,
    ) -> tekes_ui_controls::layout::MeasuredChild {
        tekes_ui_controls::layout::MeasuredChild::new(a)
    }
    fn arrange(&mut self, t: &mut dyn ButtonTextRenderer, f: Rect, c: Rect) {
        for child in &mut self.0 {
            child.arrange(t, f, c);
        }
    }
    fn children(&self) -> &[Content] {
        &self.0
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.0)
    }
}
#[test]
fn active_modal_paints_after_later_background_siblings() {
    let dialog = Content::new(
        ComponentId(2),
        DialogContent::new(
            "Settings",
            Content::new(
                ComponentId(3),
                TextContent {
                    text: "Header".into(),
                    role: proposal::TextRole::UiHeading,
                },
            ),
            Content::new(ComponentId(4), Button::new("Body")),
            Content::new(ComponentId(5), Button::new("Close")),
        ),
    );
    let mut host = ContentHost::new(Content::new(
        ComponentId(1),
        Siblings(vec![dialog, Content::new(ComponentId(6), Marker)]),
    ))
    .unwrap();
    host.show_dialog(ComponentId(2)).unwrap();
    let mut scene = Scene::new();
    host.render(
        &mut scene,
        &mut Labels,
        Rect::new(0., 0., 500., 400.),
        Rect::new(0., 0., 500., 400.),
        Theme::LIGHT.into(),
    )
    .unwrap();
    let quads = scene
        .primitives()
        .iter()
        .filter_map(|p| {
            if let tekes_ui_scene::Primitive::Quad(q) = p {
                Some(q)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    let marker = quads
        .iter()
        .position(|q| q.rect == Rect::new(9., 9., 11., 11.))
        .unwrap();
    let scrim = quads
        .iter()
        .position(|q| {
            q.fill
                == tekes_ui_tokens::Palette::from(Theme::LIGHT)
                    .color("scrim")
                    .unwrap()
        })
        .unwrap();
    assert!(
        marker < scrim,
        "modal scrim and slots must paint after every background sibling"
    );
    assert_eq!(host.semantics()[0].id, ComponentId(2));
}
#[test]
fn ignored_menu_wheel_bubbles_to_the_scroll_ancestor() {
    use tekes_ui_controls::{
        ScrollArea,
        extended::{Axis, Stack},
    };
    let stack = Content::new(
        ComponentId(2),
        StackContent {
            flexible_child: None,
            model: Stack {
                axis: Axis::Vertical,
                gap: 8.,
            },
            children: vec![
                Content::new(
                    ComponentId(3),
                    MenuContent::new("Options", vec!["One".into(), "Two".into()]),
                ),
                Content::new(ComponentId(4), Marker),
            ],
        },
    );
    let mut host = ContentHost::new(Content::new(
        ComponentId(1),
        ScrollContent::new(ScrollArea::default(), stack),
    ))
    .unwrap();
    render(&mut host).unwrap();
    host.focus(Some(ComponentId(3)), true);
    host.event(&ContentEvent::Key {
        key: ControlKey::Enter,
        down: true,
    });
    render(&mut host).unwrap();
    assert!(
        host.get(ComponentId(3))
            .unwrap()
            .component::<MenuContent>()
            .unwrap()
            .menu
            .is_open()
    );
    let response = host.event(&ContentEvent::Scroll {
        position: tekes_ui_scene::Point::new(490., 390.),
        delta: tekes_ui_scene::Point::new(0., 40.),
        precise: true,
        begin: true,
        end: false,
        momentum: false,
        time: 1.,
    });
    assert!(response.consumed && response.changed);
    assert!(
        host.get(ComponentId(1))
            .unwrap()
            .component::<ScrollContent>()
            .unwrap()
            .scroll
            .offset
            .y
            > 0.
    );
}

#[derive(Default)]
struct ClockLabels {
    now: u64,
    reduced: bool,
    frames: usize,
    draws: usize,
}
impl ButtonTextRenderer for ClockLabels {
    fn measure(&mut self, s: &str, r: proposal::TextRole, w: f32) -> Size {
        Labels.measure(s, r, w)
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
        self.draws += 1;
    }
    fn motion_context(&self) -> Option<tekes_ui_controls::ControlMotionContext> {
        Some(tekes_ui_controls::ControlMotionContext {
            now: std::time::Duration::from_millis(self.now),
            reduced_motion: self.reduced,
        })
    }
    fn request_animation_frame(&mut self) {
        self.frames += 1;
    }
}
fn clock_scene(host: &mut ContentHost, t: &mut ClockLabels, theme: Theme) -> Scene {
    let mut s = Scene::new();
    host.render(
        &mut s,
        t,
        Rect::new(0., 0., 500., 400.),
        Rect::new(0., 0., 500., 400.),
        theme.into(),
    )
    .unwrap();
    s
}
fn dialog_host() -> ContentHost {
    ContentHost::new(Content::new(
        ComponentId(1),
        Siblings(vec![
            Content::new(ComponentId(10), Button::new("Open")),
            Content::new(
                ComponentId(2),
                DialogContent::new(
                    "Settings",
                    Content::new(
                        ComponentId(3),
                        TextContent {
                            text: "Header".into(),
                            role: proposal::TextRole::UiHeading,
                        },
                    ),
                    Content::new(ComponentId(4), Button::new("Body")),
                    Content::new(ComponentId(5), Button::new("Close")),
                ),
            ),
        ]),
    ))
    .unwrap()
}
fn scrim_alpha(scene: &Scene) -> f32 {
    scene
        .primitives()
        .iter()
        .filter_map(|p| match p {
            tekes_ui_scene::Primitive::Quad(q)
                if q.rect == Rect::new(0., 0., 500., 400.)
                    && q.fill.r == 0.
                    && q.fill.g == 0.
                    && q.fill.b == 0. =>
            {
                Some(q.fill.a)
            }
            _ => None,
        })
        .next_back()
        .unwrap_or(0.)
}
#[test]
fn dialog_exit_retains_only_fresh_paint_and_restores_focus_once() {
    let mut h = dialog_host();
    let mut t = ClockLabels::default();
    clock_scene(&mut h, &mut t, Theme::LIGHT);
    h.focus(Some(ComponentId(10)), true);
    h.show_dialog(ComponentId(2)).unwrap();
    let start = clock_scene(&mut h, &mut t, Theme::LIGHT);
    assert_eq!(scrim_alpha(&start), 0.);
    assert!(t.frames > 0);
    t.now = 1000;
    let full = clock_scene(&mut h, &mut t, Theme::LIGHT);
    let full_alpha = scrim_alpha(&full);
    assert!(full_alpha > 0.);
    h.close_dialog();
    assert_eq!(h.focused(), Some(ComponentId(10)));
    assert!(
        !h.focus(Some(ComponentId(4)), true),
        "closed subtrees cannot reacquire input focus while fading"
    );
    assert_eq!(h.focused(), Some(ComponentId(10)));
    assert!(h.semantics().iter().all(|n| n.id != ComponentId(2)));
    let previous_draws = t.draws;
    t.now = 1001;
    clock_scene(&mut h, &mut t, Theme::LIGHT);
    assert!(
        t.draws > previous_draws,
        "exit must rebuild glyphs in the current atlas frame"
    );
    t.now = 1050;
    let exit = clock_scene(&mut h, &mut t, Theme::LIGHT);
    assert!(scrim_alpha(&exit) > 0. && scrim_alpha(&exit) < full_alpha);
    assert!(
        h.get(ComponentId(2))
            .unwrap()
            .component::<DialogContent>()
            .unwrap()
            .is_presented()
    );
    let stale_button = h.get(ComponentId(4)).unwrap().frame().center();
    h.event(&ContentEvent::PointerDown(stale_button));
    h.event(&ContentEvent::PointerUp(stale_button));
    assert_eq!(
        h.get(ComponentId(4))
            .unwrap()
            .component::<Button>()
            .unwrap()
            .activations,
        0
    );
    t.now = 2000;
    clock_scene(&mut h, &mut t, Theme::LIGHT);
    assert!(
        !h.get(ComponentId(2))
            .unwrap()
            .component::<DialogContent>()
            .unwrap()
            .is_presented()
    );
    assert_eq!(h.focused(), Some(ComponentId(10)));
}
#[test]
fn dialog_reverse_reopen_reduction_and_resource_invalidation_share_presence_contract() {
    let mut h = dialog_host();
    let mut t = ClockLabels::default();
    h.show_dialog(ComponentId(2)).unwrap();
    clock_scene(&mut h, &mut t, Theme::LIGHT);
    t.now = 1000;
    clock_scene(&mut h, &mut t, Theme::LIGHT);
    h.close_dialog();
    clock_scene(&mut h, &mut t, Theme::LIGHT);
    t.now = 1050;
    let exiting = clock_scene(&mut h, &mut t, Theme::LIGHT);
    let alpha = scrim_alpha(&exiting);
    h.show_dialog(ComponentId(2)).unwrap();
    let reversing = clock_scene(&mut h, &mut t, Theme::LIGHT);
    assert!(
        (scrim_alpha(&reversing) - alpha).abs() < 0.00001,
        "reversal is continuous"
    );
    t.now = 2000;
    clock_scene(&mut h, &mut t, Theme::LIGHT);
    h.close_dialog();
    t.reduced = true;
    let reduced = clock_scene(&mut h, &mut t, Theme::LIGHT);
    assert_eq!(scrim_alpha(&reduced), 0.);
    assert!(
        !h.get(ComponentId(2))
            .unwrap()
            .component::<DialogContent>()
            .unwrap()
            .is_presented()
    );
    h.show_dialog(ComponentId(2)).unwrap();
    t.reduced = false;
    t.now = 2100;
    clock_scene(&mut h, &mut t, Theme::LIGHT);
    t.now = 3100;
    clock_scene(&mut h, &mut t, Theme::LIGHT);
    h.close_dialog();
    clock_scene(&mut h, &mut t, Theme::LIGHT);
    t.now = 3150;
    let changed = clock_scene(&mut h, &mut t, Theme::DARK);
    assert_eq!(
        scrim_alpha(&changed),
        0.,
        "theme/resource context change discards exit paint"
    );
    h.show_dialog(ComponentId(2)).unwrap();
    clock_scene(&mut h, &mut t, Theme::LIGHT);
    t.now = 4150;
    clock_scene(&mut h, &mut t, Theme::LIGHT);
    h.close_dialog();
    h.clear_dialog_presentation();
    assert!(
        !h.get(ComponentId(2))
            .unwrap()
            .component::<DialogContent>()
            .unwrap()
            .is_presented()
    );
}
