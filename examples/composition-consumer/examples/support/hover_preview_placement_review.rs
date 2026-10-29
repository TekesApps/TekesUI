//! Native placement-only fixture. Stock Stack lays out original anchors; no
//! caller-provided popup position or second overlay engine is used for painting.
use std::path::Path;
use tekes_ui_composition_consumer::hover_preview::*;
use tekes_ui_controls::{
    ButtonTextRenderer,
    composition::*,
    extended::{Axis, Stack},
    layout::MeasuredChild,
};
use tekes_ui_platform_macos::selftest::Action;
use tekes_ui_scene::{Rect, Size};
const ROW: ComponentId = ComponentId(24800);
const SPACE: ComponentId = ComponentId(24801);
pub const FIRST: u32 = 46101;
pub const LAST: u32 = 46107;
#[derive(Debug)]
struct AnchorSpace {
    width: f32,
}
impl Component for AnchorSpace {
    fn semantic_hidden(&self) -> bool {
        // Pure layout spacing may legitimately have zero width. It has no
        // semantic identity and must not produce an invalid native AX frame.
        true
    }
    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, _: Size) -> MeasuredChild {
        MeasuredChild::new(Size::new(self.width, 24.))
    }
}
fn rect(frame: Rect) -> serde_json::Value {
    serde_json::json!({"x":frame.min_x(),"y":frame.min_y(),"width":frame.size.width,"height":frame.size.height})
}
pub struct Review {
    pending: Option<u32>,
    completed: Vec<u32>,
    actions: Vec<ComponentId>,
    owner_mount: u64,
}
impl Review {
    pub fn prepare(f: &mut Transcript) -> Self {
        let a = f.host.remove(TICK_A).unwrap();
        let b = f.host.remove(TICK_B).unwrap();
        let row = Content::new(
            ROW,
            StackContent {
                model: Stack {
                    axis: Axis::Horizontal,
                    gap: 8.,
                },
                flexible_child: None,
                children: vec![Content::new(SPACE, AnchorSpace { width: 0. }), a, b],
            },
        );
        f.host.insert(ROOT, row).unwrap();
        // A data update requires a laid-out anchor. This initial fixture body is
        // declared before the first layout, through the ordinary public slot.
        f.host.remove(PREVIEW).unwrap();
        let mut preview = HoverPreviewContent::new(
            "Turn preview",
            content(
                &"Input placement sample 中文 ".repeat(12),
                &"Output placement sample 中文 ".repeat(24),
            ),
        )
        .unwrap();
        preview.set_max_content_width(Some(220.));
        f.host.insert(ROOT, Content::new(PREVIEW, preview)).unwrap();
        f.host.reorder(ROOT, &[ROW, COMPOSER, PREVIEW]).unwrap();
        f.host.set_action_events(true);
        f.version = f.host.bind_hover_preview(TICK_A, PREVIEW).unwrap();
        Self {
            pending: None,
            completed: Vec::new(),
            actions: Vec::new(),
            owner_mount: f.host.get(TICK_A).unwrap().mount_identity(),
        }
    }
    pub fn script(&self, f: &Transcript) -> Vec<(u64, Action)> {
        let point = f.host.get(TICK_A).unwrap().frame().center();
        let composer = f.host.get(COMPOSER).unwrap().mount_identity();
        let hover = |shift: f32| Action::MouseMove {
            x: f64::from(point.x + shift),
            y: f64::from(point.y),
        };
        vec![
            (
                200,
                Action::AXSelect {
                    component: composer,
                    range: 0..8,
                },
            ),
            (600, hover(0.)),
            (1000, Action::MenuCommand(46101)),
            (1400, Action::MenuCommand(46102)),
            (1800, Action::MouseExit),
            (2200, hover(430.)),
            (2600, Action::MenuCommand(46103)),
            (3000, Action::MenuCommand(46104)),
            (3400, Action::Resize { w: 320., h: 260. }),
            (3800, Action::MouseExit),
            (4200, hover(90.)),
            (4600, Action::MenuCommand(46105)),
            (
                5000,
                Action::KeyCharacters {
                    key_code: 53,
                    chars: "\u{1b}",
                },
            ),
            (5400, Action::MenuCommand(46106)),
            (5800, Action::MouseExit),
            (6200, Action::MenuCommand(46107)),
            (6600, Action::Close),
        ]
    }
    pub fn collect(&mut self, f: &mut Transcript) {
        self.actions.extend(
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
        let width = match id {
            46102 => Some(430.),
            46104 => Some(90.),
            _ => None,
        };
        if let Some(width) = width {
            f.host
                .get_mut(SPACE)
                .unwrap()
                .component_mut::<AnchorSpace>()
                .unwrap()
                .width = width;
        }
        self.pending = Some(id);
        true
    }
    pub fn verify(&mut self, f: &Transcript, bounds: Rect, evidence: Option<&Path>) {
        let Some(id) = self.pending.take() else {
            return;
        };
        let mut errors = Vec::new();
        let owner = f.host.get(TICK_A).unwrap();
        let anchor = owner.frame();
        let popup = f
            .host
            .get(PREVIEW)
            .unwrap()
            .component::<HoverPreviewContent>()
            .unwrap();
        let frame = popup.layout().frame;
        let input = &f
            .host
            .get(COMPOSER)
            .unwrap()
            .component::<TextFieldContent>()
            .unwrap()
            .surface
            .input;
        if f.host.focused() != Some(COMPOSER)
            || input.text != "Original composer draft"
            || input.selection() != (0..8)
            || input.marked.is_some()
        {
            errors.push("original Composer focus/text/selection changed".to_string());
        }
        if !self.actions.is_empty() {
            errors.push("passive placement dispatched original actions".into());
        }
        if owner.mount_identity() != self.owner_mount
            || popup
                .version()
                .is_none_or(|version| version.owner() != TICK_A)
        {
            errors.push("original anchor identity changed".into());
        }
        let open = matches!(id, 46101 | 46103 | 46105);
        if popup.is_open() != open {
            errors.push(format!("unexpected placement lifetime at{id}"));
        }
        let gap = 6.;
        let epsilon = 0.5;
        let right = anchor.max_x() + gap;
        let left = anchor.min_x() - gap - frame.size.width;
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
            if !owner
                .visible()
                .is_some_and(|visible| visible.contains(anchor.center()))
            {
                errors.push("native anchor center is not visible".into());
            }
            if frame.size.width < 180. {
                errors.push("bounded long body did not create placement pressure".into());
            }
            if frame.min_x() < bounds.min_x() - epsilon
                || frame.max_x() > bounds.max_x() + epsilon
                || frame.min_y() < bounds.min_y() - epsilon
                || frame.max_y() > bounds.max_y() + epsilon
            {
                errors.push("actual native preview escaped host viewport".into());
            }
            for target in [BODY, INPUT, OUTPUT] {
                if !nodes.iter().any(|node| node.id == target) {
                    errors.push(format!("original passive subtree node{target:?} missing"));
                }
            }
            if (frame.min_y() - anchor.min_y()).abs() > epsilon {
                errors.push("RightStart original anchor alignment changed".into());
            }
            match id {
                46101
                    if right + frame.size.width > bounds.max_x() + epsilon
                        || (frame.min_x() - right).abs() > epsilon =>
                {
                    errors.push("right-fit case did not actually fit/prefer right".into());
                }
                46103
                    if right + frame.size.width <= bounds.max_x() + epsilon
                        || left < bounds.min_x() - epsilon
                        || (frame.max_x() - (anchor.min_x() - gap)).abs() > epsilon =>
                {
                    errors.push(
                        "native flip case did not overflow right and fit exactly left".into(),
                    );
                }
                46105
                    if right + frame.size.width <= bounds.max_x() + epsilon
                        || left >= bounds.min_x() - epsilon
                        || (frame.max_x() - bounds.max_x()).abs() > epsilon
                        || (frame.min_x() - right).abs() <= epsilon
                        || (frame.min_x() - left).abs() <= epsilon =>
                {
                    errors.push("native clamp case did not overflow both preferred sides and clamp to viewport".into());
                }
                _ => {}
            }
        }
        if id == LAST && self.completed != (FIRST..LAST).collect::<Vec<_>>() {
            errors.push("placement checkpoints missing/reordered".into());
        }
        let report = serde_json::json!({"phase":id,"passed":errors.is_empty(),"errors":errors,"case":match id {46101=>"right-fit",46103=>"flip-left",46105=>"both-sides-clamp",_=>"retired"},"anchor":rect(anchor),"popup":rect(frame),"viewport":rect(bounds),"right_candidate_x":right,"left_candidate_x":left,"gap":gap,"owner_mount":self.owner_mount,"text":input.text,"selection":input.selection(),"marked":input.marked,"focus":f.host.focused().map(|id|id.0),"activations":self.actions.iter().map(|id|id.0).collect::<Vec<_>>(),"native_software_only":true,"physical_ime":"not_run","voiceover":"not_run"});
        if let Some(path) = evidence {
            std::fs::create_dir_all(path).unwrap();
            std::fs::write(
                path.join(format!("checkpoint-{id}.json")),
                serde_json::to_vec_pretty(&report).unwrap(),
            )
            .unwrap();
        }
        eprintln!("HOVER_PLACEMENT_CHECK {report}");
        assert!(
            errors.is_empty(),
            "native placement checkpoint{id}:{errors:?}"
        );
        self.completed.push(id);
    }
}
