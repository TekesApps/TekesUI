use super::*;

#[cfg(test)]
mod shared_collection_seam_tests {
    use super::*;
    use std::{cell::RefCell, rc::Rc};
    #[derive(Debug)]
    struct Passive {
        ordinal: u8,
        log: Rc<RefCell<Vec<(u8, &'static str)>>>,
        children: Vec<Content>,
    }
    impl Component for Passive {
        fn measure(&mut self, _: &mut dyn ButtonTextRenderer, _: Size) -> MeasuredChild {
            MeasuredChild::new(Size::default())
        }
        fn children(&self) -> &[Content] {
            &self.children
        }
        fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
            Some(&mut self.children)
        }
        fn has_stable_hooks(&self) -> bool {
            true
        }
        fn event(&mut self, event: &ContentEvent) -> ContentResponse {
            if matches!(event, ContentEvent::Cancel) {
                self.log.borrow_mut().push((self.ordinal, "cancel"));
            }
            ContentResponse::default()
        }
        fn set_focus(&mut self, focused: bool, _: bool) {
            assert!(!focused);
            self.log.borrow_mut().push((self.ordinal, "blur"));
        }
        fn lifecycle(&mut self, event: Lifecycle) {
            self.log.borrow_mut().push((
                self.ordinal,
                match event {
                    Lifecycle::Mounted => "mounted",
                    Lifecycle::Unmounted => "unmounted",
                },
            ));
        }
    }
    #[test]
    fn reserved_passive_lifecycle_consumes_only_staged_mounts_in_original_hook_order() {
        let log = Rc::new(RefCell::new(vec![]));
        let child = Content::new(
            ComponentId(2),
            Passive {
                ordinal: 1,
                log: log.clone(),
                children: vec![],
            },
        );
        let mut root = Content::new(
            ComponentId(1),
            Passive {
                ordinal: 0,
                log: log.clone(),
                children: vec![child],
            },
        );
        let old = root.mount;
        let mut mounts = actions::reserve_mounts(2).unwrap();
        root.lifecycle_reserved(Lifecycle::Unmounted, &mut mounts);
        assert!(mounts.exhausted());
        assert_ne!(root.mount, old);
        assert_eq!(
            *log.borrow(),
            [
                (0, "cancel"),
                (0, "blur"),
                (0, "unmounted"),
                (1, "cancel"),
                (1, "blur"),
                (1, "unmounted")
            ]
        );
    }
    #[test]
    fn decorative_semantics_suppression_preserves_mounts_and_active_siblings() {
        let mut root = Content::new(
            ComponentId(1),
            ListContent::new(
                "group",
                vec![
                    Content::new(ComponentId(2), TextContent::new("decorative")),
                    Content::new(ComponentId(3), TextContent::new("inactive panel")),
                    Content::new(ComponentId(4), TextContent::new("active panel")),
                ],
            ),
        );
        let mount = root.find(ComponentId(2)).unwrap().mount;
        root.find_mut(ComponentId(2)).unwrap().suppress_semantics();
        root.find_mut(ComponentId(3))
            .unwrap()
            .set_composition_active(false);
        let mut nodes = vec![];
        root.semantic_nodes(None, None, &mut nodes);
        assert_eq!(
            nodes.iter().map(|n| n.id).collect::<Vec<_>>(),
            [ComponentId(1), ComponentId(4)]
        );
        assert_eq!(root.find(ComponentId(2)).unwrap().mount, mount);
        root.find_mut(ComponentId(3))
            .unwrap()
            .set_composition_active(true);
        nodes.clear();
        root.semantic_nodes(None, None, &mut nodes);
        assert_eq!(
            nodes.iter().map(|n| n.id).collect::<Vec<_>>(),
            [ComponentId(1), ComponentId(3), ComponentId(4)]
        );
    }
}

#[cfg(test)]
mod list_scrollbar_hover_tests {
    use super::*;

    #[derive(Debug)]
    struct TallItems;
    impl Component for TallItems {
        fn measure(&mut self, _: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
            MeasuredChild::new(Size::new(available.width, 320.))
        }
    }

    struct Text;
    impl ButtonTextRenderer for Text {
        fn measure(&mut self, _: &str, _: proposal::TextRole, width: f32) -> Size {
            Size::new(width, 16.)
        }
        fn draw(
            &mut self,
            _: &mut Scene,
            _: &str,
            _: proposal::TextRole,
            _: Rect,
            _: ButtonTextAlign,
            _: tekes_ui_tokens::Rgba,
        ) {
        }
    }

    const ROOT: ComponentId = ComponentId(8800);
    const ITEMS: ComponentId = ComponentId(8801);
    const ROW: ComponentId = ComponentId(8802);
    const FRAME: Rect = Rect::new(0., 0., 120., 80.);

    #[test]
    fn scroll_content_minimum_width_preserves_requested_document_width_after_layout() {
        for padding in [0., tekes_ui_tokens::Spacing::Loose.value()] {
            for scrollbar in [false, true] {
                for height in [80., 600.] {
                    let scroll = ScrollContent::new(
                        crate::ScrollArea::default(),
                        Content::new(ROW, TallItems),
                    )
                    .content_padding(padding)
                    .scrollbar(scrollbar);
                    let width = scroll.outer_width_for_content(&mut Text, 120., height < 320.);
                    let mut host = ContentHost::new(Content::new(ROOT, scroll)).unwrap();
                    let frame = Rect::new(20., 30., width, height);
                    host.render(
                        &mut Scene::new(),
                        &mut Text,
                        frame,
                        frame,
                        Palette::from(tekes_ui_tokens::Theme::LIGHT),
                    )
                    .unwrap();
                    assert_eq!(host.get(ROW).unwrap().frame().size.width, 120.);
                    assert!(host.get(ROW).unwrap().visible().is_some());
                }
            }
        }
    }
    fn list(show_scrollbar: bool) -> ContentHost {
        let mut scroll = ListContent::scrollable("List", ITEMS, vec![Content::new(ROW, TallItems)]);
        if !show_scrollbar {
            scroll = scroll.scrollbar(false);
        }
        let mut host = ContentHost::new(Content::new(ROOT, scroll)).unwrap();
        host.set_action_events(true);
        host
    }

    fn render(host: &mut ContentHost) -> Scene {
        let mut scene = Scene::new();
        host.render(
            &mut scene,
            &mut Text,
            FRAME,
            FRAME,
            Palette::from(tekes_ui_tokens::Theme::LIGHT),
        )
        .unwrap();
        scene
    }

    fn scrollbar_quads(scene: &Scene) -> usize {
        scene
            .primitives()
            .iter()
            .filter(|primitive| matches!(primitive, tekes_ui_scene::Primitive::Quad(_)))
            .count()
    }

    fn point_in_items(host: &ContentHost) -> Point {
        let frame = host.get(ROW).unwrap().frame();
        Point::new(frame.min_x() + 2., frame.min_y() + 2.)
    }

    #[test]
    fn list_scrollbar_shows_on_host_pointer_hover_and_hides_after_leave() {
        let mut host = list(true);
        assert_eq!(scrollbar_quads(&render(&mut host)), 0);
        let track = host
            .get(ROOT)
            .unwrap()
            .component::<ScrollContent>()
            .unwrap()
            .scrollbar_track(true);
        assert!(
            !host
                .event(&ContentEvent::PointerDown(track.center()))
                .consumed
        );
        let inside = point_in_items(&host);
        assert!(host.event(&ContentEvent::PointerMove(inside)).changed);
        assert_eq!(scrollbar_quads(&render(&mut host)), 1);
        assert!(
            host.event(&ContentEvent::PointerMove(Point::new(500., 500.)))
                .changed
        );
        assert_eq!(scrollbar_quads(&render(&mut host)), 0);
    }

    #[test]
    fn hidden_list_scrollbar_stays_hidden_even_when_pointer_hovers() {
        let mut host = list(false);
        assert_eq!(scrollbar_quads(&render(&mut host)), 0);
        let inside = point_in_items(&host);
        assert!(host.event(&ContentEvent::PointerMove(inside)).changed);
        assert_eq!(scrollbar_quads(&render(&mut host)), 0);
        let track = host
            .get(ROOT)
            .unwrap()
            .component::<ScrollContent>()
            .unwrap()
            .scrollbar_track(true);
        assert!(
            !host
                .event(&ContentEvent::PointerDown(track.center()))
                .consumed
        );
    }

    #[test]
    fn list_scrollbar_stays_visible_for_captured_drag_outside_until_release() {
        let mut host = list(true);
        let _ = render(&mut host);
        let inside = point_in_items(&host);
        host.event(&ContentEvent::PointerMove(inside));
        let _ = render(&mut host);
        let thumb = {
            let scroll = host
                .get(ROOT)
                .unwrap()
                .component::<ScrollContent>()
                .unwrap();
            scroll.scroll.thumb(
                true,
                scroll.scrollbar_track(true),
                ScrollContent::minimum_thumb(),
            )
        };
        let down = host.event(&ContentEvent::PointerDown(thumb.center()));
        assert!(down.capture_pointer);
        assert_eq!(host.pointer_capture().unwrap().0, ROOT);
        let outside = Point::new(500., 500.);
        host.event(&ContentEvent::PointerMove(outside));
        assert_eq!(scrollbar_quads(&render(&mut host)), 1);
        host.event(&ContentEvent::PointerMove(inside));
        host.event(&ContentEvent::PointerUp(outside));
        assert!(host.pointer_capture().is_none());
        assert_eq!(scrollbar_quads(&render(&mut host)), 0);
    }

    #[test]
    fn scroll_prepare_action_is_pure() {
        let mut host = list(true);
        let _ = render(&mut host);
        let thumb = {
            let scroll = host
                .get(ROOT)
                .unwrap()
                .component::<ScrollContent>()
                .unwrap();
            scroll.scroll.thumb(
                true,
                scroll.scrollbar_track(true),
                ScrollContent::minimum_thumb(),
            )
        };
        let scroll = host
            .get(ROOT)
            .unwrap()
            .component::<ScrollContent>()
            .unwrap();
        let before = format!("{scroll:?}");
        let event = ContentEvent::PointerDown(thumb.center());
        assert!(matches!(
            precheck_dispatch::prepare_action(scroll, ComponentActionRequest::PointerDown(&event)),
            ActionPreparation::Ready(_)
        ));
        assert_eq!(format!("{scroll:?}"), before, "preparing a drag is pure");
    }
}
