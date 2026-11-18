//! Minimal semantic adapters. State and presentation stay in production controls.
use super::*;
use crate::Dialog;
use crate::extended::Switch;
use crate::feedback_view::DialogView;
use crate::layout_view::{SelectionEvent, SwitchView};
use tekes_ui_tokens::Spacing;

/// General content organization, deliberately without ListBox selection semantics.
/// Children keep their own actions; adding an input never implicitly selects a row.
#[derive(Debug)]
pub struct ListContent {
    pub name: String,
    pub rows: StackContent,
}
impl ListContent {
    /// One original scrolling owner around this List's grouped Items and Model.
    /// `items_id` and the outer Content ID are explicit caller-owned identities.
    /// Headers, items and their existing selection reducers keep their own IDs.
    /// The List owns default document padding; an explicit content_padding on
    /// the returned ScrollContent replaces it rather than adding another inset.
    pub fn scrollable(
        name: impl Into<String>,
        items_id: ComponentId,
        children: Vec<Content>,
    ) -> ScrollContent {
        ScrollContent::new(
            ScrollArea::default(),
            Content::new(items_id, Self::new(name, children)),
        )
        .scrollbar(true)
        .content_padding(Spacing::Loose.value())
    }
    pub fn new(name: impl Into<String>, children: Vec<Content>) -> Self {
        Self {
            name: name.into(),
            rows: StackContent {
                flexible_child: None,
                model: Stack {
                    axis: Axis::Vertical,
                    gap: Spacing::Regular.value(),
                },
                children,
            },
        }
    }
}
impl Component for ListContent {
    fn curve_subtree_capability(&self) -> CurveSubtreeCapability {
        CurveSubtreeCapability::Recursive
    }
    fn ignores_pointer_actions(&self) -> bool {
        true
    }
    fn measure(&mut self, t: &mut dyn ButtonTextRenderer, a: Size) -> MeasuredChild {
        self.rows.measure(t, a)
    }
    fn arrange(&mut self, t: &mut dyn ButtonTextRenderer, f: Rect, c: Rect) {
        self.rows.arrange(t, f, c)
    }
    fn children(&self) -> &[Content] {
        &self.rows.children
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.rows.children)
    }
    fn semantics(&self) -> ContentSemantics {
        ContentSemantics::new(ContentRole::List, &self.name)
    }
}

#[derive(Debug)]
pub struct SwitchContent {
    pub model: Switch,
    pub view: SwitchView,
    pub label: String,
}
impl SwitchContent {
    pub fn new(label: impl Into<String>, checked: bool) -> Self {
        Self {
            model: Switch {
                checked,
                disabled: false,
            },
            view: SwitchView::default(),
            label: label.into(),
        }
    }
}
impl Component for SwitchContent {
    fn pointer_event_in_region(&mut self, event: &ContentEvent, inside: bool) -> ContentResponse {
        if !event.is_primary_pointer() {
            return event.deliver(self);
        }
        let mut response = ContentResponse::default();
        match event.routing_event().as_ref() {
            ContentEvent::PointerMove(point) => {
                response.changed = self.view.pointer_move_in_region(*point, inside)
            }
            ContentEvent::PointerUp(point) => {
                let result = self
                    .view
                    .pointer_up_in_region(*point, &mut self.model, inside);
                response.consumed = result != SelectionEvent::Ignored;
                response.changed = result == SelectionEvent::Changed;
            }
            _ => return event.deliver(self),
        }
        response
    }

    fn curve_subtree_capability(&self) -> CurveSubtreeCapability {
        CurveSubtreeCapability::Local
    }

    fn accessibility_action(
        &mut self,
        part: Option<usize>,
        action: &AccessibilityAction,
    ) -> ContentResponse {
        if part.is_some() || !matches!(action, AccessibilityAction::Press) || self.model.disabled {
            return ContentResponse::default();
        }
        let changed = self.model.toggle();
        ContentResponse {
            consumed: true,
            changed,
            activated: changed,
            ..Default::default()
        }
    }

    fn measure(&mut self, t: &mut dyn ButtonTextRenderer, a: Size) -> MeasuredChild {
        let s = t.measure(
            &self.label,
            proposal::TextRole::UiBody,
            (a.width - 62.).max(0.),
        );
        MeasuredChild::new(Size::new(a.width, 32_f32.max(s.height + 8.)))
    }
    fn paint(
        &mut self,
        s: &mut Scene,
        t: &mut dyn ButtonTextRenderer,
        f: Rect,
        c: Rect,
        theme: ContentTheme,
    ) {
        self.view.render_labeled(
            s,
            &mut TextAdapter(t),
            self.model,
            f,
            Some(c),
            theme.palette,
            &self.label,
        );
        if self.view.needs_animation() {
            t.request_animation_frame();
        }
    }
    fn focusable(&self) -> bool {
        !self.model.disabled
    }
    fn set_focus(&mut self, f: bool, v: bool) {
        if f {
            self.view.focus(v)
        } else {
            self.view.blur()
        }
    }
    fn semantics(&self) -> ContentSemantics {
        let mut s = ContentSemantics::new(ContentRole::Switch, &self.label);
        s.selected = self.model.checked;
        s.disabled = self.model.disabled;
        s.invalid = self.view.error();
        s
    }
    fn event(&mut self, e: &ContentEvent) -> ContentResponse {
        let mut r = ContentResponse::default();
        match e {
            ContentEvent::PointerMove(p) => r.changed = self.view.pointer_move(*p),
            ContentEvent::PointerDown(p) => {
                r.consumed = self.view.pointer_down(*p, &self.model);
                r.capture_pointer = r.consumed;
                r.changed = r.consumed;
            }
            ContentEvent::PointerUp(p) => {
                let e = self.view.pointer_up(*p, &mut self.model);
                r.consumed = e != SelectionEvent::Ignored;
                r.changed = e == SelectionEvent::Changed;
            }
            ContentEvent::Key { key, down } => {
                let e = self.view.key(*key, *down, &mut self.model);
                r.consumed = e != SelectionEvent::Ignored;
                r.changed = r.consumed;
            }
            ContentEvent::Cancel => {
                r.changed = self.view.cancel();
            }
            _ => {}
        }
        r
    }
}

/// Three fixed semantic slots using the same Content type. The host owns modal
/// focus/restore; the existing Dialog model owns open/close. Body overflow is
/// expressed with the existing ScrollContent, not an additional scroll model.
#[derive(Debug)]
pub struct DialogContent {
    pub(super) model: Dialog,
    presence: crate::control_motion::OverlayPresence,
    pub name: String,
    slots: Vec<Content>,
    viewport: Rect,
    modal: Rect,
    max_width: Option<f32>,
}
impl DialogContent {
    pub fn new(name: impl Into<String>, header: Content, body: Content, footer: Content) -> Self {
        Self {
            model: Dialog::default(),
            presence: Default::default(),
            name: name.into(),
            slots: vec![header, body, footer],
            viewport: Rect::default(),
            modal: Rect::default(),
            max_width: None,
        }
    }
    pub fn with_max_width(mut self, max_width: Option<f32>) -> Self {
        self.max_width = max_width;
        self
    }
    pub fn set_max_width(&mut self, max_width: Option<f32>) {
        self.max_width = max_width;
    }
    pub fn modal_frame(&self) -> Rect {
        self.modal
    }
    pub fn is_open(&self) -> bool {
        self.model.open
    }
    /// Paint may outlive semantic dismissal; it never keeps input or focus.
    pub fn is_presented(&self) -> bool {
        self.model.open || self.presence.is_presented()
    }
    pub fn needs_animation(&self) -> bool {
        self.presence.needs_animation()
    }
    /// Call before replacing the host atlas/text/image resources.
    pub fn clear_presentation(&mut self) {
        self.presence.clear();
    }
    pub(super) fn try_present(
        &mut self,
        destination: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        frame: Rect,
        clip: Rect,
        palette: Palette,
        current: Option<Scene>,
    ) -> Result<(), tekes_ui_scene::FragmentError> {
        let context = format!("{frame:?}{clip:?}{palette:?}{}", text.backing_scale());
        self.presence.try_paint(
            destination,
            &mut TextAdapter(text),
            self.model.open,
            context,
            current,
        )
    }
    pub(super) fn prepare_dialog_effects(
        &self,
        hooks: &[PreparedHook],
    ) -> Result<PreparedEffects, PreparedEffectsError> {
        if self.presence.preflight_clears(
            hooks
                .iter()
                .filter(|h| h.kind == PreparedHookKind::Lifecycle(Lifecycle::Unmounted))
                .count(),
        ) {
            Ok(PreparedEffects::local(self, hooks))
        } else {
            Err(PreparedEffectsError::CounterCapacity)
        }
    }
}
impl Component for DialogContent {
    // Arranged semantic slots translate with their original modal frame. This
    // qualifies the Dialog ancestor of a Scroll body for read-only focus reveal.
    fn host_reveal_passthrough(&self) -> bool {
        self.model.open && !self.modal.is_empty()
    }
    fn host_reveal_local_clip(&self) -> Option<Rect> {
        Some(self.modal)
    }

    fn curve_subtree_capability(&self) -> CurveSubtreeCapability {
        CurveSubtreeCapability::Recursive
    }
    fn has_stable_hooks(&self) -> bool {
        true
    }
    fn supports_prepared_lifecycle(&self) -> bool {
        true
    }
    fn ignores_pointer_actions(&self) -> bool {
        true
    }
    fn semantic_frame(&self, _: Rect) -> Rect {
        self.modal
    }
    fn lifecycle(&mut self, event: Lifecycle) {
        if event == Lifecycle::Unmounted {
            self.clear_presentation();
        }
    }
    fn fixed_slots(&self) -> bool {
        true
    }
    fn validate(&self) -> Result<(), CompositionError> {
        if self.slots.len() != 3 {
            Err(CompositionError::InvalidSlotCount)
        } else if self.name.trim().is_empty() {
            Err(CompositionError::EmptyAccessibleName)
        } else {
            Ok(())
        }
    }
    fn active(&self) -> bool {
        self.model.open
    }
    fn children(&self) -> &[Content] {
        &self.slots
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.slots)
    }
    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, a: Size) -> MeasuredChild {
        MeasuredChild::new(a)
    }
    fn arrange(&mut self, t: &mut dyn ButtonTextRenderer, f: Rect, c: Rect) {
        self.viewport = f;
        if !self.is_presented() {
            return;
        }
        let pad = Spacing::Relaxed.value();
        let gap = Spacing::Regular.value();
        let raw_w = (f.size.width - 16.).max(0.);
        let w = self
            .max_width
            .filter(|limit| limit.is_finite())
            .map_or(raw_w, |limit| raw_w.min(limit.max(0.)));
        let inner = (w - pad * 2.).max(0.);
        let header = self.slots[0]
            .measure(t, Size::new(inner, f.size.height))
            .size
            .height;
        let footer = self.slots[2]
            .measure(t, Size::new(inner, f.size.height))
            .size
            .height;
        let body_available = (f.size.height - 2. * pad - 2. * gap - header - footer).max(0.);
        let body = self.slots[1]
            .measure(t, Size::new(inner, body_available))
            .size
            .height
            .min(body_available);
        let h = (header + footer + body + 2. * pad + 2. * gap).min(f.size.height);
        self.modal = Rect::new(
            f.min_x() + (f.size.width - w) * 0.5,
            f.min_y() + (f.size.height - h) * 0.5,
            w,
            h,
        );
        let clip = self.modal.intersection(&c).unwrap_or_default();
        let x = self.modal.min_x() + pad;
        let y = self.modal.min_y() + pad;
        self.slots[0].arrange(t, Rect::new(x, y, inner, header), clip);
        self.slots[1].arrange(t, Rect::new(x, y + header + gap, inner, body), clip);
        self.slots[2].arrange(
            t,
            Rect::new(x, self.modal.max_y() - pad - footer, inner, footer),
            clip,
        );
    }
    fn paint(
        &mut self,
        s: &mut Scene,
        t: &mut dyn ButtonTextRenderer,
        _: Rect,
        c: Rect,
        theme: ContentTheme,
    ) {
        DialogView::paint_content_frame(
            s,
            self.viewport,
            self.modal,
            c,
            theme.palette,
            t.backing_scale(),
        );
    }
    fn semantics(&self) -> ContentSemantics {
        ContentSemantics::new(ContentRole::Dialog, &self.name)
    }
}

/// Range adapter for the existing Slider model/View. The name is semantic;
/// callers compose visible labels/captions with the existing content containers.
#[derive(Debug)]
pub struct SliderContent {
    pub model: crate::extended::Slider,
    pub view: crate::layout_view::SliderView,
    pub name: String,
}
impl SliderContent {
    pub fn new(name: impl Into<String>, model: crate::extended::Slider) -> Self {
        Self {
            model,
            view: crate::layout_view::SliderView::new(),
            name: name.into(),
        }
    }
}
impl Component for SliderContent {
    fn curve_subtree_capability(&self) -> CurveSubtreeCapability {
        CurveSubtreeCapability::Local
    }

    fn validate(&self) -> Result<(), CompositionError> {
        if self.name.trim().is_empty() {
            Err(CompositionError::EmptyAccessibleName)
        } else {
            Ok(())
        }
    }
    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, a: Size) -> MeasuredChild {
        MeasuredChild::new(Size::new(
            a.width,
            proposal::metric("size", "size-row-height").expect("shared row size"),
        ))
    }
    fn paint(
        &mut self,
        s: &mut Scene,
        _: &mut dyn ButtonTextRenderer,
        f: Rect,
        c: Rect,
        theme: ContentTheme,
    ) {
        self.view.render(s, self.model, f, Some(c), theme.palette);
    }
    fn focusable(&self) -> bool {
        !self.model.disabled
    }
    fn set_focus(&mut self, f: bool, visible: bool) {
        if f {
            self.view.focus(visible)
        } else {
            self.view.blur()
        }
    }
    fn semantics(&self) -> ContentSemantics {
        let mut s = ContentSemantics::new(ContentRole::Slider, &self.name);
        s.value = Some(self.model.value().to_string());
        s.disabled = self.model.disabled;
        s
    }
    fn value_range(&self) -> Option<std::ops::RangeInclusive<f64>> {
        Some(self.model.range())
    }
    fn event(&mut self, e: &ContentEvent) -> ContentResponse {
        let before = self.model.value();
        let consumed = match e {
            ContentEvent::PointerDown(p) => {
                let began = self.view.pointer_down(*p, &mut self.model);
                return ContentResponse {
                    consumed: began,
                    changed: began,
                    capture_pointer: began,
                    ..Default::default()
                };
            }
            ContentEvent::PointerMove(p) => {
                let changed = self.view.pointer_move(*p, &mut self.model);
                return ContentResponse {
                    changed,
                    ..Default::default()
                };
            }
            ContentEvent::PointerUp(p) => self.view.pointer_up(*p, &self.model),
            ContentEvent::Key { key, down }
                if matches!(
                    key,
                    ControlKey::Left
                        | ControlKey::Right
                        | ControlKey::Up
                        | ControlKey::Down
                        | ControlKey::Home
                        | ControlKey::End
                ) =>
            {
                let owned = self.view.is_focused() && !self.model.disabled;
                self.view.key(*key, *down, &mut self.model);
                owned
            }
            ContentEvent::Cancel => {
                let changed = self.view.cancel();
                return ContentResponse {
                    changed,
                    ..Default::default()
                };
            }
            _ => false,
        };
        ContentResponse {
            consumed,
            changed: self.model.value() != before,
            ..Default::default()
        }
    }
    fn accessibility_action(
        &mut self,
        part: Option<usize>,
        a: &AccessibilityAction,
    ) -> ContentResponse {
        if part.is_some() || self.model.disabled {
            return ContentResponse::default();
        }
        let changed = match a {
            AccessibilityAction::Increment => self.view.adjust_value(true, &mut self.model),
            AccessibilityAction::Decrement => self.view.adjust_value(false, &mut self.model),
            AccessibilityAction::SetValue(v) => {
                let Ok(value) = v.parse::<f64>() else {
                    return ContentResponse::default();
                };
                if !value.is_finite() {
                    return ContentResponse::default();
                }
                self.view.set_value(value, &mut self.model)
            }
            _ => return ContentResponse::default(),
        };
        changed.map_or_else(ContentResponse::default, ContentResponse::consumed)
    }
}
