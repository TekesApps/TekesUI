//! Complete Surface × use × state colors. Composition owns parent context propagation.
//! This is additive: existing `Theme` is the appearance/contrast mode selector.
use crate::{Palette, Rgba};
include!("../../../design/theme/generated/surface_rules.rs");

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SurfaceColors {
    pub background: Rgba,
    pub text: Rgba,
    pub icon: Rgba,
    pub boundary: Option<Rgba>,
    /// Independent keyboard focus, painted only when focus-visible is true.
    pub focus: Option<Rgba>,
    /// Interior check/mixed/selection marker, separate from the surrounding label.
    pub marker: Option<Rgba>,
    /// Input helper/error text, independent of keyboard focus and label text.
    pub helper: Option<Rgba>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SurfaceError {
    UnknownCombination,
    UnknownRole,
}
/// Canonical Surface vocabulary generated from the same rules as all bindings.
pub fn surface_names() -> &'static [&'static str] {
    SURFACE_NAMES
}
/// Validate once at the original mount/context boundary, then carry this Copy identity.
/// No allocation, RGB inference, case coercion or unknown-name fallback.
pub fn canonical_surface(name: &str) -> Result<&'static str, SurfaceError> {
    SURFACE_NAMES
        .binary_search(&name)
        .map(|index| SURFACE_NAMES[index])
        .map_err(|_| SurfaceError::UnknownCombination)
}
/// Resolve all visual channels together; never fall back to a guessed Surface.
/// `selected-hover` etc are presentation states, not a replacement state machine.
pub fn resolve_surface(
    palette: Palette,
    surface: &str,
    usage: &str,
    state: &str,
) -> Result<SurfaceColors, SurfaceError> {
    let roles = surface_roles(surface, usage, state).ok_or(SurfaceError::UnknownCombination)?;
    let required = |i: usize| {
        roles[i]
            .and_then(|r| palette.color(r))
            .ok_or(SurfaceError::UnknownRole)
    };
    let optional = |i: usize| {
        roles[i]
            .map(|r| palette.color(r).ok_or(SurfaceError::UnknownRole))
            .transpose()
    };
    Ok(SurfaceColors {
        background: required(0)?,
        text: required(1)?,
        icon: required(2)?,
        boundary: optional(3)?,
        focus: optional(4)?,
        marker: optional(5)?,
        helper: optional(6)?,
    })
}

/// Semantic flags select generated variants; no state machine or color derivation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SurfaceOptions {
    pub error: bool,
    pub read_only: bool,
}
/// Readonly is valid only for the original input surface and remains enabled.
/// For bare selectable text use `content-selectable` + `text-selection`.
pub fn resolve_surface_with_options(
    palette: Palette,
    surface: &str,
    usage: &str,
    state: &str,
    options: SurfaceOptions,
) -> Result<SurfaceColors, SurfaceError> {
    let resolved_use = match (usage, options.error, options.read_only) {
        ("input", false, true) => "input-readonly",
        ("input", true, true) => "input-readonly-error",
        ("input", true, false) => "input-error",
        ("action-quiet", true, false) => "action-quiet-error",
        ("action-primary", true, false) => "action-primary-error",
        ("selection", true, false) => "selection-error",
        ("check-unchecked", true, false) => "check-unchecked-error",
        ("check-checked", true, false) => "check-checked-error",
        ("check-mixed", true, false) => "check-mixed-error",
        ("toggle-track", true, false) => "toggle-track-error",
        (_, false, false) => usage,
        _ => return Err(SurfaceError::UnknownCombination),
    };
    resolve_surface(palette, surface, resolved_use, state)
}

/// Shared binary-toggle anatomy, including its outside label in the parent Surface.
/// The track owns focus/error; the thumb keeps its own fill and boundary.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ToggleSurfaceColors {
    pub track: SurfaceColors,
    pub thumb: SurfaceColors,
    pub label: SurfaceColors,
}

/// Resolve every part before painting. Original model selection means on/off;
/// this function neither changes that model nor derives colors from RGB.
pub fn resolve_toggle_surface(
    palette: Palette,
    surface: &str,
    state: &str,
    options: SurfaceOptions,
) -> Result<ToggleSurfaceColors, SurfaceError> {
    Ok(ToggleSurfaceColors {
        track: resolve_surface_with_options(palette, surface, "toggle-track", state, options)?,
        thumb: resolve_surface(palette, surface, "toggle-thumb", state)?,
        label: resolve_surface(palette, surface, "control-label", state)?,
    })
}

/// Original scalar-range anatomy. Track marker is the filled interval, not a knob.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RangeSurfaceColors {
    pub track: SurfaceColors,
    pub thumb: SurfaceColors,
    pub label: SurfaceColors,
}

/// Resolve all original range parts together. Supported states are rest/hover/
/// pressed/disabled only; the component supplies its effective disabled state.
/// Selection, validation and readonly are not invented for the original range.
pub fn resolve_range_surface(
    palette: Palette,
    surface: &str,
    state: &str,
) -> Result<RangeSurfaceColors, SurfaceError> {
    Ok(RangeSurfaceColors {
        track: resolve_surface(palette, surface, "range-track", state)?,
        thumb: resolve_surface(palette, surface, "range-thumb", state)?,
        label: resolve_surface(palette, surface, "control-label", state)?,
    })
}

#[cfg(test)]
mod range_tests {
    use super::*;
    use crate::Theme;

    #[test]
    fn original_range_parts_in_every_mode_brand_surface_and_supported_state() {
        let mut count = 0;
        for mode in Theme::ALL {
            for preset in crate::palette::gallery_brand_presets() {
                let p = Palette::new(mode, preset.palette_index).unwrap();
                for surface in surface_names() {
                    for state in ["rest", "hover", "pressed", "disabled"] {
                        let c = resolve_range_surface(p, surface, state).unwrap();
                        let disabled = state == "disabled";
                        assert_eq!(
                            c.track.background,
                            p.color(if disabled { "surface-2" } else { "surface-3" })
                                .unwrap()
                        );
                        assert_eq!(
                            c.track.marker,
                            p.color(match state {
                                "disabled" => "disabled-text",
                                "hover" => "accent-hover",
                                "pressed" => "accent-pressed",
                                _ => "accent",
                            })
                        );
                        assert_eq!(c.track.focus, None);
                        assert_eq!(
                            c.track.boundary,
                            p.color(if disabled { "disabled-text" } else { "border" })
                        );
                        assert_eq!(
                            c.thumb.background,
                            p.color(if disabled { "surface-2" } else { "surface-0" })
                                .unwrap()
                        );
                        assert_eq!(
                            c.thumb.boundary,
                            p.color(if disabled { "disabled-text" } else { "border" })
                        );
                        assert_eq!(
                            c.thumb.focus,
                            if disabled {
                                None
                            } else {
                                p.color("border-focused")
                            }
                        );
                        assert_eq!(
                            c.label.text,
                            p.color(if disabled {
                                "tertiary-text"
                            } else {
                                "primary-text"
                            })
                            .unwrap()
                        );
                        assert_eq!(
                            c.label.background,
                            resolve_surface(p, surface, "content", "rest")
                                .unwrap()
                                .background
                        );
                        count += 1;
                    }
                }
            }
        }
        assert_eq!(
            count,
            Theme::ALL.len()
                * crate::palette::gallery_brand_presets().len()
                * surface_names().len()
                * 4
        );
    }

    #[test]
    fn unsupported_range_semantics_reject_without_a_partial_result() {
        let p = Palette::new(Theme::LIGHT, 0).unwrap();
        for state in [
            "selected",
            "selected-hover",
            "selected-pressed",
            "selected-disabled",
            "focus",
            "error",
            "readonly",
        ] {
            assert_eq!(
                resolve_range_surface(p, "canvas", state),
                Err(SurfaceError::UnknownCombination)
            );
        }
        assert_eq!(
            resolve_range_surface(p, "unknown", "rest"),
            Err(SurfaceError::UnknownCombination)
        );
        for options in [
            SurfaceOptions {
                error: true,
                read_only: false,
            },
            SurfaceOptions {
                error: false,
                read_only: true,
            },
        ] {
            assert_eq!(
                resolve_surface_with_options(p, "canvas", "range-track", "rest", options),
                Err(SurfaceError::UnknownCombination)
            );
        }
    }
}

#[cfg(test)]
mod toggle_tests {
    use super::*;
    use crate::Theme;

    #[test]
    fn complete_original_toggle_anatomy_in_all_modes_brands_and_surfaces() {
        let states = [
            "rest",
            "hover",
            "pressed",
            "disabled",
            "selected",
            "selected-hover",
            "selected-pressed",
            "selected-disabled",
        ];
        let mut pairs = 0;
        for mode in Theme::ALL {
            for preset in crate::palette::gallery_brand_presets() {
                let palette = Palette::new(mode, preset.palette_index).unwrap();
                for surface in surface_names() {
                    for state in states {
                        for error in [false, true] {
                            let c = resolve_toggle_surface(
                                palette,
                                surface,
                                state,
                                SurfaceOptions {
                                    error,
                                    read_only: false,
                                },
                            )
                            .unwrap();
                            let disabled = matches!(state, "disabled" | "selected-disabled");
                            let on = state.starts_with("selected");
                            let track = if disabled {
                                "surface-2"
                            } else if on && state.ends_with("pressed") {
                                "control-checked-pressed"
                            } else if on && state.ends_with("hover") {
                                "control-checked-hover"
                            } else if on {
                                "c-switch-on-ground"
                            } else if state == "pressed" {
                                "button-pressed"
                            } else if state == "hover" {
                                "button-hover"
                            } else {
                                "surface-3"
                            };
                            assert_eq!(c.track.background, palette.color(track).unwrap());
                            assert_eq!(
                                c.track.boundary,
                                palette.color(if error {
                                    "status-failed"
                                } else if disabled {
                                    "disabled-text"
                                } else {
                                    "border"
                                })
                            );
                            assert_eq!(
                                c.thumb.background,
                                palette
                                    .color(if disabled {
                                        "disabled-text"
                                    } else if on {
                                        "c-switch-on-thumb"
                                    } else {
                                        "surface-0"
                                    })
                                    .unwrap()
                            );
                            assert_eq!(
                                c.thumb.boundary,
                                palette.color(if disabled { "disabled-text" } else { "border" })
                            );
                            assert_eq!(c.thumb.focus, None);
                            assert_eq!(
                                c.track.focus,
                                if disabled {
                                    None
                                } else {
                                    palette.color("border-focused")
                                }
                            );
                            assert_eq!(
                                c.label.text,
                                palette
                                    .color(if disabled {
                                        "tertiary-text"
                                    } else {
                                        "primary-text"
                                    })
                                    .unwrap()
                            );
                            assert_eq!(c.label.icon, c.label.text);
                            assert_eq!(
                                c.label.background,
                                resolve_surface(palette, surface, "content", "rest")
                                    .unwrap()
                                    .background
                            );
                            assert_eq!(c.label.boundary, None);
                            assert_eq!(c.label.focus, None);
                            pairs += 1;
                        }
                    }
                }
            }
        }
        assert_eq!(
            pairs,
            Theme::ALL.len()
                * crate::palette::gallery_brand_presets().len()
                * surface_names().len()
                * states.len()
                * 2
        );
    }

    #[test]
    fn unknown_or_inapplicable_toggle_requests_reject_as_a_complete_result() {
        let palette = Palette::new(Theme::LIGHT, 0).unwrap();
        for (surface, state, read_only) in [
            ("unknown", "rest", false),
            ("canvas", "focus", false),
            ("canvas", "rest", true),
        ] {
            assert_eq!(
                resolve_toggle_surface(
                    palette,
                    surface,
                    state,
                    SurfaceOptions {
                        error: false,
                        read_only
                    }
                ),
                Err(SurfaceError::UnknownCombination)
            );
        }
        assert_eq!(
            resolve_surface_with_options(
                palette,
                "canvas",
                "toggle-thumb",
                "rest",
                SurfaceOptions {
                    error: true,
                    read_only: false
                }
            ),
            Err(SurfaceError::UnknownCombination)
        );
    }
}

#[cfg(test)]
mod frame_tests {
    use super::*;
    #[test]
    fn framed_and_flat_anatomy_share_declared_ground_and_content_ink() {
        for mode in crate::Theme::ALL {
            for preset in crate::palette::gallery_brand_presets() {
                let p = Palette::new(mode, preset.palette_index).unwrap();
                for surface in surface_names() {
                    for state in ["rest", "disabled"] {
                        let mut expected = resolve_surface(p, surface, "content", state).unwrap();
                        assert_eq!(expected.boundary, None);
                        expected.boundary = p.color("separator");
                        assert_eq!(
                            resolve_surface(p, surface, "surface-frame", state).unwrap(),
                            expected
                        );
                    }
                    for state in ["hover", "pressed", "selected", "focus"] {
                        assert_eq!(
                            resolve_surface(p, surface, "surface-frame", state),
                            Err(SurfaceError::UnknownCombination)
                        );
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod status_content_tests {
    use super::*;
    #[test]
    fn passive_status_packets_are_surface_bound_and_reject_action_states() {
        let mut count = 0;
        for mode in crate::Theme::ALL {
            for preset in crate::palette::gallery_brand_presets() {
                let p = Palette::new(mode, preset.palette_index).unwrap();
                for surface in surface_names() {
                    for tone in ["info", "running", "succeeded", "failed"] {
                        let usage = format!("content-status-{tone}");
                        for state in ["rest", "disabled"] {
                            let mut expected =
                                resolve_surface(p, surface, "content", state).unwrap();
                            if state == "rest" {
                                let ink = p.color(&format!("status-{tone}")).unwrap();
                                expected.text = ink;
                                expected.icon = ink;
                            }
                            assert_eq!(
                                resolve_surface(p, surface, &usage, state).unwrap(),
                                expected
                            );
                            count += 1;
                        }
                        for state in ["hover", "pressed", "selected", "focus", "selected-disabled"]
                        {
                            assert_eq!(
                                resolve_surface(p, surface, &usage, state),
                                Err(SurfaceError::UnknownCombination)
                            );
                        }
                    }
                    assert_eq!(
                        resolve_surface(p, surface, "content-status-unknown", "rest"),
                        Err(SurfaceError::UnknownCombination)
                    );
                }
            }
        }
        assert_eq!(count, 13 * 4 * surface_names().len() * 4 * 2);
    }
}

#[cfg(test)]
mod passive_status_tests {
    use super::*;
    use crate::Theme;
    #[test]
    fn passive_status_complete_packets_and_rest_only() {
        for mode in Theme::ALL {
            for p in crate::palette::gallery_brand_presets()
                .iter()
                .map(|preset| Palette::new(mode, preset.palette_index).unwrap())
                .chain(
                    ["default", "blue", "clay", "blue-inputs"]
                        .into_iter()
                        .map(|id| Palette::for_theme_definition(mode, id).unwrap()),
                )
            {
                for surface in surface_names() {
                    for tone in ["info", "running", "succeeded", "failed"] {
                        let usage = format!("status-badge-{tone}");
                        let c = resolve_surface(p, surface, &usage, "rest").unwrap();
                        assert_eq!(c.background, p.color(&format!("status-{tone}")).unwrap());
                        assert_eq!(c.text, p.color("c-status-badge-foreground").unwrap());
                        assert_eq!(c.icon, c.text);
                        assert_eq!(
                            (c.boundary, c.focus, c.marker, c.helper),
                            (None, None, None, None)
                        );
                        for state in ["hover", "pressed", "disabled", "selected", "focus"] {
                            assert_eq!(
                                resolve_surface(p, surface, &usage, state),
                                Err(SurfaceError::UnknownCombination)
                            );
                        }
                    }
                    let c = resolve_surface(p, surface, "skeleton", "rest").unwrap();
                    assert_eq!(c.background, p.color("surface-2").unwrap());
                    assert_eq!(c.text, p.color("primary-text").unwrap());
                    assert_eq!(c.icon, c.text);
                    assert_eq!(c.boundary, p.color("border"));
                    assert_eq!((c.focus, c.marker, c.helper), (None, None, None));
                    for state in ["hover", "pressed", "disabled", "selected", "focus"] {
                        assert_eq!(
                            resolve_surface(p, surface, "skeleton", state),
                            Err(SurfaceError::UnknownCombination)
                        );
                    }
                }
                for surface in ["unknown", "Canvas", ""] {
                    assert_eq!(
                        resolve_surface(p, surface, "skeleton", "rest"),
                        Err(SurfaceError::UnknownCombination)
                    );
                }
            }
        }
    }
}
