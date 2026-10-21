use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer,
    collections_view::{DataTableView, ListView, TreeView},
    extended::{DataTable, List, Tree},
};
use tekes_ui_scene::{Primitive, Rect, Scene, Size};
use tekes_ui_tokens::{Rgba, Theme, proposal};
struct Text;
impl ButtonTextRenderer for Text {
    fn measure(&mut self, s: &str, r: proposal::TextRole, w: f32) -> Size {
        Size::new((s.len() as f32 * 7.).min(w), r.style().line_height)
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
fn scene(kind: u8, clip: Option<Rect>, height: f32) -> (Scene, Rect) {
    let bounds = Rect::new(12.25, 12.5, 300., height);
    let mut scene = Scene::new();
    let frame = match kind {
        0 => {
            let model = List::new(2);
            let mut view = ListView::new();
            view.render(
                &mut Scene::new(),
                &mut Text,
                &model,
                &["A".into(), "B".into()],
                bounds,
                clip,
                Theme::LIGHT.into(),
            );
            view.focus(0, true);
            view.render(
                &mut scene,
                &mut Text,
                &model,
                &["A".into(), "B".into()],
                bounds,
                clip,
                Theme::LIGHT.into(),
            )
            .items[0]
                .frame
        }
        1 => {
            let mut model = Tree::default();
            let root = model.add("Parent", None).unwrap();
            model.add("Child", Some(root)).unwrap();
            model.toggle(root);
            let mut view = TreeView::new();
            view.render(
                &mut Scene::new(),
                &mut Text,
                &model,
                bounds,
                clip,
                Theme::LIGHT.into(),
            );
            view.focus(root, true);
            view.render(
                &mut scene,
                &mut Text,
                &model,
                bounds,
                clip,
                Theme::LIGHT.into(),
            )
            .items[0]
                .frame
        }
        _ => {
            let model = DataTable::new(2, 3);
            let mut view = DataTableView::new();
            let headers = vec!["A".into(), "B".into(), "C".into()];
            let rows = vec![vec!["1".into(); 3], vec!["2".into(); 3]];
            view.render(
                &mut Scene::new(),
                &mut Text,
                &model,
                &headers,
                &rows,
                &[0, 1],
                bounds,
                clip,
                Theme::LIGHT.into(),
            );
            if kind == 2 {
                view.focus_header(0, true);
            } else {
                view.focus_row(0, true);
            }
            let layout = view.render(
                &mut scene,
                &mut Text,
                &model,
                &headers,
                &rows,
                &[0, 1],
                bounds,
                clip,
                Theme::LIGHT.into(),
            );
            if kind == 2 {
                layout.headers[0].frame
            } else {
                layout.rows[0].frame
            }
        }
    };
    (scene, frame)
}
fn focus(scene: &Scene, frame: Rect) -> (Option<Rect>, bool) {
    let outset = proposal::metric("stroke", "c-focus-width").unwrap()
        + proposal::metric("spacing", "c-focus-offset").unwrap();
    let expected = frame.inset(-outset);
    let mut clips = vec![None];
    let mut found = None;
    let mut last_quad = None;
    for (i, p) in scene.primitives().iter().enumerate() {
        match p {
            Primitive::PushClip(r) => {
                let parent: Option<Rect> = *clips.last().unwrap();
                clips.push(Some(
                    parent.map_or(*r, |v| v.intersection(r).unwrap_or_default()),
                ));
            }
            Primitive::PopClip if clips.len() > 1 => {
                clips.pop();
            }
            Primitive::Quad(q) => {
                last_quad = Some(i);
                if q.rect == expected
                    && q.border_color
                        == tekes_ui_tokens::Palette::from(Theme::LIGHT)
                            .color("border-focused")
                            .unwrap()
                {
                    assert!(found.is_none(), "focus must be emitted once");
                    found = Some((i, *clips.last().unwrap()));
                }
            }
            _ => {}
        }
    }
    let (index, clip) = found.expect("canonical focus quad");
    (clip, last_quad == Some(index))
}
#[test]
fn list_complete_focus_uses_true_ancestor_clip_and_follows_sibling_chrome() {
    let (s, f) = scene(0, None, 220.);
    assert_eq!(focus(&s, f), (None, true));
}
#[test]
fn tree_complete_focus_uses_true_ancestor_clip_and_follows_sibling_chrome() {
    let (s, f) = scene(1, None, 220.);
    assert_eq!(focus(&s, f), (None, true));
}
#[test]
fn table_header_complete_focus_is_not_cut_or_occluded_by_adjacent_columns() {
    let (s, f) = scene(2, None, 220.);
    assert_eq!(focus(&s, f), (None, true));
}
#[test]
fn table_row_complete_focus_uses_true_ancestor_clip() {
    let (s, f) = scene(3, None, 220.);
    assert_eq!(focus(&s, f), (None, true));
}
#[test]
fn real_ancestor_viewport_is_still_enforced() {
    let clip = Rect::new(15., 0., 250., 180.);
    for kind in 0..4 {
        let (s, f) = scene(kind, Some(clip), 220.);
        assert_eq!(focus(&s, f), (Some(clip), true));
    }
}
#[test]
fn partially_cropped_rows_keep_the_collection_overflow_clip() {
    let bounds = Rect::new(12.25, 12.5, 300., 20.);
    for kind in [0, 1, 2] {
        let (s, f) = scene(kind, None, 20.);
        assert_eq!(focus(&s, f), (Some(bounds), true));
    }
}

#[test]
fn expanded_focus_ink_never_enlarges_list_tree_or_table_pointer_targets() {
    use tekes_ui_controls::collections_view::CollectionEvent as E;
    use tekes_ui_scene::Point;
    let frame = Rect::new(12.25, 12.5, 300., 220.);
    let mut list = List::new(2);
    let mut lv = ListView::new();
    let labels = vec!["A".into(), "B".into()];
    lv.render(
        &mut Scene::new(),
        &mut Text,
        &list,
        &labels,
        frame,
        None,
        Theme::LIGHT.into(),
    );
    lv.focus(0, true);
    let row = lv
        .render(
            &mut Scene::new(),
            &mut Text,
            &list,
            &labels,
            frame,
            None,
            Theme::LIGHT.into(),
        )
        .items[0]
        .frame;
    assert_eq!(
        lv.pointer_down(Point::new(row.min_x() - 3., row.center().y), &mut list),
        E::Ignored
    );
    assert_eq!(list.selected, None);
    let mut tree = Tree::default();
    let id = tree.add("Root", None).unwrap();
    let mut tv = TreeView::new();
    tv.render(
        &mut Scene::new(),
        &mut Text,
        &tree,
        frame,
        None,
        Theme::LIGHT.into(),
    );
    tv.focus(id, true);
    let row = tv
        .render(
            &mut Scene::new(),
            &mut Text,
            &tree,
            frame,
            None,
            Theme::LIGHT.into(),
        )
        .items[0]
        .frame;
    assert_eq!(
        tv.pointer_down(Point::new(row.min_x() - 3., row.center().y), &mut tree),
        E::Ignored
    );
    assert_eq!(tree.selected, None);
    let mut table = DataTable::new(2, 1);
    let mut dv = DataTableView::new();
    let headers = vec!["Name".into()];
    let rows = vec![vec!["A".into()], vec!["B".into()]];
    dv.render(
        &mut Scene::new(),
        &mut Text,
        &table,
        &headers,
        &rows,
        &[0, 1],
        frame,
        None,
        Theme::LIGHT.into(),
    );
    dv.focus_header(0, true);
    let layout = dv.render(
        &mut Scene::new(),
        &mut Text,
        &table,
        &headers,
        &rows,
        &[0, 1],
        frame,
        None,
        Theme::LIGHT.into(),
    );
    let row = layout.headers[0].frame;
    assert_eq!(
        dv.pointer_down(Point::new(row.min_x() - 3., row.center().y), &mut table),
        E::Ignored
    );
    assert_eq!(table.sort, None);
    dv.focus_row(0, true);
    let row = dv
        .render(
            &mut Scene::new(),
            &mut Text,
            &table,
            &headers,
            &rows,
            &[0, 1],
            frame,
            None,
            Theme::LIGHT.into(),
        )
        .rows[0]
        .frame;
    assert_eq!(
        dv.pointer_down(Point::new(row.min_x() - 3., row.center().y), &mut table),
        E::Ignored
    );
    assert_eq!(table.selected_row, None);
}
#[test]
fn wholly_clipped_logical_keyboard_target_does_not_emit_a_phantom_focus_ring() {
    let model = List::new(5);
    let mut view = ListView::new();
    let labels = vec!["Row".into(); 5];
    let frame = Rect::new(10., 10., 300., 40.);
    view.render(
        &mut Scene::new(),
        &mut Text,
        &model,
        &labels,
        frame,
        None,
        Theme::LIGHT.into(),
    );
    view.focus(4, true);
    let mut scene = Scene::new();
    let layout = view.render(
        &mut scene,
        &mut Text,
        &model,
        &labels,
        frame,
        None,
        Theme::LIGHT.into(),
    );
    assert!(layout.items[4].visible.is_none());
    let width = proposal::metric("stroke", "c-focus-width").unwrap();
    let color = tekes_ui_tokens::Palette::from(Theme::LIGHT)
        .color("border-focused")
        .unwrap();
    assert!(
        !scene.primitives().iter().any(
            |p| matches!(p,Primitive::Quad(q) if q.border_width==width&&q.border_color==color)
        )
    );
}

#[test]
fn last_column_focus_survives_fractional_width_partitioning() {
    for width in [173.3, 284.2, 302.5] {
        for columns in [2, 3, 5, 7] {
            let model = DataTable::new(1, columns);
            let mut view = DataTableView::new();
            let headers = vec!["Header".into(); columns];
            let rows = vec![vec!["Value".into(); columns]];
            let bounds = Rect::new(12.25, 12.5, width, 220.);
            view.render(
                &mut Scene::new(),
                &mut Text,
                &model,
                &headers,
                &rows,
                &[0],
                bounds,
                None,
                Theme::LIGHT.into(),
            );
            view.focus_header(columns - 1, true);
            let mut scene = Scene::new();
            let layout = view.render(
                &mut scene,
                &mut Text,
                &model,
                &headers,
                &rows,
                &[0],
                bounds,
                None,
                Theme::LIGHT.into(),
            );
            assert_eq!(
                focus(&scene, layout.headers[columns - 1].frame),
                (None, true),
                "width={width} columns={columns}"
            );
        }
    }
}
