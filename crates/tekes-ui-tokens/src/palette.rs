//! Immutable semantic environment. Presets are outputs of the existing brand compiler,
//! never a second hand-authored color ramp. Reference (0) keeps the consumer bundle.
use crate::{Rgba, Theme, proposal};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GalleryBrandPreset {
    pub id: &'static str,
    pub name: &'static str,
    pub palette_index: usize,
    pub seed_srgb: &'static str,
    pub category: &'static str,
    pub color_family: &'static str,
    pub recommended_for_direction: &'static [&'static str],
    pub description: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GalleryBrandDirection {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
}

include!(concat!(env!("OUT_DIR"), "/brand_colors.rs"));
include!(concat!(env!("OUT_DIR"), "/theme_definition_colors.rs"));

pub fn gallery_brand_presets() -> &'static [GalleryBrandPreset] {
    GALLERY_BRAND_PRESETS
}

pub fn gallery_brand_directions() -> &'static [GalleryBrandDirection] {
    GALLERY_BRAND_DIRECTIONS
}

pub fn gallery_brand_preset(palette_index: usize) -> Option<&'static GalleryBrandPreset> {
    GALLERY_BRAND_PRESETS
        .iter()
        .find(|preset| preset.palette_index == palette_index)
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaletteSource {
    GalleryBrand { index: usize },
    ThemeDefinition { index: usize },
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
            source: PaletteSource::GalleryBrand { index: 0 },
        }
    }
}
impl Palette {
    /// Select a registered Gallery preview. Zero is the consumer's compiled
    /// BrandConfig, including an explicitly selected offline artifact.
    pub fn new(theme: Theme, brand: usize) -> Option<Self> {
        gallery_brand_preset(brand).map(|_| Self {
            theme,
            source: PaletteSource::GalleryBrand { index: brand },
        })
    }

    /// Select one Design System Theme definition compiled into this package.
    pub fn for_theme_definition(theme: Theme, id: &str) -> Option<Self> {
        theme_definition_index(id).map(|index| Self {
            theme,
            source: PaletteSource::ThemeDefinition { index },
        })
    }

    /// Return the explicit palette source without exposing mutable source state.
    pub const fn source(self) -> PaletteSource {
        self.source
    }

    /// Return the Gallery brand index when this palette selects a Gallery brand.
    pub const fn brand_index(self) -> Option<usize> {
        match self.source {
            PaletteSource::GalleryBrand { index } => Some(index),
            PaletteSource::ThemeDefinition { .. } => None,
        }
    }

    pub fn color(self, role: &str) -> Option<Rgba> {
        let index = Theme::ALL.iter().position(|t| *t == self.theme)?;
        match self.source {
            PaletteSource::GalleryBrand { index: brand } => {
                gallery_brand_preset(brand)?;
                let base = proposal::render_color(role, self.theme)?;
                if brand == 0 {
                    return Some(base);
                }
                let mut color = brand_color(brand, role, index).unwrap_or(base);
                color.a = base.a;
                Some(color)
            }
            PaletteSource::ThemeDefinition { index: definition } => {
                // Theme instances store opaque compiled RGB. Render alpha remains owned by the
                // original token role and mode, preserving scrim/selection/wash behavior.
                let mut color = theme_definition_color(definition, role, index)?;
                color.a = proposal::render_color(role, self.theme)?.a;
                Some(color)
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unknown_brand_and_role_fail_without_default_fallback() {
        for theme in Theme::ALL {
            assert!(Palette::new(theme, usize::MAX).is_none());
            assert!(
                Palette {
                    theme,
                    source: PaletteSource::GalleryBrand { index: usize::MAX },
                }
                .color("primary-text")
                .is_none()
            );
            assert!(
                Palette::new(theme, 0)
                    .unwrap()
                    .color("unknown-role")
                    .is_none()
            );
        }
    }
    #[test]
    fn every_registered_preset_compiles_four_themes_and_preserves_independent_roles() {
        assert_eq!(gallery_brand_presets().len(), 13);
        assert_eq!(gallery_brand_preset(0).unwrap().id, "consumer-reference");
        assert_eq!(gallery_brand_directions().len(), 7);
        for theme in Theme::ALL {
            let baseline = Palette::from(theme);
            for preset in gallery_brand_presets() {
                let candidate = Palette::new(theme, preset.palette_index).unwrap();
                for role in [
                    "action-primary-ground",
                    "action-primary-foreground",
                    "c-link-foreground",
                    "accent-subtle-ground",
                ] {
                    assert!(candidate.color(role).is_some(), "{} / {role}", preset.id);
                }
                for role in [
                    "primary-text",
                    "secondary-text",
                    "border-focused",
                    "text-selection",
                    "row-selection",
                    "scrim",
                ] {
                    assert_eq!(
                        candidate.color(role),
                        baseline.color(role),
                        "{} / {role}",
                        preset.id
                    );
                }
            }
        }
    }

    #[test]
    fn hue_choices_reference_registered_candidates_without_implicit_application() {
        for direction in gallery_brand_directions() {
            let matches: Vec<_> = gallery_brand_presets()
                .iter()
                .filter(|preset| preset.recommended_for_direction.contains(&direction.id))
                .collect();
            assert_eq!(matches.len(), 1, "{}", direction.id);
            assert_eq!(matches[0].color_family, direction.id);
        }
        assert_eq!(Palette::from(Theme::LIGHT).brand_index(), Some(0));
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
        // The reviewed blue-inputs fixture changes p-status-info and p-accent in
        // light mode; its p-primary-text override changes every compiled mode.
        let status_changes_by_mode = [true, false, false, false];
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
}
