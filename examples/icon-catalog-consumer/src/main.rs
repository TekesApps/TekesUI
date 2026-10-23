//! A second consumer of only the published icon API, with no Gallery imports.
use std::collections::HashSet;
use tekes_ui_icons::{Glyph, Icon};
fn main() {
    let mut names = HashSet::new();
    for glyph in Glyph::ALL {
        assert!(names.insert(glyph.name));
        assert_eq!(Glyph::named(glyph.name), Some(*glyph));
        assert!(glyph.svg.contains("width=\"24\" height=\"24\""));
        assert!(glyph.svg.contains("viewBox=\"0 0 24 24\""));
        assert!(glyph.svg.contains("currentColor"));
    }
    for icon in Icon::ALL {
        assert_eq!(Glyph::named(icon.name()).unwrap().svg, icon.svg());
        assert!(!icon.animation_layers().is_empty());
    }
    assert!(Glyph::named("missing-icon").is_none());
    println!("{} square canonical glyphs; all 8 animated defaults share the exact public artwork", names.len());
}
