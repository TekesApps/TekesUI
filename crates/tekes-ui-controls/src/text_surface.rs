//! Shared geometry, paint, clipping, and pointer behavior for text surfaces.
//!
//! Font preparation and glyph rasterization are injected by the host. The
//! component owns the paragraph transform and uses that same transform for
//! drawing, selection, caret reveal, and hit testing.
use std::{cell::RefCell, ops::Range, rc::Rc};

use tekes_ui_scene::{Point, Quad, Rect, Scene, Size};
use tekes_ui_text::Paragraph;
#[cfg(test)]
use tekes_ui_tokens::Theme;
use tekes_ui_tokens::{Spacing, Stroke, proposal};

use crate::{
    ContainerLayout, ControlState, Insets, ScrollArea, ScrollGeometry, ScrollPolicy, TextInput,
};

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

/// Optional application text service for composed editable controls. The host
/// retains its font backend; TextSurface retains editing and geometry.
pub trait TextSurfaceHost: TextSurfaceTextRenderer {
    /// Supplied by the native text service's blink timer, not a second component clock.
    fn caret_visible(&self) -> bool {
        true
    }
    fn shape_surface(
        &mut self,
        text: &str,
        role: proposal::TextRole,
        width: Option<f32>,
    ) -> Rc<Paragraph>;
    /// Optional read-only label capability using this host's existing font registry.
    /// Returning None rejects explicit label overflow; it must never silently wrap.
    fn shape_label(
        &mut self,
        _text: &str,
        _role: proposal::TextRole,
        _width: f32,
        _options: tekes_ui_text::LabelOptions,
    ) -> Option<Rc<tekes_ui_text::LabelLayout>> {
        None
    }
    fn type_scale(&self) -> f32 {
        1.0
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ReflowAnchor {
    pub utf16: usize,
    pub within_line: f32,
    pub applied_y: f32,
    pub bottom: bool,
}

/// Derived caret geometry only; editor values/history remain in TextInput.
#[derive(Clone, Copy, Debug)]
struct Navigation {
    revision: (u64, u64),
    anchor: usize,
    caret: usize,
    line: usize,
    preferred_x: f32,
}

/// Presentation of the existing text/selection surface. Content decoration and
/// spacing belong to its parent; editing, shaping and native ranges stay here.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextSurfacePresentation {
    #[default]
    Field,
    /// No field chrome or insets. Selection and the shared keyboard-focus
    /// indicator remain visible; a parent supplies decoration and scrolling.
    Content,
}

/// Horizontal placement for single-line text. Multiline paragraphs retain
/// their shaped line layout. All caret/hit/selection queries use this origin.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextSurfaceAlignment {
    #[default]
    Leading,
    Center,
    Trailing,
}

/// One intrinsic measurement retained until arrangement. This is derived data;
/// measuring must not publish editing/scroll/IME geometry.
#[derive(Debug)]
struct MeasuredParagraph {
    text: String,
    role: proposal::TextRole,
    width: Option<f32>,
    type_scale: f32,
    paragraph: Rc<Paragraph>,
}

/// A single- or multi-line text surface using the shared TextInput and
/// ScrollArea state models. Layout receives already-shaped text so controls do
/// not depend on a particular font backend.
#[derive(Debug)]
pub struct TextSurface {
    pub input: TextInput,
    pub presentation: TextSurfacePresentation,
    pub alignment: TextSurfaceAlignment,
    /// Soft wrapping for multiline text. Single-line editing remains unwrapped.
    /// Scroll axes remain configurable on the original `scroll` model.
    pub wrap: bool,
    pub scroll: ScrollArea,
    pub paragraph: Option<Rc<Paragraph>>,
    pub shaped: String,
    pub width: Option<f32>,
    pub reflow_anchor: Option<ReflowAnchor>,
    pub outer: Rect,
    pub inner: Rect,
    pub role: proposal::TextRole,
    shaped_role: Option<proposal::TextRole>,
    measured: RefCell<Option<MeasuredParagraph>>,
    pub error: bool,
    pub type_scale: f32,
    pub clauses: Vec<Range<usize>>,
    ancestor_clip: Option<Rect>,
    pointer_dragging: bool,
    pending_reveal: Option<Point>,
    navigation: Option<Navigation>,
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
            presentation: TextSurfacePresentation::Field,
            alignment: TextSurfaceAlignment::Leading,
            wrap: true,
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
            measured: RefCell::new(None),
            error: false,
            type_scale: 1.0,
            clauses: Vec::new(),
            ancestor_clip: None,
            pointer_dragging: false,
            pending_reveal: None,
            navigation: None,
        }
    }

    /// Read-only controlled-update preflight on the original input instance.
    /// Does not shape, reveal, alter scroll/navigation, clear clauses, commit
    /// composition or invalidate layout. Apply rechecks all conditions; callers
    /// must keep preflight and a larger synchronous commit free of event/edit
    /// interleaving. Arbitrary application/tree errors are not rolled back.
    pub fn check_external_value(
        &self,
        expected: &crate::TextEditBaseline,
        value: &crate::DraftRevision,
        selection: crate::ExternalSelection,
        undo: crate::ExternalUndo,
    ) -> crate::ExternalValueResult {
        self.input
            .check_external_value(expected, value, selection, undo)
    }

    /// Select the semantic text style used for shaping this surface.
    /// Direct writes to the public `role` field are also detected on layout.
    /// Controlled values update the original input instance and derived surface
    /// state atomically. No native owner or text backend is replaced.
    pub fn apply_external_value(
        &mut self,
        expected: &crate::TextEditBaseline,
        value: &crate::DraftRevision,
        selection: crate::ExternalSelection,
        undo: crate::ExternalUndo,
    ) -> crate::ExternalValueResult {
        let keep_navigation = self.navigation.is_some_and(|n| self.navigation_valid(n));
        let explicit_selection = !matches!(selection, crate::ExternalSelection::Preserve);
        let result = self
            .input
            .apply_external_value(expected, value, selection, undo);
        if let crate::ExternalValueResult::Applied {
            changed,
            document_changed,
        } = result
            && (changed || document_changed)
        {
            self.clauses.clear();
            self.pending_reveal = None;
            self.navigation = None;
            self.reflow_anchor = None;
            self.cancel_interaction();
            if document_changed {
                self.scroll
                    .scroll_by(-self.scroll.offset.x, -self.scroll.offset.y);
            }
            if explicit_selection || document_changed {
                self.reveal();
            }
        }
        if matches!(
            result,
            crate::ExternalValueResult::Applied {
                changed: false,
                document_changed: false
            }
        ) && keep_navigation
            && let Some(navigation) = &mut self.navigation
        {
            // A newer authority acknowledgement without an editor change must
            // not lose a soft-line caret affinity or a vertical column.
            navigation.revision = self.input.edit_revision();
        }
        result
    }
    pub fn set_role(&mut self, role: proposal::TextRole) {
        self.role = role;
    }

    fn content_insets(&self) -> Insets {
        if self.presentation == TextSurfacePresentation::Content {
            Insets::ZERO
        } else {
            let x = proposal::SCROLL_CONTENT_PADDING;
            let y = if self.input.multiline {
                x
            } else {
                proposal::SCROLL_SINGLE_LINE_PADDING_Y
            };
            Insets::new(y, x, y, x)
        }
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
        // An edit may request caret reveal before the new paragraph exists.
        // A later user scroll supersedes that request, including direct public
        // ScrollArea updates; ordinary layout never invents a reveal request.
        let reveal_after_layout = self
            .pending_reveal
            .take()
            .is_some_and(|offset| offset == self.scroll.offset);
        let scale_changed = self.type_scale != type_scale;
        let role_changed = self.shaped_role != Some(self.role);
        let insets = self.content_insets();
        let layout = ContainerLayout::new(frame, Insets::ZERO, insets, ancestor_clip);
        // The visible content area is the real editing/scroll viewport. Keeping
        // the larger allocation here makes a clipped caret impossible to reveal
        // and gives shaping, hit testing and IME different viewport bounds.
        let inner = if self.presentation == TextSurfacePresentation::Content {
            // A parent document viewport clips ink, not the paragraph's wrap
            // width or height. Scrolling past a block must not reshape it.
            layout.content()
        } else {
            layout.visible_clip().unwrap_or_else(|| layout.content())
        };
        let reveal_composition = self.input.marked.is_some()
            && (scale_changed
                || self.inner != inner
                || self.ancestor_clip != layout.ancestor_clip());
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
            && (inner.size != self.inner.size
                || scale_changed
                || role_changed
                || self.width != self.wrap_width(inner.size.width, scale))
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
        let width = self.wrap_width(inner.size.width, scale);
        if self.paragraph.is_none()
            || !same_text
            || self.width != width
            || self.shaped_role != Some(self.role)
        {
            self.navigation = None;
            let measured = self.measured.get_mut().take().filter(|m| {
                m.text == self.input.text
                    && m.role == self.role
                    && m.width == width
                    && m.type_scale == type_scale
            });
            self.paragraph =
                Some(measured.map_or_else(|| shape(&self.input.text, width), |m| m.paragraph));
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
        if (reveal_after_layout || reveal_composition)
            && !self.input.disabled
            && self.visible_frame().is_some()
        {
            self.scroll.reveal(self.caret_doc(self.input.caret));
        }
    }

    pub fn visible_frame(&self) -> Option<Rect> {
        match self.ancestor_clip {
            Some(clip) => self.outer.intersection(&clip),
            None if !self.outer.is_empty() => Some(self.outer),
            None => None,
        }
    }
    pub fn ancestor_clip(&self) -> Option<Rect> {
        self.ancestor_clip
    }

    fn navigation_valid(&self, navigation: Navigation) -> bool {
        navigation.revision == self.input.edit_revision()
            && navigation.anchor == self.input.anchor
            && navigation.caret == self.input.caret
            && self.layout_current()
            && self
                .paragraph
                .as_ref()
                .and_then(|p| p.lines.get(navigation.line))
                .is_some_and(|line| {
                    line.utf16.start <= navigation.caret && navigation.caret <= line.utf16.end
                })
    }
    fn layout_current(&self) -> bool {
        self.paragraph.is_some()
            && self.shaped == self.input.text
            && self.shaped_role == Some(self.role)
            && self.width.is_some() == (self.input.multiline && self.wrap)
    }
    fn navigation_line(&self) -> Option<usize> {
        if !self.layout_current() {
            return None;
        }
        if let Some(n) = self.navigation.filter(|n| self.navigation_valid(*n)) {
            return Some(n.line);
        }
        let p = self.paragraph.as_ref()?;
        p.lines
            .iter()
            .position(|line| {
                self.input.caret >= line.utf16.start && self.input.caret < line.utf16.end
            })
            .or_else(|| p.lines.len().checked_sub(1))
    }
    fn move_visual(&mut self, direction: isize, edge: bool, extend: bool) -> Option<()> {
        if !self.input.multiline && !edge {
            return None;
        }
        if !self.input.multiline {
            self.navigation = None;
            self.input
                .select(if direction < 0 { 0 } else { self.input.len() }, extend);
            return Some(());
        }
        let current = self.navigation_line()?;
        let p = self.paragraph.as_ref()?;
        let line_index = if edge {
            current
        } else {
            current
                .saturating_add_signed(direction)
                .min(p.lines.len() - 1)
        };
        let line = &p.lines[line_index];
        let x = self
            .navigation
            .filter(|n| self.navigation_valid(*n))
            .map_or_else(
                || self.caret_doc(self.input.caret).min_x(),
                |n| n.preferred_x,
            );
        let target = if edge {
            if direction < 0 {
                line.utf16.start
            } else if line.has_newline {
                self.input
                    .boundaries()
                    .into_iter()
                    .rev()
                    .find(|i| *i >= line.utf16.start && *i < line.utf16.end)
                    .unwrap_or(line.utf16.start)
            } else {
                line.utf16.end
            }
        } else {
            p.hit_test(Point::new(x, (line.top + line.height / 2.) as f32))
        };
        let goal_x = if edge {
            line.carets
                .iter()
                .find(|(i, _)| *i == target)
                .map_or(line.width as f32, |(_, x)| *x as f32)
        } else {
            x
        };
        self.input.select(target, extend);
        self.navigation = Some(Navigation {
            revision: self.input.edit_revision(),
            anchor: self.input.anchor,
            caret: self.input.caret,
            line: line_index,
            preferred_x: goal_x,
        });
        Some(())
    }
    fn select_visual_point(&mut self, point: Point, extend: bool) {
        let index = self.hit_drag(point);
        self.navigation = None;
        self.input.select(index, extend);
        if !self.layout_current() {
            return;
        }
        let p = self.paragraph.as_ref().expect("current paragraph");
        let y = point.y - self.origin().y;
        if let Some((index, line)) = p
            .lines
            .iter()
            .enumerate()
            .find(|(_, line)| f64::from(y) < line.top + line.height)
            .or_else(|| p.lines.iter().enumerate().next_back())
        {
            let x = line
                .carets
                .iter()
                .find(|(i, _)| *i == self.input.caret)
                .map_or(line.width, |(_, x)| *x);
            self.navigation = Some(Navigation {
                revision: self.input.edit_revision(),
                anchor: self.input.anchor,
                caret: self.input.caret,
                line: index,
                preferred_x: x as f32,
            });
        }
    }

    /// Standard editing commands over the original input/paragraph. Some(bool)
    /// means handled, with bool describing a visible/editor change. Unknown or
    /// geometry-stale commands return None without applying a guessed move.
    /// Arrange/refresh text before visual navigation; ContentHost::event_with_text
    /// supplies that barrier using the caller's existing shaping backend.
    /// Marked text remains owned by IME except explicit Cancel/Undo/Redo.
    pub fn command(&mut self, command: &str) -> Option<bool> {
        if self.input.disabled
            || (self.input.marked.is_some()
                && !matches!(command, "cancelOperation:" | "undo:" | "redo:"))
        {
            return None;
        }
        let before = (
            self.input.text.clone(),
            self.input.anchor,
            self.input.caret,
            self.input.marked.clone(),
        );
        let caret = self.caret();
        let extend = command.contains("AndModifySelection");
        match command {
            "moveToBeginningOfLine:" | "moveToBeginningOfLineAndModifySelection:" => {
                self.move_visual(-1, true, extend)?;
            }
            "moveToEndOfLine:" | "moveToEndOfLineAndModifySelection:" => {
                self.move_visual(1, true, extend)?;
            }
            "moveUp:" | "moveUpAndModifySelection:" => {
                self.move_visual(-1, false, extend)?;
            }
            "moveDown:" | "moveDownAndModifySelection:" => {
                self.move_visual(1, false, extend)?;
            }
            "selectAll:" => {
                self.navigation = None;
                self.input.select_all();
            }
            "moveToBeginningOfDocument:" | "moveToBeginningOfDocumentAndModifySelection:" => {
                self.navigation = None;
                self.input.select(0, extend);
            }
            "moveToEndOfDocument:" | "moveToEndOfDocumentAndModifySelection:" => {
                self.navigation = None;
                self.input.select(self.input.len(), extend);
            }
            "moveLeft:"
            | "moveLeftAndModifySelection:"
            | "moveBackward:"
            | "moveBackwardAndModifySelection:" => {
                self.navigation = None;
                self.input.move_by(-1, extend, false);
            }
            "moveRight:"
            | "moveRightAndModifySelection:"
            | "moveForward:"
            | "moveForwardAndModifySelection:" => {
                self.navigation = None;
                self.input.move_by(1, extend, false);
            }
            "moveWordLeft:"
            | "moveWordLeftAndModifySelection:"
            | "moveWordBackward:"
            | "moveWordBackwardAndModifySelection:" => {
                self.navigation = None;
                self.input.move_by(-1, extend, true);
            }
            "moveWordRight:"
            | "moveWordRightAndModifySelection:"
            | "moveWordForward:"
            | "moveWordForwardAndModifySelection:" => {
                self.navigation = None;
                self.input.move_by(1, extend, true);
            }
            "deleteBackward:" => {
                self.navigation = None;
                self.input.delete(true, false);
            }
            "deleteForward:" => {
                self.navigation = None;
                self.input.delete(false, false);
            }
            "deleteWordBackward:" => {
                self.navigation = None;
                self.input.delete(true, true);
            }
            "deleteWordForward:" => {
                self.navigation = None;
                self.input.delete(false, true);
            }
            "undo:" => {
                self.navigation = None;
                self.input.undo();
            }
            "redo:" => {
                self.navigation = None;
                self.input.redo();
            }
            "cancelOperation:" => {
                self.navigation = None;
                self.input.cancel_composition();
            }
            "insertNewline:" | "insertLineBreak:" => {
                self.navigation = None;
                if self.input.multiline {
                    self.input.insert("\n", None);
                }
            }
            _ => return None,
        }
        if self.input.marked.is_none() {
            self.clauses.clear();
        }
        let changed = before
            != (
                self.input.text.clone(),
                self.input.anchor,
                self.input.caret,
                self.input.marked.clone(),
            )
            || caret != self.caret();
        if changed {
            self.reveal();
        }
        Some(changed)
    }

    /// Minimum frame height for this text role at a chosen global type scale.
    /// Field presentation preserves the token minimum and input insets.
    /// Content presentation uses only the semantic line metric.
    pub fn minimum_outer_height(&self, type_scale: f32) -> f32 {
        let scale = if type_scale.is_finite() {
            type_scale.max(1.0)
        } else {
            1.0
        };
        let insets = self.content_insets();
        let height = self.role.style().line_height * scale + insets.top + insets.bottom;
        if self.presentation == TextSurfacePresentation::Content {
            height
        } else {
            height.max(proposal::metric("size", "size-button-md").expect("JSON metric"))
        }
    }

    fn wrap_width(&self, inner_width: f32, scale: f32) -> Option<f32> {
        let scale = if scale.is_finite() { scale.max(1.) } else { 1. };
        (self.input.multiline && self.wrap)
            .then_some(((inner_width * scale).floor() / scale).max(1.))
    }

    /// Intrinsic height using the same text role, padding and snapped wrapping
    /// width as layout. Measurement does not mutate editing, scrolling, reading
    /// anchors or the currently arranged paragraph/geometry.
    pub fn measured_outer_height(
        &self,
        width: f32,
        scale: f32,
        type_scale: f32,
        mut shape: impl FnMut(&str, Option<f32>) -> Rc<Paragraph>,
    ) -> f32 {
        let insets = self.content_insets();
        let width = self.wrap_width((width - insets.left - insets.right).max(0.), scale);
        let arranged = self.paragraph.as_ref().filter(|_| {
            self.shaped == self.input.text
                && self.shaped_role == Some(self.role)
                && self.width == width
                && self.type_scale == type_scale
        });
        let paragraph = if let Some(paragraph) = arranged {
            paragraph.clone()
        } else {
            let cached = self
                .measured
                .borrow()
                .as_ref()
                .filter(|m| {
                    m.text == self.input.text
                        && m.role == self.role
                        && m.width == width
                        && m.type_scale == type_scale
                })
                .map(|m| m.paragraph.clone());
            if let Some(paragraph) = cached {
                paragraph
            } else {
                // No interior borrow held across the application's font callback.
                let paragraph = shape(&self.input.text, width);
                *self.measured.borrow_mut() = Some(MeasuredParagraph {
                    text: self.input.text.clone(),
                    role: self.role,
                    width,
                    type_scale,
                    paragraph: paragraph.clone(),
                });
                paragraph
            }
        };
        (paragraph.height as f32 + insets.top + insets.bottom)
            .max(self.minimum_outer_height(type_scale))
    }

    pub fn contains_visible(&self, point: Point) -> bool {
        self.visible_frame()
            .is_some_and(|frame| frame.contains(point))
    }

    pub fn caret_doc(&self, index: usize) -> Rect {
        if index == self.input.caret
            && let Some(navigation) = self.navigation.filter(|n| self.navigation_valid(*n))
            && let Some(line) = self
                .paragraph
                .as_ref()
                .and_then(|p| p.lines.get(navigation.line))
        {
            let x = line
                .carets
                .iter()
                .find(|(i, _)| *i == index)
                .map_or(line.width, |(_, x)| *x);
            return Rect::new(
                x as f32,
                line.top as f32,
                Stroke::Rule.value(),
                line.height as f32,
            );
        }
        self.paragraph.as_ref().map_or(Rect::default(), |p| {
            p.caret_rect(index, Stroke::Rule.value())
        })
    }

    pub fn origin(&self) -> Point {
        let base = self.paragraph.as_ref().map_or(self.inner.origin, |p| {
            p.origin_in(self.inner, !self.input.multiline)
        });
        let doc_origin = self.geometry().content_origin();
        let alignment = if !self.input.multiline && self.alignment != TextSurfaceAlignment::Leading
        {
            self.paragraph.as_ref().map_or(0., |p| {
                (self.inner.size.width - p.width as f32 - Stroke::Rule.value()).max(0.)
                    * if self.alignment == TextSurfaceAlignment::Center {
                        0.5
                    } else {
                        1.0
                    }
            })
        } else {
            0.
        };
        Point::new(
            base.x + alignment + doc_origin.x - self.inner.origin.x,
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
        self.pending_reveal = (self.paragraph.is_none() || self.shaped != self.input.text)
            .then_some(self.scroll.offset);
    }

    /// Snapshot of pointer interaction using the existing hit geometry and
    /// selection/scroll capture. `None` means the pointer is not hovering;
    /// capture remains pressed outside the frame until release or cancellation.
    /// Read-only text remains selectable. Disabled text reports neither state.
    ///
    /// Hover/press preserve input paint: feedback comes from the independent
    /// focus, caret, selection and scrollbar, never Button activation/appearance.
    /// Focus remains a host input to `draw`, not owned by this pointer snapshot.
    /// Hosts call `cancel_interaction` on blur, disable, deactivation or unmount.
    pub fn pointer_state(&self, point: Option<Point>) -> ControlState {
        ControlState {
            hovered: !self.input.disabled && point.is_some_and(|p| self.contains_visible(p)),
            pressed: !self.input.disabled && self.pointer_dragging,
            disabled: self.input.disabled,
            error: self.error,
            ..ControlState::default()
        }
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
        self.select_visual_point(point, shift);
        if click_count >= 3 {
            self.navigation = None;
            self.input.select_paragraph_at(self.input.caret);
        } else if click_count == 2 {
            self.navigation = None;
            let grapheme = self.selection_hit(point, self.input.caret);
            self.input.select_word_at(grapheme);
        }
        self.pointer_dragging = true;
        Some(true)
    }

    /// Resolve the glyph under the pointer using the existing shaped selection
    /// rectangles. A nearest caret can be the end of the preceding grapheme or
    /// the start of the next one; word selection must not confuse those cases.
    fn selection_hit(&self, point: Point, caret: usize) -> usize {
        let boundaries = self.input.boundaries();
        if let Some(paragraph) = &self.paragraph {
            let origin = self.origin();
            let doc = Point::new(point.x - origin.x, point.y - origin.y);
            for pair in boundaries
                .windows(2)
                .filter(|pair| pair[0] <= caret && caret <= pair[1])
            {
                if paragraph
                    .selection_rects(pair[0]..pair[1], self.inner.offset(-origin.x, -origin.y))
                    .iter()
                    .any(|rect| rect.contains(doc))
                {
                    return pair[0];
                }
            }
        }
        if caret == self.input.len() {
            boundaries.into_iter().rev().nth(1).unwrap_or(0)
        } else {
            caret
        }
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
        self.select_visual_point(point, true);
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
        if self.scroll.scroll_policy() == (ScrollPolicy::Disabled, ScrollPolicy::Disabled) {
            // A non-scrolling text block cannot retain a trackpad gesture that
            // belongs to its parent document viewport.
            return None;
        }
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
            self.pending_reveal = None;
            Some(changed)
        }
    }

    pub fn draw<R: TextSurfaceTextRenderer + ?Sized>(
        &self,
        renderer: &mut R,
        scene: &mut Scene,
        theme: impl Into<tekes_ui_tokens::Palette>,
        focused: bool,
        caret_on: bool,
    ) {
        let theme = theme.into();
        let container =
            ContainerLayout::new(self.outer, Insets::ZERO, Insets::ZERO, self.ancestor_clip);
        let Some(frame_clip) = container.visible_clip() else {
            return;
        };
        let background_clip = container.background_paint_clip();
        if let Some(background_clip) = background_clip {
            scene.push_clip(background_clip);
        }
        if self.presentation == TextSurfacePresentation::Field {
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
                .with_corner_radius(
                    proposal::metric("radius", "radius-input").expect("JSON radius"),
                )
                .with_border(
                    Stroke::Rule.value(),
                    color(
                        proposal::control_boundary(
                            if self.error {
                                "error"
                            } else {
                                "required-input"
                            },
                            theme.theme,
                        )
                        .expect("JSON text boundary role"),
                        theme,
                    ),
                ),
            );
        }
        if focused && !self.input.disabled {
            crate::paint::focus_ring(
                scene,
                self.outer,
                theme,
                if self.presentation == TextSurfacePresentation::Field {
                    proposal::metric("radius", "radius-input").expect("input radius")
                } else {
                    0.
                },
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
        if focused
            && !self.input.disabled
            && !self.input.read_only
            && caret_on
            && self.input.selection().is_empty()
        {
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

fn color(id: &str, theme: tekes_ui_tokens::Palette) -> tekes_ui_tokens::Rgba {
    theme.color(id).expect("JSON text-surface color")
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
    fn trailing_single_line_shares_paint_hit_caret_and_overflow_origin() {
        fn paragraph(width: f64) -> Rc<Paragraph> {
            Rc::new(Paragraph {
                lines: vec![tekes_ui_text::Line {
                    index: 0,
                    utf16: 0..2,
                    bytes: 0..2,
                    ascent: 15.,
                    descent: 5.,
                    leading: 0.,
                    height: 20.,
                    top: 0.,
                    baseline_y: 15.,
                    width,
                    width_trimmed: width,
                    runs: vec![],
                    carets: vec![(0, 0.), (1, width / 2.), (2, width)],
                    has_newline: false,
                }],
                width,
                height: 20.,
                default_line_height: 20.,
                fonts: vec![],
                utf16_len: 2,
            })
        }
        let mut surface = TextSurface::new("12", false, false);
        surface.presentation = TextSurfacePresentation::Content;
        surface.alignment = TextSurfaceAlignment::Trailing;
        surface.input.caret = 2;
        let frame = Rect::new(10., 20., 200., 32.);
        surface.layout_with(frame, None, 2., 1., |_, _| paragraph(40.));
        assert!((surface.caret().max_x() - frame.max_x()).abs() < 0.01);
        assert_eq!(
            surface.hit(Point::new(frame.max_x() - 1., frame.center().y)),
            Some(2)
        );
        assert_eq!(
            surface.hit(Point::new(frame.min_x() + 1., frame.center().y)),
            Some(0)
        );
        let rects = surface.selection_rects(0..2);
        assert!(!rects.is_empty());
        assert!(rects.iter().all(|r| r.min_x() >= surface.origin().x));
        surface.paragraph = None;
        surface.layout_with(frame, None, 2., 1., |_, _| paragraph(400.));
        surface.reveal();
        assert!(surface.scroll.offset.x > 0.);
        assert!((surface.caret().max_x() - frame.max_x()).abs() < 0.01);
    }

    #[test]
    fn intrinsic_measurement_reuses_full_paragraph_without_publishing_geometry() {
        use std::cell::Cell;
        let mut surface = TextSurface::new("Chinese / emoji", true, true);
        surface.presentation = TextSurfacePresentation::Content;
        let calls = Cell::new(0);
        let shape = |_: &str, _: Option<f32>| {
            calls.set(calls.get() + 1);
            Rc::new(empty_paragraph())
        };
        let frame = Rect::new(10., 20., 200., 100.);
        surface.measured_outer_height(200., 2., 1., shape);
        surface.measured_outer_height(200., 2., 1., shape);
        assert_eq!(calls.get(), 1);
        assert!(surface.paragraph.is_none());
        assert_eq!(surface.outer, Rect::default());
        surface.layout_with(frame, None, 2., 1., shape);
        assert_eq!(calls.get(), 1);
        let original = surface.paragraph.clone().unwrap();
        surface.measured_outer_height(200., 2., 1., shape);
        assert_eq!(calls.get(), 1);
        // Speculative measurement does not replace the currently published
        // paragraph/selection geometry; arrangement adopts it only on a match.
        surface.measured_outer_height(220., 2., 1., shape);
        assert_eq!(calls.get(), 2);
        assert!(Rc::ptr_eq(surface.paragraph.as_ref().unwrap(), &original));
        surface.layout_with(frame, None, 2., 1., shape);
        assert_eq!(calls.get(), 2);
        surface.input.text.push('!');
        // Same width as the outstanding measurement, but edited text.
        surface.layout_with(Rect::new(10., 20., 220., 100.), None, 2., 1., shape);
        assert_eq!(calls.get(), 3);
        surface.measured_outer_height(200., 2., 1.5, shape);
        surface.layout_with(frame, None, 2., 1.5, shape);
        assert_eq!(calls.get(), 4);
        surface.set_role(proposal::TextRole::UiHeading);
        surface.layout_with(frame, None, 2., 1.5, shape);
        assert_eq!(calls.get(), 5);
        surface.measured_outer_height(220., 2., 1.5, shape);
        // Measured wrap width and the new arranged wrap width differ.
        surface.layout_with(Rect::new(10., 20., 240., 100.), None, 2., 1.5, shape);
        assert_eq!(calls.get(), 7);
        surface.wrap = false;
        surface.layout_with(frame, None, 2., 1.5, shape);
        assert_eq!(calls.get(), 8);
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
