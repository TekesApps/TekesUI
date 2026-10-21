//! Independent public API consumer with real CoreText first-line metrics.
use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, ControlKey,
    extended::{Banner, NoticeTone, Toast},
    feedback_view::{BannerView, ToastView},
};
use tekes_ui_scene::{Primitive, Rect, Scene, Size};
use tekes_ui_text::{
    coretext::{CoreTextFonts, CoreTextShaper},
    model::ShapeOptions,
    style::TextStyle,
    traits::Shaper,
};
use tekes_ui_tokens::{Palette, Rgba, Theme, proposal};

struct Text {
    shaper: CoreTextShaper,
    scale: f32,
    draws: Vec<(String, Rect, ButtonTextAlign)>,
}
impl Text {
    fn paragraph(
        &mut self,
        s: &str,
        role: proposal::TextRole,
        width: f32,
    ) -> tekes_ui_text::model::Paragraph {
        let style = role.style();
        self.shaper.shape(
            s,
            &TextStyle::system(style.size * self.scale).with_weight(style.weight),
            &ShapeOptions {
                width: Some(f64::from(width.max(0.))),
                line_spacing: 0.,
            },
        )
    }
}
impl ButtonTextRenderer for Text {
    fn measure(&mut self, s: &str, role: proposal::TextRole, width: f32) -> Size {
        let p = self.paragraph(s, role, width);
        Size::new(p.width as f32, p.height as f32)
    }
    fn first_line_height(&mut self, s: &str, role: proposal::TextRole, width: f32) -> f32 {
        let p = self.paragraph(s, role, width);
        p.lines
            .first()
            .map_or(p.default_line_height as f32, |l| l.height as f32)
    }
    fn draw(
        &mut self,
        _: &mut Scene,
        s: &str,
        _: proposal::TextRole,
        rect: Rect,
        align: ButtonTextAlign,
        _: Rgba,
    ) {
        self.draws.push((s.into(), rect, align));
    }
}

#[test]
fn public_notices_align_first_line_with_coretext_and_keep_dismiss_actions() {
    let role = proposal::TextRole::UiBody;
    for theme in Theme::ALL {
        for type_scale in [1., 1.5] {
            for width in [220., 640.] {
                for message in [
                    "Changes saved",
                    "文档已保存，继续编辑并保留所有更改。",
                    "A wrapped notice with several lines of mixed English 和中文 text.",
                ] {
                    for banner in [false, true] {
                        let mut text = Text {
                            shaper: CoreTextShaper::new(CoreTextFonts::new()),
                            scale: type_scale,
                            draws: vec![],
                        };
                        let mut scene = Scene::new();
                        let palette = Palette::from(theme);
                        let mut toast = Toast::new(message, NoticeTone::Success, 5000);
                        let mut banner_model = Banner {
                            message: message.into(),
                            tone: NoticeTone::Info,
                            dismissible: true,
                            visible: true,
                        };
                        let mut toast_view = ToastView::default();
                        let mut banner_view = BannerView::default();
                        let size = if banner {
                            BannerView::measure(&mut text, message, width, true, role)
                        } else {
                            ToastView::measure(&mut text, message, width, role)
                        };
                        let frame = Rect::new(20., 30., size.width, size.height);
                        if banner {
                            banner_view.render(
                                &mut scene,
                                &mut text,
                                &banner_model,
                                frame,
                                None,
                                palette,
                                role,
                            );
                        } else {
                            toast_view
                                .render(&mut scene, &mut text, &toast, frame, None, palette, role);
                        }
                        let message_frame =
                            text.draws.iter().find(|(s, _, _)| s == message).unwrap().1;
                        let action_align = text
                            .draws
                            .iter()
                            .find(|(s, _, _)| s == "Dismiss")
                            .unwrap()
                            .2;
                        assert_eq!(action_align, ButtonTextAlign::default());
                        assert_eq!(action_align, ButtonTextAlign::Center);
                        let paragraph = text.paragraph(message, role, message_frame.size.width);
                        let first_height = paragraph.lines.first().unwrap().height as f32;
                        let diameter = if banner { 12. } else { 10. };
                        let marker = scene
                            .primitives()
                            .iter()
                            .find_map(|p| match p {
                                Primitive::Quad(q)
                                    if q.rect.size == Size::new(diameter, diameter) =>
                                {
                                    Some(q.rect)
                                }
                                _ => None,
                            })
                            .unwrap();
                        assert!(
                            (marker.center().y - (message_frame.min_y() + first_height / 2.)).abs()
                                < 0.01,
                            "theme={theme:?}, scale={type_scale}, width={width}, banner={banner}, message={message}"
                        );
                        if banner {
                            banner_view.focus_dismiss(true, true);
                            assert!(banner_view.key(&mut banner_model, ControlKey::Enter, true));
                            assert!(!banner_view.key(&mut banner_model, ControlKey::Enter, false));
                            assert!(!banner_model.visible);
                        } else {
                            toast_view.focus_dismiss(true, true);
                            assert!(toast_view.key(&mut toast, ControlKey::Enter, true));
                            assert!(!toast_view.key(&mut toast, ControlKey::Enter, false));
                            assert!(!toast.visible);
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn public_empty_state_measure_contains_wrapped_copy_and_centered_action() {
    use tekes_ui_controls::{extended::EmptyState, feedback_view::EmptyStateView};
    for theme in Theme::ALL {
        for type_scale in [1., 1.5] {
            for width in [120., 220., 420., 640.] {
                for action in [None, Some("Create document".to_owned())] {
                    let mut text = Text {
                        shaper: CoreTextShaper::new(CoreTextFonts::new()),
                        scale: type_scale,
                        draws: vec![],
                    };
                    let model = EmptyState {
                        title: "No documents yet — 尚无文档".repeat(3),
                        detail: "Create a document to begin your workspace. 保留完整说明。"
                            .repeat(3),
                        action,
                    };
                    let size = EmptyStateView::measure(
                        &mut text,
                        &model,
                        width,
                        proposal::TextRole::UiBody,
                    );
                    let frame = Rect::new(20., 30., size.width, size.height);
                    let mut view = EmptyStateView::default();
                    view.render(
                        &mut Scene::new(),
                        &mut text,
                        &model,
                        frame,
                        Some(frame),
                        Palette::from(theme),
                        proposal::TextRole::UiBody,
                    );
                    for (label, rect, align) in &text.draws {
                        assert!(
                            rect.min_y() >= frame.min_y() && rect.max_y() <= frame.max_y() + 0.01,
                            "{label}: drawn text must fit its natural measured frame"
                        );
                        if Some(label) == model.action.as_ref() {
                            assert_eq!(*align, ButtonTextAlign::Center);
                        }
                    }
                    if model.action.is_some() {
                        assert_eq!(view.focus_target(), view.logical_focus_target());
                        view.focus_action(true, true);
                        assert!(view.key(ControlKey::Enter, true));
                        assert!(!view.key(ControlKey::Enter, false));
                    } else {
                        assert!(view.focus_target().is_none());
                    }
                }
            }
        }
    }
}
