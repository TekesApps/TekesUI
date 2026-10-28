//! Independent public consumer: original ContentHost owns all child measurement/layout.
use tekes_ui_controls::{
    Insets, TextSurfacePresentation,
    composition::*,
    extended::{Axis, Stack},
};
use tekes_ui_scene::{Point, Rect, Scene};
use tekes_ui_tokens::Theme;
#[path = "support/coretext.rs"]
mod coretext;
use coretext::Text;

#[path = "support/panel_fixture.rs"]
mod panel_fixture;
use panel_fixture::{
    ACTION, BODY, EDITOR, LABEL, ROOT, ROWS, SCROLL,
};

const NEXT_ROW: ComponentId = ComponentId(8208);
const LATEST_ROW: ComponentId = ComponentId(8209);
const AFTER_LATEST_ROW: ComponentId = ComponentId(8228);

fn render(host: &mut ContentHost, text: &mut Text, width: f32, height: f32, theme: Theme) {
    let bounds = Rect::new(0., 0., width, height);
    host.render(&mut Scene::new(), text, bounds, bounds, theme.into())
        .expect("original ContentHost arranges and paints the public tree");
}

fn stack(children: Vec<Content>, flexible_child: Option<ComponentId>) -> Content {
    Content::new(BODY, StackContent {
        model: Stack {
            axis: Axis::Vertical,
            gap: 8.,
        },
        children,
        // TextViewport::Fill must be measured against the space remaining after
        // fixed siblings so the later original ScrollContent stays visible.
        flexible_child,
    })
}

fn panel(tree: Content, zero_inset: bool) -> PanelContent {
    let mut panel =
        PanelContent::new(tree).expect("public PanelContent accepts this original subtree");
    if zero_inset {
        panel.set_padding(Some(Insets::default())).unwrap();
    }
    panel
}

fn editor_layout(host: &ContentHost) -> ContentTextLayout {
    host.semantic_text_layout_snapshot(EDITOR, None)
        .expect("actual original editor paragraph geometry is published")
}

fn reader(host: &ContentHost) -> &ScrollContent {
    host.get(SCROLL)
        .unwrap()
        .component::<ScrollContent>()
        .unwrap()
}

fn preserved_anchor(result: &ContentReadingResult) -> ComponentId {
    match &result.anchor {
        ContentReadingAnchorUpdate::Preserved(id) => *id,
        other => panic!("expected original history anchor to survive; got {other:?}"),
    }
}

/// Failure-only report of the original arranged Host nodes; it does not assign
/// child frames or synthesize a second layout.
fn layout_diagnostic(host: &ContentHost, ids: &[ComponentId]) -> String {
    let snapshot = host.geometry_snapshot();
    let mut report = format!(
        "geometry_snapshot_current={:?}; focused={:?}; focus_result(targets)={:?}",
        snapshot.as_ref().map(|s| s.is_current()),
        host.focused(),
        ids.iter()
            .map(|id| (*id, host.required_focus_result(*id)))
            .collect::<Vec<_>>()
    );
    for id in ids {
        match host.get(*id) {
            Some(node) => {
                let panel = node
                    .component::<PanelContent>()
                    .and_then(PanelContent::geometry);
                let text_surface = host.text_surface(*id).map(|surface| {
                    (
                        surface.presentation,
                        surface.outer,
                        surface.inner,
                        surface.input.text.len(),
                        surface.paragraph.as_ref().map(|p| p.lines.len()),
                    )
                });
                let text_layout = host.semantic_text_layout_snapshot(*id, None).map(|layout| {
                    (
                        layout.multiline,
                        layout.input.text.len(),
                        layout.lines.len(),
                        layout.carets.len(),
                    )
                });
                let scroll = node.component::<ScrollContent>().map(|owner| {
                    (
                        owner.viewport_height,
                        owner.scroll.viewport_size(),
                        owner.scroll.document_size(),
                        owner.scroll.limit(),
                        owner.scroll.offset,
                    )
                });
                let hit = snapshot
                    .as_ref()
                    .filter(|s| s.is_current())
                    .map(|s| s.hit_test(node.frame().center()));
                report.push_str(&format!(
                    "\nnode={id:?} mount={} frame={:?} visible={:?} panel_geometry={panel:?}\
  text_surface={text_surface:?} text_layout={text_layout:?} scroll={scroll:?} hit_at_frame_center={hit:?}",
                    node.mount_identity(), node.frame(), node.visible(),
                ));
            }
            None => report.push_str(&format!("\nnode={id:?} missing_from_original_host")),
        }
    }
    report
}

#[test]
fn original_panel_stack_preserves_real_editor_geometry_actions_and_reading_intent() {
    let composer = panel_fixture::composer();
    assert_eq!(
        composer.surface.presentation,
        TextSurfacePresentation::Field
    );

    let row_ids = panel_fixture::row_ids();
    let content = panel_fixture::panel_body(composer, &row_ids);
    let mut host = ContentHost::new(Content::new(ROOT, panel(content, false))).unwrap();
    host.set_action_events(true);
    let (version1, ()) = host
        .set_scroll_reading_document(SCROLL, "issue82-main-row", 1, row_ids.clone(), |_| {})
        .unwrap();
    let mut text = Text::new(1.0);
    assert!(
        text.services,
        "public TextSurface services must be enabled in this consumer"
    );

    // The original Panel/Stack/Scroll own every child frame; only the root viewport is supplied.
    render(&mut host, &mut text, 360., 360., Theme::LIGHT);
    let mounts: Vec<_> = [LABEL, ACTION, EDITOR, SCROLL]
        .into_iter()
        .chain(row_ids.iter().copied())
        .map(|id| (id, host.semantic_mount(id).expect("original mounted child")))
        .collect();
    let first_layout = editor_layout(&host);
    let first_surface = host.text_surface(EDITOR).unwrap();
    assert_eq!(first_surface.shaped, first_surface.input.text);
    assert!(
        first_surface
            .paragraph
            .as_ref()
            .is_some_and(|p| !p.lines.is_empty())
    );
    assert!(first_surface.inner.size.width > 0. && first_surface.inner.size.height > 0.);
    assert!(!first_layout.lines.is_empty() && !first_layout.carets.is_empty());
    assert!(
        !text.surface_shape_calls.is_empty(),
        "CoreText TextSurfaceHost was used to shape the mounted editor"
    );
    assert!(
        text.surface_shape_calls
            .iter()
            .any(|(_, width)| width.is_some_and(|w| w > 0.))
    );

    let button_press = host.accessibility_action(ACTION, None, AccessibilityAction::Press);
    assert!(
        button_press.activated,
        "original public Button is actionable"
    );
    let delivered = host.drain_action_records();
    assert_eq!(delivered.len(), 1);
    assert_eq!(delivered[0].action.target, ACTION);
    assert_eq!(delivered[0].action.kind, ContentActionKind::Activate);
    assert_eq!(
        delivered[0].action.source,
        ContentActionSource::Accessibility
    );
    assert!(
        !delivered
            .iter()
            .any(|a| a.action.kind == ContentActionKind::Submit)
    );

    // Use the original ScrollContent node's arranged visible frame for event targeting.
    assert_eq!(reader(&host).viewport_height, Some(68.));
    assert!(reader(&host).scroll.viewport_size().height > 0.);
    assert!(
        reader(&host).scroll.limit().y > 0.,
        "original rows must overflow the measured ScrollContent viewport"
    );
    assert!(!reader(&host).is_following_latest());
    let original_viewport = host.get(SCROLL).unwrap().visible().unwrap_or_else(|| {
        panic!(
            "original ScrollContent has no visible arranged viewport; frames, inherited clips, actual text geometry, and hit candidates follow:\n{}",
            layout_diagnostic(&host, &[ROOT, BODY, LABEL, ACTION, EDITOR, SCROLL, ROWS])
        )
    });
    assert!(original_viewport.size.height > 0.);
    let wheel_position = original_viewport.center();
    assert!(original_viewport.contains(wheel_position));
    let scroll_mount = host.semantic_mount(SCROLL).unwrap();
    // ContentEvent::Scroll uses document-space delta: positive Y moves toward later rows.
    let wheel = host.event(&ContentEvent::Scroll {
        position: wheel_position,
        delta: Point::new(0., 54.),
        precise: true,
        begin: true,
        end: true,
        momentum: false,
        time: 1.,
    });
    assert!(
        wheel.consumed && wheel.changed,
        "original ScrollContent accepts a real user-history move: response={wheel:?}, viewport={original_viewport:?}, point={wheel_position:?}, limit={:?}, reader_source={:?}",
        reader(&host).scroll.limit(),
        host.reading_scroll_source()
    );
    assert_eq!(
        host.reading_scroll_source(),
        Some((SCROLL, scroll_mount)),
        "event must hit the original mounted ScrollContent through ContentHost routing"
    );
    assert!(reader(&host).scroll.offset.y > 0.);
    render(&mut host, &mut text, 360., 360., Theme::LIGHT);
    let history_result = reader(&host)
        .reading_result()
        .expect("completed original reading layout")
        .clone();
    let history_anchor = preserved_anchor(&history_result);
    assert!(reader(&host).scroll.offset.y > 0.);
    assert!(history_result.user_input_won);
    assert!(!history_result.clamped);
    assert!(row_ids.contains(&history_anchor));
    assert_eq!(history_result.version, version1);
    let history_mount = host.semantic_mount(history_anchor).unwrap();

    assert!(host.focus(Some(EDITOR), true));
    host.event_with_text(
        &ContentEvent::Preedit {
            text: "中文組字".into(),
            selection: 0..2,
            clauses: std::iter::once(0..4).collect(),
            replacement: None,
        },
        &mut text,
    )
    .unwrap();
    let before = host.text_input_snapshot().unwrap();
    let native_binding = host.native_key_binding().unwrap();
    assert!(before.marked.is_some());
    assert_eq!(host.focused(), Some(EDITOR));

    // Width, backing scale, and type scale change through the public render input only.
    text.backing = 1.5;
    text.scale = 1.15;
    render(&mut host, &mut text, 248., 300., Theme::DARK);
    let after = host.text_input_snapshot().unwrap();
    assert_eq!(after.binding, before.binding);
    assert_eq!(after.text, before.text);
    assert_eq!(after.marked, before.marked);
    assert_eq!(after.selection, before.selection);
    assert_eq!(host.native_key_binding().as_ref(), Some(&native_binding));
    assert_eq!(host.focused(), Some(EDITOR));
    let resized_layout = editor_layout(&host);
    assert!(!resized_layout.lines.is_empty() && !resized_layout.carets.is_empty());
    assert!(host.text_surface(EDITOR).unwrap().inner.size.width > 0.);
    assert_ne!(
        first_layout.input, resized_layout.input,
        "changed layout/type scale must update original text geometry"
    );
    for (id, mount) in &mounts {
        assert_eq!(
            host.semantic_mount(*id),
            Some(*mount),
            "mounted identity changed for {id:?}"
        );
    }
    assert_eq!(host.semantic_mount(history_anchor), Some(history_mount));

    let stale = host.geometry_snapshot().unwrap();
    let anchor_geometry_before = host.get(history_anchor).unwrap().frame();
    let viewport_geometry_before = host.get(SCROLL).unwrap().frame();
    let anchor_y_in_viewport_before =
        anchor_geometry_before.min_y() - viewport_geometry_before.min_y();
    let (version2, ()) = host
        .update_scroll_reading_document(
            SCROLL,
            &version1,
            2,
            row_ids.iter().copied().chain([NEXT_ROW]).collect(),
            |host| {
                host.insert(
                    ROWS,
                    Content::new(NEXT_ROW, TextContent::new("流式追加的原始阅读行")),
                )
                .unwrap();
            },
        )
        .unwrap();
    assert!(!stale.is_current());
    render(&mut host, &mut text, 248., 300., Theme::DARK);
    let preserved = reader(&host).reading_result().unwrap();
    assert_eq!(preserved.version, version2);
    // user_input_won reports arbitration in this completed layout, not a sticky
    // history flag: the real wheel was recorded in version1, then record()
    // consumed that transition before this pure document append.
    assert!(!preserved.user_input_won);
    assert!(!preserved.clamped);
    assert_eq!(preserved_anchor(preserved), history_anchor);
    assert_eq!(reader(&host).reading_items().last(), Some(&NEXT_ROW));
    assert!(!reader(&host).is_following_latest());
    let anchor_geometry_after = host.get(history_anchor).unwrap().frame();
    let viewport_geometry_after = host.get(SCROLL).unwrap().frame();
    let anchor_y_in_viewport_after =
        anchor_geometry_after.min_y() - viewport_geometry_after.min_y();
    assert!(
        (anchor_y_in_viewport_after - anchor_y_in_viewport_before).abs() <= 1.,
        "same-width append must retain the original anchor's relative viewport position"
    );
    assert_eq!(host.semantic_mount(history_anchor), Some(history_mount));
    assert_eq!(host.focused(), Some(EDITOR));
    let streamed = host.text_input_snapshot().unwrap();
    assert_eq!(streamed.text, before.text);
    assert_eq!(streamed.marked, before.marked);
    assert_eq!(streamed.selection, before.selection);
    assert_eq!(host.native_key_binding().as_ref(), Some(&native_binding));
    for (id, mount) in &mounts {
        assert_eq!(
            host.semantic_mount(*id),
            Some(*mount),
            "stream update changed mounted identity for {id:?}"
        );
    }

    // Explicitly opt into latest, then append again; the original Scroll owner must keep that intent.
    assert!(
        host.get_mut(SCROLL)
            .unwrap()
            .component_mut::<ScrollContent>()
            .unwrap()
            .follow_latest()
    );
    render(&mut host, &mut text, 248., 300., Theme::DARK);
    assert!(reader(&host).is_following_latest());
    let latest_result = reader(&host).reading_result().unwrap().clone();
    assert_eq!(latest_result.version, version2);
    assert_eq!(
        latest_result.anchor,
        ContentReadingAnchorUpdate::FollowingLatest
    );
    assert!(!latest_result.clamped);
    assert!(!latest_result.user_input_won);
    let reader_mount = mounts.iter().find(|(id, _)| *id == SCROLL).unwrap().1;
    let (version3, ()) = host
        .update_scroll_reading_document(
            SCROLL,
            &version2,
            3,
            row_ids
                .iter()
                .copied()
                .chain([NEXT_ROW, LATEST_ROW])
                .collect(),
            |host| {
                host.insert(
                    ROWS,
                    Content::new(LATEST_ROW, TextContent::new("follow latest追加行")),
                )
                .unwrap();
            },
        )
        .unwrap();
    render(&mut host, &mut text, 248., 300., Theme::DARK);
    let latest_append_result = reader(&host).reading_result().unwrap();
    assert_eq!(latest_append_result.version, version3);
    assert_eq!(
        latest_append_result.anchor,
        ContentReadingAnchorUpdate::FollowingLatest
    );
    assert!(!latest_append_result.clamped);
    assert!(!latest_append_result.user_input_won);
    assert!(reader(&host).is_following_latest());
    assert_eq!(reader(&host).reading_items().last(), Some(&LATEST_ROW));
    let latest_mount = host.semantic_mount(LATEST_ROW).unwrap();
    assert_eq!(host.semantic_mount(SCROLL), Some(reader_mount));
    let editor_mount = mounts.iter().find(|(id, _)| *id == EDITOR).unwrap().1;
    let history_anchor_mount = history_mount;
    assert_eq!(host.semantic_mount(EDITOR), Some(editor_mount));
    assert_eq!(
        host.semantic_mount(history_anchor),
        Some(history_anchor_mount)
    );
    let (version4, ()) = host
        .update_scroll_reading_document(
            SCROLL,
            &version3,
            4,
            row_ids
                .iter()
                .copied()
                .chain([NEXT_ROW, LATEST_ROW, AFTER_LATEST_ROW])
                .collect(),
            |host| {
                host.insert(
                    ROWS,
                    Content::new(
                        AFTER_LATEST_ROW,
                        TextContent::new("再次追加后仍保留旧阅读与编辑身份"),
                    ),
                )
                .unwrap();
            },
        )
        .unwrap();
    render(&mut host, &mut text, 248., 300., Theme::DARK);
    let after_latest_append = reader(&host).reading_result().unwrap();
    assert_eq!(after_latest_append.version, version4);
    assert_eq!(
        after_latest_append.anchor,
        ContentReadingAnchorUpdate::FollowingLatest
    );
    assert!(!after_latest_append.clamped);
    assert!(!after_latest_append.user_input_won);
    assert!(reader(&host).is_following_latest());
    assert_eq!(
        reader(&host).reading_items().last(),
        Some(&AFTER_LATEST_ROW)
    );
    assert_eq!(host.semantic_mount(LATEST_ROW), Some(latest_mount));
    assert_eq!(host.semantic_mount(SCROLL), Some(reader_mount));
    assert_eq!(host.semantic_mount(EDITOR), Some(editor_mount));
    assert_eq!(
        host.semantic_mount(history_anchor),
        Some(history_anchor_mount)
    );
    assert_eq!(host.focused(), Some(EDITOR));
    assert_eq!(host.text_input_snapshot().unwrap().binding, before.binding);
    assert_eq!(host.text_input_snapshot().unwrap().marked, before.marked);
    assert_eq!(
        host.text_input_snapshot().unwrap().selection,
        before.selection
    );

    // No submit event occurred: neither a submit action nor a changed editor draft may be queued.
    let post_layout_actions = host.drain_action_records();
    assert!(
        post_layout_actions
            .iter()
            .all(|a| a.action.kind != ContentActionKind::Submit)
    );
    assert!(post_layout_actions.is_empty());
    assert_eq!(host.text_input_snapshot().unwrap().text, before.text);
    assert_eq!(host.text_input_snapshot().unwrap().marked, before.marked);
}

#[test]
fn zero_inset_field_keeps_geometry_but_rejects_clipped_focus_without_side_effects() {
    let mut field = TextFieldContent::new("Inline", "focus witness at curved edge");
    field.surface.input.multiline = true;
    field.show_metadata = false;
    field.viewport = TextViewport::Fill;
    assert_eq!(field.surface.presentation, TextSurfacePresentation::Field);
    let root = Content::new(
        ComponentId(8260),
        panel(Content::new(ComponentId(8261), field), true),
    );
    let mut host = ContentHost::new(root).unwrap();
    let mut text = Text::new(1.0);
    assert!(text.services);
    render(&mut host, &mut text, 260., 140., Theme::LIGHT);

    let mount = host.semantic_mount(ComponentId(8261)).unwrap();
    let frame = host.get(ComponentId(8261)).unwrap().frame();
    let visible = host.get(ComponentId(8261)).unwrap().visible();
    let panel_geometry = host
        .get(ComponentId(8260))
        .and_then(|node| node.component::<PanelContent>())
        .and_then(PanelContent::geometry)
        .cloned()
        .expect("the original zero-inset Panel publishes its arranged curve");
    let surface_before = host.text_surface(ComponentId(8261)).unwrap();
    assert_eq!(surface_before.presentation, TextSurfacePresentation::Field);
    assert!(surface_before.outer.size.width > 0. && surface_before.inner.size.width > 0.);
    let layout_before = host
        .semantic_text_layout_snapshot(ComponentId(8261), None)
        .unwrap();
    assert!(!layout_before.lines.is_empty() && !layout_before.carets.is_empty());
    let input_before = host.text_input_snapshot();
    let binding_before = host.native_key_binding();
    let focus_result_before = host.required_focus_result(ComponentId(8261));

    assert!(
        !host.focus(Some(ComponentId(8261)), true),
        "zero-inset Field's complete-frame focus witness touches the original curved Panel boundary and must be rejected; frame={frame:?}, visible={visible:?}, panel={panel_geometry:?}, focus_result={focus_result_before:?}"
    );
    assert_eq!(host.focused(), None);
    assert_eq!(host.semantic_mount(ComponentId(8261)), Some(mount));
    assert_eq!(host.get(ComponentId(8261)).unwrap().frame(), frame);
    assert_eq!(host.get(ComponentId(8261)).unwrap().visible(), visible);
    let panel_geometry_after = host
        .get(ComponentId(8260))
        .and_then(|node| node.component::<PanelContent>())
        .and_then(PanelContent::geometry)
        .expect("the original zero-inset Panel keeps its arranged curve after refusal");
    assert_eq!(panel_geometry_after.generation, panel_geometry.generation);
    assert_eq!(panel_geometry_after.outer, panel_geometry.outer);
    assert_eq!(panel_geometry_after.allocation, panel_geometry.allocation);
    assert_eq!(panel_geometry_after.shape, panel_geometry.shape);
    assert_eq!(host.text_input_snapshot(), input_before);
    assert_eq!(host.native_key_binding(), binding_before);
    assert_eq!(
        host.required_focus_result(ComponentId(8261)),
        focus_result_before
    );
    assert_eq!(
        host.semantic_text_layout_snapshot(ComponentId(8261), None),
        Some(layout_before),
    );
    assert!(host.drain_action_records().is_empty());
}

#[test]
fn zero_inset_self_framed_field_and_real_split_panes_keep_one_host_and_owner() {
    use tekes_ui_controls::composition::{
        SplitViewConfig, SplitViewConstruction, SplitViewContent, SplitViewSlotIds,
    };
    let mut editor = TextFieldContent::new("Inline", "self-framed 😀 value");
    editor.surface.input.multiline = true;
    editor.show_metadata = false;
    editor.viewport = TextViewport::Fill;
    assert_eq!(editor.surface.presentation, TextSurfacePresentation::Field);
    let leading = Content::new(
        ComponentId(8230),
        panel(
            stack(
                vec![
                    Content::new(ComponentId(8231), TextContent::new("Leading panel")),
                    Content::new(ComponentId(8232), Button::new("Leading action")),
                ],
                None,
            ),
            false,
        ),
    );
    // The positive nested/Split editor uses the original Panel's default inset,
    // leaving the complete Field focus witness inside the curved boundary.
    let inner_editor_panel = Content::new(
        ComponentId(8242),
        panel(Content::new(ComponentId(8241), editor), false),
    );
    let outer_stack = Content::new(ComponentId(8243), StackContent {
        model: Stack {
            axis: Axis::Vertical,
            gap: 6.,
        },
        children: vec![
            inner_editor_panel,
            Content::new(ComponentId(8244), TextContent::new("nested panel sibling")),
        ],
        flexible_child: None,
    });
    let trailing = Content::new(ComponentId(8240), panel(outer_stack, false));
    let split = SplitViewContent::try_new(SplitViewConstruction {
        config: SplitViewConfig {
            ratio: 0.48,
            ..Default::default()
        },
        slots: SplitViewSlotIds {
            leading: ComponentId(8250),
            trailing: ComponentId(8251),
            divider: ComponentId(8252),
        },
        leading,
        trailing,
        budget: 64 * 1024 * 1024,
    })
    .unwrap();
    let mut host = ContentHost::new(Content::new(ComponentId(8229), split)).unwrap();
    let mut text = Text::new(1.0);
    render(&mut host, &mut text, 420., 220., Theme::LIGHT);
    let original_editor_mount = host.semantic_mount(ComponentId(8241)).unwrap();
    let original_inner_panel_mount = host.semantic_mount(ComponentId(8242)).unwrap();
    let original_outer_panel_mount = host.semantic_mount(ComponentId(8240)).unwrap();
    let first = host
        .semantic_text_layout_snapshot(ComponentId(8241), None)
        .unwrap();
    assert!(!first.lines.is_empty() && !first.carets.is_empty());
    assert!(
        host.text_surface(ComponentId(8241))
            .unwrap()
            .inner
            .size
            .width
            > 0.
    );
    assert!(text.services && !text.surface_shape_calls.is_empty());
    assert!(
        host.focus(Some(ComponentId(8241)), true),
        "original nested editor focus admission failed after real Split/Panel layout; inspect arranged frame/clip, Panel curve geometry, text witness result, and hit candidates:\n{}",
        layout_diagnostic(&host, &[
            ComponentId(8229),
            ComponentId(8250),
            ComponentId(8251),
            ComponentId(8240),
            ComponentId(8243),
            ComponentId(8242),
            ComponentId(8241),
            ComponentId(8244),
        ],)
    );
    host.event_with_text(
        &ContentEvent::Preedit {
            text: "中文組字".into(),
            selection: 1..3,
            clauses: std::iter::once(0..4).collect(),
            replacement: None,
        },
        &mut text,
    )
    .unwrap();
    let before = host.text_input_snapshot().unwrap();
    let native_binding = host.native_key_binding().unwrap();
    assert!(before.marked.is_some());
    host.insert(
        ComponentId(8243),
        Content::new(
            ComponentId(8245),
            TextContent::new("dynamically inserted sibling"),
        ),
    )
    .unwrap();
    render(&mut host, &mut text, 420., 220., Theme::DARK);
    assert_eq!(host.focused(), Some(ComponentId(8241)));
    assert_eq!(
        host.semantic_mount(ComponentId(8241)),
        Some(original_editor_mount)
    );
    assert_eq!(
        host.semantic_mount(ComponentId(8242)),
        Some(original_inner_panel_mount)
    );
    assert_eq!(
        host.semantic_mount(ComponentId(8240)),
        Some(original_outer_panel_mount)
    );
    assert_eq!(host.text_input_snapshot().unwrap().binding, before.binding);
    assert_eq!(host.text_input_snapshot().unwrap().marked, before.marked);
    assert_eq!(
        host.text_input_snapshot().unwrap().selection,
        before.selection
    );
    assert_eq!(host.native_key_binding().as_ref(), Some(&native_binding));
    render(&mut host, &mut text, 290., 220., Theme::DARK);
    assert_eq!(host.focused(), Some(ComponentId(8241)));
    assert_eq!(
        host.semantic_mount(ComponentId(8241)),
        Some(original_editor_mount)
    );
    assert_eq!(
        host.semantic_mount(ComponentId(8242)),
        Some(original_inner_panel_mount)
    );
    assert_eq!(
        host.semantic_mount(ComponentId(8240)),
        Some(original_outer_panel_mount)
    );
    assert_eq!(host.text_input_snapshot().unwrap().binding, before.binding);
    assert_eq!(host.text_input_snapshot().unwrap().marked, before.marked);
    assert_eq!(
        host.text_input_snapshot().unwrap().selection,
        before.selection
    );
    assert_eq!(host.native_key_binding().as_ref(), Some(&native_binding));
    let after = host
        .semantic_text_layout_snapshot(ComponentId(8241), None)
        .unwrap();
    assert!(!after.lines.is_empty() && !after.carets.is_empty());
    assert_ne!(
        first, after,
        "same original Host should publish layout at the resized Split pane"
    );
}
