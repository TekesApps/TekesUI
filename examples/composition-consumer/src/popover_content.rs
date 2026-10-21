//! A consumer-defined overlay with real owned Content, never a second host.
use tekes_ui_controls::{
    ButtonTextRenderer, ControlKey,
    composition::*,
    extended::{Axis, Popover, Stack},
    feedback_view::{OverlayEvent, PopoverView},
    layout::MeasuredChild,
};
use tekes_ui_scene::{Rect, Scene, Size};
use tekes_ui_tokens::proposal;
pub const ROOT: ComponentId = ComponentId(9100);
pub const CALLER: ComponentId = ComponentId(9101);
pub const OTHER: ComponentId = ComponentId(9102);
pub const POPOVER: ComponentId = ComponentId(9103);
pub const BODY: ComponentId = ComponentId(9104);
pub const EDITOR: ComponentId = ComponentId(9105);
pub const SWITCH: ComponentId = ComponentId(9106);
const CAPTION: ComponentId = ComponentId(9107);
#[derive(Debug)]
struct Caption;
impl Component for Caption {
    fn measure(&mut self, t: &mut dyn ButtonTextRenderer, a: Size) -> MeasuredChild {
        MeasuredChild::new(t.measure(
            "Consumer-owned preferences · 自定义内容",
            proposal::TextRole::UiCaption,
            a.width,
        ))
    }
    fn paint(
        &mut self,
        scene: &mut Scene,
        t: &mut dyn ButtonTextRenderer,
        frame: Rect,
        _: Rect,
        theme: ContentTheme,
    ) {
        t.draw(
            scene,
            "Consumer-owned preferences · 自定义内容",
            proposal::TextRole::UiCaption,
            frame,
            tekes_ui_controls::ButtonTextAlign::Leading,
            theme.palette.color("primary-text").unwrap(),
        );
    }
    fn semantics(&self) -> ContentSemantics {
        ContentSemantics::new(ContentRole::Text, "Consumer-owned preferences · 自定义内容")
    }
}
#[derive(Debug)]
struct Card {
    model: Popover,
    view: PopoverView,
    children: Vec<Content>,
    anchor: Rect,
    size: Size,
    dismiss_focused: bool,
}
impl Component for Card {
    fn children(&self) -> &[Content] {
        &self.children
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.children)
    }
    fn fixed_slots(&self) -> bool {
        true
    }
    fn active(&self) -> bool {
        self.model.open
    }
    fn paints_children(&self) -> bool {
        true
    }
    fn overlay_open(&self) -> bool {
        self.model.open
    }
    fn overlay_presented(&self) -> bool {
        self.model.open || self.view.is_presented()
    }
    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, a: Size) -> MeasuredChild {
        MeasuredChild::new(a)
    }
    fn arrange(&mut self, mut t: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        let available = Size::new(
            (frame.size.width - 64.).clamp(0., 340.),
            (frame.size.height - 128.).max(0.),
        );
        let measured = self.children[0].measure(t, available).size;
        self.size = PopoverView::measure(&mut t, measured, proposal::TextRole::UiBody);
        self.size.width = self.size.width.min(frame.size.width.max(0.));
        self.size.height = self.size.height.min(frame.size.height.max(0.));
        let layout = self.view.arrange(
            &mut t,
            &self.model,
            self.anchor,
            self.size,
            frame,
            Some(clip),
            proposal::TextRole::UiBody,
        );
        self.children[0].arrange(t, layout.content, layout.visible.unwrap_or_default());
    }
    fn paint_overlay(
        &mut self,
        scene: &mut Scene,
        mut t: &mut dyn ButtonTextRenderer,
        frame: Rect,
        clip: Rect,
        theme: ContentTheme,
    ) {
        let body = &mut self.children[0];
        self.view.render_with_text_content(
            scene,
            &mut t,
            &self.model,
            self.anchor,
            self.size,
            frame,
            Some(clip),
            theme.palette,
            proposal::TextRole::UiBody,
            "consumer-owned-v1",
            |scene, text, _| {
                body.paint(scene, text, theme);
                body.paint_overlays(scene, text, theme);
            },
        );
        if self.view.needs_animation() {
            t.request_animation_frame();
        }
    }
    fn overlay_event(&mut self, event: &ContentEvent) -> ContentResponse {
        let dismiss_key = self.model.open
            && self.dismiss_focused
            && matches!(
                event,
                ContentEvent::Key {
                    key: ControlKey::Space | ControlKey::Enter,
                    ..
                }
            );
        let result = match event {
            ContentEvent::PointerMove(p) => {
                self.view.pointer_move(*p);
                return ContentResponse {
                    changed: true,
                    ..Default::default()
                };
            }
            ContentEvent::PointerDown(p) => {
                let handled = self.view.pointer_down(&mut self.model, *p);
                return ContentResponse {
                    consumed: handled,
                    changed: handled,
                    ..Default::default()
                };
            }
            ContentEvent::PointerUp(p) => self.view.pointer_up(&mut self.model, *p),
            ContentEvent::Key { key, down } => self.view.key_event(&mut self.model, *key, *down),
            _ => OverlayEvent::None,
        };
        if matches!(result, OverlayEvent::Dismissed { .. }) || dismiss_key {
            ContentResponse::consumed(true)
        } else {
            ContentResponse::default()
        }
    }
    fn event(&mut self, event: &ContentEvent) -> ContentResponse {
        if matches!(event, ContentEvent::Cancel) {
            self.view.cancel();
        }
        ContentResponse::default()
    }
    fn focusable(&self) -> bool {
        self.model.open
    }
    fn set_focus(&mut self, focused: bool, visible: bool) {
        self.dismiss_focused = focused;
        self.view.focus_dismiss(focused, visible);
    }
    fn semantic_frame(&self, _: Rect) -> Rect {
        self.view.layout().frame
    }
    fn semantics(&self) -> ContentSemantics {
        ContentSemantics::new(ContentRole::Group, "Preferences popover")
    }
    fn semantic_parts(&self) -> Vec<SemanticPart> {
        if !self.model.open {
            return vec![];
        }
        self.view
            .logical_focus_target()
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
            self.view.close(&mut self.model);
            ContentResponse::consumed(true)
        } else {
            ContentResponse::default()
        }
    }
    fn lifecycle(&mut self, event: Lifecycle) {
        if event == Lifecycle::Unmounted {
            self.view.close(&mut self.model);
            self.view.clear_presentation();
        }
    }
}
#[derive(Debug)]
struct Screen(Vec<Content>);
impl Component for Screen {
    fn children(&self) -> &[Content] {
        &self.0
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.0)
    }
    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, a: Size) -> MeasuredChild {
        MeasuredChild::new(a)
    }
    fn arrange(&mut self, t: &mut dyn ButtonTextRenderer, f: Rect, clip: Rect) {
        let anchor = Rect::new(f.min_x() + 24., f.min_y() + 24., 160., 40.);
        for c in &mut self.0 {
            let frame = match c.id() {
                CALLER => anchor,
                OTHER => Rect::new(f.min_x() + 24., f.min_y() + 88., 220., 40.),
                POPOVER => {
                    c.component_mut::<Card>().unwrap().anchor = anchor;
                    f
                }
                _ => unreachable!(),
            };
            c.arrange(t, frame, clip);
        }
    }
}
pub struct Scenario {
    pub host: ContentHost,
    restore: Option<(ComponentId, u64)>,
}
impl Scenario {
    pub fn new() -> Result<Self, CompositionError> {
        let body = Content::new(
            BODY,
            StackContent {
                model: Stack {
                    axis: Axis::Vertical,
                    gap: 8.,
                },
                flexible_child: None,
                children: vec![
                    Content::new(CAPTION, Caption),
                    Content::new(
                        EDITOR,
                        TextFieldContent::new("Workspace name", "Retained draft · 草稿 A😀B"),
                    ),
                    Content::new(SWITCH, SwitchContent::new("Keep local settings", false)),
                ],
            },
        );
        let card = Card {
            model: Popover::default(),
            view: PopoverView::default(),
            children: vec![body],
            anchor: Rect::default(),
            size: Size::default(),
            dismiss_focused: false,
        };
        let mut host = ContentHost::new(Content::new(
            ROOT,
            Screen(vec![
                Content::new(CALLER, ButtonContent::new("Edit preferences")),
                Content::new(OTHER, ButtonContent::new("Unrelated action")),
                Content::new(POPOVER, card),
            ]),
        ))?;
        host.set_action_events(true);
        Ok(Self {
            host,
            restore: None,
        })
    }
    pub fn is_open(&self) -> bool {
        self.host
            .get(POPOVER)
            .and_then(|c| c.component::<Card>())
            .is_some_and(|c| c.model.open)
    }
    pub fn is_presented(&self) -> bool {
        self.host
            .get(POPOVER)
            .and_then(|c| c.component::<Card>())
            .is_some_and(Component::overlay_presented)
    }
    pub fn open(&mut self) {
        if self.is_open() {
            return;
        }
        self.restore = self.host.capture_focus();
        let card = self
            .host
            .get_mut(POPOVER)
            .unwrap()
            .component_mut::<Card>()
            .unwrap();
        card.view.set_window_active(true, &mut card.model);
        card.view.show(&mut card.model, CALLER.0 as usize);
        self.host.focus(Some(EDITOR), true);
    }
    pub fn process(&mut self) {
        for action in self.host.drain_actions() {
            if action.target == CALLER {
                self.open();
            }
        }
        if !self.is_open() && self.restore.is_some() {
            let restore = self.restore.take();
            // Nonmodal content may already have yielded focus to another
            // control. Only finish an intent whose old owner was cleared.
            if self.host.focused().is_none() {
                self.host.restore_focus(restore);
            }
        }
    }
    pub fn escape(&mut self) {
        self.host.event(&ContentEvent::Key {
            key: ControlKey::Escape,
            down: true,
        });
        self.process();
    }
    /// Platform host boundary: inactive windows close transient presentation
    /// without returning focus or retaining an unhandled presentation action.
    pub fn deactivate(&mut self) {
        self.restore = None;
        if let Some(card) = self
            .host
            .get_mut(POPOVER)
            .and_then(|c| c.component_mut::<Card>())
        {
            card.view.set_window_active(false, &mut card.model);
        }
        self.host.event(&ContentEvent::Cancel);
        self.host.drain_actions();
    }
}
