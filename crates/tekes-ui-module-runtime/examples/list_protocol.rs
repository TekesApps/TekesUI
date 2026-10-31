//! Issue #96: actual original List owner and receiver.
use anyhow::{Context, Result, bail, ensure};
use serde_json::{Value, json};
use std::time::{Duration, Instant};
use tekes_ui_app_protocol::Request;
use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, composition::*, extended::*, layout_view::IconRenderer,
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
fn snapshot() -> KeyedListSnapshot {
    KeyedListSnapshot {
        dataset: ListDatasetKey(1),
        items: (1..=3)
            .map(|k| KeyedListItem {
                key: ListItemKey(k),
                name: format!("row{k}"),
            })
            .collect(),
        display_items: (1..=3).map(ListItemKey).collect(),
    }
}
fn rows(s: &KeyedListSnapshot) -> Vec<ListRowDisplay> {
    s.items
        .iter()
        .map(|r| ListRowDisplay {
            key: r.key,
            row_id: ComponentId(100 + r.key.0),
            display: ListDisplayUpdate::Keep,
        })
        .collect()
}
fn budget() -> ListBudget {
    ListBudget {
        max_items: 100,
        max_display_items: 100,
        max_utf8_bytes: 1000000,
        max_staging_units: 100000,
    }
}
fn list(s: &KeyedListSnapshot) -> KeyedSelectableListContent {
    SelectableListContent::keyed("L", s, &mut rows(s), budget()).unwrap()
}

fn list_content() -> Content {
    Content::new(ID, list(&snapshot()))
}
struct Child {
    host: ContentHost,
    text: Text,
    revision: u64,
}
fn create() -> Box<dyn ProcessModule> {
    Box::new(Child {
        host: ContentHost::new(Content::new(
            ComponentId(1),
            ListContent::new("Child", vec![list_content()]),
        ))
        .unwrap(),
        text: Text(Icons),
        revision: 0,
    })
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
                let c = self
                    .host
                    .get(ID)
                    .unwrap()
                    .component::<KeyedSelectableListContent>()
                    .unwrap();
                let n = self.host.semantics().into_iter().find(|n| n.id == ID);
                return Ok(
                    json!({"pid":std::process::id(),"mount":self.host.semantic_mount(ID),"focused_key":c.focused_key().map(|k|k.0),"selected":c.selected_key().map(|k|k.0),"parts":n.map(|n|n.parts.iter().map(|p|json!({"index":p.index,"name":p.semantics.name,"focused":p.focused})).collect::<Vec<_>>())}),
                );
            }
            "remount" => {
                self.host.remove(ID)?;
                self.host.insert(ComponentId(1), list_content())?;
            }
            "inactive" => self.host.set_window_active(false),
            "active" => {
                self.host.set_window_active(true);
                self.host
                    .get_mut(ID)
                    .unwrap()
                    .component_mut::<KeyedSelectableListContent>()
                    .unwrap()
                    .prepare_exposure()
                    .unwrap();
            }
            "hide-restore" => {
                let full = snapshot();
                let mut hidden = full.clone();
                hidden.display_items.clear();
                for s in [&hidden, &full] {
                    self.host
                        .replace_keyed_list(ID, s, &mut rows(s), None)
                        .unwrap();
                }
            }
            "reorder" => {
                let mut s = snapshot();
                s.display_items.reverse();
                s.items.reverse();
                self.host
                    .replace_keyed_list(ID, &s, &mut rows(&s), None)
                    .unwrap();
            }
            _ => bail!("unknown"),
        };
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
            match p.output {
                Output::Response { request, result } if request == id => {
                    return result.map_err(anyhow::Error::msg);
                }
                _ => {}
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
                .any(|p| p.semantics.name == "row1")
        {
            return Ok(());
        }
        ensure!(Instant::now() < end, "readiness deadline");
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
        .context("missing List part")
}

fn stamped(h: &ContentHost) -> Result<(ProcessContentSnapshot, content::Target, usize)> {
    let s = container(h)
        .native_content_bridge(Duration::from_secs(2))?
        .semantics()?;
    let n = s
        .reply()
        .semantics
        .as_ref()
        .unwrap()
        .iter()
        .find(|n| n.target.id == ID.0)
        .context("List owner missing")?;
    ensure!(n.semantics.role == content::SemanticRole::List);
    let p = n
        .parts
        .iter()
        .find(|p| p.semantics.name == "row2")
        .context("row2 missing")?;
    ensure!(p.semantics.role == content::SemanticRole::ListItem);
    let t = n.target;
    let i = p.index;
    Ok((s, t, i))
}
fn rejected(
    h: &ContentHost,
    s: &ProcessContentSnapshot,
    t: content::Target,
    p: usize,
) -> Result<String> {
    match container(h)
        .native_content_bridge(Duration::from_secs(2))?
        .accessibility_action(s, t, Some(p), content::AccessibilityAction::Press)
    {
        Err(e) => Ok(e.to_string()),
        Ok(_) => bail!("stale/wrong target accepted"),
    }
}
fn raw_rejected(
    h: &ContentHost,
    s: &ProcessContentSnapshot,
    t: content::Target,
    p: usize,
) -> Result<String> {
    match container(h)
        .native_content_bridge(Duration::from_secs(2))?
        .request(
            s,
            content::Input::Accessibility {
                target: t,
                part: Some(p),
                binding: None,
                action: content::AccessibilityAction::Press,
            },
        ) {
        Err(e) => Ok(e.to_string()),
        Ok(_) => bail!("raw owner accepted wrong target"),
    }
}
fn main() -> Result<()> {
    let args = std::env::args().collect::<Vec<_>>();
    let defs = [ComponentDefinition {
        id: "list",
        capabilities: Capabilities {
            view: true,
            ..Capabilities::HEADLESS
        },
        create,
    }];
    if run_component_worker_from_args(&defs, &args)? {
        return Ok(());
    }
    let mut gpu = tekes_ui_render::Renderer::new_headless()?;
    let mut h = mounted("list")?;
    ready(&mut h, &mut gpu)?;
    let before = message(container(&h), "inspect")?;
    ensure!(before["pid"].as_u64() != Some(std::process::id() as u64));
    let (s, t, p) = stamped(&h)?;
    ensure!(
        before["mount"] == t.mount
            && before["parts"]
                .as_array()
                .unwrap()
                .iter()
                .any(|v| v["name"] == "row2" && v["index"] == p),
        "mount/part rewritten"
    );
    std::fs::write(
        std::env::var_os("LIST_PROTOCOL_REPLY").context("reply output")?,
        serde_json::to_vec_pretty(s.reply())?,
    )?;
    let native = tekes_ui_platform_macos::content_accessibility::content_nodes(&h);
    let _snapshot =
        tekes_ui_platform_macos::content_accessibility::ContentAccessibilitySnapshot::from_host(&h)
            .context("whole accessibility snapshot rejected")?;
    ensure!(tekes_ui_platform_macos::accept_accessibility_nodes(native.clone()) == native);
    ensure!(
        native
            .iter()
            .filter(|n| n.role == tekes_ui_platform_macos::AccessibilityRole::ListItem)
            .count()
            == 3
    );
    for _ in 0..3 {
        let _ = stamped(&h)?;
        let _ = tekes_ui_platform_macos::content_accessibility::content_nodes(&h);
    }
    ensure!(
        message(container(&h), "inspect")? == before,
        "queries changed owner"
    );
    // Direct bridge uses the exact child mount and part; inspect reads the original owner.
    container(&h)
        .native_content_bridge(Duration::from_secs(2))?
        .accessibility_action(&s, t, Some(p), content::AccessibilityAction::Focus)?;
    ready(&mut h, &mut gpu)?;
    let focus = message(container(&h), "inspect")?;
    ensure!(
        focus["focused_key"] == 2 && focus["selected"].is_null(),
        "Focus owner readback {focus}"
    );
    let (s, t, p) = stamped(&h)?;
    container(&h)
        .native_content_bridge(Duration::from_secs(2))?
        .accessibility_action(&s, t, Some(p), content::AccessibilityAction::Press)?;
    ready(&mut h, &mut gpu)?;
    ensure!(message(container(&h), "inspect")?["selected"] == 2);
    // Host projected callback runs the same owner route.
    let row = part(&h, "row3")?;
    ensure!(
        h.accessibility_action(ComponentId(50), Some(row), AccessibilityAction::Focus)
            .consumed
    );
    ready(&mut h, &mut gpu)?;
    ensure!(message(container(&h), "inspect")?["focused_key"] == 3);
    let row = part(&h, "row3")?;
    ensure!(
        h.accessibility_action(ComponentId(50), Some(row), AccessibilityAction::Press)
            .consumed
    );
    ready(&mut h, &mut gpu)?;
    ensure!(message(container(&h), "inspect")?["selected"] == 3);
    let (s, t, p) = stamped(&h)?;
    let mut wrong = t;
    wrong.id = 999;
    let mut badmount = t;
    badmount.mount += 1;
    let wrong_target = rejected(&h, &s, wrong, p)?;
    let wrong_mount = rejected(&h, &s, badmount, p)?;
    let wrong_part = rejected(&h, &s, t, usize::MAX)?;
    let raw_wrong_target = raw_rejected(&h, &s, wrong, p)?;
    let raw_wrong_mount = raw_rejected(&h, &s, badmount, p)?;
    let raw_wrong_part = raw_rejected(&h, &s, t, usize::MAX)?;
    let mut stale = vec![];
    for topic in ["hide-restore", "remount"] {
        let (s, t, p) = stamped(&h)?;
        message(container(&h), topic)?;
        ready(&mut h, &mut gpu)?;
        stale.push(json!({"case":topic,"error":rejected(&h,&s,t,p)?}));
    }
    let (s, t, p) = stamped(&h)?;
    message(container(&h), "inactive")?;
    let inactive = rejected(&h, &s, t, p)?;
    message(container(&h), "active")?;
    ready(&mut h, &mut gpu)?;
    let (ns, nt, np) = stamped(&h)?;
    ensure!(nt == t, "root mount changed across activity");
    ensure!(np > p, "activity reused token");
    let restored = rejected(&h, &s, t, p)?;
    let activity_evidence = json!({"old_target":t,"new_target":nt,"old_part":p,"new_part":np,"root_mount_preserved":t==nt,"fresh_monotonic_part":np>p});
    container(&h)
        .native_content_bridge(Duration::from_secs(2))?
        .accessibility_action(&ns, nt, Some(np), content::AccessibilityAction::Press)?;
    ready(&mut h, &mut gpu)?;
    ensure!(message(container(&h), "inspect")?["selected"] == 2);
    let (_s, t, p) = stamped(&h)?;
    message(container(&h), "reorder")?;
    ready(&mut h, &mut gpu)?;
    let (_, nt, np) = stamped(&h)?;
    ensure!(nt == t && np == p, "reorder changed identity");
    let final_state = message(container(&h), "inspect")?;
    ensure!(final_state["selected"] == 2);
    println!(
        "{}",
        json!({"result":"LIST_PROCESS_PASS","before":before,"focus":focus,"final":final_state,"target":t,"oldpart":p,"activity":activity_evidence,"raw_wrong_target":raw_wrong_target,"raw_wrong_mount":raw_wrong_mount,"raw_wrong_part":raw_wrong_part,"stale":stale,"wrong_target":wrong_target,"wrong_mount":wrong_mount,"wrong_part":wrong_part,"inactive":inactive,"restored_old_target_rejection":restored,"native_projection":true})
    );
    Ok(())
}
