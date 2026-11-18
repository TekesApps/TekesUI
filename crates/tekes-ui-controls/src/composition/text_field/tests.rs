use super::super::*;
use super::*;
use crate::TextSurfacePresentation;

#[cfg(test)]
mod prepared_field_tests {
    use super::*;
    fn pointer(point: Point) -> action_preparation::OwnedActionRequest {
        action_preparation::OwnedActionRequest::Pointer(ContentEvent::PointerDown(point))
    }
    #[test]
    fn content_focus_source_changes_decoration_without_blurring_selection() {
        struct Text;
        impl crate::TextSurfaceTextRenderer for Text {
            fn push_visible(
                &mut self,
                _: &mut Scene,
                _: &tekes_ui_text::Paragraph,
                _: Point,
                _: tekes_ui_tokens::Rgba,
                _: Rect,
            ) {
            }
        }
        let mut field = TextFieldContent::read_only_content(
            "Document",
            "selectable text",
            proposal::TextRole::UiBody,
        );
        field.surface.outer = Rect::new(0., 0., 200., 100.);
        field.surface.input.select(5, false);
        field.surface.input.select(10, true);
        let before = field.surface.input.action_snapshot();
        for theme in tekes_ui_tokens::Theme::ALL {
            // Host can update the same owner's focus source without a blur.
            for (focused, visible) in [(true, false), (true, true), (true, false), (false, false)] {
                field.set_focus(focused, visible);
                let mut scene = Scene::new();
                field
                    .surface
                    .draw(&mut Text, &mut scene, theme, field.focused, false);
                let outlines = scene
                    .primitives()
                    .iter()
                    .filter(|p| {
                        matches!(p,
                    tekes_ui_scene::Primitive::Quad(q)
                        if q.border_color == tekes_ui_tokens::Palette::from(theme).color("border-focused").unwrap()
                            && q.border_width == proposal::metric("stroke", "c-focus-width").unwrap()
                )
                    })
                    .count();
                assert_eq!(outlines, usize::from(focused && visible));
                assert_eq!(field.focused, focused);
                assert_eq!(field.surface.input.action_snapshot(), before);
            }
        }
    }

    #[test]
    fn prepared_field_selection_is_pure_rejects_changed_state_and_keeps_readonly_selection() {
        let mut field = TextFieldContent::new("Draft", "A😀B");
        field.surface.input.read_only = true;
        let request = action_preparation::OwnedActionRequest::Accessibility(
            None,
            AccessibilityAction::SetSelection(1..2),
        );
        let before = field.surface.input.action_snapshot();
        let decision = field.decide_action(&request).unwrap();
        assert!(decision.response.consumed && decision.response.changed);
        assert_eq!(field.surface.input.action_snapshot(), before);
        field.surface.input.select(4, false);
        assert_ne!(Some(&decision), field.decide_action(&request).as_ref());
        let decision = field.decide_action(&request).unwrap();
        field.apply_action(decision);
        assert_eq!(field.surface.input.selection(), 1..3);
        assert_eq!(field.surface.input.text, "A😀B");
        assert!(field.surface.input.read_only);
    }
    #[test]
    fn prepared_field_pointer_refusal_preserves_reader_focus_and_composition() {
        let mut field = TextFieldContent::new("Reader", "draft");
        field.surface.outer = Rect::new(0., 0., 100., 40.);
        field.surface.inner = field.surface.outer;
        let mut reading = ReadingController::default();
        reading.follow_latest(&mut field.surface);
        field.reading = Some(reading);
        let outside = field
            .decide_action(&pointer(Point::new(200., 200.)))
            .unwrap();
        assert!(!outside.response.consumed);
        field.apply_action(outside);
        assert!(field.reading.as_ref().unwrap().is_following_latest());
        assert!(!field.has_focus());
        field.surface.input.preedit("中", 1..1, None);
        let before = field.surface.input.action_snapshot();
        let response = field.accessibility_action(None, &AccessibilityAction::SetSelection(0..0));
        assert_eq!(response, ContentResponse::default());
        assert_eq!(field.surface.input.action_snapshot(), before);
        let decision = field.decide_action(&pointer(Point::new(10., 10.))).unwrap();
        assert_eq!(field.surface.input.action_snapshot(), before);
        assert!(field.reading.as_ref().unwrap().is_following_latest());
        field.apply_action(decision);
        assert!(!field.reading.as_ref().unwrap().is_following_latest());
        assert_eq!(field.surface.input.text, "中draft");
        assert!(field.surface.input.marked.is_none());
    }
    #[test]
    fn prepared_field_selection_focus_rect_matches_actual_scrollbar_reveal_gates() {
        for gate in [0, 1, 2] {
            let mut field = TextFieldContent::new("Content", "ab").scrollbar(true);
            field.surface.presentation = TextSurfacePresentation::Content;
            field
                .surface
                .layout_with(Rect::new(0., 0., 100., 40.), None, 1., 1., |_, _| {
                    std::rc::Rc::new(tekes_ui_text::Paragraph {
                        lines: vec![tekes_ui_text::Line {
                            index: 0,
                            utf16: 0..2,
                            bytes: 0..2,
                            ascent: 15.,
                            descent: 5.,
                            leading: 0.,
                            height: 20.,
                            top: 0.,
                            baseline_y: 15.,
                            width: 600.,
                            width_trimmed: 600.,
                            runs: vec![],
                            carets: vec![(0, 0.), (1, 300.), (2, 600.)],
                            graphemes: vec![],
                            has_newline: false,
                        }],
                        width: 600.,
                        height: 20.,
                        default_line_height: 20.,
                        fonts: vec![],
                        utf16_len: 2,
                    })
                });
            field.surface.scroll.scroll_by(120., 0.);
            if gate != 0 {
                let track = field
                    .surface
                    .track(false)
                    .inset(-tekes_ui_tokens::Spacing::Micro.value());
                let thumb = field.surface.scroll.thumb(false, track, 24.);
                assert!(
                    field
                        .surface
                        .pointer_down(thumb.center(), false, 1, 24.)
                        .is_some()
                );
                if gate == 2 {
                    field.surface.pointer_up();
                }
            }
            let before = field.surface.scroll.offset;
            let request = action_preparation::OwnedActionRequest::Accessibility(
                None,
                AccessibilityAction::SetSelection(2..2),
            );
            let decision = field.decide_action(&request).unwrap();
            let predicted = decision.focus_rect.unwrap();
            field.apply_action(decision);
            assert_eq!(predicted, field.surface.caret(), "gate={gate}");
            if gate == 0 {
                assert_ne!(field.surface.scroll.offset, before);
            } else {
                assert_eq!(field.surface.scroll.offset, before);
            }
        }
    }

    #[test]
    fn single_line_horizontal_scrollbar_defaults_hidden_and_can_be_enabled_without_disabling_reveal()
     {
        let mut hidden = TextFieldContent::new("Draft", "a long single-line value");
        assert_eq!(
            hidden.surface.scroll.scroll_policy(),
            (crate::ScrollPolicy::Auto, crate::ScrollPolicy::Disabled)
        );
        hidden
            .surface
            .scroll
            .layout(Size::new(48., 24.), Size::new(240., 24.));
        assert!(!hidden.surface.scroll.scrollbar_visible(false));
        assert!(hidden.surface.scroll.axis_enabled(false));
        assert!(hidden.surface.scroll.reveal(Rect::new(230., 0., 2., 18.)));
        assert!(
            hidden.surface.scroll.offset.x > 0.,
            "hidden indicators must not prevent revealing the far insertion point"
        );

        let mut visible =
            TextFieldContent::new("Draft", "a long single-line value").scrollbar(true);
        visible
            .surface
            .scroll
            .layout(Size::new(48., 24.), Size::new(240., 24.));
        assert!(visible.surface.scroll.scrollbar_visible(false));
        assert_eq!(
            visible.surface.scroll.scroll_policy(),
            (crate::ScrollPolicy::Auto, crate::ScrollPolicy::Disabled)
        );

        let multiline =
            TextFieldContent::read_only_content("Document", "content", proposal::TextRole::UiBody);
        assert!(multiline.surface.scroll.scrollbar_requested(false));
        assert!(multiline.surface.scroll.scrollbar_requested(true));
    }

    #[test]
    fn prepared_field_host_focus_refusal_does_not_run_target_effects() {
        struct Text;
        impl crate::TextSurfaceTextRenderer for Text {
            fn push_visible(
                &mut self,
                _: &mut Scene,
                _: &tekes_ui_text::Paragraph,
                _: Point,
                _: tekes_ui_tokens::Rgba,
                _: Rect,
            ) {
            }
        }
        impl crate::TextSurfaceHost for Text {
            fn shape_surface(
                &mut self,
                text: &str,
                _: proposal::TextRole,
                _: Option<f32>,
            ) -> std::rc::Rc<tekes_ui_text::Paragraph> {
                let len = text.encode_utf16().count();
                let width = len as f64 * 7.;
                std::rc::Rc::new(tekes_ui_text::Paragraph {
                    lines: vec![tekes_ui_text::Line {
                        index: 0,
                        utf16: 0..len,
                        bytes: 0..text.len(),
                        ascent: 15.,
                        descent: 5.,
                        leading: 0.,
                        height: 20.,
                        top: 0.,
                        baseline_y: 15.,
                        width,
                        width_trimmed: width,
                        runs: vec![],
                        carets: (0..=len).map(|i| (i, i as f64 * 7.)).collect(),
                        graphemes: vec![],
                        has_newline: false,
                    }],
                    width,
                    height: 20.,
                    default_line_height: 20.,
                    fonts: vec![],
                    utf16_len: len,
                })
            }
        }
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
                _: tekes_ui_tokens::Rgba,
            ) {
            }
            fn text_surface_host(&mut self) -> Option<&mut dyn crate::TextSurfaceHost> {
                Some(self)
            }
        }
        let id = ComponentId(81940);
        for exhausted in [false, true] {
            let mut field = TextFieldContent::new("Draft", "draft");
            field.show_metadata = false;
            field.surface.presentation = TextSurfacePresentation::Content;
            field.viewport = TextViewport::Fixed(40.);
            field.surface.input.preedit("中", 1..1, None);
            let mut host = ContentHost::new(Content::new(id, field)).unwrap();
            let frame = Rect::new(0., 0., 100., 40.);
            host.render(
                &mut Scene::new(),
                &mut Text,
                frame,
                frame,
                tekes_ui_tokens::Theme::LIGHT.into(),
            )
            .unwrap();
            if exhausted {
                host.focus_epoch = u64::MAX;
            }
            let field = host
                .get(id)
                .unwrap()
                .component::<TextFieldContent>()
                .unwrap();
            let before = field.surface.input.action_snapshot();
            let scroll = format!("{:?}", field.surface.scroll);
            let point = field.surface.outer.center();
            let response = host.event(&ContentEvent::PointerDown(point));
            let field = host
                .get(id)
                .unwrap()
                .component::<TextFieldContent>()
                .unwrap();
            if exhausted {
                assert_eq!(response, ContentResponse::default());
                assert_eq!(host.focused(), None);
                assert!(!host.pointer_focus_requested());
                assert_eq!(host.pointer_capture(), None);
                assert_eq!(field.surface.input.action_snapshot(), before);
                assert_eq!(format!("{:?}", field.surface.scroll), scroll);
                assert!(!field.has_focus());
            } else {
                assert!(response.consumed);
                assert_eq!(host.focused(), Some(id));
                assert!(host.pointer_focus_requested());
                assert!(field.has_focus());
                assert!(field.surface.input.marked.is_none());
            }
        }
    }

    #[test]
    fn prepared_field_capacity_refusal_precedes_selection_and_reader_effects() {
        let mut field = TextFieldContent::new("Draft", "value");
        field.surface.outer = Rect::new(0., 0., 100., 40.);
        field.surface.inner = field.surface.outer;
        field.surface.input.preedit("中", 1..1, None);
        field.surface.input.test_edit_epoch(u64::MAX);
        let before = field.surface.input.action_snapshot();
        assert!(
            field
                .decide_action(&pointer(Point::new(10., 10.)))
                .is_none()
        );
        assert_eq!(
            field.event(&ContentEvent::PointerDown(Point::new(10., 10.))),
            ContentResponse::default()
        );
        assert_eq!(field.surface.input.action_snapshot(), before);
        field.surface.input.test_edit_epoch(0);
    }
}

#[cfg(test)]
mod placeholder_tests {
    use super::*;
    use crate::{TextSurfaceHost, TextSurfaceTextRenderer};
    use std::cell::Cell;
    use std::rc::Rc;
    use tekes_ui_scene::FontKey;
    use tekes_ui_text::{FontInfo, Line, Paragraph, ShapedGlyph, ShapedRun};

    struct PlaceholderTestRenderer {
        shapes: Rc<Cell<usize>>,
        pushes: Vec<(Point, Rect)>,
    }

    impl ButtonTextRenderer for PlaceholderTestRenderer {
        fn text_surface_host(&mut self) -> Option<&mut dyn TextSurfaceHost> {
            Some(self)
        }
        fn measure(&mut self, s: &str, r: proposal::TextRole, w: f32) -> Size {
            Size::new(
                (s.chars().count() as f32 * 8.).min(w),
                r.style().line_height,
            )
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

    impl TextSurfaceHost for PlaceholderTestRenderer {
        fn shape_surface(
            &mut self,
            text: &str,
            role: proposal::TextRole,
            width: Option<f32>,
        ) -> Rc<Paragraph> {
            self.shapes.set(self.shapes.get() + 1);
            let h = f64::from(role.style().line_height);
            let w = (text.chars().count() as f64 * 8.).min(width.map_or(f64::MAX, f64::from));
            let u16 = text.encode_utf16().count();
            Rc::new(Paragraph {
                lines: vec![Line {
                    index: 0,
                    utf16: 0..u16,
                    bytes: 0..text.len(),
                    ascent: h * 0.8,
                    descent: h * 0.2,
                    leading: 0.,
                    height: h,
                    top: 0.,
                    baseline_y: h * 0.8,
                    width: w,
                    width_trimmed: w,
                    runs: if text.is_empty() {
                        vec![]
                    } else {
                        vec![ShapedRun {
                            font: FontKey(1),
                            utf16: 0..u16,
                            glyphs: vec![ShapedGlyph {
                                id: 1,
                                x: 0.,
                                y: 0.,
                                utf16_index: 0,
                                cluster_utf16: 0..1.min(u16),
                            }],
                        }]
                    },
                    carets: vec![(0, 0.), (u16, w)],
                    graphemes: vec![],
                    has_newline: false,
                }],
                height: h,
                width: w,
                default_line_height: h,
                fonts: vec![FontInfo {
                    key: FontKey(1),
                    post_script_name: "Test".into(),
                    size: f64::from(role.style().size),
                    is_color: false,
                }],
                utf16_len: u16,
            })
        }
    }

    impl TextSurfaceTextRenderer for PlaceholderTestRenderer {
        fn push_visible(
            &mut self,
            _: &mut Scene,
            _: &Paragraph,
            origin: Point,
            _: tekes_ui_tokens::Rgba,
            clip: Rect,
        ) {
            self.pushes.push((origin, clip));
        }
    }

    #[test]
    fn inline_clamps_arrange_width_and_placeholder_caches_aligned_multiline_layout() {
        let shapes = Rc::new(Cell::new(0));
        let mut renderer = PlaceholderTestRenderer {
            shapes: shapes.clone(),
            pushes: Vec::new(),
        };
        let mut inline = TextFieldContent::inline("Inline", "hi", 4.0);
        let wide = Rect::new(10., 20., 300., 40.);
        inline.arrange(&mut renderer, wide, wide);
        let expected_w = proposal::TextRole::UiBody.style().size * 4.0;
        assert!((inline.surface.outer.size.width - expected_w).abs() < 0.01);

        let mut field = TextFieldContent::new("Search", "");
        field.surface.input.multiline = true;
        field.surface.wrap = true;
        field.set_placeholder(Some("Hint".into()));
        let frame = Rect::new(0., 0., 200., 120.);
        field.viewport = TextViewport::Fixed(120.);
        field.arrange(&mut renderer, frame, frame);
        shapes.set(0);
        field.paint(
            &mut Scene::new(),
            &mut renderer,
            frame,
            frame,
            Palette::from(tekes_ui_tokens::Theme::LIGHT).into(),
        );
        assert_eq!(shapes.get(), 1);
        let first_origin = renderer.pushes.last().unwrap().0;
        // Multiline placeholder stays top-aligned inside `inner`.
        assert!((first_origin.y - field.surface.inner.min_y().round()).abs() <= 1.0);

        // Repaint without placeholder/role/scale change reuses cached paragraph.
        field.paint(
            &mut Scene::new(),
            &mut renderer,
            frame,
            frame,
            Palette::from(tekes_ui_tokens::Theme::LIGHT).into(),
        );
        assert_eq!(shapes.get(), 1);

        // Single-line trailing alignment honors surface alignment.
        let mut trailing = TextFieldContent::new("Align", "");
        trailing.surface.alignment = crate::TextSurfaceAlignment::Trailing;
        trailing.set_placeholder(Some("Hint".into()));
        trailing.arrange(&mut renderer, Rect::new(0., 0., 200., 40.), frame);
        renderer.pushes.clear();
        trailing.paint(
            &mut Scene::new(),
            &mut renderer,
            Rect::new(0., 0., 200., 40.),
            frame,
            Palette::from(tekes_ui_tokens::Theme::LIGHT).into(),
        );
        let origin = renderer.pushes.last().unwrap().0;
        assert!(origin.x > trailing.surface.inner.min_x() + 100.);
    }

    #[test]
    fn measure_and_arrange_use_pure_field_geometry_with_zero_draw_calls() {
        struct CountingText {
            measures: usize,
            draws: usize,
        }
        impl ButtonTextRenderer for CountingText {
            fn measure(&mut self, text: &str, role: proposal::TextRole, max_width: f32) -> Size {
                self.measures += 1;
                Size::new(
                    (text.chars().count() as f32 * 8.).min(max_width.max(0.)),
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
                _: tekes_ui_tokens::Rgba,
            ) {
                self.draws += 1;
            }
        }

        for viewport in [TextViewport::Form, TextViewport::Fixed(88.)] {
            let mut field = TextFieldContent::new("Workspace", "Tekes");
            field.field.required = true;
            field.field.error = Some("Required".into());
            field.viewport = viewport;
            let mut text = CountingText {
                measures: 0,
                draws: 0,
            };
            let measured = field.measure(&mut text, Size::new(280., 200.));
            assert!(measured.size.width > 0. && measured.size.height > 0.);
            let frame = Rect::new(12., 16., 280., measured.size.height);
            field.arrange(&mut text, frame, frame);
            assert!(text.measures > 0);
            assert_eq!(
                text.draws, 0,
                "field_layout during measure/arrange must not paint into any Scene"
            );
            assert!(field.view.layout().slot.size.height > 0.);
        }
    }
}
