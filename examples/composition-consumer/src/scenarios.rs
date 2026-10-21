//! Independent List and Rich Dialog composition consumers.
use super::*;
use std::cell::Cell;
use std::rc::Rc;
use tekes_ui_controls::{Button, ControlKey, ScrollArea, extended::Grid};

pub mod ids {
    use tekes_ui_controls::composition::ComponentId;
    pub const LIST: ComponentId = ComponentId(100);
    pub const LIST_TEXT: ComponentId = ComponentId(101);
    pub const LIST_BADGE: ComponentId = ComponentId(102);
    pub const LIST_SWITCH: ComponentId = ComponentId(103);
    pub const LIST_FIELD: ComponentId = ComponentId(104);
    pub const LIST_MENU: ComponentId = ComponentId(105);
    pub const LIST_ROW: ComponentId = ComponentId(106);
    pub const LIST_ROW_SWITCH: ComponentId = ComponentId(107);
    pub const DIALOG_SCREEN: ComponentId = ComponentId(200);
    pub const DIALOG_INVOKER: ComponentId = ComponentId(201);
    pub const DIALOG: ComponentId = ComponentId(202);
    pub const DIALOG_HEADER: ComponentId = ComponentId(203);
    pub const DIALOG_BODY: ComponentId = ComponentId(204);
    pub const DIALOG_GRID: ComponentId = ComponentId(205);
    pub const DIALOG_FIELD: ComponentId = ComponentId(206);
    pub const DIALOG_SWITCH: ComponentId = ComponentId(207);
    pub const DIALOG_HELP: ComponentId = ComponentId(208);
    pub const DIALOG_ERROR: ComponentId = ComponentId(209);
    pub const DIALOG_FOOTER: ComponentId = ComponentId(210);
    pub const DIALOG_CANCEL: ComponentId = ComponentId(211);
    pub const DIALOG_SAVE: ComponentId = ComponentId(212);
    pub const DIALOG_CONFIRM: ComponentId = DIALOG_SAVE;
}

#[derive(Debug)]
struct Badge(String);
impl Component for Badge {
    fn measure(&mut self, t: &mut dyn ButtonTextRenderer, a: Size) -> MeasuredChild {
        let measured = t.measure(&self.0, proposal::TextRole::UiCaption, a.width);
        MeasuredChild::new(Size::new(
            measured.width + Spacing::Regular.value(),
            measured.height + Spacing::Tight.value(),
        ))
    }
    fn paint(
        &mut self,
        s: &mut Scene,
        t: &mut dyn ButtonTextRenderer,
        f: Rect,
        _: Rect,
        theme: ContentTheme,
    ) {
        paint::surface_at_scale(s, f, theme.palette, true, t.backing_scale());
        t.draw(
            s,
            &self.0,
            proposal::TextRole::UiCaption,
            f,
            ButtonTextAlign::Center,
            theme.palette.color("primary-text").unwrap(),
        );
    }
}

/// A row owns its own action while the nested Switch owns its hit target.
#[derive(Debug)]
struct ActionRow {
    pub children: Vec<Content>,
    activations: Rc<Cell<usize>>,
    action: Button,
    frame: Rect,
    clip: Rect,
}
impl Component for ActionRow {
    fn accessibility_action(
        &mut self,
        part: Option<usize>,
        action: &AccessibilityAction,
    ) -> ContentResponse {
        if part.is_none() && matches!(action, AccessibilityAction::Press) {
            return self.event(&ContentEvent::Key {
                key: ControlKey::Enter,
                down: true,
            });
        }
        ContentResponse::default()
    }
    fn paint(
        &mut self,
        scene: &mut Scene,
        _: &mut dyn ButtonTextRenderer,
        frame: Rect,
        _: Rect,
        theme: ContentTheme,
    ) {
        if self.action.state.focused && self.action.state.focus_visible {
            paint::focus_ring(
                scene,
                frame,
                theme.palette,
                proposal::metric("radius", "radius-row").expect("JSON row radius"),
            );
        }
    }
    fn measure(&mut self, t: &mut dyn ButtonTextRenderer, a: Size) -> MeasuredChild {
        let mut stack = StackContent {
            flexible_child: None,
            model: Stack {
                axis: Axis::Horizontal,
                gap: Spacing::Regular.value(),
            },
            children: std::mem::take(&mut self.children),
        };
        let measured = stack.measure(t, a);
        self.children = stack.children;
        measured
    }
    fn arrange(&mut self, t: &mut dyn ButtonTextRenderer, f: Rect, c: Rect) {
        self.frame = f;
        self.clip = c;
        let mut stack = StackContent {
            flexible_child: None,
            model: Stack {
                axis: Axis::Horizontal,
                gap: Spacing::Regular.value(),
            },
            children: std::mem::take(&mut self.children),
        };
        stack.arrange(t, f, c);
        self.children = stack.children;
    }
    fn children(&self) -> &[Content] {
        &self.children
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.children)
    }
    fn interactive(&self) -> bool {
        true
    }
    fn event(&mut self, e: &ContentEvent) -> ContentResponse {
        let mut response = ContentResponse::default();
        match e {
            ContentEvent::PointerDown(_) => {
                self.action.pointer_down();
                response.consumed = self.action.state.pressed;
                response.capture_pointer = response.consumed;
                response.changed = response.consumed;
            }
            ContentEvent::PointerUp(point) => {
                let inside = self
                    .frame
                    .intersection(&self.clip)
                    .is_some_and(|r| r.contains(*point));
                response.activated = self.action.pointer_up(inside);
                response.consumed = response.activated || self.action.state.focused;
                response.changed = response.consumed;
            }
            ContentEvent::Key { key, down } => {
                response.activated = self.action.key(*key, *down);
                response.consumed = matches!(key, ControlKey::Enter | ControlKey::Space);
                response.changed = response.consumed;
            }
            ContentEvent::Cancel => {
                self.action.cancel();
                response.consumed = true;
                response.changed = true;
            }
            _ => {}
        }
        if response.activated {
            self.activations.set(self.activations.get() + 1);
        }
        response
    }
    fn focusable(&self) -> bool {
        true
    }
    fn set_focus(&mut self, focused: bool, focus_visible: bool) {
        if focused {
            self.action.state.focused = true;
            self.action.state.focus_visible = focus_visible;
        } else {
            self.action.blur();
        }
    }
    fn semantics(&self) -> ContentSemantics {
        ContentSemantics::new(ContentRole::ListItem, "Advanced row")
    }
}

pub struct ListFixture {
    pub host: ContentHost,
    pub row_activations: Rc<Cell<usize>>,
}
pub fn heterogeneous_list() -> Result<ListFixture, CompositionError> {
    let row_activations = Rc::new(Cell::new(0));
    let row = Content::new(
        ids::LIST_ROW,
        ActionRow {
            children: vec![Content::new(
                ids::LIST_ROW_SWITCH,
                SwitchContent::new("Sync changes", false),
            )],
            activations: row_activations.clone(),
            action: Button::default(),
            frame: Rect::default(),
            clip: Rect::default(),
        },
    );
    let list = ListContent::new(
        "Workspace preferences",
        vec![
            Content::new(
                ids::LIST_TEXT,
                TextContent {
                    text: "Preferences".into(),
                    role: proposal::TextRole::UiHeading,
                },
            ),
            Content::new(ids::LIST_BADGE, Badge("Managed".into())),
            Content::new(
                ids::LIST_SWITCH,
                SwitchContent::new("Use system appearance", true),
            ),
            Content::new(
                ids::LIST_FIELD,
                TextFieldContent::new("Workspace name", "Tekes"),
            ),
            Content::new(
                ids::LIST_MENU,
                MenuContent::new(
                    "Profile",
                    vec!["Personal".into(), "Team".into(), "Custom".into()],
                ),
            ),
            row,
        ],
    );
    Ok(ListFixture {
        host: ContentHost::new(Content::new(ids::LIST, list))?,
        row_activations,
    })
}

pub fn build_list() -> (ContentHost, ComponentId) {
    let fixture = heterogeneous_list().expect("valid independent List fixture");
    (fixture.host, ids::LIST_SWITCH)
}

#[derive(Debug)]
struct DialogScreen {
    children: Vec<Content>,
}
impl Component for DialogScreen {
    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, a: Size) -> MeasuredChild {
        MeasuredChild::new(a)
    }
    fn arrange(&mut self, t: &mut dyn ButtonTextRenderer, f: Rect, c: Rect) {
        if let Some(invoker) = self
            .children
            .iter_mut()
            .find(|n| n.id() == ids::DIALOG_INVOKER)
        {
            invoker.arrange(t, Rect::new(f.min_x() + 24., f.min_y() + 24., 240., 48.), c);
        }
        if let Some(dialog) = self.children.iter_mut().find(|n| n.id() == ids::DIALOG) {
            dialog.arrange(t, f, c);
        }
    }
    fn children(&self) -> &[Content] {
        &self.children
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.children)
    }
    fn semantics(&self) -> ContentSemantics {
        ContentSemantics::new(ContentRole::Group, "Settings screen")
    }
}

pub fn rich_dialog() -> Result<ContentHost, CompositionError> {
    let mut text_field = TextFieldContent::new("Display name", "Workspace");
    text_field.field.help = "Choose a name visible to everyone on the team.".into();
    let form = Content::new(
        ids::DIALOG_GRID,
        GridContent {
            model: Grid {
                columns: 2,
                gap_x: Spacing::Regular.value(),
                gap_y: Spacing::Regular.value(),
            },
            children: vec![
                Content::new(ids::DIALOG_FIELD, text_field),
                Content::new(
                    ids::DIALOG_SWITCH,
                    SwitchContent::new("Keep local settings", true),
                ),
                Content::new(
                    ids::DIALOG_HELP,
                    TextContent {
                        text: "This name is shown to collaborators.".into(),
                        role: proposal::TextRole::UiCaption,
                    },
                ),
                Content::new(
                    ids::DIALOG_ERROR,
                    TextContent {
                        text: "Changes take effect when you save this form.".into(),
                        role: proposal::TextRole::UiCaption,
                    },
                ),
            ],
        },
    );
    let body = Content::new(
        ids::DIALOG_BODY,
        ScrollContent::new(ScrollArea::default(), form),
    );
    let footer = Content::new(
        ids::DIALOG_FOOTER,
        StackContent {
            flexible_child: None,
            model: Stack {
                axis: Axis::Horizontal,
                gap: Spacing::Regular.value(),
            },
            children: vec![
                Content::new(ids::DIALOG_CANCEL, ButtonContent::new("Cancel")),
                Content::new(ids::DIALOG_SAVE, ButtonContent::new("Save")),
            ],
        },
    );
    let dialog = Content::new(
        ids::DIALOG,
        DialogContent::new(
            "Edit workspace",
            Content::new(
                ids::DIALOG_HEADER,
                TextContent {
                    text: "Workspace details".into(),
                    role: proposal::TextRole::UiHeading,
                },
            ),
            body,
            footer,
        ),
    );
    let invoker = Content::new(ids::DIALOG_INVOKER, ButtonContent::new("Edit workspace"));
    ContentHost::new(Content::new(
        ids::DIALOG_SCREEN,
        DialogScreen {
            children: vec![invoker, dialog],
        },
    ))
}

pub fn build_dialog() -> (ContentHost, ComponentId) {
    (
        rich_dialog().expect("valid independent Dialog fixture"),
        ids::DIALOG_INVOKER,
    )
}

/// Consumer-owned form validation writes Field's shared error contract so the
/// production FieldView and TextSurface render the error boundary together.
pub fn validate_dialog(host: &mut ContentHost) -> bool {
    let invalid = host
        .get(ids::DIALOG_FIELD)
        .and_then(|c| c.component::<TextFieldContent>())
        .is_none_or(|f| f.surface.input.text.trim().is_empty());
    let Some(field) = host
        .get_mut(ids::DIALOG_FIELD)
        .and_then(|c| c.component_mut::<TextFieldContent>())
    else {
        return false;
    };
    if invalid {
        field.field.error = Some("Enter a workspace name.".into());
        host.focus(Some(ids::DIALOG_FIELD), true);
        false
    } else {
        field.field.error = None;
        true
    }
}
