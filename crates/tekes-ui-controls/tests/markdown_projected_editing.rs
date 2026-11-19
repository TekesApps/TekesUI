use std::rc::Rc;
use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, ControlKey, ExternalSelection, ExternalUndo,
    ExternalValueConflict, ExternalValueResult, TextEditCommand, TextSurfaceHost,
    TextSurfaceTextRenderer, composition::*,
};
use tekes_ui_scene::{FontKey, Point, Rect, Scene, Size};
use tekes_ui_text::{FontInfo, Line, Paragraph, ShapedGlyph, ShapedRun};
use tekes_ui_tokens::{Palette, Rgba, Theme, proposal};

#[derive(Default)]
struct TestTextHost {
    pushed_colors: Vec<(Rgba, usize)>,
}

impl ButtonTextRenderer for TestTextHost {
    fn text_surface_host(&mut self) -> Option<&mut dyn TextSurfaceHost> {
        Some(self)
    }
    fn measure(&mut self, text: &str, role: proposal::TextRole, width: f32) -> Size {
        Size::new(
            (text.chars().count() as f32 * 8.).min(width),
            role.style().line_height,
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

impl TextSurfaceHost for TestTextHost {
    fn type_scale(&self) -> f32 {
        1.0
    }
    fn shape_surface(
        &mut self,
        text: &str,
        role: proposal::TextRole,
        _width: Option<f32>,
    ) -> Rc<Paragraph> {
        let line_height = f64::from(role.style().line_height);
        let ascent = line_height * 0.8;
        let descent = line_height - ascent;
        let font_key = FontKey(1);
        let mut lines = Vec::new();
        let mut y = 0.0;
        let mut utf16 = 0usize;
        let mut byte = 0usize;
        let mut max_w = 0.0_f64;

        for segment in text.split_inclusive('\n') {
            let has_nl = segment.ends_with('\n');
            let len_utf16 = segment.encode_utf16().count();
            let end_utf16 = utf16 + len_utf16;
            let end_byte = byte + segment.len();
            let mut glyphs = Vec::new();
            let mut carets = Vec::new();
            let mut x = 0.0;
            let mut cur_u16 = utf16;
            for ch in segment.chars() {
                carets.push((cur_u16, x));
                if ch != '\n' {
                    glyphs.push(ShapedGlyph {
                        id: ch as u16,
                        x,
                        y: 0.0,
                        utf16_index: cur_u16,
                        cluster_utf16: cur_u16..cur_u16 + ch.len_utf16(),
                    });
                    x += 8.0;
                }
                cur_u16 += ch.len_utf16();
            }
            carets.push((end_utf16, x));
            max_w = max_w.max(x);
            lines.push(Line {
                index: lines.len(),
                utf16: utf16..end_utf16,
                bytes: byte..end_byte,
                ascent,
                descent,
                leading: 0.0,
                height: line_height,
                top: y,
                baseline_y: y + ascent,
                width: x,
                width_trimmed: x,
                runs: if glyphs.is_empty() {
                    Vec::new()
                } else {
                    vec![ShapedRun {
                        font: font_key,
                        utf16: utf16..end_utf16,
                        glyphs,
                    }]
                },
                carets,
                graphemes: Vec::new(),
                has_newline: has_nl,
            });
            y += line_height;
            utf16 = end_utf16;
            byte = end_byte;
        }
        if text.is_empty() || text.ends_with('\n') {
            lines.push(Line {
                index: lines.len(),
                utf16: utf16..utf16,
                bytes: byte..byte,
                ascent,
                descent,
                leading: 0.0,
                height: line_height,
                top: y,
                baseline_y: y + ascent,
                width: 0.0,
                width_trimmed: 0.0,
                runs: Vec::new(),
                carets: vec![(utf16, 0.0)],
                graphemes: Vec::new(),
                has_newline: false,
            });
            y += line_height;
        }

        Rc::new(Paragraph {
            lines,
            height: y.max(line_height),
            width: max_w,
            default_line_height: line_height,
            fonts: vec![FontInfo {
                key: font_key,
                post_script_name: "TestFont".into(),
                size: f64::from(role.style().size),
                is_color: false,
            }],
            utf16_len: text.encode_utf16().count(),
        })
    }
}

impl TextSurfaceTextRenderer for TestTextHost {
    fn push_visible(
        &mut self,
        _: &mut Scene,
        paragraph: &Paragraph,
        _: Point,
        color: Rgba,
        _: Rect,
    ) {
        let glyphs: usize = paragraph
            .lines
            .iter()
            .flat_map(|l| l.runs.iter())
            .map(|r| r.glyphs.len())
            .sum();
        if glyphs > 0 {
            self.pushed_colors.push((color, glyphs));
        }
    }
}

const EDITOR_ID: ComponentId = ComponentId(1);

fn mount_editor(source: &str) -> ContentHost {
    let editor = MarkdownEditorContent::new("notes", 1, source)
        .unwrap()
        .with_viewport(TextViewport::Fixed(240.));
    ContentHost::new(Content::new(EDITOR_ID, editor)).unwrap()
}

fn editor(host: &ContentHost) -> &MarkdownEditorContent {
    host.get(EDITOR_ID)
        .unwrap()
        .component::<MarkdownEditorContent>()
        .unwrap()
}

fn editor_mut(host: &mut ContentHost) -> &mut MarkdownEditorContent {
    host.get_mut(EDITOR_ID)
        .unwrap()
        .component_mut::<MarkdownEditorContent>()
        .unwrap()
}

fn render_editor(host: &mut ContentHost, text: &mut TestTextHost) {
    let frame = Rect::new(0., 0., 400., 260.);
    host.render(
        &mut Scene::new(),
        text,
        frame,
        frame,
        Palette::from(Theme::LIGHT),
    )
    .unwrap();
}

#[test]
fn live_preview_focus_collapse_expand_and_navigation_without_undo_pollution() {
    let source = "# Title\n\n- Item `k` **bold**\n\n[Guide](guide.md)";
    let mut host = mount_editor(source);
    let mut text = TestTextHost::default();
    render_editor(&mut host, &mut text);

    // Unfocused: all blocks collapsed.
    assert_eq!(
        editor(&host).display_text(),
        "Title\n\n• Item k bold\n\nGuide"
    );
    assert_eq!(editor(&host).source(), source);

    // Focus at offset 0: only heading expands.
    assert!(host.focus(Some(EDITOR_ID), true));
    assert_eq!(
        editor(&host).display_text(),
        "# Title\n\n• Item k bold\n\nGuide"
    );

    // Move caret down past blank line into bullet line -> heading collapses, bullet expands.
    render_editor(&mut host, &mut text);
    host.event(&ContentEvent::TextCommand(TextEditCommand::MoveDown));
    render_editor(&mut host, &mut text);
    host.event(&ContentEvent::TextCommand(TextEditCommand::MoveDown));
    render_editor(&mut host, &mut text);
    assert_eq!(
        editor(&host).display_text(),
        "Title\n\n- Item `k` **bold**\n\nGuide"
    );

    // Move down into link paragraph -> bullet collapses, link expands.
    host.event(&ContentEvent::TextCommand(TextEditCommand::MoveDown));
    render_editor(&mut host, &mut text);
    host.event(&ContentEvent::TextCommand(TextEditCommand::MoveDown));
    render_editor(&mut host, &mut text);
    assert_eq!(
        editor(&host).display_text(),
        "Title\n\n• Item k bold\n\n[Guide](guide.md)"
    );

    // Pure cursor movement must never record undo history.
    let undo = host.event(&ContentEvent::TextCommand(TextEditCommand::Undo));
    assert!(!undo.changed);
    assert_eq!(editor(&host).source(), source);

    // Moving focus out via host.focus(None, false) collapses all blocks again.
    assert!(host.focus(None, false));
    assert_eq!(
        editor(&host).display_text(),
        "Title\n\n• Item k bold\n\nGuide"
    );
}

#[test]
fn select_all_expands_all_blocks_including_empty_fence_and_edits_losslessly() {
    let source = "# Title\n\n- Item\n\n[Docs](spec.md)";
    let mut host = mount_editor(source);
    let mut text = TestTextHost::default();
    render_editor(&mut host, &mut text);
    assert!(host.focus(Some(EDITOR_ID), true));

    let sel_resp = host.event(&ContentEvent::TextCommand(TextEditCommand::SelectAll));
    assert!(sel_resp.changed);
    assert_eq!(editor(&host).display_text(), source);
    let snap = host.semantic_text_input_snapshot(EDITOR_ID).unwrap();
    assert_eq!(snap.text, source);
    assert_eq!(snap.selection, 0..source.encode_utf16().count());

    // Replacing over SelectAll mutates source cleanly and Undo restores full source.
    host.event(&ContentEvent::Commit {
        text: "Replaced".into(),
        replacement: None,
    });
    assert_eq!(editor(&host).source(), "Replaced");
    host.event(&ContentEvent::TextCommand(TextEditCommand::Undo));
    assert_eq!(editor(&host).source(), source);

    // Collapsed empty code fence must also expand on SelectAll.
    let mut empty_fence = mount_editor("```rust\n```\n");
    render_editor(&mut empty_fence, &mut text);
    assert!(empty_fence.focus(Some(EDITOR_ID), true));
    empty_fence.event(&ContentEvent::TextCommand(TextEditCommand::SelectAll));
    assert_eq!(editor(&empty_fence).display_text(), "```rust\n```\n");
}

#[test]
fn canonical_undo_redo_across_block_transitions_and_syntax_colors() {
    let mut host = mount_editor("plain line");
    let mut text = TestTextHost::default();
    render_editor(&mut host, &mut text);
    assert!(host.focus(Some(EDITOR_ID), true));

    host.event(&ContentEvent::Commit {
        text: "# ".into(),
        replacement: None,
    });
    assert_eq!(editor(&host).source(), "# plain line");
    assert_eq!(
        editor(&host).projection().blocks()[0].kind(),
        RichBlockKind::Heading { level: 1 }
    );
    assert_eq!(editor(&host).display_text(), "# plain line");

    // Blur reveals collapsed heading text ("plain line"), refocus expands "# plain line".
    assert!(host.focus(None, false));
    assert_eq!(editor(&host).display_text(), "plain line");
    assert!(host.focus(Some(EDITOR_ID), true));
    assert_eq!(editor(&host).display_text(), "# plain line");

    // Canonical Undo reverts "# " back to paragraph, Redo restores heading.
    host.event(&ContentEvent::TextCommand(TextEditCommand::Undo));
    assert_eq!(editor(&host).source(), "plain line");
    assert_eq!(
        editor(&host).projection().blocks()[0].kind(),
        RichBlockKind::Paragraph
    );
    host.event(&ContentEvent::TextCommand(TextEditCommand::Redo));
    assert_eq!(editor(&host).source(), "# plain line");
    assert_eq!(
        editor(&host).projection().blocks()[0].kind(),
        RichBlockKind::Heading { level: 1 }
    );

    // Code fences + inline code apply semantic code token colors when painted.
    let mut code_host = mount_editor("```rust\nlet n = 1;\n```\n\nUse `foo`");
    render_editor(&mut code_host, &mut text);
    assert!(code_host.focus(Some(EDITOR_ID), true));
    text.pushed_colors.clear();
    render_editor(&mut code_host, &mut text);
    let palette = Palette::from(Theme::LIGHT);
    let comment_ink = palette.color("c-code-comment").unwrap();
    let inline_ink = palette.color("c-code-string").unwrap();
    assert!(text.pushed_colors.iter().any(|(c, _)| *c == comment_ink));
    assert!(text.pushed_colors.iter().any(|(c, _)| *c == inline_ink));
}

#[test]
fn ime_preedit_freezes_projection_until_commit_with_cjk_and_zwj() {
    let source = "# 标题\n\n你好👩\u{200d}🔬";
    let mut host = mount_editor(source);
    let mut text = TestTextHost::default();
    render_editor(&mut host, &mut text);
    assert!(host.focus(Some(EDITOR_ID), true));

    // Move caret to end of Heading "# 标题|"
    editor_mut(&mut host).select_source_range(4, 4);
    let display_before_ime = editor(&host).display_text().to_owned();
    assert_eq!(display_before_ime, "# 标题\n\n你好👩\u{200d}🔬");

    // Active Preedit freezes re-projection and preserves canonical source untouched.
    host.event(&ContentEvent::Preedit {
        text: "shijie`".into(),
        selection: 7..7,
        clauses: vec![0..4, 4..7],
        replacement: None,
    });
    assert!(editor(&host).is_composing());
    assert!(editor(&host).is_projection_frozen());
    assert_eq!(editor(&host).source(), source);
    let snap = host.semantic_text_input_snapshot(EDITOR_ID).unwrap();
    assert_eq!(snap.marked, Some(4..11));

    // Commit finalizes CJK text, unfreezes projection, and reconciles source.
    host.event(&ContentEvent::Commit {
        text: "世界".into(),
        replacement: None,
    });
    assert!(!editor(&host).is_composing());
    assert!(!editor(&host).is_projection_frozen());
    assert_eq!(editor(&host).source(), "# 标题世界\n\n你好👩\u{200d}🔬");
}

#[test]
fn pointer_drag_freezes_projection_between_down_move_and_up() {
    let source = "# Heading\n\nParagraph text";
    let mut host = mount_editor(source);
    let mut text = TestTextHost::default();
    render_editor(&mut host, &mut text);
    assert!(host.focus(Some(EDITOR_ID), true));
    render_editor(&mut host, &mut text);

    let geoms = editor(&host).block_geometry();
    let heading_pt = geoms[0].frame.center();
    let para_pt = geoms[1].frame.center();

    host.event(&ContentEvent::Pointer(PointerEvent {
        position: heading_pt,
        phase: PointerPhase::Down,
        button: Some(PointerButton::Primary),
        modifiers: Default::default(),
        click_count: 1,
    }));
    assert!(editor(&host).is_projection_frozen());
    let display_during_drag = editor(&host).display_text().to_owned();

    // Drag across blocks onto Paragraph: display text must NOT jump mid-drag.
    host.event(&ContentEvent::Pointer(PointerEvent {
        position: para_pt,
        phase: PointerPhase::Move,
        button: Some(PointerButton::Primary),
        modifiers: Default::default(),
        click_count: 0,
    }));
    assert!(editor(&host).is_projection_frozen());
    assert_eq!(editor(&host).display_text(), display_during_drag);

    // Releasing pointer unfreezes and expands the intersected selection blocks.
    host.event(&ContentEvent::Pointer(PointerEvent {
        position: para_pt,
        phase: PointerPhase::Up,
        button: Some(PointerButton::Primary),
        modifiers: Default::default(),
        click_count: 1,
    }));
    assert!(!editor(&host).is_projection_frozen());
    assert_eq!(editor(&host).source(), source);
    let _ = &text;

    #[cfg(feature = "inspection")]
    {
        let props = editor(&host).inspection_properties().unwrap();
        assert!(
            props
                .iter()
                .any(|p| p.name == "component" && p.value == "MarkdownEditorContent")
        );
        assert!(
            props
                .iter()
                .any(|p| p.name == "projection_frozen" && p.value == "false")
        );
    }
}

#[test]
fn link_activation_via_accessibility_cmd_click_and_cmd_enter() {
    let source = "Read [Manual](docs/manual.md) now";
    let mut host = mount_editor(source);
    let mut text = TestTextHost::default();
    render_editor(&mut host, &mut text);

    let link_part = editor(&host).semantic_parts().into_iter().next().unwrap();
    let ax_resp =
        host.accessibility_action(EDITOR_ID, Some(link_part.index), AccessibilityAction::Press);
    assert!(ax_resp.consumed);
    let actions = editor_mut(&mut host).drain_link_actions();
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0].target, "docs/manual.md");
    assert_eq!(actions[0].source, ContentActionSource::Accessibility);
    assert_eq!(
        actions[0].source_range,
        source.find("[Manual]").unwrap()..source.find(" now").unwrap()
    );

    // Cmd-click emits Pointer action without changing source.
    host.event(&ContentEvent::Pointer(PointerEvent {
        position: link_part.frame.center(),
        phase: PointerPhase::Down,
        button: Some(PointerButton::Primary),
        modifiers: PointerModifiers {
            command: true,
            ..Default::default()
        },
        click_count: 1,
    }));
    let actions = editor_mut(&mut host).drain_link_actions();
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0].source, ContentActionSource::Pointer);
    assert_eq!(editor(&host).source(), source);

    // Cmd-Enter while caret is on link label emits Keyboard action (editor was focused by click).
    assert_eq!(host.focused(), Some(EDITOR_ID));
    let label_src = source.find("Manual").unwrap() + 1;
    editor_mut(&mut host).select_source_range(label_src, label_src);
    host.event(&ContentEvent::KeyInput(ContentKeyEvent {
        key: Some(ControlKey::Enter),
        down: true,
        repeat: false,
        modifiers: InputModifiers {
            command: true,
            ..Default::default()
        },
        native_code: 0,
        characters: String::new(),
        characters_ignoring_modifiers: String::new(),
    }));
    let actions = editor_mut(&mut host).drain_link_actions();
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0].source, ContentActionSource::Keyboard);
}

#[test]
fn controlled_streaming_external_source_and_conflict_handling() {
    let mut host = mount_editor("# Plan\n\nStep 1");
    let mut text = TestTextHost::default();
    render_editor(&mut host, &mut text);
    assert!(host.focus(Some(EDITOR_ID), true));

    let v1 = editor(&host).version();
    assert_eq!(
        editor(&host)
            .check_external_source(
                &v1,
                "# Plan\n\nStep 1 -> Step 2",
                2,
                ExternalSelection::Preserve,
                ExternalUndo::Preserve,
            )
            .unwrap(),
        ExternalValueResult::Applied {
            changed: true,
            document_changed: false,
        }
    );

    // Apply streaming chunk while preserving caret at offset 2 ("# |Plan").
    editor_mut(&mut host).select_source_range(2, 2);
    let v1_moved = editor(&host).version();
    assert_eq!(
        editor_mut(&mut host)
            .apply_external_source(
                &v1_moved,
                "# Plan\n\nStep 1 -> Step 2",
                2,
                ExternalSelection::Preserve,
                ExternalUndo::Preserve,
            )
            .unwrap(),
        ExternalValueResult::Applied {
            changed: true,
            document_changed: false,
        }
    );
    assert_eq!(editor(&host).source(), "# Plan\n\nStep 1 -> Step 2");
    assert_eq!(editor(&host).source_selection(), (2, 2));

    // Idempotent replay succeeds with Unchanged; lower revision and wrong identity error.
    let v2 = editor(&host).version();
    assert_eq!(
        editor_mut(&mut host)
            .apply_external_source(
                &v2,
                "# Plan\n\nStep 1 -> Step 2",
                2,
                ExternalSelection::Preserve,
                ExternalUndo::Preserve,
            )
            .unwrap(),
        ExternalValueResult::Unchanged
    );
    assert_eq!(
        editor_mut(&mut host).apply_external_source(
            &v1,
            "# Plan",
            1,
            ExternalSelection::Reset,
            ExternalUndo::Reset,
        ),
        Err(MarkdownEditorError::StaleVersion)
    );

    // Active IME Preedit returns Conflict(Composition) without disturbing marked state.
    host.event(&ContentEvent::Preedit {
        text: "abc".into(),
        selection: 3..3,
        clauses: vec![std::ops::Range { start: 0, end: 3 }],
        replacement: None,
    });
    assert_eq!(
        editor_mut(&mut host)
            .apply_external_source(
                &v2,
                "# Plan\n\nStep 3",
                3,
                ExternalSelection::Reset,
                ExternalUndo::Preserve,
            )
            .unwrap(),
        ExternalValueResult::Conflict(ExternalValueConflict::Composition)
    );
    assert!(editor(&host).is_composing());

    // Committing local edit advances source_epoch -> stale v2 gets Conflict(StaleBaseline).
    host.event(&ContentEvent::Commit {
        text: "!".into(),
        replacement: None,
    });
    assert_eq!(
        editor_mut(&mut host)
            .apply_external_source(
                &v2,
                "# Plan\n\nStep 3",
                3,
                ExternalSelection::Reset,
                ExternalUndo::Preserve,
            )
            .unwrap(),
        ExternalValueResult::Conflict(ExternalValueConflict::StaleBaseline)
    );

    let wrong = MarkdownEditorContent::new("other", 1, "x")
        .unwrap()
        .version();
    assert_eq!(
        editor_mut(&mut host).check_external_source(
            &wrong,
            "y",
            4,
            ExternalSelection::Reset,
            ExternalUndo::Reset,
        ),
        Err(MarkdownEditorError::WrongDocument)
    );
}
