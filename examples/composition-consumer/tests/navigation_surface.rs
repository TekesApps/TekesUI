use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, ControlKey, extended::Breadcrumb,
    layout_view::BreadcrumbView,
};
use tekes_ui_scene::{Point, Primitive, Rect, Scene, Size};
use tekes_ui_tokens::{
    Palette, Rgba, Theme, proposal,
    surface_palette::{resolve_surface, surface_names},
};
#[derive(Default)]
struct Text(Vec<(String, Rgba)>);
impl ButtonTextRenderer for Text {
    fn measure(&mut self, s: &str, _: proposal::TextRole, w: f32) -> Size {
        Size::new((s.len() as f32 * 7.).min(w), 20.)
    }
    fn draw(
        &mut self,
        _: &mut Scene,
        s: &str,
        _: proposal::TextRole,
        _: Rect,
        _: ButtonTextAlign,
        c: Rgba,
    ) {
        self.0.push((s.into(), c));
    }
}
#[derive(Default)]
struct WrappingText {
    scale: f32,
    drawn: Vec<(String, Rect)>,
}
impl ButtonTextRenderer for WrappingText {
    fn resolved_font_size(&mut self, role: proposal::TextRole) -> f32 {
        role.style().size * if self.scale > 0.0 { self.scale } else { 1.0 }
    }
    fn measure(&mut self, value: &str, role: proposal::TextRole, width: f32) -> Size {
        if !width.is_finite() || width <= 0.0 {
            return Size::default();
        }
        let scale = self.resolved_font_size(role) / role.style().size;
        let natural_width = value.chars().count() as f32 * 8.0 * scale;
        let lines = (natural_width / width).ceil().max(1.0);
        Size::new(
            natural_width.min(width),
            role.style().line_height * scale * lines,
        )
    }
    fn draw(
        &mut self,
        _: &mut Scene,
        value: &str,
        _: proposal::TextRole,
        frame: Rect,
        _: ButtonTextAlign,
        _: Rgba,
    ) {
        self.drawn.push((value.to_owned(), frame));
    }
}
struct CoreTextMetrics {
    scale: f32,
    shaper: tekes_ui_text::coretext::CoreTextShaper,
    drawn: Vec<(String, Rect)>,
    measured: Vec<(String, f32, Size)>,
}
impl CoreTextMetrics {
    fn new(scale: f32) -> Self {
        Self {
            scale,
            shaper: tekes_ui_text::coretext::CoreTextShaper::new(
                tekes_ui_text::coretext::CoreTextFonts::new(),
            ),
            drawn: Vec::new(),
            measured: Vec::new(),
        }
    }
}
impl ButtonTextRenderer for CoreTextMetrics {
    fn resolved_font_size(&mut self, role: proposal::TextRole) -> f32 {
        role.style().size * self.scale
    }
    fn measure(&mut self, value: &str, role: proposal::TextRole, width: f32) -> Size {
        use tekes_ui_text::traits::Shaper;
        if !width.is_finite() || width <= 0.0 {
            return Size::default();
        }
        let paragraph = self.shaper.shape(
            value,
            &tekes_ui_text::style::TextStyle::system(role.style().size * self.scale),
            &tekes_ui_text::model::ShapeOptions {
                width: Some(f64::from(width)),
                line_spacing: 0.0,
            },
        );
        let measured = Size::new(paragraph.width as f32, paragraph.height as f32);
        self.measured.push((value.to_owned(), width, measured));
        measured
    }
    fn draw(
        &mut self,
        _: &mut Scene,
        value: &str,
        _: proposal::TextRole,
        frame: Rect,
        _: ButtonTextAlign,
        _: Rgba,
    ) {
        self.drawn.push((value.to_owned(), frame));
    }
}
fn model() -> Breadcrumb {
    Breadcrumb {
        items: vec!["Home".into(), "Library".into(), "Current".into()],
    }
}
fn draw(v: &mut BreadcrumbView, m: &Breadcrumb, t: &mut Text, p: Palette) {
    t.0.clear();
    let labels = m.items.iter().map(String::as_str).collect::<Vec<_>>();
    v.render(
        &mut Scene::new(),
        t,
        m,
        &labels,
        Rect::new(10., 10., 700., 40.),
        None,
        p,
        proposal::TextRole::UiBody,
    );
}
fn assert_primary_trigger_fill(scene: &Scene, frame: Rect, p: Palette, state: &str) {
    let expected = resolve_surface(p, "canvas", "action-primary", state)
        .unwrap()
        .background;
    assert!(scene.primitives().iter().any(
        |primitive| matches!(primitive, Primitive::Quad(quad) if quad.rect == frame && quad.fill == expected)
    ), "trigger chrome should use {state} background {expected:?}");
}
#[test]
fn original_ancestors_current_page_and_passive_separators_use_the_declared_parent() {
    for theme in Theme::ALL {
        for brand in [0, 1, 2] {
            for surface in surface_names() {
                for phase in ["rest", "hover", "pressed", "focus", "disabled"] {
                    let p = Palette::new(theme, brand).unwrap();
                    let mut v = BreadcrumbView::default();
                    v.set_surface(surface).unwrap();
                    let m = model();
                    let mut t = Text::default();
                    draw(&mut v, &m, &mut t, p);
                    let point = v.layout().items[0].frame().center();
                    match phase {
                        "hover" => v.pointer_move(point),
                        "pressed" => {
                            assert_eq!(v.pointer_down(point), Some(0));
                        }
                        "focus" => v.focus(0),
                        "disabled" => v.set_disabled(true),
                        _ => {}
                    }
                    draw(&mut v, &m, &mut t, p);
                    let expected = resolve_surface(
                        p,
                        surface,
                        "action-quiet",
                        if phase == "focus" { "rest" } else { phase },
                    )
                    .unwrap();
                    assert_eq!(t.0.iter().find(|x| x.0 == "Home").unwrap().1, expected.text);
                    assert!(t.0.iter().filter(|x| x.0 == "/").all(
                        |x| x.1 == resolve_surface(p, surface, "content", "rest").unwrap().text
                    ));
                    assert_eq!(
                        t.0.iter().find(|x| x.0 == "Current").unwrap().1,
                        resolve_surface(p, surface, "action-quiet", "selected-disabled")
                            .unwrap()
                            .text
                    );
                    assert_eq!(v.activate(2, &m), None);
                }
            }
        }
    }
}
#[test]
fn parent_edits_preserve_original_arm_and_new_path_items_inherit_without_special_wiring() {
    let mut v = BreadcrumbView::default();
    let mut m = model();
    let mut t = Text::default();
    let p = Palette::from(Theme::LIGHT);
    draw(&mut v, &m, &mut t, p);
    let point = v.layout().items[0].frame().center();
    assert_eq!(v.pointer_down(point), Some(0));
    v.set_surface("panel").unwrap();
    assert!(v.set_surface("missing-parent").is_err());
    draw(&mut v, &m, &mut t, p);
    assert_eq!(v.pointer_up(point, &m), Some(0));
    assert_eq!(v.pointer_up(point, &m), None);
    m.items.push("Details".into());
    draw(&mut v, &m, &mut t, p);
    assert_eq!(
        t.0.iter().find(|x| x.0 == "Current").unwrap().1,
        resolve_surface(p, "panel", "action-quiet", "rest")
            .unwrap()
            .text
    );
    assert_eq!(
        t.0.iter().find(|x| x.0 == "Details").unwrap().1,
        resolve_surface(p, "panel", "action-quiet", "selected-disabled")
            .unwrap()
            .text
    );
    v.focus(0);
    assert_eq!(v.key_event(ControlKey::Enter, true, &m), (true, Some(0)));
    v.set_disabled(true);
    assert_eq!(v.key_event(ControlKey::Enter, true, &m), (false, None));
}

#[test]
fn tabs_use_parent_packets_without_replacing_selection_or_capture() {
    use tekes_ui_controls::TabsView;
    for theme in Theme::ALL {
        for brand in [0, 1, 2] {
            for surface in surface_names() {
                for disabled in [false, true] {
                    let p = Palette::new(theme, brand).unwrap();
                    let mut v = TabsView::new(2);
                    v.set_surface(surface).unwrap();
                    v.set_disabled(disabled);
                    let mut t = Text::default();
                    v.render(
                        &mut Scene::new(),
                        &mut t,
                        &["One", "Two"],
                        Rect::new(10., 10., 500., 50.),
                        None,
                        p,
                        proposal::TextRole::UiBody,
                        false,
                    );
                    for (label, state) in [
                        (
                            "One",
                            if disabled {
                                "selected-disabled"
                            } else {
                                "selected"
                            },
                        ),
                        ("Two", if disabled { "disabled" } else { "rest" }),
                    ] {
                        assert_eq!(
                            t.0.iter().find(|x| x.0 == label).unwrap().1,
                            resolve_surface(p, surface, "action-quiet", state)
                                .unwrap()
                                .text
                        );
                    }
                }
            }
        }
    }
    let p = Palette::from(Theme::LIGHT);
    let mut v = TabsView::new(2);
    let mut t = Text::default();
    let layout = v.render(
        &mut Scene::new(),
        &mut t,
        &["One", "Two"],
        Rect::new(10., 10., 500., 50.),
        None,
        p,
        proposal::TextRole::UiBody,
        false,
    );
    let point = layout.items[1].frame.center();
    assert_eq!(v.pointer_down_at(point), Some(1));
    v.set_surface("panel").unwrap();
    assert!(v.set_surface("missing-parent").is_err());
    assert_eq!(v.pointer_up_at(point), Some(1));
    assert_eq!(v.selected(), 1);
    assert_eq!(v.pointer_up_at(point), None);
    v.set_disabled(true);
    assert_eq!(v.pointer_down_at(point), None);
}

#[test]
fn command_trigger_inherits_parent_without_changing_popup_scope_or_original_open_action() {
    use tekes_ui_controls::{extended::CommandPalette, layout_view::CommandPaletteView};
    for theme in Theme::ALL {
        for brand in [0, 1, 2] {
            for surface in surface_names() {
                for disabled in [false, true] {
                    let p = Palette::new(theme, brand).unwrap();
                    let mut v = CommandPaletteView::default();
                    let mut m = CommandPalette::default();
                    m.combo.disabled = disabled;
                    v.set_surface(surface).unwrap();
                    let mut t = Text::default();
                    v.render_trigger(
                        &mut Scene::new(),
                        &mut t,
                        &mut m,
                        "Commands",
                        Rect::new(10., 10., 300., 50.),
                        None,
                        p,
                        proposal::TextRole::UiBody,
                    );
                    assert_eq!(
                        t.0[0].1,
                        resolve_surface(
                            p,
                            surface,
                            "action-primary",
                            if disabled { "disabled" } else { "rest" }
                        )
                        .unwrap()
                        .text
                    );
                    assert!(!m.combo.open);
                }
            }
        }
    }
    let p = Palette::from(Theme::LIGHT);
    let mut v = CommandPaletteView::default();
    let mut m = CommandPalette::default();
    let layout = v.render_trigger(
        &mut Scene::new(),
        &mut Text::default(),
        &mut m,
        "Commands",
        Rect::new(10., 10., 300., 50.),
        None,
        p,
        proposal::TextRole::UiBody,
    );
    let point = layout.frame.center();
    assert!(v.pointer_down_trigger(point));
    v.set_surface("panel").unwrap();
    assert!(v.set_surface("missing-parent").is_err());
    assert!(v.pointer_up_trigger(point, &mut m));
    assert!(m.combo.open);
    assert!(!v.pointer_up_trigger(point, &mut m));
    v.cancel(&mut m);
    v.focus_trigger(true);
    assert!(v.trigger_key(ControlKey::Enter, true, &mut m).changed);
    assert!(m.combo.open);
    m.combo.disabled = true;
    assert!(!v.trigger_key(ControlKey::Enter, true, &mut m).changed);
    assert!(!m.combo.open);
}

#[test]
fn replacing_a_breadcrumb_path_revokes_old_pointer_key_and_semantic_actions() {
    for route in ["pointer", "keyboard", "semantic", "repaint"] {
        let mut v = BreadcrumbView::default();
        let mut m = model();
        let mut t = Text::default();
        let p = Palette::from(Theme::LIGHT);
        draw(&mut v, &m, &mut t, p);
        let point = v.layout().items[0].frame().center();
        assert_eq!(v.pointer_down(point), Some(0));
        v.focus(0);
        // Equal length must not make the new business path inherit an old arm.
        m.items[0] = "Different root".into();
        match route {
            "pointer" => assert_eq!(v.pointer_up(point, &m), None),
            "keyboard" => assert_eq!(v.key_event(ControlKey::Enter, true, &m), (false, None)),
            "semantic" => assert_eq!(v.activate(0, &m), None),
            "repaint" => {
                draw(&mut v, &m, &mut t, p);
                assert_eq!(v.pointer_up(point, &m), None);
                assert_eq!(v.key_event(ControlKey::Enter, true, &m), (false, None));
            }
            _ => unreachable!(),
        }
        // Rendering the replacement and a new gesture may act exactly once.
        draw(&mut v, &m, &mut t, p);
        let point = v.layout().items[0].frame().center();
        assert_eq!(v.pointer_down(point), Some(0));
        assert_eq!(v.pointer_up(point, &m), Some(0));
        assert_eq!(v.pointer_up(point, &m), None);
    }
}

#[test]
fn breadcrumb_observing_a_replacement_prevents_old_path_resurrection() {
    let mut v = BreadcrumbView::default();
    let mut m = model();
    let mut t = Text::default();
    let p = Palette::from(Theme::LIGHT);
    draw(&mut v, &m, &mut t, p);
    let point = v.layout().items[0].frame().center();
    assert_eq!(v.pointer_down(point), Some(0));

    m.items[0] = "Different root".into();
    assert_eq!(v.pointer_up(point, &m), None); // observes and revokes old arm
    assert_eq!(v.layout().items.len(), 0);
    m.items[0] = "Home".into();
    assert_eq!(v.pointer_up(point, &m), None); // A -> observed B -> A cannot revive it
    assert_eq!(v.activate(0, &m), None); // no replacement geometry was published
    assert_eq!(m.activate(0), Some(0)); // original model remains a pure index query

    draw(&mut v, &m, &mut t, p);
    assert_eq!(v.pointer_up(point, &m), None);
}

#[test]
fn breadcrumb_measure_render_and_hit_geometry_share_fair_wrapped_segments() {
    let labels = [
        "Home",
        "An intermediate collection with a deliberately long name",
        "Current page with a long title",
    ];
    let model = Breadcrumb {
        items: labels.iter().map(|label| (*label).to_owned()).collect(),
    };
    let role = proposal::TextRole::UiBody;
    let palette = Palette::from(Theme::LIGHT);

    for scale in [1.0, 1.5, 2.0] {
        let mut text = WrappingText {
            scale,
            ..WrappingText::default()
        };
        let mut view = BreadcrumbView::default();
        let measured = view.measure(&mut text, &model, &labels, 320.0, role);
        let layout = view.render(
            &mut Scene::new(),
            &mut text,
            &model,
            &labels,
            Rect::new(10.0, 10.0, 320.0, 38.0),
            None,
            palette,
            role,
        );

        assert_eq!(layout.frame.size, measured);
        assert!(layout.frame.size.height > 38.0);
        assert_eq!(layout.items.len(), labels.len());
        for item in &layout.items {
            let frame = item.frame();
            assert!(frame.min_x() >= layout.frame.min_x());
            assert!(frame.max_x() <= layout.frame.max_x() + 0.001);
            assert!(frame.min_y() >= layout.frame.min_y());
            assert!(frame.max_y() <= layout.frame.max_y() + 0.001);
        }
        let slash_frames = text
            .drawn
            .iter()
            .filter(|(value, _)| value == "/")
            .map(|(_, frame)| *frame)
            .collect::<Vec<_>>();
        assert_eq!(slash_frames.len(), labels.len() - 1);
        for slash in slash_frames {
            assert!(
                !layout
                    .items
                    .iter()
                    .any(|item| item.frame().contains(slash.center()))
            );
        }

        view.focus(0);
        text.drawn.clear();
        let focused_layout = view.render(
            &mut Scene::new(),
            &mut text,
            &model,
            &labels,
            Rect::new(10.0, 10.0, 320.0, 38.0),
            None,
            palette,
            role,
        );
        let focus_outset = proposal::metric("spacing", "c-focus-offset").unwrap()
            + proposal::metric("stroke", "c-focus-width").unwrap();
        let focused_frame = focused_layout.items[0].frame();
        let expanded_focus = Rect::new(
            focused_frame.min_x() - focus_outset,
            focused_frame.min_y() - focus_outset,
            focused_frame.size.width + 2.0 * focus_outset,
            focused_frame.size.height + 2.0 * focus_outset,
        );
        for (_, slash) in text.drawn.iter().filter(|(value, _)| value == "/") {
            assert!(slash.intersection(&expanded_focus).is_none());
        }

        let clip = Rect::new(10.0, 10.0, 320.0, 38.0);
        text.drawn.clear();
        let clipped = view.render(
            &mut Scene::new(),
            &mut text,
            &model,
            &labels,
            Rect::new(10.0, 10.0, 320.0, 38.0),
            Some(clip),
            palette,
            role,
        );
        assert_eq!(clipped.frame.size.height, 38.0_f32.max(measured.height));
        assert!(clipped.items[0].visible_frame().is_some());
        assert!(
            clipped
                .items
                .iter()
                .skip(1)
                .any(|item| item.visible_frame().is_none())
        );
        for item in &clipped.items {
            assert_eq!(item.visible_frame(), item.frame().intersection(&clip));
            if item.visible_frame().is_none() {
                assert_eq!(view.pointer_down(item.frame().center()), None);
            }
        }
    }

    let mut text = WrappingText::default();
    let view = BreadcrumbView::default();
    assert_eq!(
        view.measure(&mut text, &model, &labels, 0.0, role),
        Size::new(0.0, 0.0)
    );
}

#[test]
fn breadcrumb_coretext_cjk_emoji_and_combining_labels_stay_in_measured_bounds() {
    use tekes_ui_text::traits::Shaper;

    let middle = format!("{}👩‍💻e\u{301}与历史", "项目".repeat(12));
    let current = format!("{}A😀e\u{301}", "当前窗口".repeat(8));
    let labels = ["主页", middle.as_str(), current.as_str()];
    let model = Breadcrumb {
        items: labels.iter().map(|label| (*label).to_owned()).collect(),
    };
    let role = proposal::TextRole::UiBody;
    let palette = Palette::from(Theme::LIGHT);
    for width in [320.0, 80.0, 12.0] {
        let mut text = CoreTextMetrics::new(1.5);
        let mut view = BreadcrumbView::default();
        let measured = view.measure(&mut text, &model, &labels, width, role);
        let parent_clip = Rect::new(10.0, 10.0, width, 38.0);
        let layout = view.render(
            &mut Scene::new(),
            &mut text,
            &model,
            &labels,
            Rect::new(10.0, 10.0, width, 38.0),
            Some(parent_clip),
            palette,
            role,
        );

        assert_eq!(layout.frame.size.width, measured.width);
        assert_eq!(layout.frame.size.height, 38.0_f32.max(measured.height));
        assert!(layout.frame.size.height.is_finite());
        assert_eq!(layout.frame.size.width, width);
        if width == 320.0 {
            assert!(
                measured.height > 38.0,
                "expected expanded CoreText height, got {measured:?}"
            );
            let wrapped = labels.iter().any(|label| {
                let natural_height = text
                    .shaper
                    .shape(
                        label,
                        &tekes_ui_text::style::TextStyle::system(role.style().size * 1.5),
                        &tekes_ui_text::model::ShapeOptions {
                            width: None,
                            line_spacing: 0.0,
                        },
                    )
                    .height as f32;
                text.measured.iter().any(|(value, offered_width, actual)| {
                    value == label && *offered_width > 0.0 && actual.height > natural_height + 0.1
                })
            });
            assert!(wrapped, "the 320pt case should use real CoreText wrapping");
        } else if width > 0.0 {
            assert!(
                measured.height > 0.0,
                "nonzero width should retain a measurable row: {measured:?}"
            );
        }
        assert!(layout.items[0].visible_frame().is_some());
        assert!(
            layout
                .items
                .iter()
                .skip(1)
                .any(|item| item.visible_frame().is_none())
        );
        for item in &layout.items {
            let frame = item.frame();
            assert!(frame.min_x() >= layout.frame.min_x());
            assert!(frame.max_x() <= layout.frame.max_x() + 0.001);
            assert_eq!(item.visible_frame(), frame.intersection(&parent_clip));
            if item.visible_frame().is_none() {
                assert_eq!(view.pointer_down(frame.center()), None);
            }
        }
        // An independently adequate allocation must still paint every label.
        text.drawn.clear();
        let layout = view.render(
            &mut Scene::new(),
            &mut text,
            &model,
            &labels,
            Rect::new(10.0, 10.0, width, measured.height),
            None,
            palette,
            role,
        );
        assert_eq!(layout.frame.size, measured);
        for (label, item) in labels.iter().zip(&layout.items) {
            if width >= 80.0 {
                assert!(
                    text.drawn.iter().any(|(drawn, _)| drawn == label),
                    "label omitted at width {width}: {label}"
                );
            } // At 12pt the original button padding leaves no text content area.
            assert_eq!(item.visible_frame(), Some(item.frame()));
            assert!(item.frame().max_y() <= layout.frame.max_y() + 0.001);
        }
        let slashes = text
            .drawn
            .iter()
            .filter(|(value, _)| value == "/")
            .map(|(_, frame)| *frame)
            .collect::<Vec<_>>();
        if width >= 80.0 {
            assert_eq!(slashes.len(), labels.len() - 1);
        }
        for slash in slashes {
            for item in &layout.items {
                let frame = item.frame();
                assert!(!frame.contains(slash.center()));
                let focus_outset = proposal::metric("spacing", "c-focus-offset").unwrap()
                    + proposal::metric("stroke", "c-focus-width").unwrap();
                let focus = Rect::new(
                    frame.min_x() - focus_outset,
                    frame.min_y() - focus_outset,
                    frame.size.width + 2.0 * focus_outset,
                    frame.size.height + 2.0 * focus_outset,
                );
                assert!(
                    slash.intersection(&focus).is_none(),
                    "separator overlaps a button focus outline"
                );
            }
        }
    }
    let mut text = CoreTextMetrics::new(1.5);
    let mut view = BreadcrumbView::default();
    assert_eq!(
        view.measure(&mut text, &model, &labels, 0.0, role),
        Size::new(0.0, 0.0)
    );
    let layout = view.render(
        &mut Scene::new(),
        &mut text,
        &model,
        &labels,
        Rect::new(10.0, 10.0, 0.0, 38.0),
        None,
        palette,
        role,
    );
    assert_eq!(layout.frame.size, Size::new(0.0, 38.0));
    assert_eq!(view.pointer_down(Point::new(10.0, 10.0)), None);
}

#[test]
fn breadcrumb_rejects_shrunk_or_unpublished_and_invisible_semantic_targets() {
    let p = Palette::from(Theme::LIGHT);
    let mut v = BreadcrumbView::default();
    let mut m = model();
    let mut t = Text::default();
    assert_eq!(v.activate(0, &m), None);
    assert_eq!(m.activate(0), Some(0));
    draw(&mut v, &m, &mut t, p);
    let point = v.layout().items[1].frame().center();
    assert_eq!(v.pointer_down(point), Some(1));

    m.items.truncate(2);
    assert_eq!(v.pointer_up(point, &m), None);
    assert_eq!(v.activate(0, &m), None); // changed path has no published geometry
    draw(&mut v, &m, &mut t, p);
    assert_eq!(v.activate(0, &m), Some(0));
    assert_eq!(v.activate(1, &m), None); // current page remains disabled

    let labels = m.items.iter().map(String::as_str).collect::<Vec<_>>();
    v.render(
        &mut Scene::new(),
        &mut t,
        &m,
        &labels,
        Rect::new(10., 10., 700., 40.),
        Some(Rect::new(900., 900., 5., 5.)),
        p,
        proposal::TextRole::UiBody,
    );
    assert_eq!(v.activate(0, &m), None); // layout exists but has no visible target
}

#[test]
fn command_palette_pointer_move_routes_trigger_hover_and_clears_live_unavailable_state() {
    use tekes_ui_controls::{extended::CommandPalette, layout_view::CommandPaletteView};
    let p = Palette::from(Theme::LIGHT);
    let frame = Rect::new(10., 10., 300., 50.);
    let mut v = CommandPaletteView::default();
    let mut m = CommandPalette::default();
    let mut t = Text::default();
    let draw = |v: &mut CommandPaletteView, m: &mut CommandPalette, t: &mut Text| {
        t.0.clear();
        let mut scene = Scene::new();
        v.render_trigger(
            &mut scene,
            t,
            m,
            "Commands",
            frame,
            None,
            p,
            proposal::TextRole::UiBody,
        );
        scene
    };
    let _scene = draw(&mut v, &mut m, &mut t);
    let point = v.trigger_layout().unwrap().frame.center();
    v.pointer_move(point, &mut m);
    let scene = draw(&mut v, &mut m, &mut t);
    assert_primary_trigger_fill(&scene, v.trigger_layout().unwrap().frame, p, "hover");
    assert!(!m.combo.open); // hover never enters the popup scope

    // Disable, move out, and re-enable without an intervening paint. The
    // disabled update must still remember the pointer exit for the next paint.
    m.combo.disabled = true;
    v.pointer_move(Point::new(900., 900.), &mut m);
    m.combo.disabled = false;
    let scene = draw(&mut v, &mut m, &mut t);
    assert_primary_trigger_fill(&scene, v.trigger_layout().unwrap().frame, p, "rest");

    v.pointer_move(point, &mut m);
    let scene = draw(&mut v, &mut m, &mut t);
    assert_primary_trigger_fill(&scene, v.trigger_layout().unwrap().frame, p, "hover");
    v.set_window_active(false, &mut m);
    v.pointer_move(point, &mut m);
    let scene = draw(&mut v, &mut m, &mut t);
    assert_primary_trigger_fill(&scene, v.trigger_layout().unwrap().frame, p, "rest");
    assert!(!m.combo.open);
}

#[test]
fn select_trigger_tracks_pointer_exit_while_disabled_and_blurs_when_window_deactivates() {
    use tekes_ui_controls::{Select, layout_view::SelectView};
    let p = Palette::from(Theme::LIGHT);
    let frame = Rect::new(10., 10., 300., 50.);
    let outside = Point::new(900., 900.);
    let labels = ["One", "Two"];
    let mut v = SelectView::new(2);
    let mut m = Select::new(2);
    let mut t = Text::default();
    let mut draw = |v: &mut SelectView, m: &mut Select| {
        let mut scene = Scene::new();
        v.render_trigger(
            &mut scene,
            &mut t,
            m,
            &labels,
            frame,
            None,
            p,
            proposal::TextRole::UiBody,
        );
        scene
    };
    let _scene = draw(&mut v, &mut m);
    let point = v.layout().trigger.frame.center();
    v.pointer_move(point, &mut m);
    let scene = draw(&mut v, &mut m);
    assert_primary_trigger_fill(&scene, v.layout().trigger.frame, p, "hover");

    v.set_disabled(true, &mut m);
    v.pointer_move(outside, &mut m);
    v.set_disabled(false, &mut m);
    let scene = draw(&mut v, &mut m);
    assert_primary_trigger_fill(&scene, v.layout().trigger.frame, p, "rest");

    v.pointer_move(point, &mut m);
    let scene = draw(&mut v, &mut m);
    assert_primary_trigger_fill(&scene, v.layout().trigger.frame, p, "hover");
    v.set_window_active(false, &mut m);
    v.pointer_move(outside, &mut m);
    let scene = draw(&mut v, &mut m);
    assert_primary_trigger_fill(&scene, v.layout().trigger.frame, p, "rest");
}

#[test]
fn combobox_candidate_pointer_does_not_survive_disabled_scope_cancellation() {
    use tekes_ui_controls::{extended::ComboBox, layout_view::ComboBoxView};
    let anchor = Rect::new(10., 10., 240., 40.);
    let viewport = Rect::new(0., 0., 600., 500.);
    let outside = Point::new(900., 900.);
    let mut v = ComboBoxView::default();
    let mut m = ComboBox {
        items: vec!["Alpha".into(), "Beta".into()],
        query: String::new(),
        selected: None,
        open: true,
        active: Some(1),
        disabled: false,
    };
    let render = |v: &mut ComboBoxView, m: &mut ComboBox| {
        let mut scene = Scene::new();
        let layout = v
            .render(
                &mut scene,
                &mut Text::default(),
                m,
                anchor,
                viewport,
                None,
                Palette::from(Theme::LIGHT),
                proposal::TextRole::UiBody,
                false,
            )
            .expect("open ComboBox options should be mounted");
        (scene, layout)
    };
    let (baseline, layout) = render(&mut v, &mut m);
    let row = layout.items[1].frame;
    let fill = |scene: &Scene| {
        scene
            .primitives()
            .iter()
            .find_map(|primitive| match primitive {
                Primitive::Quad(quad) if quad.rect == row => Some(quad.fill),
                _ => None,
            })
    };
    let resting_fill = fill(&baseline).expect("candidate button chrome is painted");

    v.pointer_move(row.center(), &mut m);
    let (hovered, _) = render(&mut v, &mut m);
    let hovered_fill = fill(&hovered).expect("hovered candidate chrome is painted");
    assert_ne!(
        hovered_fill, resting_fill,
        "test must observe row hover paint"
    );

    m.disabled = true;
    v.pointer_move(outside, &mut m);
    m.disabled = false;
    let (returned, _) = render(&mut v, &mut m);
    assert_eq!(
        fill(&returned),
        Some(resting_fill),
        "disabled popup cancellation must discard the old pointer location"
    );
}
