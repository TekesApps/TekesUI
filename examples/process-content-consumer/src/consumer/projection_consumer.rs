//! Independent mounted Content consumer of projected child semantic parts.
use super::*;
use tekes_ui_module_runtime::ProcessContainer;
const IDS: [ComponentId; 3] = [ComponentId(621), ComponentId(622), ComponentId(623)];
const CAPS: ViewCapabilities = ViewCapabilities {
    accessibility: true,
    ..super::CAPS
};
#[derive(Debug)]
struct Row {
    children: Vec<Content>,
}
impl Component for Row {
    fn measure(
        &mut self,
        _: &mut dyn ButtonTextRenderer,
        a: Size,
    ) -> tekes_ui_controls::layout::MeasuredChild {
        tekes_ui_controls::layout::MeasuredChild::new(Size::new(a.width, 300.))
    }
    fn arrange(&mut self, t: &mut dyn ButtonTextRenderer, f: Rect, c: Rect) {
        let w = f.size.width / 3.;
        for (i, n) in self.children.iter_mut().enumerate() {
            n.arrange(
                t,
                Rect::new(f.min_x() + i as f32 * w, f.min_y(), w, f.size.height),
                c,
            );
        }
    }
    fn children(&self) -> &[Content] {
        &self.children
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.children)
    }
}
fn create_container(i: usize) -> Result<Content> {
    Ok(Content::new(
        IDS[i],
        ProcessContainer::new_view(
            ProcessComponentOptions::new(accessibility_consumer::MODULE, 620).with_view(
                62,
                621 + i as u64,
                CAPS,
            ),
            || {},
        )?,
    ))
}
fn container(h: &ContentHost, id: ComponentId) -> &ProcessContainer {
    h.get(id).unwrap().component::<ProcessContainer>().unwrap()
}
fn container_mut(h: &mut ContentHost, id: ComponentId) -> &mut ProcessContainer {
    h.get_mut(id)
        .unwrap()
        .component_mut::<ProcessContainer>()
        .unwrap()
}
fn ready(h: &mut ContentHost, gpu: &mut tekes_ui_render::Renderer, frame: Rect) -> Result<()> {
    ready_clipped(h, gpu, frame, frame)
}
fn ready_clipped(
    h: &mut ContentHost,
    gpu: &mut tekes_ui_render::Renderer,
    frame: Rect,
    clip: Rect,
) -> Result<()> {
    let end = Instant::now() + Duration::from_secs(15);
    loop {
        for id in IDS {
            container_mut(h, id).prepare(gpu)?;
        }
        h.render(
            &mut Scene::new(),
            &mut Text,
            frame,
            clip,
            Theme::LIGHT.into(),
        )?;
        let done = IDS.iter().all(|id| {
            let c = container(h, *id);
            !c.active() || c.input_ready() && !c.semantic_parts().is_empty()
        });
        for id in IDS {
            ensure!(
                container(h, id).error().is_none(),
                "projection worker {id:?}: {:?}",
                container(h, id).error()
            );
        }
        if done {
            return Ok(());
        }
        ensure!(Instant::now() < end, "projection readiness deadline");
        std::thread::sleep(Duration::from_millis(5));
    }
}
fn part(h: &ContentHost, id: ComponentId, name: &str) -> Result<usize> {
    container(h, id)
        .semantic_parts()
        .iter()
        .find(|p| p.semantics.name == name)
        .map(|p| p.index)
        .with_context(|| format!("projected {name} missing in {id:?}"))
}
fn message(c: &ProcessContainer, topic: &str) -> Result<serde_json::Value> {
    let id = c.request(RequestBody::Message {
        topic: topic.into(),
        payload: serde_json::Value::Null,
    })?;
    let mut result = None;
    wait(|| {
        for p in c.drain_events() {
            if let Output::Response { request, result: r } = p.output
                && request == id
            {
                result = Some(r);
            }
        }
        result.is_some()
    })?;
    result.unwrap().map_err(anyhow::Error::msg)
}
fn native_metadata(h: &ContentHost, id: ComponentId) -> Result<()> {
    use tekes_ui_platform_macos::{AccessibilityRole, content_accessibility as native};
    let edit = part(h, id, "Message")?;
    let history = part(h, id, "History")?;
    let slider = part(h, id, "Level")?;
    let key = |part| native::content_key(h, id, Some(part)).unwrap();
    let nodes = native::content_nodes(h);
    let layouts = native::content_text_layouts(h);
    let field = nodes
        .iter()
        .find(|n| n.key == key(edit))
        .context("native editor missing")?;
    ensure!(field.role == AccessibilityRole::TextField && field.text_selection.is_some());
    let history_node = nodes
        .iter()
        .find(|n| n.key == key(history))
        .context("native history missing")?;
    ensure!(history_node.role == AccessibilityRole::TextArea && history_node.read_only);
    let history = layouts
        .get(&key(history))
        .context("complete original history layout missing")?;
    ensure!(
        history.lines.len() >= 3
            && history.range_for_line(0) == Some(0..5)
            && history.range_for_line(1) == Some(5..17)
    );
    ensure!(
        history.range_for_index(2) == Some(1..3),
        "surrogate mapped by guessed character width"
    );
    ensure!(
        matches!(nodes.iter().find(|n|n.key==key(slider)).context("native slider missing")?.role,
        AccessibilityRole::Slider(r) if r.minimum()==2. && r.maximum()==10.)
    );
    let editor = layouts
        .get(&key(edit))
        .context("unfocused editor layout missing")?;
    ensure!(
        editor.is_valid()
            && editor
                .state
                .carets
                .iter()
                .all(|(_, p, _)| p.x >= h.get(id).unwrap().frame().min_x() as f64)
    );
    Ok(())
}
pub(super) fn roundtrip() -> Result<serde_json::Value> {
    let children = (0..3).map(create_container).collect::<Result<Vec<_>>>()?;
    let mut h = ContentHost::new(Content::new(ComponentId(620), Row { children }))?;
    let mut gpu = tekes_ui_render::Renderer::new_headless()?;
    let original_images = gpu.image_count();
    let frame = Rect::new(20., 40., 900., 300.);
    ready(&mut h, &mut gpu, frame)?;
    let mut rows = vec![];
    for id in IDS {
        native_metadata(&h, id)?;
        let edit = part(&h, id, "Message")?;
        let aux = part(&h, id, "Auxiliary")?;
        let other = part(&h, id, "Other")?;
        let p = container(&h, id)
            .semantic_parts()
            .into_iter()
            .find(|p| p.index == edit)
            .unwrap();
        ensure!(
            p.frame.min_x() >= h.get(id).unwrap().frame().min_x()
                && p.frame.max_x() <= h.get(id).unwrap().frame().max_x()
        );
        ensure!(
            h.semantic_part_parent(id, edit).is_some(),
            "original hierarchy missing"
        );
        ensure!(
            h.accessibility_action(id, Some(edit), AccessibilityAction::Focus)
                .consumed
        );
        ensure!(
            h.focused() == Some(id),
            "delegated Focus failed parent acquisition"
        );
        ensure!(
            h.accessibility_action(id, Some(edit), AccessibilityAction::SetSelection(1..3))
                .consumed
        );
        ensure!(h.text_input_snapshot().unwrap().selection == (1..3));
        h.event(&ContentEvent::Preedit {
            text: "ni".into(),
            selection: 2..2,
            clauses: std::iter::once(0..2).collect(),
            replacement: Some(1..3),
        });
        ready(&mut h, &mut gpu, frame)?;
        let before = h.text_input_snapshot().unwrap();
        let before_aux = message(container(&h, id), "inspect")?["aux_activations"]
            .as_u64()
            .unwrap();
        let response = h.accessibility_action(id, Some(aux), AccessibilityAction::Press);
        ensure!(
            response.consumed && !response.activated,
            "parent duplicated child action"
        );
        ensure!(h.focused() == Some(id) && h.is_composing());
        ensure!(
            !h.accessibility_action(id, Some(other), AccessibilityAction::Focus)
                .consumed
        );
        let after = h.text_input_snapshot().unwrap();
        ensure!(
            before.text == after.text
                && before.marked == after.marked
                && before.selection == after.selection
        );
        let state = message(container(&h, id), "inspect")?;
        ensure!(state["aux_activations"].as_u64() == Some(before_aux + 1) && state["submits"] == 0);
        let sibling = IDS[(IDS.iter().position(|i| *i == id).unwrap() + 1) % 3];
        let sibling_aux = part(&h, sibling, "Auxiliary")?;
        let before_sibling = message(container(&h, sibling), "inspect")?["aux_activations"]
            .as_u64()
            .unwrap();
        ensure!(
            h.accessibility_action(sibling, Some(sibling_aux), AccessibilityAction::Press)
                .consumed
        );
        ensure!(
            h.focused() == Some(id) && h.is_composing(),
            "auxiliary sibling stole composing focus"
        );
        ensure!(
            message(container(&h, sibling), "inspect")?["aux_activations"].as_u64()
                == Some(before_sibling + 1)
        );
        let sibling_edit = part(&h, sibling, "Message")?;
        ensure!(
            !h.accessibility_action(sibling, Some(sibling_edit), AccessibilityAction::Focus)
                .consumed
        );
        h.event(&ContentEvent::Commit {
            text: "你".into(),
            replacement: None,
        });
        ready(&mut h, &mut gpu, frame)?;
        ensure!(!h.is_composing() && h.text_input_snapshot().unwrap().text == "A你B");
        rows.push(serde_json::json!({"pid":match container(&h,id).status(){Some(ProcessStatus::Ready{pid,..})=>pid,_=>unreachable!()},"parent_mount":h.semantic_mount(id),"session":container(&h,id).session(),"parent":id.0,"editor_part":edit,"hierarchy":true,"UTF16_selection":true,"marked_auxiliary":true,"parent_action_not_duplicated":true}));
    }
    ensure!(
        IDS.iter()
            .map(|id| h.semantic_mount(*id).unwrap())
            .collect::<std::collections::HashSet<_>>()
            .len()
            == 3,
        "parent namespaces collided"
    );
    h.focus(None, false);
    ready(&mut h, &mut gpu, frame)?;
    let primary = part(&h, IDS[0], "Primary action")?;
    let primary_result = h.accessibility_action(IDS[0], Some(primary), AccessibilityAction::Press);
    ensure!(
        primary_result.consumed && !primary_result.activated && h.focused() == Some(IDS[0]),
        "delegated Press failed parent focus or duplicated action"
    );
    ensure!(message(container(&h, IDS[0]), "inspect-primary")?["activations"] == 1);
    ready(&mut h, &mut gpu, frame)?;
    let old = IDS.map(|id| {
        (
            part(&h, id, "Message").unwrap(),
            container(&h, id)
                .semantic_parts()
                .into_iter()
                .find(|p| p.semantics.name == "Message")
                .unwrap()
                .frame,
        )
    });
    let resized = Rect::new(55., 70., 810., 320.);
    ready(&mut h, &mut gpu, resized)?;
    for (i, id) in IDS.into_iter().enumerate() {
        native_metadata(&h, id)?;
        ensure!(
            part(&h, id, "Message")? == old[i].0,
            "resize changed stable owner identity"
        );
        let p = container(&h, id)
            .semantic_parts()
            .into_iter()
            .find(|p| p.index == old[i].0)
            .unwrap();
        ensure!(p.frame != old[i].1);
    }
    for id in IDS {
        container_mut(&mut h, id).set_scale(2.)?;
    }
    ready(&mut h, &mut gpu, resized)?;
    for (i, id) in IDS.into_iter().enumerate() {
        ensure!(part(&h, id, "Message")? == old[i].0);
    }
    let clip = Rect::new(
        resized.min_x(),
        resized.min_y() + 20.,
        resized.size.width,
        resized.size.height - 40.,
    );
    ready_clipped(&mut h, &mut gpu, resized, clip)?;
    for id in IDS {
        for p in container(&h, id).semantic_parts() {
            if p.frame.size.width > 0. && p.frame.size.height > 0. {
                ensure!(
                    p.frame.min_x() >= clip.min_x()
                        && p.frame.max_x() <= clip.max_x()
                        && p.frame.min_y() >= clip.min_y()
                        && p.frame.max_y() <= clip.max_y(),
                    "semantic geometry escaped ancestor clip"
                );
            }
        }
    }
    ready(&mut h, &mut gpu, resized)?;
    let id = IDS[0];
    let pre_binding = part(&h, id, "Message")?;
    let text_before = message(container(&h, id), "inspect")?["edit_text"].clone();
    message(container(&h, id), "bind_same_value")?;
    // Deliberately deliver BEFORE prepare refreshes metadata. The original
    // target binding, not the still-focused button lease, must reject this.
    ensure!(
        !h.accessibility_action(
            id,
            Some(pre_binding),
            AccessibilityAction::SetSelection(0..0)
        )
        .consumed,
        "late pre-refresh AX edited rebound target"
    );
    ready(&mut h, &mut gpu, resized)?;
    ensure!(
        part(&h, id, "Message")? != pre_binding,
        "document binding reused old semantic part"
    );
    ensure!(
        !h.accessibility_action(
            id,
            Some(pre_binding),
            AccessibilityAction::SetSelection(0..0)
        )
        .consumed,
        "old document AX selection reached new binding"
    );
    ensure!(message(container(&h, id), "inspect")?["edit_text"] == text_before);
    let old_edit = part(&h, id, "Message")?;
    let old_aux = part(&h, id, "Auxiliary")?;
    // An old child mount must fail without cancelling the currently composing owner.
    ensure!(
        h.accessibility_action(id, Some(old_edit), AccessibilityAction::Focus)
            .consumed
    );
    h.event(&ContentEvent::Preedit {
        text: "ni".into(),
        selection: 2..2,
        clauses: std::iter::once(0..2).collect(),
        replacement: None,
    });
    ready(&mut h, &mut gpu, resized)?;
    message(container(&h, id), "remount_aux")?;
    ensure!(
        !h.accessibility_action(id, Some(old_aux), AccessibilityAction::Press)
            .consumed
    );
    ensure!(
        h.is_composing() && h.text_input_snapshot().unwrap().marked.is_some(),
        "stale AX cancelled current editor"
    );
    ready(&mut h, &mut gpu, resized)?;
    ensure!(part(&h, id, "Auxiliary")? != old_aux);
    h.event(&ContentEvent::Unmark);
    ready(&mut h, &mut gpu, resized)?;
    message(container(&h, id), "open_pointer_scope")?;
    ready(&mut h, &mut gpu, resized)?;
    ensure!(
        container(&h, id)
            .semantic_parts()
            .iter()
            .any(|p| p.semantics.name == "Overlay action")
    );
    ensure!(
        container(&h, id)
            .semantic_parts()
            .iter()
            .all(|p| p.semantics.name != "Message")
    );
    ensure!(
        !h.accessibility_action(id, Some(old_edit), AccessibilityAction::Focus)
            .consumed
    );
    message(container(&h, id), "close_pointer_scope")?;
    ready(&mut h, &mut gpu, resized)?;
    let pre_hide = part(&h, id, "Message")?;
    let session = container(&h, id).session().cloned().unwrap();
    container_mut(&mut h, id).set_visible(false)?;
    h.render(
        &mut Scene::new(),
        &mut Text,
        resized,
        resized,
        Theme::LIGHT.into(),
    )?;
    ensure!(h.semantics().iter().all(|n| n.id != id));
    ensure!(
        !h.accessibility_action(id, Some(pre_hide), AccessibilityAction::Focus)
            .consumed
    );
    container_mut(&mut h, id).set_visible(true)?;
    ready(&mut h, &mut gpu, resized)?;
    ensure!(container(&h, id).session() == Some(&session) && part(&h, id, "Message")? != pre_hide);
    ensure!(
        !h.accessibility_action(id, Some(pre_hide), AccessibilityAction::Focus)
            .consumed
    );
    // Inactive projection is withdrawn; reactivation creates a new lease.
    let pre_blur = part(&h, id, "Message")?;
    container_mut(&mut h, id).set_window_active(false);
    ensure!(container(&h, id).semantic_parts().is_empty());
    ensure!(
        !h.accessibility_action(id, Some(pre_blur), AccessibilityAction::Focus)
            .consumed
    );
    container_mut(&mut h, id).set_window_active(true);
    ready(&mut h, &mut gpu, resized)?;
    ensure!(part(&h, id, "Message")? != pre_blur);
    let old_mount = h.semantic_mount(id).unwrap();
    let original = container(&h, id)
        .native_content_bridge(Duration::from_secs(1))?
        .semantics()?;
    drop(h.remove(id)?);
    h.insert(ComponentId(620), create_container(0)?)?;
    ready(&mut h, &mut gpu, resized)?;
    ensure!(
        h.semantic_mount(id) != Some(old_mount) && container(&h, id).session() != Some(&session)
    );
    let target = original
        .reply()
        .semantics
        .as_ref()
        .unwrap()
        .iter()
        .find(|n| n.target.id == EDIT.0)
        .unwrap()
        .target;
    ensure!(
        container(&h, id)
            .native_content_bridge(Duration::from_secs(1))?
            .accessibility_action(&original, target, None, wire::AccessibilityAction::Press)
            .is_err()
    );
    drop(h);
    gpu.collect_retired_images();
    ensure!(
        gpu.image_count() == original_images,
        "unmount retained imported GPU resources"
    );
    Ok(
        serde_json::json!({"result":"MOUNTED_REMOTE_SEMANTICS_OK","runs":rows,"native_text_range_projection":true,"original_shaped_multiline":true,"readonly_unfocused_text":true,"slider_domains":true,"resize_stable_keys":true,"backing_scale_logical_points":true,"ancestor_clip":true,"primary_Press_focus_once":true,"document_rebind_revokes_old_selection":true,"GPU_resources_retired":true,"stale_AX_preserves_new_preedit":true,"overlay_scope":true,"hide_show_and_activation_revoke_old_parts":true,"remount_session_and_parent_mount_changed":true,"native":false,"VoiceOver":false,"GPU_visual":false,"performance":false}),
    )
}
