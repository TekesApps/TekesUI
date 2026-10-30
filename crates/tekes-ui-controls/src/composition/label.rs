//! Per-node readonly label service adapter. No interaction or semantics ownership.
use super::*;
use std::rc::Rc;
use tekes_ui_text::{
    LabelForeground, LabelLayout, LabelOptions, LabelOverflow, LabelSourceVersion, SemanticLabel,
    SemanticLabelError,
};

#[derive(Debug)]
pub(super) struct LabelState {
    options: LabelOptions,
    cached: Option<(String, proposal::TextRole, f32, Rc<LabelLayout>)>,
    pub failed: bool,
    pub toolbar: bool,
    semantic: Option<SemanticLabel>,
}
impl LabelState {
    pub fn new(options: LabelOptions) -> Self {
        Self {
            options,
            cached: None,
            failed: false,
            toolbar: false,
            semantic: None,
        }
    }
    pub fn toolbar(&mut self) {
        self.options.max_lines = std::num::NonZeroUsize::new(1);
        self.toolbar = true;
    }
    pub fn reset(&mut self) {
        // A new measure pass may use a new host, font registry or type scale.
        self.cached = None;
        self.failed = false;
    }
}
pub(super) struct LabelRenderer<'a> {
    host: &'a mut dyn ButtonTextRenderer,
    state: &'a mut LabelState,
    clip: Rect,
    palette: Option<Palette>,
}
impl<'a> LabelRenderer<'a> {
    pub fn new(
        host: &'a mut dyn ButtonTextRenderer,
        state: &'a mut LabelState,
        clip: Rect,
    ) -> Self {
        Self {
            host,
            state,
            clip,
            palette: None,
        }
    }
    pub fn with_palette(mut self, palette: Palette) -> Self {
        self.palette = Some(palette);
        self
    }
    pub(super) fn failed(&self) -> bool {
        self.state.failed
    }
    fn layout(
        &mut self,
        text: &str,
        role: proposal::TextRole,
        width: f32,
    ) -> Option<Rc<LabelLayout>> {
        if self
            .state
            .semantic
            .as_ref()
            .is_some_and(|label| label.text() != text)
        {
            self.state.failed = true;
            return None;
        }
        if let Some((source, cached_role, cached_width, layout)) = &self.state.cached
            && source == text
            && *cached_role == role
            && *cached_width == width
        {
            return Some(layout.clone());
        }
        let result =
            self.host
                .text_surface_host()
                .and_then(|host| match self.state.semantic.as_ref() {
                    Some(label) => {
                        host.shape_semantic_label(label, role, width, self.state.options)
                    }
                    None => host.shape_label(text, role, width, self.state.options),
                });
        if let Some(layout) = &result {
            self.state.cached = Some((text.to_owned(), role, width, layout.clone()));
        } else {
            self.state.failed = true;
            self.state.cached = None;
        }
        result
    }
}
impl ButtonTextRenderer for LabelRenderer<'_> {
    fn icon_renderer(&mut self) -> Option<&mut dyn crate::layout_view::IconRenderer> {
        self.host.icon_renderer()
    }
    fn resolved_font_size(&mut self, role: proposal::TextRole) -> f32 {
        self.host.resolved_font_size(role)
    }
    fn measure(&mut self, text: &str, role: proposal::TextRole, width: f32) -> Size {
        // Label-only legacy hosts retain clipped, unwrapped text. Hosts with
        // label shaping use the existing ellipsis service below; an advertised
        // but failed service still fails explicitly.
        if self.state.toolbar
            && self.state.semantic.is_none()
            && self.host.text_surface_host().is_none()
        {
            let source = text.replace(['\r', '\n'], " ");
            let size = self.host.measure(&source, role, f32::MAX);
            return Size::new(size.width.min(width), size.height);
        }
        self.layout(text, role, width)
            .map(|layout| {
                // Reuse the existing label service's 1/64pt fit precision.
                // Intrinsic f64 advances must not round downward into an f32
                // constraint, wrap the final grapheme, and trigger ellipsis.
                let measured_width = ((layout.width() * 64.).ceil() / 64.) as f32;
                Size::new(measured_width.min(width), layout.height() as f32)
            })
            .unwrap_or_default()
    }
    fn first_line_height(&mut self, text: &str, role: proposal::TextRole, width: f32) -> f32 {
        self.layout(text, role, width)
            .and_then(|layout| {
                layout
                    .paragraph()
                    .lines
                    .first()
                    .map(|line| line.height as f32)
            })
            .unwrap_or(0.)
    }
    fn draw(
        &mut self,
        scene: &mut Scene,
        text: &str,
        role: proposal::TextRole,
        rect: Rect,
        align: ButtonTextAlign,
        ink: Rgba,
    ) {
        if self.state.toolbar
            && self.state.semantic.is_none()
            && self.host.text_surface_host().is_none()
        {
            let source = text.replace(['\r', '\n'], " ");
            let size = self.host.measure(&source, role, f32::MAX);
            let x = match align {
                ButtonTextAlign::Leading => rect.min_x(),
                ButtonTextAlign::Center => {
                    rect.min_x() + (rect.size.width - size.width).max(0.) * 0.5
                }
            };
            scene.push_clip(self.clip.intersection(&rect).unwrap_or_default());
            self.host.draw(
                scene,
                &source,
                role,
                Rect::new(x, rect.min_y(), size.width, rect.size.height),
                ButtonTextAlign::Leading,
                ink,
            );
            scene.pop_clip();
            return;
        }
        // Use the final measurement object, rather than reshaping at the glyph
        // width (which may be smaller than its original constraint).
        let measured = self
            .state
            .cached
            .as_ref()
            .filter(|(s, r, _, _)| s == text && *r == role)
            .map(|(_, _, _, p)| p.clone());
        let Some(layout) = measured.or_else(|| self.layout(text, role, rect.size.width)) else {
            return;
        };
        let x = match align {
            ButtonTextAlign::Leading => rect.min_x(),
            ButtonTextAlign::Center => {
                rect.min_x() + (rect.size.width - layout.width() as f32).max(0.) * 0.5
            }
        };
        let origin = Point::new(
            x.round(),
            (rect.min_y() + (rect.size.height - layout.height() as f32) * 0.5).round(),
        );
        if let Some(host) = self.host.text_surface_host() {
            let clip = if self.state.options.overflow == LabelOverflow::Clip {
                self.clip.intersection(&rect).unwrap_or_default()
            } else {
                // Ellipsis keeps complete shaped glyph bearings. The original
                // component's Scene clip still enforces its visible outer frame.
                self.clip
            };
            scene.push_clip(clip);
            if let Some(label) = self.state.semantic.as_ref() {
                let Some(palette) = self.palette else {
                    self.state.failed = true;
                    scene.pop_clip();
                    return;
                };
                // CoreText has already shaped the complete attributed source.
                // Partition only its retained glyphs for painting, in original
                // line/run/glyph order. Do not shape text fragments or move ink.
                let p = layout.paragraph();
                for line in &p.lines {
                    for run in &line.runs {
                        let foreground = |g: &tekes_ui_text::ShapedGlyph| {
                            if g.cluster_utf16.is_empty() {
                                LabelForeground::Inherited
                            } else {
                                label.foreground_at(g.utf16_index)
                            }
                        };
                        let mut start = 0;
                        while start < run.glyphs.len() {
                            let role = foreground(&run.glyphs[start]);
                            let mut end = start + 1;
                            while end < run.glyphs.len() && foreground(&run.glyphs[end]) == role {
                                end += 1;
                            }
                            let color = match role {
                                LabelForeground::Inherited => ink,
                                LabelForeground::Secondary => {
                                    palette.color("secondary-text").expect("foreground role")
                                }
                                LabelForeground::Positive => {
                                    palette.color("status-succeeded").expect("foreground role")
                                }
                                LabelForeground::Negative => {
                                    palette.color("status-failed").expect("foreground role")
                                }
                            };
                            let mut part_line = line.clone();
                            part_line.runs = vec![tekes_ui_text::ShapedRun {
                                font: run.font,
                                utf16: run.utf16.clone(),
                                glyphs: run.glyphs[start..end].to_vec(),
                            }];
                            let part = tekes_ui_text::Paragraph {
                                lines: vec![part_line],
                                height: p.height,
                                width: p.width,
                                default_line_height: p.default_line_height,
                                fonts: p.fonts.clone(),
                                utf16_len: p.utf16_len,
                            };
                            host.push_visible(scene, &part, origin, color, clip);
                            start = end;
                        }
                    }
                }
            } else {
                host.push_visible(scene, layout.paragraph(), origin, ink, clip);
            }
            scene.pop_clip();
        } else {
            self.state.failed = true;
        }
    }
    fn text_surface_host(&mut self) -> Option<&mut dyn crate::TextSurfaceHost> {
        self.host.text_surface_host()
    }
    fn motion_context(&self) -> Option<ControlMotionContext> {
        self.host.motion_context()
    }
    fn animation_frames_allowed(&self) -> bool {
        self.host.animation_frames_allowed()
    }
    fn request_animation_frame(&mut self) {
        self.host.request_animation_frame();
    }
    fn backing_scale(&self) -> f32 {
        self.host.backing_scale()
    }
}

impl Content {
    /// Attach validated foreground attributes to this original label owner.
    /// Custom Buttons style their existing decorative TextContent slot; the
    /// Button itself retains its explicit complete accessible name/action.
    pub fn with_semantic_label(
        mut self,
        label: SemanticLabel,
        options: LabelOptions,
    ) -> Result<Self, SemanticLabelError> {
        let source = self
            .component::<TextContent>()
            .map(|text| text.text.as_str())
            .or_else(|| {
                self.component::<Button>()
                    .filter(|button| button.supports_label_layout())
                    .map(|button| button.label.as_str())
            })
            .ok_or(SemanticLabelError::UnsupportedComponent)?;
        if source != label.text() {
            return Err(SemanticLabelError::SourceMismatch);
        }
        let mut state = LabelState::new(options);
        state.semantic = Some(label);
        self.label = Some(state);
        Ok(self)
    }
    pub fn semantic_label(&self) -> Option<&SemanticLabel> {
        self.label.as_ref()?.semantic.as_ref()
    }
    /// Reject stale/foreign revisions before replacing either source or runs.
    /// Valid construction has already checked all Unicode/range limits.
    pub fn update_semantic_label(
        &mut self,
        expected: LabelSourceVersion,
        next: SemanticLabel,
    ) -> Result<(), SemanticLabelError> {
        let old = self
            .semantic_label()
            .ok_or(SemanticLabelError::UnsupportedComponent)?;
        if old.version() != expected
            || next.version().source != expected.source
            || next.version().revision <= expected.revision
        {
            return Err(SemanticLabelError::StaleUpdate);
        }
        let source = self
            .component::<TextContent>()
            .map(|text| text.text.as_str())
            .or_else(|| {
                self.component::<Button>()
                    .filter(|button| button.supports_label_layout())
                    .map(|button| button.label.as_str())
            })
            .ok_or(SemanticLabelError::UnsupportedComponent)?;
        if source != old.text() {
            return Err(SemanticLabelError::SourceMismatch);
        }
        if let Some(text) = self.component_mut::<TextContent>() {
            text.text = next.text().to_owned();
        } else {
            self.component_mut::<Button>()
                .expect("validated original Button")
                .label = next.text().to_owned();
        }
        let state = self.label.as_mut().expect("validated original label state");
        state.semantic = Some(next);
        state.reset();
        Ok(())
    }
}
