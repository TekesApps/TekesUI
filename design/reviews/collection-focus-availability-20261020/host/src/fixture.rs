use tekes_ui_controls::{
    BarChart, ChartDatum, ControlKey,
    collections_view::{ChartView, CollectionEvent as E, DataTableView, ListView, TreeView},
    extended::{DataTable, List, Tree},
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Family {
    List,
    Tree,
    TableRow,
    TableHeader,
    Chart,
}
impl Family {
    fn component(self) -> &'static str {
        match self {
            Self::List => "tui.list",
            Self::Tree => "tui.tree",
            Self::TableRow | Self::TableHeader => "tui.data-table",
            Self::Chart => "tui.chart",
        }
    }
    fn renderer(self) -> &'static str {
        match self {
            Self::List => "list",
            Self::Tree => "tree",
            Self::TableRow | Self::TableHeader => "data-table",
            Self::Chart => "chart",
        }
    }
}
struct CollectionsViews {
    list: ListView,
    tree: TreeView,
    table: DataTableView,
    chart: ChartView,
}
struct ExtendedStory {
    list: List,
    tree: Tree,
    table: DataTable,
    chart: BarChart,
    collections_views: CollectionsViews,
}
mod production {
    use super::*;
    include!(concat!(env!("OUT_DIR"), "/gallery_focus.rs"));
}
impl ExtendedStory {
    fn new() -> Self {
        let mut tree = Tree::default();
        let root = tree.add("Parent · 分组", None).unwrap();
        tree.add("Child · 子项", Some(root)).unwrap();
        tree.toggle(root);
        Self {
            list: List::new(2),
            tree,
            table: DataTable::new(2, 1),
            chart: BarChart::new(vec![
                ChartDatum::new("A · 类别", 1.),
                ChartDatum::new("B · 类别", 2.),
            ]),
            collections_views: CollectionsViews {
                list: ListView::new(),
                tree: TreeView::new(),
                table: DataTableView::new(),
                chart: ChartView::new(),
            },
        }
    }
    fn focus(&mut self, f: Family) {
        production::focus_changed(
            self,
            f.renderer(),
            Some(if f == Family::TableHeader { 20 } else { 31 }),
            true,
        );
    }
    fn active(&mut self, f: Family, active: bool) {
        match f {
            Family::List => self.collections_views.list.set_window_active(active),
            Family::Tree => self.collections_views.tree.set_window_active(active),
            Family::TableRow | Family::TableHeader => {
                self.collections_views.table.set_window_active(active)
            }
            Family::Chart => self
                .collections_views
                .chart
                .set_window_active(active, &mut self.chart),
        }
    }
    fn disabled(&mut self, f: Family, disabled: bool) {
        match f {
            Family::List => self.collections_views.list.set_disabled(disabled),
            Family::Tree => self.collections_views.tree.set_disabled(disabled),
            Family::TableRow | Family::TableHeader => {
                self.collections_views.table.set_disabled(disabled)
            }
            Family::Chart => assert!(
                !disabled,
                "Chart has no disabled model contract; no invented API"
            ),
        }
    }
    fn select(&mut self, f: Family) {
        match f {
            Family::List => {
                self.list.select(1);
                assert_eq!(self.list.selected, Some(1));
            }
            Family::Tree => {
                self.tree.select(1);
                assert_eq!(self.tree.selected, Some(1));
            }
            Family::TableRow => {
                self.table.select_row(1);
                assert_eq!(self.table.selected_row, Some(1));
            }
            Family::TableHeader => {
                self.table.sort_by(0);
                assert!(self.table.sort.is_some());
            }
            Family::Chart => {
                self.chart.select(1);
                assert_eq!(self.chart.selected(), Some(1));
            }
        }
    }
    fn key(&mut self, f: Family) -> E {
        match f {
            Family::List => {
                self.collections_views
                    .list
                    .key(ControlKey::Enter, true, &mut self.list)
            }
            Family::Tree => {
                self.collections_views
                    .tree
                    .key(ControlKey::Enter, true, &mut self.tree)
            }
            Family::TableRow | Family::TableHeader => {
                self.collections_views
                    .table
                    .key(ControlKey::Enter, true, &mut self.table)
            }
            Family::Chart => {
                self.collections_views
                    .chart
                    .key(ControlKey::Left, true, &mut self.chart)
            }
        }
    }
    fn paint(
        &mut self,
        f: Family,
        scene: &mut tekes_ui_scene::Scene,
        text: &mut impl tekes_ui_controls::ButtonTextRenderer,
        width: f32,
        palette: tekes_ui_tokens::Palette,
    ) {
        let frame = tekes_ui_scene::Rect::new(8., 8., width - 16., 204.);
        match f {
            Family::List => {
                self.collections_views.list.render(
                    scene,
                    text,
                    &self.list,
                    &["Source A · 当前".into(), "Source B · 第二项".into()],
                    frame,
                    None,
                    palette,
                );
            }
            Family::Tree => {
                self.collections_views
                    .tree
                    .render(scene, text, &self.tree, frame, None, palette);
            }
            Family::TableRow | Family::TableHeader => {
                self.collections_views.table.render(
                    scene,
                    text,
                    &self.table,
                    &["Name · 名称".into()],
                    &[
                        vec!["Source A · 当前".into()],
                        vec!["Source B · 第二项".into()],
                    ],
                    &[1, 0],
                    frame,
                    None,
                    palette,
                );
            }
            Family::Chart => {
                self.collections_views.chart.render(
                    scene,
                    text,
                    &mut self.chart,
                    frame,
                    None,
                    palette,
                    1.,
                );
            }
        }
    }
}
