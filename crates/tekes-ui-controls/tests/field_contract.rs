//! Field metadata is shared by ordinary and composed text inputs.
use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, Field,
    feedback_view::{FieldPresentation, FieldView},
};
use tekes_ui_scene::{Rect, Scene, Size};
use tekes_ui_tokens::{Rgba, Theme, proposal};

#[derive(Default)]
struct Text {
    draws: Vec<(String, proposal::TextRole, Rect, Rgba)>,
}
impl ButtonTextRenderer for Text {
    fn measure(&mut self, s: &str, role: proposal::TextRole, width: f32) -> Size {
        let natural = s.chars().count() as f32 * role.style().size * 0.5;
        let width = width.max(1.);
        Size::new(
            natural.min(width),
            (natural / width).ceil().max(1.) * role.style().line_height,
        )
    }
    fn draw(
        &mut self,
        _: &mut Scene,
        s: &str,
        role: proposal::TextRole,
        frame: Rect,
        _: ButtonTextAlign,
        ink: Rgba,
    ) {
        self.draws.push((s.into(), role, frame, ink));
    }
}
fn model() -> Field {
    Field {
        label: "A long required workspace display name".into(),
        help: "This help wraps below the input slot".into(),
        required: true,
        ..Default::default()
    }
}

#[test]
fn required_marker_keeps_its_width_when_the_label_wraps() {
    for width in [24., 140., 240.] {
        let mut text = Text::default();
        let layout = FieldView::default().render(
            &mut Scene::new(),
            &mut text,
            &model(),
            Rect::new(8., 12., width, 2000.),
            None,
            Theme::LIGHT.into(),
            proposal::TextRole::UiBody,
        );
        let mark = layout.required_mark.unwrap();
        assert!(
            mark.size.width > 0.,
            "required marker lost at width {width}"
        );
        assert!(mark.min_x() >= layout.label.max_x());
        assert!(mark.max_x() <= layout.frame.max_x());
        assert!(layout.slot.min_y() >= layout.label.max_y());
        assert!(text.draws.iter().any(|(s, _, _, _)| s == "*"));
    }
}

#[test]
fn required_marker_uses_the_same_typography_as_its_emphasized_label() {
    let mut text = Text::default();
    FieldView::default().render_with_presentation(
        &mut Scene::new(),
        &mut text,
        &model(),
        Rect::new(0., 0., 480., 500.),
        None,
        Theme::LIGHT.into(),
        proposal::TextRole::UiBody,
        FieldPresentation::emphasized_form(),
    );
    let label = text
        .draws
        .iter()
        .find(|(s, _, _, _)| s == &model().label)
        .unwrap();
    let mark = text.draws.iter().find(|(s, _, _, _)| s == "*").unwrap();
    assert_eq!(mark.1, label.1);
    assert_eq!(mark.2.min_y(), label.2.min_y());
}

#[test]
fn required_metadata_keeps_layout_and_message_contract_in_all_palettes() {
    use tekes_ui_tokens::Palette;
    let mut cases = Vec::new();
    for (theme_index, theme) in Theme::ALL.into_iter().enumerate() {
        for brand in 0..3 {
            for emphasized in [false, true] {
                for error in [false, true] {
                    let mut model = model();
                    model.error = error.then(|| "This workspace name already exists".into());
                    let mut text = Text::default();
                    let mut view = FieldView::default();
                    let palette = Palette::new(theme, brand).unwrap();
                    let mut scene = Scene::new();
                    let paint = || {
                        view.render_with_presentation(
                            &mut scene,
                            &mut text,
                            &model,
                            Rect::new(8., 8., 140., 2000.),
                            None,
                            palette,
                            proposal::TextRole::UiBody,
                            if emphasized {
                                FieldPresentation::emphasized_form()
                            } else {
                                FieldPresentation::default()
                            },
                        )
                    };
                    #[cfg(feature = "token-audit")]
                    let (layout, reads) = tekes_ui_tokens::audit::capture(paint);
                    #[cfg(not(feature = "token-audit"))]
                    let (layout, reads) = (
                        {
                            let mut paint = paint;
                            paint()
                        },
                        Vec::<(String, String)>::new(),
                    );
                    assert!(layout.required_mark.unwrap().size.width > 0.);
                    assert!(layout.slot.min_y() >= layout.label.max_y());
                    assert!(layout.message.min_y() >= layout.slot.max_y());
                    assert_eq!(layout.has_error, error);
                    assert!(text.draws.iter().any(|(s, _, _, _)| s == model.message()));
                    for (value, _, _, ink) in &text.draws {
                        let ink_role = if value == model.message() {
                            if error {
                                "status-failed"
                            } else {
                                "secondary-text"
                            }
                        } else {
                            "primary-text"
                        };
                        assert_eq!(
                            *ink,
                            palette.color(ink_role).unwrap(),
                            "{value}: {ink_role}"
                        );
                    }
                    if error {
                        assert!(
                            !text
                                .draws
                                .iter()
                                .any(|(value, _, _, _)| value == &model.help)
                        );
                    }
                    assert_eq!(model.label, "A long required workspace display name");
                    cases.push(serde_json::json!({"component":"tui.field","theme":theme_index,"brand":brand,"emphasized":emphasized,"error":error,"content_state":if error {"error"} else {"rest"},"state_asserted":true,"lookups":reads}));
                }
            }
        }
    }
    assert_eq!(cases.len(), 48);
    if let Some(path) = std::env::var_os("TEKESUI_FIELD_AUDIT_OUT") {
        std::fs::write(path,serde_json::to_vec_pretty(&serde_json::json!({"complete":false,"cases":cases,"scope":"Existing Field metadata only; wrapping required marker, message precedence, body/heading roles","excluded":["disabled editor presentation and interaction","font backend","compiled constants","unexecuted paths","native and VoiceOver acceptance"]})).unwrap()).unwrap();
    }
}
