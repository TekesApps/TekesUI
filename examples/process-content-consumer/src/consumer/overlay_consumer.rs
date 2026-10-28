//! One parent tree and one original remote Popover; no consumer scope state.
use super::*;
use tekes_ui_module_runtime::ProcessContainer;
const REMOTE: ComponentId = ComponentId(520);
const BACK: ComponentId = ComponentId(521);
const EDITOR: ComponentId = ComponentId(522);
fn ready(
    host: &mut ContentHost,
    renderer: &mut tekes_ui_render::Renderer,
    min_revision: u64,
) -> Result<()> {
    let frame = Rect::new(0., 0., 900., 420.);
    let end = Instant::now() + Duration::from_secs(15);
    loop {
        host.get_mut(REMOTE)
            .unwrap()
            .component_mut::<ProcessContainer>()
            .unwrap()
            .prepare(renderer)?;
        host.render(
            &mut Scene::new(),
            &mut Text,
            frame,
            frame,
            Theme::LIGHT.into(),
        )?;
        let c = host
            .get(REMOTE)
            .unwrap()
            .component::<ProcessContainer>()
            .unwrap();
        ensure!(c.error().is_none(), "overlay worker error {:?}", c.error());
        if c.input_ready() && c.revision().is_some_and(|v| v >= min_revision) {
            return Ok(());
        }
        ensure!(
            Instant::now() < end,
            "overlay layout acknowledgement deadline"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}
pub(super) fn roundtrip() -> Result<serde_json::Value> {
    let mut host = ContentHost::new(Content::new(
        ComponentId(519),
        GridContent {
            model: tekes_ui_controls::extended::Grid::new(3),
            children: vec![
                Content::new(
                    REMOTE,
                    ProcessContainer::new_view(
                        ProcessComponentOptions::new(MODULE, 101).with_view(17, 101, CAPS),
                        || {},
                    )?,
                ),
                Content::new(BACK, Button::new("Parent background")),
                Content::new(EDITOR, TextFieldContent::new("Parent editor", "keep")),
            ],
        },
    ))?;
    host.set_action_events(true);
    let mut renderer = tekes_ui_render::Renderer::new_headless()?;
    ready(&mut host, &mut renderer, 0)?;
    host.focus(Some(EDITOR), true);
    eprintln!(
        "REMOTE_OVERLAY before opening parent focus {:?}",
        host.focused()
    );
    let c = host
        .get(REMOTE)
        .unwrap()
        .component::<ProcessContainer>()
        .unwrap();
    let session = c.session().unwrap().clone();
    container_message(c, "open_pointer_scope")?;
    let state = inspect_container(c)?;
    ready(
        &mut host,
        &mut renderer,
        state["view_revision"].as_u64().unwrap(),
    )?;
    let targets: Vec<_> = host.focus_targets().iter().map(|v| v.0).collect();
    ensure!(
        targets == vec![REMOTE],
        "remote original Popover did not isolate parent focus: {targets:?}"
    );
    let pid = match host
        .get(REMOTE)
        .unwrap()
        .component::<ProcessContainer>()
        .unwrap()
        .status()
        .unwrap()
    {
        ProcessStatus::Ready { pid, .. } => pid,
        other => anyhow::bail!("unexpected worker status {other:?}"),
    };
    ensure!(
        host.semantics()
            .iter()
            .all(|node| node.id != BACK && node.id != EDITOR),
        "parent AX projection leaked background"
    );
    ensure!(
        !host
            .accessibility_action(BACK, None, AccessibilityAction::Press)
            .consumed,
        "background AX remained executable"
    );
    ensure!(
        !host.focus(Some(EDITOR), true),
        "background explicit focus bypassed remote scope"
    );
    for reverse in [false, true, false, true, false, true] {
        host.event(&ContentEvent::Key {
            key: tekes_ui_controls::ControlKey::Tab { reverse },
            down: true,
        });
        ensure!(
            host.focused() == Some(REMOTE),
            "Tab escaped remote original subtree"
        );
    }
    // Native provenance must delegate to the original popup host, not run a
    // parent fallback or synthesize another action.
    let key = ContentKeyEvent {
        key: Some(tekes_ui_controls::ControlKey::Tab { reverse: false }),
        down: true,
        repeat: false,
        modifiers: Default::default(),
        native_code: 48,
        characters: "\t".into(),
        characters_ignoring_modifiers: "\t".into(),
    };
    host.event(&ContentEvent::TextKeyStart {
        id: 9901,
        key: key.clone(),
        composing: false,
    });
    host.event(&ContentEvent::KeyInput(key));
    let finished = host.event(&ContentEvent::TextKeyEnd {
        id: 9901,
        input_context_consumed: false,
    });
    ensure!(
        finished
            .text_key
            .is_some_and(|k| k.id == 9901 && k.disposition != TextKeyDisposition::Cancelled),
        "native Tab lost original popup owner"
    );
    ensure!(
        host.focused() == Some(REMOTE),
        "native Tab escaped parent scope"
    );
    let c = host
        .get(REMOTE)
        .unwrap()
        .component::<ProcessContainer>()
        .unwrap();
    let snapshot = c
        .native_content_bridge(Duration::from_secs(5))?
        .snapshot()?;
    ensure!(
        snapshot.reply().overlay.is_some_and(|owner| owner.id == 5),
        "wire scope is not original Popover identity"
    );
    let previous = snapshot;
    // Reach the existing real child action using ordinary Tab only.
    for _ in 0..4 {
        let c = host
            .get(REMOTE)
            .unwrap()
            .component::<ProcessContainer>()
            .unwrap();
        let snapshot = c
            .native_content_bridge(Duration::from_secs(5))?
            .snapshot()?;
        if snapshot.reply().focused.is_some_and(|v| v.id == 6) {
            break;
        }
        host.event(&ContentEvent::Key {
            key: tekes_ui_controls::ControlKey::Tab { reverse: false },
            down: true,
        });
    }
    host.event(&ContentEvent::Key {
        key: tekes_ui_controls::ControlKey::Enter,
        down: true,
    });
    host.event(&ContentEvent::Key {
        key: tekes_ui_controls::ControlKey::Enter,
        down: false,
    });
    let observation = host
        .get(REMOTE)
        .unwrap()
        .component::<ProcessContainer>()
        .unwrap()
        .native_content_bridge(Duration::from_secs(1))?
        .snapshot()?;
    ensure!(
        !observation.reply().changed,
        "read-only overlay query re-invalidated pending paint"
    );
    let state = inspect_container(
        host.get(REMOTE)
            .unwrap()
            .component::<ProcessContainer>()
            .unwrap(),
    )?;
    ensure!(
        state["overlay_activations"] == 1,
        "original popup action was lost or repeated: {state}"
    );
    ensure!(
        host.drain_actions().is_empty(),
        "parent emitted a duplicate child business action"
    );
    let background = host.get(BACK).unwrap().frame();
    let point = tekes_ui_scene::Point::new(
        background.min_x() + background.size.width * 0.5,
        background.min_y() + background.size.height * 0.5,
    );
    host.event(&ContentEvent::PointerDown(point));
    host.event(&ContentEvent::PointerUp(point));
    ensure!(
        host.get(BACK)
            .unwrap()
            .component::<Button>()
            .unwrap()
            .activations
            == 0,
        "outside dismissal clicked background"
    );
    ensure!(
        host.focused() == Some(EDITOR),
        "closing child scope did not restore original parent focus: parent={:?} scope={:?}",
        host.focused(),
        host.input_overlay_owner()
    );
    ensure!(
        host.focus_targets().iter().any(|(id, _)| *id == BACK),
        "closed overlay retained a parent barrier"
    );
    let c = host
        .get(REMOTE)
        .unwrap()
        .component::<ProcessContainer>()
        .unwrap();
    let bridge = c.native_content_bridge(Duration::from_secs(5))?;
    ensure!(
        bridge
            .request(
                &previous,
                wire::Input::Commit {
                    text: "late".into(),
                    replacement: None
                }
            )
            .is_err(),
        "stale popup callback retargeted restored input"
    );
    // A fresh click after dismissal must work once.
    host.event(&ContentEvent::PointerDown(point));
    host.event(&ContentEvent::PointerUp(point));
    ensure!(
        host.get(BACK)
            .unwrap()
            .component::<Button>()
            .unwrap()
            .activations
            == 1,
        "fresh background click failed"
    );
    host.drain_actions();
    // Opening a remote overlay cannot steal an existing parent capture.
    host.event(&ContentEvent::PointerDown(point));
    let capture = host
        .pointer_capture()
        .context("background did not capture")?;
    let c = host
        .get(REMOTE)
        .unwrap()
        .component::<ProcessContainer>()
        .unwrap();
    container_message(c, "open_pointer_scope")?;
    let revision = inspect_container(c)?["view_revision"].as_u64().unwrap();
    ready(&mut host, &mut renderer, revision)?;
    ensure!(
        host.pointer_capture() == Some(capture),
        "remote scope replaced existing parent capture"
    );
    host.event(&ContentEvent::PointerUp(point));
    ensure!(
        host.pointer_capture().is_none(),
        "original capture not released"
    );
    ensure!(
        host.get(BACK)
            .unwrap()
            .component::<Button>()
            .unwrap()
            .activations
            == 2,
        "held original gesture did not finish once"
    );
    host.event(&ContentEvent::Key {
        key: tekes_ui_controls::ControlKey::Escape,
        down: true,
    });
    ensure!(
        host.get(REMOTE)
            .unwrap()
            .component::<ProcessContainer>()
            .unwrap()
            .overlay_scope()
            .is_none(),
        "Escape retained remote barrier"
    );
    host.focus(Some(EDITOR), true);
    let c = host
        .get(REMOTE)
        .unwrap()
        .component::<ProcessContainer>()
        .unwrap();
    container_message(c, "open_pointer_scope")?;
    let revision = inspect_container(c)?["view_revision"].as_u64().unwrap();
    ready(&mut host, &mut renderer, revision)?;
    host.get_mut(REMOTE)
        .unwrap()
        .component_mut::<ProcessContainer>()
        .unwrap()
        .set_visible(false)?;
    host.event(&ContentEvent::PointerMove(point));
    ensure!(
        host.focus_targets().iter().all(|(id, _)| *id != REMOTE),
        "hidden original retained focus scope"
    );
    ensure!(
        host.semantics().iter().all(|node| node.id != REMOTE),
        "hidden original retained semantic projection"
    );
    host.get_mut(REMOTE)
        .unwrap()
        .component_mut::<ProcessContainer>()
        .unwrap()
        .set_visible(true)?;
    ready(&mut host, &mut renderer, 0)?;
    ensure!(
        host.get(REMOTE)
            .unwrap()
            .component::<ProcessContainer>()
            .unwrap()
            .overlay_scope()
            .is_none(),
        "show resurrected cancelled popup"
    );
    let removed = host.remove(REMOTE)?;
    ensure!(
        removed
            .component::<ProcessContainer>()
            .unwrap()
            .session()
            .is_none(),
        "unmount kept original process"
    );
    renderer.collect_retired_images();
    Ok(
        serde_json::json!({"result":"REMOTE_OVERLAY_SCOPE_OK","session":session,"pid":pid,"parent_background_click_tab_ax_isolated":true,"original_child_action_once":true,"native_tab_original_owner":true,"parent_focus_restored":true,"old_callback_rejected":true,"existing_parent_capture_preserved":true,"hide_show_unmount_clear_scope":true,"scope":"one real worker and original Popover Content; parent semantic exclusion is not remote AX tree/native VoiceOver or cross-container overlay-pixel projection acceptance"}),
    )
}
