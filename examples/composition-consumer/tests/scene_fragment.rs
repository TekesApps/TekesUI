use tekes_ui_controls::layout::MeasuredChild;
// Independent public consumer. No private Scene, control or renderer state access.
use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, DialogView, composition::*, extended::Popover,
    feedback_view::PopoverView,
};
use tekes_ui_scene::*;
use tekes_ui_tokens::{Rgba, Theme, proposal};
struct Text;
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
    }
}
fn source() -> (ShapeTable, ShapeKey, ClipShape) {
    let shape = ClipShape::normalize(
        Rect::new(10., 10., 40., 40.),
        CornerRadii::uniform(8.),
        Point::new(0., 0.),
    )
    .unwrap();
    let mut table = ShapeTable::new(7, 4);
    let key = table.intern(shape).unwrap();
    (table, key, shape)
}
fn popup(
    view: &mut PopoverView,
    caller: &mut Scene,
    model: &Popover,
    key: ShapeKey,
    shape: ClipShape,
) -> Result<tekes_ui_controls::feedback_view::OverlayLayout, FragmentError> {
    view.try_render_with_text_content(
        caller,
        &mut Text,
        model,
        Rect::new(20., 20., 30., 20.),
        Size::new(140., 100.),
        Rect::new(0., 0., 500., 400.),
        None,
        Theme::LIGHT.into(),
        proposal::TextRole::UiBody,
        "public-fragment",
        |scene, _, _| {
            scene.push_shape_clip(key.clone());
            scene.push_keyed_quad(
                Quad::new(shape.rect(), Rgba::gray(1.)).with_corner_radius(8.),
                key,
            );
            scene.pop_clip();
        },
    )
}
#[test]
fn popover_temporary_scene_inherits_caller_inventory_and_preserves_prefix() {
    let (table, key, shape) = source();
    let mut caller = Scene::with_clip_shapes(&table);
    caller.push_clip(Rect::new(0., 0., 500., 400.));
    caller.push_quad(Quad::new(Rect::new(0., 0., 4., 4.), Rgba::gray(1.)));
    let prefix = caller.primitives().to_vec();
    let mut model = Popover::default();
    let mut view = PopoverView::default();
    view.show(&mut model, 1);
    popup(&mut view, &mut caller, &model, key.clone(), shape).unwrap();
    assert_eq!(&caller.primitives()[..prefix.len()], prefix);
    assert_eq!(caller.clip_shape(&key), Ok(shape));
    assert!(
        caller
            .primitives()
            .iter()
            .any(|p| matches!(p,Primitive::KeyedQuad{outer,..}if outer==&key))
    );
    caller.pop_clip();
}
#[test]
fn popover_try_path_is_atomic_and_legacy_entry_sets_non_submittable_error() {
    let (_, key, shape) = source();
    let mut model = Popover::default();
    let mut view = PopoverView::default();
    view.show(&mut model, 1);
    let mut caller = Scene::new();
    caller.push_quad(Quad::new(Rect::new(0., 0., 4., 4.), Rgba::gray(1.)));
    let before = caller.clone();
    assert!(popup(&mut view, &mut caller, &model, key.clone(), shape).is_err());
    assert_eq!(caller, before);
    view.render_with_text_content(
        &mut caller,
        &mut Text,
        &model,
        Rect::new(20., 20., 30., 20.),
        Size::new(140., 100.),
        Rect::new(0., 0., 500., 400.),
        None,
        Theme::LIGHT.into(),
        proposal::TextRole::UiBody,
        "legacy",
        |scene, _, _| {
            scene.push_shape_clip(key);
            scene.pop_clip();
        },
    );
    assert_eq!(caller.primitives(), before.primitives());
    assert!(caller.composition_error().is_some());
}
#[test]
fn public_dialog_try_entry_preserves_existing_shape_inventory() {
    let (table, key, shape) = source();
    let mut caller = Scene::with_clip_shapes(&table);
    let mut dialog = DialogView::default();
    dialog.show(1);
    assert!(
        dialog
            .try_render(
                &mut caller,
                &mut Text,
                "Public",
                Rect::new(0., 0., 600., 400.),
                None,
                Theme::LIGHT.into(),
                proposal::TextRole::UiBody,
                false
            )
            .unwrap()
            .is_some()
    );
    assert_eq!(caller.clip_shape(&key), Ok(shape));
    assert_eq!(caller.composition_error(), None);
}
#[derive(Debug)]
struct RejectedPaint;
impl Component for RejectedPaint {
    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, _: Size) -> MeasuredChild {
        MeasuredChild::new(Size::new(40., 40.))
    }
    fn paint(
        &mut self,
        scene: &mut Scene,
        _: &mut dyn ButtonTextRenderer,
        _: Rect,
        _: Rect,
        _: ContentTheme,
    ) {
        scene.record_composition_error(FragmentError::BudgetExceeded);
    }
}
#[test]
fn original_host_reports_scene_composition_failure_explicitly() {
    let mut host = ContentHost::new(Content::new(ComponentId(1), RejectedPaint)).unwrap();
    let mut scene = Scene::new();
    let frame = Rect::new(0., 0., 100., 100.);
    assert_eq!(
        host.render(&mut scene, &mut Text, frame, frame, Theme::LIGHT.into()),
        Err(CompositionError::SceneFragment(
            FragmentError::BudgetExceeded
        ))
    );
    assert_eq!(
        scene.composition_error(),
        Some(FragmentError::BudgetExceeded)
    );
}
