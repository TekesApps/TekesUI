use super::{
    ButtonLayout, ButtonModel, ButtonSize, ButtonTextAlign, ButtonTextRenderer, ControlKey, Dialog,
    clip_rect, empty_button_layout, focus_reveal_outset, paint, safe_rect, set_button_state,
};
use crate::control_motion::OverlayPresence;
use tekes_ui_scene::{Point, Quad, Rect, Scene};
use tekes_ui_tokens::{Palette, proposal};

#[derive(Clone, Debug, PartialEq)]
pub struct DialogLayout {
    pub preview: Rect,
    pub visible: Option<Rect>,
    pub modal: Rect,
    pub title: Rect,
    pub buttons: [ButtonLayout; 2],
    pub focus: usize,
}

impl Default for DialogLayout {
    fn default() -> Self {
        Self {
            preview: Rect::default(),
            visible: None,
            modal: Rect::default(),
            title: Rect::default(),
            buttons: [empty_button_layout(); 2],
            focus: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DialogEvent {
    #[default]
    None,
    Blocked,
    Cancelled {
        restore_focus: Option<usize>,
    },
    Confirmed {
        restore_focus: Option<usize>,
    },
}

/// Modal presentation for the existing `Dialog` model.
#[derive(Clone, Debug)]
pub struct DialogView {
    presence: OverlayPresence,
    model: Dialog,
    buttons: [ButtonModel; 2],
    placement: DialogLayout,
    return_focus: Option<usize>,
    window_active: bool,
    disabled: bool,
    focus_visible: bool,
    press_target: Option<usize>,
    max_width: Option<f32>,
}

impl Default for DialogView {
    fn default() -> Self {
        let mut buttons = [ButtonModel::default(), ButtonModel::default()];
        for button in &mut buttons {
            button.set_surface("popover");
        }
        Self {
            presence: OverlayPresence::default(),
            model: Dialog::default(),
            buttons,
            placement: DialogLayout::default(),
            return_focus: None,
            window_active: true,
            disabled: false,
            focus_visible: false,
            press_target: None,
            max_width: None,
        }
    }
}

impl DialogView {
    pub fn with_max_width(mut self, max_width: Option<f32>) -> Self {
        self.max_width = max_width;
        self
    }

    pub fn set_max_width(&mut self, max_width: Option<f32>) {
        self.max_width = max_width;
    }
    /// Shared modal surface for default actions and public composed content.
    pub fn paint_content_frame(
        scene: &mut Scene,
        viewport: Rect,
        modal: Rect,
        clip: Rect,
        palette: Palette,
        scale: f32,
    ) {
        scene.push_clip(clip);
        scene.push_quad(Quad::new(
            viewport,
            palette.color("scrim").expect("scrim token"),
        ));
        paint::surface_kind_at_scale(scene, modal, palette, paint::SurfaceKind::Dialog, scale);
        scene.pop_clip();
    }

    /// Paint can outlive semantic open during dismissal.
    pub fn is_presented(&self) -> bool {
        self.presence.is_presented()
    }

    pub fn needs_animation(&self) -> bool {
        self.presence.needs_animation()
    }

    /// Call on unmount or text/image resource replacement.
    pub fn clear_presentation(&mut self) {
        self.presence.clear();
    }

    /// Labels are shared with measurement and painting, including native AX adapters.
    pub fn button_label(index: usize) -> Option<&'static str> {
        ["Cancel", "Confirm"].get(index).copied()
    }

    /// Direct semantic focus of an existing visible action; no synthetic key.
    pub fn focus_button(&mut self, index: usize) -> bool {
        if index >= 2
            || !self.model.open
            || !self.window_active
            || self.disabled
            || self.buttons[index]
                .layout()
                .is_none_or(|l| l.visible.is_none())
        {
            return false;
        }
        let changed = self.model.focus != index || !self.focus_visible;
        self.model.focus = index;
        self.focus_visible = true;
        self.cancel_button_arms_except(index);
        self.sync_focus_buttons();
        changed
    }

    /// Activate the same retained ButtonModel and finish path used by pointer/keyboard.
    /// Closing/hidden/disabled actions cannot produce a second dismissal.
    pub fn activate_button(&mut self, index: usize) -> DialogEvent {
        if index >= 2
            || !self.model.open
            || !self.window_active
            || self.disabled
            || self.buttons[index]
                .layout()
                .is_none_or(|l| l.visible.is_none())
        {
            return DialogEvent::Blocked;
        }
        self.buttons[index].pointer_down();
        if !self.buttons[index].pointer_up(true) {
            return DialogEvent::Blocked;
        }
        self.finish(index == 1)
    }

    pub fn dialog(&self) -> &Dialog {
        &self.model
    }

    pub fn is_open(&self) -> bool {
        self.model.open
    }

    pub fn show(&mut self, invoker: usize) {
        self.model.show(invoker);
        self.return_focus = Some(invoker);
        self.window_active = true;
        self.disabled = false;
        self.focus_visible = false;
        self.cancel_buttons();
        for button in &mut self.buttons {
            button.clear_layout();
        }
        self.sync_focus_buttons();
    }

    pub fn set_window_active(&mut self, active: bool) {
        self.window_active = active;
        if !active {
            self.focus_visible = false;
            self.cancel_buttons();
            self.presence.settle(self.model.open);
            for button in &mut self.buttons {
                button.blur();
            }
        }
    }

    pub fn set_disabled(&mut self, disabled: bool) {
        self.disabled = disabled;
        if disabled {
            self.cancel_buttons();
        }
    }

    #[allow(clippy::too_many_arguments)]
    /// Compatible entry; a returned layout does not imply successful paint.
    /// Renderer rejects a Scene carrying any composition error.
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        title: &str,
        preview: Rect,
        ancestor_clip: Option<Rect>,
        palette: Palette,
        role: proposal::TextRole,
        compact: bool,
    ) -> Option<DialogLayout> {
        match self.try_render(
            scene,
            text,
            title,
            preview,
            ancestor_clip,
            palette,
            role,
            compact,
        ) {
            Ok(layout) => layout,
            Err(error) => {
                scene.record_composition_error(error);
                None
            }
        }
    }
    #[allow(clippy::too_many_arguments)]
    pub fn try_render(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        title: &str,
        preview: Rect,
        ancestor_clip: Option<Rect>,
        palette: Palette,
        role: proposal::TextRole,
        compact: bool,
    ) -> Result<Option<DialogLayout>, tekes_ui_scene::FragmentError> {
        if let Some(error) = scene.composition_error() {
            return Err(error);
        }
        if safe_rect(preview) != preview
            || preview.size.width <= 0.0
            || preview.size.height <= 0.0
            || ancestor_clip.is_some_and(|clip| safe_rect(clip) != clip)
        {
            self.presence.clear();
            self.placement = DialogLayout::default();
            self.cancel_buttons();
            for button in &mut self.buttons {
                button.clear_layout();
            }
            return Ok(None);
        }
        let paint_context = format!(
            "{title}{preview:?}{ancestor_clip:?}{palette:?}{role:?}{compact:?}{}",
            text.backing_scale()
        );
        if !self.model.open && !self.presence.is_presented() {
            self.placement = DialogLayout::default();
            self.presence
                .try_paint(scene, text, false, paint_context, None)?;
            return Ok(None);
        }
        let destination = scene;
        let mut overlay_scene = destination.fragment();
        let scene = &mut overlay_scene;
        let preview = safe_rect(preview);
        let visible = clip_rect(preview, ancestor_clip);
        if visible.is_none() {
            self.presence.clear();
            self.placement = DialogLayout::default();
            self.cancel_buttons();
            for button in &mut self.buttons {
                button.clear_layout();
            }
            return Ok(None);
        }
        let raw_w = (preview.size.width - 16.0).max(0.0);
        let modal_width = self
            .max_width
            .filter(|limit| limit.is_finite())
            .map_or(raw_w, |limit| raw_w.min(limit.max(0.0)));
        let modal_height = preview.size.height.clamp(0.0, 200.0);
        let modal = Rect::new(
            preview.min_x() + (preview.size.width - modal_width) * 0.5,
            preview.min_y() + (preview.size.height - modal_height) * 0.5,
            modal_width,
            modal_height,
        );
        let action_height: f32 = if compact { 30.0 } else { 36.0 };
        let margin = 20.0_f32.min((modal.size.width * 0.5).max(0.0));
        let action_width = (modal.size.width - margin * 2.0).max(0.0);
        let focus_gap = focus_reveal_outset(text.backing_scale()) * 2.0;
        let cancel_natural = ButtonModel::measure(
            text,
            Self::button_label(0).unwrap(),
            f32::MAX,
            ButtonSize::Body,
            role,
        );
        let confirm_natural = ButtonModel::measure(
            text,
            Self::button_label(1).unwrap(),
            f32::MAX,
            ButtonSize::Body,
            role,
        );
        let inline_width = ((action_width - focus_gap).max(0.0)) * 0.5;
        let mut stacked = inline_width < cancel_natural.width.max(confirm_natural.width);
        let vertical_margin = 20.0_f32.min((modal.size.height * 0.5).max(0.0));
        let stacked_natural_height = ButtonModel::measure(
            text,
            Self::button_label(0).unwrap(),
            action_width,
            ButtonSize::Body,
            role,
        )
        .height
        .max(
            ButtonModel::measure(
                text,
                Self::button_label(1).unwrap(),
                action_width,
                ButtonSize::Body,
                role,
            )
            .height,
        );
        if stacked
            && (modal.size.height - vertical_margin * 2.0).max(0.0)
                < stacked_natural_height * 2.0 + focus_gap
        {
            stacked = false;
        }
        let stack_gap = if stacked {
            focus_gap.min((modal.size.height / 4.0).max(0.0))
        } else {
            0.0
        };
        let button_width = if stacked { action_width } else { inline_width };
        let measured_button_height = ButtonModel::measure(
            text,
            Self::button_label(0).unwrap(),
            button_width,
            ButtonSize::Body,
            role,
        )
        .height
        .max(
            ButtonModel::measure(
                text,
                Self::button_label(1).unwrap(),
                button_width,
                ButtonSize::Body,
                role,
            )
            .height,
        );
        let available_action_height = (modal.size.height - vertical_margin * 2.0).max(0.0);
        let maximum_button_height = if stacked {
            (available_action_height - stack_gap).max(0.0) * 0.5
        } else {
            available_action_height
        };
        let action_h = action_height
            .max(measured_button_height)
            .min(maximum_button_height);
        let actions_total = if stacked {
            action_h * 2.0 + stack_gap
        } else {
            action_h
        };
        let actions_top =
            (modal.max_y() - vertical_margin - actions_total).max(modal.min_y() + vertical_margin);
        let button_frame = |index: usize| {
            if stacked {
                Rect::new(
                    modal.min_x() + margin,
                    actions_top + index as f32 * (action_h + stack_gap),
                    action_width,
                    action_h,
                )
            } else {
                Rect::new(
                    modal.min_x() + margin + index as f32 * (button_width + focus_gap),
                    actions_top,
                    button_width,
                    action_h,
                )
            }
        };
        let visible_modal = visible.and_then(|clip| modal.intersection(&clip));
        if let Some(visible) = visible {
            Self::paint_content_frame(
                scene,
                preview,
                modal,
                visible,
                palette,
                text.backing_scale(),
            );
        }
        let title_clip = Rect::new(
            modal.min_x() + margin,
            modal.min_y() + 20.0_f32.min(modal.size.height),
            (modal.size.width - margin * 2.0).max(0.0),
            (actions_top - modal.min_y() - 32.0).max(0.0),
        );
        let title_size = text.measure(title, proposal::TextRole::UiHeading, title_clip.size.width);
        let title_rect = Rect::new(
            title_clip.min_x(),
            title_clip.min_y(),
            title_clip.size.width,
            title_size.height,
        );
        if let Some(visible_modal) = visible_modal
            && let Some(clip) = title_clip.intersection(&visible_modal)
        {
            scene.push_clip(clip);
            text.draw(
                scene,
                title,
                proposal::TextRole::UiHeading,
                title_rect,
                ButtonTextAlign::Leading,
                palette.color("primary-text").expect("primary text token"),
            );
            scene.pop_clip();
        }
        let labels = [
            Self::button_label(0).unwrap(),
            Self::button_label(1).unwrap(),
        ];
        let mut button_layouts = [empty_button_layout(); 2];
        for index in 0..2 {
            let frame = button_frame(index);
            set_button_state(
                &mut self.buttons[index],
                self.window_active && self.model.open && self.model.focus == index,
                self.window_active
                    && self.model.open
                    && self.focus_visible
                    && self.model.focus == index,
                self.disabled,
                false,
            );
            button_layouts[index] = self.buttons[index].render(
                scene,
                text,
                labels[index],
                frame,
                // The action bounds are not a clip. Keep ring antialiasing
                // inside the actual modal and ancestor viewport instead.
                visible_modal,
                palette,
                paint::ButtonVariant::Quiet,
                ButtonSize::Body,
                role,
                ButtonTextAlign::default(),
            );
        }
        self.placement = DialogLayout {
            preview,
            visible,
            modal,
            title: title_rect,
            buttons: button_layouts,
            focus: self.model.focus,
        };
        self.presence.try_paint(
            destination,
            text,
            self.model.open,
            paint_context,
            Some(overlay_scene),
        )?;
        if !self.window_active {
            self.presence.settle(self.model.open);
        }
        if !self.model.open {
            self.placement = DialogLayout::default();
            for button in &mut self.buttons {
                button.clear_layout();
            }
            return Ok(None);
        }
        Ok(Some(self.placement.clone()))
    }

    pub fn pointer_move_at(&mut self, point: Point) {
        if !self.window_active || self.disabled || !self.model.open {
            return;
        }
        for button in &mut self.buttons {
            button.pointer_move_at(point);
        }
    }

    pub fn pointer_down_at(&mut self, point: Point) -> DialogEvent {
        if !self.window_active || self.disabled || !self.model.open {
            return DialogEvent::None;
        }
        if !self
            .placement
            .visible
            .is_some_and(|visible| visible.contains(point))
        {
            return DialogEvent::None;
        }
        if !self.placement.modal.contains(point) {
            self.cancel_buttons();
            return DialogEvent::Blocked;
        }
        if let Some(index) = self
            .placement
            .buttons
            .iter()
            .position(|button| button.hit_test(point))
        {
            self.model.focus = index;
            self.focus_visible = false;
            self.sync_focus_buttons();
            if self.buttons[index].pointer_down_at(point) {
                self.press_target = Some(index);
            }
        } else {
            self.cancel_buttons();
        }
        DialogEvent::Blocked
    }

    pub fn pointer_up_at(&mut self, point: Point) -> DialogEvent {
        if !self.window_active || !self.model.open {
            return DialogEvent::None;
        }
        let target = self.press_target.take();
        let Some(target) = target else {
            return if self
                .placement
                .visible
                .is_some_and(|visible| visible.contains(point))
            {
                DialogEvent::Blocked
            } else {
                DialogEvent::None
            };
        };
        let activated = self.buttons[target].pointer_up_at(point);
        for (index, button) in self.buttons.iter_mut().enumerate() {
            if index != target {
                button.cancel_pointer();
            }
        }
        if activated {
            self.finish(target == 1)
        } else if self
            .placement
            .visible
            .is_some_and(|visible| visible.contains(point))
        {
            DialogEvent::Blocked
        } else {
            DialogEvent::None
        }
    }

    /// A visible modal consumes scrolling anywhere inside its preview so the
    /// event cannot reach the page behind it.
    pub fn wheel_at(&self, point: Point) -> DialogEvent {
        if self.model.open
            && self.window_active
            && self
                .placement
                .visible
                .is_some_and(|visible| visible.contains(point))
        {
            DialogEvent::Blocked
        } else {
            DialogEvent::None
        }
    }

    /// Reports whether this open, active modal blocks background scrolling in
    /// the host that owns its modal scope. The host should call this only after
    /// routing an event belonging to that same window/document.
    pub fn blocks_background(&self) -> bool {
        self.model.open && self.window_active
    }

    pub fn key(&mut self, key: ControlKey, down: bool) -> DialogEvent {
        if !self.window_active || !self.model.open {
            return DialogEvent::None;
        }
        if down && key == ControlKey::Escape {
            return self.finish(false);
        }
        if self.disabled {
            return DialogEvent::None;
        }
        if !down {
            let index = self.model.focus.min(1);
            if self.buttons[index].key(key, false) {
                return self.finish(index == 1);
            }
            return DialogEvent::None;
        }
        match key {
            ControlKey::Tab { reverse } => {
                self.model.tab(reverse, 2);
                self.focus_visible = true;
                self.cancel_button_arms_except(self.model.focus);
                self.sync_focus_buttons();
                DialogEvent::Blocked
            }
            ControlKey::Enter | ControlKey::Space => {
                self.focus_visible = true;
                let index = self.model.focus.min(1);
                if self.buttons[index]
                    .layout()
                    .is_none_or(|layout| layout.visible.is_none())
                {
                    return DialogEvent::Blocked;
                }
                self.sync_focus_buttons();
                if self.buttons[index].key(key, true) {
                    self.finish(index == 1)
                } else {
                    DialogEvent::Blocked
                }
            }
            _ => DialogEvent::Blocked,
        }
    }

    pub fn deactivate(&mut self) {
        // A modal stays open across window deactivation, but cannot keep an
        // activation arm or react to input while the owning window is inactive.
        self.set_window_active(false);
    }

    /// Explicitly closes the modal, including during a page/window teardown.
    /// Unlike `deactivate`, this reports focus restoration and closes the model.
    pub fn close(&mut self) -> DialogEvent {
        self.cancel()
    }

    pub fn cancel(&mut self) -> DialogEvent {
        if self.model.open {
            self.finish(false)
        } else {
            DialogEvent::None
        }
    }

    fn finish(&mut self, confirmed: bool) -> DialogEvent {
        let model_restore_focus = self.model.close();
        let restore_focus = self.return_focus.take().or(model_restore_focus);
        self.model.open = false;
        self.cancel_buttons();
        for button in &mut self.buttons {
            button.clear_layout();
        }
        self.placement = DialogLayout::default();
        if confirmed {
            DialogEvent::Confirmed { restore_focus }
        } else {
            DialogEvent::Cancelled { restore_focus }
        }
    }

    fn cancel_buttons(&mut self) {
        self.press_target = None;
        for button in &mut self.buttons {
            button.cancel();
        }
    }

    fn sync_focus_buttons(&mut self) {
        for (index, button) in self.buttons.iter_mut().enumerate() {
            let focused = self.window_active && self.model.open && index == self.model.focus;
            if button.state.focused && !focused {
                button.state.focused = false;
                button.state.focus_visible = false;
                let _ = button.key(ControlKey::Space, false);
            }
            if focused {
                button.state.focused = true;
                button.state.focus_visible = self.focus_visible;
            }
        }
    }

    fn cancel_button_arms_except(&mut self, except: usize) {
        for (index, button) in self.buttons.iter_mut().enumerate() {
            if index != except {
                button.cancel();
            }
        }
    }
}
