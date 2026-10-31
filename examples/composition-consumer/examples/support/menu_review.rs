//! Fixture-only assertions and AppKit script for the existing `native menu-tree` case.
//! All menu placement, input, paint, semantics and selection remain in MenuContent.
use tekes_ui_composition_consumer::declared_menu as f;
use tekes_ui_controls::{TextEditBaseline, composition::*};
use tekes_ui_platform_macos::{AccessibilityKey, Window, selftest::Action};
use tekes_ui_scene::Rect;

#[derive(Default)]
pub struct Review {
    pub scroll_events: usize,
    verified: Vec<u32>,
    editor_mount: Option<u64>,
    marked: Option<TextEditBaseline>,
    stale: Option<AccessibilityKey>,
    first_row: Option<Rect>,
    first_revision: Option<u64>,
}
fn parts(tree: &ContentHost) -> Vec<SemanticPart> {
    tree.semantics()
        .into_iter()
        .find(|node| node.id == f::OPTIONS)
        .unwrap()
        .parts
}
fn part(tree: &ContentHost, name: &str) -> SemanticPart {
    parts(tree)
        .into_iter()
        .find(|part| part.semantics.name == name)
        .unwrap()
}
fn key(tree: &ContentHost, name: &str) -> AccessibilityKey {
    AccessibilityKey {
        component: f::OPTIONS.0,
        part: Some(part(tree, name).index),
    }
}
fn click(frame: Rect) -> Action {
    let p = frame.center();
    Action::Click {
        x: p.x.into(),
        y: p.y.into(),
    }
}
fn trigger(tree: &ContentHost) -> Action {
    click(tree.get(f::OPTIONS).unwrap().frame())
}
fn selection(record: &ContentActionRecord) -> &MenuSelection {
    match record.payload.as_ref().unwrap() {
        ContentActionPayload::MenuSelection(value) => value,
        ContentActionPayload::ToolbarCommand(_)
        | ContentActionPayload::ComboBoxSelection(_)
        | ContentActionPayload::CheckboxState(_)
        | ContentActionPayload::RadioSelection(_)
        | ContentActionPayload::SegmentSelection(_)
        | ContentActionPayload::TabSelection { .. }
        | ContentActionPayload::BreadcrumbNavigation { .. }
        | ContentActionPayload::CommandSelection { .. }
        | ContentActionPayload::DisclosureExpansion(_) => {
            panic!("expected original Menu selection")
        }
    }
}
fn menu(tree: &mut ContentHost) -> &mut MenuContent {
    tree.get_mut(f::OPTIONS)
        .unwrap()
        .component_mut::<MenuContent>()
        .unwrap()
}
fn capture(name: &str) -> Action {
    Action::Capture {
        path: std::path::PathBuf::from(std::env::var_os("TEKESUI_COMPONENT_EVIDENCE").unwrap())
            .join(name)
            .to_string_lossy()
            .into_owned(),
    }
}
impl Review {
    pub fn advance(
        &mut self,
        stage: u32,
        tree: &mut ContentHost,
        records: &[ContentActionRecord],
        window: &Window,
    ) -> Vec<(u64, Action)> {
        assert!(
            window.is_active(),
            "native Menu window must actually be active"
        );
        assert_eq!(
            stage,
            36_001 + self.verified.len() as u32,
            "ordered native phases"
        );
        let next = |stage| Action::MenuCommand(stage);
        let script = match stage {
            36_001 => {
                assert!(records.is_empty());
                assert!(!menu(tree).menu.is_open());
                self.editor_mount = tree.semantic_mount(f::MESSAGE);
                vec![(150, trigger(tree)), (500, next(36_002))]
            }
            36_002 => {
                assert!(records.is_empty(), "opening has no selection callback");
                assert!(menu(tree).menu.is_open());
                assert!(part(tree, "Move Pin Up").semantics.disabled);
                self.stale = Some(key(tree, "Move Pin Down"));
                let p = part(tree, "Sort by").frame.center();
                vec![
                    (
                        100,
                        Action::MouseMove {
                            x: p.x.into(),
                            y: p.y.into(),
                        },
                    ),
                    (
                        400,
                        Action::KeyCharacters {
                            key_code: 125,
                            chars: "\u{f701}",
                        },
                    ),
                    (700, next(36_003)),
                ]
            }
            36_003 => {
                assert!(records.is_empty());
                let recent = part(tree, "Recent activity");
                let title = part(tree, "Title A–Z");
                assert!(recent.semantics.selected && !recent.focused);
                assert!(
                    !title.semantics.selected && title.focused,
                    "controlled check is distinct from original keyboard-active row"
                );
                assert_eq!(
                    parts(tree)
                        .iter()
                        .filter(|p| p.semantics.role == ContentRole::Menu)
                        .count(),
                    2
                );
                vec![
                    (100, capture("sort-checked-vs-focus.png")),
                    (350, click(title.frame)),
                    (650, next(36_004)),
                ]
            }
            36_004 => {
                assert_eq!(records.len(), 1);
                assert_eq!(records[0].action.source, ContentActionSource::Pointer);
                assert_eq!(
                    selection(&records[0]).value_path().unwrap(),
                    ["sort", "titleAscending"]
                );
                self.first_revision = Some(selection(&records[0]).model_revision());
                vec![(100, trigger(tree)), (400, next(36_005))]
            }
            36_005 => {
                let p = part(tree, "Group by").frame.center();
                vec![
                    (
                        100,
                        Action::MouseMove {
                            x: p.x.into(),
                            y: p.y.into(),
                        },
                    ),
                    (
                        400,
                        Action::KeyCharacters {
                            key_code: 125,
                            chars: "\u{f701}",
                        },
                    ),
                    (
                        600,
                        Action::KeyCharacters {
                            key_code: 36,
                            chars: "\r",
                        },
                    ),
                    (900, next(36_006)),
                ]
            }
            36_006 => {
                assert_eq!(records.len(), 2);
                assert_eq!(records[1].action.source, ContentActionSource::Keyboard);
                assert_eq!(
                    selection(&records[1]).value_path().unwrap(),
                    ["group", "folder"]
                );
                assert_eq!(records[0].mount, records[1].mount);
                let mut entries = f::options();
                if let MenuEntry::Submenu {
                    entries: children, ..
                } = &mut entries[1]
                    && let MenuEntry::Item { checked, .. } = &mut children[1]
                {
                    *checked = Some(true);
                }
                menu(tree).set_entries(entries).unwrap();
                assert_ne!(menu(tree).model_revision(), self.first_revision);
                // The AppKit selftest helper can only find current published
                // elements. This is explicitly the original public semantic
                // stale-token check, not a fabricated native old-object Press.
                assert!(
                    !tree
                        .accessibility_action(
                            f::OPTIONS,
                            self.stale.unwrap().part,
                            AccessibilityAction::Press
                        )
                        .activated
                );
                vec![
                    (250, trigger(tree)),
                    (
                        500,
                        Action::KeyCharacters {
                            key_code: 53,
                            chars: "\u{1b}",
                        },
                    ),
                    (750, trigger(tree)),
                    (
                        1000,
                        Action::Click {
                            x: window.size().width - 20.,
                            y: window.size().height - 20.,
                        },
                    ),
                    (1250, next(36_007)),
                ]
            }
            36_007 => {
                assert_eq!(
                    records.len(),
                    2,
                    "old popup token, Escape and outside dismissal cannot select"
                );
                assert!(!menu(tree).menu.is_open());
                vec![
                    (
                        100,
                        Action::AXSelect {
                            component: f::MESSAGE.0,
                            range: 0..0,
                        },
                    ),
                    (
                        300,
                        Action::PreeditRange {
                            text: "ni".into(),
                            selection: 2..2,
                            replacement: None,
                        },
                    ),
                    (600, next(36_008)),
                ]
            }
            36_008 => {
                assert_eq!(tree.focused(), Some(f::MESSAGE));
                let input = &tree.focused_text().unwrap().input;
                assert!(input.marked.is_some());
                self.marked = Some(input.external_baseline());
                // Original public MenuView opening preserves the editor owner;
                // the following part actions use real published native AX selectors.
                menu(tree).menu.show(0);
                vec![(250, next(36_009))]
            }
            36_009 => vec![
                (
                    100,
                    Action::AXPressKey {
                        key: key(tree, "Sort by"),
                    },
                ),
                (350, next(36_010)),
            ],
            36_010 => vec![
                (
                    100,
                    Action::AXPressKey {
                        key: key(tree, "Recent activity"),
                    },
                ),
                (350, next(36_011)),
            ],
            36_011 => {
                assert_eq!(records.len(), 3);
                assert_eq!(records[2].action.source, ContentActionSource::Accessibility);
                assert_eq!(
                    selection(&records[2]).value_path().unwrap(),
                    ["sort", "recentActivity"]
                );
                assert_eq!(tree.focused(), Some(f::MESSAGE));
                assert_eq!(
                    tree.focused_text().unwrap().input.external_baseline(),
                    *self.marked.as_ref().unwrap()
                );
                vec![
                    (100, Action::Unmark),
                    (300, Action::Resize { w: 408., h: 480. }),
                    (600, trigger(tree)),
                    (900, next(36_012)),
                ]
            }
            36_012 => {
                assert_eq!(window.size().width, 408.);
                vec![
                    (
                        100,
                        Action::AXPressKey {
                            key: key(tree, "Sort by"),
                        },
                    ),
                    (350, next(36_013)),
                ]
            }
            36_013 => {
                let viewport = Rect::new(
                    0.,
                    0.,
                    window.size().width as f32,
                    window.size().height as f32,
                );
                let groups = parts(tree)
                    .into_iter()
                    .filter(|part| part.semantics.role == ContentRole::Menu)
                    .collect::<Vec<_>>();
                assert_eq!(groups.len(), 2);
                assert!(
                    groups
                        .iter()
                        .all(|group| group.frame.intersection(&viewport) == Some(group.frame))
                );
                vec![
                    (100, capture("narrow-original-submenu.png")),
                    (
                        350,
                        Action::KeyCharacters {
                            key_code: 123,
                            chars: "\u{f702}",
                        },
                    ),
                    (
                        550,
                        Action::KeyCharacters {
                            key_code: 53,
                            chars: "\u{1b}",
                        },
                    ),
                    (800, next(36_014)),
                ]
            }
            36_014 => {
                assert_eq!(records.len(), 3);
                assert!(!menu(tree).menu.is_open());
                menu(tree)
                    .set_entries(vec![MenuEntry::submenu(
                        "sort",
                        "Sort by",
                        (0..80)
                            .map(|i| {
                                MenuEntry::item(
                                    format!("row-{i}"),
                                    format!("Scrollable choice {i}"),
                                )
                            })
                            .collect(),
                    )])
                    .unwrap();
                vec![(100, trigger(tree)), (350, next(36_015))]
            }
            36_015 => vec![
                (
                    100,
                    Action::AXPressKey {
                        key: key(tree, "Sort by"),
                    },
                ),
                (350, next(36_016)),
            ],
            36_016 => {
                self.first_row = Some(part(tree, "Scrollable choice 0").frame);
                let group = parts(tree)
                    .into_iter()
                    .rfind(|p| p.semantics.role == ContentRole::Menu)
                    .unwrap();
                let p = group.frame.center();
                vec![
                    (
                        100,
                        Action::ScrollWheel {
                            x: p.x.into(),
                            y: p.y.into(),
                            delta_x: 0,
                            delta_y: -6,
                        },
                    ),
                    (500, next(36_017)),
                ]
            }
            36_017 => {
                assert!(
                    self.scroll_events > 0,
                    "actual Event::Scroll reached original host route"
                );
                let first = parts(tree)
                    .into_iter()
                    .find(|p| p.semantics.name == "Scrollable choice 0");
                assert!(
                    first.is_none_or(|p| p.frame.min_y() < self.first_row.unwrap().min_y()),
                    "original long-submenu scroll changed first row geometry or visibility"
                );
                assert_eq!(records.len(), 3, "native wheel has no business selection");
                assert_eq!(
                    selection(&records[0]).value_path().unwrap(),
                    ["sort", "titleAscending"]
                );
                assert_eq!(
                    selection(&records[1]).value_path().unwrap(),
                    ["group", "folder"]
                );
                assert_eq!(
                    Some(selection(&records[0]).model_revision()),
                    self.first_revision
                );
                assert_eq!(tree.semantic_mount(f::MESSAGE), self.editor_mount);
                vec![
                    (100, capture("original-submenu-after-wheel.png")),
                    (
                        350,
                        Action::KeyCharacters {
                            key_code: 53,
                            chars: "\u{1b}",
                        },
                    ),
                    (800, Action::Close),
                ]
            }
            _ => unreachable!(),
        };
        self.verified.push(stage);
        let snapshot = serde_json::json!({"stage":stage,"verified":self.verified,
            "window_active":window.is_active(),"width":window.size().width,
            "focus":tree.focused().map(|id|id.0),"editor_mount":tree.semantic_mount(f::MESSAGE),
            "scroll_events":self.scroll_events,
            "records":records.iter().map(|record| { let selected=selection(record);
                serde_json::json!({"source":format!("{:?}",record.action.source),"target":record.action.target.0,
                    "mount":record.mount,"value_path":selected.value_path(),"model_revision":selected.model_revision()}) }).collect::<Vec<_>>(),
            "programmatic_AppKit":true,"physical_IME":false,"VoiceOver":false,
            "complete":stage==36_017});
        eprintln!("MENU36_NATIVE {snapshot}");
        use std::io::Write;
        let path =
            std::path::PathBuf::from(std::env::var_os("TEKESUI_COMPONENT_EVIDENCE").unwrap());
        std::fs::create_dir_all(&path).unwrap();
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path.join("menu36-journal.jsonl"))
            .unwrap();
        writeln!(file, "{snapshot}").unwrap();
        script
    }
}
