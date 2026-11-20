//! Original collection stories, hosted by the receiving ContentHost.
use super::{
    StoryState,
    apphost_basic::{FixtureText, SPECIMEN},
};
use tekes_ui_controls::{
    BarChart, ButtonTextAlign, ButtonTextRenderer, ChartDatum,
    collections_view::{ChartView, DataTableView, ListView, TreeView},
    composition::*,
    extended as controls,
    layout::MeasuredChild,
};
use tekes_ui_scene::{Rect, Scene, Size};
use tekes_ui_tokens::{Palette, Rgba, Theme, proposal::TextRole};

pub(super) fn supports(renderer: &str) -> bool {
    matches!(renderer, "list" | "tree" | "data-table" | "chart")
}
fn text(id: u64, value: impl Into<String>) -> Content {
    Content::new(ComponentId(id), TextContent::new(value))
}

// The original visual matrices use the production Views with frozen local models.
// Measuring borrows the window's metrics and never creates a second text owner.
struct Metrics<'a>(&'a mut dyn ButtonTextRenderer);
impl ButtonTextRenderer for Metrics<'_> {
    fn measure(&mut self, s: &str, r: TextRole, w: f32) -> Size {
        self.0.measure(s, r, w)
    }
    fn resolved_font_size(&mut self, r: TextRole) -> f32 {
        self.0.resolved_font_size(r)
    }
    fn backing_scale(&self) -> f32 {
        self.0.backing_scale()
    }
    fn first_line_height(&mut self, s: &str, r: TextRole, w: f32) -> f32 {
        self.0.first_line_height(s, r, w)
    }
    fn draw(&mut self, _: &mut Scene, _: &str, _: TextRole, _: Rect, _: ButtonTextAlign, _: Rgba) {}
}
#[derive(Debug)]
struct Matrix(&'static str);
impl Matrix {
    fn render(
        &self,
        text: &mut dyn ButtonTextRenderer,
        scene: &mut Scene,
        frame: Rect,
        clip: Rect,
        palette: Palette,
    ) -> f32 {
        let mut text = FixtureText(text);
        let text = &mut text;
        let scale = text.resolved_font_size(TextRole::UiBody) / TextRole::UiBody.style().size;
        let mut y = frame.min_y();
        match self.0 {
            "list" => matrix_list(text, scene, frame, clip, &mut y, palette, scale),
            "tree" => matrix_tree(text, scene, frame, clip, &mut y, frame.size.width, palette),
            "data-table" => {
                matrix_table(text, scene, frame, clip, &mut y, frame.size.width, palette)
            }
            "chart" => matrix_chart(text, scene, frame, clip, &mut y, palette, scale),
            _ => unreachable!("collection matrix"),
        }
        y - frame.min_y()
    }
}
impl Component for Matrix {
    fn measure(&mut self, text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        let frame = Rect::new(0., 0., available.width, 100_000.);
        let height = self.render(
            &mut Metrics(text),
            &mut Scene::new(),
            frame,
            frame,
            Theme::LIGHT.into(),
        );
        MeasuredChild::new(Size::new(available.width, height))
    }
    fn paint(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        frame: Rect,
        clip: Rect,
        theme: ContentTheme,
    ) {
        scene.push_clip(clip);
        self.render(text, scene, frame, clip, theme.palette);
        scene.pop_clip();
    }
}
pub(super) fn matrix(renderer: &'static str) -> Vec<Content> {
    vec![
        text(
            97000,
            "Production view specimens with fixed data. Use Playground for pointer and keyboard interaction.",
        ),
        Content::new(ComponentId(97001), Matrix(renderer)),
    ]
}
/// Shared matrix text boundary: the legacy route retains its paragraph painter,
/// while AppHost borrows the receiving Host's text and icon services.
pub(in crate::gallery::workbench) trait MatrixPainter:
    ButtonTextRenderer
{
    fn heading(&mut self, scene: &mut Scene, frame: Rect, title: &str, palette: Palette) -> f32 {
        let role = TextRole::UiBody;
        let height = self.measure(title, role, frame.size.width.max(0.)).height;
        self.draw(
            scene,
            title,
            role,
            Rect::new(
                frame.min_x(),
                frame.min_y(),
                frame.size.width.max(0.),
                height,
            ),
            ButtonTextAlign::Leading,
            tekes_ui_tokens::surface_palette::resolve_surface(palette, "canvas", "content", "rest")
                .expect("matrix heading Surface packet")
                .text,
        );
        height
    }
}
impl MatrixPainter for FixtureText<'_> {}
fn matrix_heading(
    text: &mut impl MatrixPainter,
    scene: &mut Scene,
    x: f32,
    y: f32,
    w: f32,
    title: &str,
    palette: Palette,
) -> f32 {
    text.heading(scene, Rect::new(x, y, w, 0.), title, palette)
}
pub(in crate::gallery::workbench) fn matrix_list(
    text: &mut impl MatrixPainter,
    scene: &mut Scene,
    bounds: Rect,
    clip: Rect,
    y: &mut f32,
    palette: Palette,
    scale: f32,
) {
    let width = bounds.size.width.max(0.0);
    let labels = [
        "Workspace 工作区",
        "Documents 文档",
        "Long mixed-language item 中文与 English",
        "Archive 归档",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect::<Vec<_>>();
    for (title, selected, hovered, focused, disabled) in [
        ("Default", None, None, None, false),
        ("Selected", Some(1), None, None, false),
        ("Selected + hover", Some(1), Some(1), None, false),
        ("Keyboard focus-visible", Some(2), None, Some(2), false),
        ("Disabled component", Some(1), None, None, true),
    ] {
        let heading = matrix_heading(text, scene, bounds.min_x(), *y, width, title, palette);
        let frame_y = *y + heading + 7.0;
        let mut model = controls::List::new(labels.len());
        if let Some(index) = selected {
            model.select(index);
        }
        let mut view = ListView::new();
        if disabled {
            view.set_disabled(true);
        }
        if let Some(index) = focused {
            view.focus(index, true);
        }
        let frame = Rect::new(bounds.min_x(), frame_y, width.min(480.0), 1_000.0);
        let mut probe = Scene::new();
        let layout = view.render_scaled(
            &mut probe,
            text,
            &model,
            &labels,
            frame,
            Some(clip),
            palette,
            scale,
        );
        if let Some(index) = hovered
            && let Some(item) = layout.items.get(index)
        {
            view.pointer_move(item.frame.center());
        }
        let actual = view.render_scaled(
            scene,
            text,
            &model,
            &labels,
            frame,
            Some(clip),
            palette,
            scale,
        );
        let bottom = actual
            .items
            .iter()
            .map(|row| row.frame.max_y())
            .fold(frame_y, f32::max);
        *y = bottom + 18.0;
    }
}

pub(in crate::gallery::workbench) fn matrix_tree(
    text: &mut impl MatrixPainter,
    scene: &mut Scene,
    bounds: Rect,
    clip: Rect,
    y: &mut f32,
    width: f32,
    palette: Palette,
) {
    for (title, expanded, selected, hovered, focused, disabled) in [
        ("Collapsed", false, None, None, None, None),
        ("Expanded", true, None, None, None, None),
        ("Selected + hover", true, Some(1), Some(1), None, None),
        ("Keyboard focus-visible", true, Some(2), None, Some(2), None),
        ("Disabled row", true, None, None, None, Some(3)),
    ] {
        let heading = matrix_heading(text, scene, bounds.min_x(), *y, width, title, palette);
        let frame_y = *y + heading + 7.0;
        let mut model = collection_tree_fixture(expanded);
        if let Some(index) = selected {
            model.select(index);
        }
        if let Some(index) = disabled {
            model.set_disabled(index, true);
        }
        let mut view = TreeView::new();
        if let Some(index) = focused {
            view.focus(index, true);
        }
        let frame = Rect::new(bounds.min_x(), frame_y, width.min(480.0), 1_000.0);
        let mut probe = Scene::new();
        let layout = view.render(&mut probe, text, &model, frame, Some(clip), palette);
        if let Some(index) = hovered
            && let Some(item) = layout.items.iter().find(|item| item.id == index)
        {
            view.pointer_move(item.frame.center());
        }
        let actual = view.render(scene, text, &model, frame, Some(clip), palette);
        if let Some(icons) = text.icon_renderer() {
            view.render_icons(scene, &mut Icons(icons), Some(clip), palette, 16.0);
        }
        let bottom = actual
            .items
            .iter()
            .map(|row| row.frame.max_y())
            .fold(frame_y, f32::max);
        *y = bottom + 18.0;
    }
}

fn collection_tree_fixture(expanded: bool) -> controls::Tree {
    let mut tree = controls::Tree::default();
    let root = tree.add("Workspace 工作区", None).unwrap();
    tree.add("Documents 文档", Some(root)).unwrap();
    tree.add("Design system 设计规范", Some(root)).unwrap();
    tree.add("Archived material 归档资料", Some(root)).unwrap();
    if expanded {
        tree.toggle(root);
    }
    tree
}

pub(in crate::gallery::workbench) fn matrix_table(
    text: &mut impl MatrixPainter,
    scene: &mut Scene,
    bounds: Rect,
    clip: Rect,
    y: &mut f32,
    width: f32,
    palette: Palette,
) {
    let headers = ["Name", "Kind", "Updated"]
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let rows: Vec<Vec<String>> = vec![
        vec!["Design workspace".into(), "Folder".into(), "Today".into()],
        vec![
            "Notes and references".into(),
            "Text".into(),
            "Yesterday".into(),
        ],
        vec!["Icon assets".into(), "Image".into(), "Monday".into()],
    ];
    for (title, selected, sort, focus_header, focus_row, disabled) in [
        ("Default", None, None, None, None, false),
        (
            "Sorted ascending",
            None,
            Some((0, controls::SortDirection::Ascending)),
            None,
            None,
            false,
        ),
        ("Selected + hover", Some(1), None, None, None, false),
        (
            "Sorted descending",
            None,
            Some((0, controls::SortDirection::Descending)),
            None,
            None,
            false,
        ),
        (
            "Keyboard focus-visible · header",
            None,
            None,
            Some(0),
            None,
            false,
        ),
        (
            "Keyboard focus-visible · row",
            None,
            None,
            None,
            Some(1),
            false,
        ),
        ("Disabled table", None, None, None, None, true),
    ] {
        let heading = matrix_heading(text, scene, bounds.min_x(), *y, width, title, palette);
        let frame_y = *y + heading + 7.0;
        let mut model = controls::DataTable::new(rows.len(), headers.len());
        model.selected_row = selected;
        model.sort = sort;
        let mut order = (0..rows.len()).collect::<Vec<_>>();
        if let Some((column, direction)) = model.sort {
            order.sort_by(|a, b| rows[*a][column].cmp(&rows[*b][column]));
            if direction == controls::SortDirection::Descending {
                order.reverse();
            }
        }
        let mut view = DataTableView::new();
        if disabled {
            view.set_disabled(true);
        }
        if let Some(column) = focus_header {
            view.focus_header(column, true);
        }
        if let Some(row) = focus_row {
            view.focus_row(row, true);
        }
        let frame = Rect::new(bounds.min_x(), frame_y, width.min(720.0), 1_000.0);
        let mut probe = Scene::new();
        let layout = view.render(
            &mut probe,
            text,
            &model,
            &headers,
            &rows,
            &order,
            frame,
            Some(clip),
            palette,
        );
        if title == "Selected + hover"
            && let Some(item) = layout.rows.iter().find(|item| item.id == 1)
        {
            view.pointer_move(item.frame.center());
        }
        let actual = view.render(
            scene,
            text,
            &model,
            &headers,
            &rows,
            &order,
            frame,
            Some(clip),
            palette,
        );
        let bottom = actual
            .rows
            .iter()
            .map(|row| row.frame.max_y())
            .chain(actual.headers.iter().map(|item| item.frame.max_y()))
            .fold(frame_y, f32::max);
        *y = bottom + 18.0;
    }
}

pub(in crate::gallery::workbench) fn matrix_chart(
    text: &mut impl MatrixPainter,
    scene: &mut Scene,
    bounds: Rect,
    clip: Rect,
    y: &mut f32,
    palette: Palette,
    scale: f32,
) {
    let width = bounds.size.width.max(0.0);
    let data = || {
        vec![
            ChartDatum::new("Design", 18.0),
            ChartDatum::new("Build", 31.0),
            ChartDatum::new("Review", 23.0),
            ChartDatum::new("Ship", 38.0),
        ]
    };
    for (title, hovered, focused) in [
        ("Default · selected datum", None, false),
        ("Hover datum", Some(1), false),
        ("Keyboard focus-visible · selected datum", None, true),
    ] {
        let heading = matrix_heading(text, scene, bounds.min_x(), *y, width, title, palette);
        let frame_y = *y + heading + 7.0;
        let mut model = BarChart::new(data());
        let frame = Rect::new(
            bounds.min_x(),
            frame_y,
            width.min(680.0),
            220.0 * scale.max(1.0),
        );
        let mut view = ChartView::new();
        let mut probe = Scene::new();
        view.render(
            &mut probe,
            text,
            &mut model,
            frame,
            Some(clip),
            palette,
            scale,
        );
        if let Some(index) = hovered
            && let Some(bar) = view
                .layout()
                .geometry
                .bars
                .iter()
                .find(|bar| bar.index == index)
        {
            view.pointer_move(bar.rect.center(), &mut model);
        }
        if focused {
            view.focus(true, &mut model);
        }
        view.render(scene, text, &mut model, frame, Some(clip), palette, scale);
        *y = frame.max_y() + 18.0;
    }
}

struct Icons<'a>(&'a mut dyn tekes_ui_controls::layout_view::IconRenderer);
impl tekes_ui_controls::layout_view::IconRenderer for Icons<'_> {
    fn draw_icon(&mut self, scene: &mut Scene, name: &str, frame: Rect, tint: Rgba) {
        self.0.draw_icon(scene, name, frame, tint);
    }
}
// Retain the original story width caps. This is a layout-only wrapper; its
// child still owns input, focus, accessibility and the application clock.
#[derive(Debug)]
struct Viewport {
    width: f32,
    stack: StackContent,
    arranged_child: Option<(ComponentId, u64)>,
}
impl Component for Viewport {
    fn lifecycle(&mut self, _: Lifecycle) {
        self.arranged_child = None;
    }
    fn host_reveal_passthrough(&self) -> bool {
        // Width capping is translation invariant. Reveal must still reject a
        // new/unarranged child after Reset or a Host subtree replacement.
        self.stack.children.len() == 1
            && self.arranged_child.is_some_and(|identity| {
                let child = &self.stack.children[0];
                (child.id(), child.mount_identity()) == identity
            })
    }
    fn curve_subtree_capability(&self) -> CurveSubtreeCapability {
        CurveSubtreeCapability::Recursive
    }
    fn has_stable_hooks(&self) -> bool {
        true
    }
    fn supports_prepared_lifecycle(&self) -> bool {
        true
    }
    fn ignores_pointer_actions(&self) -> bool {
        true
    }
    fn children(&self) -> &[Content] {
        &self.stack.children
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.stack.children)
    }
    fn measure(&mut self, text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        self.arranged_child = None;
        self.stack.measure(
            text,
            Size::new(available.width.min(self.width), available.height),
        )
    }
    fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        self.stack.arrange(
            text,
            Rect::new(
                frame.min_x(),
                frame.min_y(),
                frame.size.width.min(self.width),
                frame.size.height,
            ),
            clip,
        );
        self.arranged_child = self
            .stack
            .children
            .first()
            .filter(|child| !child.frame().is_empty())
            .map(|child| (child.id(), child.mount_identity()));
    }
}
fn table_snapshot() -> controls::KeyedTableSnapshot {
    use controls::*;
    let names = ["Project", "Notes", "Assets", "Archive", "Readme"];
    let kinds = ["Folder", "Text", "Image", "Folder", "Text"];
    let updated = ["Today", "Yesterday", "Monday", "Friday", "Tuesday"];
    KeyedTableSnapshot {
        dataset: TableDatasetKey(1),
        columns: ["Name", "Kind", "Updated"]
            .into_iter()
            .enumerate()
            .map(|(i, name)| KeyedTableColumn {
                key: TableColumnKey(i as u64),
                name: name.into(),
            })
            .collect(),
        rows: (0..5)
            .map(|i| {
                let cells = vec![
                    format!("{} {}", names[i], i + 1),
                    kinds[i].into(),
                    updated[i].into(),
                ];
                KeyedTableRow {
                    key: TableRowKey(i as u64),
                    name: cells.join(" · "),
                    cells,
                }
            })
            .collect(),
        display_rows: (0..5).map(TableRowKey).collect(),
    }
}
fn table_displays(snapshot: &controls::KeyedTableSnapshot) -> TableDisplays {
    TableDisplays {
        headers: snapshot
            .columns
            .iter()
            .map(|c| TableHeaderDisplay {
                key: c.key,
                display: TableDisplayUpdate::Keep,
            })
            .collect(),
        rows: snapshot
            .rows
            .iter()
            .map(|r| TableRowDisplay {
                key: r.key,
                id: ComponentId(97200 + r.key.0),
                replace_wrapper: false,
                cells: snapshot
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
/// Construct only on the initial mount or Reset. The routing owner retains this
/// subtree across section changes, so the original Content remains state owner.
pub(super) fn playground(renderer: &str, state: &StoryState) -> Vec<Content> {
    use controls::*;
    let specimen = match renderer {
        "list" => {
            let snapshot = KeyedListSnapshot {
                dataset: ListDatasetKey(1),
                items: ["Overview", "Documents", "Images", "Archive", "Trash"]
                    .into_iter()
                    .enumerate()
                    .map(|(i, name)| KeyedListItem {
                        key: ListItemKey(i as u64),
                        name: name.into(),
                    })
                    .collect(),
                display_items: (0..5).map(ListItemKey).collect(),
            };
            let mut rows = snapshot
                .items
                .iter()
                .map(|item| ListRowDisplay {
                    key: item.key,
                    row_id: ComponentId(97100 + item.key.0),
                    display: ListDisplayUpdate::DefaultLabel,
                })
                .collect();
            let mut list = SelectableListContent::keyed(
                "Workspace",
                &snapshot,
                &mut rows,
                ListBudget {
                    max_items: 5,
                    max_display_items: 5,
                    max_utf8_bytes: 4096,
                    max_staging_units: 4096,
                },
            )
            .expect("finite original List fixture");
            if let Some(index) = state.extended.list.selected {
                list.select_key(ListItemKey(index as u64))
                    .expect("original row");
            }
            Content::new(SPECIMEN, list)
        }
        "tree" => {
            let nodes = state
                .extended
                .tree
                .nodes()
                .iter()
                .enumerate()
                .map(|(i, node)| TreeContentNode {
                    row_id: ComponentId(97100 + i as u64),
                    node: KeyedTreeNode {
                        key: TreeNodeKey(i as u64),
                        parent: node.parent.map(|i| TreeNodeKey(i as u64)),
                        label: node.label.clone(),
                        disabled: node.disabled,
                        initially_expanded: node.expanded,
                    },
                    display: TreeDisplayUpdate::DefaultLabel,
                })
                .collect();
            let mut tree = TreeContent::new("Workspace", TreeDatasetKey(1), nodes)
                .expect("original Tree fixture");
            if let Some(index) = state.extended.tree.selected {
                tree.select_key(TreeNodeKey(index as u64));
            }
            Content::new(SPECIMEN, tree)
        }
        "data-table" => {
            let snapshot = table_snapshot();
            let table = DataTableContent::new(
                "Workspace files",
                &snapshot,
                &mut table_displays(&snapshot),
                TableBudget {
                    max_units: 100_000,
                    max_utf8_bytes: 1_000_000,
                },
            )
            .expect("finite original Table fixture and Host staging budget");
            Content::new(SPECIMEN, table)
        }
        "chart" => Content::new(
            SPECIMEN,
            ChartContent::try_new("Release stages", state.extended.chart.data().to_vec())
                .expect("original finite chart fixture")
                .preferred_height(320.),
        ),
        _ => unreachable!("collection story"),
    };
    let help = match renderer {
        "list" => "Single selection. Up / Down move; Home / End jump to first / last.",
        "tree" => {
            "Right expands; Left collapses or moves to the parent. Up / Down navigate visible rows."
        }
        "data-table" => {
            "Activate a header to sort deterministically. Up and Down move through rows; Enter selects the focused row."
        }
        _ => {
            "Arrow keys move the selected data mark. Non-finite values are omitted and are not selectable."
        }
    };
    let width = match renderer {
        "list" => 360.,
        "tree" => 420.,
        "data-table" => 720.,
        _ => 760.,
    };
    let specimen = Content::new(
        ComponentId(97003),
        Viewport {
            width,
            arranged_child: None,
            stack: StackContent {
                model: controls::Stack::new(controls::Axis::Vertical),
                children: vec![specimen],
                flexible_child: None,
            },
        },
    );
    vec![
        specimen,
        text(97002, help),
        text(
            92003,
            state.log.iter().cloned().collect::<Vec<_>>().join("\n"),
        ),
    ]
}
/// Call after draining and processing action records. The original table's sort reducer owns
/// direction; Gallery supplies the same deterministic display order as before.
pub(super) fn sync_model(renderer: &str, state: &mut StoryState, host: &mut ContentHost) {
    let Some(content) = host.get(SPECIMEN) else {
        return;
    }; // Docs/states have no live specimen.
    let mut changed = false;
    let mut sorted = None;
    match renderer {
        "list" => {
            if let Some(list) = content.component::<KeyedSelectableListContent>() {
                let selected = list.selected_key().map(|k| k.0 as usize);
                changed = state.extended.list.selected != selected;
                state.extended.list.selected = selected;
                state.extended.selected = selected.unwrap_or_default();
            }
        }
        "tree" => {
            if let Some(tree) = content.component::<TreeContent>() {
                let selected = tree.selected_key().map(|k| k.0 as usize);
                changed = state.extended.tree.selected != selected;
                state.extended.tree.selected = selected;
                state.extended.selected = selected.unwrap_or_default();
            }
        }
        "data-table" => {
            if let Some(table) = content.component::<DataTableContent>() {
                let model = table.model().original();
                changed = state.extended.table.selected_row != model.selected_row
                    || state.extended.table.sort != model.sort;
                if state.extended.table.sort != model.sort {
                    let mut snapshot = table.model().snapshot().clone();
                    if let Some((column, direction)) = model.sort {
                        let mut order = (0..snapshot.rows.len()).collect::<Vec<_>>();
                        order.sort_by(|a, b| {
                            snapshot.rows[*a].cells[column].cmp(&snapshot.rows[*b].cells[column])
                        });
                        if direction == controls::SortDirection::Descending {
                            order.reverse();
                        }
                        snapshot.display_rows =
                            order.iter().map(|i| snapshot.rows[*i].key).collect();
                        state.extended.visible_rows = order;
                    }
                    sorted = Some(snapshot);
                }
                state.extended.table = model.clone();
                state.extended.selected = model.selected_row.unwrap_or_default();
            }
        }
        "chart" => {
            if let Some(chart) = content.component::<ChartContent>() {
                let selected = chart.selected_index();
                changed = selected != state.extended.chart.selected();
                if let Some(index) = selected {
                    state.extended.chart.select(index);
                }
                state.extended.selected = selected.unwrap_or_default();
            }
        }
        _ => {}
    }
    if let Some(snapshot) = sorted {
        host.replace_table_snapshot(SPECIMEN, &snapshot, &mut table_displays(&snapshot), None)
            .expect("same finite table rows in sorted order");
    }
    if changed {
        state.record(format!("{renderer}: selection changed"));
        refresh_log(state, host);
    }
}
/// Forward records targeted at SPECIMEN here; do not consume records for other
/// controls. Selection is synchronized above; these preserve activation logs.
pub(super) fn action(
    state: &mut StoryState,
    host: &mut ContentHost,
    payload: &ContentActionPayload,
) {
    let message = match payload {
        ContentActionPayload::ListIntent(intent) if intent.kind == ListIntentKind::Activated => {
            Some(format!("list: activated {}", intent.key.0))
        }
        ContentActionPayload::TreeIntent(intent) => match intent.kind {
            TreeIntentKind::Activated => Some(format!("tree: activated {}", intent.key.0)),
            TreeIntentKind::ExpansionChanged { expanded } => {
                let index = intent.key.0 as usize;
                if state.extended.tree.nodes()[index].expanded != expanded {
                    state.extended.tree.toggle(index);
                }
                Some(format!("tree node: {index}"))
            }
            _ => None,
        },
        ContentActionPayload::TableIntent(intent) if intent.kind == TableIntentKind::Activated => {
            match intent.target {
                TableIntentTarget::Row(key) => Some(format!("data-table: activated {}", key.0)),
                _ => None,
            }
        }
        _ => None,
    };
    if let Some(message) = message {
        state.record(message);
        refresh_log(state, host);
    }
}
fn refresh_log(state: &StoryState, host: &mut ContentHost) {
    // Section navigation can remove the live log before queued records drain.
    if let Some(log) = host.get_mut(ComponentId(92003)) {
        log.component_mut::<TextContent>().unwrap().text =
            state.log.iter().cloned().collect::<Vec<_>>().join("\n");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tekes_ui_controls::ControlKey;
    use tekes_ui_controls::layout_view::IconRenderer;
    use tekes_ui_scene::ImageId;

    #[derive(Default)]
    struct Text;
    impl IconRenderer for Text {
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
    impl ButtonTextRenderer for Text {
        fn icon_renderer(&mut self) -> Option<&mut dyn IconRenderer> {
            Some(self)
        }
        fn measure(&mut self, s: &str, _: TextRole, width: f32) -> Size {
            Size::new((s.chars().count() as f32 * 8.).min(width), 20.)
        }
        fn draw(
            &mut self,
            _: &mut Scene,
            _: &str,
            _: TextRole,
            _: Rect,
            _: ButtonTextAlign,
            _: Rgba,
        ) {
        }
    }
    fn mount(renderer: &str) -> (StoryState, ContentHost) {
        let state = StoryState::default();
        let specimen = playground(renderer, &state).remove(0);
        let mut host = ContentHost::new(specimen).unwrap();
        host.set_action_events(true);
        render(&mut host);
        (state, host)
    }
    fn render(host: &mut ContentHost) {
        let frame = Rect::new(0., 0., 720., 500.);
        host.render(
            &mut Scene::new(),
            &mut Text,
            frame,
            frame,
            Theme::LIGHT.into(),
        )
        .unwrap();
    }
    fn parts(host: &ContentHost) -> Vec<SemanticPart> {
        host.semantics()
            .into_iter()
            .find(|n| n.id == SPECIMEN)
            .unwrap()
            .parts
    }
    fn finish(renderer: &str, state: &mut StoryState, host: &mut ContentHost) {
        for record in host.drain_action_records() {
            if let Some(payload) = record.payload {
                action(state, host, &payload);
            }
        }
        sync_model(renderer, state, host);
        render(host);
    }
    #[test]
    fn apphost_collections_sort_reorders_original_rows_without_changing_selected_identity() {
        let (mut state, mut host) = mount("data-table");
        let row = parts(&host)
            .into_iter()
            .find(|p| p.semantics.role == ContentRole::TableRow { index: 1 })
            .unwrap();
        assert!(
            host.accessibility_action(SPECIMEN, Some(row.index), AccessibilityAction::Press)
                .consumed
        );
        finish("data-table", &mut state, &mut host);
        assert_eq!(state.extended.table.selected_row, Some(1));
        for expected in [vec![3, 2, 1, 0, 4], vec![4, 0, 1, 2, 3]] {
            let header = parts(&host)
                .into_iter()
                .find(|p| {
                    matches!(
                        p.semantics.role,
                        ContentRole::TableColumnHeader { index: 0, .. }
                    )
                })
                .unwrap();
            assert!(
                host.accessibility_action(SPECIMEN, Some(header.index), AccessibilityAction::Press)
                    .consumed
            );
            finish("data-table", &mut state, &mut host);
            let table = host
                .get(SPECIMEN)
                .unwrap()
                .component::<DataTableContent>()
                .unwrap();
            assert_eq!(table.model().display_indices(), expected);
            assert_eq!(table.selected_key(), Some(controls::TableRowKey(1)));
            assert_eq!(state.extended.table.selected_row, Some(1));
        }
    }
    #[test]
    fn apphost_collections_list_keyboard_updates_the_story_model() {
        let (mut state, mut host) = mount("list");
        assert!(host.focus(Some(SPECIMEN), true));
        assert!(
            host.event(&ContentEvent::Key {
                key: ControlKey::End,
                down: true
            })
            .consumed
        );
        finish("list", &mut state, &mut host);
        assert_eq!(state.extended.list.selected, Some(4));
        assert!(
            host.event(&ContentEvent::Key {
                key: ControlKey::Home,
                down: true
            })
            .consumed
        );
        finish("list", &mut state, &mut host);
        assert_eq!(state.extended.list.selected, Some(0));
    }
    #[test]
    fn apphost_collections_tree_expansion_keeps_original_children_and_log() {
        let (mut state, mut host) = mount("tree");
        assert_eq!(parts(&host).len(), 3);
        let root = parts(&host)
            .into_iter()
            .find(|p| p.semantics.expanded == Some(true))
            .unwrap();
        assert!(
            host.accessibility_action(
                SPECIMEN,
                Some(root.index),
                AccessibilityAction::SetExpanded(false)
            )
            .consumed
        );
        finish("tree", &mut state, &mut host);
        assert_eq!(parts(&host).len(), 1);
        assert!(!state.extended.tree.nodes()[0].expanded);
        assert!(state.log.iter().any(|s| s.contains("tree node: 0")));
        let root = parts(&host).remove(0);
        assert!(
            host.accessibility_action(
                SPECIMEN,
                Some(root.index),
                AccessibilityAction::SetExpanded(true)
            )
            .consumed
        );
        finish("tree", &mut state, &mut host);
        assert_eq!(parts(&host).len(), 3);
        assert!(state.extended.tree.nodes()[0].expanded);
    }
    #[test]
    fn apphost_collections_chart_keyboard_preserves_original_data_and_selection() {
        let (mut state, mut host) = mount("chart");
        let chart = host.get(SPECIMEN).unwrap();
        assert_eq!(chart.frame().size, Size::new(720., 320.));
        assert!(
            matches!(
                host.required_focus_result(SPECIMEN),
                Some(RequiredFocusResult::Visible { .. })
            ),
            "the chart already has a visible witness before ancestor admission"
        );
        assert!(host.focus(Some(SPECIMEN), true));
        assert!(
            host.event(&ContentEvent::Key {
                key: ControlKey::End,
                down: true
            })
            .consumed
        );
        finish("chart", &mut state, &mut host);
        assert_eq!(state.extended.chart.selected(), Some(3));
        assert_eq!(state.extended.chart.data()[3].label, "Release");
        assert_eq!(state.extended.chart.data()[3].value, 9.);
    }
}
