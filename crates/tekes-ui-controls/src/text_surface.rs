//! Shared geometry, paint, clipping, and pointer behavior for text surfaces.
//!
//! Font preparation and glyph rasterization are injected by the host. The
//! component owns the paragraph transform and uses that same transform for
//! drawing, selection, caret reveal, and hit testing.
use std::{ops::Range, rc::Rc};

use tekes_ui_scene::{Point, Quad, Rect, Scene, Size};
use tekes_ui_text::Paragraph;
use tekes_ui_tokens::{Spacing, Stroke, Theme, proposal};

use crate::{ContainerLayout, Insets, ScrollArea, ScrollGeometry, ScrollPolicy, TextInput};

/// Gallery or application adapter for shaping paragraphs and placing glyphs.
pub trait TextSurfaceTextRenderer {
    fn push_visible(
        &mut self,
        scene: &mut Scene,
        paragraph: &Paragraph,
        origin: Point,
        color: tekes_ui_tokens::Rgba,
        clip: Rect,
    );
}

#[derive(Debug, Clone, Copy)]
pub struct ReflowAnchor {
    pub utf16: usize,
    pub within_line: f32,
    pub applied_y: f32,
    pub bottom: bool,
}

/// A single- or multi-line text surface using the shared TextInput and
/// ScrollArea state models. Layout receives already-shaped text so controls do
/// not depend on a particular font backend.
#[derive(Debug)]
pub struct TextSurface {
    pub input: TextInput,
    pub scroll: ScrollArea,
    pub paragraph: Option<Rc<Paragraph>>,
    pub shaped: String,
    pub width: Option<f32>,
    pub reflow_anchor: Option<ReflowAnchor>,
    pub outer: Rect,
    pub inner: Rect,
    pub role: proposal::TextRole,
    shaped_role: Option<proposal::TextRole>,
    pub error: bool,
    pub type_scale: f32,
    pub clauses: Vec<Range<usize>>,
    ancestor_clip: Option<Rect>,
    pointer_dragging: bool,
}

impl TextSurface {
    pub fn new(text: &str, multiline: bool, read_only: bool) -> Self {
        let mut input = TextInput::new(text, multiline);
        input.read_only = read_only;
        let mut scroll = ScrollArea::default();
        scroll.set_scroll_policy(
            if multiline {
                ScrollPolicy::Disabled
            } else {
                ScrollPolicy::Auto
            },
            if multiline {
                ScrollPolicy::Auto
            } else {
                ScrollPolicy::Disabled
            },
        );
        Self {
            input,
            scroll,
            paragraph: None,
            shaped: String::new(),
            width: None,
            reflow_anchor: None,
            outer: Rect::default(),
            inner: Rect::default(),
            role: if multiline {
                proposal::TextRole::ComposerBody
            } else {
                proposal::TextRole::UiBody
            },
            shaped_role: None,
            error: false,
            type_scale: 1.0,
            clauses: Vec::new(),
            ancestor_clip: None,
            pointer_dragging: false,
        }
    }

    /// Select the semantic text style used for shaping this surface.
    /// Direct writes to the public `role` field are also detected on layout.
    pub fn set_role(&mut self, role: proposal::TextRole) {
        self.role = role;
    }

    /// Layout and shape only when text, type scale, or pixel-snapped wrap width
    /// changes. The closure is the platform text adapter and is not called for
    /// scroll-only or unchanged frames.
    pub fn layout_with(
        &mut self,
        frame: Rect,
        ancestor_clip: Option<Rect>,
        scale: f32,
        type_scale: f32,
        mut shape: impl FnMut(&str, Option<f32>) -> Rc<Paragraph>,
    ) {
        let scale_changed = self.type_scale != type_scale;
        let role_changed = self.shaped_role != Some(self.role);
        let padding = proposal::SCROLL_CONTENT_PADDING;
        let padding_y = if self.input.multiline {
            padding
        } else {
            proposal::SCROLL_SINGLE_LINE_PADDING_Y
        };
        let insets = Insets::new(padding_y, padding, padding_y, padding);
        let layout = ContainerLayout::new(frame, Insets::ZERO, insets, ancestor_clip);
        let inner = layout.content();
        let same_text = self.shaped == self.input.text;
        if !same_text
            || self
                .reflow_anchor
                .is_some_and(|a| (a.applied_y - self.scroll.offset.y).abs() > 0.001)
        {
            self.reflow_anchor = None;
        }
        let anchor = if self.input.multiline
            && same_text
            && (inner.size != self.inner.size || scale_changed || role_changed)
        {
            self.reflow_anchor.or_else(|| {
                self.paragraph.as_ref().and_then(|p| {
                    let offset = self.scroll.offset.y;
                    let line = p
                        .lines
                        .iter()
                        .rev()
                        .find(|line| line.top as f32 <= offset)?;
                    Some(ReflowAnchor {
                        utf16: line.utf16.start,
                        within_line: offset - line.top as f32,
                        applied_y: offset,
                        bottom: self.scroll.limit().y > 0.0
                            && (offset - self.scroll.limit().y).abs() < 0.5,
                    })
                })
            })
        } else {
            None
        };

        self.outer = layout.frame();
        self.inner = inner;
        self.ancestor_clip = layout.ancestor_clip();
        if self.visible_frame().is_none() || self.input.disabled {
            self.cancel_interaction();
        }
        if scale_changed {
            self.type_scale = type_scale;
        }
        if scale_changed || role_changed {
            self.paragraph = None;
        }
        let scale = if scale.is_finite() {
            scale.max(1.0)
        } else {
            1.0
        };
        let width = self
            .input
            .multiline
            .then_some(((inner.size.width * scale).floor() / scale).max(1.0));
        if self.paragraph.is_none()
            || !same_text
            || self.width != width
            || self.shaped_role != Some(self.role)
        {
            self.paragraph = Some(shape(&self.input.text, width));
            self.shaped.clone_from(&self.input.text);
            self.width = width;
            self.shaped_role = Some(self.role);
        }
        let paragraph = self.paragraph.as_ref().expect("layout shapes a paragraph");
        self.scroll.layout(
            inner.size,
            Size::new(
                (paragraph.width as f32 + Stroke::Rule.value()).max(inner.size.width),
                paragraph.height as f32,
            ),
        );
        if let Some(mut anchor) = anchor {
            let target = if anchor.bottom {
                self.scroll.limit().y
            } else {
                paragraph
                    .caret_rect(anchor.utf16, Stroke::Rule.value())
                    .min_y()
                    + anchor.within_line
            };
            self.scroll.scroll_by(0.0, target - self.scroll.offset.y);
            anchor.applied_y = self.scroll.offset.y;
            self.reflow_anchor = Some(anchor);
        }
    }

    pub fn visible_frame(&self) -> Option<Rect> {
        match self.ancestor_clip {
            Some(clip) => self.outer.intersection(&clip),
            None if !self.outer.is_empty() => Some(self.outer),
            None => None,
        }
    }

    /// Minimum frame height for this text role at a chosen global type scale.
    /// It preserves the token minimum at 100% and grows with the measured line
    /// metric and single-line vertical content inset.
    pub fn minimum_outer_height(&self, type_scale: f32) -> f32 {
        let scale = if type_scale.is_finite() {
            type_scale.max(1.0)
        } else {
            1.0
        };
        let inset = if self.input.multiline {
            proposal::SCROLL_CONTENT_PADDING
        } else {
            proposal::SCROLL_SINGLE_LINE_PADDING_Y
        };
        (self.role.style().line_height * scale + 2.0 * inset)
            .max(proposal::metric("size", "size-button-md").expect("JSON metric"))
    }

    pub fn contains_visible(&self, point: Point) -> bool {
        self.visible_frame()
            .is_some_and(|frame| frame.contains(point))
    }

    pub fn caret_doc(&self, index: usize) -> Rect {
        self.paragraph.as_ref().map_or(Rect::default(), |p| {
            p.caret_rect(index, Stroke::Rule.value())
        })
    }

    pub fn origin(&self) -> Point {
        let base = self.paragraph.as_ref().map_or(self.inner.origin, |p| {
            p.origin_in(self.inner, !self.input.multiline)
        });
        let doc_origin = self.geometry().content_origin();
        Point::new(
            base.x + doc_origin.x - self.inner.origin.x,
            base.y + doc_origin.y - self.inner.origin.y,
        )
    }

    pub fn caret(&self) -> Rect {
        let caret = self.caret_doc(self.input.caret);
        let origin = self.origin();
        caret.offset(origin.x, origin.y)
    }

    /// Hit testing is limited to the effective visible viewport, including all
    /// ancestors and bounce blank space.
    pub fn hit(&self, point: Point) -> Option<usize> {
        if !self
            .visible_content()
            .is_some_and(|clip| clip.contains(point))
        {
            return None;
        }
        let origin = self.origin();
        self.paragraph
            .as_ref()
            .map(|p| p.hit_test(Point::new(point.x - origin.x, point.y - origin.y)))
    }

    /// Selection drags retain pointer capture outside the clip and extend from
    /// the nearest document edge.
    pub fn hit_drag(&self, point: Point) -> usize {
        let origin = self.origin();
        let doc = Point::new(point.x - origin.x, point.y - origin.y);
        self.paragraph.as_ref().map_or(0, |p| p.hit_test(doc))
    }

    pub fn selection_rects(&self, range: Range<usize>) -> Vec<Rect> {
        let Some(paragraph) = &self.paragraph else {
            return Vec::new();
        };
        let origin = self.origin();
        let Some(clip) = self.visible_content() else {
            return Vec::new();
        };
        paragraph
            .selection_rects(range, self.inner.offset(-origin.x, -origin.y))
            .into_iter()
            // Preserve the original typographic baseline. Scene clipping trims
            // visible pixels; pre-trimming could move IME underline baselines.
            .map(|rect| rect.offset(origin.x, origin.y))
            .filter(|rect| rect.intersection(&clip).is_some())
            .collect()
    }

    pub fn track(&self, vertical: bool) -> Rect {
        let thickness = proposal::SCROLLBAR_WIDTH;
        if vertical {
            Rect::new(
                self.outer.max_x() - thickness - proposal::SCROLLBAR_INSET,
                self.inner.min_y(),
                thickness,
                self.inner.size.height,
            )
        } else {
            Rect::new(
                self.inner.min_x(),
                self.outer.max_y() - thickness - proposal::SCROLLBAR_INSET,
                self.inner.size.width,
                thickness,
            )
        }
    }

    pub fn reveal(&mut self) {
        self.scroll.reveal(self.caret_doc(self.input.caret));
    }

    pub fn pointer_down(
        &mut self,
        point: Point,
        shift: bool,
        click_count: u32,
        minimum_thumb: f32,
    ) -> Option<bool> {
        if !self.contains_visible(point) || self.input.disabled {
            return None;
        }
        self.input.unmark();
        for vertical in [true, false] {
            let track = self.track(vertical).inset(-Spacing::Micro.value());
            if self
                .scroll
                .pointer_down(point, vertical, track, minimum_thumb)
            {
                self.pointer_dragging = true;
                return Some(true);
            }
        }
        let index = self.hit_drag(point);
        self.input.select(index, shift);
        if click_count >= 3 {
            self.input.select_all();
        } else if click_count == 2 {
            self.input.move_by(-1, false, true);
            self.input.move_by(1, true, true);
        }
        self.pointer_dragging = true;
        Some(true)
    }

    pub fn pointer_drag(&mut self, point: Point, minimum_thumb: f32) -> bool {
        if self.input.disabled {
            self.cancel_interaction();
            return false;
        }
        if !self.pointer_dragging {
            return false;
        }
        if let Some(vertical) = self.scroll.dragging_axis() {
            return self.scroll.pointer_drag(
                point,
                self.track(vertical).inset(-Spacing::Micro.value()),
                minimum_thumb,
            );
        }
        let index = self.hit_drag(point);
        self.input.select(index, true);
        self.reveal();
        true
    }

    pub fn pointer_up(&mut self) {
        self.pointer_dragging = false;
        self.scroll.pointer_up();
    }

    /// End pointer capture and any in-progress scroll gesture on blur,
    /// deactivation, or disabling. Selection and text contents are preserved.
    pub fn cancel_interaction(&mut self) {
        self.pointer_dragging = false;
        self.scroll.cancel();
    }

    /// Apply one translated native wheel event to this surface. A non-precise
    /// wheel at an edge can bubble to an enclosing page; precise gestures keep
    /// their elastic response and remain consumed.
    #[allow(clippy::too_many_arguments)]
    pub fn wheel_at(
        &mut self,
        point: Point,
        delta: Point,
        precise: bool,
        begin: bool,
        end: bool,
        momentum: bool,
        time: f64,
        shift: bool,
    ) -> Option<bool> {
        self.wheel_at_with_capture(
            point, delta, precise, begin, end, momentum, time, shift, false,
        )
    }

    /// Continue a precise scroll gesture that this surface already captured.
    /// Native gesture routing may keep delivering changed, ended, cancelled,
    /// or momentum events after the pointer has moved outside the viewport.
    /// The caller must only use this after routing through its retained gesture
    /// target; an initial hit must use [`Self::wheel_at`].
    #[allow(clippy::too_many_arguments)]
    pub fn wheel_captured_at(
        &mut self,
        point: Point,
        delta: Point,
        precise: bool,
        begin: bool,
        end: bool,
        momentum: bool,
        time: f64,
        shift: bool,
    ) -> Option<bool> {
        self.wheel_at_with_capture(
            point, delta, precise, begin, end, momentum, time, shift, true,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn wheel_at_with_capture(
        &mut self,
        point: Point,
        delta: Point,
        precise: bool,
        begin: bool,
        end: bool,
        momentum: bool,
        time: f64,
        shift: bool,
        captured: bool,
    ) -> Option<bool> {
        if !captured && !self.contains_visible(point) {
            return None;
        }
        let step = if precise {
            1.0
        } else {
            proposal::metric("size", "size-row-height").expect("JSON metric")
        };
        let mut x = -delta.x * step;
        let mut y = -delta.y * step;
        if shift {
            x += y;
            y = 0.0;
        }
        let before = self.scroll.offset;
        let changed = self
            .scroll
            .wheel_at(Point::new(x, y), precise, begin, end, momentum, time);
        if !changed && before == self.scroll.offset && !precise {
            None
        } else {
            Some(changed)
        }
    }

    pub fn draw<R: TextSurfaceTextRenderer>(
        &self,
        renderer: &mut R,
        scene: &mut Scene,
        theme: Theme,
        focused: bool,
        caret_on: bool,
    ) {
        let container =
            ContainerLayout::new(self.outer, Insets::ZERO, Insets::ZERO, self.ancestor_clip);
        let Some(frame_clip) = container.visible_clip() else {
            return;
        };
        let background_clip = container.background_paint_clip();
        if let Some(background_clip) = background_clip {
            scene.push_clip(background_clip);
        }
        scene.push_quad(
            Quad::new(
                self.outer,
                color(
                    if self.input.read_only {
                        "surface-1"
                    } else {
                        "transcript-input"
                    },
                    theme,
                ),
            )
            .with_corner_radius(proposal::metric("radius", "radius-input").expect("JSON radius"))
            .with_border(
                Stroke::Rule.value(),
                color(
                    if self.error {
                        "status-failed"
                    } else {
                        "border"
                    },
                    theme,
                ),
            ),
        );
        if focused && !self.input.disabled {
            crate::paint::focus_ring(
                scene,
                self.outer,
                theme,
                proposal::metric("radius", "radius-input").expect("input radius"),
            );
        }
        if background_clip.is_some() {
            scene.pop_clip();
        }

        // Keep children, selection, caret, and scrollbar ink inside the
        // visible frame. Only the frame's own background may paint its AA
        // fringe beyond that frame, bounded by the actual ancestor clip.
        scene.push_clip(frame_clip);
        let Some(content_clip) = self.visible_content() else {
            scene.pop_clip();
            return;
        };
        scene.push_clip(content_clip);
        let selection = self.input.selection();
        for rect in self.selection_rects(selection.clone()) {
            scene.push_quad(Quad::new(rect, color("text-selection", theme)));
        }
        if let Some(paragraph) = &self.paragraph {
            renderer.push_visible(
                scene,
                paragraph,
                self.origin(),
                color(
                    if self.input.disabled {
                        "disabled-text"
                    } else {
                        "primary-text"
                    },
                    theme,
                ),
                content_clip,
            );
            if !selection.is_empty() {
                for rect in self.selection_rects(selection) {
                    scene.push_clip(rect);
                    renderer.push_visible(
                        scene,
                        paragraph,
                        self.origin(),
                        color("selected-text", theme),
                        rect,
                    );
                    scene.pop_clip();
                }
            }
        }
        if let Some(mark) = &self.input.marked {
            let ranges = if self.clauses.is_empty() {
                vec![mark.clone()]
            } else {
                self.clauses
                    .iter()
                    .map(|range| {
                        mark.start.saturating_add(range.start).min(mark.end)
                            ..mark.start.saturating_add(range.end).min(mark.end)
                    })
                    .collect()
            };
            for rect in ranges
                .into_iter()
                .flat_map(|range| self.selection_rects(range))
            {
                scene.push_quad(Quad::new(
                    Rect::new(
                        rect.min_x(),
                        rect.max_y() - Stroke::Rule.value(),
                        rect.size.width,
                        Stroke::Rule.value(),
                    ),
                    color("primary-text", theme),
                ));
            }
        }
        if focused && !self.input.read_only && caret_on && self.input.selection().is_empty() {
            scene.push_quad(Quad::new(self.caret(), color("primary-text", theme)));
        }
        scene.pop_clip();
        for vertical in [true, false] {
            if self.scroll.scrollbar_visible(vertical) {
                let track = self.track(vertical);
                let thumb = self.scroll.thumb(
                    vertical,
                    track,
                    proposal::metric("size", "size-row-height").expect("JSON metric"),
                );
                if let Some(thumb) = thumb.intersection(&frame_clip) {
                    scene.push_quad(
                        Quad::new(thumb, color("secondary-text", theme))
                            .with_corner_radius(Spacing::Micro.value()),
                    );
                }
            }
        }
        scene.pop_clip();
    }

    fn geometry(&self) -> ScrollGeometry {
        self.scroll.geometry(self.inner, self.ancestor_clip)
    }

    fn visible_content(&self) -> Option<Rect> {
        self.geometry().visible_viewport_rect()
    }
}

fn color(id: &str, theme: Theme) -> tekes_ui_tokens::Rgba {
    proposal::render_color(id, theme).expect("JSON text-surface color")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;
    use tekes_ui_scene::Primitive;

    #[derive(Default)]
    struct RecordingRenderer {
        clips: Vec<Rect>,
    }

    impl TextSurfaceTextRenderer for RecordingRenderer {
        fn push_visible(
            &mut self,
            _scene: &mut Scene,
            _paragraph: &Paragraph,
            _origin: Point,
            _color: tekes_ui_tokens::Rgba,
            clip: Rect,
        ) {
            self.clips.push(clip);
        }
    }

    fn empty_paragraph() -> Paragraph {
        Paragraph {
            lines: Vec::new(),
            height: 20.0,
            width: 0.0,
            default_line_height: 20.0,
            fonts: Vec::new(),
            utf16_len: 0,
        }
    }

    fn draw_surface(
        frame: Rect,
        ancestor: Option<Rect>,
    ) -> (TextSurface, Scene, RecordingRenderer) {
        let mut view = TextSurface::new("", false, false);
        view.layout_with(frame, ancestor, 1.0, 1.0, |_, _| Rc::new(empty_paragraph()));
        let mut scene = Scene::new();
        let mut renderer = RecordingRenderer::default();
        view.draw(&mut renderer, &mut scene, Theme::LIGHT, false, false);
        (view, scene, renderer)
    }

    #[test]
    fn fractional_text_surface_background_uses_ancestor_clip_and_keeps_inner_clip() {
        let parent = Rect::new(10.0, 10.0, 140.0, 80.0);
        let frame = Rect::new(24.1875, 24.3125, 100.0, 44.0);
        let (view, scene, renderer) = draw_surface(frame, Some(parent));
        assert_eq!(view.visible_frame(), Some(frame));
        assert!(view.contains_visible(frame.center()));
        assert!(!view.contains_visible(Point::new(frame.min_x() - 0.01, frame.center().y)));

        let content = view.visible_content().expect("visible text viewport");
        assert_eq!(renderer.clips, [content]);
        let clips: Vec<_> = scene
            .primitives()
            .iter()
            .filter_map(|primitive| match primitive {
                Primitive::PushClip(clip) => Some(*clip),
                _ => None,
            })
            .collect();
        assert_eq!(clips, [parent, frame, content]);
        assert!(matches!(
            scene.primitives().first(),
            Some(Primitive::PushClip(clip)) if *clip == parent
        ));
        assert!(matches!(
            scene.primitives().get(1),
            Some(Primitive::Quad(quad)) if quad.rect == frame
        ));
    }

    #[test]
    fn text_surface_frame_and_content_remain_strictly_clipped_to_parent_viewport() {
        let parent = Rect::new(30.0, 30.0, 60.0, 40.0);
        let frame = Rect::new(24.1875, 24.3125, 100.0, 44.0);
        let (view, scene, renderer) = draw_surface(frame, Some(parent));
        let visible_frame = Rect::new(30.0, 30.0, 60.0, 38.3125);
        assert_eq!(view.visible_frame(), Some(visible_frame));
        assert!(view.contains_visible(visible_frame.center()));
        assert!(!view.contains_visible(Point::new(29.99, visible_frame.center().y)));
        assert!(!view.contains_visible(Point::new(visible_frame.center().x, 68.32)));

        let content = view.visible_content().expect("clipped text viewport");
        assert_eq!(renderer.clips, [content]);
        let clips: Vec<_> = scene
            .primitives()
            .iter()
            .filter_map(|primitive| match primitive {
                Primitive::PushClip(clip) => Some(*clip),
                _ => None,
            })
            .collect();
        assert_eq!(clips, [parent, visible_frame, content]);
        assert!(matches!(
            scene.primitives().first(),
            Some(Primitive::PushClip(clip)) if *clip == parent
        ));
        assert!(matches!(
            scene.primitives().get(1),
            Some(Primitive::Quad(quad)) if quad.rect == frame
        ));
    }
}
