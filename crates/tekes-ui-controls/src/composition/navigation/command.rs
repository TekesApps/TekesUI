//! Command palette composition over the original command, text, and menu owners.
use super::*;
use crate::extended::CommandPalette;
use crate::layout_view::CommandPaletteView;

/// A command palette whose query remains an original `TextFieldContent`.
///
/// The trigger Content is decorative: the existing CommandPaletteView owns its
/// action and presentation. The query remains mounted while closed, preserving
/// its TextSurface and native text identity, but is excluded from layout input,
/// focus, painting, and semantics until the palette opens.
#[derive(Debug)]
pub struct CommandPaletteContent {
    pub model: CommandPalette,
    pub view: CommandPaletteView,
    children: Vec<Content>,
    trigger_height: f32,
    trigger_content_size: Size,
    query_size: Size,
    trigger_frame: Rect,
    query_frame: Rect,
    query_clip: Rect,
    trigger_layout: Option<crate::ButtonLayout>,
    last_frame: Option<std::time::Duration>,
    last_selection: Option<usize>,
}

impl CommandPaletteContent {
    /// `trigger` is display-only content. `query` must be the original editable
    /// TextFieldContent so text services, selection, and IME keep one owner.
    pub fn new(trigger: Content, query: Content, model: CommandPalette) -> Self {
        let mut trigger = trigger;
        trigger.suppress_semantics();
        let mut query = query;
        query.set_composition_active(false);
        Self {
            model,
            view: CommandPaletteView::default(),
            children: vec![trigger, query],
            trigger_height: 0.,
            trigger_content_size: Size::default(),
            query_size: Size::default(),
            trigger_frame: Rect::default(),
            query_frame: Rect::default(),
            query_clip: Rect::default(),
            trigger_layout: None,
            last_frame: None,
            last_selection: None,
        }
    }

    pub fn is_open(&self) -> bool {
        self.model.combo.open
    }

    pub fn options(&self) -> &crate::layout_view::FilteredMenuView {
        &self.view.options
    }

    fn query_content(&self) -> &Content {
        &self.children[1]
    }

    fn query_content_mut(&mut self) -> &mut Content {
        &mut self.children[1]
    }

    fn query_text(&self) -> &TextFieldContent {
        self.query_content()
            .component::<TextFieldContent>()
            .expect("CommandPaletteContent query is validated")
    }

    fn query_text_mut(&mut self) -> &mut TextFieldContent {
        self.query_content_mut()
            .component_mut::<TextFieldContent>()
            .expect("CommandPaletteContent query is validated")
    }

    fn sync_model_query(&mut self) {
        let query = self.query_text().surface.input.text.clone();
        if self.model.combo.query != query {
            self.model.combo.query = query;
            if self.model.combo.open {
                self.model.combo.active = self.model.combo.filtered().first().copied();
            }
        }
    }

    fn sync_query_text(&mut self) {
        let value = self.model.combo.query.clone();
        let field = self.query_text_mut();
        if field.surface.input.text != value {
            let len = field.surface.input.len();
            field.surface.input.insert(&value, Some(0..len));
            field.surface.clauses.clear();
            field.surface.reveal();
        }
    }

    pub fn open(&mut self) -> bool {
        if self.model.combo.disabled || self.model.combo.open || self.query_text().composing() {
            return false;
        }
        self.last_selection = None;
        self.view.open(&mut self.model);
        self.sync_query_text();
        self.sync_child_activation();
        true
    }

    pub fn close(&mut self) -> bool {
        if !self.model.combo.open || self.query_text().composing() {
            return false;
        }
        self.last_selection = None;
        self.model.combo.open = false;
        self.view.options.dismiss();
        self.view.cancel_pointer_arm();
        self.sync_child_activation();
        true
    }

    fn sync_child_activation(&mut self) {
        self.children[1].set_composition_active(self.model.combo.open);
    }

    fn sync_live_state(&mut self) {
        if self.model.combo.disabled {
            self.view.cancel(&mut self.model);
        }
        self.sync_child_activation();
    }

    fn popup_contains(&self, position: Point) -> bool {
        self.view.options.layout().is_some_and(|layout| {
            layout.popup.contains(position) && !self.query_frame.contains(position)
        })
    }

    fn query_contains(&self, position: Point) -> bool {
        self.query_frame.contains(position)
    }

    fn accept_selection(&mut self, selected: Option<usize>) -> ContentResponse {
        if let Some(index) = selected {
            self.last_selection = Some(index);
            self.sync_query_text();
            self.sync_child_activation();
            ContentResponse {
                consumed: true,
                changed: true,
                activated: true,
                ..Default::default()
            }
        } else {
            self.last_selection = None;
            self.sync_child_activation();
            ContentResponse::default()
        }
    }

    fn popup_pointer(&mut self, phase: PointerPhase, position: Point) -> ContentResponse {
        match phase {
            PointerPhase::Down if self.popup_contains(position) => {
                self.last_selection = None;
                let _ = self.view.pointer_down(position);
                ContentResponse {
                    consumed: true,
                    capture_pointer: true,
                    ..Default::default()
                }
            }
            PointerPhase::Up => {
                if self.query_text().composing() {
                    return ContentResponse::default();
                }
                let selected = self.view.pointer_up(position, &mut self.model);
                self.accept_selection(selected)
            }
            PointerPhase::Move | PointerPhase::Drag => {
                if self.popup_contains(position) {
                    self.view.pointer_move(position, &mut self.model);
                    ContentResponse::consumed(false)
                } else {
                    ContentResponse::default()
                }
            }
            _ => ContentResponse::default(),
        }
    }

    fn command_label(&self) -> String {
        let name = self.children[0].component.semantics().name;
        if name.trim().is_empty() {
            "Commands".into()
        } else {
            name
        }
    }

    fn render_trigger(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        theme: ContentTheme,
    ) {
        self.view
            .set_surface(theme.surface())
            .expect("validated inherited trigger Surface");
        let mut text = FrameTextAdapter(text, None, true);
        self.trigger_layout = Some(
            self.view.render_trigger_content(
                scene,
                &mut text,
                &mut self.model,
                self.trigger_content_size,
                (
                    self.trigger_frame,
                    Some(
                        self.trigger_frame
                            .intersection(&self.query_clip)
                            .unwrap_or_default(),
                    ),
                ),
                theme.palette,
            ),
        );
    }
}

impl Component for CommandPaletteContent {
    fn validate(&self) -> Result<(), CompositionError> {
        if self.children.len() != 2 {
            return Err(CompositionError::InvalidSlotCount);
        }
        validate_display_slot(&self.children[0])?;
        let Some(query) = self.children[1].component::<TextFieldContent>() else {
            return Err(CompositionError::InvalidSlotCount);
        };
        if query.surface.input.disabled || query.surface.input.read_only {
            return Err(CompositionError::InvalidSlotCount);
        }
        Ok(())
    }

    fn measure(&mut self, text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        self.sync_live_state();
        let trigger = self.children[0].measure_intrinsic(text, available);
        self.trigger_content_size = trigger.size;
        let trigger = crate::ButtonModel::measure_content(
            text,
            trigger.size,
            available.width,
            crate::ButtonSize::Body,
            proposal::TextRole::UiBody,
        );
        self.trigger_height = trigger.height.max(0.);
        MeasuredChild::new(Size::new(
            trigger.width.min(available.width),
            self.trigger_height.min(available.height),
        ))
    }

    fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        self.measure(text, frame.size);
        let trigger_height = self.trigger_height.min(frame.size.height);
        self.trigger_frame = Rect::new(
            frame.min_x(),
            frame.min_y(),
            frame.size.width,
            trigger_height,
        );
        // The query belongs to the floating menu allocation, not the narrow
        // ordinary trigger slot. Both layout and painting use MenuRuntime's
        // shared header geometry and the existing viewport placement rules.
        self.sync_model_query();
        let width = crate::MenuView::header_content_width(
            &mut FrameTextAdapter(text, None, true),
            clip.size.width,
        );
        self.query_size = self.children[1]
            .measure(text, Size::new(width, clip.size.height))
            .size;
        self.query_frame = self.view.header_frame(
            &mut FrameTextAdapter(text, None, true),
            &self.model,
            self.query_size,
            self.trigger_frame,
            clip,
        );
        self.query_clip = clip;
        let layout = self.view.trigger_content_layout(
            text,
            &mut self.model,
            self.trigger_content_size,
            self.trigger_frame,
            clip,
        );
        self.trigger_layout = Some(layout);
        self.children[0].arrange(
            text,
            layout.content,
            layout.content.intersection(&clip).unwrap_or_default(),
        );
        // Retain the original ancestor clip so the field's focus decoration
        // can use the popup's existing inset outside its content allocation.
        self.children[1].arrange(text, self.query_frame, clip);
    }

    fn paint(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        _frame: Rect,
        _clip: Rect,
        theme: ContentTheme,
    ) {
        self.render_trigger(scene, text, theme);
        let (foreground, icon_foreground) = self
            .view
            .trigger_content_ink(theme.palette, theme.surface());
        self.children[0].paint(
            scene,
            text,
            ContentTheme {
                foreground: Some(foreground),
                icon_foreground: Some(icon_foreground),
                ..theme
            },
        );
    }

    fn paint_overlay(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        _frame: Rect,
        clip: Rect,
        theme: ContentTheme,
    ) {
        if !self.model.combo.open && !self.view.options.is_presented() {
            return;
        }
        if self.model.combo.open {
            self.sync_model_query();
        }
        if let Some(context) = text.motion_context() {
            let dt = self
                .last_frame
                .replace(context.now)
                .map(|previous| context.now.saturating_sub(previous).as_secs_f32());
            self.view.options.set_reduced_motion(context.reduced_motion);
            if let Some(dt) = dt {
                self.view.options.tick(dt);
            }
        }
        let query = &mut self.children[1];
        let open = self.model.combo.open;
        let popup_theme = theme.with_surface("popover").expect("valid popup Surface");
        let mut paint_query =
            |scene: &mut Scene, text: &mut dyn ButtonTextRenderer, frame: Rect| {
                debug_assert_eq!(frame, query.frame());
                if open {
                    query.paint(scene, text, popup_theme);
                    query.paint_overlays(scene, text, popup_theme);
                } else {
                    // Retained exit belongs to this original menu Presence. The
                    // closed query stays absent from routing/AX; only fresh paint
                    // of its original field is submitted, never an active subtree.
                    let resolved = query.resolved_theme(popup_theme);
                    query
                        .component
                        .paint(scene, text, frame, query.clip, resolved);
                }
            };
        let _ = self.view.render_with_header(
            crate::composite::MenuRenderContext {
                scene,
                text: &mut FrameTextAdapter(text, None, true),
                anchor: self.trigger_frame,
                visible: clip,
                clip: Some(clip),
                palette: theme.palette,
                role: proposal::TextRole::UiBody,
                compact: false,
            },
            &mut self.model,
            (self.query_size, &mut paint_query),
        );
        if self.view.options.needs_animation() {
            text.request_animation_frame();
        }
    }

    fn children(&self) -> &[Content] {
        &self.children
    }

    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.children)
    }

    fn fixed_slots(&self) -> bool {
        true
    }

    fn requires_text_services(&self) -> bool {
        // Exit presentation still rebuilds this original field's glyphs;
        // inactivity removes input authority, not its paint capability need.
        self.overlay_presented() && self.query_text().requires_text_services()
    }

    fn paints_children(&self) -> bool {
        true
    }

    fn semantics(&self) -> ContentSemantics {
        let mut semantics = ContentSemantics::new(ContentRole::Group, self.command_label());
        semantics.disabled = self.model.combo.disabled;
        semantics.expanded = Some(self.model.combo.open);
        semantics
    }

    fn semantic_parts(&self) -> Vec<SemanticPart> {
        let mut parts = Vec::new();
        if !self.model.combo.open
            && let Some(layout) = self.trigger_layout
        {
            let mut semantics =
                ContentSemantics::new(ContentRole::MenuButton, self.command_label());
            semantics.disabled = self.model.combo.disabled;
            semantics.expanded = Some(self.model.combo.open);
            parts.push(SemanticPart {
                index: usize::MAX,
                frame: layout.frame,
                focused: false,
                semantics,
            });
        }
        if self.view.options.is_current(&self.model.combo)
            && let Some(layout) = self.view.options.layout()
        {
            for (filtered, button) in layout.items.iter().enumerate() {
                if button.visible.is_none_or(|visible| visible.is_empty()) {
                    continue;
                }
                let Some(index) = self.view.options.original_index(filtered) else {
                    continue;
                };
                let Some(label) = self.model.combo.items.get(index) else {
                    continue;
                };
                let mut semantics = ContentSemantics::new(ContentRole::MenuItem, label);
                semantics.selected = self.model.combo.active == Some(index);
                parts.push(SemanticPart {
                    index,
                    frame: button.frame,
                    focused: self.model.combo.active == Some(index),
                    semantics,
                });
            }
        }
        parts
    }

    fn accessibility_action(
        &mut self,
        part: Option<usize>,
        action: &AccessibilityAction,
    ) -> ContentResponse {
        self.last_selection = None;
        if part.is_some_and(|index| !self.semantic_parts().iter().any(|part| part.index == index)) {
            return ContentResponse::default();
        }
        match (part, action) {
            (Some(usize::MAX), AccessibilityAction::Press) if !self.model.combo.open => {
                let changed = self.open();
                ContentResponse {
                    consumed: changed,
                    changed,
                    activated: changed,
                    ..Default::default()
                }
            }
            (Some(usize::MAX), AccessibilityAction::Focus) => {
                self.view.focus_trigger(true);
                ContentResponse::consumed(false)
            }
            (Some(index), AccessibilityAction::Focus) if index != usize::MAX => {
                let changed = self.view.focus_option(index, &mut self.model);
                ContentResponse::consumed(changed)
            }
            (Some(index), AccessibilityAction::Press) if index != usize::MAX => {
                if self.query_text().composing() {
                    return ContentResponse::default();
                }
                let selected = self.view.activate_option(index, &mut self.model);
                self.accept_selection(selected)
            }
            (None, AccessibilityAction::Press) if !self.model.combo.open => {
                let changed = self.open();
                ContentResponse {
                    consumed: changed,
                    changed,
                    activated: changed,
                    ..Default::default()
                }
            }
            _ => ContentResponse::default(),
        }
    }

    fn focusable(&self) -> bool {
        !self.model.combo.disabled && !self.model.combo.open
    }

    fn set_focus(&mut self, focused: bool, visible: bool) {
        if focused {
            self.view.focus_trigger(visible);
        } else {
            self.view.blur_trigger();
        }
    }

    fn overlay_open(&self) -> bool {
        self.model.combo.open && !self.model.combo.disabled
    }

    fn overlay_scope(&self) -> Option<OverlayScope> {
        (self.model.combo.open && !self.model.combo.disabled).then_some(OverlayScope {
            initial_focus: Some(self.query_content().id()),
        })
    }

    fn overlay_presented(&self) -> bool {
        self.model.combo.open || self.view.options.is_presented()
    }

    fn action_kinds(&self, result: &ContentResponse) -> Vec<ContentActionKind> {
        if result.activated && self.last_selection.is_some() {
            vec![ContentActionKind::Activate]
        } else {
            Vec::new()
        }
    }

    fn action_payload(&self, kind: ContentActionKind) -> Option<ContentActionPayload> {
        if kind != ContentActionKind::Activate {
            return None;
        }
        let index = self.last_selection?;
        Some(ContentActionPayload::CommandSelection {
            index,
            label: self.model.combo.items.get(index)?.clone(),
        })
    }

    fn cancel_pointer(&mut self) -> ContentResponse {
        self.last_selection = None;
        self.view.cancel_pointer_arm();
        ContentResponse::consumed(true)
    }

    fn overlay_event(&mut self, event: &ContentEvent) -> ContentResponse {
        self.last_selection = None;
        self.sync_model_query();
        if self.query_text().composing()
            && matches!(
                event,
                ContentEvent::Key { .. }
                    | ContentEvent::Pointer(_)
                    | ContentEvent::PointerDown(_)
                    | ContentEvent::PointerUp(_)
                    | ContentEvent::PointerMove(_)
            )
        {
            return ContentResponse::default();
        }
        match event {
            ContentEvent::Key { key, down } => {
                let result = self.view.key_event(*key, *down, &mut self.model);
                let selected = (*down
                    && *key == ControlKey::Enter
                    && result.changed
                    && !self.model.combo.open)
                    .then_some(self.model.combo.selected)
                    .flatten();
                if !self.model.combo.open && result.changed {
                    self.sync_query_text();
                }
                self.sync_child_activation();
                let response = ContentResponse {
                    consumed: result.consumed,
                    changed: result.changed,
                    activated: selected.is_some(),
                    ..Default::default()
                };
                if selected.is_some() {
                    self.last_selection = selected;
                }
                response
            }
            ContentEvent::Scroll {
                position,
                delta,
                precise,
                begin,
                end,
                momentum,
                time,
            } if self.popup_contains(*position) => {
                let result = self.view.options.wheel_event(crate::MenuWheelInput {
                    point: *position,
                    delta: *delta,
                    precise: *precise,
                    begin: *begin,
                    end: *end,
                    momentum: *momentum,
                    time: *time,
                });
                match result {
                    crate::MenuWheelEvent::Ignored => ContentResponse::default(),
                    crate::MenuWheelEvent::Consumed { scrolled } => {
                        ContentResponse::consumed(scrolled)
                    }
                }
            }
            ContentEvent::Pointer(pointer) => self.overlay_pointer_event(pointer),
            ContentEvent::PointerDown(position) => {
                if self.popup_contains(*position) {
                    self.popup_pointer(PointerPhase::Down, *position)
                } else if self.query_contains(*position) {
                    ContentResponse::default()
                } else if self.close() {
                    ContentResponse::consumed(true)
                } else {
                    ContentResponse::default()
                }
            }
            ContentEvent::PointerUp(position) => self.popup_pointer(PointerPhase::Up, *position),
            ContentEvent::PointerMove(position) => {
                self.popup_pointer(PointerPhase::Move, *position)
            }
            _ => ContentResponse::default(),
        }
    }

    fn overlay_pointer_event(&mut self, pointer: &PointerEvent) -> ContentResponse {
        if !pointer.is_primary() || self.query_text().composing() {
            return ContentResponse::default();
        }
        match pointer.phase {
            PointerPhase::Down if self.popup_contains(pointer.position) => {
                self.popup_pointer(pointer.phase, pointer.position)
            }
            PointerPhase::Down if self.query_contains(pointer.position) => {
                ContentResponse::default()
            }
            PointerPhase::Down if self.close() => ContentResponse::consumed(true),
            PointerPhase::Up => self.popup_pointer(pointer.phase, pointer.position),
            PointerPhase::Move | PointerPhase::Drag if self.popup_contains(pointer.position) => {
                self.popup_pointer(pointer.phase, pointer.position)
            }
            _ => ContentResponse::default(),
        }
    }

    fn event(&mut self, event: &ContentEvent) -> ContentResponse {
        self.last_selection = None;
        if self.model.combo.open {
            match event {
                ContentEvent::PointerDown(position) if self.popup_contains(*position) => {
                    return self.popup_pointer(PointerPhase::Down, *position);
                }
                ContentEvent::PointerUp(position) => {
                    return self.popup_pointer(PointerPhase::Up, *position);
                }
                ContentEvent::PointerMove(position) => {
                    return self.popup_pointer(PointerPhase::Move, *position);
                }
                ContentEvent::Cancel => {
                    self.last_selection = None;
                    self.view.cancel(&mut self.model);
                    self.sync_child_activation();
                    return ContentResponse::consumed(true);
                }
                _ => {}
            }
        }
        match event {
            ContentEvent::Pointer(pointer) if !self.model.combo.open => match pointer.phase {
                PointerPhase::Down => {
                    let consumed = self.view.pointer_down_trigger(pointer.position);
                    ContentResponse {
                        consumed,
                        capture_pointer: consumed,
                        ..Default::default()
                    }
                }
                PointerPhase::Up => {
                    let changed = self
                        .view
                        .pointer_up_trigger(pointer.position, &mut self.model);
                    if changed {
                        self.sync_query_text();
                        self.sync_child_activation();
                    }
                    ContentResponse {
                        consumed: true,
                        changed,
                        activated: changed,
                        ..Default::default()
                    }
                }
                PointerPhase::Move | PointerPhase::Enter | PointerPhase::Exit => {
                    self.view.pointer_move(pointer.position, &mut self.model);
                    ContentResponse::consumed(false)
                }
                _ => ContentResponse::default(),
            },
            ContentEvent::PointerDown(position) if !self.model.combo.open => {
                let consumed = self.view.pointer_down_trigger(*position);
                ContentResponse {
                    consumed,
                    capture_pointer: consumed,
                    ..Default::default()
                }
            }
            ContentEvent::PointerUp(position) if !self.model.combo.open => {
                let changed = self.view.pointer_up_trigger(*position, &mut self.model);
                if changed {
                    self.sync_query_text();
                    self.sync_child_activation();
                }
                ContentResponse {
                    consumed: true,
                    changed,
                    activated: changed,
                    ..Default::default()
                }
            }
            ContentEvent::Key { key, down } if !self.model.combo.open => {
                self.last_selection = None;
                let result = self.view.trigger_key(*key, *down, &mut self.model);
                if result.changed {
                    self.sync_query_text();
                    self.sync_child_activation();
                }
                ContentResponse {
                    consumed: result.consumed,
                    changed: result.changed,
                    activated: result.changed,
                    ..Default::default()
                }
            }
            _ => ContentResponse::default(),
        }
    }
}
