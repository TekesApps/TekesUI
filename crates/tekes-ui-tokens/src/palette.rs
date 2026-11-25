//! Immutable semantic environment backed by compiled proposal tokens,
//! compiled Design System Theme definitions, or explicit consumer palette sources.
use crate::{Rgba, Theme, proposal};

include!(concat!(env!("OUT_DIR"), "/theme_definition_colors.rs"));

/// Host-provided or application-defined custom color resolver.
#[derive(Clone, Copy)]
pub struct CustomPaletteSource {
    pub id: usize,
    pub name: &'static str,
    resolve: fn(usize, &str, Theme) -> Option<Rgba>,
}

impl CustomPaletteSource {
    pub const fn new(
        id: usize,
        name: &'static str,
        resolve: fn(usize, &str, Theme) -> Option<Rgba>,
    ) -> Self {
        Self { id, name, resolve }
    }
}

impl std::fmt::Debug for CustomPaletteSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CustomPaletteSource")
            .field("id", &self.id)
            .field("name", &self.name)
            .finish()
    }
}

impl PartialEq for CustomPaletteSource {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.name == other.name
    }
}

impl Eq for CustomPaletteSource {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaletteSource {
    /// Framework neutral / consumer default compiled token bundle (`proposal::render_color`).
    Default,
    /// Design System Theme definition compiled into this package.
    ThemeDefinition { index: usize },
    /// Application or Gallery custom palette source.
    Custom(CustomPaletteSource),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Palette {
    pub theme: Theme,
    source: PaletteSource,
}

impl From<Theme> for Palette {
    fn from(theme: Theme) -> Self {
        Self {
            theme,
            source: PaletteSource::Default,
        }
    }
}

fn compat_custom_alias(role: &str) -> Option<&'static str> {
    match role {
        "input-surface" => Some("transcript-input"),
        "transcript-input" => Some("input-surface"),
        "p-input-surface" => Some("p-transcript-input"),
        "p-transcript-input" => Some("p-input-surface"),
        _ => None,
    }
}

impl Palette {
    /// Select a compiled palette by index. Index `0` selects [`PaletteSource::Default`];
    /// non-zero indices select a compiled Theme definition (`1..theme_definition_count`).
    pub fn new(theme: Theme, index: usize) -> Option<Self> {
        if index == 0 {
            Some(Self::from(theme))
        } else if index < theme_definition_count() {
            Some(Self {
                theme,
                source: PaletteSource::ThemeDefinition { index },
            })
        } else {
            None
        }
    }

    /// Select one Design System Theme definition compiled into this package.
    pub fn for_theme_definition(theme: Theme, id: &str) -> Option<Self> {
        theme_definition_index(id).map(|index| Self {
            theme,
            source: PaletteSource::ThemeDefinition { index },
        })
    }

    /// Construct a palette backed by an explicit custom color resolver.
    pub const fn from_custom(theme: Theme, custom: CustomPaletteSource) -> Self {
        Self {
            theme,
            source: PaletteSource::Custom(custom),
        }
    }

    /// Rebind this palette to `theme` while preserving its [`PaletteSource`].
    pub const fn with_theme(self, theme: Theme) -> Self {
        Self {
            theme,
            source: self.source,
        }
    }

    /// Return the explicit palette source without exposing mutable source state.
    pub const fn source(self) -> PaletteSource {
        self.source
    }

    /// Return the custom source ID when this palette uses [`PaletteSource::Custom`].
    pub const fn brand_index(self) -> Option<usize> {
        match self.source {
            PaletteSource::Custom(custom) => Some(custom.id),
            PaletteSource::Default | PaletteSource::ThemeDefinition { .. } => None,
        }
    }

    pub fn color(self, role: &str) -> Option<Rgba> {
        let index = Theme::ALL.iter().position(|t| *t == self.theme)?;
        let color = match self.source {
            PaletteSource::Default => proposal::render_color(role, self.theme),
            PaletteSource::ThemeDefinition { index: definition } => {
                // Theme instances store opaque compiled RGB. Render alpha remains owned by the
                // original token role and mode, preserving scrim/selection/wash behavior.
                let mut color = theme_definition_color(definition, role, index)?;
                color.a = proposal::render_color(role, self.theme)?.a;
                Some(color)
            }
            PaletteSource::Custom(custom) => {
                let base = proposal::render_color(role, self.theme)?;
                let resolved = (custom.resolve)(custom.id, role, self.theme).or_else(|| {
                    compat_custom_alias(role)
                        .and_then(|alias| (custom.resolve)(custom.id, alias, self.theme))
                });
                let mut color = resolved.unwrap_or(base);
                color.a = base.a;
                Some(color)
            }
        }?;
        #[cfg(feature = "token-audit")]
        crate::audit::record_color(role, self, color);
        Some(color)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_custom_color(_id: usize, role: &str, _theme: Theme) -> Option<Rgba> {
        (role == "action-primary-ground").then(|| Rgba::rgb8(12, 34, 56))
    }

    #[test]
    fn unknown_index_and_role_fail_without_default_fallback() {
        for theme in Theme::ALL {
            assert!(Palette::new(theme, usize::MAX).is_none());
            assert!(
                Palette {
                    theme,
                    source: PaletteSource::ThemeDefinition { index: usize::MAX },
                }
                .color("primary-text")
                .is_none()
            );
            assert!(Palette::from(theme).color("unknown-role").is_none());
        }
    }

    #[test]
    fn default_theme_is_neutral_and_custom_sources_preserve_identity_across_modes() {
        for theme in Theme::ALL {
            let baseline = Palette::from(theme);
            assert_eq!(baseline.source(), PaletteSource::Default);
            assert_eq!(baseline.brand_index(), None);
            assert_eq!(
                baseline.color("primary-text"),
                proposal::render_color("primary-text", theme)
            );
            let custom = Palette::from_custom(
                theme,
                CustomPaletteSource::new(7, "test-custom", test_custom_color),
            );
            assert_eq!(custom.brand_index(), Some(7));
            assert_eq!(
                custom.color("action-primary-ground").unwrap(),
                Rgba {
                    a: baseline.color("action-primary-ground").unwrap().a,
                    ..Rgba::rgb8(12, 34, 56)
                }
            );
            assert_eq!(custom.color("primary-text"), baseline.color("primary-text"));
            assert_eq!(custom.with_theme(Theme::DARK).brand_index(), Some(7));
        }
    }

    #[test]
    fn concrete_theme_definitions_share_palette_and_preserve_render_alpha() {
        for theme in Theme::ALL {
            let default = Palette::for_theme_definition(theme, "default").unwrap();
            assert_eq!(
                default.source(),
                PaletteSource::ThemeDefinition { index: 0 }
            );
            assert_eq!(default.brand_index(), None);
            for role in [
                "primary-text",
                "p-primary-text",
                "p-status-info",
                "action-primary-ground",
                "scrim",
                "text-selection",
            ] {
                let generated = default.color(role).unwrap();
                let legacy = Palette::from(theme).color(role).unwrap();
                assert_eq!(generated.r, legacy.r, "{role} / {theme:?}");
                assert_eq!(generated.g, legacy.g, "{role} / {theme:?}");
                assert_eq!(generated.b, legacy.b, "{role} / {theme:?}");
                assert_eq!(generated.a, legacy.a, "{role} / {theme:?}");
            }
            assert_eq!(
                default.color("scrim").unwrap().a,
                proposal::render_color("scrim", theme).unwrap().a
            );
        }
        assert!(Palette::for_theme_definition(Theme::LIGHT, "missing-theme").is_none());
    }

    #[test]
    fn same_brand_config_definitions_keep_independent_effective_color_inputs() {
        // The reviewed blue-inputs fixture changes p-status-info in ordinary
        // light/dark modes and p-accent in light mode; its p-primary-text
        // override changes every compiled mode.
        let status_changes_by_mode = [true, true, false, false];
        let accent_changes_by_mode = [true, false, false, false];
        for (mode_index, theme) in Theme::ALL.into_iter().enumerate() {
            let blue = Palette::for_theme_definition(theme, "blue").unwrap();
            let blue_inputs = Palette::for_theme_definition(theme, "blue-inputs").unwrap();
            assert_eq!(blue.source(), PaletteSource::ThemeDefinition { index: 1 });
            assert_eq!(
                blue_inputs.source(),
                PaletteSource::ThemeDefinition { index: 3 }
            );
            assert_eq!(blue.brand_index(), None);
            assert_ne!(
                blue.color("p-primary-text"),
                blue_inputs.color("p-primary-text")
            );
            assert_eq!(
                blue.color("p-status-info") != blue_inputs.color("p-status-info"),
                status_changes_by_mode[mode_index],
                "status input differs in mode {mode_index}"
            );
            assert_eq!(
                blue.color("p-accent") != blue_inputs.color("p-accent"),
                accent_changes_by_mode[mode_index],
                "BrandConfig-derived accent differs in mode {mode_index}"
            );
        }
    }

    #[test]
    fn custom_palette_bridges_legacy_transcript_input_and_neutral_input_surface_without_affecting_readonly()
     {
        fn legacy_only(_id: usize, role: &str, _theme: Theme) -> Option<Rgba> {
            (role == "transcript-input").then(|| Rgba::rgb8(18, 52, 86))
        }
        fn neutral_only(_id: usize, role: &str, _theme: Theme) -> Option<Rgba> {
            (role == "input-surface").then(|| Rgba::rgb8(44, 88, 122))
        }
        fn both_explicit(_id: usize, role: &str, _theme: Theme) -> Option<Rgba> {
            match role {
                "input-surface" => Some(Rgba::rgb8(10, 20, 30)),
                "transcript-input" => Some(Rgba::rgb8(40, 50, 60)),
                _ => None,
            }
        }
        for theme in Theme::ALL {
            let legacy_palette = Palette::from_custom(
                theme,
                CustomPaletteSource::new(101, "legacy-brand", legacy_only),
            );
            assert_eq!(
                legacy_palette.color("input-surface"),
                Some(Rgba::rgb8(18, 52, 86))
            );
            assert_eq!(
                legacy_palette.color("transcript-input"),
                Some(Rgba::rgb8(18, 52, 86))
            );
            let editable =
                crate::surface_palette::resolve_surface(legacy_palette, "canvas", "input", "rest")
                    .unwrap();
            assert_eq!(editable.background, Rgba::rgb8(18, 52, 86));
            let readonly = crate::surface_palette::resolve_surface_with_options(
                legacy_palette,
                "canvas",
                "input",
                "rest",
                crate::surface_palette::SurfaceOptions {
                    read_only: true,
                    error: false,
                },
            )
            .unwrap();
            assert_eq!(
                readonly.background,
                Palette::from(theme).color("surface-1").unwrap()
            );

            let neutral_palette = Palette::from_custom(
                theme,
                CustomPaletteSource::new(102, "neutral-brand", neutral_only),
            );
            assert_eq!(
                neutral_palette.color("transcript-input"),
                Some(Rgba::rgb8(44, 88, 122))
            );

            let explicit_palette = Palette::from_custom(
                theme,
                CustomPaletteSource::new(103, "explicit-brand", both_explicit),
            );
            assert_eq!(
                explicit_palette.color("input-surface"),
                Some(Rgba::rgb8(10, 20, 30))
            );
            assert_eq!(
                explicit_palette.color("transcript-input"),
                Some(Rgba::rgb8(40, 50, 60))
            );
        }
    }
}
