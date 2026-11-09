use tekes_ui_controls::{
    ButtonFramePolicy, ButtonTextAlign, ButtonTextRenderer, composition::*, extended::Grid,
};
use tekes_ui_scene::{Point, Primitive, Rect, Scene, Size};
use tekes_ui_tokens::{Palette, Rgba, Theme, proposal};

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
const SCROLL: ComponentId = ComponentId(1);
const ITEMS: ComponentId = ComponentId(2);
fn rows() -> Vec<Content> {
    (0..2)
        .map(|group| {
            Content::new(
                ComponentId(10 + group),
                ListContent::new(
                    format!("Group {group}"),
                    (0..5)
                        .map(|row| {
                            let mut button = Button::new(format!("Item {group}/{row}"));
                            button.button.frame_policy = ButtonFramePolicy::Exact;
                            Content::new(ComponentId(100 + group * 5 + row), button)
                        })
                        .collect(),
                ),
            )
        })
        .collect()
}
fn host(visible: bool) -> ContentHost {
    let mut host = ContentHost::new(Content::new(
        SCROLL,
        ListContent::scrollable("Grouped properties", ITEMS, rows()).scrollbar(visible),
    ))
    .unwrap();
    host.set_action_events(true);
    host
}
fn render(host: &mut ContentHost, frame: Rect, palette: Palette) -> Scene {
    let mut scene = Scene::new();
    host.render(&mut scene, &mut Text, frame, frame, palette)
        .unwrap();
    scene
}
fn scroll(host: &ContentHost) -> &ScrollContent {
    host.get(SCROLL)
        .unwrap()
        .component::<ScrollContent>()
        .unwrap()
}
fn thumb(scene: &Scene, palette: Palette) -> Option<Rect> {
    scene
        .primitives()
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Quad(quad)
                if quad.rect.size.width == proposal::SCROLLBAR_WIDTH
                    && quad.fill == palette.color("secondary-text").unwrap() =>
            {
                Some(quad.rect)
            }
            _ => None,
        })
}
fn pointer(position: Point, phase: PointerPhase) -> ContentEvent {
    ContentEvent::Pointer(PointerEvent {
        position,
        phase,
        button: Some(PointerButton::Primary),
        modifiers: Default::default(),
        click_count: 1,
    })
}

#[test]
fn grouped_list_uses_one_original_scroller_and_thumb_capture_never_activates_items() {
    fn owners(content: &Content) -> usize {
        if let Some(scroll) = content.component::<ScrollContent>() {
            1 + scroll.children().iter().map(owners).sum::<usize>()
        } else if let Some(list) = content.component::<ListContent>() {
            list.children().iter().map(owners).sum()
        } else {
            0
        }
    }
    for theme in Theme::ALL {
        let mut host = host(true);
        let palette = theme.into();
        let frame = Rect::new(0., 0., 240., 140.);
        let scene = render(&mut host, frame, palette);
        assert_eq!(owners(host.get(SCROLL).unwrap()), 1);
        assert!(
            host.get(ITEMS)
                .unwrap()
                .component::<ListContent>()
                .is_some()
        );
        assert!(host.get(ComponentId(109)).is_some());
        let bar = thumb(&scene, palette).expect("overflow List paints its original thumb");
        let event = pointer(bar.center(), PointerPhase::Down);
        let before = format!("{:?}", scroll(&host));
        assert!(matches!(
            scroll(&host).prepare_action(ComponentActionRequest::PointerDown(&event)),
            ActionPreparation::Ready(_)
        ));
        assert_eq!(
            format!("{:?}", scroll(&host)),
            before,
            "preparing a drag is pure"
        );
        assert!(host.event(&event).capture_pointer);
        assert_eq!(host.pointer_capture().unwrap().0, SCROLL);
        // Render between Down and Drag, as the real Host does. Unchanged
        // document layout must not end the original thumb gesture.
        render(&mut host, frame, palette);
        host.event(&pointer(
            Point::new(bar.center().x, 125.),
            PointerPhase::Drag,
        ));
        assert!(scroll(&host).scroll.offset.y > 0.);
        render(&mut host, frame, palette);
        host.event(&pointer(Point::new(400., 180.), PointerPhase::Up));
        assert!(host.pointer_capture().is_none());
        assert!(host.drain_actions().is_empty());
        assert_eq!(host.focused(), None);
    }
}

#[test]
fn hidden_scrollbar_preserves_wheel_and_checked_focus_reveal() {
    let mut host = host(false);
    let frame = Rect::new(0., 0., 240., 140.);
    let scene = render(&mut host, frame, Theme::LIGHT.into());
    assert!(thumb(&scene, Theme::LIGHT.into()).is_none());
    assert!(scroll(&host).scroll.limit().y > 0.);
    let first = host.get(ComponentId(100)).unwrap().frame();
    let old_track = Point::new(
        frame.max_x() - proposal::SCROLLBAR_INSET - proposal::SCROLLBAR_WIDTH * 0.5,
        first.center().y,
    );
    assert!(
        !first.contains(old_track),
        "the row keeps its natural width"
    );
    assert!(
        !host
            .event(&pointer(old_track, PointerPhase::Down))
            .capture_pointer
    );
    assert!(host.pointer_capture().is_none());
    host.event(&pointer(old_track, PointerPhase::Up));
    assert!(host.drain_actions().is_empty());
    assert!(
        host.event(&pointer(first.center(), PointerPhase::Down))
            .capture_pointer
    );
    assert_eq!(host.pointer_capture().unwrap().0, ComponentId(100));
    host.event(&pointer(first.center(), PointerPhase::Up));
    assert_eq!(host.drain_actions().len(), 1);
    let wheel = ContentEvent::Scroll {
        position: Point::new(80., 70.),
        delta: Point::new(0., 35.),
        precise: false,
        begin: true,
        end: true,
        momentum: false,
        time: 1.,
    };
    assert!(host.event(&wheel).changed);
    assert!(scroll(&host).scroll.offset.y > 0.);
    render(&mut host, frame, Theme::LIGHT.into());
    assert!(host.focus(Some(ComponentId(109)), true));
    assert_eq!(host.focused(), Some(ComponentId(109)));
    render(&mut host, frame, Theme::LIGHT.into());
    let last = host.get(ComponentId(109)).unwrap();
    assert!(
        last.visible()
            .is_some_and(|visible| visible.size.height > 0.)
    );
    assert!(scroll(&host).scroll.offset.y > 35.);
}

#[test]
fn track_paging_and_cancel_keep_items_unactivated_and_fitting_lists_have_no_hit() {
    let frame = Rect::new(0., 0., 240., 140.);
    let mut host = host(true);
    let scene = render(&mut host, frame, Theme::LIGHT.into());
    let bar = thumb(&scene, Theme::LIGHT.into()).unwrap();
    let page = Point::new(bar.center().x, 125.);
    assert!(
        host.event(&pointer(page, PointerPhase::Down))
            .capture_pointer
    );
    assert!(scroll(&host).scroll.offset.y > 0.);
    host.event(&ContentEvent::Cancel);
    assert!(host.pointer_capture().is_none());
    assert!(scroll(&host).scroll.dragging_axis().is_none());
    assert!(host.drain_actions().is_empty());

    let mut fitting = ContentHost::new(Content::new(
        SCROLL,
        ListContent::scrollable(
            "Fits",
            ITEMS,
            vec![Content::new(ComponentId(100), TextContent::new("Short"))],
        ),
    ))
    .unwrap();
    let scene = render(&mut fitting, frame, Theme::LIGHT.into());
    assert!(thumb(&scene, Theme::LIGHT.into()).is_none());
    assert!(
        !fitting
            .event(&pointer(page, PointerPhase::Down))
            .capture_pointer
    );
    assert_eq!(scroll(&fitting).scroll.offset, Point::default());
}

#[test]
fn admitted_thumb_navigation_stops_original_reading_follow_and_wins_reflow() {
    let mut host = host(true);
    let frame = Rect::new(0., 0., 240., 140.);
    render(&mut host, frame, Theme::LIGHT.into());
    host.set_scroll_reading_document(
        SCROLL,
        "grouped",
        1,
        (100..110).map(ComponentId).collect(),
        |_| (),
    )
    .unwrap();
    host.get_mut(SCROLL)
        .unwrap()
        .component_mut::<ScrollContent>()
        .unwrap()
        .follow_latest();
    let scene = render(&mut host, frame, Theme::LIGHT.into());
    assert!(scroll(&host).is_following_latest());
    let bar = thumb(&scene, Theme::LIGHT.into()).unwrap();
    host.event(&pointer(bar.center(), PointerPhase::Down));
    assert!(!scroll(&host).is_following_latest());
    host.event(&pointer(
        Point::new(bar.center().x, 10.),
        PointerPhase::Drag,
    ));
    let selected_offset = scroll(&host).scroll.offset.y;
    assert!(selected_offset < scroll(&host).scroll.limit().y);
    render(&mut host, frame, Theme::LIGHT.into());
    assert!((scroll(&host).scroll.offset.y - selected_offset).abs() < 0.01);
    assert!(scroll(&host).reading_result().unwrap().user_input_won);
    host.event(&pointer(Point::new(400., 180.), PointerPhase::Up));
    assert!(host.pointer_capture().is_none());
}

#[test]
fn explicit_pane_padding_stays_symmetric_with_a_real_overflow_scrollbar() {
    let padding = tekes_ui_tokens::Spacing::Loose.value();
    let mut host = ContentHost::new(Content::new(
        SCROLL,
        ListContent::scrollable("Properties", ITEMS, rows()).content_padding(padding),
    ))
    .unwrap();
    let frame = Rect::new(20., 40., 240., 140.);
    let scene = render(&mut host, frame, Theme::LIGHT.into());
    assert!(thumb(&scene, Theme::LIGHT.into()).is_some());
    let items = host.get(ITEMS).unwrap().frame();
    assert_eq!(items.min_x() - frame.min_x(), padding);
    assert_eq!(frame.max_x() - items.max_x(), padding);
    assert_eq!(items.min_y() - frame.min_y(), padding);
    let first = host.get(ComponentId(100)).unwrap().frame();
    assert_eq!(first.min_x(), items.min_x());
    assert_eq!(first.min_y(), items.min_y());
    assert!(first.max_x() <= items.max_x());
    let view = host
        .get_mut(SCROLL)
        .unwrap()
        .component_mut::<ScrollContent>()
        .unwrap();
    view.scroll.scroll_by(0., view.scroll.limit().y);
    render(&mut host, frame, Theme::LIGHT.into());
    let last = host.get(ComponentId(109)).unwrap().frame();
    let items = host.get(ITEMS).unwrap().frame();
    assert!(last.min_x() >= items.min_x() && last.max_x() <= items.max_x());
    assert_eq!(frame.max_y() - last.max_y(), padding);
}

#[test]
fn navigation_row_measurement_and_real_hits_use_gallery_tokens_without_changing_defaults() {
    use tekes_ui_tokens::{ControlSize, Spacing};
    for navigation in [false, true] {
        let mut list = SelectableListContent::new("Routes", vec!["First".into(), "Second".into()]);
        list.view.set_navigation_layout(navigation);
        let measured = list.measure(&mut Text, Size::new(240., 140.));
        let mut host = ContentHost::new(Content::new(ComponentId(55), list)).unwrap();
        render(
            &mut host,
            Rect::new(0., 0., 240., 140.),
            Theme::LIGHT.into(),
        );
        let current = host
            .get(ComponentId(55))
            .unwrap()
            .component::<SelectableListContent>()
            .unwrap();
        let layouts = &current.view.layout().items;
        let (height, gap) = if navigation {
            (ControlSize::RowHeight.value(), Spacing::Micro.value())
        } else {
            (36., 4.)
        };
        assert_eq!(layouts[0].frame.size.height, height);
        assert_eq!(layouts[1].frame.min_y() - layouts[0].frame.max_y(), gap);
        assert_eq!(measured.size.height, layouts[1].frame.max_y());
        let second = layouts[1].frame.center();
        host.event(&pointer(second, PointerPhase::Down));
        host.event(&pointer(second, PointerPhase::Up));
        assert_eq!(
            host.get(ComponentId(55))
                .unwrap()
                .component::<SelectableListContent>()
                .unwrap()
                .model
                .selected,
            Some(1)
        );
    }
}

#[cfg(target_os = "macos")]
#[test]
fn scrollbar_drag_preserves_real_sibling_editor_marked_input_and_focus() {
    use std::rc::Rc;
    use tekes_ui_controls::{TextSurfaceHost, TextSurfaceTextRenderer};
    struct EditorText;
    impl ButtonTextRenderer for EditorText {
        fn measure(&mut self, value: &str, role: proposal::TextRole, width: f32) -> Size {
            Text.measure(value, role, width)
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
        fn text_surface_host(&mut self) -> Option<&mut dyn TextSurfaceHost> {
            Some(self)
        }
    }
    impl TextSurfaceTextRenderer for EditorText {
        fn push_visible(
            &mut self,
            _: &mut Scene,
            _: &tekes_ui_text::model::Paragraph,
            _: Point,
            _: Rgba,
            _: Rect,
        ) {
        }
    }
    impl TextSurfaceHost for EditorText {
        fn shape_surface(
            &mut self,
            value: &str,
            role: proposal::TextRole,
            width: Option<f32>,
        ) -> Rc<tekes_ui_text::model::Paragraph> {
            use tekes_ui_text::traits::Shaper;
            let mut shaper = tekes_ui_text::coretext::CoreTextShaper::new(
                tekes_ui_text::coretext::CoreTextFonts::new(),
            );
            Rc::new(shaper.shape(
                value,
                &tekes_ui_text::style::TextStyle::system(role.style().size),
                &tekes_ui_text::model::ShapeOptions {
                    width: width.map(f64::from),
                    line_spacing: 0.,
                },
            ))
        }
    }
    const EDITOR: ComponentId = ComponentId(500);
    let mut host = ContentHost::new(Content::new(
        ComponentId(900),
        GridContent {
            model: Grid::new(2),
            children: vec![
                Content::new(SCROLL, ListContent::scrollable("Navigation", ITEMS, rows())),
                Content::new(EDITOR, TextFieldContent::new("Draft", "original draft")),
            ],
        },
    ))
    .unwrap();
    let frame = Rect::new(0., 0., 500., 140.);
    let mut text = EditorText;
    let mut scene = Scene::new();
    host.render(&mut scene, &mut text, frame, frame, Theme::LIGHT.into())
        .unwrap();
    assert!(host.focus(Some(EDITOR), true));
    host.event_with_text(
        &ContentEvent::Preedit {
            text: "中文".into(),
            selection: 0..2,
            clauses: std::iter::once(0..2).collect(),
            replacement: None,
        },
        &mut text,
    )
    .unwrap();
    host.render(
        &mut Scene::new(),
        &mut text,
        frame,
        frame,
        Theme::LIGHT.into(),
    )
    .unwrap();
    let before = host.text_input_snapshot().unwrap();
    let focus = host.focus_epoch();
    let bar = thumb(&scene, Theme::LIGHT.into()).unwrap();
    host.event_with_text(&pointer(bar.center(), PointerPhase::Down), &mut text)
        .unwrap();
    assert_eq!(host.pointer_capture().unwrap().0, SCROLL);
    host.event_with_text(
        &pointer(Point::new(bar.center().x, 125.), PointerPhase::Drag),
        &mut text,
    )
    .unwrap();
    assert!(scroll(&host).scroll.offset.y > 0.);
    host.event_with_text(
        &pointer(Point::new(600., 160.), PointerPhase::Up),
        &mut text,
    )
    .unwrap();
    assert_eq!(host.focused(), Some(EDITOR));
    assert_eq!(host.focus_epoch(), focus);
    assert_eq!(host.text_input_snapshot().unwrap(), before);
}
