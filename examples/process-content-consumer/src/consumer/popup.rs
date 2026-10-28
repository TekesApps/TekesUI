//! Current Main's public PopoverView + original Content-child adapter pattern.
//! Source: composition-consumer/tests/interactive_popover_intake.rs::Popup.
//! The fixture keeps the original remote scenario; no production popup/session API.
use super::*;
use tekes_ui_controls::{
    extended::Popover,
    feedback_view::{OverlayEvent, PopoverView},
    layout::MeasuredChild,
};
#[derive(Debug)]
pub(super) struct Popup {
    name: String,
    model: Popover,
    view: PopoverView,
    children: Vec<Content>,
    anchor: Rect,
    viewport: Rect,
    clip: Rect,
    size: Size,
    dismiss_focused: bool,
}
impl Popup {
    pub(super) fn new(name: &str, child: Content) -> Self {
        Self {
            name: name.into(),
            model: Popover::default(),
            view: PopoverView::default(),
            children: vec![child],
            anchor: Rect::default(),
            viewport: Rect::default(),
            clip: Rect::default(),
            size: Size::default(),
            dismiss_focused: false,
        }
    }
    pub(super) fn show(&mut self, anchor: Rect) {
        self.anchor = anchor;
        self.view.show(&mut self.model, 3);
    }
    pub(super) fn close(&mut self) -> bool {
        matches!(
            self.view.close(&mut self.model),
            OverlayEvent::Dismissed { .. }
        )
    }
}
impl Component for Popup {
    fn default_surface(&self) -> Option<&'static str> {
        Some("popover")
    }
    fn children(&self) -> &[Content] {
        &self.children
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.children)
    }
    fn paints_children(&self) -> bool {
        true
    }
    fn overlay_scope(&self) -> Option<OverlayScope> {
        Some(OverlayScope::default())
    }
    fn overlay_presented(&self) -> bool {
        self.model.open || self.view.is_presented()
    }
    fn overlay_open(&self) -> bool {
        self.model.open
    }
    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, _: Size) -> MeasuredChild {
        MeasuredChild::new(Size::default())
    }
    fn arrange(&mut self, mut text: &mut dyn ButtonTextRenderer, _: Rect, clip: Rect) {
        self.viewport = clip;
        self.clip = clip;
        if !self.overlay_presented() {
            return;
        }
        let available = self.view.arrange(
            &mut text,
            &self.model,
            self.anchor,
            clip.size,
            clip,
            Some(clip),
            proposal::TextRole::UiBody,
        );
        let body = self.children[0].measure(text, available.content.size).size;
        self.size = PopoverView::measure(&mut text, body, proposal::TextRole::UiBody);
        let layout = self.view.arrange(
            &mut text,
            &self.model,
            self.anchor,
            self.size,
            clip,
            Some(clip),
            proposal::TextRole::UiBody,
        );
        self.children[0].arrange(text, layout.content, layout.visible.unwrap_or_default());
    }
    fn paint_overlay(
        &mut self,
        scene: &mut Scene,
        mut text: &mut dyn ButtonTextRenderer,
        _: Rect,
        _: Rect,
        theme: ContentTheme,
    ) {
        let child = &mut self.children[0];
        self.view.render_with_text_content(
            scene,
            &mut text,
            &self.model,
            self.anchor,
            self.size,
            self.viewport,
            Some(self.clip),
            theme.palette,
            proposal::TextRole::UiBody,
            "original-content",
            |scene, text, _| child.paint(scene, text, theme),
        );
        if self.view.needs_animation() {
            text.request_animation_frame();
        }
    }
    fn overlay_event(&mut self, event: &ContentEvent) -> ContentResponse {
        match event {
            ContentEvent::PointerMove(p) => {
                self.view.pointer_move(*p);
                ContentResponse {
                    changed: true,
                    ..Default::default()
                }
            }
            ContentEvent::PointerDown(p) if self.view.pointer_down(&mut self.model, *p) => {
                ContentResponse::consumed(false)
            }
            ContentEvent::PointerUp(p)
                if matches!(
                    self.view.pointer_up(&mut self.model, *p),
                    OverlayEvent::Dismissed { .. }
                ) =>
            {
                ContentResponse::consumed(true)
            }
            ContentEvent::Key { key, down }
                if matches!(
                    self.view.key_event(&mut self.model, *key, *down),
                    OverlayEvent::Dismissed { .. }
                ) =>
            {
                ContentResponse::consumed(true)
            }
            _ => ContentResponse::default(),
        }
    }
    fn event(&mut self, event: &ContentEvent) -> ContentResponse {
        if matches!(event, ContentEvent::Cancel) {
            self.view.cancel();
            self.view.close(&mut self.model);
            self.view.clear_presentation();
        }
        ContentResponse::default()
    }
    fn focusable(&self) -> bool {
        self.model.open && self.view.logical_focus_target().is_some()
    }
    fn set_focus(&mut self, focused: bool, visible: bool) {
        self.dismiss_focused = focused;
        self.view.focus_dismiss(focused, visible);
    }
    fn semantic_frame(&self, _: Rect) -> Rect {
        self.view.layout().frame
    }
    fn semantics(&self) -> ContentSemantics {
        ContentSemantics::new(ContentRole::Group, &self.name)
    }
    fn semantic_parts(&self) -> Vec<SemanticPart> {
        self.model
            .open
            .then(|| self.view.logical_focus_target())
            .flatten()
            .map(|frame| SemanticPart {
                index: 0,
                frame,
                focused: self.dismiss_focused,
                semantics: ContentSemantics::new(ContentRole::Button, "Dismiss"),
            })
            .into_iter()
            .collect()
    }
    fn accessibility_action(
        &mut self,
        part: Option<usize>,
        action: &AccessibilityAction,
    ) -> ContentResponse {
        if self.model.open && part == Some(0) && matches!(action, AccessibilityAction::Press) {
            ContentResponse::consumed(self.close())
        } else {
            ContentResponse::default()
        }
    }
}
