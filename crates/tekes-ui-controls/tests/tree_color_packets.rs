//! Legacy and mounted Tree share the formal Quiet Surface packet contract.
use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, collections_view::TreeView, extended::Tree,
    layout_view::IconRenderer,
};
use tekes_ui_scene::{Rect, Scene, Size};
use tekes_ui_tokens::{Rgba, Theme, proposal, surface_palette::resolve_surface};
#[derive(Default)]
struct Text(Vec<(String, Rgba)>);
impl ButtonTextRenderer for Text {
    fn measure(&mut self, s: &str, _: proposal::TextRole, w: f32) -> Size {
        Size::new((s.len() as f32 * 8.).min(w), 20.)
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
struct Icons(Vec<Rgba>);
impl IconRenderer for Icons {
    fn draw_icon(&mut self, _: &mut Scene, _: &str, _: Rect, c: Rgba) {
        self.0.push(c);
    }
}
#[test]
fn original_tree_string_text_and_disclosure_consume_formal_quiet_packets() {
    for theme in Theme::ALL {
        for (selected, disabled, state) in [
            (false, false, "rest"),
            (true, false, "selected"),
            (false, true, "disabled"),
            (true, true, "selected-disabled"),
        ] {
            let mut model = Tree::default();
            model.add("root", None);
            model.add("child", Some(0));
            model.toggle(0);
            if selected {
                model.select(0);
            }
            let mut view = TreeView::new();
            view.set_disabled(disabled);
            let mut text = Text::default();
            let mut scene = Scene::new();
            let frame = Rect::new(0., 0., 320., 160.);
            let palette = theme.into();
            view.render_model(
                &mut scene,
                &mut text,
                &mut model,
                frame,
                Some(frame),
                palette,
            );
            let mut icons = Icons::default();
            view.render_icons_model(
                &mut scene,
                &mut icons,
                &mut model,
                Some(frame),
                palette,
                16.,
            );
            let expected = resolve_surface(palette, "canvas", "action-quiet", state).unwrap();
            assert_eq!(
                text.0.iter().find(|(s, _)| s == "root").unwrap().1,
                expected.text,
                "{theme:?}/{state} label bypassed formal Quiet packet"
            );
            assert_eq!(
                icons.0[0], expected.icon,
                "{theme:?}/{state} disclosure bypassed formal Quiet packet"
            );
        }
    }
}
