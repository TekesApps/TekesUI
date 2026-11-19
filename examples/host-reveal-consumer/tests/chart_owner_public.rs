use tekes_ui_controls::{
    BarChart, ButtonTextAlign, ButtonTextRenderer, ChartDatum,
    collections_view::{ChartOwnerCapacity, ChartOwnerFocusSpace, ChartView},
};
use tekes_ui_scene::{Rect, Scene, Size};
use tekes_ui_tokens::{Rgba, Theme, proposal::TextRole};
struct Text;
impl ButtonTextRenderer for Text {
    fn measure(&mut self, s: &str, _: TextRole, w: f32) -> Size {
        Size::new((s.len() as f32 * 7.).min(w), 18.)
    }
    fn draw(&mut self, _: &mut Scene, _: &str, _: TextRole, _: Rect, _: ButtonTextAlign, _: Rgba) {}
}
#[test]
fn second_public_consumer_can_measure_arrange_query_and_admit_without_host_access() {
    let mut chart = BarChart::new(vec![
        ChartDatum::new("same", -1.),
        ChartDatum::new("same", 0.),
    ]);
    let mut view = ChartView::new();
    let mut text = Text;
    assert_eq!(
        view.measure_owner(&mut text, &chart, 300., 180.).capacity,
        ChartOwnerCapacity::Sufficient
    );
    assert_eq!(
        view.owner_focus_space(&chart),
        ChartOwnerFocusSpace::Unmeasured
    );
    assert_eq!(
        view.arrange_owner(&mut text, &mut chart, Rect::new(0., 0., 300., 180.), None),
        ChartOwnerCapacity::Sufficient
    );
    assert!(matches!(
        view.owner_focus_space(&chart),
        ChartOwnerFocusSpace::Visible { .. }
    ));
    view.try_focus_owner(true, &mut chart).unwrap();
    let selection = chart.selected();
    let mut scene = Scene::new();
    assert!(matches!(
        view.paint_owner(&mut scene, &mut text, &mut chart, Theme::LIGHT.into()),
        ChartOwnerFocusSpace::Visible { .. }
    ));
    assert!(!scene.primitives().is_empty());
    assert_eq!(
        view.arrange_owner(&mut text, &mut chart, Rect::new(0., 0., 4., 4.), None),
        ChartOwnerCapacity::InsufficientAllocation
    );
    let before = chart.clone();
    assert!(view.try_focus_owner(true, &mut chart).is_err());
    assert_eq!(chart, before);
    assert!(chart.is_focused());
    assert_eq!(chart.selected(), selection);
    view.invalidate_owner_layout();
    assert_eq!(
        view.owner_focus_space(&chart),
        ChartOwnerFocusSpace::Unmeasured
    );
}
