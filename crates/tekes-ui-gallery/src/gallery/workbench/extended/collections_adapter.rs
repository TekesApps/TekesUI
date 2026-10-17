//! Gallery adapters for the collection and document components.
//!
//! The Gallery supplies deterministic example data and the available viewport.
//! Component geometry, clipping, paint and input remain in the production views.
use super::super::super::pages::icons::GalleryIconRenderer;
use super::super::{Entry, PageCtx, StoryState, line};
use super::{ExtendedStory, controls};
use tekes_ui_controls::{
    ControlKey,
    collections_view::{
        ChartView, CodeBlockView, CollectionEvent, DataTableView, DiffViewRuntime, ListView,
        MarkdownView, TreeView,
    },
};
use tekes_ui_scene::{Point, Rect, Scene};
use tekes_ui_tokens::Palette;

/// Whether an input reached the component and whether it changed the model.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct InputResult {
    pub consumed: bool,
    pub changed: bool,
}

fn place_instruction_below_rows(
    ctx: &mut PageCtx<'_>,
    scene: &mut Scene,
    bounds: Rect,
    rows: &[Rect],
    text: &str,
) -> Option<Rect> {
    if bounds.size.width <= 0.0 || bounds.size.height <= 0.0 {
        return None;
    }
    let role = tekes_ui_tokens::proposal::TextRole::UiCaption;
    // Keep the measured paragraph and the drawing path on the same wrapped
    // shaping route. The one-line `line` helper intentionally does not wrap.
    let measured = ctx.text.proposal_label(text, role, Some(bounds.size.width));
    let height = measured.height as f32;
    if !height.is_finite() || height <= 0.0 {
        return None;
    }
    let row_bottom = rows
        .iter()
        .filter(|row| row.max_y().is_finite())
        .map(|row| row.max_y())
        .fold(bounds.min_y(), f32::max);
    let y = row_bottom + tekes_ui_tokens::proposal::metric("spacing", "space-tight").unwrap_or(4.0);
    if !y.is_finite() || y + height > bounds.max_y() {
        return None;
    }
    let frame = Rect::new(bounds.min_x(), y, bounds.size.width, height);
    scene.push_clip(bounds);
    ctx.text.proposal_wrapped_in(
        scene,
        ctx.theme,
        frame,
        text,
        role,
        tekes_ui_tokens::ColorRole::SecondaryText.resolve(ctx.theme),
    );
    scene.pop_clip();
    Some(frame)
}

/// Draw a wrapped caption footer using the same width and role for shaping and
/// painting. Returns no slot when the complete footer cannot fit in the visible
/// story bounds.
fn draw_measured_footer(
    ctx: &mut PageCtx<'_>,
    scene: &mut Scene,
    visible_bounds: Rect,
    x: f32,
    y: f32,
    width: f32,
    text: &str,
) -> Option<Rect> {
    if !x.is_finite() || !y.is_finite() || !width.is_finite() || width <= 0.0 {
        return None;
    }
    if x < visible_bounds.min_x() || x + width > visible_bounds.max_x() {
        return None;
    }
    let role = tekes_ui_tokens::proposal::TextRole::UiCaption;
    let measured = ctx.text.proposal_label(text, role, Some(width));
    let height = measured.height as f32;
    if !height.is_finite()
        || height <= 0.0
        || y < visible_bounds.min_y()
        || y + height > visible_bounds.max_y()
    {
        return None;
    }
    let slot = Rect::new(x, y, width, height);
    scene.push_clip(visible_bounds);
    ctx.text.proposal_wrapped_in(
        scene,
        ctx.theme,
        slot,
        text,
        role,
        tekes_ui_tokens::ColorRole::SecondaryText.resolve(ctx.theme),
    );
    scene.pop_clip();
    Some(slot)
}

/// One runtime per existing component model. Gallery fixtures stay on
/// `ExtendedStory`; this struct contains only presentation and interaction state.
#[derive(Debug, Default)]
pub(super) struct CollectionsViews {
    pub list: ListView,
    pub tree: TreeView,
    pub table: DataTableView,
    pub chart: ChartView,
    pub markdown: MarkdownView,
    pub code: CodeBlockView,
    pub diff: DiffViewRuntime,
}

pub(super) fn is_collection(renderer: &str) -> bool {
    matches!(
        renderer,
        "list" | "tree" | "data-table" | "chart" | "markdown" | "code-block" | "diff-view"
    )
}

/// Render one of the registered collection/document stories with its real
/// production view. Returns false for renderers owned by another adapter.
#[allow(clippy::too_many_arguments)]
pub(super) fn build(
    ext: &mut ExtendedStory,
    entry: &Entry,
    state: &mut StoryState,
    ctx: &mut PageCtx<'_>,
    scene: &mut Scene,
    body: Rect,
    clip: Rect,
    palette: Palette,
) -> bool {
    let renderer = entry.renderer();
    if !is_collection(renderer) {
        return false;
    }
    let clip = body
        .intersection(&clip)
        .unwrap_or(Rect::new(body.min_x(), body.min_y(), 0.0, 0.0));
    state.hits.retain(|(id, _)| !component_id(renderer, *id));
    let text_scale = ctx.text.type_scale();
    let x = body.min_x();
    let y = body.min_y();
    let width = body.size.width.max(0.0);

    match renderer {
        "list" => {
            let labels = ["Overview", "Documents", "Images", "Archive", "Trash"]
                .into_iter()
                .map(str::to_owned)
                .collect::<Vec<_>>();
            let frame = Rect::new(x, y, width.min(360.0), body.size.height.max(0.0));
            let layout = ext.collections_views.list.render_scaled(
                scene,
                ctx.text,
                &ext.list,
                &labels,
                frame,
                Some(clip),
                palette,
                text_scale,
            );
            ext.rows = layout.items.iter().map(|item| item.frame).collect();
            let row_frames = layout
                .items
                .iter()
                .map(|item| item.frame)
                .collect::<Vec<_>>();
            for item in layout.items {
                push_hit(state, 30 + item.id, item.visible);
            }
            let note_bounds =
                body.intersection(&clip)
                    .unwrap_or(Rect::new(body.min_x(), body.min_y(), 0.0, 0.0));
            place_instruction_below_rows(
                ctx,
                scene,
                note_bounds,
                &row_frames,
                "Single selection. Up / Down move; Home / End jump to first / last.",
            );
        }
        "tree" => {
            let frame = Rect::new(x, y, width.min(420.0), body.size.height.max(0.0));
            let layout = ext.collections_views.tree.render(
                scene,
                ctx.text,
                &ext.tree,
                frame,
                Some(clip),
                palette,
            );
            let mut icon_renderer = GalleryIconRenderer::native(ctx);
            ext.collections_views.tree.render_icons(
                scene,
                &mut icon_renderer,
                Some(clip),
                palette,
                16.0,
            );
            ext.rows = layout.items.iter().map(|item| item.frame).collect();
            let row_frames = layout
                .items
                .iter()
                .map(|item| item.frame)
                .collect::<Vec<_>>();
            for item in layout.items {
                push_hit(state, 30 + item.id, item.visible);
            }
            let note_bounds =
                body.intersection(&clip)
                    .unwrap_or(Rect::new(body.min_x(), body.min_y(), 0.0, 0.0));
            place_instruction_below_rows(
                ctx,
                scene,
                note_bounds,
                &row_frames,
                "Right expands; Left collapses or moves to the parent. Up / Down navigate visible rows.",
            );
        }
        "data-table" => {
            let headers = ["Name", "Kind", "Updated"]
                .into_iter()
                .map(str::to_owned)
                .collect::<Vec<_>>();
            let names = ["Project", "Notes", "Assets", "Archive", "Readme"];
            let kinds = ["Folder", "Text", "Image", "Folder", "Text"];
            let updated = ["Today", "Yesterday", "Monday", "Friday", "Tuesday"];
            let rows = (0..ext.table.rows.min(names.len()))
                .map(|row| {
                    vec![
                        format!("{} {}", names[row], row + 1),
                        kinds[row].to_owned(),
                        updated[row].to_owned(),
                    ]
                })
                .collect::<Vec<_>>();
            let mut order = (0..rows.len()).collect::<Vec<_>>();
            if let Some((column, direction)) = ext.table.sort {
                order.sort_by(|left, right| rows[*left].get(column).cmp(&rows[*right].get(column)));
                if direction == controls::SortDirection::Descending {
                    order.reverse();
                }
            }
            ext.visible_rows = order.clone();
            let frame = Rect::new(x, y, width.min(720.0), body.size.height.max(0.0));
            let layout = ext.collections_views.table.render(
                scene,
                ctx.text,
                &ext.table,
                &headers,
                &rows,
                &order,
                frame,
                Some(clip),
                palette,
            );
            ext.rows = layout.rows.iter().map(|item| item.frame).collect();
            for header in layout.headers {
                push_hit(state, 20 + header.id, header.visible);
            }
            for row in layout.rows {
                push_hit(state, 30 + row.id, row.visible);
            }
        }
        "chart" => {
            let frame = Rect::new(
                x,
                y + 8.0,
                width.min(760.0),
                (body.size.height - 36.0).max(0.0),
            );
            let layout = ext.collections_views.chart.render(
                scene,
                ctx.text,
                &mut ext.chart,
                frame,
                Some(clip),
                palette,
                text_scale,
            );
            for bar in &layout.geometry.bars {
                let rect = bar.rect.inset(-6.0);
                push_hit(state, 30 + bar.index, rect.intersection(&clip));
            }
            let invalid_count = ext
                .chart
                .data()
                .iter()
                .filter(|datum| !datum.value.is_finite())
                .count();
            if invalid_count > 0 {
                line(
                    ctx,
                    scene,
                    Rect::new(x, frame.max_y() + 4.0, width, 22.0),
                    &format!("{invalid_count} non-finite values omitted"),
                );
            }
        }
        "markdown" => {
            let frame = Rect::new(x, y, width, body.size.height.max(0.0));
            let layout = ext.collections_views.markdown.render(
                scene,
                ctx.text,
                &ext.markdown,
                frame,
                Some(clip),
                palette,
                text_scale,
            );
            let note = "Supported subset: headings, paragraphs, bullets and fenced code.";
            let note_x = x.max(clip.min_x());
            let note_width = (clip.max_x() - note_x).max(0.0);
            let note_height = ctx
                .text
                .proposal_label(
                    note,
                    tekes_ui_tokens::proposal::TextRole::UiCaption,
                    Some(note_width),
                )
                .height as f32;
            let note_y = (body.max_y() - note_height - 4.0)
                .max(frame.min_y() + layout.content_height + 12.0);
            draw_measured_footer(ctx, scene, clip, note_x, note_y, note_width, note);
        }
        "code-block" => {
            let frame = Rect::new(
                x,
                y + 8.0,
                width.min(680.0),
                (body.size.height - 18.0).max(80.0),
            );
            let layout = ext.collections_views.code.render(
                scene,
                ctx.text,
                &ext.code,
                frame,
                Some(clip),
                palette,
                text_scale,
            );
            let footer =
                "Plain light surface; syntax coloring and editing are not part of this component.";
            let footer_x = (frame.min_x() + 14.0).max(clip.min_x());
            let footer_width = ((frame.max_x() - 14.0).min(clip.max_x()) - footer_x).max(0.0);
            let footer_height = ctx
                .text
                .proposal_label(
                    footer,
                    tekes_ui_tokens::proposal::TextRole::UiCaption,
                    Some(footer_width),
                )
                .height as f32;
            let content_bottom = layout
                .blocks
                .iter()
                .map(|block| block.frame.max_y())
                .fold(frame.min_y(), f32::max);
            let footer_y = (frame.max_y() - 14.0 - footer_height).max(content_bottom + 12.0);
            draw_measured_footer(ctx, scene, clip, footer_x, footer_y, footer_width, footer);
        }
        "diff-view" => {
            let frame = Rect::new(x, y, width.min(700.0), (body.size.height - 42.0).max(0.0));
            let layout = ext.collections_views.diff.render(
                scene,
                ctx.text,
                &ext.diff,
                frame,
                Some(clip),
                palette,
                text_scale,
            );
            let footer = "Deterministic line-based LCS fixture; large inputs fall back to delete/add blocks.";
            let footer_x = x.max(clip.min_x());
            let footer_width = (clip.max_x() - footer_x).max(0.0);
            let content_bottom = layout
                .blocks
                .iter()
                .map(|block| block.frame.max_y())
                .fold(frame.min_y(), f32::max);
            let footer_y = content_bottom + 12.0 + 2.0 * text_scale;
            draw_measured_footer(ctx, scene, clip, footer_x, footer_y, footer_width, footer);
        }
        _ => return false,
    }
    true
}

fn component_id(renderer: &str, id: usize) -> bool {
    match renderer {
        "list" => (30..35).contains(&id),
        "tree" => (30..usize::MAX).contains(&id),
        "data-table" => (20..23).contains(&id) || (30..35).contains(&id),
        "chart" => (30..34).contains(&id),
        _ => false,
    }
}
fn push_hit(state: &mut StoryState, id: usize, rect: Option<Rect>) {
    if let Some(rect) = rect.filter(|rect| rect.size.width > 0.0 && rect.size.height > 0.0) {
        state.hits.push((id, rect));
    }
}
fn result(
    event: CollectionEvent,
    renderer: &str,
    log: &mut impl FnMut(String),
) -> Option<InputResult> {
    let (changed, message) = match event {
        CollectionEvent::Ignored => return None,
        CollectionEvent::Handled => (false, None),
        CollectionEvent::Changed => (true, None),
        CollectionEvent::Activated(index) => {
            (false, Some(format!("{renderer}: activated {index}")))
        }
        CollectionEvent::Toggled(index) => (true, Some(format!("tree node: {index}"))),
    };
    if let Some(message) = message {
        log(message);
    }
    Some(InputResult {
        consumed: true,
        changed,
    })
}

pub(super) fn accepts_key(renderer: &str, focus: Option<usize>) -> bool {
    let Some(id) = focus else {
        return false;
    };
    match renderer {
        "list" => (30..35).contains(&id),
        "tree" => id >= 30,
        "data-table" => (20..23).contains(&id) || (30..35).contains(&id),
        "chart" => (30..34).contains(&id),
        _ => false,
    }
}

pub(super) fn activate(
    ext: &mut ExtendedStory,
    renderer: &str,
    id: usize,
    log: &mut impl FnMut(String),
) -> Option<InputResult> {
    match (renderer, id) {
        ("list", 30..=34) => result(
            ext.collections_views
                .list
                .key(ControlKey::Enter, true, &mut ext.list),
            renderer,
            log,
        ),
        ("tree", id) if id >= 30 => result(
            ext.collections_views
                .tree
                .key(ControlKey::Enter, true, &mut ext.tree),
            renderer,
            log,
        ),
        ("data-table", id) if (30..35).contains(&id) || (20..23).contains(&id) => result(
            ext.collections_views
                .table
                .key(ControlKey::Enter, true, &mut ext.table),
            renderer,
            log,
        ),
        ("chart", 30..=33) => Some(InputResult {
            consumed: true,
            changed: false,
        }),
        _ => None,
    }
}

pub(super) fn key(
    ext: &mut ExtendedStory,
    renderer: &str,
    focus: Option<usize>,
    key: ControlKey,
    down: bool,
    log: &mut impl FnMut(String),
) -> Option<InputResult> {
    if !is_collection(renderer) || !accepts_key(renderer, focus) {
        return None;
    }
    let event = match renderer {
        "list" => ext.collections_views.list.key(key, down, &mut ext.list),
        "tree" => {
            let id = focus?.saturating_sub(30);
            if !ext.tree.visible().contains(&id)
                || ext.tree.nodes().get(id).is_none_or(|node| node.disabled)
            {
                return Some(InputResult {
                    consumed: true,
                    changed: false,
                });
            }
            ext.collections_views.tree.key(key, down, &mut ext.tree)
        }
        "data-table" => ext.collections_views.table.key(key, down, &mut ext.table),
        "chart" => ext.collections_views.chart.key(key, down, &mut ext.chart),
        _ => return None,
    };
    let result = result(event, renderer, log);
    if result.is_some_and(|r| r.changed) {
        match renderer {
            "list" => {
                ext.selected = ext.list.selected.unwrap_or_default();
                ext.focused = ext.list.selected.map(|index| 30 + index);
            }
            "tree" => {
                ext.selected = ext.tree.selected.unwrap_or_default();
                ext.focused = ext.tree.selected.map(|index| 30 + index);
            }
            "data-table" => {
                ext.selected = ext.table.selected_row.unwrap_or_default();
                if focus.is_some_and(|id| (20..23).contains(&id))
                    && matches!(key, ControlKey::Left | ControlKey::Right)
                {
                    let column = focus.unwrap_or(20).saturating_sub(20);
                    let next = if key == ControlKey::Right {
                        (column + 1).min(ext.table.columns.saturating_sub(1))
                    } else {
                        column.saturating_sub(1)
                    };
                    ext.focused = Some(20 + next);
                } else {
                    ext.focused = ext.table.selected_row.map(|index| 30 + index);
                }
            }
            "chart" => {
                ext.selected = ext.chart.selected().unwrap_or_default();
                ext.focused = ext.chart.selected().map(|index| 30 + index);
            }
            _ => {}
        }
        log(format!("{renderer}: keyboard state changed"));
    }
    result
}

pub(super) fn pointer_move(
    ext: &mut ExtendedStory,
    renderer: &str,
    point: Point,
    _hit: Option<usize>,
    _log: &mut impl FnMut(String),
) -> Option<InputResult> {
    let changed = match renderer {
        "list" => ext.collections_views.list.pointer_move(point),
        "tree" => ext.collections_views.tree.pointer_move(point),
        "data-table" => ext.collections_views.table.pointer_move(point),
        "chart" => ext
            .collections_views
            .chart
            .pointer_move(point, &mut ext.chart),
        _ => return None,
    };
    Some(InputResult {
        consumed: true,
        changed,
    })
}

pub(super) fn pointer_down(
    ext: &mut ExtendedStory,
    renderer: &str,
    point: Point,
    _id: Option<usize>,
    log: &mut impl FnMut(String),
) -> Option<InputResult> {
    let event = match renderer {
        "list" => ext
            .collections_views
            .list
            .pointer_down(point, &mut ext.list),
        "tree" => ext
            .collections_views
            .tree
            .pointer_down(point, &mut ext.tree),
        "data-table" => ext
            .collections_views
            .table
            .pointer_down(point, &mut ext.table),
        "chart" => ext
            .collections_views
            .chart
            .pointer_down(point, &mut ext.chart),
        _ => return None,
    };
    let result = result(event, renderer, log);
    if result.is_some() {
        ext.focused = _id;
    }
    if result.is_some_and(|r| r.changed) {
        match renderer {
            "list" => ext.selected = ext.list.selected.unwrap_or_default(),
            "tree" => ext.selected = ext.tree.selected.unwrap_or_default(),
            "data-table" => ext.selected = ext.table.selected_row.unwrap_or_default(),
            "chart" => ext.selected = ext.chart.selected().unwrap_or_default(),
            _ => {}
        }
        if !matches!(renderer, "tree") {
            log(format!("{renderer}: selection changed"));
        }
    }
    result
}

pub(super) fn pointer_up(
    ext: &mut ExtendedStory,
    renderer: &str,
    point: Point,
    _hit: Option<usize>,
    log: &mut impl FnMut(String),
) -> Option<InputResult> {
    let event = match renderer {
        "list" => ext.collections_views.list.pointer_up(point, &mut ext.list),
        "tree" => ext.collections_views.tree.pointer_up(point, &mut ext.tree),
        "data-table" => ext
            .collections_views
            .table
            .pointer_up(point, &mut ext.table),
        "chart" => {
            return Some(InputResult {
                consumed: true,
                changed: false,
            });
        }
        _ => return None,
    };
    let result = result(event, renderer, log);
    if result.is_some_and(|r| r.changed) {
        match renderer {
            "list" => ext.selected = ext.list.selected.unwrap_or_default(),
            "tree" => ext.selected = ext.tree.selected.unwrap_or_default(),
            "data-table" => ext.selected = ext.table.selected_row.unwrap_or_default(),
            _ => {}
        }
    }
    result
}

pub(super) fn focus_changed(
    ext: &mut ExtendedStory,
    renderer: &str,
    focus: Option<usize>,
    keyboard: bool,
) {
    match renderer {
        "list" => {
            if let Some(index) = focus
                .and_then(|id| id.checked_sub(30))
                .filter(|i| *i < ext.list.count)
            {
                ext.collections_views.list.focus(index, keyboard);
            } else {
                ext.collections_views.list.blur();
            }
        }
        "tree" => {
            if let Some(index) = focus.and_then(|id| id.checked_sub(30)).filter(|i| {
                ext.tree.visible().contains(i)
                    && ext.tree.nodes().get(*i).is_some_and(|node| !node.disabled)
            }) {
                ext.collections_views.tree.focus(index, keyboard);
            } else {
                ext.collections_views.tree.blur();
            }
        }
        "data-table" => match focus {
            Some(id @ 20..=22) => ext.collections_views.table.focus_header(id - 20, keyboard),
            Some(id @ 30..=34) if id - 30 < ext.table.rows => {
                ext.collections_views.table.focus_row(id - 30, keyboard)
            }
            _ => ext.collections_views.table.blur(),
        },
        "chart" => {
            if focus.is_some_and(|id| (30..34).contains(&id)) {
                ext.collections_views.chart.focus(keyboard, &mut ext.chart);
            } else {
                ext.collections_views.chart.blur(&mut ext.chart);
            }
        }
        _ => {}
    }
}

pub(super) fn blur(ext: &mut ExtendedStory, renderer: &str) {
    focus_changed(ext, renderer, None, false);
}
pub(super) fn window_active(ext: &mut ExtendedStory, renderer: &str, active: bool) {
    match renderer {
        "list" => ext.collections_views.list.set_window_active(active),
        "tree" => ext.collections_views.tree.set_window_active(active),
        "data-table" => ext.collections_views.table.set_window_active(active),
        "chart" => ext
            .collections_views
            .chart
            .set_window_active(active, &mut ext.chart),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gallery::{static_text::StaticText, text::TextEngine};
    use tekes_ui_controls::{
        collections_view::{ListView, TreeView},
        extended::{CodeBlock, DiffView, List, Markdown, Tree},
    };

    fn assert_note_follows_rows(note: Rect, rows: &[Rect], bounds: Rect) {
        let bottom = rows
            .iter()
            .map(|row| row.max_y())
            .fold(bounds.min_y(), f32::max);
        assert!(
            note.min_y() >= bottom + 3.9,
            "note must start after measured rows: {note:?}, rows={rows:?}"
        );
        assert!(note.max_y() <= bounds.max_y());
        assert!(rows.iter().all(|row| row.intersection(&note).is_none()));
    }

    #[test]
    fn list_and_tree_instructions_follow_real_large_text_layout() {
        let mut text = TextEngine::new();
        text.set_type_scale(1.5);
        let mut static_text = StaticText::new(None);
        let mut ctx = PageCtx {
            theme: tekes_ui_tokens::Theme::LIGHT,
            text: &mut text,
            static_text: &mut static_text,
        };
        let mut scene = Scene::new();
        let bounds = Rect::new(120.0, 80.0, 188.0, 420.0);
        let mut list_view = ListView::new();
        let list = List::new(5);
        let labels = ["Overview", "Documents", "Images", "Archive", "Trash"]
            .into_iter()
            .map(str::to_owned)
            .collect::<Vec<_>>();
        let list_layout = list_view.render_scaled(
            &mut scene,
            ctx.text,
            &list,
            &labels,
            bounds,
            Some(bounds),
            Palette::from(ctx.theme),
            1.5,
        );
        let list_rows = list_layout
            .items
            .iter()
            .map(|row| row.frame)
            .collect::<Vec<_>>();
        let list_note_text = "Single selection. Up / Down move; Home / End jump to first / last.";
        let list_note_start = scene.len();
        let list_note =
            place_instruction_below_rows(&mut ctx, &mut scene, bounds, &list_rows, list_note_text)
                .expect("the roomy viewport has space after List rows");
        assert_note_follows_rows(list_note, &list_rows, bounds);
        assert_wrapped_caption_glyph_lines(&scene, list_note_start, list_note);

        let mut tree = Tree::default();
        let root = tree.add("Workspace", None).unwrap();
        tree.add("Documents", Some(root)).unwrap();
        tree.add("Images", Some(root)).unwrap();
        tree.toggle(root);
        let mut tree_view = TreeView::new();
        let tree_layout = tree_view.render(
            &mut scene,
            ctx.text,
            &tree,
            bounds,
            Some(bounds),
            Palette::from(ctx.theme),
        );
        let tree_rows = tree_layout
            .items
            .iter()
            .map(|row| row.frame)
            .collect::<Vec<_>>();
        let tree_note = place_instruction_below_rows(
            &mut ctx,
            &mut scene,
            bounds,
            &tree_rows,
            "Right expands; Left collapses or moves to the parent. Up / Down navigate visible rows.",
        ).expect("the roomy viewport has space after Tree rows");
        assert_note_follows_rows(tree_note, &tree_rows, bounds);

        let too_short = Rect::new(bounds.min_x(), bounds.min_y(), bounds.size.width, 80.0);
        assert!(
            place_instruction_below_rows(&mut ctx, &mut scene, too_short, &list_rows, "List help")
                .is_none()
        );
    }

    #[test]
    fn document_footers_wrap_inside_narrow_viewports_at_default_and_large_text() {
        for scale in [1.0, 1.5] {
            let mut text = TextEngine::new();
            text.set_type_scale(scale);
            let mut static_text = StaticText::new(None);
            let mut ctx = PageCtx {
                theme: tekes_ui_tokens::Theme::LIGHT,
                text: &mut text,
                static_text: &mut static_text,
            };
            let body = Rect::new(24.0, 32.0, 172.0, 760.0);
            let clip = body;

            let mut markdown_view = MarkdownView::new();
            let markdown = Markdown::parse(
                "# Heading\n\nA wrapped paragraph with enough content to occupy several lines in this narrow story container.\n\n- One realistic item that wraps\n- Another item that wraps",
            );
            let markdown_layout = markdown_view.render(
                &mut Scene::new(),
                ctx.text,
                &markdown,
                body,
                Some(clip),
                Palette::from(ctx.theme),
                scale,
            );
            let markdown_footer =
                "Supported subset: headings, paragraphs, bullets and fenced code.";
            let markdown_height = ctx
                .text
                .proposal_label(
                    markdown_footer,
                    tekes_ui_tokens::proposal::TextRole::UiCaption,
                    Some(body.size.width),
                )
                .height as f32;
            let markdown_y = (body.max_y() - markdown_height - 4.0)
                .max(body.min_y() + markdown_layout.content_height + 12.0);
            let mut markdown_scene = Scene::new();
            let start = markdown_scene.len();
            let markdown_slot = draw_measured_footer(
                &mut ctx,
                &mut markdown_scene,
                clip,
                body.min_x(),
                markdown_y,
                body.size.width,
                markdown_footer,
            )
            .expect("Markdown footer fits after its static fixture");
            assert!(markdown_slot.min_y() >= body.min_y() + markdown_layout.content_height + 12.0);
            assert_eq!(markdown_slot.size.height, markdown_height);
            assert_wrapped_caption_glyph_lines(&markdown_scene, start, markdown_slot);

            let code_frame = Rect::new(body.min_x(), body.min_y() + 8.0, body.size.width, 620.0);
            let code = CodeBlock {
                language: "text".into(),
                source: "first explanatory source line\nsecond source line\nthird source line"
                    .into(),
            };
            let mut code_view = CodeBlockView::new();
            let mut code_scene = Scene::new();
            let code_layout = code_view.render(
                &mut code_scene,
                ctx.text,
                &code,
                code_frame,
                Some(clip),
                Palette::from(ctx.theme),
                scale,
            );
            let code_footer =
                "Plain light surface; syntax coloring and editing are not part of this component.";
            let code_footer_x = code_frame.min_x() + 14.0;
            let code_footer_width = code_frame.size.width - 28.0;
            let code_footer_height = ctx
                .text
                .proposal_label(
                    code_footer,
                    tekes_ui_tokens::proposal::TextRole::UiCaption,
                    Some(code_footer_width),
                )
                .height as f32;
            let code_content_bottom = code_layout
                .blocks
                .iter()
                .map(|block| block.frame.max_y())
                .fold(code_frame.min_y(), f32::max);
            let code_footer_y =
                (code_frame.max_y() - 14.0 - code_footer_height).max(code_content_bottom + 12.0);
            let code_start = code_scene.len();
            let code_slot = draw_measured_footer(
                &mut ctx,
                &mut code_scene,
                clip,
                code_footer_x,
                code_footer_y,
                code_footer_width,
                code_footer,
            )
            .expect("CodeBlock footer fits in the inset panel");
            assert!(code_slot.max_y() <= code_frame.max_y() - 14.0);
            assert!(code_slot.min_y() >= code_content_bottom + 12.0);
            assert_eq!(code_slot.size.height, code_footer_height);
            assert_wrapped_caption_glyph_lines(&code_scene, code_start, code_slot);

            let diff_frame = Rect::new(body.min_x(), body.min_y(), body.size.width, 600.0);
            let diff = DiffView::between(
                "alpha line\nbeta line\ngamma line",
                "alpha line\nchanged beta line\ngamma line\ndelta line",
            );
            let mut diff_view = DiffViewRuntime::new();
            let mut diff_scene = Scene::new();
            let diff_layout = diff_view.render(
                &mut diff_scene,
                ctx.text,
                &diff,
                diff_frame,
                Some(clip),
                Palette::from(ctx.theme),
                scale,
            );
            let diff_footer = "Deterministic line-based LCS fixture; large inputs fall back to delete/add blocks.";
            let diff_content_bottom = diff_layout
                .blocks
                .iter()
                .map(|block| block.frame.max_y())
                .fold(diff_frame.min_y(), f32::max);
            let diff_y = diff_content_bottom + 12.0 + 2.0 * scale;
            let diff_start = diff_scene.len();
            let diff_slot = draw_measured_footer(
                &mut ctx,
                &mut diff_scene,
                clip,
                body.min_x(),
                diff_y,
                body.size.width,
                diff_footer,
            )
            .expect("Diff footer fits below its static rows");
            assert!(diff_slot.min_y() >= diff_content_bottom + 12.0 + 2.0 * scale);
            assert_eq!(
                diff_slot.size.height,
                ctx.text
                    .proposal_label(
                        diff_footer,
                        tekes_ui_tokens::proposal::TextRole::UiCaption,
                        Some(body.size.width),
                    )
                    .height as f32
            );
            assert_wrapped_caption_glyph_lines(&diff_scene, diff_start, diff_slot);
        }
    }

    fn assert_wrapped_caption_glyph_lines(scene: &Scene, start: usize, slot: Rect) {
        let baselines = scene.primitives()[start..]
            .iter()
            .filter_map(|primitive| match primitive {
                tekes_ui_scene::Primitive::Glyphs(run) => Some(
                    run.glyphs
                        .iter()
                        .map(|glyph| (glyph.origin.y * 64.0).round() as i32),
                ),
                _ => None,
            })
            .flatten()
            .collect::<std::collections::BTreeSet<_>>();
        assert!(
            baselines.len() > 1,
            "narrow caption should produce glyphs on multiple wrapped lines: {baselines:?}"
        );
        let min_baseline = baselines.first().copied().unwrap() as f32 / 64.0;
        let max_baseline = baselines.last().copied().unwrap() as f32 / 64.0;
        assert!(min_baseline >= slot.min_y());
        assert!(max_baseline < slot.max_y());
    }
}
