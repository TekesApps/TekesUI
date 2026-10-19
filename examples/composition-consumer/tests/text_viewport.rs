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

use tekes_ui_composition_consumer::text_viewport::*;
use tekes_ui_controls::{ScrollPolicy, TextInput};
fn render(f: &mut ViewportFixture, width: f32, height: f32) {
    let rect = Rect::new(70., 90., width, height);
    f.host
        .render(
            &mut Scene::new(),
            &mut Text,
            rect,
            rect,
            Theme::LIGHT.into(),
        )
        .unwrap();
}
#[test]
fn stock_stack_and_fields_fill_remaining_space_and_grow_then_scroll() {
    let mut f = ViewportFixture::new();
    render(&mut f, 400., 600.);
    let small = f.field(COMPOSER).surface.outer;
    let transcript = f.field(TRANSCRIPT).surface.outer;
    assert_eq!(small.size.height, 56.);
    assert!(transcript.size.height > 350.);
    f.event(&ContentEvent::Commit {
        text: "中文
English
😀 e\u{301}
line"
            .into(),
        replacement: None,
    });
    render(&mut f, 400., 600.);
    assert!(f.field(COMPOSER).surface.outer.size.height > small.size.height);
    let long = "行 English 中文 😀\n".repeat(100);
    f.event(&ContentEvent::Commit {
        text: long,
        replacement: None,
    });
    render(&mut f, 400., 600.);
    let input = &f.field(COMPOSER).surface;
    assert_eq!(input.outer.size.height, 160.);
    assert!(input.scroll.limit().y > 0.);
    let nodes = f.host.semantics();
    let send = nodes.iter().find(|n| n.id == SEND).unwrap();
    assert!(send.frame.min_y() >= input.outer.max_y());
    assert!(send.frame.max_y() <= 690.);
    assert_eq!(f.host.focused(), Some(COMPOSER));
}
#[test]
fn measuring_and_resizing_preserves_ime_selection_history_and_reading_state() {
    let mut f = ViewportFixture::new();
    render(&mut f, 500., 650.);
    f.event(&ContentEvent::Commit {
        text: "saved 中文😀".into(),
        replacement: None,
    });
    f.event(&ContentEvent::Preedit {
        text: "拼".into(),
        selection: 1..1,
        replacement: None,
        clauses: vec![],
    });
    let before = format!("{:?}", f.field(COMPOSER).surface.input);
    render(&mut f, 220., 450.);
    assert_eq!(format!("{:?}", f.field(COMPOSER).surface.input), before);
    assert_eq!(f.host.focused(), Some(COMPOSER));
    assert_eq!(
        f.field(TRANSCRIPT)
            .reading
            .as_ref()
            .unwrap()
            .document_version()
            .unwrap()
            .key(),
        "reading"
    );
    f.event(&ContentEvent::TextCommand("cancelOperation:".into()));
    f.event(&ContentEvent::TextCommand("undo:".into()));
    assert_eq!(f.field(COMPOSER).surface.input.text, "");
}
#[test]
fn wrap_fixed_height_and_existing_scroll_policy_share_arranged_geometry() {
    let mut f = ViewportFixture::new();
    let field = f.field_mut(COMPOSER);
    field.viewport = TextViewport::Fixed(90.);
    field.field.help = "帮助内容".into();
    field.surface.input = TextInput::new(&"English中文😀".repeat(50), true);
    field.surface.wrap = false;
    field
        .surface
        .scroll
        .set_scroll_policy(ScrollPolicy::Auto, ScrollPolicy::Auto);
    render(&mut f, 240., 600.);
    let input = &f.field(COMPOSER).surface;
    assert_eq!(input.outer.size.height, 90.);
    assert!(input.scroll.limit().x > 0.);
    assert_eq!(input.width, None);
    let semantic = f
        .host
        .semantics()
        .into_iter()
        .find(|n| n.id == COMPOSER)
        .unwrap();
    assert_eq!(semantic.frame, input.outer);
    assert!(input.outer.min_x() >= 70. && input.outer.min_y() > 90.);
    let layout = f.field(COMPOSER).view.layout();
    assert!(layout.message.min_y() >= input.outer.max_y());
    f.field_mut(COMPOSER).surface.wrap = true;
    render(&mut f, 240., 600.);
    assert!(f.field(COMPOSER).surface.width.is_some());
    assert!(f.field(COMPOSER).surface.scroll.limit().y > 0.);
}

#[test]
fn intrinsic_measurement_does_not_mutate_arranged_surface_or_default_form_size() {
    let mut f = ViewportFixture::new();
    render(&mut f, 400., 600.);
    let before = format!("{:?}", f.field(COMPOSER).surface);
    let measured = f
        .field_mut(COMPOSER)
        .measure(&mut Text, Size::new(130., 900.));
    assert!(measured.size.height > 56.);
    assert_eq!(format!("{:?}", f.field(COMPOSER).surface), before);
    let mut form = TextFieldContent::new("Name", "value");
    assert_eq!(form.viewport, TextViewport::Form);
    let normal = form.measure(&mut Text, Size::new(300., 1000.)).size;
    let tiny = form.measure(&mut Text, Size::new(300., 1.)).size;
    assert_eq!(normal, tiny);
    let mut legacy = tekes_ui_controls::feedback_view::FieldView::default();
    let old = legacy.render(
        &mut Scene::new(),
        &mut Text,
        &form.field,
        Rect::new(0., 0., 300., 1000.),
        None,
        Theme::LIGHT.into(),
        proposal::TextRole::UiBody,
    );
    assert_eq!(normal.height, old.slot.max_y().max(old.message.max_y()));
}
#[test]
fn readonly_fill_uses_original_selection_and_rejects_user_edits() {
    let mut f = ViewportFixture::new();
    render(&mut f, 400., 600.);
    f.host.focus(Some(TRANSCRIPT), true);
    f.event(&ContentEvent::TextCommand("selectAll:".into()));
    let original = f.field(TRANSCRIPT).surface.input.text.clone();
    assert_eq!(f.field(TRANSCRIPT).surface.input.selected_text(), original);
    assert!(
        !f.event(&ContentEvent::Commit {
            text: "bad".into(),
            replacement: None
        })
        .changed
    );
    f.event(&ContentEvent::TextCommand("deleteBackward:".into()));
    assert_eq!(f.field(TRANSCRIPT).surface.input.text, original);
    assert_eq!(f.field(TRANSCRIPT).surface.input.selected_text(), original);
}
#[test]
fn nonzero_origin_clip_and_backing_scales_share_caret_hit_and_semantic_frame() {
    struct ScaledText {
        backing: f32,
        types: f32,
    }
    impl ButtonTextRenderer for ScaledText {
        fn backing_scale(&self) -> f32 {
            self.backing
        }
        fn text_surface_host(&mut self) -> Option<&mut dyn tekes_ui_controls::TextSurfaceHost> {
            Some(self)
        }
        fn measure(&mut self, s: &str, r: proposal::TextRole, w: f32) -> Size {
            Size::new(
                (s.len() as f32 * 8. * self.types).min(w),
                r.style().line_height * self.types,
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
    impl tekes_ui_controls::TextSurfaceTextRenderer for ScaledText {
        fn push_visible(
            &mut self,
            _: &mut Scene,
            _: &tekes_ui_text::Paragraph,
            _: tekes_ui_scene::Point,
            _: Rgba,
            _: Rect,
        ) {
        }
    }
    impl tekes_ui_controls::TextSurfaceHost for ScaledText {
        fn type_scale(&self) -> f32 {
            self.types
        }
        fn shape_surface(
            &mut self,
            text: &str,
            role: proposal::TextRole,
            width: Option<f32>,
        ) -> std::rc::Rc<tekes_ui_text::Paragraph> {
            use tekes_ui_text::traits::Shaper;
            let mut shaper = tekes_ui_text::coretext::CoreTextShaper::new(
                tekes_ui_text::coretext::CoreTextFonts::new(),
            );
            std::rc::Rc::new(shaper.shape(
                text,
                &tekes_ui_text::style::TextStyle::system(role.style().size * self.types),
                &tekes_ui_text::model::ShapeOptions {
                    width: width.map(f64::from),
                    line_spacing: 0.,
                },
            ))
        }
    }
    for backing in [1., 2.] {
        for types in [1., 1.5, 2.] {
            let mut f = ViewportFixture::new();
            f.field_mut(COMPOSER).viewport = TextViewport::Fixed(120.);
            let frame = Rect::new(70., 90., 320.25, 650.);
            let clip = Rect::new(80., 100., 300., 630.);
            f.host
                .render(
                    &mut Scene::new(),
                    &mut ScaledText { backing, types },
                    frame,
                    clip,
                    Theme::LIGHT.into(),
                )
                .unwrap();
            let surface = &f.field(COMPOSER).surface;
            let node = f
                .host
                .semantics()
                .into_iter()
                .find(|n| n.id == COMPOSER)
                .unwrap();
            assert_eq!(node.frame, surface.outer);
            assert_eq!(node.visible, surface.visible_frame());
            let caret = surface.caret();
            assert!(surface.inner.contains(caret.origin));
            assert_eq!(surface.hit(caret.origin), Some(0));
            assert!(
                surface
                    .hit(tekes_ui_scene::Point::new(75., caret.min_y()))
                    .is_none()
            );
        }
    }
}

#[cfg(feature = "token-audit")]
#[test]
fn actual_viewport_modes_reuse_current_input_tokens_across_themes_and_brands() {
    let mut cases = Vec::new();
    for theme in Theme::ALL {
        for brand in 0..3 {
            for wrap in [false, true] {
                for viewport in [
                    TextViewport::Form,
                    TextViewport::Fixed(100.),
                    TextViewport::Fill,
                    TextViewport::Grow {
                        min: 56.,
                        max: 160.,
                    },
                ] {
                    let mut f = ViewportFixture::new();
                    let input = f.field_mut(COMPOSER);
                    input.viewport = viewport;
                    input.surface.wrap = wrap;
                    input
                        .surface
                        .input
                        .insert(&"中文 English 😀 e\u{301}\n".repeat(30), None);
                    input
                        .surface
                        .scroll
                        .set_scroll_policy(ScrollPolicy::Auto, ScrollPolicy::Auto);
                    let frame = Rect::new(25., 35., 350., 700.);
                    let (scene, lookups) = tekes_ui_tokens::audit::capture(|| {
                        let mut scene = Scene::new();
                        f.host
                            .render(
                                &mut scene,
                                &mut Text,
                                frame,
                                frame,
                                tekes_ui_tokens::Palette::new(theme, brand).unwrap(),
                            )
                            .unwrap();
                        scene
                    });
                    let input = &f.field(COMPOSER).surface;
                    assert!(scene.primitives().iter().any(
                        |p| matches!(p,tekes_ui_scene::Primitive::Quad(q) if q.rect==input.outer)
                    ));
                    assert!(
                        lookups
                            .iter()
                            .any(|(family, id)| family == "color" && id == "transcript-input")
                    );
                    assert_eq!(input.width.is_some(), wrap);
                    if matches!(viewport, TextViewport::Grow { .. }) {
                        assert_eq!(input.outer.size.height, 160.);
                        assert!(input.scroll.limit().y > 0.);
                    }
                    cases.push(serde_json::json!({"component":"tui.text-field","scope":"actual stock multiline Field/Stack consumer; transcript also rendered","viewport":format!("{viewport:?}"),"wrap":wrap,"theme":format!("{theme:?}"),"brand":brand,"state_asserted":true,"lookups":lookups}));
                }
            }
        }
    }
    assert_eq!(cases.len(), 96);
    if let Some(path) = std::env::var_os("TEKESUI_VIEWPORT_TOKEN_AUDIT_OUT") {
        std::fs::write(path,serde_json::to_vec_pretty(&serde_json::json!({"format":"tekesui-runtime-token-observation/1","complete":false,"scope":"bounded real public viewport modes and original TextSurface/ScrollArea, wrap on/off; four themes,three existing brands; CoreText,no-op glyph painter","excluded":["compiled font/style constants","unexecuted paths","physicalIME/VoiceOver","complete dependency extraction"],"cases":cases})).unwrap()).unwrap();
    }
}

#[cfg(target_os = "macos")]
#[test]
fn public_ax_layout_uses_real_wrapped_lines_for_unfocused_readonly_and_composer() {
    use tekes_ui_platform_macos::{AccessibilityTextLayout, Point, Size, TextInputState};
    let mut f = ViewportFixture::new();
    render(&mut f, 220., 300.);
    // Both semantic owners, including the unfocused read-only transcript, have
    // their own shaped geometry. A focused-only IME snapshot cannot substitute.
    for id in [TRANSCRIPT, COMPOSER] {
        let surface = &f.field(id).surface;
        let p = surface.paragraph.as_ref().unwrap();
        let origin = surface.origin();
        let layout = AccessibilityTextLayout {
            state: TextInputState {
                text: surface.input.text.clone(),
                caret_index: surface.input.caret,
                selection: surface.input.selection(),
                read_only: surface.input.read_only,
                carets: p
                    .lines
                    .iter()
                    .flat_map(|line| {
                        line.carets.iter().map(move |(i, x)| {
                            (
                                *i,
                                Point {
                                    x: f64::from(origin.x) + x,
                                    y: f64::from(origin.y) + line.top,
                                },
                                Size {
                                    width: 1.,
                                    height: line.height,
                                },
                            )
                        })
                    })
                    .collect(),
                ..Default::default()
            },
            lines: p.lines.iter().map(|l| l.utf16.clone()).collect(),
        };
        assert!(layout.is_valid());
        let viewport = tekes_ui_platform_macos::Rect {
            origin: Point {
                x: f64::from(surface.inner.min_x()),
                y: f64::from(surface.inner.min_y()),
            },
            size: Size {
                width: f64::from(surface.inner.size.width),
                height: f64::from(surface.inner.size.height),
            },
        };
        assert!(layout.visible_range(viewport).is_some());
        if let Some(byte) = surface.input.text.find('😀') {
            let start = surface.input.text[..byte].encode_utf16().count();
            assert_eq!(layout.range_for_index(start + 1), Some(start..start + 2));
        }
        for line in &p.lines {
            for pair in line.carets.windows(2) {
                if pair[0].1 != pair[1].1 {
                    let point = Point {
                        x: f64::from(origin.x) + (pair[0].1 + pair[1].1) * 0.5,
                        y: f64::from(origin.y) + line.top + line.height * 0.5,
                    };
                    assert_eq!(layout.range_for_position(point), Some(pair[0].0..pair[1].0));
                    break;
                }
            }
        }
        for (index, line) in p.lines.iter().enumerate() {
            assert_eq!(layout.range_for_line(index), Some(line.utf16.clone()));
            assert_eq!(layout.line_for_index(line.utf16.start), Some(index));
            let frame = layout
                .frame_for_range(line.utf16.start..line.utf16.start)
                .unwrap();
            assert!((frame.origin.y - f64::from(origin.y) - line.top).abs() < 0.01);
        }
    }
}
