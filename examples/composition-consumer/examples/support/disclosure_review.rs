//! Phased AppKit script on the existing native Host. No replacement renderer/input owner.
use tekes_ui_composition_consumer::disclosure_review as f;
use tekes_ui_controls::{TextEditBaseline, composition::*};
use tekes_ui_platform_macos::selftest::Action;
pub struct Review {
    directory: std::path::PathBuf,
    programmatic_appkit: bool,
    verified: Vec<u32>,
    trigger_mount: Option<u64>,
    editor_mount: Option<u64>,
    outside_mount: Option<u64>,
    marked: Option<TextEditBaseline>,
    marked_ax_refused: bool,
}
fn click(tree: &ContentHost) -> Action {
    let p = tree.get(f::TRIGGER).unwrap().frame().center();
    Action::Click {
        x: p.x.into(),
        y: p.y.into(),
    }
}
fn ax() -> Action {
    Action::AXPress {
        component: f::TRIGGER.0,
    }
}
fn key(code: u16, chars: &'static str) -> Action {
    Action::KeyCharacters {
        key_code: code,
        chars,
    }
}
fn capture(directory: &std::path::Path, name: &str) -> Action {
    Action::Capture {
        path: directory.join(name).to_string_lossy().into_owned(),
    }
}

fn one(
    records: &[ContentActionRecord],
    count: usize,
    source: ContentActionSource,
) -> DisclosureIntent {
    assert_eq!(records.len(), count, "one original action per gesture");
    let record = records.last().unwrap();
    assert_eq!(record.action.target, f::TRIGGER);
    assert_eq!(record.action.source, source);
    f::intent(record).clone()
}
impl Review {
    pub fn new(directory: std::path::PathBuf, programmatic_appkit: bool) -> Self {
        Self {
            directory,
            programmatic_appkit,
            verified: Vec::new(),
            trigger_mount: None,
            editor_mount: None,
            outside_mount: None,
            marked: None,
            marked_ax_refused: false,
        }
    }
    pub fn advance(
        &mut self,
        phase: u32,
        tree: &mut ContentHost,
        records: &[ContentActionRecord],
        window_active: bool,
        width: f64,
    ) -> Vec<(u64, Action)> {
        assert!(window_active, "native Disclosure window must be active");
        assert_eq!(
            phase,
            38_001 + self.verified.len() as u32,
            "ordered checkpoints"
        );
        let next = |phase| Action::MenuCommand(phase);
        let capture = |name| capture(&self.directory, name);
        let script = match phase {
            38_001 => {
                assert!(records.is_empty());
                assert!(!f::disclosure(tree).expanded());
                self.trigger_mount = tree.semantic_mount(f::TRIGGER);
                self.editor_mount = Some(tree.get(f::EDITOR).unwrap().mount_identity());
                self.outside_mount = Some(tree.get(f::OUTSIDE).unwrap().mount_identity());
                assert!(self.trigger_mount.is_some());
                assert!(!tree.semantics().iter().any(|n| {
                    [f::TITLE, f::TITLE_DETAIL, f::EDITOR, f::BODY_BUTTON].contains(&n.id)
                }));
                vec![
                    (100, capture("collapsed-light.png")),
                    (350, click(tree)),
                    (650, next(38_002)),
                ]
            }
            38_002 => {
                let intent = one(records, 1, ContentActionSource::Pointer);
                assert!(intent.expanded());
                assert!(
                    !f::disclosure(tree).expanded(),
                    "request is not eager toggle"
                );
                assert!(f::disclosure_mut(tree).confirm(&intent, true));
                assert_eq!(tree.focused(), Some(f::TRIGGER));
                vec![
                    (300, capture("expanded-focus-light.png")),
                    (550, key(36, "\r")),
                    (850, next(38_003)),
                ]
            }
            38_003 => {
                let intent = one(records, 2, ContentActionSource::Keyboard);
                assert!(!intent.expanded());
                assert!(f::disclosure_mut(tree).confirm(&intent, false));
                assert!(f::disclosure(tree).expanded());
                assert!(!f::disclosure_mut(tree).confirm(&intent, true));
                vec![(100, key(49, " ")), (400, next(38_004))]
            }
            38_004 => {
                let intent = one(records, 3, ContentActionSource::Keyboard);
                assert!(f::disclosure_mut(tree).confirm(&intent, true));
                // Existing deterministic headless tests cover intermediate retained paint.
                vec![(350, key(48, "\t")), (650, next(38_005))]
            }
            38_005 => {
                assert_eq!(records.len(), 3);
                assert_eq!(
                    tree.focused(),
                    Some(f::OUTSIDE),
                    "Tab skips collapsed original subtree"
                );
                assert!(
                    !tree
                        .semantics()
                        .iter()
                        .any(|n| [f::EDITOR, f::BODY_BUTTON].contains(&n.id))
                );
                assert!(
                    !tree
                        .accessibility_action(f::BODY_BUTTON, None, AccessibilityAction::Press)
                        .activated
                );
                assert!(
                    !tree
                        .focus_targets()
                        .iter()
                        .any(|(id, _)| [f::EDITOR, f::BODY_BUTTON].contains(id))
                );
                vec![(100, ax()), (400, next(38_006))]
            }
            38_006 => {
                let intent = one(records, 4, ContentActionSource::Accessibility);
                assert!(f::disclosure_mut(tree).confirm(&intent, true));
                assert!(f::disclosure_mut(tree).set_expanded(false)); // authoritative rollback
                assert!(!f::disclosure_mut(tree).confirm(&intent, true));
                assert!(f::disclosure_mut(tree).set_expanded(true));
                vec![(300, ax()), (600, next(38_007))]
            }
            38_007 => {
                one(records, 5, ContentActionSource::Accessibility);
                vec![(100, ax()), (400, next(38_008))]
            }
            38_008 => {
                let latest = one(records, 6, ContentActionSource::Accessibility);
                let old = f::intent(&records[4]);
                assert_ne!(old.generation(), latest.generation());
                assert!(!f::disclosure_mut(tree).confirm(old, true));
                // Keep the latest real AX intent pending until the body starts composing.
                assert!(f::disclosure(tree).expanded());
                vec![
                    (
                        100,
                        Action::AXSelect {
                            component: f::EDITOR.0,
                            range: 0..8,
                        },
                    ),
                    (
                        350,
                        Action::PreeditRange {
                            text: "中文".into(),
                            selection: 1..1,
                            replacement: None,
                        },
                    ),
                    (650, next(38_009)),
                ]
            }
            38_009 => {
                let input = &tree
                    .focused_text()
                    .expect("original body editor focused")
                    .input;
                assert_eq!(tree.focused(), Some(f::EDITOR));
                assert_eq!(input.text, "中文 body draft");
                assert_eq!(input.selection(), 1..1);
                assert_eq!(input.marked, Some(0..2));
                self.marked = Some(input.external_baseline());
                assert!(!f::disclosure_mut(tree).set_expanded(false));
                // The original host refuses a cross-owner AX Press during composition.
                // AXPress selftest requires a positive native selector result, so this
                // negative case explicitly exercises the public SDK boundary instead.
                let response =
                    tree.accessibility_action(f::TRIGGER, None, AccessibilityAction::Press);
                assert!(
                    !response.activated && !response.consumed && !response.changed,
                    "marked cross-owner AX Press must be refused without mutation"
                );
                assert!(
                    tree.drain_action_records().is_empty(),
                    "refused AX Press emits no action"
                );
                assert_eq!(tree.focused(), Some(f::EDITOR));
                assert_eq!(
                    tree.focused_text().unwrap().input.external_baseline(),
                    *self.marked.as_ref().unwrap()
                );
                self.marked_ax_refused = true;
                vec![(400, next(38_010))]
            }
            38_010 => {
                let intent = one(records, 6, ContentActionSource::Accessibility);
                assert!(
                    !f::disclosure_mut(tree).confirm(&intent, true),
                    "marked body refuses collapse"
                );
                assert!(f::disclosure_mut(tree).confirm(&intent, false));
                assert_eq!(tree.focused(), Some(f::EDITOR));
                assert_eq!(
                    tree.focused_text().unwrap().input.external_baseline(),
                    *self.marked.as_ref().unwrap()
                );
                vec![
                    (100, capture("marked-collapse-refused.png")),
                    (350, Action::Unmark),
                    (650, next(38_011)),
                ]
            }
            38_011 => {
                assert!(tree.focused_text().unwrap().input.marked.is_none());
                f::disclosure_mut(tree).set_disabled(true);
                vec![
                    (150, capture("disabled-light.png")),
                    (400, click(tree)),
                    (700, next(38_012)),
                ]
            }
            38_012 => {
                assert_eq!(records.len(), 6, "disabled trigger has no action");
                f::disclosure_mut(tree).set_disabled(false);
                f::disclosure_mut(tree).set_rtl(true);
                vec![
                    (100, Action::Resize { w: 360., h: 640. }),
                    (300, Action::WindowDark(true)),
                    (650, next(38_013)),
                ]
            }
            38_013 => {
                assert_eq!(width, 360.);
                assert!(f::disclosure(tree).expanded());
                vec![
                    (100, capture("narrow-rtl-dark.png")),
                    (350, ax()),
                    (650, next(38_014)),
                ]
            }
            38_014 => {
                let intent = one(records, 7, ContentActionSource::Accessibility);
                assert!(f::disclosure_mut(tree).confirm(&intent, true));
                vec![
                    (350, capture("collapsed-rtl-dark.png")),
                    (650, next(38_015)),
                ]
            }
            38_015 => {
                assert!(!f::disclosure(tree).expanded());
                assert_eq!(f::disclosure(tree).body_layout().progress, 0.);
                assert_eq!(records.len(), 7);
                vec![(100, Action::Close)]
            }
            _ => panic!("unknown Disclosure phase"),
        };
        assert_eq!(
            tree.semantic_mount(f::TRIGGER),
            self.trigger_mount,
            "original owner retained"
        );
        assert_eq!(
            Some(tree.get(f::EDITOR).unwrap().mount_identity()),
            self.editor_mount,
            "original editor retained"
        );
        assert_eq!(
            Some(tree.get(f::OUTSIDE).unwrap().mount_identity()),
            self.outside_mount,
            "unrelated editor owner retained"
        );
        let outside = tree
            .get(f::OUTSIDE)
            .unwrap()
            .component::<TextFieldContent>()
            .unwrap();
        assert_eq!(outside.surface.input.text, "Keep A😀B");
        assert_eq!(outside.surface.input.selection(), 0..0);
        assert!(outside.surface.input.marked.is_none());
        self.verified.push(phase);
        let editor = &tree
            .get(f::EDITOR)
            .unwrap()
            .component::<TextFieldContent>()
            .unwrap()
            .surface
            .input;
        if phase >= 38_011 {
            assert_eq!(editor.text, "中文 body draft");
            assert_eq!(editor.selection(), 1..1);
            assert!(editor.marked.is_none(), "unmarked body stays unmarked");
        }
        let snapshot = serde_json::json!({"phase":phase,"passed":true,"complete":phase==38_015,
            "window_active":window_active,"width":width,
            "expanded":f::disclosure(tree).expanded(),"disabled":f::disclosure(tree).disabled(),
            "focus":tree.focused().map(|id|id.0),"trigger_mount":self.trigger_mount,"editor_mount":self.editor_mount,"outside_mount":self.outside_mount,
            "editor":{"text":editor.text,"selection":editor.selection(),"marked":editor.marked},
            "outside":{"text":outside.surface.input.text,"selection":outside.surface.input.selection(),"marked":outside.surface.input.marked},
            "marked_baseline":self.marked.as_ref().map(|_| "中文 body draft|1..1|0..2"),
            "public_sdk_marked_ax_refused":self.marked_ax_refused,
            "records":records.iter().map(|r|serde_json::json!({"source":format!("{:?}",r.action.source),"target":r.action.target.0,"mount":r.mount,
                "expanded":f::intent(r).expanded(),"generation":f::intent(r).generation(),"revision":f::intent(r).revision()})).collect::<Vec<_>>(),
            "programmatic_AppKit":self.programmatic_appkit,"physical_IME":false,"VoiceOver":false});
        use std::io::Write;
        let directory = &self.directory;
        std::fs::create_dir_all(directory).unwrap();
        let mut journal = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(directory.join("disclosure38-journal.jsonl"))
            .unwrap();
        writeln!(journal, "{snapshot}").unwrap();
        eprintln!("DISCLOSURE38_NATIVE {snapshot}");
        script
    }
}
