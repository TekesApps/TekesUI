use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, extended::Popover, feedback_view::PopoverView,
};
use tekes_ui_scene::{Rect, Scene, Size};
use tekes_ui_tokens::{Rgba, Theme, proposal};
#[derive(Default)]
struct Text {
    draws: usize,
}
impl ButtonTextRenderer for Text {
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
        self.draws += 1;
    }
}
#[test]
fn public_popover_arrange_matches_render_without_drawing_or_reviving_closed_focus() {
    for theme in Theme::ALL {
        for width in [220., 640.] {
            let mut view = PopoverView::default();
            let mut model = Popover::default();
            let mut text = Text::default();
            let frame = Rect::new(0., 0., width, 480.);
            let anchor = Rect::new(20., 20., 140., 40.);
            let size =
                PopoverView::measure(&mut text, Size::new(160., 100.), proposal::TextRole::UiBody);
            view.show(&mut model, 11);
            let arranged = view.arrange(
                &mut text,
                &model,
                anchor,
                size,
                frame,
                Some(frame),
                proposal::TextRole::UiBody,
            );
            assert_eq!(text.draws, 0);
            assert!(!view.is_presented(), "layout does not sample/open presence");
            let mut callback_bounds = None;
            let painted = view.render_with_content(
                &mut Scene::new(),
                &mut text,
                &model,
                anchor,
                size,
                frame,
                Some(frame),
                theme.into(),
                proposal::TextRole::UiBody,
                |_, bounds| callback_bounds = Some(bounds),
            );
            assert_eq!(arranged, painted);
            assert_eq!(callback_bounds, Some(arranged.content));
            assert!(text.draws > 0);
            assert!(view.logical_focus_target().is_some());
            view.close(&mut model);
            view.render(
                &mut Scene::new(),
                &mut text,
                &model,
                anchor,
                size,
                frame,
                Some(frame),
                theme.into(),
                proposal::TextRole::UiBody,
            );
            assert!(
                view.logical_focus_target().is_none(),
                "exit paint must not publish dismiss focus"
            );
            view.arrange(
                &mut text,
                &model,
                anchor,
                size,
                frame,
                Some(frame),
                proposal::TextRole::UiBody,
            );
            assert!(
                view.logical_focus_target().is_none(),
                "closing presentation cannot resurrect the dismiss focus target"
            );
        }
    }
}
