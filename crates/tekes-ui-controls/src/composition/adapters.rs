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
    name: String,
    rows: StackContent,
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
                    gap: Self::SECTION_GAP,
                },
                children,
            },
        }
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn set_name(&mut self, name: impl Into<String>) {
        self.name = name.into();
    }
    pub fn gap(&self) -> f32 {
        Self::SECTION_GAP
    }
    pub fn items(&self) -> &[Content] {
        &self.rows.children
    }
    pub fn items_mut(&mut self) -> &mut Vec<Content> {
        &mut self.rows.children
    }
    pub fn replace_items(&mut self, children: Vec<Content>) {
        self.rows.children = children;
    }
    pub fn push_item(&mut self, child: Content) {
        self.rows.children.push(child);
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
    pub(crate) model: Switch,
    pub(crate) view: SwitchView,
    pub(crate) label: String,
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
    pub fn with_disabled(mut self, disabled: bool) -> Self {
        self.set_disabled(disabled);
        self
    }
    pub fn is_checked(&self) -> bool {
        self.model.checked
    }
    pub fn set_checked(&mut self, checked: bool) {
        self.model.checked = checked;
    }
    pub fn is_disabled(&self) -> bool {
        self.model.disabled
    }
    pub fn set_disabled(&mut self, disabled: bool) {
        self.model.disabled = disabled;
        if disabled {
            self.view.cancel();
            self.view.blur();
        }
    }
    pub fn label(&self) -> &str {
        &self.label
    }
    pub fn set_label(&mut self, label: impl Into<String>) {
        self.label = label.into();
    }
    pub fn error(&self) -> bool {
        self.view.error()
    }
    pub fn set_error(&mut self, error: bool) {
        self.view.set_error(error);
    }
    pub fn model(&self) -> Switch {
        self.model
    }
    pub fn view(&self) -> &SwitchView {
        &self.view
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
        if self.label.is_empty() {
            return MeasuredChild::new(Size::new(a.width.min(SwitchView::TRACK_WIDTH), 32.));
        }
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
        let _ = self.view.set_surface(theme.surface());
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
    name: String,
    slots: Vec<Content>,
    viewport: Rect,
    modal: Rect,
    max_width: Option<f32>,
}
impl DialogContent {
    /// Standard horizontal trailing-aligned action row (`Spacing::Regular` = 8 pt gap)
    /// for dialog footers.
    pub fn actions_row(id: ComponentId, actions: Vec<Content>) -> Content {
        StackContent {
            model: Stack {
                axis: Axis::Horizontal,
                gap: Spacing::Regular.value(),
            },
            flexible_child: None,
            children: actions,
        }
        .into_content_with_distribution(id, proposal::layout::MainAxisDistribution::End)
    }

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
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn set_name(&mut self, name: impl Into<String>) {
        self.name = name.into();
    }
    pub fn body(&self) -> &Content {
        &self.slots[1]
    }
    pub fn body_mut(&mut self) -> &mut Content {
        &mut self.slots[1]
    }
    /// Replace the center body slot while keeping the three-slot `[header, body, footer]` invariant intact.
    pub fn set_body(&mut self, body: Content) -> Content {
        std::mem::replace(&mut self.slots[1], body)
    }
    /// Transform and replace the center body slot safely while preserving all three Dialog slots.
    pub fn replace_body_with(&mut self, replace: impl FnOnce(Content) -> Content) {
        let placeholder = Content::new(ComponentId(0), TextContent::new(""));
        let previous = std::mem::replace(&mut self.slots[1], placeholder);
        self.slots[1] = replace(previous);
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
        let viewport_margin = Spacing::Regular.value();
        let pad = Spacing::Loose.value();
        let gap = Spacing::Relaxed.value();
        let raw_w = (f.size.width - viewport_margin * 2.).max(0.);
        let raw_h = (f.size.height - viewport_margin * 2.).max(0.);
        let w = self
            .max_width
            .filter(|limit| limit.is_finite())
            .map_or(raw_w, |limit| raw_w.min(limit.max(0.)));
        let inner = (w - pad * 2.).max(0.);
        let header = self.slots[0]
            .measure(t, Size::new(inner, raw_h))
            .size
            .height;
        let footer = self.slots[2]
            .measure(t, Size::new(inner, raw_h))
            .size
            .height;
        let body_available = (raw_h - 2. * pad - 2. * gap - header - footer).max(0.);
        let body = self.slots[1]
            .measure(t, Size::new(inner, body_available))
            .size
            .height
            .min(body_available);
        let h = (header + footer + body + 2. * pad + 2. * gap).min(raw_h);
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
    pub(crate) model: crate::extended::Slider,
    pub(crate) view: crate::layout_view::SliderView,
    pub(crate) name: String,
    runnable: bool,
}
impl SliderContent {
    pub fn new(name: impl Into<String>, model: crate::extended::Slider) -> Self {
        Self {
            model,
            view: crate::layout_view::SliderView::new(),
            name: name.into(),
            runnable: false,
        }
    }
    pub fn with_disabled(mut self, disabled: bool) -> Self {
        self.set_disabled(disabled);
        self
    }
    pub fn value(&self) -> f64 {
        self.model.value()
    }
    pub fn set_value(&mut self, value: f64) -> bool {
        self.model.set(value)
    }
    pub fn is_disabled(&self) -> bool {
        self.model.disabled
    }
    pub fn set_disabled(&mut self, disabled: bool) {
        self.model.disabled = disabled;
        if disabled {
            self.view.cancel();
            self.view.blur();
        }
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn set_name(&mut self, name: impl Into<String>) {
        self.name = name.into();
    }
    pub fn model(&self) -> crate::extended::Slider {
        self.model
    }
    pub fn view(&self) -> &crate::layout_view::SliderView {
        &self.view
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
        let _ = self.view.set_surface(theme.surface());
        self.view.render(s, self.model, f, Some(c), theme.palette);
    }
    fn lifecycle(&mut self, event: Lifecycle) {
        if event == Lifecycle::Unmounted {
            self.runnable = false;
            self.view.clear_layout();
        }
    }
    fn window_active_changed(&mut self, active: bool) {
        self.view.set_window_active(active);
    }
    fn presentation_runnable_changed(&mut self, runnable: bool) {
        if runnable {
            self.runnable = true;
        } else if self.runnable && self.view.is_window_active() {
            self.runnable = false;
            self.view.clear_layout();
        }
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

#[derive(Debug)]
struct ListSectionContent {
    rows: StackContent,
    arranged: bool,
}
impl Component for ListSectionContent {
    fn host_reveal_passthrough(&self) -> bool {
        self.arranged
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
    fn fixed_slots(&self) -> bool {
        true
    }
    fn validate(&self) -> Result<(), CompositionError> {
        if self.rows.children.len() != 2 {
            Err(CompositionError::InvalidSlotCount)
        } else {
            Ok(())
        }
    }
    fn lifecycle(&mut self, event: Lifecycle) {
        if event == Lifecycle::Unmounted {
            self.arranged = false;
        }
    }
    fn children(&self) -> &[Content] {
        &self.rows.children
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.rows.children)
    }
    fn measure(&mut self, text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        self.arranged = false;
        self.rows.measure(text, available)
    }
    fn measure_intrinsic(
        &mut self,
        text: &mut dyn ButtonTextRenderer,
        available: Size,
    ) -> MeasuredChild {
        self.rows.measure_intrinsic(text, available)
    }
    fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        self.rows.arrange(text, frame, clip);
        self.arranged = frame.size.width > 0. && frame.size.height > 0.;
    }
}

impl ListContent {
    /// Canonical vertical gap above and below every list section header.
    pub const SECTION_GAP: f32 = Spacing::Regular.value();

    /// Build a titled list section (`[UiCaption header, items]`) whose header-to-items gap
    /// equals `ListContent::SECTION_GAP`, ensuring equal vertical spacing above and below
    /// section headers while preserving caller-owned IDs.
    pub fn section(
        group_id: ComponentId,
        header_id: ComponentId,
        title: impl Into<String>,
        items: Content,
    ) -> Content {
        let heading = TextContent::caption(title);
        Content::new(
            group_id,
            ListSectionContent {
                rows: StackContent {
                    model: Stack {
                        axis: Axis::Vertical,
                        gap: Self::SECTION_GAP,
                    },
                    flexible_child: None,
                    children: vec![
                        Content::new(header_id, heading).with_secondary_foreground(),
                        items,
                    ],
                },
                arranged: false,
            },
        )
    }

    /// Compose a fixed non-scrolling header above a scrolling `ListContent` with
    /// symmetric `Spacing::Loose` (16 pt) outer insets, shared horizontal alignment,
    /// `Spacing::Regular` (8 pt) scroll-clip clearance below the header, and a
    /// flush right scrollbar track.
    #[allow(clippy::too_many_arguments)]
    pub fn scrollable_with_header(
        root_id: ComponentId,
        header_frame_id: ComponentId,
        scroll_id: ComponentId,
        items_id: ComponentId,
        name: impl Into<String>,
        header: Content,
        children: Vec<Content>,
    ) -> Content {
        let pad = Spacing::Loose.value();
        let clip_gap = Spacing::Regular.value();
        let header_wrap = StackContent {
            model: Stack {
                axis: Axis::Vertical,
                gap: 0.,
            },
            flexible_child: None,
            children: vec![header],
        }
        .into_content_with_padding(
            header_frame_id,
            crate::layout::Insets::new(pad, pad, clip_gap, pad),
        );
        let scroll = Content::new(
            scroll_id,
            Self::scrollable(name, items_id, children)
                .content_insets(crate::layout::Insets::new(clip_gap, pad, pad, pad)),
        );
        Content::new(
            root_id,
            StackContent {
                model: Stack {
                    axis: Axis::Vertical,
                    gap: 0.,
                },
                flexible_child: Some(scroll_id),
                children: vec![header_wrap, scroll],
            },
        )
    }
}

/// Full-viewport overlay host placing background content alongside one or more
/// `DialogContent` siblings without coupling modal bounds to scrolled documents.
#[derive(Debug)]
pub struct ModalViewportContent {
    children: Vec<Content>,
    name: String,
    content_padding: crate::layout::Insets,
    arranged: bool,
}

impl ModalViewportContent {
    pub fn new(children: Vec<Content>) -> Self {
        Self {
            children,
            name: String::new(),
            content_padding: crate::layout::Insets::ZERO,
            arranged: false,
        }
    }

    pub fn with_accessible_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Optional inner padding applied to non-dialog children (e.g. a screen invoker
    /// button), while `DialogContent` children continue to occupy the full viewport.
    pub fn with_content_padding(mut self, padding: crate::layout::Insets) -> Self {
        self.content_padding = crate::layout::Insets::new(
            finite(padding.top),
            finite(padding.right),
            finite(padding.bottom),
            finite(padding.left),
        );
        self
    }
}

impl Component for ModalViewportContent {
    fn children(&self) -> &[Content] {
        &self.children
    }

    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.children)
    }

    fn host_reveal_passthrough(&self) -> bool {
        self.arranged
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

    fn lifecycle(&mut self, event: Lifecycle) {
        if event == Lifecycle::Unmounted {
            self.arranged = false;
        }
    }

    fn semantics(&self) -> ContentSemantics {
        ContentSemantics::new(ContentRole::Group, &self.name)
    }

    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, a: Size) -> MeasuredChild {
        self.arranged = false;
        MeasuredChild::new(a)
    }

    fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        let has_pad = self.content_padding != crate::layout::Insets::ZERO;
        let inner = if has_pad {
            Rect::new(
                frame.min_x() + self.content_padding.left,
                frame.min_y() + self.content_padding.top,
                (frame.size.width - self.content_padding.left - self.content_padding.right).max(0.),
                (frame.size.height - self.content_padding.top - self.content_padding.bottom)
                    .max(0.),
            )
        } else {
            frame
        };
        for child in &mut self.children {
            if !has_pad || child.component::<DialogContent>().is_some() {
                child.arrange(text, frame, clip);
            } else {
                let measured = child.measure_intrinsic(text, inner.size).size;
                let rect = Rect::new(
                    inner.min_x(),
                    inner.min_y(),
                    measured.width.min(inner.size.width),
                    measured.height.min(inner.size.height),
                );
                child.arrange(text, rect, clip);
            }
        }
        self.arranged = frame.size.width > 0. && frame.size.height > 0.;
    }
}

#[cfg(test)]
mod dialog_geometry_tests {
    use super::*;

    #[test]
    fn dialog_uses_symmetric_viewport_margins_and_pad_exceeds_gap() {
        struct Text;
        impl ButtonTextRenderer for Text {
            fn measure(&mut self, value: &str, role: proposal::TextRole, width: f32) -> Size {
                Size::new(
                    (value.chars().count() as f32 * 8.).min(width),
                    role.style().line_height,
                )
            }
            fn draw(
                &mut self,
                _: &mut Scene,
                _: &str,
                _: proposal::TextRole,
                _: Rect,
                _: crate::ButtonTextAlign,
                _: Rgba,
            ) {
            }
        }
        let mut dialog = DialogContent::new(
            "Confirm",
            Content::new(ComponentId(1), TextContent::heading("Title")),
            Content::new(ComponentId(2), TextContent::new("Body copy")),
            Content::new(ComponentId(3), Button::new("OK")),
        );
        dialog.model.open = true;
        assert!(dialog.is_open());
        let viewport = Rect::new(0., 0., 400., 300.);
        dialog.arrange(&mut Text, viewport, viewport);
        let modal = dialog.modal_frame();
        assert!(modal.min_x() - viewport.min_x() >= Spacing::Regular.value() - 0.01);
        assert!(viewport.max_x() - modal.max_x() >= Spacing::Regular.value() - 0.01);
        assert!(modal.min_y() - viewport.min_y() >= Spacing::Regular.value() - 0.01);
        assert!(viewport.max_y() - modal.max_y() >= Spacing::Regular.value() - 0.01);
        let header = dialog.slots[0].frame();
        let body = dialog.slots[1].frame();
        let footer = dialog.slots[2].frame();
        let pad_top = header.min_y() - modal.min_y();
        let pad_bottom = modal.max_y() - footer.max_y();
        let gap1 = body.min_y() - header.max_y();
        let gap2 = footer.min_y() - body.max_y();
        assert!((pad_top - Spacing::Loose.value()).abs() <= 0.01);
        assert!((pad_bottom - Spacing::Loose.value()).abs() <= 0.01);
        assert!(pad_top > gap1 && pad_top > gap2);
    }

    #[test]
    fn list_scrollable_with_header_aligns_columns_and_preserves_clip_clearance() {
        struct Text;
        impl ButtonTextRenderer for Text {
            fn measure(&mut self, value: &str, role: proposal::TextRole, width: f32) -> Size {
                Size::new(
                    (value.chars().count() as f32 * 8.).min(width),
                    role.style().line_height,
                )
            }
            fn draw(
                &mut self,
                _: &mut Scene,
                _: &str,
                _: proposal::TextRole,
                _: Rect,
                _: crate::ButtonTextAlign,
                _: Rgba,
            ) {
            }
        }
        let header = StackContent {
            model: Stack {
                axis: Axis::Horizontal,
                gap: Spacing::Regular.value(),
            },
            flexible_child: Some(ComponentId(10)),
            children: vec![
                Content::new(ComponentId(10), Button::new("Header Action")),
                Content::new(ComponentId(11), Button::new("Search")),
            ],
        }
        .into_content_with_distribution(
            ComponentId(9),
            proposal::layout::MainAxisDistribution::SpaceBetween,
        );
        let mut host = ContentHost::new(ListContent::scrollable_with_header(
            ComponentId(1),
            ComponentId(2),
            ComponentId(3),
            ComponentId(4),
            "Sidebar",
            header,
            vec![Content::new(ComponentId(5), TextContent::new("Row 1"))],
        ))
        .unwrap();
        let viewport = Rect::new(0., 0., 260., 300.);
        host.render(
            &mut Scene::new(),
            &mut Text,
            viewport,
            viewport,
            tekes_ui_tokens::Theme::LIGHT.into(),
        )
        .unwrap();
        let leading = host.get(ComponentId(10)).unwrap().frame();
        let trailing = host.get(ComponentId(11)).unwrap().frame();
        let header_frame = host.get(ComponentId(9)).unwrap().frame();
        let scroll_frame = host.get(ComponentId(3)).unwrap().frame();
        let row_frame = host.get(ComponentId(5)).unwrap().frame();
        assert_eq!(leading.min_x() - viewport.min_x(), 16.);
        assert_eq!(viewport.max_x() - trailing.max_x(), 16.);
        assert_eq!(header_frame.min_y() - viewport.min_y(), 16.);
        assert_eq!(scroll_frame.min_x(), viewport.min_x());
        assert_eq!(scroll_frame.max_x(), viewport.max_x());
        assert_eq!(scroll_frame.min_y() - header_frame.max_y(), 8.);
        assert_eq!(row_frame.min_x(), leading.min_x());
        assert_eq!(row_frame.min_y() - header_frame.max_y(), 16.);
    }

    #[test]
    fn modal_viewport_and_dialog_actions_row_align_horizontally_and_preserve_reveal() {
        struct Text;
        impl ButtonTextRenderer for Text {
            fn measure(&mut self, value: &str, role: proposal::TextRole, width: f32) -> Size {
                Size::new(
                    (value.chars().count() as f32 * 8.).min(width),
                    role.style().line_height,
                )
            }
            fn draw(
                &mut self,
                _: &mut Scene,
                _: &str,
                _: proposal::TextRole,
                _: Rect,
                _: crate::ButtonTextAlign,
                _: Rgba,
            ) {
            }
        }
        let footer = DialogContent::actions_row(
            ComponentId(20),
            vec![
                Content::new(ComponentId(21), Button::new("Cancel")),
                Content::new(ComponentId(22), Button::new("Confirm")),
            ],
        );
        let dialog = Content::new(
            ComponentId(2),
            DialogContent::new(
                "Confirm",
                Content::new(ComponentId(10), TextContent::heading("Title")),
                Content::new(ComponentId(11), TextContent::new("Body")),
                footer,
            ),
        );
        let mut host = ContentHost::new(Content::new(
            ComponentId(1),
            ModalViewportContent::new(vec![
                Content::new(ComponentId(3), Button::new("Open")),
                dialog,
            ])
            .with_content_padding(crate::Insets::new(
                Spacing::Section.value(),
                Spacing::Section.value(),
                Spacing::Section.value(),
                Spacing::Section.value(),
            )),
        ))
        .unwrap();
        let viewport = Rect::new(0., 0., 500., 400.);
        host.render(
            &mut Scene::new(),
            &mut Text,
            viewport,
            viewport,
            tekes_ui_tokens::Theme::LIGHT.into(),
        )
        .unwrap();
        assert_eq!(host.get(ComponentId(3)).unwrap().frame().min_x(), 24.);
        assert!(
            host.get(ComponentId(1))
                .unwrap()
                .component::<ModalViewportContent>()
                .unwrap()
                .host_reveal_passthrough()
        );
        host.show_dialog(ComponentId(2)).unwrap();
        host.render(
            &mut Scene::new(),
            &mut Text,
            viewport,
            viewport,
            tekes_ui_tokens::Theme::LIGHT.into(),
        )
        .unwrap();
        let cancel = host.get(ComponentId(21)).unwrap().frame();
        let confirm = host.get(ComponentId(22)).unwrap().frame();
        assert_eq!(cancel.min_y(), confirm.min_y());
        assert_eq!(confirm.min_x() - cancel.max_x(), Spacing::Regular.value());
        let footer_frame = host.get(ComponentId(20)).unwrap().frame();
        assert!((footer_frame.max_x() - confirm.max_x()).abs() <= 0.1);
    }
}
