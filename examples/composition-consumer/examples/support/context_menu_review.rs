//! One fixed native acceptance script for the public context-menu consumer.
//! Layout/setup uses public Content controls; all tested pointer/key input uses
//! the existing AppKit selftest adapter. No popup geometry or menu engine lives here.
use std::path::Path;
use tekes_ui_composition_consumer::context_menu::{A, B, DRAFT, MENU_A, MENU_B, ROOT, Sidebar};
use tekes_ui_controls::{
    ButtonTextRenderer, ScrollArea,
    composition::*,
    extended::{Axis, Stack},
    layout::MeasuredChild,
};
use tekes_ui_platform_macos::{self as p, selftest::Action};
use tekes_ui_scene::{Point, Rect, Size};
const SCROLL: ComponentId = ComponentId(24410);
const ROWS: ComponentId = ComponentId(24411);
const SPACE: ComponentId = ComponentId(24412);
pub const FIRST: u32 = 44001;
pub const LAST: u32 = 44023;
#[derive(Debug)]
struct Space;
impl Component for Space {
    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, _: Size) -> MeasuredChild {
        MeasuredChild::new(Size::new(24., 420.))
    }
}
pub fn selection_records(sidebar: &Sidebar) -> Vec<serde_json::Value> {
    sidebar.selections.iter().map(|selection| {
        let request=selection.context_request().unwrap();
        serde_json::json!({"owner":request.owner().0,"owner_mount":request.owner_mount(),"source":format!("{:?}",request.source()),"anchor_identity":request.anchor_identity(),"model_revision":selection.model_revision(),"value_path":selection.value_path()})
    }).collect()
}
pub struct Review {
    pending: Option<u32>,
    completed: Vec<u32>,
    original_b: u64,
    menu_b: u64,
    retired: Option<Content>,
}
impl Review {
    pub fn prepare(sidebar: &mut Sidebar) -> Self {
        let a = sidebar.host.remove(A).unwrap();
        let b = sidebar.host.remove(B).unwrap();
        let rows = Content::new(
            ROWS,
            StackContent {
                model: Stack {
                    axis: Axis::Vertical,
                    gap: 8.,
                },
                flexible_child: None,
                children: vec![a, b, Content::new(SPACE, Space)],
            },
        );
        let mut scroll = ScrollContent::new(ScrollArea::default(), rows);
        scroll.viewport_height = Some(90.);
        sidebar
            .host
            .insert(ROOT, Content::new(SCROLL, scroll))
            .unwrap();
        sidebar
            .host
            .reorder(ROOT, &[SCROLL, DRAFT, MENU_A, MENU_B])
            .unwrap();
        sidebar.a = sidebar.host.bind_context_menu(A, MENU_A).unwrap();
        sidebar.b = sidebar.host.bind_context_menu(B, MENU_B).unwrap();
        for id in [MENU_A, MENU_B] {
            sidebar
                .host
                .get_mut(id)
                .unwrap()
                .component_mut::<MenuContent>()
                .unwrap()
                .set_entries(vec![
                    MenuEntry::Item {
                        value: "pin".into(),
                        label: "Pin".into(),
                        checked: Some(true),
                        disabled: false,
                    },
                    MenuEntry::submenu("more", "More", vec![MenuEntry::item("archive", "Archive")]),
                    MenuEntry::Item {
                        value: "blocked".into(),
                        label: "Unavailable".into(),
                        checked: None,
                        disabled: true,
                    },
                ])
                .unwrap();
        }
        Self {
            pending: None,
            completed: Vec::new(),
            original_b: sidebar.host.get(B).unwrap().mount_identity(),
            menu_b: sidebar.host.get(MENU_B).unwrap().mount_identity(),
            retired: None,
        }
    }
    pub fn phase(&self) -> Option<u32> {
        self.pending
    }
    pub fn script(&self, sidebar: &Sidebar) -> Vec<(u64, Action)> {
        let current_menu = sidebar.host.get(MENU_B).unwrap().mount_identity();
        assert_eq!(
            current_menu, self.menu_b,
            "menu owner mount changed during review setup"
        );
        eprintln!(
            "CONTEXT_NATIVE_IDENTITIES original_b={} current_b={} menu_b={current_menu}",
            self.original_b,
            sidebar.host.get(B).unwrap().mount_identity()
        );

        let command = |id| Action::MenuCommand(id);
        let right = || Action::ClickPublished {
            label: "Session B",
            secondary: true,
        };
        let left = |label| Action::ClickPublished {
            label,
            secondary: false,
        };
        let key = |code, chars| Action::KeyCharacters {
            key_code: code,
            chars,
        };
        vec![
            (
                200,
                Action::AXSelect {
                    component: sidebar.host.get(DRAFT).unwrap().mount_identity(),
                    range: 0..8,
                },
            ),
            (600, right()),
            (
                1000,
                Action::AuditAXMenu {
                    component: self.menu_b,
                },
            ),
            (1400, command(44001)),
            (1800, left("Unavailable")),
            (2200, command(44002)),
            (2600, left("More")),
            (
                3000,
                Action::AuditAXMenu {
                    component: self.menu_b,
                },
            ),
            (3400, command(44003)),
            (3800, left("Archive")),
            (4200, command(44004)),
            (4600, command(44005)),
            (
                5000,
                Action::KeyCharactersModified {
                    key_code: 109,
                    chars: "\u{f70d}",
                    modifiers: p::Modifiers {
                        shift: true,
                        ..p::Modifiers::NONE
                    },
                },
            ),
            (
                5400,
                Action::AuditAXMenu {
                    component: self.menu_b,
                },
            ),
            (5800, command(44006)),
            (6200, key(36, "\r")),
            (6600, command(44007)),
            (
                7000,
                Action::AXSelect {
                    component: sidebar.host.get(DRAFT).unwrap().mount_identity(),
                    range: 0..8,
                },
            ),
            (7400, right()),
            (
                7800,
                Action::ScrollWheel {
                    x: 30.,
                    y: 28.,
                    delta_x: 0,
                    delta_y: -8,
                },
            ),
            (8400, command(44008)),
            (8800, command(44009)),
            (9200, right()),
            (9600, command(44010)),
            (10000, command(44011)),
            (10400, command(44012)),
            (10800, right()),
            (11200, command(44013)),
            (11600, Action::ExpectAXMissing(self.original_b)),
            (12000, command(44014)),
            (12400, right()),
            (12800, command(44015)),
            (13200, command(44016)),
            (13600, right()),
            (14000, command(44017)),
            (14400, key(53, "\u{1b}")),
            (14800, command(44018)),
            (15200, Action::Resize { w: 320., h: 260. }),
            (15800, right()),
            (
                16200,
                Action::AuditAXMenu {
                    component: self.menu_b,
                },
            ),
            (16600, command(44019)),
            (17000, left("Pin")),
            (17400, command(44020)),
            (17800, right()),
            (18200, Action::Click { x: 8., y: 8. }),
            (18600, command(44021)),
            (19000, right()),
            (19400, Action::Miniaturize),
            (20200, Action::Deminiaturize),
            (21000, command(44022)),
            (21400, command(44023)),
            (22400, Action::Close),
        ]
    }
    pub fn command(&mut self, id: u32, sidebar: &mut Sidebar) -> bool {
        if !(FIRST..=LAST).contains(&id) {
            return false;
        }
        match id {
            44005 => {
                assert!(sidebar.host.focus(Some(B), true));
            }
            44009 => {
                sidebar
                    .host
                    .get_mut(SCROLL)
                    .unwrap()
                    .component_mut::<ScrollContent>()
                    .unwrap()
                    .scroll
                    .offset = Point::default();
            }
            44010 => {
                sidebar
                    .host
                    .get_mut(SCROLL)
                    .unwrap()
                    .component_mut::<ScrollContent>()
                    .unwrap()
                    .viewport_height = Some(8.);
            }
            44012 => {
                sidebar
                    .host
                    .get_mut(SCROLL)
                    .unwrap()
                    .component_mut::<ScrollContent>()
                    .unwrap()
                    .viewport_height = Some(90.);
            }
            44013 => {
                self.retired = Some(sidebar.host.remove(B).unwrap());
            }
            44014 => {
                sidebar
                    .host
                    .insert(ROWS, self.retired.take().unwrap())
                    .unwrap();
                sidebar.host.reorder(ROWS, &[A, B, SPACE]).unwrap();
            }
            44016 => {
                sidebar.b = sidebar.host.bind_context_menu(B, MENU_B).unwrap();
            }
            44018 => {
                // The tall spacer belongs to the completed scroll checks. A
                // horizontal Stack centers shorter rows on that cross axis;
                // retire the spacer so the edge-click target stays visible.
                sidebar.host.remove(SPACE).unwrap();
                sidebar
                    .host
                    .get_mut(ROWS)
                    .unwrap()
                    .component_mut::<StackContent>()
                    .unwrap()
                    .model
                    .axis = Axis::Horizontal;
            }
            _ => {}
        }
        self.pending = Some(id);
        true
    }
    pub fn verify(&mut self, sidebar: &Sidebar, viewport: Rect, evidence: Option<&Path>) {
        let Some(id) = self.pending.take() else {
            return;
        };
        let mut errors = Vec::new();
        let open = |menu| {
            sidebar
                .host
                .get(menu)
                .unwrap()
                .component::<MenuContent>()
                .unwrap()
                .menu
                .is_open()
        };
        let count = sidebar.commands.len();
        if sidebar.primary_selection != "session-a" {
            errors.push("primary selection changed".to_string());
        }
        let draft = &sidebar
            .host
            .get(DRAFT)
            .unwrap()
            .component::<TextFieldContent>()
            .unwrap()
            .surface
            .input;
        if draft.text != "Draft belongs to A"
            || draft.selection() != (0..8)
            || draft.marked.is_some()
        {
            errors.push("original draft text/selection/marked changed".into());
        }
        let expected_count = match id {
            44001..=44003 => 0,
            44004..=44006 => 1,
            44007..=44019 => 2,
            _ => 3,
        };
        if count != expected_count {
            errors.push(format!(
                "expected {expected_count} immutable commands, observed {count}"
            ));
        }
        let wanted_open = matches!(id, 44001 | 44002 | 44003 | 44006 | 44017 | 44019);
        if open(MENU_B) != wanted_open || open(MENU_A) {
            errors.push(format!("unexpected popup lifetime at {id}"));
        }
        if id <= 44021 && !matches!(id, 44005..=44007) && sidebar.host.focused() != Some(DRAFT) {
            errors.push(format!("original Composer focus lost at {id}"));
        }
        // Existing ContentHost::set_window_active(false) calls cancel(), which
        // explicitly clears focus. Reactivation must not invent a return owner.
        if id >= 44022 && sidebar.host.focused().is_some() {
            errors.push("deactivation/return unexpectedly revived focus".into());
        }
        if matches!(id, 44005..=44007) && sidebar.host.focused() != Some(B) {
            errors.push("keyboard context owner focus changed".into());
        }
        if id == 44003
            && !sidebar
                .host
                .semantics()
                .iter()
                .flat_map(|node| &node.parts)
                .any(|part| part.semantics.name == "Archive")
        {
            errors.push("original submenu not published".into());
        }
        if id == 44008
            && sidebar
                .host
                .get(SCROLL)
                .unwrap()
                .component::<ScrollContent>()
                .unwrap()
                .scroll
                .offset
                .y
                <= 0.
        {
            errors.push("real native scroll did not move original viewport".into());
        }
        if matches!(id, 44010 | 44011)
            && (sidebar.host.get(B).unwrap().visible().is_some()
                || sidebar.host.can_show_context_menu(B))
        {
            errors.push("clipped anchor remained eligible".into());
        }
        if id == 44013 && sidebar.host.get(B).is_some() {
            errors.push("removed original row remains mounted".into());
        }
        if matches!(id, 44014 | 44015) && sidebar.host.can_show_context_menu(B) {
            errors.push("remounted row inherited stale context binding".into());
        }
        if id == 44017
            && (sidebar.host.get(B).unwrap().mount_identity() == self.original_b
                || !sidebar.host.can_show_context_menu(B))
        {
            errors.push("new original mount was not explicitly rebound".into());
        }
        if matches!(id, 44018 | 44019) {
            let row = sidebar.host.get(B).unwrap();
            if !row.visible().is_some_and(|visible| visible.contains(row.frame().center()))
                || !sidebar.host.can_show_context_menu(B)
            {
                errors.push("edge-click original owner is outside its visible viewport".into());
            }
            eprintln!("CONTEXT_NATIVE_EDGE_OWNER phase={id} frame={:?} visible={:?}", row.frame(), row.visible());
        }
        if id == 44019 {
            let menu = sidebar
                .host
                .get(MENU_B)
                .unwrap()
                .component::<MenuContent>()
                .unwrap();
            if let Some(layout) = &menu.popup {
                let frame = layout.popup;
                if frame.min_x() < viewport.min_x()
                    || frame.max_x() > viewport.max_x()
                    || frame.min_y() < viewport.min_y()
                    || frame.max_y() > viewport.max_y()
                {
                    errors.push("edge popup escaped native viewport".into());
                }
                if frame.min_x() >= layout.anchor.min_x() {
                    errors.push("narrow right edge did not exercise SDK clamp".into());
                }
            } else {
                errors.push("edge popup has no real MenuLayout".into());
            }
        }
        for (index, selection) in sidebar.selections.iter().enumerate() {
            let request = selection.context_request().unwrap();
            let path = if index == 0 {
                vec!["more".to_string(), "archive".to_string()]
            } else {
                vec!["pin".to_string()]
            };
            let source = if index == 1 {
                ContentActionSource::Keyboard
            } else {
                ContentActionSource::Pointer
            };
            let mount = if index < 2 {
                self.original_b
            } else {
                sidebar.host.get(B).unwrap().mount_identity()
            };
            if request.owner() != B
                || request.owner_mount() != mount
                || request.source() != source
                || selection.value_path() != Some(path.as_slice())
            {
                errors.push(format!(
                    "selection {index} lost original owner/mount/source/value"
                ));
            }
        }
        if id == LAST && self.completed != (FIRST..LAST).collect::<Vec<_>>() {
            errors.push("native acceptance checkpoints were skipped or reordered".into());
        }
        let report = serde_json::json!({"phase":id,"passed":errors.is_empty(),"errors":errors,"commands":sidebar.commands,"selections":selection_records(sidebar),"original_b_mount":self.original_b,"current_b_mount":sidebar.host.get(B).map(Content::mount_identity),"focused":sidebar.host.focused().map(|id|id.0),"native_software_only":true,"physical_ime":"not_run","voiceover":"not_run"});
        if let Some(path) = evidence {
            std::fs::create_dir_all(path).unwrap();
            std::fs::write(
                path.join(format!("checkpoint-{id}.json")),
                serde_json::to_vec_pretty(&report).unwrap(),
            )
            .unwrap();
        }
        eprintln!("CONTEXT_NATIVE_CHECK {report}");
        assert!(
            errors.is_empty(),
            "native context checkpoint {id} failed: {errors:?}"
        );
        self.completed.push(id);
    }
}
