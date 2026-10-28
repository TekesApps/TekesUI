//! Validate inherited environment, then update the original module/backend.
use crate::{ProcessModule, component::ViewContext};
use anyhow::{Result, ensure};
use tekes_ui_tokens::{Appearance, Contrast, Palette, Theme};

/// Resolve the existing compiled JSON palette. No colors are authored on wire.
pub fn view_context_palette(context: &ViewContext) -> Result<Palette> {
    ensure!(
        context.type_scale.is_finite() && context.type_scale > 0.,
        "invalid View typography scale"
    );
    ensure!(
        tekes_ui_tokens::surface_palette::canonical_surface(&context.surface).is_ok(),
        "unknown View Surface: {}",
        context.surface
    );
    Palette::new(
        Theme {
            appearance: if context.dark {
                Appearance::Dark
            } else {
                Appearance::Light
            },
            contrast: if context.increased_contrast {
                Contrast::Increased
            } else {
                Contrast::Standard
            },
        },
        context.brand,
    )
    .ok_or_else(|| anyhow::anyhow!("unknown View palette"))
}
/// Inherited context for the original SDK Content tree. Explicit Content
/// declarations still override this window/module default.
pub fn view_context_theme(
    context: &ViewContext,
) -> Result<tekes_ui_controls::composition::ContentTheme> {
    Ok(
        tekes_ui_controls::composition::ContentTheme::from(view_context_palette(context)?)
            .with_surface(&context.surface)
            .expect("validated View Surface"),
    )
}
/// The worker's clear color follows the same Surface as its original Content.
pub fn view_context_background(context: &ViewContext) -> Result<tekes_ui_tokens::Rgba> {
    let theme = view_context_theme(context)?;
    Ok(tekes_ui_tokens::surface_palette::resolve_surface(
        theme.palette,
        theme.surface(),
        "content",
        "rest",
    )
    .expect("generated View content background")
    .background)
}
pub(crate) fn apply(
    module: &mut dyn ProcessModule,
    previous: Option<&ViewContext>,
    next: &ViewContext,
) -> Result<bool> {
    view_context_palette(next)?;
    ensure!(
        previous.is_none_or(|old| next.elapsed_nanos >= old.elapsed_nanos),
        "View clock went backwards"
    );
    module.set_view_context(next)?;
    if let Some((host, _)) = module.content() {
        host.set_inherited_surface(&next.surface)
            .expect("validated original Content Surface");
    }
    Ok(previous.is_none_or(|old| old.type_scale != next.type_scale))
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Original {
        received: Vec<ViewContext>,
    }
    impl ProcessModule for Original {
        fn set_view_context(&mut self, context: &ViewContext) -> Result<()> {
            self.received.push(context.clone());
            Ok(())
        }
        fn handle(
            &mut self,
            _: crate::component::RequestBody,
            _: &crate::ComponentEmitter,
        ) -> Result<serde_json::Value> {
            unreachable!()
        }
    }
    #[test]
    fn only_typography_revokes_original_shape_and_clock_never_goes_backwards() {
        let mut original = Original { received: vec![] };
        let first = ViewContext::default();
        assert!(apply(&mut original, None, &first).unwrap());
        let visual = ViewContext {
            dark: true,
            increased_contrast: true,
            caret_visible: false,
            reduced_motion: true,
            elapsed_nanos: 10,
            ..first.clone()
        };
        assert!(!apply(&mut original, Some(&first), &visual).unwrap());
        assert_eq!(original.received.last(), Some(&visual));
        let zoom = ViewContext {
            type_scale: 2.,
            ..visual.clone()
        };
        assert!(apply(&mut original, Some(&visual), &zoom).unwrap());
        assert!(apply(&mut original, Some(&zoom), &first).is_err());
        assert_eq!(original.received.len(), 3);
    }
    #[test]
    fn malformed_environment_never_reaches_original_module() {
        let mut original = Original { received: vec![] };
        for scale in [f32::NAN, f32::INFINITY, -1., 0.] {
            assert!(
                apply(
                    &mut original,
                    None,
                    &ViewContext {
                        type_scale: scale,
                        ..Default::default()
                    }
                )
                .is_err()
            );
        }
        assert!(
            apply(
                &mut original,
                None,
                &ViewContext {
                    brand: usize::MAX,
                    ..Default::default()
                }
            )
            .is_err()
        );
        assert!(original.received.is_empty());
        for surface in ["unknown", "Panel", "#ffffff", ""] {
            let invalid = ViewContext {
                surface: surface.into(),
                ..Default::default()
            };
            assert!(apply(&mut original, None, &invalid).is_err());
        }
        assert!(original.received.is_empty());
        for dark in [false, true] {
            for increased_contrast in [false, true] {
                let c = ViewContext {
                    dark,
                    increased_contrast,
                    ..Default::default()
                };
                let p = view_context_palette(&c).unwrap();
                assert_eq!(p.theme.is_dark(), dark);
                assert_eq!(p.theme.is_increased(), increased_contrast);
                assert!(p.color("surface-1").is_some());
            }
        }
    }
    #[test]
    fn every_compiled_surface_uses_the_same_original_theme_and_clear_color() {
        for mode in Theme::ALL {
            for surface in tekes_ui_tokens::surface_palette::surface_names() {
                let context = ViewContext {
                    surface: (*surface).into(),
                    dark: mode.is_dark(),
                    increased_contrast: mode.is_increased(),
                    ..Default::default()
                };
                let resolved = view_context_theme(&context).unwrap();
                assert_eq!(resolved.surface(), *surface);
                assert_eq!(resolved.palette.theme, mode);
                let expected = tekes_ui_tokens::surface_palette::resolve_surface(
                    resolved.palette,
                    surface,
                    "content",
                    "rest",
                )
                .unwrap();
                assert_eq!(
                    view_context_background(&context).unwrap(),
                    expected.background
                );
            }
        }
    }
    #[test]
    fn module_without_context_backend_fails_explicitly() {
        struct Legacy;
        impl ProcessModule for Legacy {
            fn handle(
                &mut self,
                _: crate::component::RequestBody,
                _: &crate::ComponentEmitter,
            ) -> Result<serde_json::Value> {
                unreachable!()
            }
        }
        assert!(
            apply(&mut Legacy, None, &ViewContext::default())
                .unwrap_err()
                .to_string()
                .contains("not supported")
        );
    }
}
