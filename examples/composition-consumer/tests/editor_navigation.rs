use tekes_ui_controls::{ButtonTextAlign, ButtonTextRenderer, composition::*};
use tekes_ui_scene::{Rect, Scene, Size};
use tekes_ui_tokens::{Rgba, Theme, proposal};
struct Text;
impl ButtonTextRenderer for Text {
    fn text_surface_host(&mut self) -> Option<&mut dyn tekes_ui_controls::TextSurfaceHost> {
        Some(self)
    }
    fn measure(&mut self, s: &str, r: proposal::TextRole, w: f32) -> Size {
        Size::new((s.len() as f32 * 8.).min(w), r.style().line_height)
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
impl tekes_ui_controls::TextSurfaceHost for Text {
    fn shape_surface(
        &mut self,
        text: &str,
        role: proposal::TextRole,
        width: Option<f32>,
    ) -> std::rc::Rc<tekes_ui_text::model::Paragraph> {
        use tekes_ui_text::traits::Shaper;
        let mut shaper = tekes_ui_text::coretext::CoreTextShaper::new(
            tekes_ui_text::coretext::CoreTextFonts::new(),
        );
        std::rc::Rc::new(shaper.shape(
            text,
            &tekes_ui_text::style::TextStyle::system(role.style().size),
            &tekes_ui_text::model::ShapeOptions {
                width: width.map(f64::from),
                line_spacing: 0.,
            },
        ))
    }
}
impl tekes_ui_controls::TextSurfaceTextRenderer for Text {
    fn push_visible(
        &mut self,
        _: &mut Scene,
        _: &tekes_ui_text::model::Paragraph,
        _: tekes_ui_scene::Point,
        _: Rgba,
        _: Rect,
    ) {
    }
}

use tekes_ui_controls::TextSurface;
const EDIT: ComponentId = ComponentId(2);
fn host(value: &str, width: f32) -> ContentHost {
    let mut f = TextFieldContent::new("editor", value);
    f.surface = TextSurface::new(value, true, false);
    let mut h = ContentHost::new(Content::new(EDIT, f)).unwrap();
    let frame = Rect::new(30., 40., width, 250.);
    h.render(
        &mut Scene::new(),
        &mut Text,
        frame,
        frame,
        Theme::LIGHT.into(),
    )
    .unwrap();
    h.focus(Some(EDIT), true);
    h
}
fn field(h: &ContentHost) -> &TextFieldContent {
    h.get(EDIT)
        .unwrap()
        .component::<TextFieldContent>()
        .unwrap()
}
fn field_mut(h: &mut ContentHost) -> &mut TextFieldContent {
    h.get_mut(EDIT)
        .unwrap()
        .component_mut::<TextFieldContent>()
        .unwrap()
}
fn text_command(h: &mut ContentHost, command: &str) -> bool {
    h.event_with_text(&ContentEvent::TextCommand(command.into()), &mut Text)
        .unwrap()
        .consumed
}
#[test]
fn multiline_line_edge_is_not_document_edge() {
    let mut h = host("first\nsecond\nlast", 500.);
    field_mut(&mut h).surface.input.select(9, false);
    assert!(
        h.event(&ContentEvent::TextCommand("moveToBeginningOfLine:".into()))
            .consumed
    );
    assert_eq!(field(&h).surface.input.caret, 6);
    h.event(&ContentEvent::TextCommand("moveToEndOfLine:".into()));
    assert_eq!(field(&h).surface.input.caret, 12);
}
#[test]
fn vertical_movement_keeps_preferred_column_across_a_short_line() {
    let mut h = host("abcdef\nx\nabcdef", 500.);
    field_mut(&mut h).surface.input.select(5, false);
    assert!(
        h.event(&ContentEvent::TextCommand("moveDown:".into()))
            .consumed
    );
    assert_eq!(field(&h).surface.input.caret, 8);
    h.event(&ContentEvent::TextCommand("moveDown:".into()));
    assert_eq!(field(&h).surface.input.caret, 14);
    h.event(&ContentEvent::TextCommand(
        "moveUpAndModifySelection:".into(),
    ));
    assert_eq!(field(&h).surface.input.anchor, 14);
    assert_eq!(field(&h).surface.input.caret, 8);
    h.event(&ContentEvent::TextCommand(
        "moveUpAndModifySelection:".into(),
    ));
    assert_eq!(field(&h).surface.input.caret, 5);
}
#[test]
fn soft_line_end_keeps_caret_on_that_visual_line_and_preserves_affinity() {
    let mut h = host(&"abcdef ".repeat(30), 180.);
    let line = field(&h).surface.paragraph.as_ref().unwrap().lines[0].clone();
    assert!(!line.has_newline);
    field_mut(&mut h).surface.input.select(2, false);
    h.event(&ContentEvent::TextCommand("moveToEndOfLine:".into()));
    let s = &field(&h).surface;
    assert_eq!(s.input.caret, line.utf16.end);
    assert_eq!(s.caret_doc(s.input.caret).min_y(), line.top as f32);
    h.event(&ContentEvent::TextCommand("moveDown:".into()));
    let s = &field(&h).surface;
    assert!(s.caret_doc(s.input.caret).min_y() > line.top as f32);
}

#[test]
fn delete_to_visual_line_start_preserves_soft_wrap_prefix_and_undo_selection() {
    let value = "one two three four five six seven eight";
    let mut h = host(value, 128.);
    let paragraph = field(&h).surface.paragraph.as_ref().unwrap();
    assert!(paragraph.lines.len() > 1, "fixture must soft-wrap");
    let line = paragraph.lines[1].clone();
    assert!(!line.has_newline, "the target is a soft-wrapped line");

    // Starting from a caret at the end of the visual line deletes that line's
    // visible text back to its visual start, without crossing the wrap.
    field_mut(&mut h)
        .surface
        .input
        .select(line.utf16.start + 1, false);
    assert!(h.event(&ContentEvent::TextCommand("moveToEndOfLine:".into())).consumed);
    assert_eq!(field(&h).surface.caret_doc(line.utf16.end).min_y(), line.top as f32);
    let start = line.utf16.start;
    let end = line.utf16.end;
    let expected = format!("{}{}", &value[..start], &value[end..]);
    assert!(
        h.event(&ContentEvent::TextCommand(
            "deleteToBeginningOfLine:".into()
        ))
        .consumed
    );
    assert_eq!(field(&h).surface.input.text, expected);
    assert!(field_mut(&mut h).surface.input.undo());
    assert_eq!(field(&h).surface.input.text, value);
    assert_eq!(field(&h).surface.input.selection(), end..end);

    // A pre-existing selection wins over the line boundary and Undo restores
    // that original selection, not an expanded temporary deletion range.
    field_mut(&mut h).surface.input.select_range(1..4);
    assert!(
        h.event(&ContentEvent::TextCommand(
            "deleteToBeginningOfLine:".into()
        ))
        .consumed
    );
    assert_eq!(
        field(&h).surface.input.text,
        format!("{}{}", &value[..1], &value[4..])
    );
    assert!(field_mut(&mut h).surface.input.undo());
    assert_eq!(field(&h).surface.input.text, value);
    assert_eq!(field(&h).surface.input.selection(), 1..4);
}

#[test]
fn paragraph_navigation_selection_and_delete_use_logical_crlf_boundaries() {
    let value = "first\r\n😀second\r\nlast";
    let mut h = host(value, 500.);

    // UTF-16 caret 11 is inside the second paragraph (emoji occupies two
    // UTF-16 code units); paragraph navigation must not land inside CRLF.
    field_mut(&mut h).surface.input.select(11, false);
    assert!(
        h.event(&ContentEvent::TextCommand(
            "moveToBeginningOfParagraph:".into()
        ))
        .consumed
    );
    assert_eq!(field(&h).surface.input.caret, 7);
    assert!(
        h.event(&ContentEvent::TextCommand("moveToEndOfParagraph:".into()))
            .consumed
    );
    assert_eq!(field(&h).surface.input.caret, 15);

    field_mut(&mut h).surface.input.select(11, false);
    assert!(
        h.event(&ContentEvent::TextCommand("selectLine:".into()))
            .consumed
    );
    assert_eq!(field(&h).surface.input.selected_text(), "😀second\r\n");

    field_mut(&mut h).surface.input.select(11, false);
    assert!(
        h.event(&ContentEvent::TextCommand("selectParagraph:".into()))
            .consumed
    );
    assert_eq!(field(&h).surface.input.selected_text(), "😀second\r\n");

    field_mut(&mut h).surface.input.select(11, false);
    assert!(
        h.event(&ContentEvent::TextCommand("deleteToEndOfParagraph:".into()))
            .consumed
    );
    assert_eq!(field(&h).surface.input.text, "first\r\n😀se\r\nlast");
    assert!(field_mut(&mut h).surface.input.undo());
    assert_eq!(field(&h).surface.input.text, value);
    assert_eq!(field(&h).surface.input.selection(), 11..11);
}

#[test]
fn line_deletion_respects_hard_break_read_only_and_marked_text() {
    let mut h = host("ab\ncd\nef", 500.);
    field_mut(&mut h).surface.input.select(3, false);
    assert!(
        h.event(&ContentEvent::TextCommand(
            "deleteToBeginningOfLine:".into()
        ))
        .consumed
    );
    assert_eq!(field(&h).surface.input.text, "ab\ncd\nef");

    assert!(
        h.event(&ContentEvent::TextCommand("deleteToEndOfLine:".into()))
            .consumed
    );
    assert_eq!(field(&h).surface.input.text, "ab\n\nef");

    field_mut(&mut h).surface.input.select(1, false);
    field_mut(&mut h).surface.input.read_only = true;
    let before = field(&h).surface.input.text.clone();
    h.event(&ContentEvent::TextCommand(
        "deleteToBeginningOfLine:".into(),
    ));
    assert_eq!(field(&h).surface.input.text, before);

    field_mut(&mut h).surface.input.read_only = false;
    field_mut(&mut h).surface.input.preedit("mark", 4..4, None);
    let before = field(&h).surface.input.text.clone();
    let marked = field(&h).surface.input.marked.clone();
    h.event(&ContentEvent::TextCommand("deleteToEndOfLine:".into()));
    assert_eq!(field(&h).surface.input.text, before);
    assert_eq!(field(&h).surface.input.marked, marked);
}

#[test]
fn crlf_line_end_command_matches_native_selection_and_line_boundary() {
    let mut h = host("first\r\nsecond\r\nlast", 500.);
    field_mut(&mut h).surface.input.select(10, false);
    assert!(
        h.event(&ContentEvent::TextCommand("deleteToEndOfLine:".into()))
            .consumed
    );
    assert_eq!(field(&h).surface.input.text, "first\r\nseclast");
    assert!(field_mut(&mut h).surface.input.undo());
    assert_eq!(field(&h).surface.input.text, "first\r\nsecond\r\nlast");
    assert_eq!(field(&h).surface.input.selection(), 10..10);
}

#[test]
fn delete_to_line_or_paragraph_end_at_boundary_removes_the_terminating_break() {
    for command in ["deleteToEndOfLine:", "deleteToEndOfParagraph:"] {
        let mut h = host("ab\ncd\nef", 500.);
        // Position exactly at the content end, before its line/paragraph break.
        field_mut(&mut h).surface.input.select(5, false);
        assert!(h.event(&ContentEvent::TextCommand(command.into())).consumed);
        assert_eq!(field(&h).surface.input.text, "ab\ncdef", "{command}");
        assert!(field_mut(&mut h).surface.input.undo());
        assert_eq!(field(&h).surface.input.text, "ab\ncd\nef");
        assert_eq!(field(&h).surface.input.selection(), 5..5);
    }
}

#[test]
fn paragraph_backward_forward_selection_and_native_line_edge_aliases() {
    let mut h = host("first\r\nsecond\r\nlast", 500.);
    field_mut(&mut h).surface.input.select(9, false);
    assert!(
        h.event(&ContentEvent::TextCommand(
            "moveParagraphBackwardAndModifySelection:".into()
        ))
        .consumed
    );
    assert_eq!(field(&h).surface.input.anchor, 9);
    assert_eq!(field(&h).surface.input.caret, 7);
    assert!(
        h.event(&ContentEvent::TextCommand(
            "moveParagraphForwardAndModifySelection:".into()
        ))
        .consumed
    );
    assert_eq!(field(&h).surface.input.anchor, 9);
    assert_eq!(field(&h).surface.input.caret, 15);

    // At the paragraph start, backward selection reaches the previous
    // paragraph start and includes the intervening CRLF.
    field_mut(&mut h).surface.input.select(7, false);
    assert!(
        h.event(&ContentEvent::TextCommand(
            "moveParagraphBackwardAndModifySelection:".into()
        ))
        .consumed
    );
    assert_eq!(field(&h).surface.input.anchor, 7);
    assert_eq!(field(&h).surface.input.caret, 0);

    field_mut(&mut h).surface.input.select(9, false);
    assert!(
        h.event(&ContentEvent::TextCommand("moveToLeftEndOfLine:".into()))
            .consumed
    );
    assert_eq!(field(&h).surface.input.caret, 7);
    assert!(
        h.event(&ContentEvent::TextCommand("moveToRightEndOfLine:".into()))
            .consumed
    );
    assert_eq!(field(&h).surface.input.caret, 13);
}

#[test]
fn transpose_swaps_whole_graphemes_and_is_one_undo_transaction() {
    for (value, caret, expected, expected_caret) in [
        ("alpha beta gamma", 10, "alpha bet agamma", 11),
        (
            "first line\nsecond word\nlast",
            11,
            "first lines\necond word\nlast",
            12,
        ),
        (
            "first line\nsecond word\nlast",
            22,
            "first line\nsecond wodr\nlast",
            22,
        ),
        ("first\r\nsecond\r\nlast", 13, "first\r\nsecodn\r\nlast", 13),
        ("A👨‍👩‍👧‍👦B", 1, "👨‍👩‍👧‍👦AB", 12),
    ] {
        let mut h = host(value, 500.);
        field_mut(&mut h).surface.input.select(caret, false);
        assert!(
            h.event(&ContentEvent::TextCommand("transpose:".into()))
                .consumed
        );
        assert_eq!(field(&h).surface.input.text, expected);
        assert_eq!(field(&h).surface.input.caret, expected_caret);
        assert!(field_mut(&mut h).surface.input.undo());
        assert_eq!(field(&h).surface.input.text, value);
        assert_eq!(field(&h).surface.input.selection(), caret..caret);
    }

    let mut h = host("alpha beta gamma", 500.);
    field_mut(&mut h).surface.input.select_range(3..7);
    assert!(
        h.event(&ContentEvent::TextCommand("transpose:".into()))
            .consumed
    );
    assert_eq!(field(&h).surface.input.text, "alpha beta gamma");
    assert_eq!(field(&h).surface.input.selection(), 3..7);
}

#[test]
fn decomposing_backward_delete_removes_the_final_nfd_scalar_and_undoes() {
    let cases = [
        ("é", "e"),
        ("e\u{301}", "e"),
        ("a\u{301}\u{327}", "a\u{327}"),
        ("각", "가"),
        ("👍🏽", "👍"),
        ("👩‍👩‍👧‍👦", "👩‍👩‍👧‍"),
        ("🇨🇳", "🇨"),
        ("ﬁ", ""),
        ("1️⃣", "1️"),
    ];
    for (value, after_one) in cases {
        let mut h = host(value, 500.);
        let end = value.encode_utf16().count();
        field_mut(&mut h).surface.input.select(end, false);
        assert!(
            h.event(&ContentEvent::TextCommand(
                "deleteBackwardByDecomposingPreviousCharacter:".into()
            ))
            .consumed
        );
        assert_eq!(field(&h).surface.input.text, after_one, "{value:?}");
        assert_eq!(
            field(&h).surface.input.caret,
            after_one.encode_utf16().count()
        );
        assert!(field_mut(&mut h).surface.input.undo());
        assert_eq!(field(&h).surface.input.text, value);
        assert_eq!(field(&h).surface.input.selection(), end..end);
        assert!(field_mut(&mut h).surface.input.redo());
        assert_eq!(field(&h).surface.input.text, after_one);
    }
}

#[test]
fn decomposing_delete_treats_crlf_atomically_and_selection_as_one_range() {
    let mut h = host("a\r\nb", 500.);
    field_mut(&mut h).surface.input.select(4, false);
    assert!(
        h.event(&ContentEvent::TextCommand(
            "deleteBackwardByDecomposingPreviousCharacter:".into()
        ))
        .consumed
    );
    assert_eq!(field(&h).surface.input.text, "a\r\n");
    assert!(
        h.event(&ContentEvent::TextCommand(
            "deleteBackwardByDecomposingPreviousCharacter:".into()
        ))
        .consumed
    );
    assert_eq!(field(&h).surface.input.text, "a");
    assert!(field_mut(&mut h).surface.input.undo());
    assert_eq!(field(&h).surface.input.text, "a\r\n");
    assert!(field_mut(&mut h).surface.input.redo());
    assert_eq!(field(&h).surface.input.text, "a");

    let mut h = host("éabc", 500.);
    field_mut(&mut h).surface.input.select_range(0..2);
    assert!(
        h.event(&ContentEvent::TextCommand(
            "deleteBackwardByDecomposingPreviousCharacter:".into()
        ))
        .consumed
    );
    assert_eq!(field(&h).surface.input.text, "bc");
    assert_eq!(field(&h).surface.input.caret, 0);
}

#[test]
fn paragraph_kill_appends_and_repeated_yank_reuses_the_shared_value() {
    let mut h = host("alpha beta\ngamma delta\nomega", 500.);
    field_mut(&mut h).surface.input.select(6, false);
    assert!(text_command(&mut h, "deleteToEndOfParagraph:"));
    assert_eq!(field(&h).surface.input.text, "alpha \ngamma delta\nomega");
    assert!(text_command(&mut h, "deleteToEndOfParagraph:"));
    assert_eq!(field(&h).surface.input.text, "alpha gamma delta\nomega");
    assert!(text_command(&mut h, "deleteToEndOfParagraph:"));
    assert_eq!(field(&h).surface.input.text, "alpha \nomega");

    assert!(text_command(&mut h, "yank:"));
    assert_eq!(
        field(&h).surface.input.text,
        "alpha beta\ngamma delta\nomega"
    );
    assert!(text_command(&mut h, "yank:"));
    assert_eq!(
        field(&h).surface.input.text,
        "alpha beta\ngamma deltabeta\ngamma delta\nomega"
    );
}

#[test]
fn line_kill_prepends_and_switching_kill_selector_resets_coalescing() {
    let mut h = host("alpha beta\ngamma delta\nomega", 500.);
    field_mut(&mut h).surface.input.select(6, false);
    assert!(text_command(&mut h, "deleteToEndOfLine:"));
    assert!(text_command(&mut h, "deleteToEndOfLine:"));
    assert!(text_command(&mut h, "deleteToEndOfLine:"));
    assert_eq!(field(&h).surface.input.text, "alpha \nomega");
    assert!(text_command(&mut h, "yank:"));
    assert_eq!(
        field(&h).surface.input.text,
        "alpha gamma delta\nbeta\nomega"
    );

    let mut h = host("alpha beta\ngamma delta\nomega", 500.);
    field_mut(&mut h).surface.input.select(6, false);
    assert!(text_command(&mut h, "deleteToEndOfParagraph:"));
    assert!(text_command(&mut h, "deleteToEndOfLine:"));
    assert!(text_command(&mut h, "yank:"));
    assert_eq!(field(&h).surface.input.text, "alpha \ngamma delta\nomega");
}

#[test]
fn ordinary_deletes_rebase_kill_chain_but_insert_and_move_break_it() {
    let mut h = host("alpha beta\ngamma delta\nomega", 500.);
    field_mut(&mut h).surface.input.select(6, false);
    assert!(text_command(&mut h, "deleteToEndOfParagraph:"));
    assert!(text_command(&mut h, "deleteBackward:"));
    assert!(text_command(&mut h, "deleteToEndOfParagraph:"));
    assert!(text_command(&mut h, "yank:"));
    assert_eq!(
        field(&h).surface.input.text,
        "alphabeta\ngamma delta\nomega"
    );

    let mut h = host("alpha beta\ngamma delta\nomega", 500.);
    field_mut(&mut h).surface.input.select(6, false);
    assert!(text_command(&mut h, "deleteToEndOfParagraph:"));
    assert!(
        h.event(&ContentEvent::Commit {
            text: "X".into(),
            replacement: None,
        })
        .consumed
    );
    assert!(text_command(&mut h, "deleteToEndOfParagraph:"));
    assert!(text_command(&mut h, "yank:"));
    assert_eq!(field(&h).surface.input.text, "alpha X\ngamma delta\nomega");

    let mut h = host("alpha beta\ngamma delta\nomega", 500.);
    field_mut(&mut h).surface.input.select(6, false);
    assert!(text_command(&mut h, "deleteToEndOfParagraph:"));
    assert!(text_command(&mut h, "moveToEndOfParagraph:"));
    assert!(text_command(&mut h, "deleteToEndOfParagraph:"));
    assert!(text_command(&mut h, "yank:"));
    assert_eq!(field(&h).surface.input.text, "alpha \ngamma delta\nomega");
}

#[test]
fn kill_value_is_shared_across_editors_but_new_owner_kill_replaces_it() {
    let mut owner = host("alpha beta\ngamma", 500.);
    field_mut(&mut owner).surface.input.select(6, false);
    assert!(text_command(&mut owner, "deleteToEndOfParagraph:"));

    let mut other = host("target ", 500.);
    field_mut(&mut other).surface.input.select(7, false);
    assert!(text_command(&mut other, "yank:"));
    assert_eq!(field(&other).surface.input.text, "target beta");

    let mut other = host("target ", 500.);
    field_mut(&mut other).surface.input.select(7, false);
    assert!(text_command(&mut other, "deleteToBeginningOfParagraph:"));
    assert_eq!(field(&other).surface.input.text, "");
    assert!(text_command(&mut other, "yank:"));
    assert_eq!(field(&other).surface.input.text, "target ");
}

#[test]
fn another_editors_move_does_not_break_the_original_owner_kill_chain() {
    let mut owner = host("alpha beta\ngamma", 500.);
    field_mut(&mut owner).surface.input.select(6, false);
    assert!(text_command(&mut owner, "deleteToEndOfParagraph:"));

    let mut other = host("target ", 500.);
    field_mut(&mut other).surface.input.select(7, false);
    assert!(text_command(&mut other, "moveLeft:"));

    assert!(text_command(&mut owner, "deleteToEndOfParagraph:"));
    assert!(text_command(&mut owner, "yank:"));
    assert_eq!(field(&owner).surface.input.text, "alpha beta\ngamma");
}

#[test]
fn crlf_kill_yank_preserves_bytes_and_crlf_paragraph_tail_is_noop() {
    let mut h = host("first\r\nsecond\r\nlast", 500.);
    field_mut(&mut h).surface.input.select(3, false);
    assert!(text_command(&mut h, "deleteToEndOfParagraph:"));
    assert_eq!(field(&h).surface.input.text, "fir\r\nsecond\r\nlast");
    assert!(text_command(&mut h, "deleteToEndOfParagraph:"));
    assert_eq!(field(&h).surface.input.text, "fir\r\nsecond\r\nlast");
    assert!(text_command(&mut h, "yank:"));
    assert_eq!(field(&h).surface.input.text, "first\r\nsecond\r\nlast");

    let mut h = host("first\r\nsecond\r\nlast", 500.);
    field_mut(&mut h).surface.input.select_range(3..9);
    assert!(text_command(&mut h, "deleteToEndOfParagraph:"));
    assert_eq!(field(&h).surface.input.text, "fircond\r\nlast");
    assert!(text_command(&mut h, "yank:"));
    assert_eq!(field(&h).surface.input.text, "first\r\nsecond\r\nlast");
}

#[test]
fn delete_selector_only_deletes_a_selection_and_ignoring_field_editor_aliases_work() {
    let mut h = host("alpha beta gamma", 500.);
    field_mut(&mut h).surface.input.select(10, false);
    assert!(text_command(&mut h, "delete:"));
    assert_eq!(field(&h).surface.input.text, "alpha beta gamma");
    field_mut(&mut h).surface.input.select_range(3..10);
    assert!(text_command(&mut h, "delete:"));
    assert_eq!(field(&h).surface.input.text, "alp gamma");
    assert!(field_mut(&mut h).surface.input.undo());
    assert_eq!(field(&h).surface.input.selection(), 3..10);

    let mut h = host("ab", 500.);
    field_mut(&mut h).surface.input.select(1, false);
    assert!(text_command(&mut h, "insertNewlineIgnoringFieldEditor:"));
    assert!(text_command(&mut h, "moveBackward:"));
    assert_eq!(field(&h).surface.input.text, "a\nb");
    assert_eq!(field(&h).surface.input.caret, 1);
    assert!(text_command(&mut h, "insertTabIgnoringFieldEditor:"));
    assert_eq!(field(&h).surface.input.text, "a\t\nb");
}

#[test]
fn empty_paragraph_edges_remain_on_the_break_and_delete_as_one_transaction() {
    let mut h = host("first\n\nlast", 500.);
    field_mut(&mut h).surface.input.select(6, false);
    assert!(
        h.event(&ContentEvent::TextCommand(
            "moveToBeginningOfParagraph:".into()
        ))
        .consumed
    );
    assert_eq!(field(&h).surface.input.caret, 6);
    assert!(
        h.event(&ContentEvent::TextCommand("moveToEndOfParagraph:".into()))
            .consumed
    );
    assert_eq!(field(&h).surface.input.caret, 6);
    assert!(
        h.event(&ContentEvent::TextCommand("selectParagraph:".into()))
            .consumed
    );
    assert_eq!(field(&h).surface.input.selected_text(), "\n");

    field_mut(&mut h).surface.input.select(6, false);
    assert!(
        h.event(&ContentEvent::TextCommand("deleteToEndOfParagraph:".into()))
            .consumed
    );
    assert_eq!(field(&h).surface.input.text, "first\nlast");
    assert!(field_mut(&mut h).surface.input.undo());
    assert_eq!(field(&h).surface.input.text, "first\n\nlast");
}

fn key(control: tekes_ui_controls::ControlKey) -> ContentKeyEvent {
    ContentKeyEvent {
        key: Some(control),
        down: true,
        repeat: false,
        modifiers: InputModifiers::default(),
        native_code: 0,
        characters: String::new(),
        characters_ignoring_modifiers: String::new(),
    }
}
#[test]
fn stale_geometry_is_never_guessed_and_existing_backend_refreshes_consecutive_commands() {
    let mut h = host("one\nsecond", 500.);
    field_mut(&mut h).surface.input.insert("new\n", Some(0..0));
    let caret = field(&h).surface.input.caret;
    assert!(
        !h.event(&ContentEvent::TextCommand("moveToEndOfLine:".into()))
            .consumed
    );
    assert_eq!(field(&h).surface.input.caret, caret);
    assert!(
        h.event_with_text(
            &ContentEvent::TextCommand("moveToEndOfLine:".into()),
            &mut Text
        )
        .unwrap()
        .consumed
    );
    assert_eq!(field(&h).surface.input.caret, 7);
    assert_eq!(field(&h).surface.shaped, field(&h).surface.input.text);
    h.event_with_text(
        &ContentEvent::Commit {
            text: "X\n".into(),
            replacement: None,
        },
        &mut Text,
    )
    .unwrap();
    h.event_with_text(&ContentEvent::TextCommand("moveUp:".into()), &mut Text)
        .unwrap();
    assert!(field(&h).surface.input.caret < 9);
}
#[test]
fn metadata_and_selection_aliases_share_line_geometry_and_legal_utf16_boundaries() {
    use tekes_ui_controls::ControlKey;
    let mut h = host("first\nsecond\nlast", 500.);
    field_mut(&mut h).surface.input.select(9, false);
    let mut input = key(ControlKey::Left);
    input.modifiers.command = true;
    h.event_with_text(&ContentEvent::KeyInput(input), &mut Text)
        .unwrap();
    assert_eq!(field(&h).surface.input.caret, 6);
    let mut input = key(ControlKey::Right);
    input.modifiers.command = true;
    input.modifiers.shift = true;
    h.event_with_text(&ContentEvent::KeyInput(input), &mut Text)
        .unwrap();
    assert_eq!(field(&h).surface.input.selection(), 6..12);
    for value in [
        "甲😀e\u{301}\r\n乙\u{2028}丙\r丁\n",
        "中文 😀 e\u{301}\nEnglish 😀\n",
    ] {
        let mut h = host(value, 500.);
        let lines = field(&h).surface.paragraph.as_ref().unwrap().lines.clone();
        for line in lines {
            field_mut(&mut h)
                .surface
                .input
                .select(line.utf16.start, false);
            h.event_with_text(
                &ContentEvent::TextCommand("moveToEndOfLineAndModifySelection:".into()),
                &mut Text,
            )
            .unwrap();
            let expected = line.utf16.start
                + value[line.bytes.clone()]
                    .trim_end_matches(&['\r', '\n', '\u{2028}', '\u{2029}'][..])
                    .encode_utf16()
                    .count();
            let input = &field(&h).surface.input;
            assert_eq!(input.caret, expected);
            assert!(input.boundaries().contains(&input.caret));
            assert_eq!(input.anchor, line.utf16.start);
        }
    }
}
#[test]
fn pointer_soft_edge_uses_the_same_affinity_and_authority_ack_preserves_it() {
    use tekes_ui_controls::{DraftRevision, ExternalSelection, ExternalUndo};
    let mut h = host(&"abcdef ".repeat(30), 180.);
    let s = &field(&h).surface;
    let line = s.paragraph.as_ref().unwrap().lines[0].clone();
    let origin = s.origin();
    let p = tekes_ui_scene::Point::new(
        (origin.x + line.width as f32 + 0.5).min(s.inner.max_x() - 0.5),
        origin.y + line.top as f32 + line.height as f32 / 2.,
    );
    h.event(&ContentEvent::PointerDown(p));
    h.event(&ContentEvent::PointerUp(p));
    let s = &field(&h).surface;
    assert_eq!(s.input.caret, line.utf16.end);
    assert_eq!(s.caret_doc(s.input.caret).min_y(), line.top as f32);
    let surface = &mut field_mut(&mut h).surface;
    let baseline = surface.input.external_baseline();
    let value = surface.input.text.clone();
    surface.apply_external_value(
        &baseline,
        &DraftRevision {
            identity: "doc".into(),
            version: 1,
            payload: value,
        },
        ExternalSelection::Preserve,
        ExternalUndo::Preserve,
    );
    assert_eq!(
        surface.caret_doc(surface.input.caret).min_y(),
        line.top as f32
    );
}
#[test]
fn one_native_vertical_chain_does_not_repeat_and_replacement_cancels_button_fallback() {
    use tekes_ui_controls::ControlKey;
    let mut h = host("abcdef\nx\nabcdef", 500.);
    field_mut(&mut h).surface.input.select(5, false);
    h.event_with_text(
        &ContentEvent::TextKeyStart {
            id: 1,
            key: key(ControlKey::Down),
            composing: false,
        },
        &mut Text,
    )
    .unwrap();
    h.event_with_text(&ContentEvent::TextCommand("moveDown:".into()), &mut Text)
        .unwrap();
    let r = h
        .event_with_text(
            &ContentEvent::TextKeyEnd {
                id: 1,
                input_context_consumed: false,
            },
            &mut Text,
        )
        .unwrap();
    assert_eq!(
        r.text_key.unwrap().disposition,
        TextKeyDisposition::EditHandled
    );
    assert_eq!(field(&h).surface.input.caret, 8);
    h.event(&ContentEvent::TextKeyEnd {
        id: 1,
        input_context_consumed: false,
    });
    assert_eq!(field(&h).surface.input.caret, 8);
    let root = ComponentId(10);
    let button = ComponentId(11);
    let mut h = ContentHost::new(Content::new(
        root,
        ListContent::new(
            "root",
            vec![Content::new(button, Button::new("old"))],
        ),
    ))
    .unwrap();
    let frame = Rect::new(0., 0., 400., 300.);
    h.render(
        &mut Scene::new(),
        &mut Text,
        frame,
        frame,
        Theme::LIGHT.into(),
    )
    .unwrap();
    h.focus(Some(button), true);
    h.set_action_events(true);
    h.event(&ContentEvent::TextKeyStart {
        id: 1,
        key: key(ControlKey::Enter),
        composing: false,
    });
    h.event(&ContentEvent::TextCommand("insertNewline:".into()));
    h.get_mut(root)
        .unwrap()
        .component_mut::<ListContent>()
        .unwrap()
        .rows
        .children[0] = Content::new(button, Button::new("replacement"));
    h.focus(Some(button), true);
    let r = h.event(&ContentEvent::TextKeyEnd {
        id: 1,
        input_context_consumed: false,
    });
    assert_eq!(
        r.text_key.unwrap().disposition,
        TextKeyDisposition::Cancelled
    );
    assert!(h.drain_actions().is_empty());
}
#[test]
fn combo_callback_only_arrows_keep_overlay_priority_and_ime_keeps_editor_ownership() {
    use tekes_ui_composition_consumer::combo_status::{QUERY, Scenario};
    use tekes_ui_controls::ControlKey;
    let mut s = Scenario::new(vec![
        "Automatic".into(),
        "Compact".into(),
        "Comfortable".into(),
    ])
    .unwrap();
    let frame = Rect::new(0., 0., 400., 420.);
    s.host
        .render(
            &mut Scene::new(),
            &mut Text,
            frame,
            frame,
            Theme::LIGHT.into(),
        )
        .unwrap();
    s.host.focus(Some(QUERY), true);
    s.host
        .event_with_text(
            &ContentEvent::Commit {
                text: "Com".into(),
                replacement: None,
            },
            &mut Text,
        )
        .unwrap();
    s.host
        .render(
            &mut Scene::new(),
            &mut Text,
            frame,
            frame,
            Theme::LIGHT.into(),
        )
        .unwrap();
    assert_eq!(s.active(), Some(1));
    s.host
        .event_with_text(
            &ContentEvent::TextKeyStart {
                id: 1,
                key: key(ControlKey::Down),
                composing: false,
            },
            &mut Text,
        )
        .unwrap();
    s.host
        .event_with_text(&ContentEvent::TextCommand("moveDown:".into()), &mut Text)
        .unwrap();
    s.host
        .event_with_text(
            &ContentEvent::TextKeyEnd {
                id: 1,
                input_context_consumed: false,
            },
            &mut Text,
        )
        .unwrap();
    assert_eq!(s.active(), Some(2));
    assert_eq!(s.query(), "Com");
    assert!(s.selected().is_none());
    s.host
        .event_with_text(
            &ContentEvent::Preedit {
                text: "Com".into(),
                selection: 3..3,
                clauses: vec![],
                replacement: Some(0..3),
            },
            &mut Text,
        )
        .unwrap();
    s.host
        .event_with_text(
            &ContentEvent::TextKeyStart {
                id: 2,
                key: key(ControlKey::Up),
                composing: true,
            },
            &mut Text,
        )
        .unwrap();
    s.host
        .event_with_text(&ContentEvent::TextCommand("moveUp:".into()), &mut Text)
        .unwrap();
    s.host
        .event_with_text(
            &ContentEvent::TextKeyEnd {
                id: 2,
                input_context_consumed: false,
            },
            &mut Text,
        )
        .unwrap();
    assert_eq!(s.active(), Some(2));
    assert!(s.host.focused_text().unwrap().input.marked.is_some());
    assert_eq!(s.host.focused(), Some(QUERY));
}

#[test]
fn combo_selection_is_activation_even_when_its_semantic_role_is_text_input() {
    use tekes_ui_composition_consumer::combo_status::{QUERY, Scenario};
    use tekes_ui_controls::ControlKey;
    let mut s = Scenario::new(vec!["Compact".into(), "Comfortable".into()]).unwrap();
    let frame = Rect::new(0., 0., 400., 420.);
    s.host.set_action_events(true);
    s.host
        .render(
            &mut Scene::new(),
            &mut Text,
            frame,
            frame,
            Theme::LIGHT.into(),
        )
        .unwrap();
    s.host.focus(Some(QUERY), true);
    s.host
        .event_with_text(
            &ContentEvent::Commit {
                text: "Com".into(),
                replacement: None,
            },
            &mut Text,
        )
        .unwrap();
    s.host
        .render(
            &mut Scene::new(),
            &mut Text,
            frame,
            frame,
            Theme::LIGHT.into(),
        )
        .unwrap();
    s.host
        .event_with_text(
            &ContentEvent::TextKeyStart {
                id: 1,
                key: key(ControlKey::Enter),
                composing: false,
            },
            &mut Text,
        )
        .unwrap();
    s.host
        .event_with_text(
            &ContentEvent::TextCommand("insertNewline:".into()),
            &mut Text,
        )
        .unwrap();
    s.host
        .event_with_text(
            &ContentEvent::TextKeyEnd {
                id: 1,
                input_context_consumed: false,
            },
            &mut Text,
        )
        .unwrap();
    assert_eq!(s.selected(), Some("Compact"));
    assert_eq!(
        s.host.drain_actions(),
        vec![ContentAction {
            target: QUERY,
            kind: ContentActionKind::Activate,
            source: ContentActionSource::Keyboard
        }]
    );
}

#[cfg(feature = "token-audit")]
#[test]
fn navigation_uses_original_field_tokens_in_all_themes_and_brands() {
    let mut cases = Vec::new();
    for theme in Theme::ALL {
        for brand in 0..3 {
            for read_only in [false, true] {
                let mut h = host("abcdef\nx\nabcdef", 500.);
                field_mut(&mut h).surface.input.read_only = read_only;
                field_mut(&mut h).surface.input.select(5, false);
                let rect = Rect::new(30., 40., 500., 250.);
                let (scene, lookups) = tekes_ui_tokens::audit::capture(|| {
                    h.event_with_text(&ContentEvent::TextCommand("moveDown:".into()), &mut Text)
                        .unwrap();
                    assert_eq!(field(&h).surface.input.caret, 8);
                    h.event_with_text(&ContentEvent::TextCommand("moveDown:".into()), &mut Text)
                        .unwrap();
                    assert_eq!(field(&h).surface.input.caret, 14);
                    h.event_with_text(
                        &ContentEvent::TextCommand("moveUpAndModifySelection:".into()),
                        &mut Text,
                    )
                    .unwrap();
                    assert_eq!(field(&h).surface.input.selection(), 8..14);
                    let mut scene = Scene::new();
                    h.render(
                        &mut scene,
                        &mut Text,
                        rect,
                        rect,
                        tekes_ui_tokens::Palette::new(theme, brand).unwrap(),
                    )
                    .unwrap();
                    scene
                });
                assert!(!scene.primitives().is_empty());
                assert!(lookups.iter().any(|(family, id)| family == "color"
                    && id
                        == if read_only {
                            "surface-1"
                        } else {
                            "transcript-input"
                        }));
                assert_eq!(field(&h).surface.input.text, "abcdef\nx\nabcdef");
                cases.push(serde_json::json!({"component":"tui.text-field","theme":format!("{theme:?}"),"brand":brand,"read_only":read_only,"state_asserted":true,"lookups":lookups}));
            }
        }
    }
    assert_eq!(cases.len(), 24);
    if let Some(path) = std::env::var_os("TEKESUI_EDITOR_NAVIGATION_TOKEN_AUDIT_OUT") {
        std::fs::write(path,serde_json::to_vec_pretty(&serde_json::json!({"format":"tekesui-runtime-token-observation/1","complete":false,"scope":"public multiline Field navigation, editable/readonly,4themes3brands,CoreText and no-op glyph painter","excluded":["physicalIME/VoiceOver","complete dependency extraction","compiled style constants"],"cases":cases})).unwrap()).unwrap();
    }
}

#[test]
fn empty_line_kill_keeps_the_existing_shared_yank_value() {
    let mut source = host("KNOWN-SEED", 500.);
    assert!(
        source
            .event(&ContentEvent::TextCommand("deleteToEndOfParagraph:".into()))
            .consumed
    );
    assert_eq!(field(&source).surface.input.text, "");
    let mut target = host("target", 500.);
    assert!(
        target
            .event(&ContentEvent::TextCommand(
                "deleteToBeginningOfLine:".into()
            ))
            .consumed
    );
    assert_eq!(field(&target).surface.input.text, "target");
    assert!(
        target
            .event(&ContentEvent::TextCommand("yank:".into()))
            .consumed
    );
    assert_eq!(field(&target).surface.input.text, "KNOWN-SEEDtarget");
}
