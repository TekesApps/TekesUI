//! Business-owned row identities and menu commands on the original ContentHost.
//! No popup geometry, secondary host, synthetic trigger or selected-row polling.
use tekes_ui_controls::{
    TextSurface,
    composition::*,
    extended::{Axis, Stack},
};
pub const ROOT: ComponentId = ComponentId(24400);
pub const A: ComponentId = ComponentId(24401);
pub const B: ComponentId = ComponentId(24402);
pub const DRAFT: ComponentId = ComponentId(24403);
pub const MENU_A: ComponentId = ComponentId(24404);
pub const MENU_B: ComponentId = ComponentId(24405);
#[derive(Debug)]
pub struct Sidebar {
    pub host: ContentHost,
    pub primary_selection: &'static str,
    pub commands: Vec<(&'static str, Vec<String>)>,
    /// Immutable original selections retained for the independent review host.
    pub selections: Vec<MenuSelection>,
    pub a: ContextMenuBinding,
    pub b: ContextMenuBinding,
}
impl Default for Sidebar {
    fn default() -> Self {
        Self::new()
    }
}
impl Sidebar {
    pub fn new() -> Self {
        let mut draft = TextFieldContent::new("Composer A", "");
        draft.surface = TextSurface::new("Draft belongs to A", true, false);
        let menu = || {
            MenuContent::context_menu(
                "Session actions",
                vec![
                    MenuEntry::item("pin", "Pin"),
                    MenuEntry::submenu("more", "More", vec![MenuEntry::item("archive", "Archive")]),
                ],
            )
            .unwrap()
        };
        let mut host = ContentHost::new(Content::new(
            ROOT,
            StackContent {
                model: Stack {
                    axis: Axis::Vertical,
                    gap: 8.,
                },
                flexible_child: None,
                children: vec![
                    Content::new(A, Button::new("Session A")),
                    Content::new(B, Button::new("Session B")),
                    Content::new(DRAFT, draft),
                    Content::new(MENU_A, menu()),
                    Content::new(MENU_B, menu()),
                ],
            },
        ))
        .unwrap();
        host.set_action_events(true);
        let a = host.bind_context_menu(A, MENU_A).unwrap();
        let b = host.bind_context_menu(B, MENU_B).unwrap();
        Self {
            host,
            primary_selection: "session-a",
            commands: Vec::new(),
            selections: Vec::new(),
            a,
            b,
        }
    }
    pub fn collect(&mut self) {
        for record in self.host.drain_action_records() {
            if let Some(ContentActionPayload::MenuSelection(value)) = record.payload {
                self.selections.push(value.clone());
                let owner = value
                    .context_request()
                    .expect("only contextual menus")
                    .owner();
                let session = match owner {
                    A => "session-a",
                    B => "session-b",
                    _ => continue,
                };
                self.commands
                    .push((session, value.value_path().unwrap().to_vec()));
            } else if record.action.target == A {
                self.primary_selection = "session-a";
            } else if record.action.target == B {
                self.primary_selection = "session-b";
            }
        }
    }
}
