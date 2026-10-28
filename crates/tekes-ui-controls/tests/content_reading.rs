use std::{cell::Cell, rc::Rc};
use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, ReadingUpdateError, ScrollArea,
    composition::*,
    extended::{Axis, Stack},
    layout::MeasuredChild,
};
use tekes_ui_scene::{Point, Rect, Scene, Size};
use tekes_ui_tokens::{Rgba, Theme, proposal};

const ROOT: ComponentId = ComponentId(1);
const SCROLL: ComponentId = ComponentId(2);
const ITEMS: ComponentId = ComponentId(3);
const EDITOR: ComponentId = ComponentId(4);
struct Text;
impl ButtonTextRenderer for Text {
    fn measure(&mut self, s: &str, _: proposal::TextRole, width: f32) -> Size {
        Size::new((s.len() as f32 * 7.).min(width), 16.)
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
#[derive(Debug)]
struct Row(Rc<Cell<f32>>);
impl Component for Row {
    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, size: Size) -> MeasuredChild {
        MeasuredChild::new(Size::new(size.width, self.0.get()))
    }
}
fn row(id: ComponentId, height: f32) -> Content {
    Content::new(id, Row(Rc::new(Cell::new(height))))
}
fn fixture() -> (ContentHost, Vec<ComponentId>) {
    let ids: Vec<_> = (100..120).map(ComponentId).collect();
    let host = ContentHost::new(Content::new(
        ROOT,
        StackContent {
            model: Stack {
                axis: Axis::Vertical,
                gap: 8.,
            },
            flexible_child: Some(SCROLL),
            children: vec![
                Content::new(
                    SCROLL,
                    ScrollContent::new(
                        ScrollArea::default(),
                        Content::new(
                            ITEMS,
                            ListContent::new(
                                "Mixed history",
                                ids.iter().map(|id| row(*id, 60.)).collect(),
                            ),
                        ),
                    ),
                ),
                Content::new(EDITOR, Button::new("Independent action")),
            ],
        },
    ))
    .unwrap();
    (host, ids)
}
fn render(host: &mut ContentHost) {
    let f = Rect::new(0., 0., 320., 200.);
    host.render(&mut Scene::new(), &mut Text, f, f, Theme::LIGHT.into())
        .unwrap();
}
fn scroll(host: &ContentHost) -> &ScrollContent {
    host.get(SCROLL)
        .unwrap()
        .component::<ScrollContent>()
        .unwrap()
}
fn scroll_mut(host: &mut ContentHost) -> &mut ScrollContent {
    host.get_mut(SCROLL)
        .unwrap()
        .component_mut::<ScrollContent>()
        .unwrap()
}
fn mid(host: &mut ContentHost) {
    render(host);
    scroll_mut(host).scroll.scroll_by(0., 280.);
    render(host);
}
fn anchor(host: &ContentHost) -> ComponentId {
    match scroll(host).reading_result().unwrap().anchor {
        ContentReadingAnchorUpdate::Preserved(id)
        | ContentReadingAnchorUpdate::Fallback(Some(id)) => id,
        ref other => panic!("expected item anchor: {other:?}"),
    }
}
fn prepend(host: &mut ContentHost, ids: &[ComponentId], id: ComponentId) {
    host.insert(ITEMS, row(id, 45.)).unwrap();
    let order = std::iter::once(id)
        .chain(ids.iter().copied())
        .collect::<Vec<_>>();
    host.reorder(ITEMS, &order).unwrap();
}

#[test]
fn opt_in_prepend_keeps_item_position_while_ordinary_viewport_keeps_plain_offsets() {
    let (mut legacy, ids) = fixture();
    mid(&mut legacy);
    let item = ids[4];
    let old = legacy.get(item).unwrap().frame().min_y();
    prepend(&mut legacy, &ids, ComponentId(900));
    render(&mut legacy);
    assert!((legacy.get(item).unwrap().frame().min_y() - old).abs() > 40.);

    let (mut host, ids) = fixture();
    let (version, ()) = host
        .set_scroll_reading_document(SCROLL, "A", 1, ids.clone(), |_| ())
        .unwrap();
    mid(&mut host);
    let item = anchor(&host);
    let before = host.get(item).unwrap().frame().min_y();
    host.focus(Some(EDITOR), true);
    let order = std::iter::once(ComponentId(900))
        .chain(ids.iter().copied())
        .collect();
    let (updated, ()) = host
        .update_scroll_reading_document(SCROLL, &version, 2, order, |h| {
            prepend(h, &ids, ComponentId(900))
        })
        .unwrap();
    render(&mut host);
    assert_eq!(anchor(&host), item);
    assert!((host.get(item).unwrap().frame().min_y() - before).abs() <= 1.);
    assert_eq!(scroll(&host).reading_result().unwrap().version, updated);
    assert_eq!(host.focused(), Some(EDITOR));
    assert!(!scroll(&host).is_following_latest());
}

#[test]
fn coalesced_updates_reorder_height_change_and_resize_restore_before_paint() {
    let (mut host, ids) = fixture();
    let (v1, ()) = host
        .set_scroll_reading_document(SCROLL, "A", 1, ids.clone(), |_| ())
        .unwrap();
    mid(&mut host);
    let item = anchor(&host);
    let before = host.get(item).unwrap().frame().min_y();
    let first: Vec<_> = std::iter::once(ComponentId(900))
        .chain(ids.iter().copied())
        .collect();
    let (v2, ()) = host
        .update_scroll_reading_document(SCROLL, &v1, 2, first.clone(), |h| {
            prepend(h, &ids, ComponentId(900))
        })
        .unwrap();
    let second: Vec<_> = std::iter::once(ComponentId(901))
        .chain(first.iter().copied())
        .collect();
    let (v3, ()) = host
        .update_scroll_reading_document(SCROLL, &v2, 3, second, |h| {
            prepend(h, &first, ComponentId(901));
            h.get_mut(ids[0])
                .unwrap()
                .component_mut::<Row>()
                .unwrap()
                .0
                .set(125.);
        })
        .unwrap();
    let f = Rect::new(0., 0., 260., 240.);
    host.render(&mut Scene::new(), &mut Text, f, f, Theme::DARK.into())
        .unwrap();
    assert_eq!(anchor(&host), item);
    assert!((host.get(item).unwrap().frame().min_y() - before).abs() <= 1.);
    assert_eq!(scroll(&host).reading_result().unwrap().version, v3);
    assert!(!scroll(&host).is_following_latest());
}

#[test]
fn stale_or_foreign_updates_never_enter_mutation_callback_and_mid_update_cannot_render() {
    let (mut host, ids) = fixture();
    let (v1, ()) = host
        .set_scroll_reading_document(SCROLL, "A", 1, ids.clone(), |_| ())
        .unwrap();
    render(&mut host);
    let (v2, ()) = host
        .update_scroll_reading_document(SCROLL, &v1, 2, ids.clone(), |h| {
            let f = Rect::new(0., 0., 320., 200.);
            assert_eq!(
                h.render(&mut Scene::new(), &mut Text, f, f, Theme::LIGHT.into()),
                Err(CompositionError::ReadingTransactionInProgress)
            );
        })
        .unwrap();
    let called = Cell::new(false);
    assert_eq!(
        host.update_scroll_reading_document(SCROLL, &v1, 3, ids.clone(), |_| called.set(true)),
        Err(ReadingUpdateError::StaleRevision)
    );
    let (mut other, _) = fixture();
    let (foreign, ()) = other
        .set_scroll_reading_document(SCROLL, "A", 2, ids.clone(), |_| ())
        .unwrap();
    assert_eq!(
        host.update_scroll_reading_document(SCROLL, &foreign, 3, ids.clone(), |_| called.set(true)),
        Err(ReadingUpdateError::WrongDocument)
    );
    assert_eq!(
        host.update_scroll_reading_document(SCROLL, &v2, 3, vec![ids[0], ids[0]], |_| called
            .set(true)),
        Err(ReadingUpdateError::InvalidDocument)
    );
    assert!(!called.get());
    assert_eq!(scroll(&host).reading_document_version(), Some(&v2));
}

#[test]
fn incoming_user_scroll_overrides_queued_arrival_and_never_repins_implicitly() {
    let (mut host, ids) = fixture();
    let (v1, ()) = host
        .set_scroll_reading_document(SCROLL, "A", 1, ids.clone(), |_| ())
        .unwrap();
    render(&mut host);
    assert!(scroll_mut(&mut host).follow_latest());
    render(&mut host);
    assert_eq!(
        scroll(&host).scroll.offset.y,
        scroll(&host).scroll.limit().y
    );
    let point = host.get(SCROLL).unwrap().frame().center();
    let order: Vec<_> = ids
        .iter()
        .copied()
        .chain(std::iter::once(ComponentId(900)))
        .collect();
    host.update_scroll_reading_document(SCROLL, &v1, 2, order, |h| {
        h.insert(ITEMS, row(ComponentId(900), 60.)).unwrap()
    })
    .unwrap();
    host.event(&ContentEvent::Scroll {
        position: point,
        delta: Point::new(0., -40.),
        precise: true,
        begin: true,
        end: false,
        momentum: false,
        time: 1.,
    });
    render(&mut host);
    assert!(!scroll(&host).is_following_latest());
    assert!(scroll(&host).scroll.offset.y < scroll(&host).scroll.limit().y - 40.);
    assert!(scroll(&host).scroll.needs_animation());
    assert!(scroll(&host).reading_result().unwrap().user_input_won);
    let before = scroll(&host).scroll.offset.y;
    host.event(&ContentEvent::Scroll {
        position: point,
        delta: Point::new(0., -7.),
        precise: true,
        begin: false,
        end: false,
        momentum: true,
        time: 1.01,
    });
    render(&mut host);
    assert!((scroll(&host).scroll.offset.y - (before - 7.)).abs() < 0.01);
    assert!(!scroll(&host).is_following_latest());
    scroll_mut(&mut host).follow_latest();
    render(&mut host);
    assert!(scroll(&host).is_following_latest());
    assert_eq!(
        scroll(&host).reading_result().unwrap().anchor,
        ContentReadingAnchorUpdate::FollowingLatest
    );
}

#[test]
fn deletion_uses_first_surviving_successor_and_empty_document_has_explicit_fallback() {
    let (mut host, ids) = fixture();
    let (v1, ()) = host
        .set_scroll_reading_document(SCROLL, "A", 1, ids.clone(), |_| ())
        .unwrap();
    mid(&mut host);
    let current = anchor(&host);
    let index = ids.iter().position(|id| *id == current).unwrap();
    let remaining: Vec<_> = ids.iter().copied().filter(|id| *id != current).collect();
    let (v2, ()) = host
        .update_scroll_reading_document(SCROLL, &v1, 2, remaining.clone(), |h| {
            h.remove(current).unwrap();
            h.get_mut(ids[index + 1])
                .unwrap()
                .component_mut::<Row>()
                .unwrap()
                .0
                .set(0.);
        })
        .unwrap();
    render(&mut host);
    assert_eq!(
        scroll(&host).reading_result().unwrap().anchor,
        ContentReadingAnchorUpdate::Fallback(Some(ids[index + 2]))
    );
    host.update_scroll_reading_document(SCROLL, &v2, 3, vec![], |h| {
        for id in remaining {
            h.remove(id).unwrap();
        }
    })
    .unwrap();
    render(&mut host);
    assert_eq!(
        scroll(&host).reading_result().unwrap().anchor,
        ContentReadingAnchorUpdate::Fallback(None)
    );
    assert_eq!(scroll(&host).scroll.offset, Point::default());
    assert!(!scroll(&host).is_following_latest());
}

#[test]
fn document_switch_restores_each_intent_and_rejects_old_attachment_even_when_key_returns() {
    let (mut host, ids) = fixture();
    let (a, ()) = host
        .set_scroll_reading_document(SCROLL, "A", 1, ids.clone(), |_| ())
        .unwrap();
    mid(&mut host);
    let item = anchor(&host);
    let y = host.get(item).unwrap().frame().min_y();
    let b_ids: Vec<_> = (500..520).map(ComponentId).collect();
    let (b, a_rows) = host
        .set_scroll_reading_document(SCROLL, "B", 1, b_ids.clone(), |h| {
            let retained: Vec<_> = ids.iter().map(|id| h.remove(*id).unwrap()).collect();
            for id in &b_ids {
                h.insert(ITEMS, row(*id, 60.)).unwrap();
            }
            retained
        })
        .unwrap();
    render(&mut host);
    scroll_mut(&mut host).follow_latest();
    render(&mut host);
    let (a_return, b_rows) = host
        .set_scroll_reading_document(SCROLL, "A", 1, ids.clone(), |h| {
            let retained: Vec<_> = b_ids.iter().map(|id| h.remove(*id).unwrap()).collect();
            for row in a_rows {
                h.insert(ITEMS, row).unwrap();
            }
            retained
        })
        .unwrap();
    render(&mut host);
    assert_eq!(anchor(&host), item);
    assert!((host.get(item).unwrap().frame().min_y() - y).abs() <= 1.);
    assert!(!scroll(&host).is_following_latest());
    assert_ne!(a, a_return);
    assert_eq!(
        host.update_scroll_reading_document(SCROLL, &a, 2, ids.clone(), |_| panic!(
            "stale callback"
        )),
        Err(ReadingUpdateError::WrongDocument)
    );
    assert_eq!(
        host.update_scroll_reading_document(SCROLL, &b, 2, b_ids.clone(), |_| panic!(
            "other document"
        )),
        Err(ReadingUpdateError::WrongDocument)
    );
    host.set_scroll_reading_document(SCROLL, "B", 1, b_ids.clone(), |h| {
        for id in &ids {
            h.remove(*id).unwrap();
        }
        for row in b_rows {
            h.insert(ITEMS, row).unwrap();
        }
    })
    .unwrap();
    render(&mut host);
    assert!(scroll(&host).is_following_latest());
}

#[test]
fn unmount_invalidates_pending_request_but_retained_owner_keeps_reading_intent() {
    let (mut host, ids) = fixture();
    let (old, ()) = host
        .set_scroll_reading_document(SCROLL, "A", 1, ids.clone(), |_| ())
        .unwrap();
    mid(&mut host);
    let item = anchor(&host);
    let before = host.get(item).unwrap().frame().min_y();
    let retained = host.remove(SCROLL).unwrap();
    host.insert(ROOT, retained).unwrap();
    host.reorder(ROOT, &[SCROLL, EDITOR]).unwrap();
    assert_eq!(
        host.update_scroll_reading_document(SCROLL, &old, 2, ids, |_| panic!("old mount request")),
        Err(ReadingUpdateError::WrongDocument)
    );
    render(&mut host);
    assert_eq!(anchor(&host), item);
    assert!((host.get(item).unwrap().frame().min_y() - before).abs() <= 1.);
}

#[test]
fn document_switch_and_explicit_latest_reject_the_old_momentum_tail() {
    let (mut host, ids) = fixture();
    host.set_scroll_reading_document(SCROLL, "A", 1, ids.clone(), |_| ())
        .unwrap();
    mid(&mut host);
    let p = host.get(SCROLL).unwrap().frame().center();
    host.event(&ContentEvent::Scroll {
        position: p,
        delta: Point::new(0., 10.),
        precise: true,
        begin: true,
        end: false,
        momentum: false,
        time: 1.,
    });
    host.set_scroll_reading_document(SCROLL, "B", 1, ids, |_| ())
        .unwrap();
    render(&mut host);
    let before = scroll(&host).scroll.offset;
    let old = host.event(&ContentEvent::Scroll {
        position: p,
        delta: Point::new(0., 15.),
        precise: true,
        begin: false,
        end: false,
        momentum: true,
        time: 1.01,
    });
    assert!(
        !old.consumed,
        "old document momentum cannot acquire the new document"
    );
    assert_eq!(scroll(&host).scroll.offset, before);
    host.event(&ContentEvent::Scroll {
        position: p,
        delta: Point::new(0., 5.),
        precise: true,
        begin: true,
        end: false,
        momentum: false,
        time: 2.,
    });
    scroll_mut(&mut host).follow_latest();
    render(&mut host);
    let bottom = scroll(&host).scroll.offset.y;
    host.event(&ContentEvent::Scroll {
        position: p,
        delta: Point::new(0., -10.),
        precise: true,
        begin: false,
        end: false,
        momentum: true,
        time: 2.01,
    });
    assert_eq!(scroll(&host).scroll.offset.y, bottom);
    assert!(scroll(&host).is_following_latest());
    host.event(&ContentEvent::Scroll {
        position: p,
        delta: Point::new(0., -10.),
        precise: true,
        begin: true,
        end: false,
        momentum: false,
        time: 3.,
    });
    assert_eq!(scroll(&host).scroll.offset.y, bottom - 10.);
    assert!(!scroll(&host).is_following_latest());
}

#[test]
fn fully_clipped_viewport_suspends_and_restores_the_same_reading_anchor() {
    let (mut host, ids) = fixture();
    host.set_scroll_reading_document(SCROLL, "A", 1, ids, |_| ())
        .unwrap();
    mid(&mut host);
    let item = anchor(&host);
    let y = host.get(item).unwrap().frame().min_y();
    let frame = Rect::new(0., 0., 320., 200.);
    host.render(
        &mut Scene::new(),
        &mut Text,
        frame,
        Rect::new(0., 400., 320., 10.),
        Theme::LIGHT.into(),
    )
    .unwrap();
    assert_eq!(
        scroll(&host).reading_result().unwrap().anchor,
        ContentReadingAnchorUpdate::Fallback(None)
    );
    render(&mut host);
    assert_eq!(anchor(&host), item);
    assert!((host.get(item).unwrap().frame().min_y() - y).abs() <= 1.);
    assert!(!scroll(&host).is_following_latest());
}

#[test]
fn stopped_child_reader_pauses_ancestor_following_without_redispatching_its_wheel() {
    const INNER: ComponentId = ComponentId(801);
    const BODY: ComponentId = ComponentId(802);
    let mut inner = ScrollContent::new(ScrollArea::default(), row(BODY, 1000.));
    inner.viewport_height = Some(400.);
    let mut host = ContentHost::new(Content::new(
        SCROLL,
        ScrollContent::new(ScrollArea::default(), Content::new(INNER, inner)),
    ))
    .unwrap();
    host.set_scroll_reading_document(SCROLL, "outer-stopped-reader", 1, vec![INNER], |_| ())
        .unwrap();
    host.set_scroll_reading_document(INNER, "inner-stopped-reader", 1, vec![BODY], |_| ())
        .unwrap();
    scroll_mut(&mut host).follow_latest();
    host.get_mut(INNER)
        .unwrap()
        .component_mut::<ScrollContent>()
        .unwrap()
        .follow_latest();
    render(&mut host);
    let outer_offset = scroll(&host).scroll.offset;
    let inner_offset = host
        .get(INNER)
        .unwrap()
        .component::<ScrollContent>()
        .unwrap()
        .scroll
        .offset
        .y;
    assert!(scroll(&host).is_following_latest());
    assert!(
        host.get(INNER)
            .unwrap()
            .component::<ScrollContent>()
            .unwrap()
            .is_following_latest()
    );
    let visible = host.get(INNER).unwrap().visible().unwrap();
    let response = host.event(&ContentEvent::Scroll {
        position: Point::new(visible.min_x() + 10., visible.min_y() + 10.),
        delta: Point::new(0., -16.),
        precise: true,
        begin: true,
        end: false,
        momentum: false,
        time: 1.,
    });
    assert!(response.consumed && !host.event_bubbled());
    assert!(!scroll(&host).is_following_latest());
    assert_eq!(scroll(&host).scroll.offset, outer_offset);
    let child = host
        .get(INNER)
        .unwrap()
        .component::<ScrollContent>()
        .unwrap();
    assert!(!child.is_following_latest());
    assert_eq!(inner_offset - child.scroll.offset.y, 16.);
    assert!(
        host.reading_scroll_source()
            .is_some_and(|(source, _)| source == INNER)
    );
}

#[derive(Debug)]
struct NonReadingWheelSink {
    wheels: usize,
}
impl Component for NonReadingWheelSink {
    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, size: Size) -> MeasuredChild {
        MeasuredChild::new(Size::new(size.width, 400.))
    }
    fn interactive(&self) -> bool {
        true
    }
    fn event(&mut self, event: &ContentEvent) -> ContentResponse {
        if matches!(event, ContentEvent::Scroll { .. }) {
            self.wheels += 1;
            ContentResponse::consumed(false)
        } else {
            ContentResponse::default()
        }
    }
}

#[test]
fn stopped_nonreading_child_does_not_pause_ancestor_following() {
    const SINK: ComponentId = ComponentId(803);
    let mut host = ContentHost::new(Content::new(
        SCROLL,
        ScrollContent::new(
            ScrollArea::default(),
            Content::new(SINK, NonReadingWheelSink { wheels: 0 }),
        ),
    ))
    .unwrap();
    host.set_scroll_reading_document(SCROLL, "outer-nonreader", 1, vec![SINK], |_| ())
        .unwrap();
    scroll_mut(&mut host).follow_latest();
    render(&mut host);
    let offset = scroll(&host).scroll.offset;
    let visible = host.get(SINK).unwrap().visible().unwrap();
    let response = host.event(&ContentEvent::Scroll {
        position: Point::new(visible.min_x() + 10., visible.min_y() + 10.),
        delta: Point::new(0., -16.),
        precise: true,
        begin: true,
        end: false,
        momentum: false,
        time: 1.,
    });
    assert!(response.consumed && !host.event_bubbled());
    assert_eq!(
        host.get(SINK)
            .unwrap()
            .component::<NonReadingWheelSink>()
            .unwrap()
            .wheels,
        1
    );
    assert!(scroll(&host).is_following_latest());
    assert_eq!(scroll(&host).scroll.offset, offset);
    assert!(host.reading_scroll_source().is_none());
}
