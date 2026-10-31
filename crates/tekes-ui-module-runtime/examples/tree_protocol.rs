//! Issue #75 independent public consumer: real process, original Tree owner.
use anyhow::{Context, Result, bail, ensure};
use serde_json::{Value, json};
use std::time::{Duration, Instant};
use tekes_ui_app_protocol::Request;
use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer,
    composition::*,
    extended::{KeyedTreeNode, TreeDatasetKey as D, TreeNodeKey as K},
    layout_view::IconRenderer,
};
use tekes_ui_module_runtime::{component::*, *};
use tekes_ui_scene::{GlyphAtlas, ImageId, Rect, Scene, Size};
use tekes_ui_tokens::{Rgba, Theme, proposal};
const ID: ComponentId = ComponentId(10);
const CAPS: ViewCapabilities = ViewCapabilities {
    accessibility: true,
    primary_pointer: false,
    content_pointer: false,
    keyboard: false,
    scroll: false,
    text_ime: false,
    independent_toolbar: false,
};
struct Icons;
impl IconRenderer for Icons {
    fn draw_icon(&mut self, _: &mut Scene, _: &str, _: Rect, _: Rgba) {}
    fn prepare_icon(
        &mut self,
        _: &str,
        _: f32,
        _: f32,
        _: Rgba,
    ) -> Result<IconImage, IconRenderError> {
        Ok(IconImage::new(ImageId(1), std::rc::Rc::new(())))
    }
}
struct Text(Icons);
impl ButtonTextRenderer for Text {
    fn icon_renderer(&mut self) -> Option<&mut dyn IconRenderer> {
        Some(&mut self.0)
    }
    fn measure(&mut self, s: &str, _: proposal::TextRole, w: f32) -> Size {
        Size::new((s.len() as f32 * 8.).min(w), 20.)
    }
    fn draw(
        &mut self,
        _: &mut Scene,
        _: &str,
        _: proposal::TextRole,
        _: Rect,
        _: ButtonTextAlign,
        _: Rgba,
    ) {
    }
}
fn nodes() -> Vec<TreeContentNode> {
    vec![(1, None), (2, Some(1)), (3, Some(2)), (4, None)]
        .into_iter()
        .map(|(k, p)| TreeContentNode {
            row_id: ComponentId(100 + k),
            node: KeyedTreeNode {
                key: K(k),
                parent: p.map(K),
                label: format!("node{k}"),
                disabled: false,
                initially_expanded: true,
            },
            display: TreeDisplayUpdate::Keep,
        })
        .collect()
}
fn tree() -> Content {
    Content::new(ID, TreeContent::new("Tree", D(1), nodes()).unwrap())
}
#[derive(Debug)]
struct Malformed {
    orphan: bool,
}
impl Component for Malformed {
    fn measure(
        &mut self,
        _: &mut dyn ButtonTextRenderer,
        a: Size,
    ) -> tekes_ui_controls::layout::MeasuredChild {
        tekes_ui_controls::layout::MeasuredChild::new(a)
    }
    fn semantics(&self) -> ContentSemantics {
        ContentSemantics::new(ContentRole::Tree, "malformed")
    }
    fn semantic_parts(&self) -> Vec<SemanticPart> {
        vec![SemanticPart {
            index: 1,
            frame: Rect::new(0., 0., 80., 30.),
            focused: false,
            semantics: ContentSemantics::new(
                ContentRole::TreeItem {
                    level: if self.orphan { 0 } else { usize::MAX },
                },
                "invalid",
            ),
        }]
    }
    fn semantic_part_parent(&self, _: usize) -> Option<usize> {
        self.orphan.then_some(999)
    }
}
struct Child {
    host: ContentHost,
    text: Text,
    revision: u64,
}
fn create_mode(mode: u8) -> Box<dyn ProcessModule> {
    let body = if mode == 0 {
        tree()
    } else {
        Content::new(ID, Malformed { orphan: mode == 1 })
    };
    Box::new(Child {
        host: ContentHost::new(Content::new(
            ComponentId(1),
            ListContent::new("Child", vec![body]),
        ))
        .unwrap(),
        text: Text(Icons),
        revision: 0,
    })
}
fn create() -> Box<dyn ProcessModule> {
    create_mode(0)
}
fn orphan() -> Box<dyn ProcessModule> {
    create_mode(1)
}
fn level() -> Box<dyn ProcessModule> {
    create_mode(2)
}
impl PanelModule for Child {
    fn update(&mut self, r: &Request, s: &mut Scene, _: &mut GlyphAtlas) -> Result<ModuleState> {
        let f = Rect::new(0., 0., r.width as f32, r.height as f32);
        self.host
            .render(s, &mut self.text, f, f, Theme::LIGHT.into())?;
        Ok(ModuleState {
            revision: self.revision,
            toolbar: vec![],
        })
    }
}
impl ProcessModule for Child {
    fn view(&mut self) -> Option<&mut dyn PanelModule> {
        Some(self)
    }
    fn content(&mut self) -> Option<(&mut ContentHost, &mut dyn ButtonTextRenderer)> {
        Some((&mut self.host, &mut self.text))
    }
    fn view_capabilities(&self) -> ViewCapabilities {
        CAPS
    }
    fn handle(&mut self, r: RequestBody, e: &ComponentEmitter) -> Result<Value> {
        let RequestBody::Message { topic, .. } = r else {
            bail!("unsupported")
        };
        match topic.as_str() {
            "inspect" => {
                let s = self.host.semantics();
                let n = s.iter().find(|n| n.id == ID);
                return Ok(
                    json!({"pid":std::process::id(),"parts":n.map(|n|n.parts.iter().map(|p|json!({"token":p.index,"name":p.semantics.name,"expanded":p.semantics.expanded})).collect::<Vec<_>>()),"focused_part":n.and_then(|n|n.parts.iter().find(|p|p.focused)).map(|p|p.semantics.name.clone()),"selected":self.host.get(ID).and_then(|n|n.component::<TreeContent>()).and_then(|t|t.selected_key()).map(|k|k.0)}),
                );
            }
            "remount" => {
                self.host.remove(ID)?;
                self.host.insert(ComponentId(1), tree())?;
            }
            "inactive" => {
                self.host.set_window_active(false);
            }
            "active" => {
                self.host.set_window_active(true);
            }
            "remove-restore" => {
                self.host.replace_tree_nodes(ID, D(1), vec![]).unwrap();
                self.host.replace_tree_nodes(ID, D(1), nodes()).unwrap();
            }
            _ => bail!("unknown message"),
        }
        self.revision += 1;
        e.invalidate_view(self.revision)?;
        Ok(Value::Null)
    }
}
fn container(h: &ContentHost) -> &ProcessContainer {
    h.get(ComponentId(50))
        .unwrap()
        .component::<ProcessContainer>()
        .unwrap()
}
fn container_mut(h: &mut ContentHost) -> &mut ProcessContainer {
    h.get_mut(ComponentId(50))
        .unwrap()
        .component_mut::<ProcessContainer>()
        .unwrap()
}
fn message(c: &ProcessContainer, topic: &str) -> Result<Value> {
    let id = c.request(RequestBody::Message {
        topic: topic.into(),
        payload: Value::Null,
    })?;
    let end = Instant::now() + Duration::from_secs(15);
    loop {
        for p in c.drain_events() {
            if let Output::Response { request, result } = p.output
                && request == id
            {
                return result.map_err(anyhow::Error::msg);
            }
        }
        ensure!(Instant::now() < end, "message deadline");
        std::thread::sleep(Duration::from_millis(5));
    }
}
fn paint(h: &mut ContentHost, gpu: &mut tekes_ui_render::Renderer) -> Result<()> {
    container_mut(h).prepare(gpu)?;
    let f = Rect::new(20., 30., 320., 240.);
    h.render(
        &mut Scene::new(),
        &mut Text(Icons),
        f,
        f,
        Theme::LIGHT.into(),
    )?;
    Ok(())
}
fn ready(h: &mut ContentHost, gpu: &mut tekes_ui_render::Renderer) -> Result<()> {
    let end = Instant::now() + Duration::from_secs(15);
    loop {
        paint(h, gpu)?;
        if let Some(e) = container(h).error() {
            bail!("receiver: {e}")
        };
        if container(h).input_ready()
            && container(h)
                .semantic_parts()
                .iter()
                .any(|p| p.semantics.name == "node1")
        {
            return Ok(());
        }
        ensure!(Instant::now() < end, "readiness deadline");
        std::thread::sleep(Duration::from_millis(5));
    }
}
fn clipped_ready(h: &mut ContentHost, gpu: &mut tekes_ui_render::Renderer) -> Result<()> {
    let end = Instant::now() + Duration::from_secs(15);
    loop {
        container_mut(h).prepare(gpu)?;
        let f = Rect::new(20., 30., 320., 240.);
        let c = Rect::new(20., 100., 320., 50.);
        h.render(
            &mut Scene::new(),
            &mut Text(Icons),
            f,
            c,
            Theme::LIGHT.into(),
        )?;
        if let Some(e) = container(h).error() {
            bail!("clip receiver: {e}")
        }
        let parts = container(h).semantic_parts();
        if container(h).input_ready() && parts.iter().any(|p| p.semantics.name == "node1") {
            return Ok(());
        }
        ensure!(Instant::now() < end, "clip deadline");
        std::thread::sleep(Duration::from_millis(5));
    }
}
fn mounted(module: &str) -> Result<ContentHost> {
    ContentHost::new(Content::new(
        ComponentId(50),
        ProcessContainer::new_view(
            ProcessComponentOptions::new(module, 75).with_view(75, 1, CAPS),
            || {},
        )?,
    ))
    .map_err(Into::into)
}
fn part(h: &ContentHost, name: &str) -> Result<usize> {
    container(h)
        .semantic_parts()
        .iter()
        .find(|p| p.semantics.name == name)
        .map(|p| p.index)
        .context("missing Tree part")
}
fn act(h: &mut ContentHost, part: usize, expanded: bool) -> ContentResponse {
    h.accessibility_action(
        ComponentId(50),
        Some(part),
        AccessibilityAction::SetExpanded(expanded),
    )
}
fn main() -> Result<()> {
    let args = std::env::args().collect::<Vec<_>>();
    let capabilities = Capabilities {
        view: true,
        ..Capabilities::HEADLESS
    };
    let defs = [
        ComponentDefinition {
            id: "tree",
            capabilities,
            create,
        },
        ComponentDefinition {
            id: "orphan",
            capabilities,
            create: orphan,
        },
        ComponentDefinition {
            id: "level",
            capabilities,
            create: level,
        },
    ];
    if run_component_worker_from_args(&defs, &args)? {
        return Ok(());
    }
    let mut gpu = tekes_ui_render::Renderer::new_headless()?;
    let mut h = mounted("tree")?;
    ready(&mut h, &mut gpu)?;
    let original = message(container(&h), "inspect")?;
    ensure!(original["pid"].as_u64() != Some(std::process::id() as u64));
    let branch = part(&h, "node1")?;
    let child = part(&h, "node2")?;
    let grand = part(&h, "node3")?;
    let ps = container(&h).semantic_parts();
    ensure!(ps.iter().any(|p| p.semantics.role == ContentRole::Tree));
    ensure!(
        ps.iter().find(|p| p.index == grand).unwrap().semantics.role
            == ContentRole::TreeItem { level: 2 }
    );
    ensure!(h.semantic_part_parent(ComponentId(50), grand) == Some(child));
    let unchanged = message(container(&h), "inspect")?;
    for _ in 0..3 {
        let _ = container(&h)
            .native_content_bridge(Duration::from_secs(2))?
            .semantics()?;
        let _ = tekes_ui_platform_macos::content_accessibility::content_nodes(&h);
    }
    ensure!(
        message(container(&h), "inspect")? == unchanged,
        "queries changed original owner"
    );
    ensure!(
        h.accessibility_action(ComponentId(50), Some(child), AccessibilityAction::Focus)
            .consumed
    );
    ready(&mut h, &mut gpu)?;
    let focused = message(container(&h), "inspect")?;
    ensure!(
        focused["focused_part"] == "node2",
        "Focus did not reach original child row: {focused}"
    );
    let fresh_child = part(&h, "node2")?;
    ensure!(
        h.accessibility_action(
            ComponentId(50),
            Some(fresh_child),
            AccessibilityAction::Press
        )
        .consumed
    );
    ready(&mut h, &mut gpu)?;
    let pressed = message(container(&h), "inspect")?;
    ensure!(
        pressed["selected"] == 2
            && pressed["parts"]
                .as_array()
                .unwrap()
                .iter()
                .find(|p| p["name"] == "node2")
                .unwrap()["expanded"]
                == true,
        "Press must select/activate without disclosure: {pressed}"
    );
    clipped_ready(&mut h, &mut gpu)?;
    let clipped_parts = container(&h).semantic_parts();
    let clipped_root = clipped_parts
        .iter()
        .find(|p| p.semantics.name == "node1")
        .unwrap();
    ensure!(clipped_root.frame.is_empty());
    let clipped_token = clipped_root.index;
    let clipped_child = part(&h, "node2")?;
    let clipped_grand = part(&h, "node3")?;
    ensure!(
        h.semantic_part_parent(ComponentId(50), clipped_grand) == Some(clipped_child)
            && h.semantic_part_parent(ComponentId(50), clipped_child) == Some(clipped_token)
    );
    let bridge = container(&h).native_content_bridge(Duration::from_secs(2))?;
    let clipped_snapshot = bridge.semantics()?;
    let remote_root = clipped_snapshot
        .reply()
        .semantics
        .as_ref()
        .unwrap()
        .iter()
        .find(|n| n.target.id == ID.0)
        .unwrap();
    let root_token = remote_root
        .parts
        .iter()
        .find(|p| p.semantics.name == "node1")
        .unwrap()
        .index;
    ensure!(
        bridge
            .accessibility_action(
                &clipped_snapshot,
                remote_root.target,
                Some(root_token),
                content::AccessibilityAction::SetExpanded(false)
            )
            .is_err(),
        "direct bridge accepted host-clipped TreeItem"
    );
    for action in [
        AccessibilityAction::Focus,
        AccessibilityAction::Press,
        AccessibilityAction::SetExpanded(false),
    ] {
        ensure!(
            !h.accessibility_action(ComponentId(50), Some(clipped_token), action.clone())
                .consumed,
            "zero-area remote TreeItem accepted {action:?}"
        );
    }
    ready(&mut h, &mut gpu)?;
    let native = tekes_ui_platform_macos::content_accessibility::content_nodes(&h);
    let root = native
        .iter()
        .find(|n| n.label == "node1")
        .context("native root missing")?;
    ensure!(root.role == tekes_ui_platform_macos::AccessibilityRole::TreeItem { level: 0 });
    let key = root.key;
    ensure!(
        tekes_ui_platform_macos::content_accessibility::content_action(
            &mut h,
            key,
            tekes_ui_platform_macos::AccessibilityAction::SetExpanded(false)
        )
        .changed,
        "host platform must route disclosure"
    );
    ready(&mut h, &mut gpu)?;
    let collapsed = message(container(&h), "inspect")?;
    ensure!(
        collapsed["parts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["name"] == "node1")
            .unwrap()["expanded"]
            == false
    );
    let branch_now = part(&h, "node1")?;
    eprintln!(
        "branch old={branch} now={branch_now} parts={:?}",
        container(&h).semantic_parts()
    );
    let expanded = act(&mut h, branch_now, true);
    ensure!(
        expanded.changed,
        "expand failed: {expanded:?}, error={:?}",
        container(&h).error()
    );
    ready(&mut h, &mut gpu)?;
    ensure!(
        part(&h, "node2")? != child,
        "child host token must be revoked on collapse"
    );
    ensure!(
        !act(&mut h, child, false).consumed,
        "old host token accepted"
    );
    // Direct stamped callback: child must reject its own old token, not just host map.
    let bridge = container(&h).native_content_bridge(Duration::from_secs(2))?;
    let snap = bridge.semantics()?;
    let n = snap
        .reply()
        .semantics
        .as_ref()
        .unwrap()
        .iter()
        .find(|n| n.target.id == ID.0)
        .unwrap();
    let target = n.target;
    let token = n
        .parts
        .iter()
        .find(|p| p.semantics.name == "node2")
        .unwrap()
        .index;
    message(container(&h), "remove-restore")?;
    ready(&mut h, &mut gpu)?;
    let stale = container(&h)
        .native_content_bridge(Duration::from_secs(2))?
        .accessibility_action(
            &snap,
            target,
            Some(token),
            content::AccessibilityAction::SetExpanded(false),
        );
    ensure!(stale.is_err(), "child accepted old exposure");
    let bridge = container(&h).native_content_bridge(Duration::from_secs(2))?;
    let snap = bridge.semantics()?;
    let n = snap
        .reply()
        .semantics
        .as_ref()
        .unwrap()
        .iter()
        .find(|n| n.target.id == ID.0)
        .unwrap();
    let target = n.target;
    let token = n.parts[0].index;
    message(container(&h), "remount")?;
    ready(&mut h, &mut gpu)?;
    ensure!(
        container(&h)
            .native_content_bridge(Duration::from_secs(2))?
            .accessibility_action(
                &snap,
                target,
                Some(token),
                content::AccessibilityAction::SetExpanded(false)
            )
            .is_err(),
        "child accepted old mount"
    );
    let current = part(&h, "node1")?;
    message(container(&h), "inactive")?;
    ensure!(
        !act(&mut h, current, false).consumed,
        "inactive child accepted disclosure"
    );
    h.set_window_active(false);
    ensure!(!act(&mut h, current, false).consumed);
    h.set_window_active(true);
    // Each malformed snapshot really crosses a child transport into ProcessContainer.
    let mut rejected = vec![];
    for mode in ["orphan", "level"] {
        let mut bad = mounted(mode)?;
        let end = Instant::now() + Duration::from_secs(15);
        loop {
            let r = paint(&mut bad, &mut gpu);
            if let Some(e) = container(&bad).error() {
                rejected.push(json!({"mode":mode,"error":e}));
                break;
            }
            if let Err(e) = r {
                rejected.push(json!({"mode":mode,"error":e.to_string()}));
                break;
            }
            ensure!(Instant::now() < end, "malformed snapshot accepted");
            std::thread::sleep(Duration::from_millis(5));
        }
        ensure!(container(&bad).semantic_parts().is_empty());
    }
    println!(
        "{}",
        json!({"result":"TREE_PROCESS_PASS","parent_pid":std::process::id(),"original":original,"collapsed":collapsed,"real_malformed_receivers":rejected,"old_mount_token_inactive_rejected":true,"Focus_Press_readback_and_readonly_queries":true,"host_zero_area_parent_chain_and_mutation_reject":true,"host_platform_processcontainer_child_original_owner":true,"GPU_visual_acceptance":false})
    );
    Ok(())
}
