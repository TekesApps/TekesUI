#![cfg(target_os = "macos")]
#[path = "support/coretext.rs"]
mod coretext;
use tekes_ui_controls::{
    BarChart, ChartContent, ChartDatum,
    ControlKey, RequiredFocusResult, ScrollArea,
    composition::{ComponentId, Content, ContentEvent, ContentHost, ScrollContent},
};
use tekes_ui_scene::{Point, Rect, Scene};
use tekes_ui_tokens::Theme;

const ROOT: ComponentId = ComponentId(9200);
const CHART: ComponentId = ComponentId(9201);
fn data(value: f64, label: &str) -> Vec<ChartDatum> {
    (0..64).map(|_| ChartDatum::new(label, value)).collect()
}
fn render(host: &mut ContentHost, text: &mut coretext::Text, frame: Rect, clip: Rect) {
    host.render(&mut Scene::new(), text, frame, clip, tekes_ui_tokens::Palette::from(Theme::LIGHT))
        .unwrap();
}
fn make_host(chart: ChartContent) -> ContentHost {
    ContentHost::new(Content::new(CHART, chart)).unwrap()
}
fn move_right(host: &mut ContentHost, count: usize) {
    for _ in 0..count {
        host.event(&ContentEvent::Key {
            key: ControlKey::Right,
            down: true,
        });
    }
}

#[test]
fn original_narrow_fixture_uses_content_host_and_preserves_zero_width_slots() {
    let mut text = coretext::Text::new(1.);
    text.backing = 1.;
    let mut host = make_host(ChartContent::try_new("Series", data(1., "")).unwrap());
    let frame = Rect::new(16., 16., 120., 240.);
    render(&mut host, &mut text, frame, frame);
    assert!(matches!(
        host.required_focus_result(CHART),
        Some(RequiredFocusResult::Visible { .. })
    ));
    assert!(host.focus(Some(CHART), true));
    move_right(&mut host, 2);
    assert_eq!(
        host.get(CHART)
            .unwrap()
            .component::<ChartContent>()
            .unwrap()
            .selected_index(),
        Some(2)
    );
    assert_eq!(host.focused(), Some(CHART));
}

#[test]
fn original_long_label_fixture_requires_real_parent_height_then_focuses() {
    let mut text = coretext::Text::new(1.);
    text.backing = 1.;
    let label = "Long duplicate category label with original complete text";
    let values = data(1., label);
    let model = BarChart::new(values.clone());
    let measurement_view = tekes_ui_controls::collections_view::ChartView::new();
    let measurement = measurement_view.measure_owner(&mut text, &model, 480., 240.);
    assert_eq!(
        measurement.capacity,
        tekes_ui_controls::collections_view::ChartOwnerCapacity::InsufficientBody
    );
    let extent = measurement.measured_label_extent.unwrap();
    assert!(extent.height > 240.);
    let mut host = make_host(
        ChartContent::try_new("Series", values)
            .unwrap()
            .preferred_height(extent.height + 240.),
    );
    let short = Rect::new(16., 16., 480., 240.);
    render(&mut host, &mut text, short, short);
    assert_eq!(
        host.required_focus_result(CHART),
        Some(RequiredFocusResult::Insufficient)
    );
    assert!(!host.focus(Some(CHART), true));
    let sufficient = Rect::new(16., 16., 480., extent.height + 240.);
    render(&mut host, &mut text, sufficient, sufficient);
    assert!(matches!(
        host.required_focus_result(CHART),
        Some(RequiredFocusResult::Visible { .. })
    ));
    assert!(host.focus(Some(CHART), true));
    move_right(&mut host, 2);
    assert_eq!(
        host.get(CHART)
            .unwrap()
            .component::<ChartContent>()
            .unwrap()
            .data()
            .len(),
        64
    );
}

#[test]
fn original_interior_clip_uses_host_scroll_reveal_and_renders_visible_witness() {
    let mut text = coretext::Text::new(1.);
    text.backing = 1.;
    let mut scroll = ScrollArea::default();
    scroll.offset = Point::new(0., 110.);
    let mut host = ContentHost::new(Content::new(
        ROOT,
        ScrollContent::new(
            scroll,
            Content::new(
                CHART,
                ChartContent::try_new("Series", data(-1., "")).unwrap(),
            ),
        ),
    ))
    .unwrap();
    let frame = Rect::new(21., 80., 470., 120.);
    let interior_clip = Rect::new(80., 80., 320., 120.);
    render(&mut host, &mut text, frame, interior_clip);
    assert!(matches!(
        host.required_focus_result(CHART),
        Some(RequiredFocusResult::Clipped { .. })
    ));
    let before_offset = host
        .get(ROOT)
        .unwrap()
        .component::<ScrollContent>()
        .unwrap()
        .scroll
        .offset
        .y;
    assert!(host.focus(Some(CHART), true));
    let after_offset = host
        .get(ROOT)
        .unwrap()
        .component::<ScrollContent>()
        .unwrap()
        .scroll
        .offset
        .y;
    assert!(after_offset > before_offset);
    render(&mut host, &mut text, frame, interior_clip);
    assert!(matches!(
        host.required_focus_result(CHART),
        Some(RequiredFocusResult::Visible { .. })
    ));
    assert_eq!(host.focused(), Some(CHART));
}

#[test]
fn original_tiny_fixture_refuses_new_focus_but_retains_live_owner_and_selection() {
    let mut text = coretext::Text::new(1.);
    let mut host = make_host(ChartContent::try_new("Series", data(-1., "")).unwrap());
    let enough = Rect::new(16., 16., 480., 240.);
    render(&mut host, &mut text, enough, enough);
    assert!(host.focus(Some(CHART), true));
    move_right(&mut host, 3);

    let tiny = Rect::new(16., 16., 4., 4.);
    render(&mut host, &mut text, tiny, tiny);
    assert_eq!(
        host.required_focus_result(CHART),
        Some(RequiredFocusResult::Insufficient)
    );
    assert_eq!(host.focused(), Some(CHART));
    assert_eq!(
        host.get(CHART)
            .unwrap()
            .component::<ChartContent>()
            .unwrap()
            .selected_index(),
        Some(3)
    );
    let mut fresh = make_host(ChartContent::try_new("Series", data(-1., "")).unwrap());
    render(&mut fresh, &mut text, tiny, tiny);
    assert!(!fresh.focus(Some(CHART), true));
    assert_eq!(fresh.focused(), None);
}
