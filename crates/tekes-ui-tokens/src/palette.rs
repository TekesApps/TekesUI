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
#[derive(Clone, Copy, Debug)]
pub struct Palette {
    pub theme: Theme,
    pub brand: usize,
}
impl From<Theme> for Palette {
    fn from(theme: Theme) -> Self {
        Self { theme, brand: 0 }
    }
}
impl Palette {
    pub fn color(self, role: &str) -> Option<Rgba> {
        let base = proposal::render_color(role, self.theme)?;
        if self.brand == 0 {
            return Some(base);
        }
        let index = Theme::ALL.iter().position(|t| *t == self.theme)?;
        let mut color = brand_color(self.brand, role, index).unwrap_or(base);
        color.a = base.a;
        Some(color)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_registered_preset_compiles_four_themes_and_preserves_independent_roles() {
        assert_eq!(gallery_brand_presets().len(), 13);
        assert_eq!(gallery_brand_preset(0).unwrap().id, "consumer-reference");
        assert_eq!(gallery_brand_directions().len(), 7);
        for theme in Theme::ALL {
            let baseline = Palette { theme, brand: 0 };
            for preset in gallery_brand_presets() {
                let candidate = Palette {
                    theme,
                    brand: preset.palette_index,
                };
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
        assert_eq!(Palette::from(Theme::LIGHT).brand, 0);
    }
}
