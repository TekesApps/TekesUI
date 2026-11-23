use super::chart_and_docs::chart_text_frame;
use super::*;
use crate::extended::{DataTable, List, Tree};
use crate::layout_view::IconRenderer;
use tekes_ui_scene::{Primitive, Quad, Size};
use tekes_ui_tokens::{Rgba, Theme};
pub(super) struct FixedText {
    pub(super) height: f32,
}

pub(super) struct ScaledText(pub(super) f32);

impl ButtonTextRenderer for ScaledText {
    fn backing_scale(&self) -> f32 {
        self.0
    }

    fn measure(&mut self, _text: &str, _role: proposal::TextRole, max_width: f32) -> Size {
        Size::new(max_width.min(100.0), 14.0)
    }

    fn draw(
        &mut self,
        _scene: &mut Scene,
        _text: &str,
        _role: proposal::TextRole,
        _rect: Rect,
        _align: ButtonTextAlign,
        _ink: Rgba,
    ) {
    }
}
impl ButtonTextRenderer for FixedText {
    fn measure(&mut self, _text: &str, _role: proposal::TextRole, max_width: f32) -> Size {
        Size::new(max_width.min(100.), self.height)
    }
    fn draw(
        &mut self,
        _scene: &mut Scene,
        _text: &str,
        _role: proposal::TextRole,
        _rect: Rect,
        _align: ButtonTextAlign,
        _ink: Rgba,
    ) {
    }
}
#[derive(Default)]
pub(super) struct RecordingText {
    pub(super) measured: Vec<(String, f32)>,
    pub(super) drawn: Vec<String>,
    pub(super) draw_rects: Vec<Rect>,
    pub(super) alignments: Vec<ButtonTextAlign>,
    pub(super) ink_origins: Vec<Point>,
    pub(super) ink_widths: Vec<f32>,
}
impl ButtonTextRenderer for RecordingText {
    fn measure(&mut self, value: &str, _role: proposal::TextRole, max_width: f32) -> Size {
        self.measured.push((value.to_owned(), max_width));
        let height = if value.chars().count() > 12 { 72. } else { 14. };
        Size::new(max_width.min(100.), height)
    }
    fn draw(
        &mut self,
        _scene: &mut Scene,
        value: &str,
        role: proposal::TextRole,
        rect: Rect,
        align: ButtonTextAlign,
        _ink: Rgba,
    ) {
        // Mirror the Gallery text engine's `label_origin` contract so
        // tests verify the requested ink origin as well as the slot.
        let ink_width = self.measure(value, role, rect.size.width).width;
        let ink_x = match align {
            ButtonTextAlign::Leading => rect.min_x(),
            ButtonTextAlign::Center => rect.center().x - ink_width * 0.5,
        };
        self.drawn.push(value.to_owned());
        self.draw_rects.push(rect);
        self.alignments.push(align);
        self.ink_origins.push(Point::new(ink_x, rect.min_y()));
        self.ink_widths.push(ink_width);
    }
}
#[derive(Default)]
pub(super) struct RecordingIcon(pub(super) Vec<(String, Rect, Rgba)>);
impl IconRenderer for RecordingIcon {
    fn draw_icon(&mut self, scene: &mut Scene, canonical_name: &str, frame: Rect, tint: Rgba) {
        self.0.push((canonical_name.to_owned(), frame, tint));
        scene.push_quad(Quad::new(frame, tint));
    }
}
pub(super) fn rect() -> Rect {
    Rect::new(10., 20., 240., 180.)
}
#[test]
fn wholly_offscreen_mounted_collections_keep_keyboard_geometry_but_no_paint_or_hits() {
    let palette = Palette::from(Theme::LIGHT);
    let onscreen = Rect::new(0., 0., 240., 240.);
    let offscreen = Rect::new(0., 400., 240., 240.);
    let clip = Some(Rect::new(0., 0., 240., 240.));
    let labels: [String; 4] = ["one".into(), "two".into(), "three".into(), "four".into()];
    let mut text = RecordingText::default();
    let mut scene = Scene::new();
    let mut list = List::new(4);
    let mut list_view = ListView::new();
    list_view.render(
        &mut scene, &mut text, &list, &labels, onscreen, clip, palette,
    );
    list_view.focus(0, true);
    scene = Scene::new();
    text.drawn.clear();
    let layout = list_view.render(
        &mut scene, &mut text, &list, &labels, offscreen, clip, palette,
    );
    assert_eq!(layout.items.len(), 4);
    assert_eq!(layout.visible, None);
    assert!(layout.items.iter().all(|row| row.visible.is_none()));
    assert!(scene.primitives().is_empty());
    assert!(text.drawn.is_empty());
    assert_eq!(
        list_view.key(ControlKey::End, true, &mut list),
        CollectionEvent::Changed
    );
    assert_eq!(list.selected, Some(3));
    assert_eq!(
        list_view.pointer_down(layout.items[3].frame.center(), &mut list),
        CollectionEvent::Ignored
    );
    assert_eq!(
        list_view.key(ControlKey::Home, true, &mut list),
        CollectionEvent::Changed
    );
    assert_eq!(list.selected, Some(0));
    list_view.render(
        &mut scene,
        &mut text,
        &list,
        &labels,
        offscreen,
        Some(Rect::default()),
        palette,
    );
    assert!(list_view.layout().items.is_empty());
    assert_eq!(
        list_view.key(ControlKey::End, true, &mut list),
        CollectionEvent::Ignored
    );

    let mut tree = Tree::default();
    for label in ["one", "two", "three", "four"] {
        tree.add(label, None);
    }
    let mut tree_view = TreeView::new();
    tree_view.render(&mut scene, &mut text, &tree, onscreen, clip, palette);
    tree_view.focus(0, true);
    scene = Scene::new();
    text.drawn.clear();
    let layout = tree_view.render(&mut scene, &mut text, &tree, offscreen, clip, palette);
    assert_eq!(layout.items.len(), 4);
    assert!(layout.items.iter().all(|row| row.visible.is_none()));
    assert!(scene.primitives().is_empty());
    assert!(text.drawn.is_empty());
    tree_view.key(ControlKey::End, true, &mut tree);
    assert_eq!(tree.selected, Some(3));
    assert_eq!(
        tree_view.pointer_down(layout.items[3].frame.center(), &mut tree),
        CollectionEvent::Ignored
    );
    tree_view.key(ControlKey::Home, true, &mut tree);
    assert_eq!(tree.selected, Some(0));
    tree_view.render(
        &mut scene,
        &mut text,
        &tree,
        offscreen,
        Some(Rect::default()),
        palette,
    );
    assert!(tree_view.layout().items.is_empty());
    assert_eq!(
        tree_view.key(ControlKey::End, true, &mut tree),
        CollectionEvent::Ignored
    );

    let rows = labels
        .iter()
        .map(|label| vec![label.clone()])
        .collect::<Vec<_>>();
    let headers = ["Name".into()];
    let order = [0, 1, 2, 3];
    let mut table = DataTable::new(4, 1);
    let mut table_view = DataTableView::new();
    table_view.render(
        &mut scene, &mut text, &table, &headers, &rows, &order, onscreen, clip, palette,
    );
    table_view.focus_row(0, true);
    scene = Scene::new();
    text.drawn.clear();
    let layout = table_view.render(
        &mut scene, &mut text, &table, &headers, &rows, &order, offscreen, clip, palette,
    );
    assert_eq!(layout.rows.len(), 4);
    assert_eq!(layout.headers.len(), 1);
    assert!(layout.rows.iter().all(|row| row.visible.is_none()));
    assert!(scene.primitives().is_empty());
    assert!(text.drawn.is_empty());
    table_view.key(ControlKey::End, true, &mut table);
    assert_eq!(table.selected_row, Some(3));
    assert_eq!(
        table_view.pointer_down(layout.rows[3].frame.center(), &mut table),
        CollectionEvent::Ignored
    );
    table_view.key(ControlKey::Home, true, &mut table);
    assert_eq!(table.selected_row, Some(0));
    table_view.render(
        &mut scene,
        &mut text,
        &table,
        &headers,
        &rows,
        &order,
        offscreen,
        Some(Rect::default()),
        palette,
    );
    assert!(table_view.layout().rows.is_empty());
    assert_eq!(
        table_view.key(ControlKey::End, true, &mut table),
        CollectionEvent::Ignored
    );
    let mut chart = BarChart::new(vec![
        crate::ChartDatum::new("first", 1.),
        crate::ChartDatum::new("last", 2.),
    ]);
    let mut chart_view = ChartView::new();
    chart_view.render(
        &mut scene, &mut text, &mut chart, onscreen, clip, palette, 1.,
    );
    chart_view.focus(true, &mut chart);
    scene = Scene::new();
    text.drawn.clear();
    let layout = chart_view.render(
        &mut scene, &mut text, &mut chart, offscreen, clip, palette, 1.,
    );
    assert_eq!(layout.geometry.bars.len(), 2);
    assert_eq!(layout.visible, None);
    assert!(scene.primitives().is_empty());
    assert!(text.drawn.is_empty());
    assert_eq!(
        chart_view.key(ControlKey::End, true, &mut chart),
        CollectionEvent::Changed
    );
    assert_eq!(chart.selected(), Some(1));
    assert_eq!(
        chart_view.pointer_down(layout.geometry.bars[1].rect.center(), &mut chart),
        CollectionEvent::Ignored
    );
    chart_view.key(ControlKey::Home, true, &mut chart);
    assert_eq!(chart.selected(), Some(0));
    chart_view.render(
        &mut scene,
        &mut text,
        &mut chart,
        offscreen,
        Some(Rect::default()),
        palette,
        1.,
    );
    assert!(chart_view.layout().geometry.bars.is_empty());
    assert_eq!(
        chart_view.key(ControlKey::End, true, &mut chart),
        CollectionEvent::Ignored
    );
}

#[test]
fn offscreen_collection_rows_remain_keyboard_targets_without_pointer_hits() {
    let mut scene = Scene::new();
    let mut text = FixedText { height: 14. };
    let palette = Palette::from(Theme::LIGHT);
    let frame = Rect::new(0., 0., 240., 240.);
    let clip = Some(Rect::new(0., 0., 240., 78.));
    let mut list = List::new(4);
    let mut list_view = ListView::new();
    list_view.render(
        &mut scene,
        &mut text,
        &list,
        &["one".into(), "two".into(), "three".into(), "four".into()],
        frame,
        clip,
        palette,
    );
    list_view.focus(0, true);
    list_view.key(ControlKey::End, true, &mut list);
    assert_eq!(list.selected, Some(3));
    assert!(list_view.layout().items[3].visible.is_none());
    assert_eq!(
        list_view.pointer_down(list_view.layout().items[3].frame.center(), &mut list),
        CollectionEvent::Ignored
    );
    list_view.focus(3, true);
    assert_eq!(
        list_view.key(ControlKey::Enter, true, &mut list),
        CollectionEvent::Activated(3)
    );
    assert_eq!(list.selected, Some(3));

    let mut tree = Tree::default();
    for label in ["one", "two", "three", "four"] {
        tree.add(label, None);
    }
    tree.set_disabled(1, true);
    let mut tree_view = TreeView::new();
    tree_view.render(&mut scene, &mut text, &tree, frame, clip, palette);
    tree_view.focus(0, true);
    tree_view.key(ControlKey::End, true, &mut tree);
    assert_eq!(tree.selected, Some(3));
    assert!(tree_view.layout().items[3].visible.is_none());
    assert_eq!(
        tree_view.pointer_down(tree_view.layout().items[3].frame.center(), &mut tree),
        CollectionEvent::Ignored
    );
    tree_view.focus(3, true);
    assert_eq!(
        tree_view.key(ControlKey::Enter, true, &mut tree),
        CollectionEvent::Activated(3)
    );

    let mut table = DataTable::new(4, 1);
    let mut table_view = DataTableView::new();
    table_view.render(
        &mut scene,
        &mut text,
        &table,
        &["header".into()],
        &[
            vec!["one".into()],
            vec!["two".into()],
            vec!["three".into()],
            vec!["four".into()],
        ],
        &[0, 1, 2, 3],
        frame,
        clip,
        palette,
    );
    table_view.focus_row(3, true);
    assert_eq!(
        table_view.key(ControlKey::Enter, true, &mut table),
        CollectionEvent::Changed
    );
    assert_eq!(table.selected_row, Some(3));
    assert!(table_view.layout().rows[3].visible.is_none());
    assert_eq!(
        table_view.pointer_down(table_view.layout().rows[3].frame.center(), &mut table),
        CollectionEvent::Ignored
    );
}

#[test]
fn collection_keys_follow_restored_focus_and_reject_disabled_tree_focus() {
    let mut scene = Scene::new();
    let mut text = FixedText { height: 14. };
    let palette = Palette::from(Theme::LIGHT);
    let frame = Rect::new(0., 0., 240., 240.);
    let mut list = List::new(4);
    list.select(0);
    let mut view = ListView::new();
    view.render(
        &mut scene,
        &mut text,
        &list,
        &["one".into(), "two".into(), "three".into(), "four".into()],
        frame,
        None,
        palette,
    );
    view.focus(2, true);
    assert_eq!(
        view.key(ControlKey::Down, true, &mut list),
        CollectionEvent::Changed
    );
    assert_eq!(list.selected, Some(3));
    view.focus(1, true);
    assert_eq!(
        view.key(ControlKey::Enter, true, &mut list),
        CollectionEvent::Activated(1)
    );
    assert_eq!(list.selected, Some(1));

    let mut tree = Tree::default();
    tree.add("one", None);
    tree.add("two", None);
    tree.add("three", None);
    tree.select(0);
    let mut view = TreeView::new();
    view.render(&mut scene, &mut text, &tree, frame, None, palette);
    view.focus(1, true);
    assert_eq!(
        view.key(ControlKey::Down, true, &mut tree),
        CollectionEvent::Changed
    );
    assert_eq!(tree.selected, Some(2));
    view.focus(1, true);
    assert_eq!(
        view.key(ControlKey::Enter, true, &mut tree),
        CollectionEvent::Activated(1)
    );
    tree.set_disabled(1, true);
    assert_eq!(
        view.key(ControlKey::Enter, true, &mut tree),
        CollectionEvent::Ignored
    );
    assert_eq!(tree.selected, None);
}

#[test]
fn collection_views_ignore_tab_without_marking_focus_visible_and_chart_frame_clamps_nan() {
    let mut scene = Scene::new();
    let mut text = FixedText { height: 14. };
    let palette = Palette::from(Theme::LIGHT);
    let frame = Rect::new(0., 0., 240., 240.);

    let mut list = List::new(2);
    let mut list_view = ListView::new();
    list_view.render(
        &mut scene,
        &mut text,
        &list,
        &["one".into(), "two".into()],
        frame,
        None,
        palette,
    );
    list_view.focus(0, false);
    assert_eq!(
        list_view.key(ControlKey::Tab { reverse: false }, true, &mut list),
        CollectionEvent::Ignored
    );
    assert!(!list_view.state.focus_visible);

    let mut tree = Tree::default();
    tree.add("one", None);
    let mut tree_view = TreeView::new();
    tree_view.render(&mut scene, &mut text, &tree, frame, None, palette);
    tree_view.focus(0, false);
    assert_eq!(
        tree_view.key(ControlKey::Tab { reverse: true }, true, &mut tree),
        CollectionEvent::Ignored
    );
    assert!(!tree_view.state.focus_visible);

    let mut table = DataTable::new(1, 1);
    let mut table_view = DataTableView::new();
    table_view.render(
        &mut scene,
        &mut text,
        &table,
        &["header".into()],
        &[vec!["row".into()]],
        &[0],
        frame,
        None,
        palette,
    );
    table_view.focus_row(0, false);
    assert_eq!(
        table_view.key(ControlKey::Tab { reverse: false }, true, &mut table),
        CollectionEvent::Ignored
    );
    assert!(!table_view.state.focus_visible);

    for width in [-10.0, f32::NAN] {
        let (rect, _) = chart_text_frame(
            &mut text,
            "100",
            Rect::new(0.0, 0.0, width, 16.0),
            tekes_ui_tokens::proposal::layout::TextAlignment::Trailing,
        );
        assert!(rect.size.width.is_finite());
        assert!(rect.size.width >= 0.0);
    }
}

#[test]
fn list_uses_owned_clip_geometry_and_clips_wide_text() {
    let mut view = ListView::new();
    let model = List::new(4);
    let labels = (0..4).map(|i| format!("row {i}")).collect::<Vec<_>>();
    let mut text = FixedText { height: 60. };
    let mut scene = Scene::new();
    let layout = view.render(
        &mut scene,
        &mut text,
        &model,
        &labels,
        rect(),
        Some(Rect::new(10., 20., 240., 30.)),
        Theme::LIGHT.into(),
    );
    assert_eq!(layout.items.len(), 4);
    assert!(layout.items[0].visible.is_some());
    assert!(layout.items[1].visible.is_none());
    assert!(layout.items[0].frame.size.height >= 60.);
}

#[test]
fn list_tree_and_table_keep_gallery_default_row_geometry() {
    let mut scene = Scene::new();
    let mut text = FixedText { height: 14. };

    let mut list = ListView::new();
    let list_model = List::new(2);
    let list_layout = list.render(
        &mut scene,
        &mut text,
        &list_model,
        &["first".into(), "second".into()],
        Rect::new(0., 0., 200., 100.),
        None,
        Theme::LIGHT.into(),
    );
    assert_eq!(list_layout.items[0].frame.size.height, 32.);
    assert_eq!(list_layout.items[1].frame.min_y(), 36.);

    let mut tree = Tree::default();
    let root = tree.add("root", None).unwrap();
    let child = tree.add("child", Some(root)).unwrap();
    tree.toggle(root);
    let mut tree_view = TreeView::new();
    let tree_layout = tree_view.render(
        &mut scene,
        &mut text,
        &tree,
        Rect::new(0., 0., 200., 100.),
        None,
        Theme::LIGHT.into(),
    );
    assert_eq!(tree_layout.items[0].frame.size.height, 32.);
    assert_eq!(tree_layout.items[1].frame.min_y(), 36.);
    assert_eq!(
        tree_layout.items[1].frame.min_x() - tree_layout.items[0].frame.min_x(),
        20.
    );
    assert_eq!(tree_layout.items[1].id, child);

    let table_model = DataTable::new(2, 1);
    let mut table_view = DataTableView::new();
    let table_layout = table_view.render(
        &mut scene,
        &mut text,
        &table_model,
        &["Name".into()],
        &[vec!["first".into()], vec!["second".into()]],
        &[0, 1],
        Rect::new(0., 0., 200., 120.),
        None,
        Theme::LIGHT.into(),
    );
    assert_eq!(table_layout.headers[0].frame.size.height, 32.);
    assert_eq!(table_layout.rows[0].frame.min_y(), 38.);
    assert_eq!(table_layout.rows[0].frame.size.height, 32.);
    assert_eq!(
        table_layout.rows[1].frame.min_y() - table_layout.rows[0].frame.min_y(),
        36.
    );
}

#[test]
fn collection_compact_mode_padding_and_accessory_gap() {
    let mut scene = Scene::new();
    let mut text = FixedText { height: 14. };

    let mut list = ListView::new().with_compact(true);
    let list_model = List::new(2);
    let list_layout = list.render(
        &mut scene,
        &mut text,
        &list_model,
        &["first".into(), "second".into()],
        Rect::new(0., 0., 200., 100.),
        None,
        Theme::LIGHT.into(),
    );
    assert_eq!(list_layout.items[0].frame.size.height, 24.);
    assert_eq!(list_layout.items[1].frame.min_y(), 28.);

    let mut tree = Tree::default();
    tree.add("root", None).unwrap();
    let mut tree_view = TreeView::new().with_compact(true);
    let tree_layout = tree_view.render(
        &mut scene,
        &mut text,
        &tree,
        Rect::new(0., 0., 200., 100.),
        None,
        Theme::LIGHT.into(),
    );
    assert_eq!(tree_layout.items[0].frame.size.height, 24.);

    let mut wrapped_tree = Tree::default();
    wrapped_tree.add("wrapped item", None).unwrap();
    let mut wrapped_text = FixedText { height: 40. };
    let wrapped_layout = TreeView::new().render(
        &mut scene,
        &mut wrapped_text,
        &wrapped_tree,
        Rect::new(0., 0., 200., 100.),
        None,
        Theme::LIGHT.into(),
    );
    assert_eq!(wrapped_layout.items[0].frame.size.height, 48.);

    let table_model = DataTable::new(1, 1);
    let mut compact_table = DataTableView::new().with_compact(true);
    let compact_layout = compact_table.render(
        &mut scene,
        &mut text,
        &table_model,
        &["Name".into()],
        &[vec!["first".into()]],
        &[0],
        Rect::new(0., 0., 200., 120.),
        None,
        Theme::LIGHT.into(),
    );
    assert_eq!(compact_layout.headers[0].frame.size.height, 24.);
    assert_eq!(compact_layout.rows[0].frame.size.height, 24.);
    assert_eq!(compact_layout.rows[0].frame.min_y(), 30.);

    let wrapped_table_layout = DataTableView::new().render(
        &mut scene,
        &mut wrapped_text,
        &table_model,
        &["Name".into()],
        &[vec!["wrapped cell".into()]],
        &[0],
        Rect::new(0., 0., 200., 200.),
        None,
        Theme::LIGHT.into(),
    );
    assert_eq!(wrapped_table_layout.rows[0].frame.size.height, 48.);
}

#[test]
fn tree_uses_fixed_real_icon_slot_and_clips_branch_icons() {
    let mut tree = Tree::default();
    let root = tree.add("Workspace", None).unwrap();
    let branch = tree.add("Documents", Some(root)).unwrap();
    tree.add("Report", Some(branch)).unwrap();
    let leaf = tree.add("Settings", None).unwrap();
    tree.toggle(root);
    let mut view = TreeView::new();
    let mut scene = Scene::new();
    let mut text = RecordingText::default();
    let frame = Rect::new(20.0, 30.0, 360.0, 120.0);
    let full_clip = frame;
    let layout = view.render(
        &mut scene,
        &mut text,
        &tree,
        frame,
        Some(full_clip),
        Theme::LIGHT.into(),
    );
    assert_eq!(
        layout.items.iter().map(|row| row.id).collect::<Vec<_>>(),
        [root, branch, leaf]
    );
    assert_eq!(text.drawn, ["Workspace", "Documents", "Settings"]);
    assert_eq!(
        text.ink_origins[0].x, text.ink_origins[2].x,
        "branch and leaf at the same depth keep the same label origin"
    );
    assert_eq!(text.draw_rects[0].min_x(), text.draw_rects[2].min_x());

    let mut icons = RecordingIcon::default();
    view.render_icons(
        &mut scene,
        &mut icons,
        Some(full_clip),
        Theme::LIGHT.into(),
        16.0,
    );
    assert_eq!(
        icons
            .0
            .iter()
            .map(|item| item.0.as_str())
            .collect::<Vec<_>>(),
        ["tekes-expand", "tekes-forward"]
    );
    assert!(
        icons
            .0
            .iter()
            .all(|(_, rect, _)| rect.size.width == 16.0 && rect.size.height == 16.0)
    );
    let expected_ink = tekes_ui_tokens::surface_palette::resolve_surface(
        Palette::from(Theme::LIGHT),
        "canvas",
        "action-quiet",
        "rest",
    )
    .unwrap()
    .icon;
    assert!(
        icons.0.iter().all(|(_, _, tint)| *tint == expected_ink),
        "Quiet/rest icon tint: actual={:?}, expected={:?}",
        icons.0,
        expected_ink
    );

    view.set_disabled(true);
    view.render(
        &mut scene,
        &mut text,
        &tree,
        frame,
        Some(full_clip),
        Theme::LIGHT.into(),
    );
    let mut disabled_icons = RecordingIcon::default();
    view.render_icons(
        &mut scene,
        &mut disabled_icons,
        Some(full_clip),
        Theme::LIGHT.into(),
        16.0,
    );
    let disabled_ink = Palette::from(Theme::LIGHT).color("tertiary-text").unwrap();
    assert!(
        disabled_icons
            .0
            .iter()
            .all(|(_, _, tint)| *tint == disabled_ink),
        "Quiet/disabled icon tint: actual={:?}, expected tertiary-text={:?}",
        disabled_icons.0,
        disabled_ink
    );
    view.set_disabled(false);

    let clipped = Rect::new(frame.min_x(), frame.min_y(), frame.size.width, 34.0);
    view.render(
        &mut scene,
        &mut text,
        &tree,
        frame,
        Some(clipped),
        Theme::LIGHT.into(),
    );
    let mut clipped_icons = RecordingIcon::default();
    view.render_icons(
        &mut scene,
        &mut clipped_icons,
        Some(clipped),
        Theme::LIGHT.into(),
        16.0,
    );
    assert_eq!(
        clipped_icons
            .0
            .iter()
            .map(|item| item.0.as_str())
            .collect::<Vec<_>>(),
        ["tekes-expand"]
    );
    assert!(clipped_icons.0[0].1.intersection(&clipped).is_some());

    view.render(
        &mut scene,
        &mut text,
        &tree,
        Rect::new(frame.min_x(), frame.min_y(), 0.0, frame.size.height),
        Some(full_clip),
        Theme::LIGHT.into(),
    );
    let mut hidden_icons = RecordingIcon::default();
    view.render_icons(
        &mut scene,
        &mut hidden_icons,
        None,
        Theme::LIGHT.into(),
        16.0,
    );
    assert!(
        hidden_icons.0.is_empty(),
        "hidden render clears old icon slots"
    );
}

#[test]
fn tree_icon_render_reuses_full_and_partial_row_clips() {
    let mut tree = Tree::default();
    let root = tree.add("Workspace", None).unwrap();
    let branch = tree.add("Documents", Some(root)).unwrap();
    tree.add("Report", Some(branch)).unwrap();
    tree.toggle(root);
    let mut view = TreeView::new();
    let mut scene = Scene::new();
    let mut text = FixedText { height: 14.0 };
    let frame = Rect::new(0.0, 0.0, 300.0, 120.0);
    view.render(
        &mut scene,
        &mut text,
        &tree,
        frame,
        None,
        Theme::LIGHT.into(),
    );
    scene.clear();
    let mut visible_icons = RecordingIcon::default();
    view.render_icons(
        &mut scene,
        &mut visible_icons,
        None,
        Theme::LIGHT.into(),
        16.0,
    );
    assert_eq!(
        visible_icons
            .0
            .iter()
            .map(|icon| icon.0.as_str())
            .collect::<Vec<_>>(),
        ["tekes-expand", "tekes-forward"]
    );

    let full_clip = Rect::new(0.0, 0.0, frame.size.width, 34.0);
    view.render(
        &mut scene,
        &mut text,
        &tree,
        frame,
        Some(full_clip),
        Theme::LIGHT.into(),
    );
    scene.clear();
    let mut row_clipped_icons = RecordingIcon::default();
    view.render_icons(
        &mut scene,
        &mut row_clipped_icons,
        None,
        Theme::LIGHT.into(),
        16.0,
    );
    assert_eq!(
        row_clipped_icons
            .0
            .iter()
            .map(|icon| icon.0.as_str())
            .collect::<Vec<_>>(),
        ["tekes-expand"]
    );

    let partial_clip = Rect::new(0.0, 42.0, frame.size.width, 10.0);
    let partial_layout = view.render(
        &mut scene,
        &mut text,
        &tree,
        frame,
        Some(partial_clip),
        Theme::LIGHT.into(),
    );
    scene.clear();
    let mut partial_icons = RecordingIcon::default();
    view.render_icons(
        &mut scene,
        &mut partial_icons,
        None,
        Theme::LIGHT.into(),
        16.0,
    );
    assert_eq!(
        partial_icons
            .0
            .iter()
            .map(|icon| icon.0.as_str())
            .collect::<Vec<_>>(),
        ["tekes-forward"]
    );
    let clips = scene
        .primitives()
        .iter()
        .filter_map(|primitive| {
            if let Primitive::PushClip(rect) = primitive {
                Some(*rect)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    let icon_slot = Rect::new(
        partial_layout.items[1].frame.min_x()
            + proposal::metric("spacing", "space-relaxed").unwrap_or(12.0),
        partial_layout.items[1].frame.min_y(),
        proposal::metric("size", "size-inline-action").expect("Tree expand action slot token"),
        partial_layout.items[1].frame.size.height,
    );
    assert_eq!(
        clips,
        [icon_slot.intersection(&partial_clip).unwrap()],
        "icon pass must retain the rendered row clip and remain inside its reserved slot"
    );
}

#[test]
fn data_table_measures_final_sorted_header_label_before_positioning_rows() {
    let mut model = DataTable::new(1, 2);
    model.sort_by(0);
    let headers = ["primary field".to_owned(), "Kind".to_owned()];
    let rows = vec![vec!["A".to_owned(), "Text".to_owned()]];
    let mut view = DataTableView::new();
    let mut text = RecordingText::default();
    let mut scene = Scene::new();
    let frame = Rect::new(10.0, 20.0, 400.0, 180.0);
    let layout = view.render(
        &mut scene,
        &mut text,
        &model,
        &headers,
        &rows,
        &[0],
        frame,
        Some(frame),
        Theme::LIGHT.into(),
    );
    assert!(
        text.measured
            .iter()
            .any(|(label, _)| label == "primary field ↑")
    );
    let header_rect = text
        .drawn
        .iter()
        .zip(text.draw_rects.iter())
        .find(|(label, _)| label.as_str() == "primary field ↑")
        .map(|(_, rect)| *rect)
        .expect("rendered sorted header");
    let cell_rect = text
        .drawn
        .iter()
        .zip(text.draw_rects.iter())
        .find(|(label, _)| label.as_str() == "A")
        .map(|(_, rect)| *rect)
        .expect("rendered first column cell");
    assert!(
        (header_rect.min_x() - cell_rect.min_x()).abs() <= 0.01,
        "header min_x={} vs cell min_x={}",
        header_rect.min_x(),
        cell_rect.min_x()
    );
    assert_eq!(layout.headers[0].frame.size.height, 80.0);
    assert_eq!(layout.rows[0].frame.min_y(), frame.min_y() + 86.0);
    assert_eq!(layout.content_height, 118.0);
}
