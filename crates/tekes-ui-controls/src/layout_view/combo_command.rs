use super::{ComponentEvent, FilteredMenuView};
use crate::composite::{MenuEvent, MenuHeader, MenuLayout, MenuRenderContext};
use crate::{
    ButtonLayout, ButtonModel, ButtonSize, ButtonTextAlign, ButtonTextRenderer, ControlKey,
    extended::{ComboBox, CommandPalette},
    paint::ButtonVariant,
};
use tekes_ui_scene::{Point, Rect, Scene, Size};
use tekes_ui_tokens::{Palette, proposal};

#[derive(Debug, PartialEq)]
pub(crate) struct PreparedComboConfirmation {
    selection: crate::extended::ComboSelectionDecision,
    editor: crate::text_input::PreparedTextInsert,
    generation: u64,
    presented: bool,
    pointer: Option<Point>,
}
#[derive(Debug, Default)]
pub struct ComboBoxView {
    pub options: FilteredMenuView,
}
impl PreparedComboConfirmation {
    pub(crate) fn index(&self) -> usize {
        self.selection.index
    }
}
impl ComboBoxView {
    /// No effects: original reducer + editor + presented candidate identity.
    pub(crate) fn prepare_confirmation(
        &self,
        model: &ComboBox,
        input: &crate::TextInput,
        index: usize,
        presented: bool,
    ) -> Option<PreparedComboConfirmation> {
        if input.marked.is_some() || presented && !self.options.is_current(model) {
            return None;
        }
        let selection = model.prepare_selection(index)?;
        let editor = input.prepare_insert(&selection.label, Some(0..input.len()))?;
        Some(PreparedComboConfirmation {
            selection,
            editor,
            generation: self.options.menu_view.open_generation(),
            presented,
            pointer: None,
        })
    }
    pub(crate) fn confirmation_current(
        &self,
        model: &ComboBox,
        input: &crate::TextInput,
        prepared: &PreparedComboConfirmation,
    ) -> bool {
        input.marked.is_none()
            && input.insert_current(&prepared.editor)
            && model.selection_current(&prepared.selection)
            && prepared.generation == self.options.menu_view.open_generation()
            && (!prepared.presented || self.options.is_current(model))
            && prepared.pointer.is_none_or(|p| {
                self.options
                    .menu_view
                    .pointer_release_index(&self.options.menu, p)
                    .and_then(|i| self.options.original_index(i))
                    == Some(prepared.selection.index)
            })
    }
    /// All checks precede effects. No replay of Enter and no fallible write after the first effect.
    pub(crate) fn apply_confirmation(
        &mut self,
        model: &mut ComboBox,
        input: &mut crate::TextInput,
        prepared: PreparedComboConfirmation,
    ) -> Option<usize> {
        if !self.confirmation_current(model, input, &prepared) {
            return None;
        }
        Some(self.commit_confirmation(model, input, prepared))
    }
    pub(crate) fn commit_confirmation(
        &mut self,
        model: &mut ComboBox,
        input: &mut crate::TextInput,
        prepared: PreparedComboConfirmation,
    ) -> usize {
        if let Some(point) = prepared.pointer {
            let _ = self.options.pointer_up(point);
        } else if prepared.presented {
            let filtered = self
                .options
                .mapping
                .iter()
                .position(|i| *i == prepared.selection.index)
                .expect("validated original candidate");
            let _ = self
                .options
                .menu_view
                .activate_item_with_model(&mut self.options.menu, filtered);
        }
        input.apply_insert(prepared.editor);
        let index = model.apply_selection(prepared.selection);
        self.options.dismiss();
        index
    }
    pub(crate) fn prepare_pointer_confirmation(
        &self,
        model: &ComboBox,
        input: &crate::TextInput,
        point: Point,
    ) -> Option<PreparedComboConfirmation> {
        let filtered = self
            .options
            .menu_view
            .pointer_release_index(&self.options.menu, point)?;
        let mut decision =
            self.prepare_confirmation(model, input, self.options.original_index(filtered)?, true)?;
        decision.pointer = Some(point);
        Some(decision)
    }
    pub(crate) fn candidate_pointer_armed(&self) -> bool {
        self.options.menu_view.pointer_armed()
    }
    pub(crate) fn next_open_generation(&self) -> Option<u64> {
        self.options.menu_view.next_open_generation()
    }
    pub(crate) fn apply_open_generation(&mut self, generation: u64) {
        self.options
            .menu_view
            .apply_open_generation(&mut self.options.menu, 0, generation);
    }
    pub(crate) fn revoke_candidates(&mut self) {
        self.options.cancel();
    }
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        model: &mut ComboBox,
        anchor: Rect,
        visible: Rect,
        clip: Option<Rect>,
        palette: Palette,
        role: proposal::TextRole,
        compact: bool,
    ) -> Option<MenuLayout> {
        if model.disabled {
            model.open = false;
            self.options.cancel();
        }
        let map = self.options.mapping_for_model(model);
        let filtered: Vec<String> = map.iter().map(|i| model.items[*i].clone()).collect();
        let active = model.active.and_then(|i| map.iter().position(|x| *x == i));
        self.options.render(
            scene, text, &filtered, &map, active, model.open, anchor, visible, clip, palette, role,
            compact,
        )
    }
    pub fn focus_option(&mut self, original: usize, model: &mut ComboBox) -> bool {
        self.options.focus_item(original, model)
    }
    pub fn activate_option(&mut self, original: usize, model: &mut ComboBox) -> Option<usize> {
        self.options.activate_item(original, model)
    }
    pub fn pointer_move(&mut self, p: Point, model: &mut ComboBox) {
        if self.options.accepts_model(model) && self.options.pointer_move(p) {
            model.active = self.options.active_index();
        }
    }
    pub fn pointer_down(&mut self, p: Point) -> MenuEvent {
        self.options.pointer_down(p)
    }
    pub fn pointer_up(&mut self, p: Point, model: &mut ComboBox) -> Option<usize> {
        if !self.options.accepts_model(model) {
            return None;
        }
        if let MenuEvent::Activated { index, .. } = self.options.pointer_up(p) {
            let original = self.options.original_index(index)?;
            model.active = Some(original);
            let selected = model.key(ControlKey::Enter);
            self.options.dismiss();
            selected
        } else {
            None
        }
    }
    pub fn key_event(&mut self, k: ControlKey, down: bool, model: &mut ComboBox) -> ComponentEvent {
        if model.disabled {
            self.options.cancel();
            return ComponentEvent::default();
        }
        let supported = matches!(
            k,
            ControlKey::Escape | ControlKey::Enter | ControlKey::Up | ControlKey::Down
        );
        let can_open = !model.open && matches!(k, ControlKey::Enter | ControlKey::Down);
        if !supported || (!model.open && !can_open) {
            return ComponentEvent::default();
        }
        let before = (
            model.open,
            model.active,
            model.selected,
            model.query.clone(),
        );
        if down {
            // The model owns the current filtered mapping and active item. A
            // text command can arrive before another render, so routing keys
            // through the last rendered menu would use stale query results.
            let _ = model.key(k);
            if !model.open {
                self.options.dismiss();
            }
        }
        ComponentEvent {
            consumed: true,
            changed: before
                != (
                    model.open,
                    model.active,
                    model.selected,
                    model.query.clone(),
                ),
        }
    }
    pub fn key(&mut self, k: ControlKey, down: bool, model: &mut ComboBox) -> Option<usize> {
        let event = self.key_event(k, down, model);
        // Confirmation is an event, including confirmation of the same value.
        // Query/candidate movement, cancellation and key-up are not selections.
        if down && k == ControlKey::Enter && event.changed && !model.open {
            model.selected
        } else {
            None
        }
    }
}
#[derive(Debug)]
pub struct CommandPaletteView {
    pub options: FilteredMenuView,
    trigger: ButtonModel,
    trigger_layout: Option<ButtonLayout>,
    window_active: bool,
    return_focus: Option<usize>,
}
impl CommandPaletteView {
    /// The caller's Surface affects the original trigger only. The existing
    /// filtered popup keeps its separate semantic Surface and input scope.
    pub fn set_surface(
        &mut self,
        name: &str,
    ) -> Result<(), tekes_ui_tokens::surface_palette::SurfaceError> {
        let surface = tekes_ui_tokens::surface_palette::canonical_surface(name)?;
        self.trigger.set_surface(surface);
        Ok(())
    }
    fn sync_trigger_availability(&mut self, model: &mut CommandPalette) -> bool {
        self.trigger.state.disabled = model.combo.disabled;
        if model.combo.disabled || !self.window_active {
            self.cancel(model);
            false
        } else {
            true
        }
    }
    #[allow(clippy::too_many_arguments)]
    pub fn render_trigger(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        model: &mut CommandPalette,
        label: &str,
        frame: Rect,
        clip: Option<Rect>,
        palette: Palette,
        role: proposal::TextRole,
    ) -> ButtonLayout {
        self.sync_trigger_availability(model);
        self.trigger_layout = Some(self.trigger.render(
            scene,
            text,
            label,
            frame,
            clip,
            palette,
            ButtonVariant::Primary,
            ButtonSize::Body,
            role,
            ButtonTextAlign::Leading,
        ));
        self.trigger_layout.unwrap_or_else(|| ButtonLayout {
            frame: Rect::default(),
            content: Rect::default(),
            visible: None,
            label_size: tekes_ui_scene::Size::default(),
        })
    }
    pub(crate) fn trigger_content_ink(
        &self,
        palette: Palette,
        surface: &str,
    ) -> (tekes_ui_tokens::Rgba, tekes_ui_tokens::Rgba) {
        let colors = self
            .trigger
            .surface_colors(palette, ButtonVariant::Primary, surface);
        (colors.text, colors.icon)
    }
    /// Paint the original trigger action around arbitrary mounted Content.
    /// Layout and semantics remain attached to the same ButtonModel; the
    /// caller measures and arranges its original decorative child.
    /// Pass raw `(frame, ancestor_clip)` together; `None` and an empty clip keep
    /// their original meanings. This replaces the separate frame/clip arguments
    /// introduced with the Content trigger API.
    pub fn render_trigger_content(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        model: &mut CommandPalette,
        content_size: Size,
        frame_and_clip: (Rect, Option<Rect>),
        palette: Palette,
    ) -> ButtonLayout {
        let (frame, clip) = frame_and_clip;
        self.sync_trigger_availability(model);
        self.trigger_layout = Some(self.trigger.render_content_frame(
            scene,
            text,
            content_size,
            frame,
            clip,
            palette,
            ButtonVariant::Primary,
        ));
        self.trigger_layout.unwrap_or_else(|| ButtonLayout {
            frame: Rect::default(),
            content: Rect::default(),
            visible: None,
            label_size: tekes_ui_scene::Size::default(),
        })
    }
    /// Resolve the same custom trigger slot before the host arranges its
    /// original decorative Content child.
    pub fn trigger_content_layout(
        &mut self,
        text: &mut dyn ButtonTextRenderer,
        model: &mut CommandPalette,
        content_size: Size,
        frame: Rect,
        clip: Rect,
    ) -> ButtonLayout {
        self.sync_trigger_availability(model);
        let layout = ButtonLayout::resolve_with_text(
            text,
            frame,
            Some(clip),
            content_size,
            ButtonSize::Body,
            proposal::TextRole::UiBody,
            ButtonTextAlign::Leading,
            self.trigger.frame_policy,
        );
        self.trigger_layout = Some(layout);
        layout
    }
    pub fn pointer_down_trigger(&mut self, p: Point) -> bool {
        if !self.window_active || self.trigger.state.disabled {
            return false;
        }
        self.trigger.pointer_down_at(p)
    }
    pub fn pointer_up_trigger(&mut self, p: Point, model: &mut CommandPalette) -> bool {
        if !self.sync_trigger_availability(model) {
            return false;
        }
        let activated = self.trigger.pointer_up_at(p);
        if activated && self.window_active {
            self.open(model);
            true
        } else {
            false
        }
    }
    pub fn trigger_key(
        &mut self,
        k: ControlKey,
        down: bool,
        model: &mut CommandPalette,
    ) -> ComponentEvent {
        if !self.sync_trigger_availability(model)
            || !self.trigger.state.focused
            || !matches!(k, ControlKey::Enter | ControlKey::Space)
        {
            return ComponentEvent::default();
        }
        let changed = self.trigger.key(k, down);
        if changed {
            self.open(model);
        }
        ComponentEvent {
            consumed: true,
            changed,
        }
    }
    pub fn open(&mut self, model: &mut CommandPalette) {
        if self.sync_trigger_availability(model) {
            self.return_focus = None;
            model.open();
        }
    }
    /// Open from a caller and request transfer to the host-owned query input.
    /// Returns false when unavailable or already opened from a caller; repeated
    /// activation must not reset the live query or replace the original caller.
    /// This also adopts a palette just opened by the trigger methods.
    pub fn open_from(&mut self, model: &mut CommandPalette, invoker: usize) -> bool {
        if !self.sync_trigger_availability(model)
            || (model.combo.open && self.return_focus.is_some())
        {
            return false;
        }
        if !model.combo.open {
            model.open();
        }
        self.return_focus = Some(invoker);
        self.blur_trigger();
        true
    }
    /// Take the caller once after any close path. The host must validate that
    /// this ID is still mounted and enabled before restoring focus. Scope exit
    /// (`blur` or window deactivation) discards the handoff instead.
    pub fn take_restore_focus(&mut self, model: &CommandPalette) -> Option<usize> {
        if model.combo.open {
            None
        } else {
            self.return_focus.take()
        }
    }
    pub fn focus_trigger(&mut self, visible: bool) {
        self.trigger.state.focused = true;
        self.trigger.state.focus_visible = visible;
    }
    /// Transfer focus to the query or another host target without dismissing
    /// the palette. `blur` continues to release the entire component.
    pub fn blur_trigger(&mut self) {
        self.trigger.blur();
    }
    pub fn blur(&mut self, model: &mut CommandPalette) {
        self.blur_trigger();
        self.cancel(model);
        self.return_focus = None;
    }
    pub(crate) fn cancel_pointer_arm(&mut self) {
        self.trigger.cancel_pointer();
        self.options.cancel_pointer_arm();
    }
    pub fn cancel(&mut self, model: &mut CommandPalette) {
        model.combo.open = false;
        self.trigger.cancel();
        self.options.cancel();
    }
    pub fn set_window_active(&mut self, active: bool, model: &mut CommandPalette) {
        self.window_active = active;
        if !active {
            self.blur(model);
        }
    }
    pub fn trigger_layout(&self) -> Option<ButtonLayout> {
        self.trigger_layout
    }
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        model: &mut CommandPalette,
        anchor: Rect,
        visible: Rect,
        clip: Option<Rect>,
        palette: Palette,
        role: proposal::TextRole,
        compact: bool,
    ) -> Option<MenuLayout> {
        self.render_inner(
            MenuRenderContext {
                scene,
                text,
                anchor,
                visible,
                clip,
                palette,
                role,
                compact,
            },
            model,
            None,
        )
    }
    pub(crate) fn render_with_header(
        &mut self,
        context: MenuRenderContext<'_, impl ButtonTextRenderer>,
        model: &mut CommandPalette,
        header: MenuHeader<'_>,
    ) -> Option<MenuLayout> {
        self.render_inner(context, model, Some(header))
    }
    pub(crate) fn header_frame(
        &mut self,
        text: &mut impl ButtonTextRenderer,
        model: &CommandPalette,
        header: Size,
        anchor: Rect,
        viewport: Rect,
    ) -> Rect {
        let map = self.options.mapping_for_model(&model.combo);
        let labels: Vec<&str> = map
            .iter()
            .map(|index| model.combo.items[*index].as_str())
            .collect();
        crate::feedback_view::MenuView::header_frame(
            text,
            &labels,
            header,
            anchor,
            viewport,
            proposal::TextRole::UiBody,
            false,
        )
    }
    fn render_inner(
        &mut self,
        context: MenuRenderContext<'_, impl ButtonTextRenderer>,
        model: &mut CommandPalette,
        header: Option<MenuHeader<'_>>,
    ) -> Option<MenuLayout> {
        self.sync_trigger_availability(model);
        if model.combo.open {
            let _ = self
                .options
                .menu_view
                .set_window_active_with_model(&mut self.options.menu, true);
        }
        let map = self.options.mapping_for_model(&model.combo);
        let items: Vec<String> = map.iter().map(|i| model.combo.items[*i].clone()).collect();
        let active = model
            .combo
            .active
            .and_then(|i| map.iter().position(|x| *x == i));
        self.options
            .render_inner(context, &items, &map, active, model.combo.open, header)
    }
    pub fn key_event(
        &mut self,
        k: ControlKey,
        down: bool,
        model: &mut CommandPalette,
    ) -> ComponentEvent {
        if model.combo.disabled || !self.window_active {
            self.options.cancel();
            return ComponentEvent::default();
        }
        if !model.combo.open
            || !matches!(
                k,
                ControlKey::Escape | ControlKey::Up | ControlKey::Down | ControlKey::Enter
            )
        {
            return ComponentEvent::default();
        }
        let before = (
            model.combo.open,
            model.combo.active,
            model.combo.selected,
            model.combo.query.clone(),
        );
        if down {
            // CommandPalette shares the ComboBox model contract. Resolve from
            // its live query rather than a menu mapping populated by render.
            let _ = model.combo.key(k);
            if !model.combo.open {
                self.options.dismiss();
            }
        }
        ComponentEvent {
            consumed: true,
            changed: before
                != (
                    model.combo.open,
                    model.combo.active,
                    model.combo.selected,
                    model.combo.query.clone(),
                ),
        }
    }
    pub fn key(&mut self, k: ControlKey, down: bool, model: &mut CommandPalette) -> Option<usize> {
        let event = self.key_event(k, down, model);
        // Executing the same command again must still notify the consumer.
        if down && k == ControlKey::Enter && event.changed && !model.combo.open {
            model.combo.selected
        } else {
            None
        }
    }
    pub fn focus_option(&mut self, original: usize, model: &mut CommandPalette) -> bool {
        self.sync_trigger_availability(model) && self.options.focus_item(original, &mut model.combo)
    }
    pub fn activate_option(
        &mut self,
        original: usize,
        model: &mut CommandPalette,
    ) -> Option<usize> {
        if !self.sync_trigger_availability(model) {
            return None;
        }
        self.options.activate_item(original, &mut model.combo)
    }
    pub fn pointer_move(&mut self, p: Point, model: &mut CommandPalette) {
        if !self.sync_trigger_availability(model) {
            if self.window_active {
                // sync_trigger_availability has applied the live disabled
                // state. Re-evaluate hover so a stale enabled hover cannot
                // survive a model update that arrived before the next paint.
                self.trigger.pointer_move_at(p);
            } else {
                self.trigger.blur();
            }
            return;
        }
        self.trigger.pointer_move_at(p);
        if self.options.accepts_model(&model.combo) && self.options.pointer_move(p) {
            model.combo.active = self.options.active_index();
        }
    }
    pub fn pointer_down(&mut self, p: Point) -> MenuEvent {
        self.options.pointer_down(p)
    }
    pub fn pointer_up(&mut self, p: Point, model: &mut CommandPalette) -> Option<usize> {
        if !self.window_active {
            self.options.cancel();
            return None;
        }
        if !self.options.accepts_model(&model.combo) {
            return None;
        }
        if let MenuEvent::Activated { index, .. } = self.options.pointer_up(p) {
            let original = self.options.original_index(index)?;
            model.combo.active = Some(original);
            let selected = model.combo.key(ControlKey::Enter);
            self.options.dismiss();
            selected
        } else {
            None
        }
    }
}
impl Default for CommandPaletteView {
    fn default() -> Self {
        Self {
            options: FilteredMenuView::default(),
            trigger: ButtonModel::default(),
            trigger_layout: None,
            window_active: true,
            return_focus: None,
        }
    }
}
