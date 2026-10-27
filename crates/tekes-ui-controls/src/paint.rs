//! Shared token-driven control geometry; clients supply text shaping via their adapter.
use crate::{CheckState, ControlState};
use tekes_ui_scene::{Point, Quad, Rect, Scene, ShadowQuad};
use tekes_ui_tokens::{Palette, Rgba, proposal};

/// Semantic surface roles mapped directly to the JSON Design System tokens.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SurfaceKind {
    Canvas,
    Panel,
    Floating,
    Dialog,
    Code,
}

pub fn color(theme: impl Into<Palette>, role: &str) -> Rgba {
    theme.into().color(role).expect("semantic control role")
}
/// Flat window-chrome ground, shared by Toolbar consumers and split seams.
/// Reuse the existing window-base semantic; no panel boundary or elevation.
/// The allocation, natural row and ancestor clip all bound this paint.
pub(crate) fn toolbar_background(
    scene: &mut Scene,
    allocated: Rect,
    clip: Option<Rect>,
    row_height: f32,
    palette: Palette,
) {
    let row = Rect::new(
        allocated.min_x(),
        allocated.min_y(),
        allocated.size.width,
        allocated.size.height.min(row_height),
    );
    let visible = clip.map_or(Some(row), |clip| row.intersection(&clip));
    if let Some(rect) = visible.filter(|r| r.size.width > 0. && r.size.height > 0.) {
        scene.push_quad(Quad::new(rect, color(palette, "surface-0")));
    }
}

pub fn surface(scene: &mut Scene, rect: Rect, theme: impl Into<Palette>, panel: bool) {
    surface_at_scale(scene, rect, theme, panel, 1.0);
}

/// Paints a Surface/Panel at the renderer's backing scale. Decorative panel
/// boundaries follow the JSON hairline policy; the legacy `surface` API uses
/// a 1x backing scale for source compatibility.
pub fn surface_at_scale(
    scene: &mut Scene,
    rect: Rect,
    theme: impl Into<Palette>,
    panel: bool,
    backing_scale: f32,
) {
    surface_kind_at_scale(
        scene,
        rect,
        theme,
        if panel {
            SurfaceKind::Panel
        } else {
            SurfaceKind::Canvas
        },
        backing_scale,
    );
}

/// Paint a semantic surface role using existing color, radius and boundary
/// tokens. All non-canvas roles retain a boundary; its width follows the
/// decorative hairline policy at the host backing scale.
pub fn surface_kind_at_scale(
    scene: &mut Scene,
    rect: Rect,
    theme: impl Into<Palette>,
    kind: SurfaceKind,
    backing_scale: f32,
) {
    let elevation_role = match kind {
        SurfaceKind::Floating => "popover",
        SurfaceKind::Dialog => "dialog",
        _ => "panel",
    };
    surface_kind_with_elevation_at_scale(
        scene,
        rect,
        theme.into(),
        kind,
        backing_scale,
        elevation_role,
    );
}

pub(crate) fn surface_kind_with_elevation_at_scale(
    scene: &mut Scene,
    rect: Rect,
    theme: Palette,
    kind: SurfaceKind,
    backing_scale: f32,
    elevation_role: &str,
) {
    let (fill, radius, boundary) = match kind {
        SurfaceKind::Canvas => ("surface-1", "radius-panel", false),
        SurfaceKind::Panel => ("surface-panel", "radius-panel", true),
        SurfaceKind::Floating => ("surface-popover", "radius-floating", true),
        SurfaceKind::Dialog => ("surface-popover", "c-dialog-radius", true),
        SurfaceKind::Code => ("code-surface", "c-code-radius", true),
    };
    let radius = proposal::metric("radius", radius).expect("surface radius");
    // JSON layers are front-to-back; painter order is back-to-front. Shadows
    // retain the caller's ancestor clip, never the surface's own bounds.
    for layer in proposal::elevation(elevation_role, theme.theme)
        .iter()
        .rev()
    {
        let mut ink = color(theme, layer.color);
        ink.a *= layer.alpha;
        scene.push_shadow(
            ShadowQuad::new(rect, ink)
                .with_corner_radius(radius)
                .with_offset(Point::new(layer.offset_x, layer.offset_y))
                .with_blur_radius(layer.blur_radius)
                .with_outset(layer.spread),
        );
    }
    let mut q = Quad::new(rect, color(theme, fill)).with_corner_radius(radius);
    if boundary {
        let (width, ink) = decorative_border(theme, backing_scale);
        q = q.with_border(width, ink);
    }
    scene.push_quad(q);
}
#[allow(clippy::too_many_arguments)]
pub(crate) fn surface_kind_with_elevation_colors_at_scale(
    scene: &mut Scene,
    rect: Rect,
    theme: Palette,
    kind: SurfaceKind,
    backing_scale: f32,
    elevation_role: &str,
    colors: tekes_ui_tokens::surface_palette::SurfaceColors,
) {
    let (radius, framed) = match kind {
        SurfaceKind::Canvas => ("radius-panel", false),
        SurfaceKind::Panel => ("radius-panel", true),
        SurfaceKind::Floating => ("radius-floating", true),
        SurfaceKind::Dialog => ("c-dialog-radius", true),
        SurfaceKind::Code => ("c-code-radius", true),
    };
    let radius = proposal::metric("radius", radius).expect("surface radius");
    // JSON layers are front-to-back; painter order is back-to-front. Shadows
    // retain the caller's ancestor clip, never the surface's own bounds.
    for layer in proposal::elevation(elevation_role, theme.theme)
        .iter()
        .rev()
    {
        let mut ink = color(theme, layer.color);
        ink.a *= layer.alpha;
        scene.push_shadow(
            ShadowQuad::new(rect, ink)
                .with_corner_radius(radius)
                .with_offset(Point::new(layer.offset_x, layer.offset_y))
                .with_blur_radius(layer.blur_radius)
                .with_outset(layer.spread),
        );
    }
    let mut q = Quad::new(rect, colors.background).with_corner_radius(radius);
    if framed && let Some(ink) = colors.boundary {
        q = q.with_border(decorative_border(theme, backing_scale).0, ink);
    }
    scene.push_quad(q);
}
// Panel's original paint order places children between the semantic Surface
// fill/elevation and the Surface boundary/focus decoration.
#[derive(Clone, Copy)]
enum PanelSurfacePhase {
    Background,
    Boundary,
}
fn panel_surface_parts(
    scene: &mut Scene,
    rect: Rect,
    palette: Palette,
    surface: &str,
    canvas: bool,
    backing_scale: f32,
    phase: PanelSurfacePhase,
) -> Result<(), tekes_ui_tokens::surface_palette::SurfaceError> {
    let (fill, border, shadows) = match phase {
        PanelSurfacePhase::Background => (true, false, true),
        PanelSurfacePhase::Boundary => (false, true, false),
    };
    let framed = !canvas;
    let colors = tekes_ui_tokens::surface_palette::resolve_surface(
        palette,
        surface,
        if framed { "surface-frame" } else { "content" },
        "rest",
    )?;
    let radius = proposal::metric("radius", "radius-panel").expect("Surface radius");
    for layer in proposal::elevation("panel", palette.theme)
        .iter()
        .rev()
        .filter(|_| shadows)
    {
        let mut ink = color(palette, layer.color);
        ink.a *= layer.alpha;
        scene.push_shadow(
            ShadowQuad::new(rect, ink)
                .with_corner_radius(radius)
                .with_offset(Point::new(layer.offset_x, layer.offset_y))
                .with_blur_radius(layer.blur_radius)
                .with_outset(layer.spread),
        );
    }
    let mut quad = Quad::new(
        rect,
        if fill {
            colors.background
        } else {
            Rgba::new(0., 0., 0., 0.)
        },
    )
    .with_corner_radius(radius);
    if border
        && framed
        && let Some(ink) = colors.boundary
    {
        let (width, _) = decorative_border(palette, backing_scale);
        quad = quad.with_border(width, ink);
    }
    scene.push_quad(quad);
    Ok(())
}

pub(crate) fn panel_background(
    scene: &mut Scene,
    rect: Rect,
    palette: Palette,
    surface: &str,
    canvas: bool,
    backing_scale: f32,
) {
    panel_surface_parts(
        scene,
        rect,
        palette,
        surface,
        canvas,
        backing_scale,
        PanelSurfacePhase::Background,
    )
    .expect("validated Panel Surface");
}

pub(crate) fn panel_boundary(
    scene: &mut Scene,
    rect: Rect,
    palette: Palette,
    surface: &str,
    canvas: bool,
    backing_scale: f32,
    focused: bool,
) {
    if !canvas {
        panel_surface_parts(
            scene,
            rect,
            palette,
            surface,
            canvas,
            backing_scale,
            PanelSurfacePhase::Boundary,
        )
        .expect("validated Panel Surface");
    }
    if focused {
        let radius = proposal::metric("radius", "radius-panel").expect("Panel radius");
        focus_ring(scene, rect, palette, radius);
    }
}
/// Decorative frame, following layout.json hairline policy. Never use this
/// for a necessary control boundary or a focus/selection indicator.
pub fn decorative_border(theme: impl Into<Palette>, backing_scale: f32) -> (f32, Rgba) {
    let theme = theme.into();
    let width = if theme.theme.contrast == tekes_ui_tokens::Contrast::Increased {
        proposal::metric("stroke", "stroke-rule").expect("contrast stroke")
    } else {
        1.0 / if backing_scale.is_finite() && backing_scale > 0.0 {
            backing_scale
        } else {
            1.0
        }
    };
    (width, color(theme, "separator"))
}
/// Independent keyboard/editing focus ring. The quad shader draws borders
/// inward, so expand by offset + width to retain the full visible gap. Hosts
/// keep their ancestor viewport clip; this never changes layout or hit targets.
pub fn focus_ring(scene: &mut Scene, rect: Rect, theme: impl Into<Palette>, radius: f32) {
    focus_ring_with_color(scene, rect, color(theme, "border-focused"), radius);
}

pub(crate) fn focus_ring_with_color(scene: &mut Scene, rect: Rect, ink: Rgba, radius: f32) {
    let metrics = crate::extended::FocusRingMetrics::new(1.0);
    let outset = metrics.outset();
    scene.push_quad(
        Quad::new(rect.inset(-outset), Rgba::new(0., 0., 0., 0.))
            .with_corner_radius(radius + outset)
            .with_border(metrics.width, ink),
    );
}

pub fn button(
    scene: &mut Scene,
    rect: Rect,
    theme: impl Into<Palette>,
    state: ControlState,
) -> Rgba {
    button_with_radius(
        scene,
        rect,
        theme.into(),
        state,
        proposal::metric("radius", "c-button-md-radius").expect("button radius"),
    )
}
fn button_with_radius(
    scene: &mut Scene,
    rect: Rect,
    theme: Palette,
    state: ControlState,
    radius: f32,
) -> Rgba {
    button_variant_with_interaction(
        scene,
        rect,
        theme,
        state,
        ButtonVariant::Quiet,
        radius,
        if state.hovered { 1. } else { 0. },
        if state.pressed { 1. } else { 0. },
    )
}

pub fn checkbox(
    scene: &mut Scene,
    rect: Rect,
    theme: impl Into<Palette>,
    state: ControlState,
    checked: CheckState,
) -> Rgba {
    let theme = theme.into();
    let checked_ink = checked != CheckState::Unchecked && !state.disabled;
    let fill = if state.disabled {
        "surface-2"
    } else if checked_ink {
        if state.pressed {
            "control-checked-pressed"
        } else if state.hovered {
            "control-checked-hover"
        } else {
            "control-checked-ground"
        }
    } else if state.selected {
        if state.hovered {
            "row-selection-hover"
        } else {
            "row-selection"
        }
    } else if state.pressed {
        "button-pressed"
    } else if state.hovered {
        "button-hover"
    } else {
        "surface-2"
    };
    let border = if state.error {
        "status-failed"
    } else {
        "border"
    };
    let radius = proposal::metric("radius", "c-checkbox-radius").expect("checkbox radius");
    let ink = color(
        theme,
        if checked_ink {
            "control-checked-mark"
        } else if state.disabled {
            "tertiary-text"
        } else {
            "primary-text"
        },
    );
    scene.push_quad(
        Quad::new(rect, color(theme, fill))
            .with_corner_radius(radius)
            .with_border(
                proposal::metric("stroke", "stroke-rule").expect("checkbox stroke"),
                color(theme, border),
            ),
    );
    if checked == CheckState::Mixed {
        scene.push_quad(
            Quad::new(
                Rect::new(
                    rect.min_x() + 4.,
                    rect.center().y - 1.,
                    rect.size.width - 8.,
                    2.,
                ),
                ink,
            )
            .with_corner_radius(1.),
        );
    } else if checked == CheckState::Checked {
        let stroke = proposal::metric("size", "size-icon-artboard-stroke").expect("icon stroke");
        for (a, b) in [((0.20, 0.50), (0.42, 0.72)), ((0.42, 0.72), (0.80, 0.28))] {
            let ax = rect.min_x() + rect.size.width * a.0;
            let ay = rect.min_y() + rect.size.height * a.1;
            let dx = rect.size.width * (b.0 - a.0);
            let dy = rect.size.height * (b.1 - a.1);
            let steps = dx.abs().max(dy.abs()).ceil().max(1.) as usize;
            for n in 0..=steps {
                let t = n as f32 / steps as f32;
                scene.push_quad(
                    Quad::new(Rect::new(ax + dx * t, ay + dy * t, stroke, stroke), ink)
                        .with_corner_radius(stroke / 2.),
                );
            }
        }
    }

    if state.focused && state.focus_visible && !state.disabled {
        focus_ring(scene, rect, theme, radius);
    }
    ink
}
pub fn skeleton(scene: &mut Scene, rect: Rect, theme: impl Into<Palette>) {
    skeleton_on_surface(scene, rect, theme.into(), "canvas")
        .expect("generated Skeleton canvas packet");
}
/// Resolve the complete shared packet before changing the scene. Geometry and
/// surface-2 fill are the original Skeleton; its necessary border stays inside
/// the same frame. No intrinsic extent, animation phase or state is introduced.
pub fn skeleton_on_surface(
    scene: &mut Scene,
    rect: Rect,
    palette: Palette,
    surface: &str,
) -> Result<(), tekes_ui_tokens::surface_palette::SurfaceError> {
    use tekes_ui_tokens::surface_palette::{SurfaceError, resolve_surface};
    let colors = resolve_surface(palette, surface, "skeleton", "rest")?;
    let boundary = colors.boundary.ok_or(SurfaceError::UnknownRole)?;
    let stroke = proposal::metric("stroke", "stroke-rule").expect("Skeleton boundary stroke");
    scene.push_quad(Quad::new(rect, colors.background).with_corner_radius(4.));
    scene.push_quad(
        Quad::new(rect, Rgba::new(0., 0., 0., 0.))
            .with_corner_radius(4.)
            .with_border(stroke, boundary),
    );
    Ok(())
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonVariant {
    #[default]
    Quiet,
    Primary,
    /// Quiet action whose resting surface inherits its parent panel.
    Panel,
    /// Quiet action on the existing window toolbar Surface.
    Toolbar,
}
pub fn button_variant(
    scene: &mut Scene,
    rect: Rect,
    theme: impl Into<Palette>,
    state: ControlState,
    variant: ButtonVariant,
) -> Rgba {
    button_variant_with_radius(
        scene,
        rect,
        theme.into(),
        state,
        variant,
        proposal::metric("radius", "c-button-md-radius").expect("button radius"),
    )
}
/// Size-aware production Button paint; legacy painters retain their md radius.
pub fn button_variant_with_radius(
    scene: &mut Scene,
    rect: Rect,
    theme: Palette,
    state: ControlState,
    variant: ButtonVariant,
    radius: f32,
) -> Rgba {
    button_variant_with_interaction(
        scene,
        rect,
        theme,
        state,
        variant,
        radius,
        if state.hovered { 1. } else { 0. },
        if state.pressed { 1. } else { 0. },
    )
}

/// Component-owned interaction progress changes only presentation colors;
/// layout, semantic state, hit testing and independent focus remain immediate.
#[allow(clippy::too_many_arguments)]
pub fn button_variant_with_interaction(
    scene: &mut Scene,
    rect: Rect,
    theme: Palette,
    state: ControlState,
    variant: ButtonVariant,
    radius: f32,
    hover_progress: f32,
    pressed_progress: f32,
) -> Rgba {
    let primary = variant == ButtonVariant::Primary && !state.disabled;
    let (rest, hover, pressed) = if state.disabled {
        ("surface-2", "surface-2", "surface-2")
    } else if primary {
        (
            "action-primary-ground",
            "action-primary-hover",
            "action-primary-pressed",
        )
    } else if state.selected {
        (
            "row-selection",
            "row-selection-hover",
            if variant == ButtonVariant::Panel {
                "button-pressed"
            } else {
                "row-selection"
            },
        )
    } else {
        (
            if variant == ButtonVariant::Panel {
                "surface-panel"
            } else {
                "surface-2"
            },
            "button-hover",
            "button-pressed",
        )
    };
    let progress = |value: f32| {
        if value.is_finite() {
            value.clamp(0., 1.)
        } else {
            0.
        }
    };
    let mix = |a: Rgba, b: Rgba, t: f32| {
        if t <= 0. {
            return a;
        }
        if t >= 1. {
            return b;
        }
        Rgba::new(
            a.r + (b.r - a.r) * t,
            a.g + (b.g - a.g) * t,
            a.b + (b.b - a.b) * t,
            a.a + (b.a - a.a) * t,
        )
    };
    let pressed_progress = if state.selected && variant == ButtonVariant::Quiet {
        0.
    } else {
        pressed_progress
    };
    let fill = mix(
        mix(
            color(theme, rest),
            color(theme, hover),
            progress(hover_progress),
        ),
        color(theme, pressed),
        progress(pressed_progress),
    );
    let role = if state.error && !state.disabled {
        "error"
    } else if state.selected && !state.disabled && !primary {
        "selected"
    } else {
        match variant {
            ButtonVariant::Primary if primary => "action-primary",
            ButtonVariant::Panel => "action-panel",
            _ => "action-quiet",
        }
    };
    let boundary = proposal::control_boundary(role, theme.theme).expect("JSON control emphasis");
    let quad = Quad::new(rect, fill).with_corner_radius(radius);
    let quad = match boundary {
        "none" => quad,
        "fill" => quad.with_border(
            proposal::metric("stroke", "stroke-rule").expect("control stroke"),
            fill,
        ),
        color_role => quad.with_border(
            proposal::metric("stroke", "stroke-rule").expect("control stroke"),
            color(theme, color_role),
        ),
    };
    scene.push_quad(quad);
    if state.focused && state.focus_visible && !state.disabled {
        focus_ring(scene, rect, theme, radius);
    }
    color(
        theme,
        if state.disabled {
            "tertiary-text"
        } else if primary {
            "action-primary-foreground"
        } else {
            "primary-text"
        },
    )
}

// Surface override for the original EmptyState action; legacy callers retain
// their existing palette path unless their Button owner supplies a Surface.
pub(crate) fn button_surface_colors(
    palette: Palette,
    surface: &str,
    state: ControlState,
    variant: ButtonVariant,
    hover: f32,
    pressed: f32,
) -> tekes_ui_tokens::surface_palette::SurfaceColors {
    use tekes_ui_tokens::surface_palette::{SurfaceOptions, resolve_surface_with_options};
    let usage = if variant == ButtonVariant::Primary {
        "action-primary"
    } else {
        "action-quiet"
    };
    let options = SurfaceOptions {
        error: state.error,
        read_only: false,
    };
    let endpoint = |suffix| {
        let name = match (state.selected, suffix) {
            (true, "rest") => "selected",
            (true, "hover") => "selected-hover",
            (true, "pressed") => "selected-pressed",
            (true, "disabled") => "selected-disabled",
            (_, name) => name,
        };
        resolve_surface_with_options(palette, surface, usage, name, options)
            .expect("validated original Button Surface/use/state")
    };
    if state.disabled {
        return endpoint("disabled");
    }
    mix_button_colors(
        mix_button_colors(endpoint("rest"), endpoint("hover"), hover),
        endpoint("pressed"),
        pressed,
    )
}

fn mix_button_colors(
    a: tekes_ui_tokens::surface_palette::SurfaceColors,
    b: tekes_ui_tokens::surface_palette::SurfaceColors,
    progress: f32,
) -> tekes_ui_tokens::surface_palette::SurfaceColors {
    use tekes_ui_tokens::surface_palette::SurfaceColors;
    let t = if progress.is_finite() {
        progress.clamp(0., 1.)
    } else {
        0.
    };
    if t == 0. {
        return a;
    }
    if t == 1. {
        return b;
    }
    let mix = |a: Rgba, b: Rgba| {
        Rgba::new(
            a.r + (b.r - a.r) * t,
            a.g + (b.g - a.g) * t,
            a.b + (b.b - a.b) * t,
            a.a + (b.a - a.a) * t,
        )
    };
    let optional = |a: Option<Rgba>, b: Option<Rgba>| match (a, b) {
        (None, None) => None,
        (Some(a), Some(b)) => Some(mix(a, b)),
        (Some(a), None) => Some(mix(a, Rgba::new(a.r, a.g, a.b, 0.))),
        (None, Some(b)) => Some(mix(Rgba::new(b.r, b.g, b.b, 0.), b)),
    };
    SurfaceColors {
        background: mix(a.background, b.background),
        text: mix(a.text, b.text),
        icon: mix(a.icon, b.icon),
        boundary: optional(a.boundary, b.boundary),
        focus: optional(a.focus, b.focus),
        marker: optional(a.marker, b.marker),
        helper: optional(a.helper, b.helper),
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn button_variant_on_surface(
    scene: &mut Scene,
    rect: Rect,
    palette: Palette,
    state: ControlState,
    variant: ButtonVariant,
    radius: f32,
    hover: f32,
    pressed: f32,
    surface: &str,
) -> tekes_ui_tokens::surface_palette::SurfaceColors {
    let colors = button_surface_colors(palette, surface, state, variant, hover, pressed);
    let mut quad = Quad::new(rect, colors.background).with_corner_radius(radius);
    if let Some(boundary) = colors.boundary {
        quad = quad.with_border(
            proposal::metric("stroke", "stroke-rule").expect("control stroke"),
            boundary,
        );
    }
    scene.push_quad(quad);
    if state.focused
        && state.focus_visible
        && !state.disabled
        && let Some(focus) = colors.focus
    {
        focus_ring_with_color(scene, rect, focus, radius);
    }
    colors
}

#[cfg(test)]
mod boundary_tests {
    use super::*;
    use tekes_ui_tokens::{Contrast, Theme};
    #[test]
    fn decorative_frames_and_optional_actions_have_distinct_boundary_roles() {
        for theme in Theme::ALL {
            for scale in [1., 2.] {
                let (width, ink) = decorative_border(theme, scale);
                assert_eq!(
                    width,
                    if theme.contrast == Contrast::Increased {
                        1.
                    } else {
                        1. / scale
                    }
                );
                assert_eq!(ink, color(theme, "separator"));
                let mut scene = Scene::new();
                button(
                    &mut scene,
                    Rect::new(0., 0., 100., 32.),
                    theme,
                    ControlState::default(),
                );
                let tekes_ui_scene::Primitive::Quad(q) = &scene.primitives()[0] else {
                    panic!("control quad")
                };
                assert_eq!(q.border_width, 0.);
            }
        }
    }

    #[test]
    fn checkbox_base_and_marked_states_share_the_component_square_radius() {
        let frame = Rect::new(4.0, 6.0, 16.0, 16.0);
        let states = [
            (ControlState::default(), CheckState::Unchecked),
            (ControlState::default(), CheckState::Checked),
            (
                ControlState {
                    hovered: true,
                    pressed: true,
                    ..Default::default()
                },
                CheckState::Mixed,
            ),
            (
                ControlState {
                    disabled: true,
                    ..Default::default()
                },
                CheckState::Unchecked,
            ),
            (
                ControlState {
                    disabled: true,
                    focused: true,
                    focus_visible: true,
                    ..Default::default()
                },
                CheckState::Checked,
            ),
        ];
        let radius = proposal::metric("radius", "c-checkbox-radius").unwrap();
        assert_eq!(radius, 4.0);
        assert!(radius < frame.size.width.min(frame.size.height) * 0.5);

        for theme in Theme::ALL {
            for (state, checked) in states {
                let mut scene = Scene::new();
                checkbox(&mut scene, frame, theme, state, checked);
                let tekes_ui_scene::Primitive::Quad(outline) = &scene.primitives()[0] else {
                    panic!("checkbox outline")
                };
                assert_eq!(outline.rect, frame);
                assert_eq!(
                    outline.corner_radii,
                    tekes_ui_scene::CornerRadii::uniform(radius),
                    "outer shape must not change for {checked:?} / {state:?} / {theme:?}"
                );
            }
        }
    }

    #[test]
    fn panel_surface_uses_panel_token_and_scale_aware_decorative_boundary() {
        for theme in Theme::ALL {
            for scale in [1.0, 2.0] {
                let panel_rect = Rect::new(2.0, 3.0, 120.0, 64.0);
                let mut panel_scene = Scene::new();
                surface_at_scale(&mut panel_scene, panel_rect, theme, true, scale);
                let tekes_ui_scene::Primitive::Quad(panel) = &panel_scene.primitives()[0] else {
                    panic!("panel surface quad")
                };
                let (width, ink) = decorative_border(theme, scale);
                assert_eq!(panel.rect, panel_rect);
                assert_eq!(panel.fill, color(theme, "surface-panel"));
                assert_eq!(panel.border_color, ink);
                assert_eq!(panel.border_width, width);

                let mut surface_scene = Scene::new();
                surface_at_scale(&mut surface_scene, panel_rect, theme, false, scale);
                let tekes_ui_scene::Primitive::Quad(surface) = &surface_scene.primitives()[0]
                else {
                    panic!("surface quad")
                };
                assert_eq!(surface.rect, panel_rect);
                assert_eq!(surface.fill, color(theme, "surface-1"));
                assert_eq!(surface.border_width, 0.0);
            }
        }
    }

    #[test]
    fn semantic_surface_roles_use_existing_fill_radius_and_hairline_tokens() {
        let cases = [
            (SurfaceKind::Canvas, "surface-1", "radius-panel", false),
            (SurfaceKind::Panel, "surface-panel", "radius-panel", true),
            (
                SurfaceKind::Floating,
                "surface-popover",
                "radius-floating",
                true,
            ),
            (
                SurfaceKind::Dialog,
                "surface-popover",
                "c-dialog-radius",
                true,
            ),
            (SurfaceKind::Code, "code-surface", "c-code-radius", true),
        ];
        for theme in Theme::ALL {
            for scale in [1.0, 2.0] {
                for (kind, fill, radius, bordered) in cases {
                    let frame = Rect::new(10.0, 20.0, 180.0, 90.0);
                    let mut scene = Scene::new();
                    surface_kind_at_scale(&mut scene, frame, theme, kind, scale);
                    let tekes_ui_scene::Primitive::Quad(q) = scene.primitives().last().unwrap()
                    else {
                        panic!("semantic surface quad")
                    };
                    assert_eq!(q.rect, frame);
                    assert_eq!(q.fill, color(theme, fill));
                    assert_eq!(
                        q.corner_radii,
                        tekes_ui_scene::CornerRadii::uniform(
                            proposal::metric("radius", radius).expect("radius token"),
                        )
                    );
                    if bordered {
                        assert_eq!(q.border_color, color(theme, "separator"));
                        assert_eq!(q.border_width, decorative_border(theme, scale).0);
                    } else {
                        assert_eq!(q.border_width, 0.0);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod focus_tests {
    use super::*;
    use tekes_ui_scene::Primitive;
    use tekes_ui_tokens::Theme;

    #[test]
    fn focus_is_an_independent_two_point_ring_with_two_point_visible_gap() {
        for theme in Theme::ALL {
            for brand in [0, 1, 2] {
                for scale in [1.0, 2.0] {
                    let palette = Palette::new(theme, brand).expect("registered Gallery brand");
                    let frame = Rect::new(20.25, 22.5, 120., 36.);
                    let mut scene = Scene::new();
                    button(
                        &mut scene,
                        frame,
                        palette,
                        ControlState {
                            focused: true,
                            focus_visible: true,
                            error: true,
                            ..Default::default()
                        },
                    );
                    let quads = scene
                        .primitives()
                        .iter()
                        .filter_map(|p| match p {
                            Primitive::Quad(q) => Some(q),
                            _ => None,
                        })
                        .collect::<Vec<_>>();
                    assert_eq!(quads.len(), 2);
                    assert_eq!(quads[0].rect, frame);
                    assert_eq!(quads[0].border_color, color(palette, "status-failed"));
                    let ring = quads[1];
                    assert_eq!(ring.fill.a, 0.);
                    assert_eq!(ring.rect, frame.inset(-4.));
                    assert_eq!(ring.border_width * scale, 2. * scale);
                    assert_eq!(frame.min_x() - (ring.rect.min_x() + ring.border_width), 2.);
                    assert_eq!(ring.border_color, color(palette, "border-focused"));
                }
            }
        }
    }

    #[test]
    fn action_emphasis_preserves_primary_error_selection_and_independent_focus_in_all_themes() {
        for theme in Theme::ALL {
            for variant in [
                ButtonVariant::Quiet,
                ButtonVariant::Panel,
                ButtonVariant::Primary,
            ] {
                for (selected, error, focused) in [
                    (false, false, false),
                    (false, false, true),
                    (true, false, false),
                    (false, true, true),
                ] {
                    let mut scene = Scene::new();
                    button_variant(
                        &mut scene,
                        Rect::new(10., 10., 100., 32.),
                        theme,
                        ControlState {
                            selected,
                            error,
                            focused,
                            focus_visible: focused,
                            ..Default::default()
                        },
                        variant,
                    );
                    let tekes_ui_scene::Primitive::Quad(q) = &scene.primitives()[0] else {
                        panic!("action")
                    };
                    if error {
                        assert_eq!(q.border_color, color(theme, "status-failed"));
                        assert_eq!(
                            q.border_width,
                            proposal::metric("stroke", "stroke-rule").unwrap()
                        );
                    } else if variant == ButtonVariant::Primary {
                        assert_eq!(q.border_color, q.fill);
                    } else if selected {
                        assert_eq!(q.border_color, color(theme, "border"));
                    } else {
                        assert_eq!(q.border_width, 0.);
                    }
                    assert_eq!(scene.primitives().len(), if focused { 2 } else { 1 });
                    if focused {
                        let tekes_ui_scene::Primitive::Quad(ring) = &scene.primitives()[1] else {
                            panic!("focus")
                        };
                        assert_eq!(ring.border_color, color(theme, "border-focused"));
                    }
                }
            }
        }
    }

    #[test]
    fn disabled_controls_do_not_paint_keyboard_focus() {
        for variant in [
            ButtonVariant::Quiet,
            ButtonVariant::Primary,
            ButtonVariant::Panel,
        ] {
            let mut scene = Scene::new();
            button_variant(
                &mut scene,
                Rect::new(0., 0., 80., 32.),
                Theme::LIGHT,
                ControlState {
                    focused: true,
                    focus_visible: true,
                    disabled: true,
                    ..Default::default()
                },
                variant,
            );
            assert_eq!(scene.primitives().len(), 1);
        }
    }
}
