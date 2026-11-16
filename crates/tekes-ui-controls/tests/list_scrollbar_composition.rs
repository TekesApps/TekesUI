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
                    && quad.fill
                        == palette
                            .color(if palette.theme.is_increased() {
                                "border"
                            } else {
                                "scrollbar-thumb"
                            })
                            .unwrap() =>
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
fn hover(host: &mut ContentHost, frame: Rect) {
    host.event(&pointer(frame.center(), PointerPhase::Move));
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
        let _ = render(&mut host, frame, palette);
        hover(&mut host, frame);
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
        precise: true,
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
    let _ = render(&mut host, frame, Theme::LIGHT.into());
    hover(&mut host, frame);
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
    hover(&mut host, frame);
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
    pane_padding(Some(tekes_ui_tokens::Spacing::Loose.value() * 2.));
}
#[test]
fn default_list_padding_stays_symmetric_with_a_real_overflow_scrollbar() {
    pane_padding(None);
}
fn pane_padding(explicit: Option<f32>) {
    let padding = explicit.unwrap_or(tekes_ui_tokens::Spacing::Loose.value());
    let list = ListContent::scrollable("Properties", ITEMS, rows());
    let list = if let Some(value) = explicit {
        list.content_padding(value)
    } else {
        list
    };
    let mut host = ContentHost::new(Content::new(SCROLL, list)).unwrap();
    let frame = Rect::new(20., 40., 240., 140.);
    let _ = render(&mut host, frame, Theme::LIGHT.into());
    hover(&mut host, frame);
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
    assert_eq!(host.get(ComponentId(109)).unwrap().visible(), Some(last));
    assert!(host.get(ComponentId(100)).unwrap().visible().is_none());
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
mod native_text {
    use super::*;
    use std::rc::Rc;
    use tekes_ui_controls::{TextSurfaceHost, TextSurfaceTextRenderer};
    pub(super) struct EditorText;
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
}

#[cfg(target_os = "macos")]
#[test]
fn search_clear_host_exit_preserves_query_focus_capture_and_actions() {
    use native_text::EditorText;
    const SEARCH: ComponentId = ComponentId(500);
    const OTHER: ComponentId = ComponentId(501);
    let frame = Rect::new(0., 0., 640., 80.);
    for mode in [
        "editor",
        "other",
        "outside",
        "blur",
        "cancel",
        "inactive",
        "clear_drag",
        "editor_drag",
        "click",
    ] {
        let mut host = ContentHost::new(Content::new(
            ComponentId(900),
            GridContent {
                model: Grid::new(2),
                children: vec![
                    Content::new(SEARCH, SearchFieldContent::new("Find", "query")),
                    Content::new(OTHER, Button::new("Other")),
                ],
            },
        ))
        .unwrap();
        host.set_action_events(true);
        let mut text = EditorText;
        let paint = |host: &mut ContentHost, text: &mut EditorText| {
            let mut scene = Scene::new();
            host.render(&mut scene, text, frame, frame, Theme::LIGHT.into())
                .unwrap();
            let clear = host
                .get(SEARCH)
                .unwrap()
                .component::<SearchFieldContent>()
                .unwrap()
                .semantic_parts()[0]
                .frame
                .center();
            let fill = scene
                .primitives()
                .iter()
                .rev()
                .find_map(|p| match p {
                    Primitive::Quad(q) if q.rect.contains(clear) => Some(q.fill),
                    _ => None,
                })
                .expect("original Clear chrome");
            (clear, fill)
        };
        paint(&mut host, &mut text);
        assert!(host.focus(Some(SEARCH), true));
        let (clear, rest) = paint(&mut host, &mut text);
        let mount = host.get(SEARCH).unwrap().mount_identity();
        let surface = host.text_surface(SEARCH).unwrap();
        let editor = surface.inner.center();
        assert_eq!(
            surface.outer.size.width,
            host.get(SEARCH).unwrap().frame().size.width
        );
        assert!(surface.outer.contains(clear));
        assert!(!surface.inner.contains(clear));
        let cursor_regions = host.text_cursor_regions();
        assert!(cursor_regions.iter().any(|r| r.contains(editor)));
        assert!(cursor_regions.iter().all(|r| !r.contains(clear)));
        let outside = Point::new(-1., -1.);
        host.event_with_text(&pointer(clear, PointerPhase::Move), &mut text)
            .unwrap();
        assert_ne!(paint(&mut host, &mut text).1, rest);
        match mode {
            "editor" | "other" | "outside" => {
                let point = match mode {
                    "editor" => editor,
                    "other" => host.get(OTHER).unwrap().frame().center(),
                    _ => outside,
                };
                host.event_with_text(&pointer(point, PointerPhase::Move), &mut text)
                    .unwrap();
            }
            "blur" => {
                host.focus(Some(OTHER), true);
            }
            "cancel" => host.cancel(),
            "inactive" => {
                host.set_window_active(false);
                host.set_window_active(true);
            }
            "clear_drag" | "click" => {
                host.event_with_text(&pointer(clear, PointerPhase::Down), &mut text)
                    .unwrap();
                assert_eq!(host.pointer_capture().map(|p| p.0), Some(SEARCH));
                let end = if mode == "click" { clear } else { outside };
                host.event_with_text(&pointer(end, PointerPhase::Drag), &mut text)
                    .unwrap();
                host.event_with_text(&pointer(end, PointerPhase::Up), &mut text)
                    .unwrap();
            }
            "editor_drag" => {
                host.event_with_text(&pointer(editor, PointerPhase::Move), &mut text)
                    .unwrap();
                host.event_with_text(&pointer(editor, PointerPhase::Down), &mut text)
                    .unwrap();
                host.event_with_text(&pointer(clear, PointerPhase::Drag), &mut text)
                    .unwrap();
                host.event_with_text(&pointer(clear, PointerPhase::Up), &mut text)
                    .unwrap();
            }
            _ => unreachable!(),
        }
        let actions = host.drain_action_records();
        let search = host
            .get(SEARCH)
            .unwrap()
            .component::<SearchFieldContent>()
            .unwrap();
        assert_eq!(
            search.query(),
            if mode == "click" { "" } else { "query" },
            "{mode}"
        );
        assert_eq!(actions.len(), usize::from(mode == "click"), "{mode}");
        assert!(host.pointer_capture().is_none());
        assert_eq!(host.get(SEARCH).unwrap().mount_identity(), mount);
        if mode != "blur" && mode != "cancel" && mode != "inactive" {
            assert_eq!(host.focused(), Some(SEARCH), "{mode}");
        }
        if mode != "click" {
            assert_eq!(paint(&mut host, &mut text).1, rest, "{mode}");
        }
    }
}

#[cfg(target_os = "macos")]
#[test]
fn search_clear_keeps_intrinsic_height_in_a_tall_host_allocation() {
    use native_text::EditorText;
    const SEARCH: ComponentId = ComponentId(500);
    for theme in [Theme::LIGHT, Theme::DARK] {
        for height in [24., 80., 560.] {
            let frame = Rect::new(20., 30., 640., height);
            let mut text = EditorText;
            let mut host = ContentHost::new(Content::new(
                SEARCH,
                SearchFieldContent::new("Find", "query"),
            ))
            .unwrap();
            host.set_action_events(true);
            let mut scene = Scene::new();
            host.render(&mut scene, &mut text, frame, frame, theme.into())
                .unwrap();
            let part = host
                .get(SEARCH)
                .unwrap()
                .component::<SearchFieldContent>()
                .unwrap()
                .semantic_parts()[0]
                .frame;
            let outer = host.text_surface(SEARCH).unwrap().outer;
            let inset = tekes_ui_tokens::Spacing::Micro.value();
            assert_eq!(part.min_y(), outer.min_y() + inset);
            // ContentHost preserves the editor's minimum layout height even
            // when the caller's viewport is shorter. AX publishes the clipped
            // interior button, not a second button resized to that viewport.
            assert_eq!(part.max_y(), (outer.max_y() - inset).min(frame.max_y()));
            assert_eq!(part.max_x(), outer.max_x() - inset);
            assert!(outer.intersection(&part) == Some(part));
            if height > outer.size.height {
                let below = Point::new(part.center().x, frame.max_y() - 1.);
                for phase in [PointerPhase::Move, PointerPhase::Down, PointerPhase::Up] {
                    host.event_with_text(&pointer(below, phase), &mut text)
                        .unwrap();
                }
                assert!(host.drain_action_records().is_empty());
                assert_eq!(
                    host.get(SEARCH)
                        .unwrap()
                        .component::<SearchFieldContent>()
                        .unwrap()
                        .query(),
                    "query"
                );
            }
        }
    }
}

#[cfg(target_os = "macos")]
#[test]
fn scrollbar_drag_preserves_real_sibling_editor_marked_input_and_focus() {
    use native_text::EditorText;
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
    let scroll_frame = host.get(SCROLL).unwrap().frame();
    hover(&mut host, scroll_frame);
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
    scene = Scene::new();
    host.render(&mut scene, &mut text, frame, frame, Theme::LIGHT.into())
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

#[cfg(target_os = "macos")]
#[test]
fn padded_stack_header_keeps_natural_height_and_stays_fixed_while_list_scrolls() {
    use tekes_ui_controls::{
        extended::{Axis, Stack},
        layout::Insets,
    };
    let mut text = native_text::EditorText;
    const HEADER: ComponentId = ComponentId(800);
    const FIELD: ComponentId = ComponentId(801);
    let padding = tekes_ui_tokens::Spacing::Loose.value();
    let header = StackContent {
        model: Stack {
            axis: Axis::Vertical,
            gap: 0.,
        },
        children: vec![Content::new(FIELD, SearchFieldContent::new("Find", ""))],
        flexible_child: None,
    }
    .into_content_with_padding(HEADER, Insets::new(padding, padding, 0., padding));
    let mut root = Content::new(
        ComponentId(900),
        StackContent {
            model: Stack {
                axis: Axis::Vertical,
                gap: 0.,
            },
            children: vec![
                header,
                Content::new(SCROLL, ListContent::scrollable("Items", ITEMS, rows())),
            ],
            flexible_child: Some(SCROLL),
        },
    );
    let mut natural_field = SearchFieldContent::new("Find", "");
    let field_height = natural_field
        .measure(&mut text, Size::new(300. - 2. * padding, 300.))
        .size
        .height;
    let header_height = root.component_mut::<StackContent>().unwrap().children[0]
        .measure(&mut text, Size::new(300., 300.))
        .size
        .height;
    assert_eq!(header_height, field_height + padding);
    let mut host = ContentHost::new(root).unwrap();
    for width in [300., 80.] {
        let frame = Rect::new(10., 20., width, 160.);
        host.render(
            &mut Scene::new(),
            &mut text,
            frame,
            frame,
            Theme::LIGHT.into(),
        )
        .unwrap();
        let field = host.get(FIELD).unwrap().frame();
        assert_eq!(field.min_x(), frame.min_x() + padding);
        assert_eq!(field.max_x(), frame.max_x() - padding);
        assert_eq!(field.min_y(), frame.min_y() + padding);
        assert_eq!(host.get(HEADER).unwrap().frame().size.height, header_height);
        let mount = host.get(FIELD).unwrap().mount_identity();
        host.focus(Some(FIELD), false);
        assert_eq!(host.focused(), Some(FIELD));
        let view = host
            .get_mut(SCROLL)
            .unwrap()
            .component_mut::<ScrollContent>()
            .unwrap();
        view.scroll.scroll_by(0., view.scroll.limit().y);
        host.render(
            &mut Scene::new(),
            &mut text,
            frame,
            frame,
            Theme::LIGHT.into(),
        )
        .unwrap();
        assert_eq!(host.get(FIELD).unwrap().frame(), field);
        assert_eq!(host.get(FIELD).unwrap().mount_identity(), mount);
        assert_eq!(host.focused(), Some(FIELD));
        assert!(scroll(&host).scroll.offset.y > 0.);
        assert_eq!(host.get(FIELD).unwrap().visible(), Some(field));
    }
}

#[cfg(target_os = "macos")]
#[test]
fn ordinary_text_field_clear_uses_original_editor_and_rejects_replaced_draft() {
    use native_text::EditorText;
    const FIELD: ComponentId = ComponentId(960);
    let frame = Rect::new(0., 0., 320., 80.);
    for replacement in [false, true] {
        let mut host = ContentHost::new(Content::new(
            FIELD,
            TextFieldContent::new("Draft", "original"),
        ))
        .unwrap();
        host.set_action_events(true);
        let mut text = EditorText;
        host.render(
            &mut Scene::new(),
            &mut text,
            frame,
            frame,
            Theme::LIGHT.into(),
        )
        .unwrap();
        assert!(host.focus(Some(FIELD), true));
        let mount = host.get(FIELD).unwrap().mount_identity();
        let field = host
            .get(FIELD)
            .unwrap()
            .component::<TextFieldContent>()
            .unwrap();
        let clear = field.semantic_parts()[0].frame;
        assert_eq!(field.semantics().role, ContentRole::TextInput);
        assert_eq!(field.surface.outer.intersection(&clear), Some(clear));
        assert!(field.surface.inner.max_x() <= clear.min_x());
        assert!(
            host.text_cursor_regions()
                .iter()
                .all(|r| !r.contains(clear.center()))
        );
        host.event_with_text(&pointer(clear.center(), PointerPhase::Down), &mut text)
            .unwrap();
        if replacement {
            let field = host
                .get_mut(FIELD)
                .unwrap()
                .component_mut::<TextFieldContent>()
                .unwrap();
            // The public original editor may receive an external publication
            // between down and up; the pending Clear must not erase that value.
            let baseline = field.surface.input.external_baseline();
            let value = tekes_ui_controls::DraftRevision {
                identity: "replacement".into(),
                version: 1,
                payload: "new value".into(),
            };
            let result = field.surface.apply_external_value(
                &baseline,
                &value,
                tekes_ui_controls::ExternalSelection::Reset,
                tekes_ui_controls::ExternalUndo::Preserve,
            );
            assert!(matches!(
                result,
                tekes_ui_controls::ExternalValueResult::Applied { .. }
            ));
        }
        host.event_with_text(&pointer(clear.center(), PointerPhase::Up), &mut text)
            .unwrap();
        let actions = host.drain_action_records();
        assert_eq!(actions.len(), usize::from(!replacement));
        if !replacement {
            assert_eq!(actions[0].action.kind, ContentActionKind::Clear);
        }
        assert_eq!(host.focused(), Some(FIELD));
        assert_eq!(host.get(FIELD).unwrap().mount_identity(), mount);
        let field = host
            .get_mut(FIELD)
            .unwrap()
            .component_mut::<TextFieldContent>()
            .unwrap();
        assert_eq!(
            field.surface.input.text,
            if replacement { "new value" } else { "" }
        );
        if !replacement {
            assert!(field.surface.input.undo());
            assert_eq!(field.surface.input.text, "original");
            assert!(!field.surface.input.undo());
        }
    }
}

#[test]
fn text_field_clear_policy_preserves_multiline_document_and_unavailable_input() {
    for mode in [
        "empty",
        "multiline",
        "document",
        "opt_out",
        "readonly",
        "disabled",
        "marked",
    ] {
        let mut field = TextFieldContent::new("Draft", if mode == "empty" { "" } else { "draft" });
        match mode {
            "multiline" => field.surface.input.multiline = true,
            "document" => {
                field.surface.presentation = tekes_ui_controls::TextSurfacePresentation::Content
            }
            "opt_out" => field = field.clear_button(false),
            "readonly" => field.surface.input.read_only = true,
            "disabled" => field.surface.input.disabled = true,
            "marked" => field.surface.input.marked = Some(0..5),
            _ => {}
        }
        let original = field.surface.input.text.clone();
        let frame = Rect::new(0., 0., 300., 80.);
        field.arrange(&mut Text, frame, frame);
        let parts = field.semantic_parts();
        if ["empty", "multiline", "document", "opt_out"].contains(&mode) {
            assert!(parts.is_empty(), "{mode}");
        } else {
            assert!(parts[0].semantics.disabled, "{mode}");
            for event in [
                ContentEvent::PointerDown(parts[0].frame.center()),
                ContentEvent::PointerUp(parts[0].frame.center()),
            ] {
                assert!(!field.event(&event).activated, "{mode}");
            }
        }
        assert_eq!(field.surface.input.text, original, "{mode}");
    }
}
