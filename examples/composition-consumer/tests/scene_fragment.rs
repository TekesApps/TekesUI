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

struct ClockText(u64);
impl ButtonTextRenderer for ClockText {
    fn measure(&mut self, s: &str, r: proposal::TextRole, w: f32) -> Size {
        Text.measure(s, r, w)
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
    fn motion_context(&self) -> Option<tekes_ui_controls::ControlMotionContext> {
        Some(tekes_ui_controls::ControlMotionContext {
            now: std::time::Duration::from_millis(self.0),
            reduced_motion: false,
        })
    }
}
#[test]
fn retained_exit_replays_translated_immutable_keys_with_original_source() {
    let (table, key, shape) = source();
    let mut original = Scene::with_clip_shapes(&table);
    original.push_shape_clip(key.clone());
    original.push_keyed_quad(
        Quad::new(shape.rect(), Rgba::gray(1.)).with_corner_radius(8.),
        key,
    );
    original.pop_clip();
    let mut translated = Scene::new();
    translated
        .try_append_fragment(
            &original,
            Default::default(),
            FragmentTransform {
                translation: Point::new(0.25, 0.5),
                ..Default::default()
            },
        )
        .unwrap();
    let expected =
        ClipShape::normalize(shape.rect(), shape.radii(), Point::new(0.25, 0.5)).unwrap();
    let mut model = Popover::default();
    let mut view = PopoverView::default();
    view.show(&mut model, 1);
    let mut samples = Vec::new();
    for (millis, open) in [(0, true), (1000, true), (1000, false), (1050, false)] {
        model.open = open;
        let mut scene = Scene::with_clip_shapes(&table);
        view.try_render_with_text_content(
            &mut scene,
            &mut ClockText(millis),
            &model,
            Rect::new(20., 20., 30., 20.),
            Size::new(140., 100.),
            Rect::new(0., 0., 500., 400.),
            None,
            Theme::LIGHT.into(),
            proposal::TextRole::UiBody,
            "translated-retained",
            |scene, _, _| {
                scene
                    .try_append_fragment(&translated, Default::default(), Default::default())
                    .unwrap();
            },
        )
        .unwrap();
        if millis > 0 {
            let (quad, key) = scene
                .primitives()
                .iter()
                .find_map(|p| match p {
                    Primitive::KeyedQuad { quad, outer } => Some((quad, outer)),
                    _ => None,
                })
                .expect("nonempty retained translated keyed paint");
            assert_eq!(scene.clip_shape(key), Ok(expected));
            assert_eq!(quad.rect, expected.rect());
            samples.push((open, quad.fill.a));
        }
    }
    assert_eq!(samples[0].1, 1.);
    assert_eq!(samples[1].1, 1.);
    assert!(
        samples[2].1 > 0. && samples[2].1 < 1.,
        "exit alpha should animate without dropping translated keys: {samples:?}"
    );
}
#[test]
fn nested_half_ulp_translation_remains_aligned_during_original_retained_exit() {
    let original_shape = ClipShape::normalize(
        Rect::new(1_000_000., 0., 8., 8.),
        CornerRadii::uniform(2.),
        Point::new(0., 0.),
    )
    .unwrap();
    let mut table = ShapeTable::new(77, 4);
    let original_key = table.intern(original_shape).unwrap();
    let marker = Rgba::new(0.123, 0.456, 0.789, 1.);
    let mut fragment = Scene::with_clip_shapes(&table);
    fragment.push_shape_clip(original_key.clone());
    fragment.push_keyed_quad(
        Quad::new(original_shape.rect(), Rgba::gray(1.)).with_corner_radius(2.),
        original_key.clone(),
    );
    fragment.push_quad(Quad::new(original_shape.rect(), marker));
    fragment.pop_clip();
    for delta in [
        Point::new(0.03125, 0.),
        Point::new(0.03125, 0.),
        Point::new(-999_990., 10.),
    ] {
        let mut next = Scene::new();
        next.try_append_fragment(
            &fragment,
            Default::default(),
            FragmentTransform {
                translation: delta,
                ..Default::default()
            },
        )
        .unwrap();
        fragment = next;
    }
    let expected = ClipShape::normalize(
        Rect::new(10., 10., 8., 8.),
        CornerRadii::uniform(2.),
        Point::new(0., 0.),
    )
    .unwrap();
    let mut model = Popover::default();
    let mut view = PopoverView::default();
    view.show(&mut model, 1);
    for (millis, open) in [(0, true), (1000, true), (1000, false), (1050, false)] {
        model.open = open;
        let mut scene = Scene::new();
        view.try_render_with_text_content(
            &mut scene,
            &mut ClockText(millis),
            &model,
            Rect::new(20., 20., 30., 20.),
            Size::new(140., 100.),
            Rect::new(0., 0., 500., 400.),
            None,
            Theme::LIGHT.into(),
            proposal::TextRole::UiBody,
            "nested-rounding-retained",
            |scene, _, _| {
                scene
                    .try_append_fragment(&fragment, Default::default(), Default::default())
                    .unwrap();
            },
        )
        .unwrap();
        if millis > 0 {
            let (keyed, key) = scene
                .primitives()
                .iter()
                .find_map(|p| match p {
                    Primitive::KeyedQuad { quad, outer } => Some((quad, outer)),
                    _ => None,
                })
                .unwrap();
            let ordinary = scene
                .primitives()
                .iter()
                .find_map(|p| match p {
                    Primitive::Quad(quad)
                        if quad.fill.r == marker.r
                            && quad.fill.g == marker.g
                            && quad.fill.b == marker.b =>
                    {
                        Some(quad)
                    }
                    _ => None,
                })
                .unwrap();
            assert_eq!(scene.clip_shape(key), Ok(expected));
            assert_eq!(keyed.rect, expected.rect());
            assert_eq!(ordinary.rect, expected.rect());
            assert!(keyed.fill.a > 0.);
        }
    }
    assert_eq!(table.get(original_key), Ok(original_shape));
}
