#![cfg(target_os = "macos")]
#[path = "support/coretext.rs"]
mod coretext;
use std::rc::Rc;
use tekes_ui_composition_consumer::semantic_label as f;
use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, ControlKey, TextSurfaceHost, TextSurfaceTextRenderer,
    composition::*,
};
use tekes_ui_scene::{Point, Rect, Scene, Size};
use tekes_ui_text::{
    coretext::{CoreTextFonts, CoreTextShaper},
    *,
};
use tekes_ui_tokens::{Palette, Rgba, Theme, proposal};
struct Recorder {
    shaper: CoreTextShaper,
    layouts: Vec<Rc<LabelLayout>>,
    glyphs: Vec<(ShapedGlyph, Point, Rgba)>,
}
impl Recorder {
    fn new() -> Self {
        Self {
            shaper: CoreTextShaper::new(CoreTextFonts::new()),
            layouts: vec![],
            glyphs: vec![],
        }
    }
}
impl ButtonTextRenderer for Recorder {
    fn text_surface_host(&mut self) -> Option<&mut dyn TextSurfaceHost> {
        Some(self)
    }
    fn measure(&mut self, _: &str, _: proposal::TextRole, _: f32) -> Size {
        panic!("original label service required")
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
        panic!("no consumer string repaint")
    }
}
impl TextSurfaceHost for Recorder {
    fn shape_surface(&mut self, _: &str, _: proposal::TextRole, _: Option<f32>) -> Rc<Paragraph> {
        panic!("not an editor")
    }
    fn shape_semantic_label(
        &mut self,
        label: &SemanticLabel,
        role: proposal::TextRole,
        width: f32,
        options: LabelOptions,
    ) -> Option<Rc<LabelLayout>> {
        let layout = Rc::new(self.shaper.shape_semantic_label(
            label,
            &TextStyle::system(role.style().size),
            &ShapeOptions::with_width(width as f64),
            options,
        ));
        self.layouts.push(layout.clone());
        Some(layout)
    }
}
impl TextSurfaceTextRenderer for Recorder {
    fn push_visible(&mut self, _: &mut Scene, p: &Paragraph, origin: Point, color: Rgba, _: Rect) {
        self.glyphs.extend(
            p.lines
                .iter()
                .flat_map(|l| &l.runs)
                .flat_map(|r| &r.glyphs)
                .map(|g| (g.clone(), origin, color)),
        );
    }
}
fn node(kind: usize, overflow: LabelOverflow) -> Content {
    let options = LabelOptions::single_line(overflow);
    match kind {
        0 => Content::new(f::LABEL, TextContent::new(f::SOURCE))
            .with_semantic_label(f::model(1), options)
            .unwrap(),
        1 => Content::new(f::ACTION, Button::new(f::SOURCE))
            .with_semantic_label(f::model(1), options)
            .unwrap(),
        _ => Content::new(
            f::CUSTOM,
            Button::new(
                Content::new(f::DECORATION, TextContent::new(f::SOURCE))
                    .with_semantic_label(f::model(1), options)
                    .unwrap(),
            )
            .with_accessible_name(f::SOURCE),
        ),
    }
}
#[test]
fn original_owners_paint_exact_retained_glyph_order_coordinates_and_theme_roles() {
    for kind in 0..3 {
        for theme in Theme::ALL {
            for width in [100., 600.] {
                for overflow in [LabelOverflow::Clip, LabelOverflow::TailEllipsis] {
                    let primary = kind != 0 && width == 600.;
                    let mut content = node(kind, overflow);
                    if primary {
                        content.component_mut::<Button>().unwrap().variant =
                            tekes_ui_controls::paint::ButtonVariant::Primary;
                    }
                    let mut host = ContentHost::new(content).unwrap();
                    let mut recorder = Recorder::new();
                    let frame = Rect::new(0., 0., width, 200.);
                    host.render(&mut Scene::new(), &mut recorder, frame, frame, theme.into())
                        .unwrap();
                    let layout = recorder.layouts.last().unwrap();
                    let actual: Vec<_> = recorder.glyphs.iter().map(|(g, _, _)| g).collect();
                    let expected: Vec<_> = layout
                        .paragraph()
                        .lines
                        .iter()
                        .flat_map(|l| &l.runs)
                        .flat_map(|r| &r.glyphs)
                        .collect();
                    assert_eq!(
                        actual, expected,
                        "paint must retain exact glyphs/order/coordinates, no re-shaping fragments"
                    );
                    assert!(!actual.is_empty());
                    let palette = Palette::from(theme);
                    for (glyph, origin, color) in &recorder.glyphs {
                        assert_eq!(*origin, recorder.glyphs[0].1);
                        let role = if glyph.cluster_utf16.is_empty() {
                            LabelForeground::Inherited
                        } else {
                            f::model(1).foreground_at(glyph.utf16_index)
                        };
                        let token = match role {
                            LabelForeground::Inherited => {
                                if primary {
                                    "action-primary-foreground"
                                } else {
                                    "primary-text"
                                }
                            }
                            LabelForeground::Secondary => "secondary-text",
                            LabelForeground::Positive => "status-succeeded",
                            LabelForeground::Negative => "status-failed",
                        };
                        assert_eq!(*color, palette.color(token).unwrap());
                    }
                    assert_eq!(host.semantics().len(), 1);
                    assert_eq!(host.semantics()[0].semantics.name, f::SOURCE);
                    assert_eq!(layout.source(), f::SOURCE);
                    assert!(host.focused().is_none());
                }
            }
        }
    }
}
#[test]
fn stale_foreign_or_mismatched_updates_leave_original_source_and_model_unchanged() {
    for kind in [0, 1] {
        let mut content = node(kind, LabelOverflow::TailEllipsis);
        let before = content.semantic_label().unwrap().clone();
        let version = before.version();
        let foreign = SemanticLabel::new(
            LabelSourceVersion {
                source: 999,
                revision: 2,
            },
            "foreign",
            vec![],
        )
        .unwrap();
        for next in [f::model(1), foreign] {
            assert_eq!(
                content.update_semantic_label(version, next),
                Err(SemanticLabelError::StaleUpdate)
            );
            assert_eq!(content.semantic_label().unwrap(), &before);
        }
        assert_eq!(
            content.update_semantic_label(
                LabelSourceVersion {
                    revision: 99,
                    ..version
                },
                f::model(2)
            ),
            Err(SemanticLabelError::StaleUpdate)
        );
        let next_source = f::SOURCE.replace("+12", "+13");
        let next = SemanticLabel::new(
            LabelSourceVersion {
                revision: 2,
                ..version
            },
            &next_source,
            before.runs().to_vec(),
        )
        .unwrap();
        content.update_semantic_label(version, next).unwrap();
        assert_eq!(content.semantic_label().unwrap().version().revision, 2);
        let source = content
            .component::<TextContent>()
            .map(|c| c.text.as_str())
            .or_else(|| content.component::<Button>().map(|c| c.label.as_str()))
            .unwrap();
        assert_eq!(source, next_source);
    }
    assert!(
        Content::new(ComponentId(9), TextContent::new("wrong source"))
            .with_semantic_label(f::model(1), LabelOptions::default())
            .is_err()
    );
}
#[test]
fn original_button_pointer_key_and_ax_have_one_action_owner() {
    for kind in [1, 2] {
        let mut host = ContentHost::new(node(kind, LabelOverflow::TailEllipsis)).unwrap();
        let mut text = coretext::Text::new(1.);
        text.services = true;
        let frame = Rect::new(0., 0., 600., 200.);
        host.render(
            &mut Scene::new(),
            &mut text,
            frame,
            frame,
            Theme::LIGHT.into(),
        )
        .unwrap();
        let id = if kind == 1 { f::ACTION } else { f::CUSTOM };
        let mount = host.semantic_mount(id);
        let point = host.get(id).unwrap().frame().center();
        assert!(!host.event(&ContentEvent::PointerDown(point)).activated);
        assert!(host.event(&ContentEvent::PointerUp(point)).activated);
        host.focus(Some(id), true);
        assert!(
            host.event(&ContentEvent::Key {
                key: ControlKey::Enter,
                down: true
            })
            .activated
        );
        assert!(
            host.accessibility_action(id, None, AccessibilityAction::Press)
                .activated
        );
        assert_eq!(
            host.get(id)
                .unwrap()
                .component::<Button>()
                .unwrap()
                .activations,
            3
        );
        assert_eq!(host.semantic_mount(id), mount);
        assert_eq!(host.semantics().len(), 1);
        assert_eq!(host.semantics()[0].semantics.name, f::SOURCE);
    }
}
#[test]
fn background_label_revision_theme_font_and_width_keep_composer_marked_state() {
    let mut host = f::build().unwrap();
    let mut text = coretext::Text::new(1.);
    text.services = true;
    let frame = Rect::new(0., 0., 600., 400.);
    host.render(
        &mut Scene::new(),
        &mut text,
        frame,
        frame,
        Theme::LIGHT.into(),
    )
    .unwrap();
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
    for id in [f::LABEL, f::ACTION, f::DECORATION] {
        host.get_mut(id)
            .unwrap()
            .update_semantic_label(f::model(1).version(), f::model(2))
            .unwrap();
    }
    for theme in Theme::ALL {
        for (width, scale) in [(600., 1.), (180., 1.5), (400., 2.)] {
            text.scale = scale;
            let frame = Rect::new(0., 0., width, 600.);
            host.render(&mut Scene::new(), &mut text, frame, frame, theme.into())
                .unwrap();
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
}

#[test]
fn legacy_backend_default_keeps_plain_service_and_explicitly_declines_attributes() {
    struct PlainOnly(CoreTextShaper);
    impl TextSurfaceTextRenderer for PlainOnly {
        fn push_visible(&mut self, _: &mut Scene, _: &Paragraph, _: Point, _: Rgba, _: Rect) {}
    }
    impl TextSurfaceHost for PlainOnly {
        fn shape_surface(
            &mut self,
            text: &str,
            _: proposal::TextRole,
            _: Option<f32>,
        ) -> Rc<Paragraph> {
            Rc::new(
                self.0
                    .shape(text, &TextStyle::system(17.), &ShapeOptions::default()),
            )
        }
        fn shape_label(
            &mut self,
            text: &str,
            _: proposal::TextRole,
            width: f32,
            options: LabelOptions,
        ) -> Option<Rc<LabelLayout>> {
            Some(Rc::new(self.0.shape_label(
                text,
                &TextStyle::system(17.),
                &ShapeOptions::with_width(width as f64),
                options,
            )))
        }
    }
    let mut old = PlainOnly(CoreTextShaper::new(CoreTextFonts::new()));
    let options = LabelOptions::single_line(LabelOverflow::TailEllipsis);
    assert!(
        old.shape_semantic_label(&f::model(1), proposal::TextRole::UiBody, 100., options)
            .is_none()
    );
    let plain = SemanticLabel::new(f::model(1).version(), f::SOURCE, vec![]).unwrap();
    assert_eq!(
        old.shape_semantic_label(&plain, proposal::TextRole::UiBody, 100., options)
            .unwrap()
            .source(),
        f::SOURCE
    );
}

#[test]
fn long_semantic_label_actually_elides_multiple_roles_on_each_original_owner() {
    for theme in Theme::ALL {
        for width in [100., 180., 260., 720.] {
            for kind in 0..3 {
                let label = f::ellipsis_model(1);
                let options = LabelOptions::single_line(LabelOverflow::TailEllipsis);
                let node = match kind {
                    0 => Content::new(f::LABEL, TextContent::new(label.text()))
                        .with_semantic_label(label.clone(), options)
                        .unwrap(),
                    1 => Content::new(f::ACTION, Button::new(label.text()))
                        .with_semantic_label(label.clone(), options)
                        .unwrap(),
                    _ => Content::new(
                        f::CUSTOM,
                        Button::new(
                            Content::new(f::DECORATION, TextContent::new(label.text()))
                                .with_semantic_label(label.clone(), options)
                                .unwrap(),
                        )
                        .with_accessible_name(label.text()),
                    ),
                };
                let mut host = ContentHost::new(node).unwrap();
                let mut recorder = Recorder::new();
                let frame = Rect::new(0., 0., width, 200.);
                host.render(&mut Scene::new(), &mut recorder, frame, frame, theme.into())
                    .unwrap();
                let layout = recorder.layouts.last().unwrap();
                f::assert_cross_run_ellipsis(&label, layout);
                assert_eq!(host.semantics().len(), 1);
                assert_eq!(host.semantics()[0].semantics.name, label.text());
                let expected: Vec<_> = layout
                    .paragraph()
                    .lines
                    .iter()
                    .flat_map(|l| &l.runs)
                    .flat_map(|r| &r.glyphs)
                    .collect();
                assert_eq!(
                    recorder
                        .glyphs
                        .iter()
                        .map(|(g, _, _)| g)
                        .collect::<Vec<_>>(),
                    expected
                );
                let ellipsis = recorder
                    .glyphs
                    .iter()
                    .find(|(g, _, _)| g.cluster_utf16.is_empty())
                    .unwrap();
                assert_eq!(
                    ellipsis.2,
                    Palette::from(theme).color("primary-text").unwrap(),
                    "synthetic ellipsis inherits original control foreground"
                );
            }
        }
    }
}
