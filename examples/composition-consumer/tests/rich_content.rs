#![cfg(target_os = "macos")]
#[path = "support/coretext.rs"]
mod coretext;
use tekes_ui_composition_consumer::rich_content as f;
use tekes_ui_controls::{
    composition::*,
    extended::{CodeBlock, DiffView},
};
use tekes_ui_platform_macos::content_accessibility as ax;
use tekes_ui_scene::{Point, Primitive, Rect, Scene};
use tekes_ui_tokens::Theme;

fn paint(host: &mut ContentHost, width: f32, scale: f32, theme: Theme) -> Scene {
    let mut text = coretext::Text::new(scale);
    text.services = true;
    let mut scene = Scene::new();
    let frame = Rect::new(20., 20., width, 720.);
    host.render(&mut scene, &mut text, frame, frame, theme.into())
        .unwrap();
    scene
}
#[test]
fn second_consumer_is_one_continuous_selectable_owner_with_original_shaped_native_geometry() {
    let mut host = f::build().unwrap();
    let scene = paint(&mut host, 560., 1., Theme::LIGHT);
    let content = f::rich(&host, f::MARKDOWN);
    let surface = host.text_surface(f::MARKDOWN).unwrap();
    assert!(surface.input.read_only);
    assert_eq!(surface.input.text, content.document().text());
    assert_eq!(content.document().blocks().len(), 5);
    let paragraph = surface.paragraph.as_ref().unwrap();
    let origin = surface.origin();
    let query = host
        .semantic_text_layout_snapshot(f::MARKDOWN, None)
        .unwrap();
    assert_eq!(
        query.lines,
        paragraph
            .lines
            .iter()
            .map(|l| l.utf16.clone())
            .collect::<Vec<_>>()
    );
    assert_eq!(query.input.text, content.document().text());
    assert!(
        paragraph
            .fonts
            .iter()
            .any(|a| paragraph.fonts.iter().any(|b| a.size != b.size)),
        "heading/body use original different font sizes in one paragraph"
    );
    assert!(
        paragraph
            .lines
            .windows(2)
            .all(|pair| pair[0].top + pair[0].height <= pair[1].top)
    );
    for line in &paragraph.lines {
        for (index, x) in &line.carets {
            let (_, rect) = query
                .carets
                .iter()
                .find(|(i, r)| i == index && (r.min_y() - origin.y - line.top as f32).abs() < 0.01)
                .unwrap();
            assert!((rect.min_x() - origin.x - *x as f32).abs() < 0.01);
            assert!((rect.size.height - line.height as f32).abs() < 0.01);
        }
    }
    assert_eq!(content.block_geometry().len(), 5);
    let nodes = host.semantics();
    assert_eq!(
        nodes
            .iter()
            .filter(|n| n.id == f::MARKDOWN && n.semantics.role == ContentRole::TextInput)
            .count(),
        1
    );
    assert!(
        scene
            .primitives()
            .iter()
            .any(|p| matches!(p, Primitive::Glyphs(_)))
    );
    let key = ax::content_key(&host, f::MARKDOWN, None).unwrap();
    let native = ax::content_text_layouts(&host);
    let state = &native.get(&key).unwrap().state;
    assert_eq!(state.text, query.input.text);
    let block = &content.document().blocks()[1];
    let (actual, point, size) = state
        .first_rect_for_range(block.display())
        .unwrap_or_else(|| {
            panic!(
                "firstRect unavailable for {:?}; same-index original carets {:?}",
                block.display(),
                query
                    .carets
                    .iter()
                    .filter(|(i, _)| *i == block.display().start)
                    .collect::<Vec<_>>()
            )
        });
    assert_eq!(actual.start, block.display().start);
    assert!(size.width > 0.);
    assert!(
        (point.y
            - origin.y as f64
            - paragraph.caret_rect(block.display().start, 1.).min_y() as f64)
            .abs()
            < 0.01,
        "native query and original Paragraph use the same downstream line boundary"
    );
    for line in &paragraph.lines {
        if line.utf16.is_empty() {
            continue;
        }
        let (_, point, _) = state
            .first_rect_for_range(line.utf16.clone())
            .expect("every original nonempty line has a native firstRect");
        assert!(
            (point.y - origin.y as f64 - line.top).abs() < 0.01,
            "hard/soft line start shares original line geometry"
        );
    }
    assert!(query.carets.iter().any(|(i, r)| *i == actual.start
        && (r.min_x() as f64 - point.x).abs() < 0.01
        && (r.min_y() as f64 - point.y).abs() < 0.01));
}
#[test]
fn cross_block_unicode_selection_copy_commands_and_readonly_guards_use_original_input() {
    let mut host = f::build().unwrap();
    paint(&mut host, 400., 1., Theme::LIGHT);
    let text = f::rich(&host, f::MARKDOWN).document().text().to_owned();
    let length = text.encode_utf16().count();
    host.accessibility_action(
        f::MARKDOWN,
        None,
        AccessibilityAction::SetSelection(0..length),
    );
    assert_eq!(
        host.text_surface(f::MARKDOWN)
            .unwrap()
            .input
            .selected_text(),
        text
    );
    host.event(&ContentEvent::TextCommand(
        "moveToBeginningOfDocument:".into(),
    ));
    host.event(&ContentEvent::TextCommand(
        "moveToEndOfDocumentAndModifySelection:".into(),
    ));
    assert_eq!(
        host.text_surface(f::MARKDOWN)
            .unwrap()
            .input
            .selected_text(),
        text
    );
    let before = host
        .text_surface(f::MARKDOWN)
        .unwrap()
        .input
        .external_baseline();
    for event in [
        ContentEvent::Commit {
            text: "forbidden".into(),
            replacement: None,
        },
        ContentEvent::Preedit {
            text: "ni".into(),
            selection: 0..2,
            clauses: vec![],
            replacement: None,
        },
        ContentEvent::TextCommand("deleteBackward:".into()),
    ] {
        host.event(&event);
        assert_eq!(
            host.text_surface(f::MARKDOWN)
                .unwrap()
                .input
                .external_baseline(),
            before
        );
    }
    assert!(
        !host
            .accessibility_action(
                f::MARKDOWN,
                None,
                AccessibilityAction::ReplaceSelectedText("forbidden".into())
            )
            .changed
    );
    assert!(
        !host
            .accessibility_action(
                f::MARKDOWN,
                None,
                AccessibilityAction::SetValue("forbidden".into())
            )
            .changed
    );
    assert!(host.drain_actions().is_empty());
}
#[test]
fn direct_cross_block_drag_uses_original_capture_and_shaped_caret_mapping() {
    let mut host = f::build().unwrap();
    paint(&mut host, 560., 1., Theme::LIGHT);
    let surface = host.text_surface(f::MARKDOWN).unwrap();
    let p = surface.paragraph.as_ref().unwrap();
    let origin = surface.origin();
    let first = &p.lines[0];
    let last = &p.lines[p.lines.len() - 1];
    let from = Point::new(
        origin.x + 1.,
        origin.y + first.top as f32 + first.height as f32 * 0.5,
    );
    let to = Point::new(
        origin.x + last.width as f32,
        origin.y + last.top as f32 + last.height as f32 * 0.5,
    );
    assert!(host.event(&ContentEvent::PointerDown(from)).capture_pointer);
    host.event(&ContentEvent::PointerMove(to));
    host.event(&ContentEvent::PointerUp(to));
    let selection = host.text_surface(f::MARKDOWN).unwrap().input.selection();
    assert!(
        selection.end
            > f::rich(&host, f::MARKDOWN).document().blocks()[1]
                .display()
                .end
    );
    assert_eq!(host.focused(), Some(f::MARKDOWN));
    assert!(
        f::rich_mut(&mut host, f::MARKDOWN)
            .drain_link_actions()
            .is_empty(),
        "plain drag never opens link"
    );
}
#[test]
fn versioned_streaming_and_shape_only_updates_preserve_selection_and_reject_stale_foreign_inputs() {
    let mut content = RichContent::new(
        "doc",
        1,
        RichDocument::from_markdown("# Same 中文😀 e\u{301}"),
    )
    .unwrap();
    let initial = content.version();
    let foreign = RichContent::new("other", 1, RichDocument::from_markdown("x"))
        .unwrap()
        .version();
    assert_eq!(
        content.update(&foreign, 2, RichDocument::from_markdown("bad")),
        Err(RichContentError::WrongDocument)
    );
    let mut host = ContentHost::new(Content::new(f::MARKDOWN, content)).unwrap();
    paint(&mut host, 250., 1., Theme::LIGHT);
    host.accessibility_action(f::MARKDOWN, None, AccessibilityAction::SetSelection(5..10));
    let before = host.text_surface(f::MARKDOWN).unwrap().input.selection();
    let old_height = host
        .text_surface(f::MARKDOWN)
        .unwrap()
        .paragraph
        .as_ref()
        .unwrap()
        .height;
    f::rich_mut(&mut host, f::MARKDOWN)
        .update(
            &initial,
            2,
            RichDocument::from_markdown("Same 中文😀 e\u{301}"),
        )
        .unwrap();
    assert!(
        host.semantic_text_layout_snapshot(f::MARKDOWN, None)
            .is_none(),
        "style-only update invalidates old geometry before new layout"
    );
    paint(&mut host, 250., 1., Theme::LIGHT);
    assert_eq!(
        host.text_surface(f::MARKDOWN).unwrap().input.selection(),
        before
    );
    assert!(
        host.text_surface(f::MARKDOWN)
            .unwrap()
            .paragraph
            .as_ref()
            .unwrap()
            .height
            < old_height
    );
    let current = f::rich(&host, f::MARKDOWN).version();
    assert_eq!(
        f::rich_mut(&mut host, f::MARKDOWN).update(&initial, 3, RichDocument::from_markdown("bad")),
        Err(RichContentError::StaleVersion)
    );
    let same_name_other = RichContent::new("doc", 2, RichDocument::from_markdown("x"))
        .unwrap()
        .version();
    assert_eq!(
        f::rich_mut(&mut host, f::MARKDOWN).update(
            &same_name_other,
            3,
            RichDocument::from_markdown("bad")
        ),
        Err(RichContentError::StaleVersion)
    );
    f::rich_mut(&mut host, f::MARKDOWN)
        .update(
            &current,
            3,
            RichDocument::from_markdown("Same 中文😀 e\u{301}\n\n```rust\nlet v = 1;\n```"),
        )
        .unwrap();
    paint(&mut host, 250., 1., Theme::LIGHT);
    assert_eq!(
        host.text_surface(f::MARKDOWN).unwrap().input.selection(),
        before
    );
}
#[test]
fn theme_width_scale_and_background_updates_keep_original_composer_marked_baseline() {
    let mut host = f::build().unwrap();
    paint(&mut host, 560., 1., Theme::LIGHT);
    host.focus(Some(f::COMPOSER), true);
    host.event(&ContentEvent::Preedit {
        text: "ni".into(),
        selection: 0..2,
        clauses: vec![],
        replacement: None,
    });
    let before = host
        .text_surface(f::COMPOSER)
        .unwrap()
        .input
        .external_baseline();
    let mount = host.semantic_mount(f::COMPOSER);
    let original_height = host
        .text_surface(f::MARKDOWN)
        .unwrap()
        .paragraph
        .as_ref()
        .unwrap()
        .height;
    f::append(&mut host);
    for theme in Theme::ALL {
        for (width, scale) in [(560., 1.), (220., 1.5)] {
            paint(&mut host, width, scale, theme);
            assert_eq!(host.focused(), Some(f::COMPOSER));
            assert_eq!(host.semantic_mount(f::COMPOSER), mount);
            assert_eq!(
                host.text_surface(f::COMPOSER)
                    .unwrap()
                    .input
                    .external_baseline(),
                before
            );
            assert!(host.drain_actions().is_empty());
        }
    }
    assert!(
        host.text_surface(f::MARKDOWN)
            .unwrap()
            .paragraph
            .as_ref()
            .unwrap()
            .height
            > original_height
    );
}
#[test]
fn links_are_owned_source_target_callbacks_without_system_actions_or_duplicate_submit() {
    let mut host = f::build().unwrap();
    paint(&mut host, 560., 1., Theme::LIGHT);
    let link = f::rich(&host, f::MARKDOWN).document().links()[0].clone();
    let part = host
        .get(f::MARKDOWN)
        .unwrap()
        .component::<RichContent>()
        .unwrap()
        .semantic_parts()
        .remove(0);
    let response = host.event(&ContentEvent::Pointer(PointerEvent {
        position: part.frame.center(),
        phase: PointerPhase::Down,
        button: Some(PointerButton::Primary),
        modifiers: InputModifiers {
            command: true,
            ..Default::default()
        },
        click_count: 1,
    }));
    assert!(response.consumed && !response.capture_pointer);
    let actions = f::rich_mut(&mut host, f::MARKDOWN).drain_link_actions();
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0].source, ContentActionSource::Pointer);
    assert_eq!(actions[0].target, link.target());
    assert_eq!(actions[0].display, link.display());
    assert_eq!(actions[0].document, "message-1");
    assert_eq!(actions[0].revision, 1);
    host.focus(Some(f::COMPOSER), true);
    assert!(
        host.accessibility_action(f::MARKDOWN, Some(0), AccessibilityAction::Press)
            .consumed
    );
    assert_eq!(host.focused(), Some(f::COMPOSER));
    assert_eq!(
        f::rich_mut(&mut host, f::MARKDOWN).drain_link_actions()[0].source,
        ContentActionSource::Accessibility
    );
    assert!(
        host.drain_actions().is_empty(),
        "rich link callback is not a second form Submit"
    );
}
#[test]
fn old_native_link_part_cannot_activate_a_new_target_after_a_versioned_update() {
    let mut host = f::build().unwrap();
    paint(&mut host, 560., 1., Theme::LIGHT);
    let old = ax::content_key(&host, f::MARKDOWN, Some(0)).unwrap();
    let mount = host.semantic_mount(f::MARKDOWN);
    let expected = f::rich(&host, f::MARKDOWN).version();
    f::rich_mut(&mut host, f::MARKDOWN)
        .update(
            &expected,
            2,
            RichDocument::from_markdown("# New\n\n[new target](https://example.com/new)"),
        )
        .unwrap();
    paint(&mut host, 560., 1., Theme::LIGHT);
    assert_eq!(
        host.semantic_mount(f::MARKDOWN),
        mount,
        "same original text owner"
    );
    assert!(
        !ax::content_action(
            &mut host,
            old,
            tekes_ui_platform_macos::AccessibilityAction::Press
        )
        .consumed
    );
    assert!(
        f::rich_mut(&mut host, f::MARKDOWN)
            .drain_link_actions()
            .is_empty()
    );
    let part = f::rich(&host, f::MARKDOWN).semantic_parts().remove(0).index;
    assert_ne!(part, 0);
    let current = ax::content_key(&host, f::MARKDOWN, Some(part)).unwrap();
    assert!(
        ax::content_action(
            &mut host,
            current,
            tekes_ui_platform_macos::AccessibilityAction::Press
        )
        .consumed
    );
    let actions = f::rich_mut(&mut host, f::MARKDOWN).drain_link_actions();
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0].revision, 2);
    assert_eq!(actions[0].target, "https://example.com/new");
}
#[test]
fn code_and_multi_hunk_diff_copy_text_excludes_decorations_and_reflows_from_same_layout() {
    let code = CodeBlock {
        language: "rust".into(),
        source: "let 中文 = \"😀e\u{301}\";\n// tail".into(),
    };
    let doc = RichDocument::from_code(&code);
    assert_eq!(doc.text(), code.source);
    let diff = DiffView::between(
        "same\nold 中文\nretain\nremove 😀",
        "same\nnew 中文\nretain\nadd 😀\ntail e\u{301}",
    );
    let doc = RichDocument::from_diff(&diff);
    assert_eq!(
        doc.text(),
        diff.lines
            .iter()
            .map(|line| line.text.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    );
    let mut host = ContentHost::new(Content::new(
        f::MARKDOWN,
        RichContent::new("diff", 1, doc).unwrap(),
    ))
    .unwrap();
    for theme in Theme::ALL {
        paint(&mut host, 160., 1.5, theme);
    }
    let length = host.text_surface(f::MARKDOWN).unwrap().input.len();
    host.accessibility_action(
        f::MARKDOWN,
        None,
        AccessibilityAction::SetSelection(0..length),
    );
    assert!(
        !host
            .text_surface(f::MARKDOWN)
            .unwrap()
            .input
            .selected_text()
            .starts_with("+ ")
    );
    assert_eq!(
        f::rich(&host, f::MARKDOWN).block_geometry().len(),
        diff.lines.len()
    );
    assert!(
        host.text_surface(f::MARKDOWN)
            .unwrap()
            .paragraph
            .as_ref()
            .unwrap()
            .lines
            .iter()
            .all(|line| line.carets.iter().all(|(_, x)| *x > 0.)),
        "line-number gutter is part of original text geometry but never copy text"
    );
}
#[test]
fn mixed_reading_prepend_expand_and_reflow_use_existing_stable_item_transaction() {
    let mut host = f::build().unwrap();
    paint(&mut host, 300., 1., Theme::LIGHT);
    let version = f::reading_version(&host);
    let keys = vec![
        ("md".into(), f::MARKDOWN),
        ("code".into(), f::CODE),
        ("diff".into(), f::DIFF),
    ];
    host.get_mut(f::READER)
        .unwrap()
        .component_mut::<ScrollContent>()
        .unwrap()
        .stop_following();
    host.get_mut(f::READER)
        .unwrap()
        .component_mut::<ScrollContent>()
        .unwrap()
        .scroll
        .scroll_by(0., 80.);
    paint(&mut host, 300., 1., Theme::LIGHT);
    let checkpoint = host
        .capture_scroll_reading_checkpoint(f::READER, &version, &keys)
        .unwrap();
    f::prepend(&mut host);
    f::append(&mut host);
    paint(&mut host, 220., 1.5, Theme::DARK);
    let version = f::reading_version(&host);
    let request = host
        .prepare_scroll_reading_restore(f::READER, &version)
        .unwrap();
    let keys = vec![
        ("earlier".into(), f::EARLIER),
        ("md".into(), f::MARKDOWN),
        ("code".into(), f::CODE),
        ("diff".into(), f::DIFF),
    ];
    host.restore_scroll_reading_checkpoint(&request, &checkpoint, &keys)
        .unwrap();
    paint(&mut host, 220., 1.5, Theme::DARK);
    let next = host
        .capture_scroll_reading_checkpoint(f::READER, &version, &keys)
        .unwrap();
    match (next.position, checkpoint.position) {
        (
            ContentReadingPosition::History {
                item: a, within: x, ..
            },
            ContentReadingPosition::History {
                item: b, within: y, ..
            },
        ) => {
            assert_eq!(a, b);
            assert!((x - y).abs() < 1.);
        }
        other => panic!("expected item-local history: {other:?}"),
    }
}
