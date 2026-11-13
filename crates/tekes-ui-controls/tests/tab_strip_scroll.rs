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

#[test]
fn toolbar_tabs_share_outer_spacing_and_reveal_first_and_last_backgrounds() {
    struct ToolbarText {
        scale: f32,
        backing: f32,
    }
    impl tekes_ui_controls::layout_view::IconRenderer for ToolbarText {
        fn prepare_icon(
            &mut self,
            _: &str,
            _: f32,
            _: f32,
            _: Rgba,
        ) -> Result<IconImage, IconRenderError> {
            Ok(IconImage::new(
                tekes_ui_scene::ImageId(1),
                std::rc::Rc::new(()),
            ))
        }
        fn draw_icon(&mut self, _: &mut Scene, _: &str, _: Rect, _: Rgba) {
            unreachable!()
        }
    }
    impl ButtonTextRenderer for ToolbarText {
        fn icon_renderer(
            &mut self,
        ) -> Option<&mut dyn tekes_ui_controls::layout_view::IconRenderer> {
            Some(self)
        }
        fn backing_scale(&self) -> f32 {
            self.backing
        }
        fn resolved_font_size(&mut self, role: proposal::TextRole) -> f32 {
            role.style().size * self.scale
        }
        fn measure(&mut self, label: &str, role: proposal::TextRole, width: f32) -> Size {
            Size::new(
                (label.len() as f32 * 7. * self.scale).min(width),
                role.style().line_height * self.scale,
            )
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
    fn near(a: f32, b: f32) {
        assert!((a - b).abs() < 0.002, "{a} != {b}");
    }
    for backing in [1., 2.] {
        let mut text = ToolbarText { scale: 1., backing };
        let viewport = Rect::new(0., 0., 1000., 500.);
        let mut host: ToolbarHost<()> = ToolbarHost::new(ComponentId(100), 1, viewport).unwrap();
        let strip = TabStripContent::new(
            ["Inspector", "Reference", "Summary"]
                .into_iter()
                .enumerate()
                .map(|(i, name)| TabStripItem {
                    label: Content::new(ComponentId(110 + i as u64), TextContent::new(name)),
                    name: name.into(),
                    closable: true,
                })
                .collect(),
            0,
        );
        let region = ToolbarRegion::new(ComponentId(101), Rect::new(0., 0., 200., 42.));
        host.set_regions(1, viewport, vec![region]).unwrap();
        host.set_region_horizontal_inset(ComponentId(101), Some(0.))
            .unwrap();
        host.register(ToolbarRegistration {
            region: ComponentId(101),
            order: 0,
            group: Content::new(
                ComponentId(102),
                ToolbarGroup::new(
                    ToolbarPlacement::Navigation,
                    vec![Content::new(
                        ComponentId(103),
                        StackContent {
                            model: tekes_ui_controls::extended::Stack::new(
                                tekes_ui_controls::extended::Axis::Horizontal,
                            ),
                            flexible_child: Some(ComponentId(104)),
                            children: vec![
                                Content::new(
                                    ComponentId(104),
                                    ScrollContent::new(
                                        ScrollArea::default(),
                                        Content::new(ComponentId(105), strip),
                                    )
                                    .horizontal(true),
                                ),
                                Content::new(ComponentId(106), Button::new("+")),
                            ],
                        },
                    )],
                ),
            ),
            bindings: vec![ToolbarCommandBinding {
                target: ComponentId(105),
                action: None,
                command: 0,
                source: (),
            }],
        })
        .unwrap();
        // Reuse mounted state across typography changes, as Settings does.
        for scale in [0.5, 1., 1.1, 2., 3.] {
            text.scale = scale;
            let row = tekes_ui_controls::ToolbarControl::window().row_height(&mut text);
            let region_frame = Rect::new(0., 0., 200. * scale, row);
            let region = ToolbarRegion::new(ComponentId(101), region_frame);
            host.set_regions(1, viewport, vec![region]).unwrap();
            host.render(&mut Scene::new(), &mut text, Theme::LIGHT.into())
                .unwrap();
            let focus = tekes_ui_controls::extended::FocusRingMetrics::new(backing).reserved_band();
            let padding = (tekes_ui_tokens::Spacing::Regular.value() * scale).max(focus);
            let plus = host.content_host().get(ComponentId(106)).unwrap().frame();
            for selected in [0, 2, 0] {
                let mut target = Rect::default();
                host.update_component::<TabStripContent>(
                    ComponentId(102),
                    ComponentId(105),
                    |strip| {
                        strip.select(selected);
                        target = strip.selected_rect();
                    },
                )
                .unwrap();
                host.update_component::<ScrollContent>(
                    ComponentId(102),
                    ComponentId(104),
                    |scroll| scroll.reveal_horizontal(target),
                )
                .unwrap();
                let mut scene = Scene::new();
                host.render(&mut scene, &mut text, Theme::LIGHT.into())
                    .unwrap();
                let content = host.content_host();
                let node = content.get(ComponentId(105)).unwrap();
                let strip = node.component::<TabStripContent>().unwrap();
                let parts = strip.semantic_parts();
                let tab = parts.iter().find(|p| p.index == selected).unwrap().frame;
                let close = parts
                    .iter()
                    .find(|p| p.index == 3 + selected)
                    .unwrap()
                    .frame;
                let scroll = content.get(ComponentId(104)).unwrap();
                near(tab.min_y() - region_frame.min_y(), padding);
                near(region_frame.max_y() - tab.max_y(), padding);
                near(tab.size.height, row - 2. * padding);
                if selected == 0 {
                    near(tab.min_x() - region_frame.min_x(), padding);
                } else {
                    near(scroll.frame().max_x() - tab.max_x(), padding);
                }
                assert_eq!(
                    content.get(ComponentId(106)).unwrap().frame(),
                    plus,
                    "fixed sibling does not scroll"
                );
                near(close.size.height, tab.size.height);
                let label = content
                    .get(ComponentId(110 + selected as u64))
                    .unwrap()
                    .frame();
                assert!(label.min_y() >= tab.min_y() && label.max_y() <= tab.max_y());
                let focus_rect =
                    tekes_ui_controls::extended::ControlFrame::new(tab, None).focus_bounds(backing);
                assert!(
                    focus_rect.min_y() >= region_frame.min_y() - 0.002
                        && focus_rect.max_y() <= region_frame.max_y() + 0.002
                );
                assert!(
                    focus_rect.min_x() >= scroll.frame().min_x() - 0.002
                        && focus_rect.max_x() <= scroll.frame().max_x() + 0.002
                );
                let mut clips = Vec::new();
                let mut fill = false;
                let mut icon = false;
                for primitive in scene.primitives() {
                    match primitive {
                        tekes_ui_scene::Primitive::PushClip(clip) => clips.push(*clip),
                        tekes_ui_scene::Primitive::PopClip => {
                            clips.pop();
                        }
                        tekes_ui_scene::Primitive::Quad(q)
                            if (q.rect.min_x() - tab.min_x()).abs() < 0.002
                                && (q.rect.size.height - tab.size.height).abs() < 0.002 =>
                        {
                            fill = true;
                            for clip in &clips {
                                assert!(
                                    q.rect.min_y() >= clip.min_y() - 0.002
                                        && q.rect.max_y() <= clip.max_y() + 0.002
                                );
                            }
                        }
                        tekes_ui_scene::Primitive::Image(i)
                            if (i.rect.center().x - close.center().x).abs() < 0.002 =>
                        {
                            icon = true;
                            near(i.rect.size.height, 16. * scale);
                            assert!(
                                i.rect.min_y() >= tab.min_y() - 0.002
                                    && i.rect.max_y() <= tab.max_y() + 0.002
                            );
                        }
                        _ => {}
                    }
                }
                assert!(fill && icon);
                // Adjacent background distance uses the same outer spacing.
                host.update_component::<TabStripContent>(ComponentId(102), ComponentId(105), |s| {
                    s.select(0);
                    let first = s.selected_rect();
                    s.select(1);
                    let next = s.selected_rect();
                    near(next.min_x() - first.max_x(), padding);
                    s.select(selected);
                })
                .unwrap();
                host.render(&mut Scene::new(), &mut text, Theme::LIGHT.into())
                    .unwrap();
                assert!(
                    host.event(&ContentEvent::PointerDown(close.center()))
                        .consumed
                );
                host.event(&ContentEvent::PointerUp(close.center()));
                assert!(host.drain_commands().iter().any(|event| matches!(&event.payload, Some(ContentActionPayload::TabStrip(intent)) if intent.close)));
            }
        }
    }
    // A consumer can deliberately request more padding than the automatic
    // Toolbar budget; that request must survive even when it clips content.
    let mut text = ToolbarText {
        scale: 1.,
        backing: 2.,
    };
    let strip = TabStripContent::new(
        vec![TabStripItem {
            label: Content::new(ComponentId(203), TextContent::new("Inspector")),
            name: "Inspector".into(),
            closable: false,
        }],
        0,
    );
    let mut host = ContentHost::new(Content::new(
        ComponentId(200),
        ToolbarContent::new(vec![Content::new(
            ComponentId(201),
            ScrollContent::new(ScrollArea::default(), Content::new(ComponentId(202), strip))
                .horizontal(true)
                .content_padding(20.),
        )]),
    ))
    .unwrap();
    let frame = Rect::new(0., 0., 300., 42.);
    host.render(
        &mut Scene::new(),
        &mut text,
        frame,
        frame,
        Theme::LIGHT.into(),
    )
    .unwrap();
    let viewport = host.get(ComponentId(201)).unwrap();
    let child = host.get(ComponentId(202)).unwrap().frame();
    near(child.min_x() - viewport.frame().min_x(), 20.);
    near(child.min_y() - viewport.frame().min_y(), 20.);
    near(
        viewport
            .component::<ScrollContent>()
            .unwrap()
            .scroll
            .document_size()
            .height
            - child.size.height,
        40.,
    );
}
