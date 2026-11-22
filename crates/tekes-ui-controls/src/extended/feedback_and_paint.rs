use super::{Axis, Slider, finite_or, paint_color};
use crate::{ControlKey, ControlState};
use tekes_ui_scene::{Quad, Rect, Scene};
use tekes_ui_tokens::{Palette, Rgba, proposal};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Tooltip {
    pub visible: bool,
    pub text: String,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Popover {
    pub open: bool,
    pub return_focus: Option<usize>,
}
impl Popover {
    pub fn show(&mut self, invoker: usize) {
        self.open = true;
        self.return_focus = Some(invoker)
    }
    pub fn close(&mut self) -> Option<usize> {
        self.open = false;
        self.return_focus.take()
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoticeTone {
    Info,
    Success,
    Warning,
    Error,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Toast {
    pub message: String,
    pub tone: NoticeTone,
    pub visible: bool,
    remaining_ms: u32,
    pub paused: bool,
}
impl Toast {
    pub fn new(message: impl Into<String>, tone: NoticeTone, duration_ms: u32) -> Self {
        Self {
            message: message.into(),
            tone,
            visible: true,
            remaining_ms: duration_ms,
            paused: false,
        }
    }
    pub fn remaining_ms(&self) -> u32 {
        self.remaining_ms
    }
    pub fn show(&mut self, message: impl Into<String>, tone: NoticeTone, duration_ms: u32) {
        self.message = message.into();
        self.tone = tone;
        self.remaining_ms = duration_ms;
        self.visible = true;
        self.paused = false
    }
    /// Advance deterministic fixture time. Paused toasts do not expire.
    pub fn advance(&mut self, elapsed_ms: u32) -> bool {
        if !self.visible || self.paused {
            return false;
        }
        self.remaining_ms = self.remaining_ms.saturating_sub(elapsed_ms);
        if self.remaining_ms == 0 {
            self.visible = false;
            true
        } else {
            false
        }
    }
    pub fn dismiss(&mut self) {
        self.visible = false;
        self.remaining_ms = 0;
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Banner {
    pub message: String,
    pub tone: NoticeTone,
    pub dismissible: bool,
    pub visible: bool,
}
impl Banner {
    pub fn dismiss(&mut self) -> bool {
        if !self.dismissible || !self.visible {
            return false;
        }
        self.visible = false;
        true
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatusBadge {
    Info,
    Running,
    Succeeded,
    Failed,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Progress {
    value: f32,
    pub indeterminate: bool,
}
impl Progress {
    pub fn new(value: f32) -> Self {
        Self {
            value: finite_or(value, 0.).clamp(0., 1.),
            indeterminate: false,
        }
    }
    pub fn value(self) -> f32 {
        self.value
    }
    pub fn set(&mut self, value: f32) -> bool {
        if !value.is_finite() {
            return false;
        }
        let v = value.clamp(0., 1.);
        let changed = self.value != v;
        self.value = v;
        changed
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EmptyState {
    pub title: String,
    pub detail: String,
    pub action: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ComboBox {
    pub items: Vec<String>,
    pub query: String,
    pub selected: Option<usize>,
    pub open: bool,
    pub active: Option<usize>,
    pub disabled: bool,
}
#[derive(Debug, PartialEq)]
pub(crate) struct ComboSelectionDecision {
    expected: ComboBox,
    pub(crate) index: usize,
    pub(crate) label: String,
}
impl ComboBox {
    pub(crate) fn prepare_selection(&self, index: usize) -> Option<ComboSelectionDecision> {
        if self.disabled || !self.open || !self.filtered().contains(&index) {
            return None;
        }
        Some(ComboSelectionDecision {
            expected: self.clone(),
            index,
            label: self.items.get(index)?.clone(),
        })
    }
    pub(crate) fn selection_current(&self, decision: &ComboSelectionDecision) -> bool {
        *self == decision.expected
    }
    pub(crate) fn apply_selection(&mut self, decision: ComboSelectionDecision) -> usize {
        debug_assert!(self.selection_current(&decision));
        self.active = Some(decision.index);
        self.selected = Some(decision.index);
        self.query = decision.label;
        self.open = false;
        decision.index
    }
    pub fn filtered(&self) -> Vec<usize> {
        let q = self.query.to_lowercase();
        self.items
            .iter()
            .enumerate()
            .filter_map(|(i, s)| s.to_lowercase().contains(&q).then_some(i))
            .collect()
    }
    pub fn set_query(&mut self, query: impl Into<String>) {
        if !self.disabled {
            self.query = query.into();
            self.open = true;
            self.active = self.filtered().first().copied()
        }
    }
    pub fn key(&mut self, key: ControlKey) -> Option<usize> {
        if self.disabled {
            return None;
        }
        let filtered = self.filtered();
        if !self.open && matches!(key, ControlKey::Enter | ControlKey::Down) {
            self.open = true;
            self.active = filtered.first().copied();
            return None;
        }
        match key {
            ControlKey::Escape => self.open = false,
            ControlKey::Up | ControlKey::Down => {
                if filtered.is_empty() {
                    return None;
                }
                let pos = self
                    .active
                    .and_then(|i| filtered.iter().position(|x| *x == i))
                    .unwrap_or(0);
                let next = if key == ControlKey::Down {
                    (pos + 1).min(filtered.len() - 1)
                } else {
                    pos.saturating_sub(1)
                };
                self.active = Some(filtered[next]);
            }
            ControlKey::Enter => {
                let decision = self.prepare_selection(self.active?)?;
                return Some(self.apply_selection(decision));
            }
            _ => {}
        }
        None
    }
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CommandPalette {
    pub combo: ComboBox,
}
impl CommandPalette {
    pub fn open(&mut self) {
        if self.combo.disabled {
            return;
        }
        self.combo.open = true;
        self.combo.query.clear();
        self.combo.active = self.combo.filtered().first().copied()
    }
    pub fn key(&mut self, key: ControlKey) -> Option<usize> {
        self.combo.key(key)
    }
}

/// A deliberately bounded Markdown subset: headings, paragraphs, bullet lines, and fenced code.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MarkdownBlock {
    Heading { level: u8, text: String },
    Paragraph(String),
    Bullet(String),
    Code { language: String, source: String },
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Markdown {
    pub blocks: Vec<MarkdownBlock>,
}
impl Markdown {
    /// Parse the repository's bounded Markdown subset. RichDocument uses this
    /// same parser to retain source/display UTF-16 mappings for selection.
    pub fn parse(source: &str) -> Self {
        Self {
            blocks: crate::composition::RichDocument::markdown_blocks(source),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CodeBlock {
    pub language: String,
    pub source: String,
}
impl CodeBlock {
    pub fn lines(&self) -> impl Iterator<Item = &str> {
        self.source.lines()
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiffKind {
    Equal,
    Added,
    Removed,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiffLine {
    pub kind: DiffKind,
    pub text: String,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DiffView {
    pub lines: Vec<DiffLine>,
}
impl DiffView {
    /// Line-oriented LCS diff. For very large inputs, falls back to a linear delete/add block.
    pub fn between(before: &str, after: &str) -> Self {
        let a: Vec<_> = before.lines().collect();
        let b: Vec<_> = after.lines().collect();
        if a.len().saturating_mul(b.len()) > 1_000_000
            || a.len() + b.len() > 4_000
            || before.len().saturating_add(after.len()) > 1_000_000
        {
            return Self {
                lines: a
                    .iter()
                    .map(|s| DiffLine {
                        kind: DiffKind::Removed,
                        text: (*s).into(),
                    })
                    .chain(b.iter().map(|s| DiffLine {
                        kind: DiffKind::Added,
                        text: (*s).into(),
                    }))
                    .collect(),
            };
        }
        let mut dp = vec![vec![0u32; b.len() + 1]; a.len() + 1];
        for i in (0..a.len()).rev() {
            for j in (0..b.len()).rev() {
                dp[i][j] = if a[i] == b[j] {
                    dp[i + 1][j + 1] + 1
                } else {
                    dp[i + 1][j].max(dp[i][j + 1])
                }
            }
        }
        let (mut i, mut j) = (0, 0);
        let mut lines = Vec::new();
        while i < a.len() && j < b.len() {
            if a[i] == b[j] {
                lines.push(DiffLine {
                    kind: DiffKind::Equal,
                    text: a[i].into(),
                });
                i += 1;
                j += 1
            } else if dp[i + 1][j] >= dp[i][j + 1] {
                lines.push(DiffLine {
                    kind: DiffKind::Removed,
                    text: a[i].into(),
                });
                i += 1
            } else {
                lines.push(DiffLine {
                    kind: DiffKind::Added,
                    text: b[j].into(),
                });
                j += 1
            }
        }
        lines.extend(a[i..].iter().map(|s| DiffLine {
            kind: DiffKind::Removed,
            text: (*s).into(),
        }));
        lines.extend(b[j..].iter().map(|s| DiffLine {
            kind: DiffKind::Added,
            text: (*s).into(),
        }));
        Self { lines }
    }
}

// Token-driven painters for models whose visuals are shared by consumers.
/// Decorative separator with the compatible 1x backing-scale default.
/// Hosts that know their renderer scale should use [`separator_at_scale`].
pub fn separator(scene: &mut Scene, rect: Rect, theme: impl Into<Palette>, axis: Axis) {
    separator_at_scale(scene, rect, theme, axis, 1.0);
}
/// Decorative separator using the existing JSON-backed hairline rule.
/// Does not change the supplied layout frame or imply a draggable boundary.
pub fn separator_at_scale(
    scene: &mut Scene,
    rect: Rect,
    theme: impl Into<Palette>,
    axis: Axis,
    backing_scale: f32,
) {
    let (width, color) = crate::paint::decorative_border(theme, backing_scale);
    let r = match axis {
        Axis::Horizontal => Rect::new(
            rect.min_x(),
            rect.center().y - width * 0.5,
            rect.size.width.max(0.),
            width,
        ),
        Axis::Vertical => Rect::new(
            rect.center().x - width * 0.5,
            rect.min_y(),
            width,
            rect.size.height.max(0.),
        ),
    };
    scene.push_quad(Quad::new(r, color));
}
pub fn switch(
    scene: &mut Scene,
    rect: Rect,
    theme: impl Into<Palette>,
    checked: bool,
    disabled: bool,
) {
    switch_with_state(
        scene,
        rect,
        theme,
        ControlState {
            selected: checked,
            disabled,
            ..ControlState::default()
        },
    );
}
pub fn switch_with_state(
    scene: &mut Scene,
    rect: Rect,
    theme: impl Into<Palette>,
    state: ControlState,
) {
    switch_with_state_and_thumb_progress(
        scene,
        rect,
        theme,
        state,
        if state.selected { 1. } else { 0. },
    );
}
pub fn switch_on_surface(
    scene: &mut Scene,
    rect: Rect,
    palette: Palette,
    state: ControlState,
    surface: &str,
) -> Result<(), tekes_ui_tokens::surface_palette::SurfaceError> {
    switch_on_surface_with_thumb_progress(
        scene,
        rect,
        palette,
        state,
        if state.selected { 1. } else { 0. },
        surface,
    )
}
/// Thumb presentation progress never changes the model or full-track hit area.
pub fn switch_with_state_and_thumb_progress(
    scene: &mut Scene,
    rect: Rect,
    theme: impl Into<Palette>,
    state: ControlState,
    thumb_progress: f32,
) {
    let _ = switch_on_surface_with_thumb_progress(
        scene,
        rect,
        theme.into(),
        state,
        thumb_progress,
        "canvas",
    );
}
/// Paint a toggle track/thumb resolving all channels from the declared Surface packet.
pub fn switch_on_surface_with_thumb_progress(
    scene: &mut Scene,
    rect: Rect,
    palette: Palette,
    state: ControlState,
    thumb_progress: f32,
    surface: &str,
) -> Result<(), tekes_ui_tokens::surface_palette::SurfaceError> {
    let checked = state.selected;
    let radius = rect.size.height.max(0.) * 0.5;
    let stroke = proposal::metric("stroke", "stroke-rule").expect("Design System control stroke");
    let toggle = tekes_ui_tokens::surface_palette::resolve_toggle_surface(
        palette,
        surface,
        crate::paint::control_surface_state(state),
        tekes_ui_tokens::surface_palette::SurfaceOptions {
            error: state.error,
            read_only: false,
        },
    )?;
    let boundary = toggle
        .track
        .boundary
        .expect("Switch track Surface boundary");
    scene.push_quad(
        Quad::new(rect, toggle.track.background)
            .with_corner_radius(radius)
            .with_border(stroke, boundary),
    );
    let pad = 2.;
    let d = (rect.size.height - pad * 2.).max(0.);
    let thumb_radius = tekes_ui_tokens::CornerRadius::concentric(radius, pad);
    let progress = if thumb_progress.is_finite() {
        thumb_progress.clamp(0., 1.)
    } else if checked {
        1.
    } else {
        0.
    };
    let x = rect.min_x() + pad + (rect.size.width - d - pad * 2.).max(0.) * progress;
    scene.push_quad(
        Quad::new(
            Rect::new(x, rect.min_y() + pad, d, d),
            toggle.thumb.background,
        )
        .with_corner_radius(thumb_radius)
        .with_border(
            stroke,
            toggle
                .thumb
                .boundary
                .expect("Switch thumb Surface boundary"),
        ),
    );
    if state.focused && state.focus_visible && !state.disabled {
        focus_outline(scene, rect, palette);
    }
    Ok(())
}
pub fn radio(
    scene: &mut Scene,
    rect: Rect,
    theme: impl Into<Palette>,
    checked: bool,
    disabled: bool,
) {
    radio_with_state(
        scene,
        rect,
        theme,
        ControlState {
            selected: checked,
            disabled,
            ..ControlState::default()
        },
    );
}
pub fn radio_with_state(
    scene: &mut Scene,
    rect: Rect,
    theme: impl Into<Palette>,
    state: ControlState,
) {
    let theme = theme.into();
    let checked = state.selected;
    let focused = state.focused && state.focus_visible && !state.disabled;
    let size = rect.size.width.min(rect.size.height).max(0.);
    let frame = Rect::new(
        rect.center().x - size * 0.5,
        rect.center().y - size * 0.5,
        size,
        size,
    );
    let fill = if state.disabled {
        paint_color(theme, "surface-2")
    } else if checked && state.pressed {
        paint_color(theme, "control-checked-pressed")
    } else if checked && state.hovered {
        paint_color(theme, "control-checked-hover")
    } else if checked {
        paint_color(theme, "c-radio-checked-ground")
    } else if state.pressed {
        paint_color(theme, "button-pressed")
    } else if state.hovered {
        paint_color(theme, "button-hover")
    } else {
        paint_color(theme, "surface-0")
    };
    let toggle = tekes_ui_tokens::surface_palette::resolve_toggle_surface(
        theme,
        "canvas",
        crate::paint::control_surface_state(state),
        tekes_ui_tokens::surface_palette::SurfaceOptions {
            error: state.error,
            read_only: false,
        },
    )
    .expect("Radio Surface packet");
    let edge = if state.error || state.disabled || !focused {
        toggle.track.boundary.expect("Radio track Surface boundary")
    } else {
        toggle.track.focus.expect("Radio track Surface focus")
    };
    scene.push_quad(
        Quad::new(frame, fill)
            .with_corner_radius(size * 0.5)
            .with_border(
                proposal::metric(
                    "stroke",
                    if focused {
                        "c-focus-width"
                    } else {
                        "stroke-rule"
                    },
                )
                .expect("Design System control stroke"),
                edge,
            ),
    );
    if checked {
        let inset = size * 0.28;
        scene.push_quad(
            Quad::new(
                frame.inset(inset),
                paint_color(
                    theme,
                    if state.disabled {
                        "disabled-text"
                    } else {
                        "c-radio-checked-mark"
                    },
                ),
            )
            .with_corner_radius(size * 0.5),
        );
    }
}

pub(crate) fn radio_on_surface_with_progress(
    scene: &mut Scene,
    rect: Rect,
    palette: Palette,
    state: ControlState,
    surface: &str,
    select_progress: f32,
) {
    let progress = if select_progress.is_finite() {
        select_progress.clamp(0., 1.)
    } else if state.selected {
        1.
    } else {
        0.
    };
    let size = rect.size.width.min(rect.size.height).max(0.);
    let frame = Rect::new(
        rect.center().x - size * 0.5,
        rect.center().y - size * 0.5,
        size,
        size,
    );
    let disabled_ink = state
        .disabled
        .then(|| paint_color(palette, "disabled-text"));
    let focused = state.focused && state.focus_visible && !state.disabled;
    let boundary_for = |st: ControlState, checked: crate::CheckState| {
        let colors = crate::paint::check_surface_colors(palette, surface, st, checked);
        let bg = if checked == crate::CheckState::Unchecked
            && !st.disabled
            && !st.hovered
            && !st.pressed
        {
            paint_color(palette, "surface-0")
        } else {
            colors.background
        };
        let boundary = if st.error {
            colors.boundary
        } else if focused {
            colors.focus
        } else {
            disabled_ink.or(colors.boundary)
        };
        (bg, boundary, disabled_ink.or(colors.marker))
    };

    let (background, boundary, marker) = if progress <= 0. {
        let (bg, bd, _) = boundary_for(
            ControlState {
                selected: false,
                ..state
            },
            crate::CheckState::Unchecked,
        );
        (bg, bd, None)
    } else if progress >= 1. {
        boundary_for(
            ControlState {
                selected: true,
                ..state
            },
            crate::CheckState::Checked,
        )
    } else {
        let (bg0, bd0, _) = boundary_for(
            ControlState {
                selected: false,
                ..state
            },
            crate::CheckState::Unchecked,
        );
        let (bg1, bd1, m1) = boundary_for(
            ControlState {
                selected: true,
                ..state
            },
            crate::CheckState::Checked,
        );
        let bg = crate::paint::mix_rgba_linear(bg0, bg1, progress);
        let bd = match (bd0, bd1) {
            (Some(a), Some(b)) => Some(crate::paint::mix_rgba_linear(a, b, progress)),
            (Some(a), None) => Some(crate::paint::mix_rgba_linear(
                a,
                Rgba::new(a.r, a.g, a.b, 0.),
                progress,
            )),
            (None, Some(b)) => Some(crate::paint::mix_rgba_linear(
                Rgba::new(b.r, b.g, b.b, 0.),
                b,
                progress,
            )),
            (None, None) => None,
        };
        let marker = m1.map(|m| Rgba::new(m.r, m.g, m.b, (m.a * progress).clamp(0., 1.)));
        (bg, bd, marker)
    };

    let mut quad = Quad::new(frame, background).with_corner_radius(size * 0.5);
    if let Some(boundary) = boundary {
        quad = quad.with_border(
            proposal::metric(
                "stroke",
                if focused {
                    "c-focus-width"
                } else {
                    "stroke-rule"
                },
            )
            .expect("control stroke"),
            boundary,
        );
    }
    scene.push_quad(quad);
    if progress > 0.
        && let Some(marker) = marker
    {
        let dot_rect = if progress >= 1. {
            frame.inset(size * 0.28)
        } else {
            let dot_size = (size * 0.44 * progress).max(0.);
            Rect::new(
                frame.center().x - dot_size * 0.5,
                frame.center().y - dot_size * 0.5,
                dot_size,
                dot_size,
            )
        };
        scene.push_quad(Quad::new(dot_rect, marker).with_corner_radius(size * 0.5));
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SliderGeometry {
    pub track: Rect,
    pub thumb: Rect,
    pub thumb_center_x: f32,
}
pub fn slider(
    scene: &mut Scene,
    rect: Rect,
    theme: impl Into<Palette>,
    model: Slider,
    state: ControlState,
) -> SliderGeometry {
    slider_on_surface(scene, rect, theme.into(), model, state, "canvas")
        .expect("Slider canvas Surface packet")
}
/// Paint a Slider resolving all track, fill and thumb channels from the declared Surface packet.
pub fn slider_on_surface(
    scene: &mut Scene,
    rect: Rect,
    palette: Palette,
    model: Slider,
    state: ControlState,
    surface: &str,
) -> Result<SliderGeometry, tekes_ui_tokens::surface_palette::SurfaceError> {
    let disabled = state.disabled || model.disabled;
    // The visual thumb stays compact while SliderView retains the full row hit target.
    let diameter = proposal::metric("size", "c-slider-thumb-size")
        .expect("slider thumb size")
        .min(rect.size.height.max(0.))
        .min(rect.size.width.max(0.));
    let radius = diameter * 0.5;
    let travel = (rect.size.width.max(0.) - diameter).max(0.);
    let center_x = rect.min_x() + radius + travel * model.normalized_value() as f32;
    let track_height = proposal::metric("size", "c-slider-track-size").expect("slider track size");
    let track = Rect::new(
        rect.min_x() + radius,
        rect.center().y - track_height * 0.5,
        travel,
        track_height,
    );
    let thumb = Rect::new(
        center_x - radius,
        rect.center().y - radius,
        diameter,
        diameter,
    );
    let range = tekes_ui_tokens::surface_palette::resolve_range_surface(
        palette,
        surface,
        if disabled {
            "disabled"
        } else if state.pressed {
            "pressed"
        } else if state.hovered {
            "hover"
        } else {
            "rest"
        },
    )?;
    scene.push_quad(
        Quad::new(track, range.track.background).with_corner_radius(track.size.height * 0.5),
    );
    if let Some(marker) = range.track.marker {
        scene.push_quad(
            Quad::new(
                Rect::new(
                    track.min_x(),
                    track.min_y(),
                    (center_x - track.min_x()).max(0.),
                    track.size.height,
                ),
                marker,
            )
            .with_corner_radius(track.size.height * 0.5),
        );
    }
    scene.push_quad(
        Quad::new(thumb, range.thumb.background)
            .with_corner_radius(radius)
            .with_border(
                1.,
                range.thumb.boundary.expect("Slider thumb Surface boundary"),
            ),
    );
    if state.focused && state.focus_visible && !state.disabled {
        focus_outline(scene, thumb, palette);
    }
    Ok(SliderGeometry {
        track,
        thumb,
        thumb_center_x: center_x,
    })
}
fn focus_outline(scene: &mut Scene, rect: Rect, theme: Palette) {
    crate::paint::focus_ring(
        scene,
        rect,
        theme,
        rect.size.width.min(rect.size.height) * 0.5,
    );
}
/// The host shapes the label text and uses the returned semantic ink color.
pub fn link_underline(
    scene: &mut Scene,
    rect: Rect,
    theme: impl Into<Palette>,
    state: ControlState,
) -> Rgba {
    link_underline_on_surface(scene, rect, theme.into(), state, "canvas")
        .expect("default Link Surface")
}
/// Resolve the complete original Link packet before painting. No Button styling
/// or new hover/pressed state machine is introduced.
pub fn link_underline_on_surface(
    scene: &mut Scene,
    rect: Rect,
    palette: Palette,
    state: ControlState,
    surface: &str,
) -> Result<Rgba, tekes_ui_tokens::surface_palette::SurfaceError> {
    link_underline_with_content_on_surface(scene, rect, palette, state, surface, None)
}
// Geometry comes from the same renderer/role/width that paints the original
// label. A long wrapped label must not extend into a rounded ground's cutouts.
pub(crate) fn link_underline_with_content_on_surface(
    scene: &mut Scene,
    rect: Rect,
    palette: Palette,
    state: ControlState,
    surface: &str,
    text_bounds: Option<Rect>,
) -> Result<Rgba, tekes_ui_tokens::surface_palette::SurfaceError> {
    let phase = if state.disabled {
        "disabled"
    } else if state.pressed {
        "pressed"
    } else if state.hovered {
        "hover"
    } else {
        "rest"
    };
    let colors =
        tekes_ui_tokens::surface_palette::resolve_surface(palette, surface, "link", phase)?;
    let mut radius = rect.size.width.min(rect.size.height).max(0.) * 0.5;
    if let Some(content) = text_bounds {
        let horizontal = (content.min_x() - rect.min_x())
            .min(rect.max_x() - content.max_x())
            .max(0.);
        let vertical = (content.min_y() - rect.min_y())
            .min(rect.max_y() - content.max_y())
            .max(0.);
        // A corner radius no larger than the sum of both clearances contains
        // the whole text rectangle, including multi-line first/last rows.
        radius = radius.min(horizontal + vertical);
    }
    let mut background = Quad::new(rect, colors.background).with_corner_radius(radius);
    if let Some(boundary) = colors.boundary {
        background = background.with_border(
            proposal::metric("stroke", "stroke-rule").expect("Link boundary stroke"),
            boundary,
        );
    }
    scene.push_quad(background);
    let focused = state.focused && state.focus_visible && !state.disabled;
    if focused && let Some(focus) = colors.focus {
        crate::paint::focus_ring_with_color(scene, rect, focus, radius);
    }
    let thickness = proposal::metric(
        "stroke",
        if focused {
            "stroke-emphasis"
        } else {
            "stroke-rule"
        },
    )
    .expect("Link underline stroke");
    // Keep the underline inside the original rounded ground, rather than on
    // its bottom tangent where it crosses the independently offset focus ring.
    // The same content inset is used by LinkView; extreme short/narrow frames
    // are clamped without changing their hit or accessibility allocation.
    let inset = proposal::metric("spacing", "space-regular")
        .expect("Link content inset")
        .max(0.)
        .min(radius);
    let dy = (radius - inset).max(0.);
    let arc_inset = radius - (radius * radius - dy * dy).max(0.).sqrt();
    let horizontal = inset.max(arc_inset).min(rect.size.width.max(0.) * 0.5);
    let height = thickness.min((rect.size.height - inset * 2.).max(0.));
    scene.push_quad(Quad::new(
        Rect::new(
            rect.min_x() + horizontal,
            rect.max_y() - inset - height,
            (rect.size.width - horizontal * 2.).max(0.),
            height,
        ),
        colors.text,
    ));
    Ok(colors.text)
}
/// Compatibility entry point using the original canvas context.
pub fn progress(scene: &mut Scene, rect: Rect, theme: impl Into<Palette>, model: Progress) {
    progress_on_surface(scene, rect, theme.into(), model, "canvas")
        .expect("generated Progress canvas packet");
}
/// Resolve the complete shared packet before writing any scene primitives.
/// Progress retains its original finite scalar and static unknown geometry.
pub fn progress_on_surface(
    scene: &mut Scene,
    rect: Rect,
    palette: Palette,
    model: Progress,
    surface: &str,
) -> Result<(), tekes_ui_tokens::surface_palette::SurfaceError> {
    use tekes_ui_tokens::surface_palette::{SurfaceError, resolve_surface};
    let colors = resolve_surface(palette, surface, "progress-track", "rest")?;
    let marker = colors.marker.ok_or(SurfaceError::UnknownRole)?;
    let width = rect.size.width.max(0.);
    let height = rect.size.height.max(0.);
    let track = Rect::new(rect.min_x(), rect.min_y(), width, height);
    scene.push_quad(Quad::new(track, colors.background).with_corner_radius(height * 0.5));
    let active = if model.indeterminate {
        width.min(width * 0.3)
    } else {
        width * model.value
    };
    scene.push_quad(
        Quad::new(
            Rect::new(rect.min_x(), rect.min_y(), active, height),
            marker,
        )
        .with_corner_radius(height * 0.5),
    );
    // Keep the shared necessary boundary visible even at 100%. The fill and
    // rounded geometry remain the original model's full allocation.
    if let Some(boundary) = colors.boundary {
        scene.push_quad(
            Quad::new(track, Rgba::new(0., 0., 0., 0.))
                .with_corner_radius(height * 0.5)
                .with_border(
                    proposal::metric("stroke", "stroke-rule").expect("Progress boundary stroke"),
                    boundary,
                ),
        );
    }
    Ok(())
}
/// Compatibility entry point; the canonical canvas packet owns all solid ink.
pub fn status_badge(scene: &mut Scene, rect: Rect, theme: impl Into<Palette>, status: StatusBadge) {
    status_badge_on_surface(scene, rect, theme, status, "canvas")
        .expect("valid StatusBadge canvas allocation and packet");
}
/// Resolve the complete rest-only packet before writing any Scene primitive.
/// Invalid/non-finite allocations reject atomically as an unknown combination.
pub fn status_badge_on_surface(
    scene: &mut Scene,
    rect: Rect,
    theme: impl Into<Palette>,
    status: StatusBadge,
    surface: &str,
) -> Result<(), tekes_ui_tokens::surface_palette::SurfaceError> {
    let colors = tekes_ui_tokens::surface_palette::resolve_surface(
        theme.into(),
        surface,
        status_badge_usage(status),
        "rest",
    )?;
    if !rect.min_x().is_finite()
        || !rect.min_y().is_finite()
        || !rect.max_x().is_finite()
        || !rect.max_y().is_finite()
        || !rect.size.width.is_finite()
        || !rect.size.height.is_finite()
        || rect.size.width < 0.
        || rect.size.height < 0.
    {
        return Err(tekes_ui_tokens::surface_palette::SurfaceError::UnknownCombination);
    }
    scene.push_quad(
        Quad::new(rect, colors.background).with_corner_radius(rect.size.height.max(0.) * 0.5),
    );
    Ok(())
}
pub(crate) fn status_badge_usage(status: StatusBadge) -> &'static str {
    match status {
        StatusBadge::Info => "status-badge-info",
        StatusBadge::Running => "status-badge-running",
        StatusBadge::Succeeded => "status-badge-succeeded",
        StatusBadge::Failed => "status-badge-failed",
    }
}

// Shared semantic token lookup for the existing extended painters.
