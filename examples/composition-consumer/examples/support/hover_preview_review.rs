//! Fixed native software acceptance script; layout uses original stock Content.
//! Synthetic NSTextInputClient preedit is not a physical Chinese IME session.
use std::path::Path;
use tekes_ui_composition_consumer::hover_preview::*;
use tekes_ui_controls::{
    ButtonTextRenderer, ScrollArea,
    composition::*,
    extended::{Axis, Stack},
    layout::MeasuredChild,
};
use tekes_ui_platform_macos::selftest::Action;
use tekes_ui_scene::{Point, Rect, Size};
const SCROLL: ComponentId = ComponentId(24700);
const COLUMN: ComponentId = ComponentId(24701);
const ROWS: ComponentId = ComponentId(24702);
const SPACE: ComponentId = ComponentId(24703);
pub const FIRST: u32 = 46001;
pub const LAST: u32 = 46026;
#[derive(Debug)]
struct Space;
impl Component for Space {
    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, _: Size) -> MeasuredChild {
        MeasuredChild::new(Size::new(24., 420.))
    }
}
pub fn snapshot(f: &Transcript, active: bool) -> serde_json::Value {
    let input = &f
        .host
        .get(COMPOSER)
        .unwrap()
        .component::<TextFieldContent>()
        .unwrap()
        .surface
        .input;
    let preview = f
        .host
        .get(PREVIEW)
        .unwrap()
        .component::<HoverPreviewContent>()
        .unwrap();
    serde_json::json!({"active":active,"focus":f.host.focused().map(|id|id.0),"text":input.text,"selection":input.selection(),"marked":input.marked,"preview_open":preview.is_open(),"preview_version":format!("{:?}",preview.version()),"preview_frame":format!("{:?}",preview.layout().frame),"nodes":f.host.semantics().iter().map(|node|serde_json::json!({"id":node.id.0,"mount":f.host.semantic_mount(node.id),"name":node.semantics.name,"role":format!("{:?}",node.semantics.role),"frame":format!("{:?}",node.frame),"parts":node.parts.iter().map(|part|part.semantics.name.clone()).collect::<Vec<_>>()})).collect::<Vec<_>>()})
}
pub struct Review {
    pending: Option<u32>,
    completed: Vec<u32>,
    baseline: Option<serde_json::Value>,
    old: HoverPreviewVersion,
    retired: Option<Content>,
    original_b: u64,
    activations: Vec<ComponentId>,
}
impl Review {
    pub fn prepare(f: &mut Transcript) -> Self {
        let a = f.host.remove(TICK_A).unwrap();
        let b = f.host.remove(TICK_B).unwrap();
        let rows = Content::new(
            ROWS,
            StackContent {
                model: Stack {
                    axis: Axis::Horizontal,
                    gap: 8.,
                },
                flexible_child: None,
                children: vec![a, b],
            },
        );
        let column = Content::new(
            COLUMN,
            StackContent {
                model: Stack {
                    axis: Axis::Vertical,
                    gap: 8.,
                },
                flexible_child: None,
                children: vec![rows, Content::new(SPACE, Space)],
            },
        );
        let mut scroll = ScrollContent::new(ScrollArea::default(), column);
        scroll.viewport_height = Some(90.);
        f.host.insert(ROOT, Content::new(SCROLL, scroll)).unwrap();
        f.host.reorder(ROOT, &[SCROLL, COMPOSER, PREVIEW]).unwrap();
        f.host.set_action_events(true);
        f.version = f.host.bind_hover_preview(TICK_A, PREVIEW).unwrap();
        Self {
            pending: None,
            completed: Vec::new(),
            baseline: None,
            old: f.version,
            retired: None,
            original_b: f.host.get(TICK_B).unwrap().mount_identity(),
            activations: Vec::new(),
        }
    }
    pub fn script(&self, f: &Transcript) -> Vec<(u64, Action)> {
        let a = f.host.get(TICK_A).unwrap().frame().center();
        let b = f.host.get(TICK_B).unwrap().frame().center();
        let composer = f.host.get(COMPOSER).unwrap().mount_identity();
        let hover = |point: Point| Action::MouseMove {
            x: f64::from(point.x),
            y: f64::from(point.y),
        };
        let mut steps = Vec::new();
        let mut at = 200;
        let mut push = |action| {
            steps.push((at, action));
            at += 400;
        };
        push(Action::AXSelect {
            component: composer,
            range: 0..8,
        });
        push(Action::PreeditRange {
            text: "中文".into(),
            selection: 1..1,
            replacement: None,
        });
        push(Action::MenuCommand(46001));
        push(hover(a));
        push(Action::MenuCommand(46002));
        push(Action::MenuCommand(46003));
        push(Action::MouseExit);
        push(Action::MenuCommand(46004));
        push(Action::Unmark);
        // AXReplace invokes setAccessibilitySelectedText, not SetValue.
        // Select the full known synthetic-preedit document before replacing it.
        push(Action::AXSelect {
            component: composer,
            range: 0.."中文 composer draft".encode_utf16().count(),
        });
        push(Action::AXReplace {
            component: composer,
            text: "Original composer draft".into(),
        });
        push(Action::AXSelect {
            component: composer,
            range: 0..8,
        });
        push(Action::MenuCommand(46005));
        push(hover(a));
        push(Action::MenuCommand(46006));
        push(Action::KeyCharacters {
            key_code: 53,
            chars: "\u{1b}",
        });
        push(Action::MenuCommand(46007));
        push(Action::MenuCommand(46008));
        push(hover(a));
        push(Action::MenuCommand(46009));
        push(Action::MouseExit);
        push(hover(a));
        push(Action::MenuCommand(46010));
        push(Action::ClickPublished {
            label: "Turn B",
            secondary: false,
        });
        push(Action::MenuCommand(46011));
        push(Action::AXSelect {
            component: composer,
            range: 0..8,
        });
        push(Action::MenuCommand(46012));
        push(hover(b));
        push(Action::MenuCommand(46013));
        push(Action::ScrollWheel {
            x: 30.,
            y: 28.,
            delta_x: 0,
            delta_y: -100,
        });
        push(Action::MenuCommand(46014));
        push(Action::MenuCommand(46015));
        push(Action::MouseExit);
        push(hover(b));
        push(Action::MenuCommand(46016));
        push(Action::MenuCommand(46017));
        push(Action::MenuCommand(46018));
        push(Action::MouseExit);
        push(hover(b));
        push(Action::MenuCommand(46019));
        push(Action::MenuCommand(46020));
        push(Action::MenuCommand(46021));
        push(hover(b));
        push(Action::MenuCommand(46022));
        push(Action::MenuCommand(46023));
        push(Action::MouseExit);
        push(hover(b));
        push(Action::Resize { w: 320., h: 260. });
        // AppKit may emit mouseExited after tracking-area replacement on resize.
        // Keep leave retirement; exercise a fresh real native enter at new size.
        push(Action::MouseExit);
        push(hover(b));
        push(Action::MenuCommand(46024));
        push(Action::Miniaturize);
        push(Action::Deminiaturize);
        push(Action::MenuCommand(46025));
        push(Action::MenuCommand(46026));
        push(Action::Close);
        steps
    }
    pub fn collect(&mut self, f: &mut Transcript) {
        self.activations.extend(
            f.host
                .drain_actions()
                .into_iter()
                .map(|action| action.target),
        );
    }
    pub fn command(&mut self, id: u32, f: &mut Transcript) -> bool {
        if !(FIRST..=LAST).contains(&id) {
            return false;
        }
        match id {
            46001 | 46005 => self.baseline = Some(snapshot(f, true)),
            46003 => {
                let old = f.version;
                f.update("Input A streamed", "Output A streamed twice")
                    .unwrap();
                assert_eq!(
                    f.host.update_hover_preview(old, content("late", "late")),
                    Err(HoverPreviewError::StaleVersion)
                );
            }
            46008 => {
                f.update("Input A after Escape", "Output A after Escape")
                    .unwrap();
            }
            46012 => {
                self.old = f.version;
                f.bind_b().unwrap();
                f.update("Input B", "Output B").unwrap();
                assert_eq!(
                    f.host
                        .update_hover_preview(self.old, content("late A", "late A")),
                    Err(HoverPreviewError::StaleVersion)
                );
            }
            46015 => {
                f.host
                    .get_mut(SCROLL)
                    .unwrap()
                    .component_mut::<ScrollContent>()
                    .unwrap()
                    .scroll
                    .offset = Point::default()
            }
            46017 => {
                f.host
                    .get_mut(SCROLL)
                    .unwrap()
                    .component_mut::<ScrollContent>()
                    .unwrap()
                    .viewport_height = Some(0.)
            }
            46018 => {
                f.host
                    .get_mut(SCROLL)
                    .unwrap()
                    .component_mut::<ScrollContent>()
                    .unwrap()
                    .viewport_height = Some(90.)
            }
            46020 => {
                self.old = f.version;
                self.retired = Some(f.host.remove(TICK_B).unwrap());
                assert_eq!(
                    f.host
                        .update_hover_preview(self.old, content("late B", "late B")),
                    Err(HoverPreviewError::OwnerUnavailable)
                );
            }
            46021 => {
                f.host.insert(ROWS, self.retired.take().unwrap()).unwrap();
                f.host.reorder(ROWS, &[TICK_A, TICK_B]).unwrap();
            }
            46023 => {
                f.bind_b().unwrap();
                assert_eq!(
                    f.host
                        .update_hover_preview(self.old, content("old mount", "old mount")),
                    Err(HoverPreviewError::StaleVersion)
                );
            }
            _ => {}
        }
        self.pending = Some(id);
        true
    }
    pub fn verify(&mut self, f: &Transcript, viewport: Rect, evidence: Option<&Path>) {
        let Some(id) = self.pending.take() else {
            return;
        };
        let mut errors = Vec::new();
        let state = snapshot(f, true);
        if id == 46001
            && (state["text"] != "中文 composer draft"
                || state["marked"] != serde_json::json!({"start":0,"end":2})
                || state["selection"] != serde_json::json!({"start":1,"end":1}))
        {
            errors.push("native Chinese marked setup differs from fixed input".into());
        }
        if id == 46005
            && (state["text"] != "Original composer draft"
                || !state["marked"].is_null()
                || state["selection"] != serde_json::json!({"start":0,"end":8}))
        {
            errors.push("native plain reset did not restore exact original document".into());
        }
        let preview = f
            .host
            .get(PREVIEW)
            .unwrap()
            .component::<HoverPreviewContent>()
            .unwrap();
        let open = matches!(
            id,
            46002 | 46003 | 46006 | 46010 | 46013 | 46016 | 46019 | 46024
        );
        if preview.is_open() != open {
            errors.push(format!("preview lifetime differs at {id}"));
        }
        if let Some(baseline) = &self.baseline {
            for key in ["text", "selection", "marked"] {
                if state[key] != baseline[key] {
                    errors.push(format!("original Composer {key} changed at{id}"));
                }
            }
        }
        let focus = if id >= 46025 {
            None
        } else if id == 46011 {
            Some(TICK_B)
        } else {
            Some(COMPOSER)
        };
        if f.host.focused() != focus {
            errors.push(format!("unexpected original focus at{id}"));
        }
        let expected = if id < 46011 { vec![] } else { vec![TICK_B] };
        if self.activations != expected {
            errors.push(format!(
                "click-through original action mismatch: {:?}",
                self.activations
            ));
        }
        let nodes = f.host.semantics();
        if nodes
            .iter()
            .flat_map(|node| &node.parts)
            .any(|part| part.semantics.name == "Dismiss")
            || f.host
                .focus_targets()
                .iter()
                .any(|(owner, _)| *owner == PREVIEW)
        {
            errors.push("passive preview added Dismiss/focus owner".into());
        }
        if open {
            for target in [BODY, INPUT, OUTPUT] {
                if !nodes.iter().any(|node| node.id == target) {
                    errors.push(format!("original passive subtree node {target:?} missing"));
                }
            }
            let frame = preview.layout().frame;
            if frame.min_x() < viewport.min_x()
                || frame.max_x() > viewport.max_x()
                || frame.min_y() < viewport.min_y()
                || frame.max_y() > viewport.max_y()
            {
                errors.push("preview escaped real native viewport".into());
            }
        }
        if id == 46003
            && !nodes
                .iter()
                .any(|node| node.id == OUTPUT && node.semantics.name == "Output A streamed twice")
        {
            errors.push("static pointer did not publish accepted stream update".into());
        }
        if id == 46010
            && !preview
                .layout()
                .frame
                .contains(f.host.get(TICK_B).unwrap().frame().center())
        {
            errors.push("original B click target does not overlap preview card".into());
        }
        if id == 46014
            && f.host
                .get(SCROLL)
                .unwrap()
                .component::<ScrollContent>()
                .unwrap()
                .scroll
                .offset
                .y
                <= 0.
        {
            errors.push("actual native scroll did not move viewport".into());
        }
        if id == 46017 && f.host.get(TICK_B).unwrap().visible().is_some() {
            errors.push("clip stage did not retire anchor geometry".into());
        }
        if id == 46022 && f.host.get(TICK_B).unwrap().mount_identity() == self.original_b {
            errors.push("remount retained original mount".into());
        }
        if id == LAST && self.completed != (FIRST..LAST).collect::<Vec<_>>() {
            errors.push("checkpoints missing or reordered".into());
        }
        let report = serde_json::json!({"phase":id,"passed":errors.is_empty(),"errors":errors,"state":state,"activations":self.activations.iter().map(|id|id.0).collect::<Vec<_>>(),"original_b_mount":self.original_b,"current_b_mount":f.host.get(TICK_B).map(Content::mount_identity),"native_software_only":true,"physical_ime":"not_run","voiceover":"not_run"});
        if let Some(path) = evidence {
            std::fs::create_dir_all(path).unwrap();
            std::fs::write(
                path.join(format!("checkpoint-{id}.json")),
                serde_json::to_vec_pretty(&report).unwrap(),
            )
            .unwrap();
        }
        eprintln!("HOVER_NATIVE_CHECK {report}");
        assert!(
            errors.is_empty(),
            "native preview checkpoint{id}: {errors:?}"
        );
        self.completed.push(id);
    }
}
