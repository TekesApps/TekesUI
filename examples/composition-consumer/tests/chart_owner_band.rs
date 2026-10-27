#![cfg(target_os = "macos")]
#[path = "support/coretext.rs"]
mod coretext;
use tekes_ui_controls::{
    BarChart, ChartDatum, ControlKey,
    collections_view::{ChartOwnerCapacity as Capacity, ChartOwnerFocusSpace as Space, ChartView},
    extended::{ControlFrame, FocusRingMetrics},
};
use tekes_ui_scene::{Primitive, Rect, Scene};
use tekes_ui_tokens::{Palette, Theme};
fn model(value: f64, label: &str) -> BarChart {
    let mut m = BarChart::new((0..64).map(|_| ChartDatum::new(label, value)).collect());
    m.select(2);
    m
}
fn contains(a: Rect, b: Rect) -> bool {
    b.min_x() >= a.min_x()
        && b.max_x() <= a.max_x()
        && b.min_y() >= a.min_y()
        && b.max_y() <= a.max_y()
}
fn witness(space: Space) -> Rect {
    let Space::Visible { witness } = space else {
        panic!("missing witness: {space:?}")
    };
    witness
}
#[test]
fn old_narrow_has_persistent_band_and_same_body_hit_geometry() {
    let mut t = coretext::Text::new(1.);
    t.backing = 1.;
    let mut m = model(1., "");
    let data = m.data().to_vec();
    let mut v = ChartView::new();
    let a = Rect::new(16., 16., 120., 240.);
    let measure = v.measure_owner(&mut t, &m, a.size.width, a.size.height);
    assert_eq!(measure.capacity, Capacity::Sufficient);
    assert_eq!(v.owner_focus_space(&m), Space::Unmeasured);
    assert_eq!(
        v.arrange_owner(&mut t, &mut m, a, None),
        Capacity::Sufficient
    );
    let layout = v.layout().clone();
    assert_eq!(
        layout.frame,
        a.inset(FocusRingMetrics::new(1.).reserved_band())
    );
    assert_eq!(layout.geometry.bars.len(), 64);
    assert_eq!(layout.geometry.bars[2].rect.size.width, 0.);
    let e = witness(v.owner_focus_space(&m));
    assert!(contains(a, e));
    let mut nonfocus = Scene::new();
    assert_eq!(
        v.paint_owner(&mut nonfocus, &mut t, &mut m, Palette::from(Theme::ALL[0])),
        Space::Visible { witness: e }
    );
    v.try_focus_owner(true, &mut m).unwrap();
    let mut focus = Scene::new();
    v.paint_owner(&mut focus, &mut t, &mut m, Palette::from(Theme::ALL[0]));
    assert_eq!(v.layout().frame, layout.frame);
    assert_eq!(v.layout().geometry.plot, layout.geometry.plot);
    assert!(
        focus
            .primitives()
            .iter()
            .any(|p| matches!(p,Primitive::Quad(q) if q.border_width==2. && q.rect==a.inset(1.)))
    );
    let pt = v.layout().geometry.bars[5].rect.center();
    let event = v.pointer_down(pt, &mut m);
    assert_ne!(format!("{event:?}"), "Ignored");
    assert!(m.is_focused());
    assert_eq!(m.data(), data);
}
#[test]
fn long_labels_report_actual_budget_then_real_parent_can_supply_it() {
    let mut t = coretext::Text::new(1.);
    t.backing = 1.;
    let label = "Long duplicate category label with original complete text";
    let mut m = model(1., label);
    let mut v = ChartView::new();
    let a = Rect::new(16., 16., 480., 240.);
    let measured = v.measure_owner(&mut t, &m, 480., 240.);
    assert_eq!(measured.capacity, Capacity::InsufficientBody);
    let extent = measured.measured_label_extent.unwrap();
    assert!(extent.height > 240.);
    assert_eq!(
        v.arrange_owner(&mut t, &mut m, a, None),
        Capacity::InsufficientBody
    );
    assert!(matches!(
        v.owner_focus_space(&m),
        Space::Insufficient(Capacity::InsufficientBody)
    ));
    assert!(!m.is_focused());
    let before = m.clone();
    assert!(v.try_focus_owner(true, &mut m).is_err());
    assert_eq!(m, before);
    let real = Rect::new(16., 16., 480., extent.height + 240.);
    assert_eq!(
        v.arrange_owner(&mut t, &mut m, real, None),
        Capacity::Sufficient
    );
    witness(v.owner_focus_space(&m));
    assert_eq!(m.data().len(), 64);
    assert!(m.data().iter().all(|d| d.label == label));
}
#[test]
fn tiny_refuses_new_focus_but_retains_existing_owner_and_selection() {
    let mut t = coretext::Text::new(1.);
    let mut m = model(-1., "");
    let mut v = ChartView::new();
    v.arrange_owner(&mut t, &mut m, Rect::new(16., 16., 480., 240.), None);
    v.try_focus_owner(true, &mut m).unwrap();
    m.select(3);
    assert_eq!(
        v.arrange_owner(&mut t, &mut m, Rect::new(16., 16., 4., 4.), None),
        Capacity::InsufficientAllocation
    );
    let before = m.clone();
    let space = v.owner_focus_space(&m);
    assert_eq!(space, Space::Insufficient(Capacity::InsufficientAllocation));
    let mut scene = Scene::new();
    assert_eq!(
        v.paint_owner(&mut scene, &mut t, &mut m, Theme::ALL[0].into()),
        space
    );
    assert!(scene.primitives().is_empty());
    assert_eq!(m, before);
    assert!(m.is_focused() && m.focus_visible());
    assert_eq!(m.selected(), Some(3));
    let mut fresh = ChartView::new();
    let mut other = model(1., "");
    fresh.arrange_owner(&mut t, &mut other, Rect::new(16., 16., 4., 4.), None);
    let before = other.clone();
    assert!(fresh.try_focus_owner(true, &mut other).is_err());
    fresh.pointer_down(Rect::new(16., 16., 4., 4.).center(), &mut other);
    assert_eq!(other, before);
}
#[test]
fn real_edges_only_interior_corner_and_excluded_are_distinct() {
    let mut t = coretext::Text::new(1.);
    t.backing = 1.;
    let mut m = model(-1., "");
    let mut v = ChartView::new();
    let a = Rect::new(21., 21., 470., 240.);
    let clip = Rect::new(80., 80., 320., 120.);
    v.arrange_owner(&mut t, &mut m, a, Some(clip));
    assert_eq!(v.owner_focus_space(&m), Space::NoVisibleWitness);
    let before = m.clone();
    assert!(v.try_focus_owner(true, &mut m).is_err());
    assert_eq!(m, before);
    // Parent-assigned translation only: this does not execute/claim Host reveal.
    v.arrange_owner(&mut t, &mut m, Rect::new(21., -40., 470., 240.), Some(clip));
    let e = witness(v.owner_focus_space(&m));
    assert!(contains(clip, e));
    assert!(e.min_y() >= 196. && e.max_y() <= 200.);
    v.arrange_owner(&mut t, &mut m, a, Some(Rect::new(21., 21., 3., 3.)));
    assert_eq!(v.owner_focus_space(&m), Space::NoVisibleWitness);
    v.arrange_owner(&mut t, &mut m, a, Some(Rect::new(800., 800., 20., 20.)));
    assert_eq!(v.owner_focus_space(&m), Space::Excluded);
}
#[test]
fn zero_blank_duplicate_and_same_owner_keyboard_stay_original() {
    let mut t = coretext::Text::new(1.);
    let mut m = model(0., " ");
    let mut v = ChartView::new();
    v.arrange_owner(&mut t, &mut m, Rect::new(16., 16., 480., 240.), None);
    v.try_focus_owner(true, &mut m).unwrap();
    assert!(
        v.layout()
            .geometry
            .bars
            .iter()
            .all(|b| b.rect.size.height == 0.)
    );
    let before = m.data().to_vec();
    v.key(ControlKey::End, true, &mut m);
    assert_eq!(m.selected(), Some(63));
    assert_eq!(m.data(), before);
    let mut s = Scene::new();
    assert!(matches!(
        v.paint_owner(&mut s, &mut t, &mut m, Theme::ALL[0].into()),
        Space::Visible { .. }
    ));
}
#[test]
fn stale_model_and_explicit_invalidation_do_not_publish_old_hit() {
    let mut t = coretext::Text::new(1.);
    let mut m = model(1., "");
    let mut v = ChartView::new();
    v.arrange_owner(&mut t, &mut m, Rect::new(16., 16., 480., 240.), None);
    let pt = v.layout().geometry.bars[2].rect.center();
    let mut replacement = model(-1., "changed");
    assert_eq!(v.owner_focus_space(&replacement), Space::Unmeasured);
    let before = replacement.clone();
    assert_eq!(
        format!("{:?}", v.pointer_down(pt, &mut replacement)),
        "Ignored"
    );
    assert_eq!(replacement, before);
    v.try_focus_owner(true, &mut m).unwrap();
    v.invalidate_owner_layout();
    assert_eq!(v.owner_focus_space(&m), Space::Unmeasured);
    assert!(v.layout().visible.is_none());
    assert!(m.is_focused());
}
#[test]
fn shared_metrics_preserve_original_control_frame_contract() {
    let r = Rect::new(10., 20., 80., 40.);
    for scale in [1., 2., 1.5] {
        let m = FocusRingMetrics::new(scale);
        assert_eq!(m.width, 2.);
        assert_eq!(m.gap, 2.);
        assert_eq!(m.bounds(r), ControlFrame::new(r, None).focus_bounds(scale));
        assert_eq!(m.antialias_guard, 1. / scale);
    }
}
#[derive(Debug)]
struct FixedText;
impl tekes_ui_controls::ButtonTextRenderer for FixedText {
    fn measure(
        &mut self,
        _: &str,
        _: tekes_ui_tokens::proposal::TextRole,
        w: f32,
    ) -> tekes_ui_scene::Size {
        tekes_ui_scene::Size::new(w, 16.)
    }
    fn draw(
        &mut self,
        _: &mut Scene,
        _: &str,
        _: tekes_ui_tokens::proposal::TextRole,
        _: Rect,
        _: tekes_ui_controls::ButtonTextAlign,
        _: tekes_ui_tokens::Rgba,
    ) {
    }
}
#[test]
fn no_new_physical_pixel_plot_floor_is_imposed() {
    let mut t = FixedText;
    let mut m = model(1., "");
    let mut v = ChartView::new();
    let h = v
        .measure_owner(&mut t, &m, 480., 240.)
        .measured_label_extent
        .unwrap()
        .height;
    assert_eq!(
        v.arrange_owner(&mut t, &mut m, Rect::new(0., 0., 480., h), None),
        Capacity::InsufficientBody
    );
    assert_eq!(
        v.arrange_owner(&mut t, &mut m, Rect::new(0., 0., 480., h.next_up()), None),
        Capacity::Sufficient
    );
    assert!(v.layout().geometry.plot.size.height > 0. && v.layout().geometry.plot.size.height < 1.);
}
#[test]
fn failed_measurement_and_backing_change_are_not_visible_certificates() {
    #[derive(Debug)]
    struct Unknown;
    impl tekes_ui_controls::ButtonTextRenderer for Unknown {
        fn measure(
            &mut self,
            _: &str,
            _: tekes_ui_tokens::proposal::TextRole,
            _: f32,
        ) -> tekes_ui_scene::Size {
            tekes_ui_scene::Size::new(1., f32::NAN)
        }
        fn draw(
            &mut self,
            _: &mut Scene,
            _: &str,
            _: tekes_ui_tokens::proposal::TextRole,
            _: Rect,
            _: tekes_ui_controls::ButtonTextAlign,
            _: tekes_ui_tokens::Rgba,
        ) {
        }
    }
    let mut m = model(1., "");
    let mut v = ChartView::new();
    assert_eq!(
        v.arrange_owner(&mut Unknown, &mut m, Rect::new(16., 16., 480., 240.), None),
        Capacity::Unmeasured
    );
    assert_eq!(v.owner_focus_space(&m), Space::Unmeasured);
    let mut t = coretext::Text::new(1.);
    t.backing = 1.;
    v.arrange_owner(&mut t, &mut m, Rect::new(16., 16., 480., 240.), None);
    v.try_focus_owner(true, &mut m).unwrap();
    t.backing = 2.;
    let mut s = Scene::new();
    assert_eq!(
        v.paint_owner(&mut s, &mut t, &mut m, Theme::ALL[0].into()),
        Space::Unmeasured
    );
    assert!(m.is_focused());
    assert!(s.primitives().is_empty());
}
