//! Issue #86 public consumer: actual child DataTableContent owner and host projection.
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

fn snapshot() -> KeyedTableSnapshot {
    KeyedTableSnapshot {
        dataset: TableDatasetKey(1),
        columns: (1..=2)
            .map(|k| KeyedTableColumn {
                key: TableColumnKey(k),
                name: format!("column{k}"),
            })
            .collect(),
        rows: (1..=3)
            .map(|k| KeyedTableRow {
                key: TableRowKey(k),
                name: format!("row{k}"),
                cells: vec![format!("value{k}"), "cell".into()],
            })
            .collect(),
        display_rows: (1..=3).map(TableRowKey).collect(),
    }
}
fn displays(s: &KeyedTableSnapshot) -> TableDisplays {
    TableDisplays {
        headers: s
            .columns
            .iter()
            .map(|c| TableHeaderDisplay {
                key: c.key,
                display: TableDisplayUpdate::Keep,
            })
            .collect(),
        rows: s
            .rows
            .iter()
            .map(|r| TableRowDisplay {
                key: r.key,
                id: ComponentId(100 + r.key.0),
                replace_wrapper: false,
                cells: s
                    .columns
                    .iter()
                    .map(|c| TableCellDisplay {
                        column: c.key,
                        display: TableDisplayUpdate::Keep,
                        accessory: TableDisplayUpdate::Keep,
                    })
                    .collect(),
            })
            .collect(),
    }
}
fn table() -> Content {
    let s = snapshot();
    let mut d = displays(&s);
    Content::new(
        ID,
        DataTableContent::new(
            "Table",
            &s,
            &mut d,
            TableBudget {
                max_units: 100_000,
                max_utf8_bytes: 1_000_000,
            },
        )
        .unwrap(),
    )
}
fn empty_table() -> Content {
    let snapshot = KeyedTableSnapshot {
        dataset: TableDatasetKey(2),
        columns: vec![],
        rows: vec![],
        display_rows: vec![],
    };
    let mut displays = TableDisplays::default();
    Content::new(
        ID,
        DataTableContent::new(
            "Empty table",
            &snapshot,
            &mut displays,
            TableBudget {
                max_units: 1000,
                max_utf8_bytes: 10_000,
            },
        )
        .unwrap(),
    )
}
#[derive(Debug)]
struct Malformed(u8);
impl Component for Malformed {
    fn measure(
        &mut self,
        _: &mut dyn ButtonTextRenderer,
        a: Size,
    ) -> tekes_ui_controls::layout::MeasuredChild {
        tekes_ui_controls::layout::MeasuredChild::new(a)
    }
    fn semantics(&self) -> ContentSemantics {
        ContentSemantics::new(
            ContentRole::Table,
            if self.0 == 5 {
                "x".repeat(1_048_576)
            } else {
                "malformed".into()
            },
        )
    }
    fn semantic_parts(&self) -> Vec<SemanticPart> {
        let p = |i, role| SemanticPart {
            index: i,
            frame: Rect::new(0., 0., 60., 30.),
            focused: false,
            semantics: ContentSemantics::new(role, "part"),
        };
        let mut v = vec![
            p(1, ContentRole::TableColumn { index: 0 }),
            p(
                2,
                ContentRole::TableColumnHeader {
                    index: 0,
                    sort: None,
                },
            ),
            p(3, ContentRole::TableRow { index: 0 }),
            p(4, ContentRole::TableCell { row: 0, column: 0 }),
        ];
        match self.0 {
            1 => v[0].semantics.role = ContentRole::TableColumn { index: usize::MAX },
            2 => (),
            3 => v[3].index = 3,
            4 => v[3].semantics.role = ContentRole::TableColumn { index: 0 },
            _ => (),
        };
        v
    }
    fn semantic_part_parent(&self, i: usize) -> Option<usize> {
        match i {
            2 => Some(if self.0 == 2 { 999 } else { 1 }),
            4 => Some(3),
            _ => None,
        }
    }
}
struct Child {
    host: ContentHost,
    text: Text,
    revision: u64,
}
fn create_mode(mode: u8) -> Box<dyn ProcessModule> {
    Box::new(Child {
        host: ContentHost::new(Content::new(
            ComponentId(1),
            ListContent::new(
                "Child",
                vec![if mode == 0 {
                    table()
                } else if mode == 6 {
                    empty_table()
                } else {
                    Content::new(ID, Malformed(mode))
                }],
            ),
        ))
        .unwrap(),
        text: Text(Icons),
        revision: 0,
    })
}
fn create() -> Box<dyn ProcessModule> {
    create_mode(0)
}
fn ordinal() -> Box<dyn ProcessModule> {
    create_mode(1)
}
fn orphan() -> Box<dyn ProcessModule> {
    create_mode(2)
}
fn duplicate() -> Box<dyn ProcessModule> {
    create_mode(3)
}
fn kind() -> Box<dyn ProcessModule> {
    create_mode(4)
}
fn budget() -> Box<dyn ProcessModule> {
    create_mode(5)
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
    fn started(&mut self, _: &ComponentEmitter) -> Result<()> {
        self.host.set_action_events(true);
        Ok(())
    }
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
                let t = self
                    .host
                    .get(ID)
                    .and_then(|n| n.component::<DataTableContent>())
                    .unwrap();
                let n = self.host.semantics().into_iter().find(|n| n.id == ID);
                return Ok(
                    json!({"pid":std::process::id(),"selected":t.selected_key().map(|k|k.0),"sort":t.sort_key_state().map(|(k,d)|json!([k.0,format!("{d:?}")])),"focused":n.as_ref().and_then(|n|n.parts.iter().find(|p|p.focused)).map(|p|p.semantics.name.clone()),"host_debug":format!("{:?}",self.host)}),
                );
            }
            "drain-actions" => {
                return Ok(json!({"records":self.host.drain_action_records().len()}));
            }
            "restore-order" => {
                let s = snapshot();
                self.host
                    .replace_table_snapshot(ID, &s, &mut displays(&s), None)
                    .unwrap();
            }
            "remount" => {
                self.host.remove(ID)?;
                self.host.insert(ComponentId(1), table())?;
            }
            "inactive" => self.host.set_window_active(false),
            "active" => self.host.set_window_active(true),
            "remove-restore" => {
                let full = snapshot();
                let mut hidden = full.clone();
                hidden.display_rows.clear();
                for s in [&hidden, &full] {
                    let mut d = displays(s);
                    self.host
                        .replace_table_snapshot(ID, s, &mut d, None)
                        .unwrap();
                }
            }
            "reorder" => {
                let mut s = snapshot();
                s.display_rows.reverse();
                s.columns.reverse();
                for r in &mut s.rows {
                    r.cells.reverse();
                }
                let mut d = displays(&s);
                self.host
                    .replace_table_snapshot(ID, &s, &mut d, None)
                    .unwrap();
            }
            _ => bail!("unknown message"),
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
                .any(|p| p.semantics.name == "row1")
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
        if container(h).input_ready() && parts.iter().any(|p| p.semantics.name == "row1") {
            return Ok(());
        }
        ensure!(Instant::now() < end, "clip deadline");
        std::thread::sleep(Duration::from_millis(5));
    }
}
fn left_ready(h: &mut ContentHost, gpu: &mut tekes_ui_render::Renderer) -> Result<()> {
    let end = Instant::now() + Duration::from_secs(15);
    loop {
        container_mut(h).prepare(gpu)?;
        let f = Rect::new(20., 30., 320., 240.);
        let c = Rect::new(20., 30., 120., 240.);
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
        if container(h).input_ready() && parts.iter().any(|p| p.semantics.name == "row1") {
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
        .context("missing Table part")
}

fn debug_field<'a>(debug: &'a str, name: &str) -> Option<&'a str> {
    let marker = format!("{name}: ");
    let value = &debug[debug.find(&marker)? + marker.len()..];
    let (mut parens, mut braces, mut brackets) = (0isize, 0isize, 0isize);
    let (mut quoted, mut escaped) = (false, false);
    for (offset, ch) in value.char_indices() {
        if quoted {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                quoted = false;
            }
            continue;
        }
        match ch {
            '"' => quoted = true,
            '(' => parens += 1,
            ')' => parens -= 1,
            '{' => braces += 1,
            '}' => braces -= 1,
            '[' => brackets += 1,
            ']' => brackets -= 1,
            ',' if parens == 0 && braces == 0 && brackets == 0 => {
                return Some(value[..offset].trim());
            }
            _ => (),
        }
    }
    Some(value.trim_end_matches('}').trim())
}
fn owner_business_state(value: &Value) -> Result<Value> {
    let debug = value["host_debug"]
        .as_str()
        .context("inspect omitted host debug")?;
    let actions = debug_field(debug, "actions").context("inspect omitted action queue")?;
    Ok(json!({
        "pid": value.get("pid"),
        "focused": value.get("focused"),
        "selected": value.get("selected"),
        "sort": value.get("sort"),
        "actions": actions,
    }))
}

fn named_header(h: &ContentHost) -> usize {
    container(h)
        .semantic_parts()
        .iter()
        .find(|p| {
            p.semantics.name == "column1"
                && matches!(p.semantics.role, ContentRole::TableColumnHeader { .. })
        })
        .unwrap()
        .index
}
fn by_role(h: &ContentHost, role: ContentRole) -> usize {
    container(h)
        .semantic_parts()
        .into_iter()
        .find(|p| p.semantics.role == role)
        .unwrap()
        .index
}
fn stamped(h: &ContentHost) -> Result<(ProcessContentSnapshot, content::Target, usize)> {
    let b = container(h).native_content_bridge(Duration::from_secs(2))?;
    let snap = b.semantics()?;
    let n = snap
        .reply()
        .semantics
        .as_ref()
        .unwrap()
        .iter()
        .find(|n| n.target.id == ID.0)
        .unwrap();
    let token = n
        .parts
        .iter()
        .find(|p| {
            matches!(
                p.semantics.role,
                content::SemanticRole::TableRow { index: 0 }
            )
        })
        .unwrap()
        .index;
    let t = n.target;
    Ok((snap, t, token))
}
fn stale_rejected(
    h: &ContentHost,
    s: &ProcessContentSnapshot,
    t: content::Target,
    p: usize,
) -> Result<bool> {
    Ok(container(h)
        .native_content_bridge(Duration::from_secs(2))?
        .accessibility_action(s, t, Some(p), content::AccessibilityAction::Press)
        .is_err())
}
fn original_acceptance_main() -> Result<()> {
    let args = std::env::args().collect::<Vec<_>>();
    let caps = Capabilities {
        view: true,
        ..Capabilities::HEADLESS
    };
    let defs = [
        ComponentDefinition {
            id: "table",
            capabilities: caps,
            create,
        },
        ComponentDefinition {
            id: "ordinal",
            capabilities: caps,
            create: ordinal,
        },
        ComponentDefinition {
            id: "orphan",
            capabilities: caps,
            create: orphan,
        },
        ComponentDefinition {
            id: "duplicate",
            capabilities: caps,
            create: duplicate,
        },
        ComponentDefinition {
            id: "kind",
            capabilities: caps,
            create: kind,
        },
        ComponentDefinition {
            id: "budget",
            capabilities: caps,
            create: budget,
        },
    ];
    if run_component_worker_from_args(&defs, &args)? {
        return Ok(());
    }
    let mut gpu = tekes_ui_render::Renderer::new_headless()?;
    let mut h = mounted("table")?;
    ready(&mut h, &mut gpu)?;
    let before = message(container(&h), "inspect")?;
    ensure!(before["pid"].as_u64() != Some(std::process::id() as u64));
    let snap = container(&h)
        .native_content_bridge(Duration::from_secs(2))?
        .semantics()?;
    std::fs::write(
        std::env::var_os("TABLE_PROTOCOL_REPLY").context("reply path")?,
        serde_json::to_vec_pretty(snap.reply())?,
    )?;
    let native = tekes_ui_platform_macos::content_accessibility::content_nodes(&h);
    tekes_ui_platform_macos::content_accessibility::ContentAccessibilitySnapshot::from_host(&h)
        .context("unclipped Table accessibility snapshot rejected nodes")?;
    ensure!(
        tekes_ui_platform_macos::accept_accessibility_nodes(native.clone()) == native,
        "unclipped Table accessibility packet was rejected"
    );
    ensure!(
        native
            .iter()
            .filter(|n| matches!(
                n.role,
                tekes_ui_platform_macos::AccessibilityRole::TableCell { .. }
            ))
            .count()
            == 6
    );
    for _ in 0..3 {
        let _ = container(&h)
            .native_content_bridge(Duration::from_secs(2))?
            .semantics()?;
        let _ = tekes_ui_platform_macos::content_accessibility::content_nodes(&h);
    }
    ensure!(
        message(container(&h), "inspect")? == before,
        "queries mutated child"
    );
    for role in [
        ContentRole::TableColumnHeader {
            index: 0,
            sort: None,
        },
        ContentRole::TableRow { index: 1 },
    ] {
        let i = by_role(&h, role);
        ensure!(
            h.accessibility_action(ComponentId(50), Some(i), AccessibilityAction::Focus)
                .consumed
        );
        ready(&mut h, &mut gpu)?;
        let focused = message(container(&h), "inspect")?;
        ensure!(
            focused["focused"]
                == if matches!(role, ContentRole::TableRow { .. }) {
                    "row2"
                } else {
                    "column1"
                }
        );
        let i = if matches!(role, ContentRole::TableRow { .. }) {
            part(&h, "row2")?
        } else {
            named_header(&h)
        };
        ensure!(
            h.accessibility_action(ComponentId(50), Some(i), AccessibilityAction::Press)
                .consumed
        );
        ready(&mut h, &mut gpu)?;
    }
    let state = message(container(&h), "inspect")?;
    ensure!(
        state["selected"] == 2 && state["sort"] == json!([1, "Ascending"]),
        "owner readback {state}"
    );
    // Native platform callback goes through the real host map and child sort owner.
    let native = tekes_ui_platform_macos::content_accessibility::content_nodes(&h);
    let key = native
        .iter()
        .find(|n| {
            matches!(
                n.role,
                tekes_ui_platform_macos::AccessibilityRole::TableColumnHeader { index: 0, .. }
            )
        })
        .unwrap()
        .key;
    ensure!(
        tekes_ui_platform_macos::content_accessibility::content_action(
            &mut h,
            key,
            tekes_ui_platform_macos::AccessibilityAction::Press
        )
        .consumed
    );
    ready(&mut h, &mut gpu)?;
    ensure!(message(container(&h), "inspect")?["sort"] == json!([1, "Descending"]));
    let row = part(&h, "row2")?;
    let header = named_header(&h);
    left_ready(&mut h, &mut gpu)?;
    ensure!(
        part(&h, "row2")? == row && named_header(&h) == header,
        "parent clip changed stable part tokens"
    );
    let old_bridge = container(&h).native_content_bridge(Duration::from_secs(2))?;
    let retained = old_bridge.semantics()?;
    let retained_node = retained
        .reply()
        .semantics
        .as_ref()
        .unwrap()
        .iter()
        .find(|n| n.target.id == ID.0)
        .unwrap();
    let retained_target = retained_node.target;
    let retained_header = retained_node
        .parts
        .iter()
        .find(|p| {
            matches!(
                p.semantics.role,
                content::SemanticRole::TableColumnHeader { index: 0, .. }
            )
        })
        .unwrap()
        .index;
    message(container(&h), "reorder")?;
    left_ready(&mut h, &mut gpu)?;
    let stale_geometry = container(&h)
        .native_content_bridge(Duration::from_secs(2))?
        .accessibility_action(
            &retained,
            retained_target,
            Some(retained_header),
            content::AccessibilityAction::Press,
        );
    eprintln!(
        "{}",
        json!({"retained_geometry_rejected":stale_geometry.is_err(),"reply_consumed":stale_geometry.as_ref().ok().map(|s|s.reply().consumed),"reply_changed":stale_geometry.as_ref().ok().map(|s|s.reply().changed),"original_child_after_retained_press":message(container(&h),"inspect")?})
    );
    ensure!(
        stale_geometry.is_err(),
        "direct retained Table geometry accepted after stable header moved outside parent clip"
    );
    ready(&mut h, &mut gpu)?;
    ensure!(part(&h, "row2")? == row && named_header(&h) == header);
    ensure!(
        container(&h)
            .semantic_parts()
            .iter()
            .any(|p| p.index == header
                && matches!(
                    p.semantics.role,
                    ContentRole::TableColumnHeader { index: 1, .. }
                ))
    );
    let untouched = message(container(&h), "inspect")?;
    for role in [
        ContentRole::TableColumn { index: 0 },
        ContentRole::TableCell { row: 0, column: 0 },
    ] {
        let i = by_role(&h, role);
        for a in [
            AccessibilityAction::Focus,
            AccessibilityAction::Press,
            AccessibilityAction::SetValue("bad".into()),
            AccessibilityAction::SetExpanded(true),
        ] {
            ensure!(!h.accessibility_action(ComponentId(50), Some(i), a).consumed);
        }
    }
    ensure!(message(container(&h), "inspect")? == untouched);
    clipped_ready(&mut h, &mut gpu)?;
    let ns = tekes_ui_platform_macos::content_accessibility::content_nodes(&h);
    let clipped_snapshot =
        tekes_ui_platform_macos::content_accessibility::ContentAccessibilitySnapshot::from_host(&h)
            .context("clipped Table accessibility snapshot rejected nodes")?;
    ensure!(
        clipped_snapshot.is_valid(),
        "clipped Table accessibility snapshot is not current or valid"
    );
    let raw_keys = ns
        .iter()
        .map(|node| node.key)
        .collect::<std::collections::BTreeSet<_>>();
    let snapshot_nodes = clipped_snapshot.nodes();
    let snapshot_keys = snapshot_nodes
        .iter()
        .map(|node| node.key)
        .collect::<std::collections::BTreeSet<_>>();
    ensure!(!ns.is_empty() && raw_keys == snapshot_keys);
    for raw in &ns {
        let projected = snapshot_nodes
            .iter()
            .find(|node| node.key == raw.key)
            .context("clipped snapshot is missing a raw Table node")?;
        ensure!(raw.parent == projected.parent && raw.role == projected.role);
    }
    let raw_header = ns
        .iter()
        .find(|node| {
            node.key == key
                && node.key.part == Some(header)
                && matches!(
                    node.role,
                    tekes_ui_platform_macos::AccessibilityRole::TableColumnHeader { index: 1, .. }
                )
        })
        .context("missing original clipped raw header")?;
    ensure!(raw_header.frame.size.width == 0.0 || raw_header.frame.size.height == 0.0);
    let snapshot_header = snapshot_nodes
        .iter()
        .find(|node| node.key == raw_header.key)
        .context("clipped snapshot is missing the original header key")?;
    ensure!(
        matches!(
            snapshot_header.role,
            tekes_ui_platform_macos::AccessibilityRole::TableColumnHeader { index: 1, .. }
        ) && snapshot_header.frame.origin.x == 0.0
            && snapshot_header.frame.origin.y == 0.0
            && snapshot_header.frame.size.width == 0.0
            && snapshot_header.frame.size.height == 0.0
            && !snapshot_header.focused
    );
    ensure!(
        tekes_ui_platform_macos::accept_accessibility_nodes(ns.clone()).is_empty(),
        "uncertified clipped Table raw packet was accepted"
    );
    let ps = container(&h).semantic_parts();
    let clipped_part = ps
        .iter()
        .find(|p| {
            p.index == header && matches!(p.semantics.role, ContentRole::TableColumnHeader { .. })
        })
        .context("missing original focused header")?;
    ensure!(clipped_part.frame.is_empty() && !clipped_part.focused);
    let clipped = clipped_part.index;
    let owner_before_clipped_actions = message(container(&h), "inspect")?;
    for action in [AccessibilityAction::Focus, AccessibilityAction::Press] {
        ensure!(
            !h.accessibility_action(ComponentId(50), Some(clipped), action)
                .consumed,
            "clipped Table header action was consumed"
        );
    }
    let owner_after_clipped_actions = message(container(&h), "inspect")?;
    ensure!(
        owner_after_clipped_actions == owner_before_clipped_actions,
        "rejected clipped actions changed full owner state: before={owner_before_clipped_actions:#?}; after={owner_after_clipped_actions:#?}"
    );
    let b = container(&h).native_content_bridge(Duration::from_secs(2))?;
    let ss = b.semantics()?;
    let n = ss
        .reply()
        .semantics
        .as_ref()
        .unwrap()
        .iter()
        .find(|n| n.target.id == ID.0)
        .unwrap();
    let p = n
        .parts
        .iter()
        .find(|p| {
            matches!(
                p.semantics.role,
                content::SemanticRole::TableColumnHeader { .. }
            )
        })
        .unwrap();
    ensure!(
        b.accessibility_action(
            &ss,
            n.target,
            Some(p.index),
            content::AccessibilityAction::Press
        )
        .is_err()
    );
    ready(&mut h, &mut gpu)?;
    let restored_header = container(&h)
        .semantic_parts()
        .into_iter()
        .find(|p| p.index == header)
        .context("stable focused header token missing after unclipping")?;
    ensure!(!restored_header.frame.is_empty() && restored_header.focused);
    let owner_after_restore = message(container(&h), "inspect")?;
    let business_before_restore = owner_business_state(&owner_before_clipped_actions)?;
    let business_after_restore = owner_business_state(&owner_after_restore)?;
    ensure!(
        business_after_restore == business_before_restore,
        "layout restore changed Table owner business state (geometry/reveal debug is intentionally excluded): before={business_before_restore:#?}; after={business_after_restore:#?}; full_before={owner_before_clipped_actions:#?}; full_after={owner_after_restore:#?}"
    );
    for topic in ["remove-restore", "remount"] {
        let (s, t, p) = stamped(&h)?;
        message(container(&h), topic)?;
        ready(&mut h, &mut gpu)?;
        ensure!(stale_rejected(&h, &s, t, p)?, "stale {topic} accepted");
    }
    let (s, t, p) = stamped(&h)?;
    message(container(&h), "inactive")?;
    ensure!(stale_rejected(&h, &s, t, p)?);
    h.set_window_active(false);
    ensure!(
        !h.accessibility_action(ComponentId(50), Some(row), AccessibilityAction::Press)
            .consumed
    );
    let mut rejected = vec![];
    for mode in ["ordinal", "orphan", "duplicate", "kind", "budget"] {
        let mut bad = mounted(mode)?;
        let end = Instant::now() + Duration::from_secs(15);
        loop {
            let result = paint(&mut bad, &mut gpu);
            if let Some(e) = container(&bad).error() {
                rejected.push(json!({"mode":mode,"error":e}));
                break;
            }
            if let Err(e) = result {
                rejected.push(json!({"mode":mode,"error":e.to_string()}));
                break;
            }
            ensure!(Instant::now() < end, "malformed accepted {mode}");
            std::thread::sleep(Duration::from_millis(5));
        }
        ensure!(container(&bad).semantic_parts().is_empty());
    }
    println!(
        "{}",
        json!({"result":"TABLE_PROCESS_PASS","before":before,"owner_readback":state,"real_malformed_receivers":rejected,"native_projection":true,"old_mount_exposure_inactive_rejected":true})
    );
    Ok(())
}

fn main() -> Result<()> {
    original_acceptance_main()
}
