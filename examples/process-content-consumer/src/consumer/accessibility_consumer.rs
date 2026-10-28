//! Exact-stamped original child semantic query/action; no platform AX claim.
use super::*;
use tekes_ui_module_runtime::{NativeViewOptions, ProcessContentBridge};
pub(super) const MODULE: &str = "original-content-accessibility";
const CAPS: ViewCapabilities = ViewCapabilities {
    accessibility: true,
    ..super::CAPS
};
struct SemanticModule(Module);
pub(super) fn create() -> Box<dyn ProcessModule> {
    Box::new(SemanticModule(original_semantic_module()))
}
pub(super) fn original_semantic_module() -> Module {
    let mut module = original_module(false);
    module
        .host
        .insert(
            ComponentId(1),
            Content::new(ComponentId(7), Button::new("Primary action")),
        )
        .unwrap();
    module
        .host
        .insert(
            ComponentId(1),
            Content::new(
                ComponentId(8),
                SliderContent::new(
                    "Level",
                    tekes_ui_controls::extended::Slider::new(4., 2., 10., 1.).unwrap(),
                ),
            ),
        )
        .unwrap();
    module
        .host
        .insert(
            ComponentId(1),
            Content::new(
                ComponentId(9),
                TextFieldContent::read_only_content(
                    "History",
                    "A😀B\nsecond line\nthird line",
                    proposal::TextRole::UiBody,
                ),
            ),
        )
        .unwrap();
    module
}
impl ProcessModule for SemanticModule {
    fn content(&mut self) -> Option<(&mut ContentHost, &mut dyn ButtonTextRenderer)> {
        Some((&mut self.0.host, &mut self.0.text))
    }
    fn view(&mut self) -> Option<&mut dyn PanelModule> {
        Some(&mut self.0)
    }
    fn view_capabilities(&self) -> ViewCapabilities {
        CAPS
    }
    fn started(&mut self, e: &ComponentEmitter) -> Result<()> {
        self.0.started(e)
    }
    fn handle(&mut self, r: RequestBody, e: &ComponentEmitter) -> Result<serde_json::Value> {
        if matches!(&r,RequestBody::Message{topic,..} if topic=="inspect-primary") {
            return Ok(
                serde_json::json!({"activations":self.0.host.get(ComponentId(7)).unwrap().component::<Button>().unwrap().activations}),
            );
        }
        if matches!(&r,RequestBody::Message{topic,..} if topic=="oversized_semantics") {
            self.0.host.insert(
                ComponentId(1),
                Content::new(
                    ComponentId(90),
                    TextContent::new("oversized".repeat(131072)),
                ),
            )?;
            return Ok(serde_json::Value::Null);
        }
        self.0.handle(r, e)
    }
}
pub(super) fn roundtrip() -> Result<serde_json::Value> {
    let mut rows = vec![];
    let processes = (0..3)
        .map(|i| {
            ProcessComponent::spawn(
                ProcessComponentOptions::new(MODULE, 610).with_view(61, 700 + i, CAPS),
                || {},
            )
        })
        .collect::<Result<Vec<_>>>()?;
    for (i, p) in processes.iter().enumerate() {
        wait(|| matches!(p.status(), ProcessStatus::Ready { .. }))?;
        layout(p, 1, 1, 300)?;
        let opts = NativeViewOptions {
            window: 61,
            view: 700 + i as u64,
            required: CAPS,
        };
        let frame = Rect::new(i as f32 * 300., 0., 300., 300.);
        let bridge = ProcessContentBridge::new(p, &opts, 1, frame, frame, Duration::from_secs(2))?;
        let q = bridge.semantics()?;
        let nodes = q
            .reply()
            .semantics
            .as_ref()
            .context("semantic nodes missing")?;
        ensure!(!q.reply().changed && !q.reply().consumed);
        let edit = nodes
            .iter()
            .find(|n| n.target.id == EDIT.0)
            .context("editor missing")?;
        let other = nodes
            .iter()
            .find(|n| n.target.id == OTHER.0)
            .context("other missing")?
            .target;
        let aux = nodes
            .iter()
            .find(|n| n.target.id == 3)
            .context("action missing")?
            .target;
        ensure!(
            edit.text
                .as_ref()
                .context("original shaped text missing")?
                .text
                == "A😀B"
        );
        ensure!(
            nodes.iter().all(|n| n.target.id != 6),
            "closed overlay child leaked"
        );
        let focus =
            bridge.accessibility_action(&q, edit.target, None, wire::AccessibilityAction::Focus)?;
        ensure!(focus.reply().focused == Some(edit.target));
        let q = bridge.semantics()?;
        let selection = bridge.accessibility_action(
            &q,
            edit.target,
            None,
            wire::AccessibilityAction::SetSelection(1..3),
        )?;
        ensure!(selection.reply().text.as_ref().unwrap().selection == (1..3));
        let marked = bridge.request(
            &selection,
            wire::Input::Preedit {
                text: "ni".into(),
                selection: 2..2,
                clauses: std::iter::once(0..2).collect(),
                replacement: Some(1..3),
            },
        )?;
        ensure!(marked.reply().composing);
        let q = bridge.semantics()?;
        let blocked =
            bridge.accessibility_action(&q, other, None, wire::AccessibilityAction::Focus)?;
        ensure!(
            !blocked.reply().consumed
                && blocked.reply().focused == Some(edit.target)
                && blocked.reply().composing
        );
        let q = bridge.semantics()?;
        let action =
            bridge.accessibility_action(&q, aux, None, wire::AccessibilityAction::Press)?;
        ensure!(
            action.reply().activated
                && action.reply().composing
                && action.reply().focused == Some(edit.target)
        );
        let stale = wire::Target {
            mount: edit.target.mount + 1,
            ..edit.target
        };
        ensure!(
            bridge
                .accessibility_action(&q, stale, None, wire::AccessibilityAction::Press)
                .is_err()
        );
        let q = bridge.semantics()?;
        ensure!(
            bridge
                .accessibility_action(
                    &q,
                    edit.target,
                    None,
                    wire::AccessibilityAction::SetSelection(std::ops::Range { start: 3, end: 1 })
                )
                .is_err()
        );
        // Sender-side rejection must leave the actor alive and marked state intact.
        let q = bridge.semantics()?;
        ensure!(q.reply().composing);
        let commit = bridge.request(
            &q,
            wire::Input::Commit {
                text: "你".into(),
                replacement: None,
            },
        )?;
        ensure!(commit.reply().text.as_ref().unwrap().text == "A你B" && !commit.reply().composing);
        let shifted = Rect::new(frame.min_x() + 10., 20., 280., 280.);
        let new_bridge =
            ProcessContentBridge::new(p, &opts, 1, shifted, shifted, Duration::from_secs(2))?;
        ensure!(
            new_bridge
                .accessibility_action(&q, edit.target, None, wire::AccessibilityAction::Press)
                .is_err(),
            "old allocation callback accepted"
        );
        let sibling = &processes[(i + 1) % 3];
        wait(|| matches!(sibling.status(), ProcessStatus::Ready { .. }))?;
        let sibling_opts = NativeViewOptions {
            view: 700 + ((i + 1) % 3) as u64,
            ..opts
        };
        let sibling_bridge = ProcessContentBridge::new(
            sibling,
            &sibling_opts,
            1,
            frame,
            frame,
            Duration::from_secs(2),
        )?;
        ensure!(
            sibling_bridge
                .accessibility_action(&q, edit.target, None, wire::AccessibilityAction::Press)
                .is_err(),
            "sibling stamp accepted"
        );
        let before_remount = bridge.semantics()?;
        process_message(p, "remount_aux")?;
        ensure!(
            bridge
                .accessibility_action(&before_remount, aux, None, wire::AccessibilityAction::Press)
                .is_err(),
            "replaced mount accepted late AX"
        );
        layout(p, 2, 1, 300)?;
        let before_overlay = bridge.semantics()?;
        process_message(p, "open_pointer_scope")?;
        layout(p, 3, 1, 300)?;
        let popup = bridge.semantics()?;
        let popup_nodes = popup.reply().semantics.as_ref().unwrap();
        ensure!(
            popup_nodes.iter().any(|n| n.target.id == 6)
                && popup_nodes.iter().all(|n| n.target.id != EDIT.0)
        );
        ensure!(
            bridge
                .accessibility_action(
                    &before_overlay,
                    edit.target,
                    None,
                    wire::AccessibilityAction::Focus
                )
                .is_err(),
            "background AX accepted through popup"
        );
        process_message(p, "close_pointer_scope")?;
        layout(p, 4, 1, 300)?;
        process_message(p, "oversized_semantics")?;
        ensure!(
            bridge.semantics().is_err(),
            "oversized semantic packet accepted"
        );
        ensure!(
            bridge.snapshot().is_ok(),
            "oversized semantic reply killed actor"
        );
        rows.push(serde_json::json!({"pid":match p.status(){ProcessStatus::Ready{pid,..}=>pid,_=>unreachable!()},"session":p.session(),"semantic_nodes":nodes.len(),"read_only_query":true,"UTF16_selection":true,"marked_auxiliary_action":true,"composition_focus_barrier":true,"stale_mount_allocation_sibling_rejected":true}));
    }
    let denied = ProcessComponent::spawn(
        ProcessComponentOptions::new(super::MODULE, 611).with_view(61, 999, super::CAPS),
        || {},
    )?;
    wait(|| matches!(denied.status(), ProcessStatus::Ready { .. }))?;
    layout(&denied, 1, 1, 300)?;
    let opts = NativeViewOptions {
        window: 61,
        view: 999,
        required: super::CAPS,
    };
    let f = Rect::new(0., 0., 300., 300.);
    let bridge = ProcessContentBridge::new(&denied, &opts, 1, f, f, Duration::from_secs(2))?;
    ensure!(
        bridge.semantics().is_err(),
        "unnegotiated AX query accepted"
    );
    ensure!(
        bridge.snapshot().is_ok(),
        "unnegotiated AX killed original actor"
    );
    denied.close();
    wait(|| matches!(denied.status(), ProcessStatus::Closed))?;
    for p in &processes {
        p.close();
        wait(|| matches!(p.status(), ProcessStatus::Closed))?;
    }
    Ok(
        serde_json::json!({"result":"ORIGINAL_REMOTE_SEMANTICS_ACTION_OK","runs":rows,"native":false,"GPU_visual":false,"VoiceOver":false,"performance":false,"remaining":"parent semantic projection and native text geometry/AX lifecycle"}),
    )
}
