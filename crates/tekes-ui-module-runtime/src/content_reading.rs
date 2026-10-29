//! Projection onto the original reading document and item reveal transaction.
use anyhow::{Context, Result, ensure};
use tekes_ui_app_protocol::component::content as wire;
use tekes_ui_controls::{
    ButtonTextRenderer, Insets, ReadingDocumentVersion, ScrollRevealOptions, ScrollRevealPlacement,
    composition::{ComponentId, ContentHost, ContentRevealStatus, ScrollContent},
    extended::Axis,
};
fn version(value: &ReadingDocumentVersion) -> wire::ReadingVersion {
    wire::ReadingVersion {
        key: value.key().into(),
        revision: value.revision(),
        attachment: value.attachment(),
    }
}
pub(crate) fn read_only_state(host: &ContentHost, reader: u64) -> Result<wire::ReadingState> {
    let id = ComponentId(reader);
    let mount = host
        .semantic_mount(id)
        .context("unavailable Content reading owner")?;
    let scroll = host
        .get(id)
        .and_then(|node| node.component::<ScrollContent>())
        .context("Content owner is not an original ScrollContent")?;
    let document = scroll
        .reading_document_version()
        .context("unversioned Content reader")?;
    let navigation = host
        .scroll_reading_reveal_result()
        .filter(|result| result.owner == id && result.version == *document)
        .map(|result| wire::ReadingNavigation {
            item: result.item.0,
            changed: result.changed,
            clamped: result.clamped,
            status: match result.status {
                ContentRevealStatus::Animating => wire::RevealStatus::Animating,
                ContentRevealStatus::Completed => wire::RevealStatus::Completed,
                ContentRevealStatus::Interrupted => wire::RevealStatus::Interrupted,
                ContentRevealStatus::Rejected(error) => {
                    wire::RevealStatus::Rejected(format!("{error:?}"))
                }
            },
        });
    Ok(wire::ReadingState {
        reader: wire::Target { id: reader, mount },
        document: version(document),
        items: scroll
            .reading_items()
            .iter()
            .filter_map(|id| {
                host.semantic_mount(*id)
                    .map(|mount| wire::Target { id: id.0, mount })
            })
            .collect(),
        following_latest: scroll.is_following_latest(),
        navigation,
    })
}
pub(crate) fn dispatch(
    host: &mut ContentHost,
    text: &mut dyn ButtonTextRenderer,
    input: &wire::Input,
) -> Result<Option<wire::ReadingState>> {
    match input {
        wire::Input::ReadingState { reader } => read_only_state(host, *reader).map(Some),
        wire::Input::RevealReadingItem {
            reader,
            document,
            item,
            options,
        } => {
            ensure!(options.is_valid(), "invalid Content reveal options");
            let current = read_only_state(host, reader.id)?;
            ensure!(current.reader == *reader, "stale Content reading mount");
            ensure!(
                current.document == *document,
                "stale Content reading document"
            );
            ensure!(
                current.items.contains(item),
                "stale or undeclared Content reading item"
            );
            let expected = host
                .get(ComponentId(reader.id))
                .and_then(|node| node.component::<ScrollContent>())
                .and_then(ScrollContent::reading_document_version)
                .context("unavailable Content reader")?
                .clone();
            let placement = match options.placement {
                wire::RevealPlacement::Nearest => ScrollRevealPlacement::Nearest,
                wire::RevealPlacement::Centered => ScrollRevealPlacement::Centered,
                wire::RevealPlacement::StartVertical => {
                    ScrollRevealPlacement::Start(Axis::Vertical)
                }
                wire::RevealPlacement::StartHorizontal => {
                    ScrollRevealPlacement::Start(Axis::Horizontal)
                }
            };
            let [top, right, bottom, left] = options.margins;
            host.reveal_scroll_reading_item(
                ComponentId(reader.id),
                &expected,
                ComponentId(item.id),
                ScrollRevealOptions {
                    placement,
                    margins: Insets::new(top, right, bottom, left),
                    animated: options.animated,
                },
                text,
            )
            .map_err(|error| anyhow::anyhow!("Content reveal rejected: {error:?}"))?;
            read_only_state(host, reader.id).map(Some)
        }
        _ => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tekes_ui_controls::{
        ButtonTextAlign, ScrollArea,
        composition::*,
        extended::{Axis, Stack},
        layout::MeasuredChild,
    };
    use tekes_ui_scene::{Rect, Scene, Size};
    use tekes_ui_tokens::{Rgba, Theme, proposal};
    struct Text;
    impl ButtonTextRenderer for Text {
        fn measure(&mut self, _: &str, _: proposal::TextRole, _: f32) -> Size {
            Size::new(100., 80.)
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
    struct Row;
    impl Component for Row {
        fn measure(&mut self, _: &mut dyn ButtonTextRenderer, a: Size) -> MeasuredChild {
            MeasuredChild::new(Size::new(a.width, 80.))
        }
    }
    fn setup() -> ContentHost {
        let mut host = ContentHost::new(Content::new(
            ComponentId(1),
            ScrollContent::new(
                ScrollArea::default(),
                Content::new(
                    ComponentId(2),
                    StackContent {
                        model: Stack {
                            axis: Axis::Vertical,
                            gap: 0.,
                        },
                        flexible_child: None,
                        children: vec![
                            Content::new(ComponentId(10), Row),
                            Content::new(ComponentId(11), Row),
                        ],
                    },
                ),
            ),
        ))
        .unwrap();
        host.set_scroll_reading_document(
            ComponentId(1),
            "reader",
            1,
            vec![ComponentId(10), ComponentId(11)],
            |_| (),
        )
        .unwrap();
        render(&mut host);
        host
    }
    fn render(host: &mut ContentHost) {
        let frame = Rect::new(0., 0., 200., 80.);
        host.render(
            &mut Scene::new(),
            &mut Text,
            frame,
            frame,
            Theme::LIGHT.into(),
        )
        .unwrap();
    }
    fn request(host: &ContentHost, input: wire::Input) -> wire::Request {
        wire::Request {
            view: 1,
            viewport: 1,
            owner: None,
            focus_epoch: host.focus_epoch(),
            text_binding: None,
            capture: None,
            input,
        }
    }
    fn options() -> wire::RevealOptions {
        wire::RevealOptions {
            placement: wire::RevealPlacement::Nearest,
            margins: [0.; 4],
            animated: false,
        }
    }
    #[test]
    fn query_is_read_only_and_reveal_uses_original_item_transaction() {
        let mut host = setup();
        let epoch = host.focus_epoch();
        let query = request(&host, wire::Input::ReadingState { reader: 1 });
        let reply = crate::content_input::dispatch(&mut host, &mut Text, query).unwrap();
        assert!(!reply.changed);
        assert!(!reply.consumed);
        let state = reply.reading.unwrap();
        assert_eq!(state.items.len(), 2);
        assert_eq!(host.focus_epoch(), epoch);
        let input = wire::Input::RevealReadingItem {
            reader: state.reader,
            document: state.document.clone(),
            item: state.items[1],
            options: options(),
        };
        let req = request(&host, input);
        let reply = crate::content_input::dispatch(&mut host, &mut Text, req).unwrap();
        assert!(reply.changed);
        assert_eq!(host.focus_epoch(), epoch);
        assert!(host.focused().is_none());
        let mut direct = setup();
        let original = direct
            .get(ComponentId(1))
            .unwrap()
            .component::<ScrollContent>()
            .unwrap()
            .reading_document_version()
            .unwrap()
            .clone();
        direct
            .reveal_scroll_reading_item(
                ComponentId(1),
                &original,
                ComponentId(11),
                ScrollRevealOptions {
                    animated: false,
                    ..Default::default()
                },
                &mut Text,
            )
            .unwrap();
        let original_offset = direct
            .get(ComponentId(1))
            .unwrap()
            .component::<ScrollContent>()
            .unwrap()
            .scroll
            .offset;
        assert!(original_offset.y > 0.);
        assert_eq!(
            host.get(ComponentId(1))
                .unwrap()
                .component::<ScrollContent>()
                .unwrap()
                .scroll
                .offset,
            original_offset
        );
        assert_eq!(
            reply.reading.unwrap().navigation.unwrap().status,
            wire::RevealStatus::Completed
        );
        // Reading queries report no new invalidation, even with paint pending.
        let req = request(&host, wire::Input::ReadingState { reader: 1 });
        let reply = crate::content_input::dispatch(&mut host, &mut Text, req).unwrap();
        assert!(!reply.changed);
        assert!(host.needs_render());
    }
    #[test]
    fn inactive_window_cannot_execute_retained_reveal() {
        let mut host = setup();
        let state = read_only_state(&host, 1).unwrap();
        host.set_window_active(false);
        let req = request(
            &host,
            wire::Input::RevealReadingItem {
                reader: state.reader,
                document: state.document,
                item: state.items[1],
                options: options(),
            },
        );
        assert!(crate::content_input::dispatch(&mut host, &mut Text, req).is_err());
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
    fn invalid_document_mount_and_margins_never_move_original_reader() {
        let mut host = setup();
        let state = read_only_state(&host, 1).unwrap();
        let base = wire::Input::RevealReadingItem {
            reader: state.reader,
            document: state.document.clone(),
            item: state.items[1],
            options: options(),
        };
        let mut invalids = vec![];
        let mut value = base.clone();
        if let wire::Input::RevealReadingItem { document, .. } = &mut value {
            document.attachment += 1;
        }
        invalids.push(value);
        let mut value = base.clone();
        if let wire::Input::RevealReadingItem { document, .. } = &mut value {
            document.revision += 1;
        }
        invalids.push(value);
        let mut value = base.clone();
        if let wire::Input::RevealReadingItem { reader, .. } = &mut value {
            reader.mount += 1;
        }
        invalids.push(value);
        let mut value = base.clone();
        if let wire::Input::RevealReadingItem { item, .. } = &mut value {
            item.mount += 1;
        }
        invalids.push(value);
        for margin in [f32::NAN, f32::INFINITY, -1., 1000.] {
            let mut value = base.clone();
            if let wire::Input::RevealReadingItem { options, .. } = &mut value {
                options.margins = [margin; 4];
            }
            invalids.push(value);
        }
        for input in invalids {
            let req = request(&host, input);
            assert!(crate::content_input::dispatch(&mut host, &mut Text, req).is_err());
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
        // Same IDs and revision reattached are a new document identity.
        host.set_scroll_reading_document(
            ComponentId(1),
            "reader",
            1,
            vec![ComponentId(10), ComponentId(11)],
            |_| (),
        )
        .unwrap();
        render(&mut host);
        assert_ne!(
            state.document.attachment,
            super::read_only_state(&host, 1)
                .unwrap()
                .document
                .attachment
        );
        let req = request(&host, base);
        assert!(crate::content_input::dispatch(&mut host, &mut Text, req).is_err());
    }
}
