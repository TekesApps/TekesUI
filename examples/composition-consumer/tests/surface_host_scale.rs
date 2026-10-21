use tekes_ui_composition_consumer::{IconImages, compound_button, scenarios};
use tekes_ui_controls::{ButtonTextAlign, ButtonTextRenderer};
use tekes_ui_scene::{ImageId, Primitive, Rect, Scene, Size};
use tekes_ui_tokens::{Contrast, Palette, Rgba, Theme, proposal};
struct Text(f32);
impl ButtonTextRenderer for Text {
    fn measure(&mut self, t: &str, r: proposal::TextRole, w: f32) -> Size {
        Size::new((t.len() as f32 * 7.).min(w), r.style().line_height)
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
    fn backing_scale(&self) -> f32 {
        self.0
    }
    fn text_surface_host(&mut self) -> Option<&mut dyn tekes_ui_controls::TextSurfaceHost> {
        Some(self)
    }
}
#[test]
fn actual_compound_and_list_badges_forward_the_public_text_renderer_scale() {
    for theme in Theme::ALL {
        for scale in [1., 2.] {
            for list in [false, true] {
                let palette = Palette::from(theme);
                let mut host = if list {
                    scenarios::heterogeneous_list().unwrap().host
                } else {
                    compound_button(IconImages(vec![(
                        palette.color("primary-text").unwrap(),
                        ImageId(1),
                    )]))
                    .unwrap()
                };
                let mut scene = Scene::new();
                host.render(
                    &mut scene,
                    &mut Text(scale),
                    Rect::new(8., 8., 600., 600.),
                    Rect::new(0., 0., 640., 640.),
                    palette,
                )
                .unwrap();
                let widths = scene
                    .primitives()
                    .iter()
                    .filter_map(|p| match p {
                        Primitive::Quad(q)
                            if q.fill == palette.color("surface-panel").unwrap()
                                && q.border_color == palette.color("separator").unwrap()
                                && q.border_width > 0. =>
                        {
                            Some(q.border_width)
                        }
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                assert!(!widths.is_empty());
                let expected = if theme.contrast == Contrast::Increased {
                    1.
                } else {
                    1. / scale
                };
                assert!(
                    widths.iter().all(|w| *w == expected),
                    "actual consumer list={list},scale={scale},widths={widths:?}"
                );
            }
        }
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
