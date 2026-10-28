//! Public SDK PopoverContent with original form controls, never a second host.
use tekes_ui_controls::{
    ButtonTextRenderer, ControlKey,
    composition::*,
    extended::{Axis, Stack},
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
pub const CANCEL: ComponentId = ComponentId(9108);
pub const SAVE: ComponentId = ComponentId(9109);
const ACTIONS: ComponentId = ComponentId(9110);
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
                POPOVER => f,
                _ => unreachable!(),
            };
            c.arrange(t, frame, clip);
        }
    }
}
pub struct Scenario {
    pub host: ContentHost,
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
                    Content::new(
                        ACTIONS,
                        StackContent {
                            model: Stack {
                                axis: Axis::Horizontal,
                                gap: 8.,
                            },
                            flexible_child: None,
                            children: vec![
                                Content::new(CANCEL, Button::new("Cancel")),
                                Content::new(SAVE, Button::new("Save")),
                            ],
                        },
                    ),
                ],
            },
        );
        let mut card = PopoverContent::new("Preferences popover", body);
        card.initial_focus = Some(EDITOR);
        card.max_content_width = Some(340.); // Existing form preference; SDK owns shell geometry.
        let mut host = ContentHost::new(Content::new(
            ROOT,
            Screen(vec![
                Content::new(CALLER, Button::new("Edit preferences")),
                Content::new(OTHER, Button::new("Unrelated action")),
                Content::new(POPOVER, card),
            ]),
        ))?;
        host.set_action_events(true);
        Ok(Self { host })
    }
    /// Same retained form/editor, with only its declared Cancel/Save actions.
    pub fn form() -> Result<Self, CompositionError> {
        let mut scenario = Self::new()?;
        scenario
            .host
            .get_mut(POPOVER)
            .unwrap()
            .component_mut::<PopoverContent>()
            .unwrap()
            .set_dismiss_visible(false);
        Ok(scenario)
    }
    pub fn is_open(&self) -> bool {
        self.host
            .get(POPOVER)
            .and_then(|c| c.component::<PopoverContent>())
            .is_some_and(PopoverContent::is_open)
    }
    pub fn is_presented(&self) -> bool {
        self.host
            .get(POPOVER)
            .and_then(|c| c.component::<PopoverContent>())
            .is_some_and(Component::overlay_presented)
    }
    pub fn open(&mut self) {
        if self.is_open() {
            return;
        }
        open(&mut self.host);
    }
    pub fn process(&mut self) {
        process_actions(&mut self.host);
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
        self.host.set_window_active(false);
        self.host.drain_actions();
    }
}

/// Native/independent consumers share this same scenario; no second ContentHost.
pub fn open(host: &mut ContentHost) {
    host.show_popover(POPOVER, CALLER)
        .expect("original Preferences owner and invoker must be available");
}
pub fn process_actions(host: &mut ContentHost) {
    for action in host.drain_actions() {
        if action.target == CALLER {
            open(host);
        }
        if matches!(action.target, CANCEL | SAVE) {
            let session = host
                .get(POPOVER)
                .unwrap()
                .component::<PopoverContent>()
                .unwrap()
                .current_session();
            if let Some(session) = session {
                host.close_popover(session);
            }
        }
    }
}
