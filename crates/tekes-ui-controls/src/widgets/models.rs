use super::{
    ButtonActionDecision, ButtonLayout, ButtonModel, ButtonTextAlign, ButtonTextRenderer,
    ControlKey, expand_for_focus, finite_nonnegative, valid_clip, valid_frame,
};
use crate::{ContainerLayout, Insets};
use tekes_ui_scene::{Point, Rect, Scene, Size};
use tekes_ui_tokens::{Palette, proposal};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CheckState {
    #[default]
    Unchecked,
    Checked,
    Mixed,
}
#[derive(Clone, Debug, Default)]
pub struct Checkbox {
    pub button: ButtonModel,
    pub checked: CheckState,
}
/// The original Checkbox's decision, authenticated against its ButtonModel and value.
#[derive(Debug, PartialEq)]
pub(crate) struct CheckboxActionDecision {
    button: ButtonActionDecision,
    expected_checked: CheckState,
    checked: CheckState,
    preserve_focus: bool,
    pub consumed: bool,
    pub activated: bool,
}
impl Checkbox {
    pub(crate) fn prepare_content_down(&self, point: Point) -> CheckboxActionDecision {
        let button = self.button.prepare_content_down(point, false);
        CheckboxActionDecision {
            consumed: button.consumed,
            activated: false,
            button,
            expected_checked: self.checked,
            checked: self.checked,
            preserve_focus: false,
        }
    }
    pub(crate) fn prepare_content_press(&self) -> CheckboxActionDecision {
        let checked = self.next_checked();
        CheckboxActionDecision {
            button: self.button.prepare_content_cancel(),
            expected_checked: self.checked,
            checked: checked.unwrap_or(self.checked),
            preserve_focus: true,
            consumed: checked.is_some(),
            activated: checked.is_some(),
        }
    }
    pub(crate) fn apply_content_action(&mut self, decision: CheckboxActionDecision) {
        let focus = (self.button.state.focused, self.button.state.focus_visible);
        self.button.apply_content_action(decision.button);
        // The prepared cancellation changes activation only. Preserve the original
        // Host's already-admitted AX focus hook rather than replaying any handler.
        if decision.preserve_focus {
            self.button.state.focused = focus.0;
            self.button.state.focus_visible = focus.1;
        }
        self.checked = decision.checked;
    }
    fn next_checked(&self) -> Option<CheckState> {
        if self.button.state.disabled || self.button.state.loading {
            return None;
        }
        Some(if self.checked == CheckState::Checked {
            CheckState::Unchecked
        } else {
            CheckState::Checked
        })
    }
    /// Measure the complete checkbox row. The visual box is 16pt; the label
    /// wraps inside the space remaining after that box and the 6pt label gap.
    pub fn measure(
        text: &mut impl ButtonTextRenderer,
        label: &str,
        max_width: f32,
        role: proposal::TextRole,
    ) -> Size {
        let width = finite_nonnegative(max_width);
        let control_size = checkbox_control_size();
        let label_gap = checkbox_label_gap();
        let label_width = (width - control_size - label_gap).max(0.0);
        let (label_size, first_line_height) = if label.is_empty() || label_width == 0.0 {
            (Size::default(), 0.0)
        } else {
            let first_line_height =
                finite_nonnegative(text.first_line_height(label, role, label_width));
            let label_size = text.measure(label, role, label_width);
            (label_size, first_line_height)
        };
        let (content_height, _, _) = checkbox_content_geometry(
            first_line_height,
            finite_nonnegative(label_size.height),
            !label.is_empty() && label_width > 0.0,
        );
        Size::new(width, checkbox_row_min_height().max(content_height + 12.0))
    }

    /// Paint and place the complete checkbox row. Its outer placement is also
    /// installed on the shared ButtonModel interaction model, so pointer and
    /// keyboard behavior use the same clipped geometry as drawing.
    // Like ButtonModel::render, this adapts layout, state, drawing, and text shaping.
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        label: &str,
        frame: Rect,
        clip: Option<Rect>,
        palette: impl Into<Palette>,
        role: proposal::TextRole,
    ) -> CheckboxLayout {
        self.render_with_surface(scene, text, label, frame, clip, palette, role, "canvas")
    }

    /// Render on an inherited semantic Surface. Reject invalid names before placement or capture changes.
    #[allow(clippy::too_many_arguments)]
    pub fn render_on_surface(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        label: &str,
        frame: Rect,
        clip: Option<Rect>,
        palette: impl Into<Palette>,
        role: proposal::TextRole,
        surface: &str,
    ) -> Result<CheckboxLayout, tekes_ui_tokens::surface_palette::SurfaceError> {
        let surface = tekes_ui_tokens::surface_palette::canonical_surface(surface)?;
        Ok(self.render_with_surface(scene, text, label, frame, clip, palette, role, surface))
    }

    #[allow(clippy::too_many_arguments)]
    fn render_with_surface(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        label: &str,
        frame: Rect,
        clip: Option<Rect>,
        palette: impl Into<Palette>,
        role: proposal::TextRole,
        surface: &str,
    ) -> CheckboxLayout {
        let prepared = self.prepare_layout(text, label, frame, clip, role);
        self.paint_prepared(scene, text, label, prepared, palette.into(), role, surface);
        prepared.layout
    }

    /// The original placement/reconciliation pass, shared with Content arrangement.
    pub(crate) fn prepare_layout(
        &mut self,
        text: &mut impl ButtonTextRenderer,
        label: &str,
        frame: Rect,
        clip: Option<Rect>,
        role: proposal::TextRole,
    ) -> CheckboxPrepared {
        if !valid_frame(frame) || clip.is_some_and(|clip| !valid_clip(clip)) {
            self.button.placement = Some(ButtonLayout::empty());
            self.button.blur();
            return CheckboxPrepared::default();
        }
        let frame_width = finite_nonnegative(frame.size.width);
        let control_size = checkbox_control_size();
        let label_gap = checkbox_label_gap();
        let label_width = (frame_width - control_size - label_gap).max(0.0);
        let has_label = !label.is_empty() && label_width > 0.0;
        let first_line_height = if has_label {
            finite_nonnegative(text.first_line_height(label, role, label_width))
        } else {
            0.0
        };
        let measured = if !has_label {
            Size::default()
        } else {
            text.measure(label, role, label_width)
        };
        let measured = Size::new(
            finite_nonnegative(measured.width),
            finite_nonnegative(measured.height),
        );
        let (content_height, control_content_top, label_content_top) =
            checkbox_content_geometry(first_line_height, measured.height, has_label);
        let height = finite_nonnegative(frame.size.height)
            .max(checkbox_row_min_height().max(content_height + 12.0));
        let content_top = (height - content_height).max(0.0) * 0.5;
        let container = ContainerLayout::new(
            Rect::new(frame.min_x(), frame.min_y(), frame_width, height),
            Insets::ZERO,
            Insets::ZERO,
            clip,
        );
        let frame = container.frame();
        let control = Rect::new(
            frame.min_x(),
            frame.min_y() + content_top + control_content_top,
            control_size,
            control_size,
        );
        let label_slot = if has_label {
            Rect::new(
                frame.min_x() + control_size + label_gap,
                frame.min_y() + content_top + label_content_top,
                label_width,
                measured.height,
            )
        } else {
            Rect::new(
                frame.min_x() + control_size + label_gap,
                frame.min_y(),
                0.0,
                0.0,
            )
        };
        let layout = CheckboxLayout {
            frame,
            control,
            label: label_slot,
            visible: container.visible_clip(),
        };
        self.button.placement = Some(ButtonLayout {
            frame,
            content: label_slot,
            visible: layout.visible,
            label_size: measured,
        });
        self.button.synchronize_activation();
        self.button.refresh_pointer_state();

        let control_fits_row = control.min_x() >= frame.min_x()
            && control.min_y() >= frame.min_y()
            && control.max_x() <= frame.max_x()
            && control.max_y() <= frame.max_y();
        let control_clip = if control_fits_row {
            container.background_paint_clip()
        } else {
            container.visible_clip()
        };
        if layout.visible.is_none() {
            self.button.blur();
        }
        CheckboxPrepared {
            layout,
            control_clip,
        }
    }

    /// Paint only the original prepared geometry; never replay placement or input reconciliation.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn paint_prepared(
        &self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        label: &str,
        prepared: CheckboxPrepared,
        palette: Palette,
        role: proposal::TextRole,
        surface: &str,
    ) {
        let layout = prepared.layout;
        let control = layout.control;
        let label_slot = layout.label;
        if let Some(visible) = layout.visible {
            let control_clip = prepared.control_clip;
            if let Some(control_clip) = control_clip {
                scene.push_clip(control_clip);
            }
            let _mark_ink = crate::paint::checkbox_on_surface(
                scene,
                control,
                palette,
                self.button.state,
                self.checked,
                surface,
            );
            if control_clip.is_some() {
                scene.pop_clip();
            }
            let label_ink = crate::paint::choice_label_color(palette, surface, self.button.state);
            if !label_slot.is_empty() {
                scene.push_clip(visible);
                text.draw(
                    scene,
                    label,
                    role,
                    label_slot,
                    ButtonTextAlign::Leading,
                    label_ink,
                );
                scene.pop_clip();
            }
        }
    }

    pub fn layout(&self) -> Option<ButtonLayout> {
        self.button.layout()
    }

    pub fn clear_layout(&mut self) {
        self.button.clear_layout();
    }

    pub fn pointer_move_at(&mut self, point: Point) {
        self.button.pointer_move_at(point);
    }

    pub fn pointer_down_at(&mut self, point: Point) -> bool {
        let decision = self.prepare_content_down(point);
        let consumed = decision.consumed;
        self.apply_content_action(decision);
        consumed
    }

    pub fn pointer_up_at(&mut self, point: Point) -> bool {
        let activated = self.button.pointer_up_at(point);
        if activated {
            self.toggle();
        }
        activated
    }

    pub fn key(&mut self, key: ControlKey, down: bool) -> bool {
        // Every observed key must synchronize the shared activation owner,
        // including keys this checkbox leaves to its parent.
        let activated = self.button.key(key, down);
        if key == ControlKey::Space && activated {
            self.toggle()
        } else {
            false
        }
    }

    pub fn toggle(&mut self) -> bool {
        let Some(checked) = self.next_checked() else {
            return false;
        };
        self.checked = checked;
        true
    }
}

fn checkbox_control_size() -> f32 {
    proposal::metric("size", "c-checkbox-size").expect("checkbox visual size")
}

fn checkbox_label_gap() -> f32 {
    proposal::metric("spacing", "c-checkbox-label-gap").expect("checkbox label gap")
}

fn checkbox_row_min_height() -> f32 {
    proposal::metric("size", "c-checkbox-row-min").expect("checkbox hit row")
}

fn checkbox_content_geometry(
    first_line_height: f32,
    label_height: f32,
    has_label: bool,
) -> (f32, f32, f32) {
    let control_size = checkbox_control_size();
    if !has_label {
        return (control_size, 0.0, 0.0);
    }
    let first_line_height = finite_nonnegative(first_line_height);
    let label_height = finite_nonnegative(label_height);
    let label_top = (control_size - first_line_height).max(0.0) * 0.5;
    let control_top = (first_line_height - control_size).max(0.0) * 0.5;
    let content_height = (label_top + label_height).max(control_top + control_size);
    (content_height, control_top, label_top)
}

/// One result of the original checkbox arrangement. No second layout owner.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct CheckboxPrepared {
    pub(crate) layout: CheckboxLayout,
    control_clip: Option<Rect>,
}

/// Geometry for a checkbox's full hit row, visual mark, label, and visible clip.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CheckboxLayout {
    pub frame: Rect,
    pub control: Rect,
    pub label: Rect,
    pub visible: Option<Rect>,
}

impl CheckboxLayout {
    pub fn hit_test(&self, point: Point) -> bool {
        self.visible.is_some_and(|rect| rect.contains(point))
    }

    /// Bounds including the Design System focus ring, for host focus reveal.
    pub fn focus_bounds(&self) -> Rect {
        expand_for_focus(self.frame)
    }
}
#[derive(Clone, Debug)]
pub struct Tabs {
    pub selected: usize,
    pub count: usize,
}
impl Tabs {
    pub fn new(count: usize) -> Self {
        Self { selected: 0, count }
    }
    pub fn select(&mut self, i: usize) -> bool {
        if i >= self.count || i == self.selected {
            return false;
        }
        self.selected = i;
        true
    }
    pub fn key(&mut self, key: ControlKey) -> bool {
        if self.count == 0 {
            return false;
        }
        let n = match key {
            ControlKey::Left => (self.selected + self.count - 1) % self.count,
            ControlKey::Right => (self.selected + 1) % self.count,
            ControlKey::Home => 0,
            ControlKey::End => self.count - 1,
            _ => return false,
        };
        self.select(n)
    }
}
#[derive(Clone, Debug)]
pub struct Menu {
    pub open: bool,
    pub items: Tabs,
    pub last_action: Option<usize>,
}
impl Menu {
    pub fn new(count: usize) -> Self {
        Self {
            open: false,
            items: Tabs::new(count),
            last_action: None,
        }
    }
    pub fn show(&mut self) {
        self.open = true;
        self.items.selected = 0;
    }
    pub fn key(&mut self, key: ControlKey) -> Option<usize> {
        if !self.open {
            return None;
        }
        match key {
            ControlKey::Escape => self.open = false,
            ControlKey::Enter | ControlKey::Space if self.items.count > 0 => {
                let i = self.items.selected;
                self.last_action = Some(i);
                self.open = false;
                return Some(i);
            }
            ControlKey::Up => {
                self.items.key(ControlKey::Left);
            }
            ControlKey::Down => {
                self.items.key(ControlKey::Right);
            }
            ControlKey::Home | ControlKey::End => {
                self.items.key(key);
            }
            _ => {}
        }
        None
    }
}
#[derive(Clone, Debug, Default)]
pub struct Dialog {
    pub open: bool,
    pub focus: usize,
    return_focus: Option<usize>,
}
impl Dialog {
    pub fn show(&mut self, invoker: usize) {
        self.open = true;
        self.focus = 0;
        self.return_focus = Some(invoker);
    }
    pub fn close(&mut self) -> Option<usize> {
        self.open = false;
        self.return_focus.take()
    }
    pub fn tab(&mut self, reverse: bool, count: usize) {
        if self.open && count > 0 {
            self.focus = if reverse {
                (self.focus + count - 1) % count
            } else {
                (self.focus + 1) % count
            };
        }
    }
}
#[derive(Clone, Debug)]
pub struct Select {
    pub menu: Menu,
    pub selected: usize,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SelectChoice {
    index: usize,
    expected: (usize, usize),
}
impl Select {
    pub fn new(count: usize) -> Self {
        Self {
            menu: Menu::new(count),
            selected: 0,
        }
    }
    pub(crate) fn prepare_choice(&self, index: usize) -> Option<SelectChoice> {
        (index < self.menu.items.count).then_some(SelectChoice {
            index,
            expected: (self.selected, self.menu.items.count),
        })
    }
    pub(crate) fn apply_choice(&mut self, choice: SelectChoice) {
        self.selected = choice.index;
        self.menu.open = false;
    }
    pub fn choose(&mut self, i: usize) -> bool {
        let Some(choice) = self.prepare_choice(i) else {
            return false;
        };
        self.apply_choice(choice);
        true
    }
}
#[derive(Clone, Copy, Debug)]
pub struct NumberField {
    value: f64,
    min: f64,
    max: f64,
    step: f64,
}
impl NumberField {
    pub fn new(value: f64, min: f64, max: f64, step: f64) -> Option<Self> {
        if ![value, min, max, step].iter().all(|v| v.is_finite()) || min > max || step <= 0.0 {
            return None;
        }
        Some(Self {
            value: value.clamp(min, max),
            min,
            max,
            step,
        })
    }
    pub fn value(self) -> f64 {
        self.value
    }
    /// Whether one configured step can change the value in the requested direction.
    pub fn can_increment(self, up: bool) -> bool {
        if up {
            self.value < self.max
        } else {
            self.value > self.min
        }
    }
    pub fn set(&mut self, value: f64) -> bool {
        if !value.is_finite() {
            return false;
        }
        let next = value.clamp(self.min, self.max);
        let changed = next != self.value;
        self.value = next;
        changed
    }
    pub fn increment(&mut self, up: bool) -> bool {
        self.set((self.value + if up { self.step } else { -self.step }).clamp(self.min, self.max))
    }
}
/// Semantic field metadata, separate from the input's editing model.
#[derive(Clone, Debug, Default)]
pub struct Field {
    pub label: String,
    pub help: String,
    pub error: Option<String>,
    pub required: bool,
}
impl Field {
    pub fn message(&self) -> &str {
        self.error.as_deref().unwrap_or(&self.help)
    }
}

/// Toolbar geometry is shared by clients; narrow bounds wrap rather than overflow.
#[derive(Clone, Copy, Debug, Default)]
pub struct Toolbar;
impl Toolbar {
    pub fn layout(
        bounds: tekes_ui_scene::Rect,
        count: usize,
        gap: f32,
    ) -> Vec<tekes_ui_scene::Rect> {
        if count == 0 {
            return Vec::new();
        }
        let width = ((bounds.size.width - gap * (count - 1) as f32) / count as f32).max(0.);
        (0..count)
            .map(|i| {
                tekes_ui_scene::Rect::new(
                    bounds.min_x() + i as f32 * (width + gap),
                    bounds.min_y(),
                    width,
                    bounds.size.height,
                )
            })
            .collect()
    }
}
