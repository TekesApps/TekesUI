use tekes_ui_controls::{ButtonTextAlign, ButtonTextRenderer, ScrollArea, composition::*};
use tekes_ui_scene::{Point, Rect, Scene, Size};
use tekes_ui_tokens::{Rgba, Theme, proposal};
struct Text;
impl ButtonTextRenderer for Text {
    fn measure(&mut self, text: &str, _: proposal::TextRole, width: f32) -> Size {
        Size::new((text.len() as f32 * 7.).min(width), 20.)
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
fn render(host: &mut ContentHost) {
    let frame = Rect::new(0., 0., 150., 48.);
    host.render(
        &mut Scene::new(),
        &mut Text,
        frame,
        frame,
        Theme::LIGHT.into(),
    )
    .unwrap();
}
#[test]
fn horizontal_tab_strip_uses_real_document_wheel_clip_and_selected_reveal() {
    let strip = TabStripContent::new(
        [
            "I",
            "Long title exceeding the maximum title width by many characters",
            "Last tab",
        ]
        .into_iter()
        .enumerate()
        .map(|(i, name)| TabStripItem {
            label: Content::new(ComponentId(10 + i as u64), TextContent::new(name)),
            name: name.into(),
            closable: false,
        })
        .collect(),
        0,
    );
    let mut host = ContentHost::new(Content::new(
        ComponentId(1),
        ScrollContent::new(ScrollArea::default(), Content::new(ComponentId(2), strip))
            .horizontal(true),
    ))
    .unwrap();
    host.set_action_events(true);
    render(&mut host);
    let scroll = host
        .get(ComponentId(1))
        .unwrap()
        .component::<ScrollContent>()
        .unwrap();
    assert!(scroll.scroll.document_size().width > 300.);
    assert_eq!(scroll.scroll.offset.x, 0.);
    let first = host.get(ComponentId(10)).unwrap().frame();
    let long = host.get(ComponentId(11)).unwrap().frame();
    assert!(long.size.width > first.size.width);
    assert!(long.size.width <= TAB_TITLE_MAX_WIDTH);
    assert!(
        host.event(&ContentEvent::Scroll {
            position: Point::new(60., 20.),
            delta: Point::new(90., 0.),
            precise: true,
            begin: true,
            end: true,
            momentum: false,
            time: 1.
        })
        .consumed
    );
    render(&mut host);
    assert!(
        host.get(ComponentId(1))
            .unwrap()
            .component::<ScrollContent>()
            .unwrap()
            .scroll
            .offset
            .x
            > 0.
    );
    assert!(host.get(ComponentId(10)).unwrap().frame().max_x() <= 0.);
    assert!(
        !host
            .event(&ContentEvent::PointerDown(Point::new(-10., 20.)))
            .consumed
    );
    let strip = host
        .get_mut(ComponentId(2))
        .unwrap()
        .component_mut::<TabStripContent>()
        .unwrap();
    strip.select(2);
    let rect = strip.selected_rect();
    host.get_mut(ComponentId(1))
        .unwrap()
        .component_mut::<ScrollContent>()
        .unwrap()
        .reveal_horizontal(rect);
    render(&mut host);
    let last = host.get(ComponentId(12)).unwrap().frame();
    assert!(last.min_x() >= 0. && last.max_x() <= 150.);
    assert_eq!(
        host.get(ComponentId(1))
            .unwrap()
            .component::<ScrollContent>()
            .unwrap()
            .scroll
            .offset
            .y,
        0.
    );
}

#[test]
fn horizontal_false_restores_vertical_document_and_wheel_policy() {
    let rows = (0..12)
        .map(|index| Content::new(ComponentId(10 + index), TextContent::new("Row")))
        .collect();
    let scroll = ScrollContent::new(
        ScrollArea::default(),
        Content::new(ComponentId(2), ListContent::new("Rows", rows)),
    )
    .horizontal(true)
    .horizontal(false);
    let mut host = ContentHost::new(Content::new(ComponentId(1), scroll)).unwrap();
    render(&mut host);
    assert!(
        host.event(&ContentEvent::Scroll {
            position: Point::new(60., 20.),
            delta: Point::new(0., 30.),
            precise: true,
            begin: true,
            end: true,
            momentum: false,
            time: 1.
        })
        .consumed
    );
    let scroll = host
        .get(ComponentId(1))
        .unwrap()
        .component::<ScrollContent>()
        .unwrap();
    assert!(scroll.scroll.offset.y > 0.);
    assert_eq!(scroll.scroll.offset.x, 0.);
}

#[test]
fn toolbar_scroll_admits_natural_group_and_rejects_clipped_group_focus() {
    for horizontal in [true, false] {
        let group = ToolbarGroup::new(
            ToolbarPlacement::Navigation,
            vec![
                Content::new(ComponentId(10), Button::new("First long destination")),
                Content::new(ComponentId(11), Button::new("Second long destination")),
            ],
        );
        let mut host = ContentHost::new(Content::new(
            ComponentId(1),
            ScrollContent::new(ScrollArea::default(), Content::new(ComponentId(2), group))
                .horizontal(horizontal),
        ))
        .unwrap();
        render(&mut host);
        let frame = host.get(ComponentId(10)).unwrap().frame();
        let point = Point::new(frame.min_x() + 5., frame.center().y);
        assert_eq!(
            host.event(&ContentEvent::PointerDown(point)).consumed,
            horizontal
        );
        assert_eq!(host.focused(), horizontal.then_some(ComponentId(10)));
    }
}

#[test]
fn wheel_line_units_convert_once_on_both_axes_while_precise_points_stay_unchanged() {
    for horizontal in [true, false] {
        for precise in [false, true] {
            let children = (0..12)
                .map(|index| {
                    Content::new(
                        ComponentId(10 + index),
                        TextContent::new("Long row for horizontal overflow"),
                    )
                })
                .collect();
            let body = if horizontal {
                Content::new(
                    ComponentId(2),
                    ToolbarGroup::new(ToolbarPlacement::Navigation, children),
                )
            } else {
                Content::new(ComponentId(2), ListContent::new("Rows", children))
            };
            let mut host = ContentHost::new(Content::new(
                ComponentId(1),
                ScrollContent::new(ScrollArea::default(), body).horizontal(horizontal),
            ))
            .unwrap();
            render(&mut host);
            let delta = if horizontal {
                Point::new(0.1, 0.)
            } else {
                Point::new(0., 0.1)
            };
            assert!(
                host.event(&ContentEvent::Scroll {
                    position: Point::new(60., 20.),
                    delta,
                    precise,
                    begin: false,
                    end: false,
                    momentum: false,
                    time: 1.
                })
                .consumed
            );
            render(&mut host);
            let scroll = &host
                .get(ComponentId(1))
                .unwrap()
                .component::<ScrollContent>()
                .unwrap()
                .scroll;
            let offset = if horizontal {
                scroll.offset.x
            } else {
                scroll.offset.y
            };
            let expected = if precise {
                0.1
            } else {
                0.1 * proposal::metric("size", "size-row-height").unwrap()
            };
            assert!(
                (offset - expected).abs() < 0.0001,
                "horizontal={horizontal}, precise={precise}, offset={offset}"
            );
        }
    }
}
