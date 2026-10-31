//! Original Link model/view, explicit parent identity and unchanged original actions.
use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, ControlKey, ControlState,
    extended::{Link, link_underline_on_surface},
    layout_view::LinkView,
};
use tekes_ui_scene::{Primitive, Rect, Scene, Size};
use tekes_ui_tokens::{
    Palette, Rgba, Theme, proposal,
    surface_palette::{resolve_surface, surface_names},
};
#[derive(Default)]
struct Text {
    inks: Vec<Rgba>,
}
impl ButtonTextRenderer for Text {
    fn measure(&mut self, t: &str, _: proposal::TextRole, w: f32) -> Size {
        Size::new((t.len() as f32 * 7.).min(w), 20.)
    }
    fn draw(
        &mut self,
        _: &mut Scene,
        _: &str,
        _: proposal::TextRole,
        _: Rect,
        _: ButtonTextAlign,
        c: Rgba,
    ) {
        self.inks.push(c);
    }
}
#[test]
fn original_view_and_painter_resolve_one_complete_packet_on_every_parent() {
    for theme in Theme::ALL {
        for brand in [0, 1, 2] {
            for surface in surface_names() {
                for phase in ["rest", "hover", "pressed", "disabled", "focus"] {
                    let p = Palette::new(theme, brand).unwrap();
                    let f = Rect::new(10., 12., 180., 32.);
                    let clip = Rect::new(0., 0., 220., 80.);
                    let mut model = Link::default();
                    let mut view = LinkView::default();
                    view.set_surface(surface).unwrap();
                    let mut text = Text::default();
                    view.render(
                        &mut Scene::new(),
                        &mut text,
                        &mut model,
                        "Read 阅读",
                        f,
                        Some(clip),
                        p,
                        proposal::TextRole::UiBody,
                    );
                    if matches!(phase, "hover" | "pressed") {
                        view.pointer_move(f.center());
                    }
                    if phase == "pressed" {
                        assert!(view.pointer_down(f.center()));
                    }
                    if phase == "focus" {
                        view.focus(true);
                    }
                    model.button.state.disabled = phase == "disabled";
                    text.inks.clear();
                    let mut s = Scene::new();
                    view.render(
                        &mut s,
                        &mut text,
                        &mut model,
                        "Read 阅读",
                        f,
                        Some(clip),
                        p,
                        proposal::TextRole::UiBody,
                    );
                    let packet = resolve_surface(
                        p,
                        surface,
                        "link",
                        if phase == "focus" { "rest" } else { phase },
                    )
                    .unwrap();
                    assert_eq!(text.inks, [packet.text]);
                    assert!(s.primitives().iter().any(
                        |q| matches!(q,Primitive::Quad(q)if q.rect==f&&q.fill==packet.background)
                    ));
                    assert!(s.primitives().iter().any(|q|matches!(q,Primitive::Quad(q)if q.fill==packet.text&&q.rect.max_y()<f.max_y()&&q.rect.min_x()>f.min_x())));
                    if phase == "focus" {
                        assert!(s.primitives().iter().any(|q|matches!(q,Primitive::Quad(q)if Some(q.border_color)==packet.focus&&q.border_width>0.)));
                    }
                    let mut direct = Scene::new();
                    let ink =
                        link_underline_on_surface(&mut direct, f, p, model.button.state, surface)
                            .unwrap();
                    assert_eq!(ink, packet.text);
                }
            }
        }
    }
}
#[test]
fn surface_and_palette_edits_preserve_original_capture_and_invalid_surface_cannot_mutate() {
    let mut model = Link::default();
    let mut view = LinkView::default();
    let mut text = Text::default();
    let f = Rect::new(8., 8., 220., 40.);
    let p = Palette::from(Theme::LIGHT);
    view.render(
        &mut Scene::new(),
        &mut text,
        &mut model,
        "Read",
        f,
        Some(f),
        p,
        proposal::TextRole::UiBody,
    );
    assert!(view.pointer_down(f.center()));
    view.set_surface("panel").unwrap();
    view.render(
        &mut Scene::new(),
        &mut text,
        &mut model,
        "Read",
        f,
        Some(f),
        Palette::new(Theme::DARK, 2).unwrap(),
        proposal::TextRole::UiBody,
    );
    assert!(view.set_surface("missing-parent").is_err());
    assert!(view.pointer_up(f.center(), &mut model));
    assert!(!view.pointer_up(f.center(), &mut model));
    let mut text = Text::default();
    view.render(
        &mut Scene::new(),
        &mut text,
        &mut model,
        "Read",
        f,
        Some(f),
        p,
        proposal::TextRole::UiBody,
    );
    assert_eq!(
        text.inks,
        [resolve_surface(p, "panel", "link", "hover").unwrap().text]
    );
    view.focus(true);
    assert!(!view.key(ControlKey::Space, true, &mut model).consumed);
    assert!(view.key(ControlKey::Enter, true, &mut model).changed);
    assert!(view.pointer_down(f.center()));
    view.cancel();
    assert!(!view.pointer_up(f.center(), &mut model));
    assert!(view.pointer_down(f.center()));
    model.button.state.disabled = true;
    assert!(!view.pointer_up(f.center(), &mut model));
    model.button.state.disabled = false;
    view.set_window_active(false);
    assert!(!view.activate(&mut model));
}
#[test]
fn invalid_parent_packet_fails_before_any_paint() {
    let mut s = Scene::new();
    assert!(
        link_underline_on_surface(
            &mut s,
            Rect::new(0., 0., 100., 32.),
            Theme::LIGHT.into(),
            ControlState {
                focused: true,
                focus_visible: true,
                ..Default::default()
            },
            "missing-parent"
        )
        .is_err()
    );
    assert!(s.is_empty());
}

#[test]
fn underline_stays_inside_original_ground_and_never_crosses_the_offset_focus_ring() {
    for (w, h) in [(200., 44.), (80., 100.), (8., 8.), (0., 0.)] {
        let frame = Rect::new(10., 10., w, h);
        let p = Palette::from(Theme::LIGHT);
        let packet = resolve_surface(p, "canvas", "link", "rest").unwrap();
        let mut scene = Scene::new();
        link_underline_on_surface(
            &mut scene,
            frame,
            p,
            ControlState {
                focused: true,
                focus_visible: true,
                ..Default::default()
            },
            "canvas",
        )
        .unwrap();
        let line = scene
            .primitives()
            .iter()
            .rev()
            .find_map(|q| match q {
                Primitive::Quad(q) if q.fill == packet.text => Some(q.rect),
                _ => None,
            })
            .unwrap();
        assert!(line.min_x() >= frame.min_x() && line.max_x() <= frame.max_x());
        assert!(line.min_y() >= frame.min_y() && line.max_y() <= frame.max_y());
        assert!(line.size.width >= 0. && line.size.height >= 0.);
        if w >= 80. {
            assert!(line.max_y() < frame.max_y());
            assert!(line.min_x() > frame.min_x());
        }
    }
}
