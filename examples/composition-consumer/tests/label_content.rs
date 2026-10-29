//! Independent public API consumer: production Content and Button state, real
//! CoreText label layout. Glyph sink records exact shaped-object identity.
use std::rc::Rc;
use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, ControlKey, TextSurfaceHost, TextSurfaceTextRenderer,
    composition::*,
};
use tekes_ui_scene::{Point, Rect, Scene, Size};
use tekes_ui_text::{
    LabelLayout, LabelOptions, LabelOverflow, Paragraph, ShapeOptions, Shaper, TextStyle,
    coretext::{CoreTextFonts, CoreTextShaper},
};
use tekes_ui_tokens::{Rgba, Theme, proposal};
struct Text {
    shaper: CoreTextShaper,
    scale: f32,
    enabled: bool,
    shaped: Vec<Rc<LabelLayout>>,
    painted: Vec<(usize, Point, Rect)>,
}
impl Text {
    fn new(scale: f32) -> Self {
        Self {
            shaper: CoreTextShaper::new(CoreTextFonts::new()),
            scale,
            enabled: true,
            shaped: Vec::new(),
            painted: Vec::new(),
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
    fn measure(&mut self, text: &str, role: proposal::TextRole, width: f32) -> Size {
        let p = self.shape_surface(text, role, Some(width));
        Size::new(p.width as f32, p.height as f32)
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
        panic!("opt-in must paint its measured layout, not legacy string");
    }
}
impl TextSurfaceHost for Text {
    fn shape_surface(
        &mut self,
        text: &str,
        role: proposal::TextRole,
        width: Option<f32>,
    ) -> Rc<Paragraph> {
        Rc::new(self.shaper.shape(
            text,
            &TextStyle::system(role.style().size * self.scale).with_weight(role.style().weight),
            &ShapeOptions {
                width: width.map(f64::from),
                line_spacing: 0.,
            },
        ))
    }
    fn shape_label(
        &mut self,
        text: &str,
        role: proposal::TextRole,
        width: f32,
        options: LabelOptions,
    ) -> Option<Rc<LabelLayout>> {
        if !self.enabled {
            return None;
        }
        let p = Rc::new(
            self.shaper
                .shape_label(
                    text,
                    &TextStyle::system(role.style().size * self.scale)
                        .with_weight(role.style().weight),
                    &ShapeOptions::with_width(width as f64),
                    options,
                )
                .with_min_line_height(f64::from(role.style().line_height * self.scale)),
        );
        self.shaped.push(p.clone());
        Some(p)
    }
}
impl TextSurfaceTextRenderer for Text {
    fn push_visible(&mut self, _: &mut Scene, p: &Paragraph, o: Point, _: Rgba, c: Rect) {
        assert_eq!(p.lines.len(), 1);
        let layout = self
            .shaped
            .iter()
            .rev()
            .find(|l| std::ptr::eq(l.paragraph(), p))
            .expect("paint must borrow exact measured layout");
        assert!(layout.height() <= c.size.height as f64 + 0.01);
        self.painted.push((p as *const Paragraph as usize, o, c));
    }
}
const ORIGINAL: &str =
    "真实长标题 English 👨‍👩‍👧‍👦 e\u{301} שלום العربية 123，完整源文不能被消费者裁剪。";
fn node(kind: usize) -> Content {
    let options = LabelOptions::single_line(LabelOverflow::TailEllipsis);
    match kind {
        0 => Content::new(ComponentId(1), TextContent::new(ORIGINAL))
            .with_label_layout(options)
            .unwrap(),
        1 => Content::new(ComponentId(1), Button::new(ORIGINAL))
            .with_label_layout(options)
            .unwrap(),
        _ => Content::new(
            ComponentId(1),
            Button::new(Content::new(ComponentId(2), TextContent::new(ORIGINAL))
                    .with_label_layout(options)
                    .unwrap()).with_accessible_name(ORIGINAL),
        ),
    }
}
fn render(
    host: &mut ContentHost,
    text: &mut Text,
    width: f32,
    theme: Theme,
) -> Result<(), CompositionError> {
    host.render(
        &mut Scene::new(),
        text,
        Rect::new(10., 20., width, 1.),
        Rect::new(0., 0., 1000., 1000.),
        theme.into(),
    )
}
#[test]
fn real_text_button_and_compound_share_final_measurement_and_preserve_full_names() {
    let mut cases = 0;
    for theme in Theme::ALL {
        for scale in [1., 1.5, 2.] {
            for kind in 0..3 {
                let mut host = ContentHost::new(node(kind)).unwrap();
                let mut text = Text::new(scale);
                for width in [100., 600., 180., 600.] {
                    render(&mut host, &mut text, width, theme).unwrap();
                    assert_eq!(text.painted.len(), cases % 4 + 1);
                    let names = host.semantics();
                    assert_eq!(names.len(), 1);
                    assert_eq!(names[0].semantics.name, ORIGINAL);
                    assert!(text.shaped.iter().all(|l| l.source() == ORIGINAL));
                    let l = text.shaped.last().unwrap();
                    assert!(l.paragraph().lines.len() == 1);
                    if width == 100. {
                        assert!(l.is_truncated());
                    }
                    cases += 1;
                }
            }
        }
    }
    assert_eq!(cases, 144);
}
#[test]
fn unavailable_label_capability_fails_even_after_a_previously_successful_frame() {
    let mut host = ContentHost::new(node(1)).unwrap();
    let mut text = Text::new(1.);
    render(&mut host, &mut text, 160., Theme::LIGHT).unwrap();
    text.enabled = false;
    let previous = text.painted.len();
    assert_eq!(
        render(&mut host, &mut text, 160., Theme::LIGHT),
        Err(CompositionError::MissingLabelServices(ComponentId(1)))
    );
    assert_eq!(
        text.painted.len(),
        previous,
        "no stale paint after missing capability"
    );
    text.enabled = true;
    text.scale = 2.;
    render(&mut host, &mut text, 160., Theme::LIGHT).unwrap();
    assert!(
        text.shaped.last().unwrap().height()
            >= proposal::TextRole::UiBody.style().line_height as f64 * 2.
    );
}
#[test]
fn opt_in_does_not_replace_button_activation_focus_or_disabled_loading_isolation() {
    let mut host = ContentHost::new(node(1)).unwrap();
    let mut text = Text::new(1.);
    render(&mut host, &mut text, 140., Theme::LIGHT).unwrap();
    assert!(host.focus(Some(ComponentId(1)), true));
    assert!(
        host.event(&ContentEvent::Key {
            key: ControlKey::Enter,
            down: true
        })
        .activated
    );
    for loading in [false, true] {
        let button = host
            .get_mut(ComponentId(1))
            .unwrap()
            .component_mut::<Button>()
            .unwrap();
        button.button.state.disabled = !loading;
        button.button.state.loading = loading;
        render(&mut host, &mut text, 140., Theme::LIGHT).unwrap();
        assert!(
            !host
                .event(&ContentEvent::Key {
                    key: ControlKey::Enter,
                    down: true
                })
                .activated
        );
        assert!(host.semantics()[0].semantics.disabled);
    }
    let button = host
        .get(ComponentId(1))
        .unwrap()
        .component::<Button>()
        .unwrap();
    assert_eq!(button.activations, 1);
    assert_eq!(button.label, ORIGINAL);
}
#[test]
fn editable_or_container_nodes_cannot_accidentally_use_readonly_label_geometry() {
    let options = LabelOptions::single_line(LabelOverflow::Clip);
    let compound = node(2);
    assert!(matches!(
        compound.with_label_layout(options),
        Err(CompositionError::UnsupportedLabelLayout(ComponentId(1)))
    ));
}

#[test]
fn intrinsic_stack_round_trip_does_not_elide_a_fully_fitting_source() {
    let mut cases = 0;
    for scale in [1., 1.5, 2.] {
        let mut host = tekes_ui_composition_consumer::label_content::build().unwrap();
        let mut text = Text::new(scale);
        for width in [1600., 1600., 1400.] {
            render(&mut host, &mut text, width, Theme::LIGHT).unwrap();
            for (address, _, _) in text.painted.iter().rev().take(3) {
                let layout = text
                    .shaped
                    .iter()
                    .find(|l| l.paragraph() as *const Paragraph as usize == *address)
                    .unwrap();
                assert!(
                    !layout.is_truncated(),
                    "a fitting intrinsic label must not elide its last grapheme: width={width},scale={scale},measured={},source={}",
                    layout.width(),
                    layout.source()
                );
            }
            cases += 1;
        }
    }
    assert_eq!(cases, 9);
}

#[test]
fn dialog_slot_recovers_label_capability_before_its_arrange_only_measurement() {
    let options = LabelOptions::single_line(LabelOverflow::TailEllipsis);
    let slot = |id| {
        Content::new(ComponentId(id), TextContent::new(ORIGINAL))
            .with_label_layout(options)
            .unwrap()
    };
    let mut host = ContentHost::new(Content::new(
        ComponentId(1),
        DialogContent::new("Label capability recovery", slot(2), slot(3), slot(4)),
    ))
    .unwrap();
    host.show_dialog(ComponentId(1)).unwrap();
    let mut text = Text::new(1.);
    text.enabled = false;
    let frame = Rect::new(0., 0., 720., 500.);
    assert_eq!(
        host.render(
            &mut Scene::new(),
            &mut text,
            frame,
            frame,
            Theme::LIGHT.into()
        ),
        Err(CompositionError::MissingLabelServices(ComponentId(2)))
    );
    text.enabled = true;
    host.render(
        &mut Scene::new(),
        &mut text,
        frame,
        frame,
        Theme::LIGHT.into(),
    )
    .expect("a previous failed slot must be remeasured with the new available host");
    assert!(!text.painted.is_empty());
}

#[test]
fn toolbar_defaults_to_single_line_for_original_text_and_button_at_narrow_widths() {
    for scale in [1., 1.5, 2.] {
        for width in [80., 160., 480.] {
            let mut text = Text::new(scale);
            let root = Content::new(
                ComponentId(600),
                ToolbarContent::new(vec![
                    Content::new(
                        ComponentId(601),
                        ToolbarGroup::new(
                            ToolbarPlacement::Navigation,
                            vec![Content::new(ComponentId(602), TextContent::new(ORIGINAL))],
                        ),
                    ),
                    Content::new(
                        ComponentId(603),
                        ToolbarGroup::new(
                            ToolbarPlacement::PrimaryAction,
                            vec![Content::new(ComponentId(604), Button::new(ORIGINAL))],
                        ),
                    ),
                ]),
            );
            let mut host = ContentHost::new(root).unwrap();
            let frame = Rect::new(0., 0., width, 100. * scale);
            host.render(
                &mut Scene::new(),
                &mut text,
                frame,
                frame,
                Theme::LIGHT.into(),
            )
            .unwrap();
            // A fully squeezed slot may have no visible glyphs, but never wraps.
            if width > 80. {
                assert!(!text.painted.is_empty());
            }
            assert!(text.shaped.iter().all(|l| l.paragraph().lines.len() <= 1));
            assert!(text.shaped.iter().any(|l| l.is_truncated()));
            for id in [ComponentId(602), ComponentId(604)] {
                let node = host.get(id).unwrap();
                assert_eq!(
                    node.component::<TextContent>()
                        .map(|c| c.text.as_str())
                        .or_else(|| node.component::<Button>().map(|c| c.label.as_str()))
                        .unwrap(),
                    ORIGINAL
                );
            }
            // Reflow the same mounted controls; do not reconstruct labels/actions.
            let mount = host.semantic_mount(ComponentId(604)).unwrap();
            let larger = Rect::new(0., 0., width + 100., frame.size.height);
            host.render(
                &mut Scene::new(),
                &mut text,
                larger,
                larger,
                Theme::LIGHT.into(),
            )
            .unwrap();
            assert_eq!(host.semantic_mount(ComponentId(604)).unwrap(), mount);
        }
    }
}

#[test]
fn opt_in_label_keeps_original_pointer_chrome_frame_and_focus_ring() {
    for kind in [1, 2] {
        let id = ComponentId(1);
        let mut host = ContentHost::new(node(kind)).unwrap();
        let mut text = Text::new(1.5);
        render(&mut host, &mut text, 140., Theme::LIGHT).unwrap();
        let frame = host.get(id).unwrap().frame();
        let mount = host.semantic_mount(id).unwrap();
        let point = frame.center();
        host.event(&ContentEvent::PointerMove(point));
        assert!(
            host.get(id)
                .unwrap()
                .component::<Button>()
                .unwrap()
                .button
                .state
                .hovered
        );
        assert!(!host.event(&ContentEvent::PointerDown(point)).activated);
        assert!(
            host.get(id)
                .unwrap()
                .component::<Button>()
                .unwrap()
                .button
                .state
                .pressed
        );
        assert!(host.event(&ContentEvent::PointerUp(point)).activated);
        assert_eq!(
            host.get(id)
                .unwrap()
                .component::<Button>()
                .unwrap()
                .activations,
            1
        );
        host.focus(Some(id), true);
        let mut scene = Scene::new();
        let bounds = Rect::new(10., 20., 140., 1.);
        host.render(
            &mut scene,
            &mut text,
            bounds,
            Rect::new(0., 0., 1000., 1000.),
            Theme::LIGHT.into(),
        )
        .unwrap();
        let bleed = proposal::metric("stroke", "c-focus-width").unwrap()
            + proposal::metric("spacing", "c-focus-offset").unwrap();
        assert!(scene.primitives().iter().any(|p| matches!(p,
            tekes_ui_scene::Primitive::Quad(q) if q.rect == frame.inset(-bleed)
                && q.fill.a == 0. && q.border_width > 0.)));
        let semantics = host.semantics();
        assert_eq!(semantics.len(), 1);
        assert_eq!(semantics[0].frame, frame);
        assert_eq!(semantics[0].semantics.name, ORIGINAL);
        assert_eq!(host.semantic_mount(id).unwrap(), mount);
        for loading in [false, true] {
            let button = host.get_mut(id).unwrap().component_mut::<Button>().unwrap();
            button.button.state.disabled = !loading;
            button.button.state.loading = loading;
            assert!(!host.event(&ContentEvent::PointerDown(point)).activated);
            assert!(!host.event(&ContentEvent::PointerUp(point)).activated);
            assert_eq!(
                host.get(id)
                    .unwrap()
                    .component::<Button>()
                    .unwrap()
                    .activations,
                1
            );
        }
    }
}
