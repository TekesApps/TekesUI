//! #122: retained editable numeric drafts; parser/IME matrices stay in editable_number.
#![cfg(target_os = "macos")]
#[path = "support/coretext.rs"]
mod coretext;
use tekes_ui_controls::{ControlKey, composition::*};
use tekes_ui_scene::{Primitive, Rect, Scene};
use tekes_ui_tokens::{Palette, Rgba, Theme, proposal};
const NUMBER: ComponentId = ComponentId(122_030);
fn field(host: &ContentHost) -> &NumberFieldContent {
    host.get(NUMBER)
        .unwrap()
        .component::<NumberFieldContent>()
        .unwrap()
}
fn field_mut(host: &mut ContentHost) -> &mut NumberFieldContent {
    host.get_mut(NUMBER)
        .unwrap()
        .component_mut::<NumberFieldContent>()
        .unwrap()
}
fn contrast(a: Rgba, b: Rgba) -> f32 {
    let luminance = |c: Rgba| {
        let c = c.to_linear();
        0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
    };
    let (a, b) = (luminance(a), luminance(b));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}
#[test]
fn number_field_retained_themes_keep_draft_committed_value_and_actual_numeric_ink() {
    let mut number =
        NumberFieldContent::new("Quantity", 4., 0., 8., 1., DecimalNumberFormat::default())
            .unwrap();
    assert!(number.set_input_digits(3));
    let mut host = ContentHost::new(Content::new(NUMBER, number)).unwrap();
    let mut text = coretext::Text::new(1.);
    text.services = true;
    text.reduced_motion = true;
    let frame = Rect::new(20.25, 30.5, 280., 56.);
    let clip = Rect::new(10., 10., 400., 160.);
    host.render(
        &mut Scene::new(),
        &mut text,
        frame,
        clip,
        Theme::LIGHT.into(),
    )
    .unwrap();
    assert!(host.focus(Some(NUMBER), true));
    let mount = host.semantic_mount(NUMBER).unwrap();
    let mut observed = Vec::new();
    for (draft, committed, validation) in [
        ("7", 4., NumberValidation::Valid(7.)),
        ("-", 8., NumberValidation::Incomplete),
        ("x", 8., NumberValidation::Invalid),
    ] {
        host.accessibility_action(NUMBER, None, AccessibilityAction::SetValue(draft.into()));
        let original = field(&host).edit_state();
        assert_eq!(original.text, draft);
        assert_eq!(original.committed_value, committed);
        assert_eq!(original.validation, validation);
        assert!(original.dirty && original.focused && !original.composing);
        let mut geometry = None;
        for theme in Theme::ALL {
            let palette = Palette::new(theme, 0).unwrap();
            text.atlas.begin_frame();
            let mut scene = Scene::new();
            host.render(&mut scene, &mut text, frame, clip, palette)
                .unwrap();
            assert_eq!(host.semantic_mount(NUMBER), Some(mount));
            assert_eq!(field(&host).edit_state(), original);
            assert_eq!(field(&host).value(), committed);
            assert_eq!(field_mut(&mut host).drain_value_changes().count(), 0);
            let surface = host.text_surface(NUMBER).unwrap();
            let parts = field(&host).semantic_parts();
            assert_eq!(parts.len(), 2);
            let current = (surface.outer, surface.inner, parts[0].frame, parts[1].frame);
            if let Some(geometry) = geometry {
                assert_eq!(current, geometry);
            } else {
                geometry = Some(current);
            }
            assert_eq!(surface.error, draft != "7");
            assert_eq!(field(&host).semantics().invalid, draft != "7");
            let background = scene
                .primitives()
                .iter()
                .find_map(|p| match p {
                    Primitive::Quad(q) if q.rect == surface.outer && q.fill.a > 0. => Some(q),
                    _ => None,
                })
                .expect("original editable numeric TextSurface background");
            assert_eq!(background.fill, palette.color("transcript-input").unwrap());
            assert_eq!(
                background.border_color,
                palette
                    .color(
                        proposal::control_boundary(
                            if draft != "7" {
                                "error"
                            } else {
                                "required-input"
                            },
                            theme
                        )
                        .unwrap()
                    )
                    .unwrap()
            );
            assert!(
                contrast(background.border_color, background.fill) >= 3.,
                "{theme:?}: original numeric input boundary"
            );
            assert!(scene.primitives().iter().any(|p| matches!(p, Primitive::Quad(q) if q.fill.a == 0. && q.border_color == palette.color("border-focused").unwrap() && q.rect.intersection(&surface.outer).is_some())), "real focused editor ring");
            let regions = [
                (surface.inner, false, background.fill),
                (
                    parts[0].frame,
                    parts[0].semantics.disabled,
                    palette.color("surface-2").unwrap(),
                ),
                (
                    parts[1].frame,
                    parts[1].semantics.disabled,
                    palette.color("surface-2").unwrap(),
                ),
            ];
            for (region, disabled, ground) in regions {
                let labels: Vec<_> = scene
                    .primitives()
                    .iter()
                    .filter_map(|p| match p {
                        Primitive::Glyphs(run)
                            if run.glyphs.iter().any(|g| region.contains(g.origin)) =>
                        {
                            Some(run)
                        }
                        _ => None,
                    })
                    .collect();
                assert!(
                    !labels.is_empty(),
                    "actual numeric draft/step glyphs in {region:?}"
                );
                for run in labels {
                    assert_eq!(
                        run.color,
                        palette
                            .color(if disabled {
                                "tertiary-text"
                            } else {
                                "primary-text"
                            })
                            .unwrap()
                    );
                    if !disabled {
                        assert!(
                            contrast(run.color, ground)
                                >= if theme.is_increased() { 7. } else { 4.5 },
                            "{theme:?} {region:?}: {}",
                            contrast(run.color, ground)
                        );
                    }
                }
            }
            for part in &parts {
                assert_eq!(part.semantics.disabled, draft != "7");
                assert!(scene.primitives().iter().any(|p| matches!(p, Primitive::Quad(q) if q.rect == part.frame && q.fill == palette.color("surface-2").unwrap())));
            }
            observed.push(background.fill);
        }
        host.event(&ContentEvent::Key {
            key: ControlKey::Enter,
            down: true,
        });
        if draft == "7" {
            assert_eq!(field(&host).value(), 7.);
            host.event(&ContentEvent::Key {
                key: ControlKey::Up,
                down: true,
            });
            assert_eq!(field(&host).value(), 8.);
            let changes: Vec<_> = field_mut(&mut host).drain_value_changes().collect();
            assert_eq!(
                changes,
                vec![
                    NumberValueChange {
                        previous: 4.,
                        value: 7.,
                        reason: NumberCommitReason::Submit
                    },
                    NumberValueChange {
                        previous: 7.,
                        value: 8.,
                        reason: NumberCommitReason::Step
                    }
                ]
            );
            assert!(!field(&host).can_step(true));
        } else {
            assert_eq!(
                field(&host).edit_state(),
                original,
                "invalid/incomplete submit preserves the real draft"
            );
            host.event(&ContentEvent::Key {
                key: ControlKey::Escape,
                down: true,
            });
            assert_eq!(field(&host).edit_state().text, "8");
            assert_eq!(field(&host).value(), 8.);
            assert_eq!(field_mut(&mut host).drain_value_changes().count(), 0);
        }
    }
    assert!(observed.windows(2).any(|w| w[0] != w[1]));
}
