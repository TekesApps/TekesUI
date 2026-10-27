//! #122: original Field metadata/View only; the input's editor is a separate owner.
#![cfg(target_os = "macos")]
#[path = "support/coretext.rs"]
mod coretext;
use tekes_ui_controls::{Field, feedback_view::FieldView};
use tekes_ui_scene::{Primitive, Quad, Rect, Scene};
use tekes_ui_tokens::{Palette, Rgba, Theme, proposal};
fn contrast(a: Rgba, b: Rgba) -> f32 {
    let luminance = |c: Rgba| {
        let v = c.to_linear();
        0.2126 * v[0] + 0.7152 * v[1] + 0.0722 * v[2]
    };
    let (a, b) = (luminance(a), luminance(b));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}
#[test]
fn field_retained_themes_keep_required_label_message_roles_and_input_slot() {
    let mut model = Field {
        label: "Workspace name".into(),
        help: "Use a clear name.".into(),
        error: None,
        required: true,
    };
    let mut view = FieldView::default();
    let mut text = coretext::Text::new(1.);
    let frame = Rect::new(16., 20., 340., 140.);
    let clip = Rect::new(8., 8., 400., 220.);
    let mut observed = Vec::new();
    for error in [false, true] {
        model.error = error.then(|| "A name is required.".into());
        let mut original = None;
        for theme in Theme::ALL {
            let palette = Palette::new(theme, 0).unwrap();
            let background = palette.color("surface-1").unwrap();
            text.atlas.begin_frame();
            let mut scene = Scene::new();
            scene.push_quad(Quad::new(clip, background));
            let layout = view.render(
                &mut scene,
                &mut text,
                &model,
                frame,
                Some(clip),
                palette,
                proposal::TextRole::UiBody,
            );
            if let Some(original) = original {
                assert_eq!(layout, original);
            } else {
                original = Some(layout);
            }
            assert_eq!(view.layout(), layout);
            assert_eq!(layout.visible, Some(frame));
            assert_eq!(layout.has_error, error);
            assert_eq!(
                model.message(),
                if error {
                    "A name is required."
                } else {
                    "Use a clear name."
                }
            );
            assert!(layout.label.max_y() <= layout.slot.min_y());
            assert!(layout.slot.max_y() <= layout.message.min_y());
            let mark = layout
                .required_mark
                .expect("required metadata retains its original visible mark");
            assert!(mark.size.width > 0. && mark.max_x() <= frame.max_x());
            for (region, role) in [
                (layout.label, "primary-text"),
                (mark, "primary-text"),
                (
                    layout.message,
                    if error {
                        "status-failed"
                    } else {
                        "secondary-text"
                    },
                ),
            ] {
                let inks: Vec<_> = scene
                    .primitives()
                    .iter()
                    .filter_map(|p| match p {
                        Primitive::Glyphs(run)
                            if run.glyphs.iter().any(|g| region.contains(g.origin)) =>
                        {
                            Some(run.color)
                        }
                        _ => None,
                    })
                    .collect();
                assert!(
                    !inks.is_empty(),
                    "actual Field metadata CoreText glyphs in {region:?}"
                );
                for ink in inks {
                    assert_eq!(ink, palette.color(role).unwrap());
                    assert!(
                        contrast(ink, background) >= if theme.is_increased() { 7. } else { 4.5 },
                        "{theme:?} {role}: contrast={}",
                        contrast(ink, background)
                    );
                }
            }
            observed.push(palette.color("primary-text").unwrap());
        }
    }
    assert!(observed.windows(2).any(|w| w[0] != w[1]));
}
