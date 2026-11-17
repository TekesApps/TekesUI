//! Gallery brand preview presets compiled from `design/gallery/brands.json`.
use tekes_ui_tokens::{CustomPaletteSource, Palette, Rgba, Theme};

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

include!(concat!(env!("OUT_DIR"), "/gallery_brands.rs"));

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

fn resolve_gallery_brand(brand: usize, role: &str, theme: Theme) -> Option<Rgba> {
    if brand == 0 {
        return None;
    }
    let theme_index = Theme::ALL
        .iter()
        .position(|candidate| *candidate == theme)?;
    brand_color(brand, role, theme_index)
}

/// Select a compiled Gallery preview preset by its runtime index (`0..GALLERY_BRAND_PRESETS.len()`).
pub fn brand_palette(theme: Theme, palette_index: usize) -> Option<Palette> {
    let preset = gallery_brand_preset(palette_index)?;
    Some(Palette::from_custom(
        theme,
        CustomPaletteSource::new(preset.palette_index, preset.id, resolve_gallery_brand),
    ))
}

/// Select a compiled Gallery preview preset by its preset identifier.
#[cfg(test)]
pub fn brand_palette_by_id(theme: Theme, id: &str) -> Option<Palette> {
    GALLERY_BRAND_PRESETS
        .iter()
        .find(|preset| preset.id == id)
        .map(|preset| {
            Palette::from_custom(
                theme,
                CustomPaletteSource::new(preset.palette_index, preset.id, resolve_gallery_brand),
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_registered_preset_compiles_four_themes_and_preserves_independent_roles() {
        assert_eq!(gallery_brand_presets().len(), 13);
        assert_eq!(gallery_brand_preset(0).unwrap().id, "consumer-reference");
        assert_eq!(gallery_brand_directions().len(), 7);
        assert!(brand_palette(Theme::LIGHT, usize::MAX).is_none());
        for theme in Theme::ALL {
            let baseline = Palette::from(theme);
            for preset in gallery_brand_presets() {
                let candidate = brand_palette(theme, preset.palette_index).unwrap();
                assert_eq!(candidate.brand_index(), Some(preset.palette_index));
                assert_eq!(
                    brand_palette_by_id(theme, preset.id).and_then(|p| p.brand_index()),
                    Some(preset.palette_index)
                );
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
        assert_eq!(
            brand_palette(Theme::LIGHT, 0).unwrap().brand_index(),
            Some(0)
        );
    }
}
