//! Gallery adapters for the collection and document components.
//!
//! The Gallery supplies deterministic example data and the available viewport.
//! Component geometry, clipping, paint and input remain in the production views.
use super::super::super::pages::icons::GalleryIconRenderer;
use super::super::{Entry, PageCtx, StoryState, line};
use super::{ExtendedStory, controls};
use tekes_ui_controls::{
    BarChart, ButtonTextRenderer, ChartDatum, ControlKey,
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
    state.hits.retain(|(id, _)| !component_id(renderer, *id));
    state
        .logical_focus
        .retain(|target| !component_id(renderer, target.id));
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
            let note = if renderer == "list" {
                "Single selection. Up / Down move; Home / End jump to first / last."
            } else {
                "Right expands; Left collapses or moves to the parent. Up / Down navigate visible rows."
            };
            let note_height = ctx
                .text
                .proposal_label(
                    note,
                    tekes_ui_tokens::proposal::TextRole::UiCaption,
                    Some(width),
                )
                .height as f32;
            state.document_bottom =
                row_frames.iter().map(|r| r.max_y()).fold(y, f32::max) + 4. + note_height;
            for item in layout.items {
                state
                    .button_labels
                    .insert(30 + item.id, labels[item.id].clone());
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
            let note = if renderer == "list" {
                "Single selection. Up / Down move; Home / End jump to first / last."
            } else {
                "Right expands; Left collapses or moves to the parent. Up / Down navigate visible rows."
            };
            let note_height = ctx
                .text
                .proposal_label(
                    note,
                    tekes_ui_tokens::proposal::TextRole::UiCaption,
                    Some(width),
                )
                .height as f32;
            state.document_bottom =
                row_frames.iter().map(|r| r.max_y()).fold(y, f32::max) + 4. + note_height;
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
            state.document_bottom = layout
                .rows
                .iter()
                .chain(&layout.headers)
                .map(|item| item.frame.max_y())
                .fold(y, f32::max);
            for header in layout.headers {
                state
                    .button_labels
                    .insert(20 + header.id, headers[header.id].clone());
                push_hit(state, 20 + header.id, header.visible);
            }
            for row in layout.rows {
                state
                    .button_labels
                    .insert(30 + row.id, rows[row.id].join(" · "));
                for (column, value) in rows[row.id].iter().enumerate() {
                    state
                        .button_labels
                        .insert(2000 + row.id * ext.table.columns + column, value.clone());
                }
                push_hit(state, 30 + row.id, row.visible);
            }
        }
        "chart" => {
            let frame = Rect::new(
                x,
                y + 8.0,
                width.min(760.0),
                (body.size.height - 36.0).clamp(0.0, 320.0),
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
            state.document_bottom = frame.max_y() + 26.;
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
            let note_y = frame.min_y() + layout.content_height + 12.0;
            state.document_bottom = frame.min_y() + layout.content_height + 12. + note_height;
            draw_measured_footer(ctx, scene, clip, note_x, note_y, note_width, note);
        }
        "code-block" => {
            let inset = tekes_ui_tokens::proposal::metric("spacing", "space-button-md-inset")
                .unwrap_or(14.);
            let code_role = tekes_ui_tokens::proposal::TextRole::Code;
            // Match the production CodeBlockView's text slot: 14pt on both
            // sides, capped at the established 650pt line width.
            let code_width = (width.min(680.0) - inset * 2.0).clamp(0., 650.);
            let natural_height = ext
                .code
                .lines()
                .map(|line| {
                    (ctx.text
                        .proposal_label(line, code_role, Some(code_width))
                        .height as f32)
                        .max(code_role.style().line_height * text_scale)
                })
                .sum::<f32>()
                + inset * 2.;
            let frame = Rect::new(x, y + 8.0, width.min(680.0), natural_height.max(80.));
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
            let footer_y = frame.max_y().max(content_bottom) + 12.0;
            state.document_bottom = footer_y + footer_height;
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
            let footer_height = ctx
                .text
                .proposal_label(
                    footer,
                    tekes_ui_tokens::proposal::TextRole::UiCaption,
                    Some(footer_width),
                )
                .height as f32;
            state.document_bottom = footer_y + footer_height;
            draw_measured_footer(ctx, scene, clip, footer_x, footer_y, footer_width, footer);
        }
        _ => return false,
    }
    register_collection_logical_focus(state, ext, renderer);
    true
}

/// Collection components with a meaningful visual-state comparison.
pub(super) fn supports_states(renderer: &str) -> bool {
    matches!(renderer, "list" | "tree" | "data-table" | "chart")
}

/// Draw state specimens using fresh production views and fixed local models.
/// This deliberately does not write Gallery hit targets or touch its Playground
/// model, selection, focus, scroll position, or event log.
pub(super) fn matrix(
    e: &Entry,
    s: &mut StoryState,
    ctx: &mut PageCtx<'_>,
    scene: &mut Scene,
    viewport: Rect,
) {
    if !supports_states(e.renderer()) || viewport.size.width <= 0.0 || viewport.size.height <= 0.0 {
        return;
    }
    scene.push_clip(viewport);
    let gutter = -tekes_ui_controls::extended::ControlFrame::new(Rect::new(0., 0., 1., 1.), None)
        .focus_bounds(ctx.text.backing_scale())
        .min_x();
    let bounds = viewport.inset(gutter);
    let x = bounds.min_x();
    let width = bounds.size.width.max(0.0);
    let mut y = bounds.min_y();
    let help = "Production view specimens with fixed data. Use Playground for pointer and keyboard interaction.";
    draw_matrix_caption(ctx, scene, x, y, width, help);
    y += ctx
        .text
        .proposal_label(
            help,
            tekes_ui_tokens::proposal::TextRole::UiCaption,
            Some(width),
        )
        .height as f32
        + 18.0;
    let palette = Palette {
        theme: ctx.theme,
        brand: s.brand,
    };
    let scale = ctx.text.type_scale();
    match e.renderer() {
        "list" => matrix_list(ctx, scene, bounds, viewport, &mut y, palette, scale),
        "tree" => matrix_tree(ctx, scene, bounds, viewport, &mut y, width, palette),
        "data-table" => matrix_table(ctx, scene, bounds, viewport, &mut y, width, palette),
        "chart" => matrix_chart(ctx, scene, bounds, viewport, &mut y, palette, scale),
        _ => {}
    }
    s.document_bottom = y + gutter;
    scene.pop_clip();
}

fn draw_matrix_caption(
    ctx: &mut PageCtx<'_>,
    scene: &mut Scene,
    x: f32,
    y: f32,
    w: f32,
    text: &str,
) {
    let role = tekes_ui_tokens::proposal::TextRole::UiCaption;
    let measured = ctx.text.proposal_label(text, role, Some(w.max(0.)));
    ctx.text.proposal_wrapped_in(
        scene,
        ctx.theme,
        Rect::new(x, y, w.max(0.), measured.height as f32),
        text,
        role,
        tekes_ui_tokens::ColorRole::SecondaryText.resolve(ctx.theme),
    );
}

fn matrix_heading(
    ctx: &mut PageCtx<'_>,
    scene: &mut Scene,
    x: f32,
    y: f32,
    w: f32,
    title: &str,
) -> f32 {
    let role = tekes_ui_tokens::proposal::TextRole::UiBody;
    let measured = ctx.text.proposal_label(title, role, Some(w.max(0.)));
    ctx.text.proposal_wrapped_in(
        scene,
        ctx.theme,
        Rect::new(x, y, w.max(0.), measured.height as f32),
        title,
        role,
        tekes_ui_tokens::ColorRole::PrimaryText.resolve(ctx.theme),
    );
    measured.height as f32
}

fn matrix_list(
    ctx: &mut PageCtx<'_>,
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
        let heading = matrix_heading(ctx, scene, bounds.min_x(), *y, width, title);
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
            ctx.text,
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
            ctx.text,
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

fn matrix_tree(
    ctx: &mut PageCtx<'_>,
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
        let heading = matrix_heading(ctx, scene, bounds.min_x(), *y, width, title);
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
        let layout = view.render(&mut probe, ctx.text, &model, frame, Some(clip), palette);
        if let Some(index) = hovered
            && let Some(item) = layout.items.iter().find(|item| item.id == index)
        {
            view.pointer_move(item.frame.center());
        }
        let actual = view.render(scene, ctx.text, &model, frame, Some(clip), palette);
        let mut icons = GalleryIconRenderer::native(ctx);
        view.render_icons(scene, &mut icons, Some(clip), palette, 16.0);
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

fn matrix_table(
    ctx: &mut PageCtx<'_>,
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
        let heading = matrix_heading(ctx, scene, bounds.min_x(), *y, width, title);
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
            ctx.text,
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
            ctx.text,
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

fn matrix_chart(
    ctx: &mut PageCtx<'_>,
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
        let heading = matrix_heading(ctx, scene, bounds.min_x(), *y, width, title);
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
            ctx.text,
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
        view.render(
            scene,
            ctx.text,
            &mut model,
            frame,
            Some(clip),
            palette,
            scale,
        );
        *y = frame.max_y() + 18.0;
    }
}

/// Full document-space collection frames from the most recent production
/// layout. The `visible` clip is intentionally not applied so Tab can reveal a
/// target that currently sits outside the nested scroll viewport.
pub(super) fn logical_focus_targets(ext: &ExtendedStory, renderer: &str) -> Vec<(usize, Rect)> {
    match renderer {
        "list" => ext
            .collections_views
            .list
            .layout()
            .items
            .iter()
            .map(|item| (30 + item.id, item.frame))
            .collect(),
        "tree" => ext
            .collections_views
            .tree
            .layout()
            .items
            .iter()
            .filter(|item| {
                ext.tree
                    .nodes()
                    .get(item.id)
                    .is_some_and(|node| !node.disabled)
            })
            .map(|item| (30 + item.id, item.frame))
            .collect(),
        "data-table" => {
            let layout = ext.collections_views.table.layout();
            layout
                .headers
                .iter()
                .map(|item| (20 + item.id, item.frame))
                .chain(layout.rows.iter().map(|item| (30 + item.id, item.frame)))
                .collect()
        }
        "chart" => ext
            .collections_views
            .chart
            .layout()
            .geometry
            .bars
            .iter()
            .map(|bar| (30 + bar.index, bar.rect.inset(-6.0)))
            .collect(),
        _ => Vec::new(),
    }
}

fn register_collection_logical_focus(state: &mut StoryState, ext: &ExtendedStory, renderer: &str) {
    let targets = logical_focus_targets(ext, renderer);
    let has_target = |id| targets.iter().any(|(target, _)| *target == id);
    let tree_selection = ext
        .focused
        .filter(|id| *id >= 30)
        .map(|id| id - 30)
        .filter(|id| ext.tree.visible().contains(id) && !ext.tree.nodes()[*id].disabled)
        .or(ext.tree.selected)
        .filter(|id| ext.tree.visible().contains(id) && !ext.tree.nodes()[*id].disabled)
        .or_else(|| {
            ext.tree
                .visible()
                .into_iter()
                .find(|id| !ext.tree.nodes()[*id].disabled)
        });
    let table_header = ext
        .focused
        .filter(|id| (20..23).contains(id) && has_target(*id))
        .or_else(|| {
            ext.table
                .sort
                .map(|(column, _)| 20 + column)
                .filter(|id| has_target(*id))
        })
        .unwrap_or(20);
    let table_row = ext
        .focused
        .filter(|id| *id >= 30 && has_target(*id))
        .or_else(|| {
            ext.table
                .selected_row
                .map(|row| 30 + row)
                .filter(|id| has_target(*id))
        })
        .or_else(|| targets.iter().find(|(id, _)| *id >= 30).map(|(id, _)| *id));
    let roving_id = match renderer {
        "list" => ext
            .focused
            .filter(|id| *id >= 30)
            .filter(|id| has_target(*id))
            .or_else(|| {
                ext.list
                    .selected
                    .map(|index| 30 + index)
                    .filter(|id| has_target(*id))
            })
            .or_else(|| targets.first().map(|(id, _)| *id)),
        "tree" => tree_selection.map(|id| 30 + id),
        "data-table" => None,
        "chart" => ext
            .focused
            .filter(|id| *id >= 30)
            .filter(|id| has_target(*id))
            .or_else(|| {
                ext.chart
                    .selected()
                    .map(|index| 30 + index)
                    .filter(|id| has_target(*id))
            })
            .or_else(|| targets.first().map(|(id, _)| *id)),
        _ => None,
    };
    for (id, frame) in targets {
        let eligible = !ext.disabled
            && match renderer {
                "tree" => ext
                    .tree
                    .nodes()
                    .get(id.saturating_sub(30))
                    .is_some_and(|node| !node.disabled),
                _ => true,
            };
        let (group, selected) = match renderer {
            "data-table" if id < 30 => (Some(3), id == table_header),
            "data-table" => (Some(4), Some(id) == table_row),
            _ => (Some(3), Some(id) == roving_id),
        };
        state.register_logical_focus(id, frame, eligible, true, group, selected);
    }
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
            // Owned cancellation must reach the View even when a model update
            // concealed/disabled the old focused row before the next paint.
            if key != ControlKey::Escape
                && (!ext.tree.visible().contains(&id)
                    || ext.tree.nodes().get(id).is_none_or(|node| node.disabled))
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

/// Logical focus geometry remains available when a row is outside the
/// ancestor clip; the Gallery scroll viewport reveals it after keyboard input.
fn valid_logical_frame(frame: Rect) -> bool {
    [frame.min_x(), frame.min_y(), frame.max_x(), frame.max_y()]
        .into_iter()
        .all(f32::is_finite)
        && frame.size.width > 0.
        && frame.size.height > 0.
}

pub(super) fn focus_frame(ext: &ExtendedStory, renderer: &str, id: usize) -> Option<Rect> {
    match renderer {
        "list" | "tree" => {
            let layout = if renderer == "list" {
                ext.collections_views.list.layout()
            } else {
                ext.collections_views.tree.layout()
            };
            if !valid_logical_frame(layout.frame) {
                return None;
            }
            layout
                .items
                .iter()
                .find(|item| item.id + 30 == id)
                .map(|item| item.frame)
        }
        "data-table" => {
            let layout = ext.collections_views.table.layout();
            if !valid_logical_frame(layout.frame) {
                return None;
            }
            let (items, offset) = if id < 30 {
                (&layout.headers, 20)
            } else {
                (&layout.rows, 30)
            };
            items
                .iter()
                .find(|item| item.id + offset == id)
                .map(|item| item.frame)
        }
        "chart" => {
            let layout = ext.collections_views.chart.layout();
            if !valid_logical_frame(layout.frame) {
                return None;
            }
            layout
                .geometry
                .bars
                .iter()
                .find(|bar| bar.index + 30 == id)
                .map(|bar| {
                    if bar.rect.is_empty() {
                        bar.category_label
                    } else {
                        bar.rect
                    }
                })
        }
        _ => None,
    }
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
    fn collection_state_matrices_render_at_narrow_150_percent_text_and_keep_story_isolated() {
        let mut renderer = tekes_ui_render::Renderer::new_headless()
            .expect("headless GPU is required for collection state visual fixtures");
        for renderer_name in ["list", "tree", "data-table", "chart"] {
            let entry = crate::gallery::workbench::entries()
                .iter()
                .find(|entry| entry.renderer() == renderer_name)
                .expect("registered collection story");
            let mut state = StoryState::default();
            state.hits.push((777, Rect::new(1., 2., 3., 4.)));
            let playground_models = (
                state.extended.list.clone(),
                state.extended.tree.clone(),
                state.extended.table.clone(),
                state.extended.chart.clone(),
            );
            let mut text = TextEngine::new();
            text.set_scale(1.0);
            text.set_type_scale(1.5);
            text.begin_frame();
            assert_eq!(
                text.backing_scale(),
                1.0,
                "fixture is in logical points at backing scale 1"
            );
            let mut static_text = StaticText::new(None);
            let mut ctx = PageCtx {
                theme: tekes_ui_tokens::Theme::LIGHT,
                text: &mut text,
                static_text: &mut static_text,
            };
            let bounds = Rect::new(0.0, 0.0, 360.0, 5_000.0);
            let mut scene = Scene::new();
            matrix(entry, &mut state, &mut ctx, &mut scene, bounds);
            assert!(state.document_bottom > bounds.min_y());
            assert_eq!(
                state.hits,
                vec![(777, Rect::new(1., 2., 3., 4.))],
                "state specimens do not register real hits"
            );
            assert_eq!(
                (
                    state.extended.list.clone(),
                    state.extended.tree.clone(),
                    state.extended.table.clone(),
                    state.extended.chart.clone(),
                ),
                playground_models,
                "state specimens must not mutate Playground models",
            );
            assert!(!scene.is_empty());
            let image_height = state.document_bottom.ceil().max(1.0) as u32;
            let rgba = renderer
                .render_to_rgba8(
                    &scene,
                    ctx.text.atlas_mut(),
                    (360, image_height),
                    1.0,
                    crate::gallery::Gallery::background(ctx.theme),
                )
                .expect("render actual production collection views");
            if let Some(output) = std::env::var_os("TEKESUI_GOLD7_PAGE_OUT") {
                let output = std::path::PathBuf::from(output);
                std::fs::create_dir_all(&output).unwrap();
                let file = std::fs::File::create(output.join(format!(
                    "collection-states-{renderer_name}-360pt-text150.png"
                )))
                .unwrap();
                let mut encoder = png::Encoder::new(file, 360, image_height);
                encoder.set_color(png::ColorType::Rgba);
                encoder.set_depth(png::BitDepth::Eight);
                encoder
                    .write_header()
                    .unwrap()
                    .write_image_data(&rgba)
                    .unwrap();
            }
        }
    }

    #[test]
    fn collection_focus_registration_keeps_full_unclipped_row_geometry_in_reading_order() {
        let entry = crate::gallery::workbench::entries()
            .iter()
            .find(|entry| entry.renderer() == "list")
            .expect("registered List story");
        let mut ext = ExtendedStory::default();
        let mut state = StoryState::default();
        let mut text = TextEngine::new();
        text.set_scale(1.0);
        text.set_type_scale(1.5);
        let mut static_text = StaticText::new(None);
        let mut ctx = PageCtx {
            theme: tekes_ui_tokens::Theme::LIGHT,
            text: &mut text,
            static_text: &mut static_text,
        };
        let body = Rect::new(0.0, 0.0, 360.0, 54.0);
        let mut scene = Scene::new();
        let palette = Palette::from(ctx.theme);
        assert!(build(
            &mut ext, entry, &mut state, &mut ctx, &mut scene, body, body, palette,
        ));
        let targets = logical_focus_targets(&ext, "list");
        assert_eq!(targets.len(), ext.list.count);
        assert!(
            targets
                .windows(2)
                .all(|pair| pair[0].1.min_y() < pair[1].1.min_y())
        );
        assert!(
            targets
                .iter()
                .any(|(_, frame)| frame.max_y() > body.max_y())
        );
        assert!(
            targets
                .iter()
                .all(|(_, frame)| frame.min_x() == body.min_x())
        );
        assert!(state.logical_focus.iter().all(|target| target.mounted));
        assert!(
            state
                .logical_focus
                .iter()
                .any(|target| target.frame.max_y() > body.max_y())
        );
        assert!(
            state.hits.len() < targets.len(),
            "clipped-out rows must not become pointer hits"
        );
        assert!(
            state.document_bottom > body.max_y(),
            "natural document extent remains available to the host scroll area"
        );
    }

    #[test]
    fn data_table_exposes_roving_header_and_row_groups_with_real_keyboard_contract() {
        let entry = crate::gallery::workbench::entries()
            .iter()
            .find(|entry| entry.renderer() == "data-table")
            .expect("registered DataTable story");
        let mut ext = ExtendedStory::default();
        let mut state = StoryState::default();
        let mut text = TextEngine::new();
        text.set_scale(1.0);
        text.set_type_scale(1.5);
        let mut static_text = StaticText::new(None);
        let mut ctx = PageCtx {
            theme: tekes_ui_tokens::Theme::LIGHT,
            text: &mut text,
            static_text: &mut static_text,
        };
        let body = Rect::new(0.0, 0.0, 360.0, 120.0);
        let mut scene = Scene::new();
        let palette = Palette::from(ctx.theme);
        assert!(build(
            &mut ext, entry, &mut state, &mut ctx, &mut scene, body, body, palette,
        ));
        let header_targets = state
            .logical_focus
            .iter()
            .filter(|target| target.roving_group == Some(3))
            .collect::<Vec<_>>();
        let row_targets = state
            .logical_focus
            .iter()
            .filter(|target| target.roving_group == Some(4))
            .collect::<Vec<_>>();
        assert_eq!(
            header_targets
                .iter()
                .filter(|target| target.roving_selected)
                .count(),
            1
        );
        assert_eq!(
            row_targets
                .iter()
                .filter(|target| target.roving_selected)
                .count(),
            1
        );
        assert_eq!(
            header_targets
                .iter()
                .map(|target| target.id)
                .collect::<Vec<_>>(),
            [20, 21, 22]
        );
        assert_eq!(
            row_targets
                .iter()
                .map(|target| target.id)
                .collect::<Vec<_>>(),
            [30, 31, 32, 33, 34]
        );

        // Production DataTable keyboard behavior: Left/Right roves sortable
        // headers; Down/Up is available after Tab enters the separate row group.
        focus_changed(&mut ext, "data-table", Some(20), true);
        let mut log = |_| {};
        assert!(
            key(
                &mut ext,
                "data-table",
                Some(20),
                ControlKey::Right,
                true,
                &mut log
            )
            .is_some_and(|r| r.changed)
        );
        assert_eq!(ext.focused, Some(21));
        focus_changed(&mut ext, "data-table", Some(30), true);
        assert!(
            key(
                &mut ext,
                "data-table",
                Some(30),
                ControlKey::Down,
                true,
                &mut log
            )
            .is_some_and(|r| r.changed)
        );
        assert_eq!(ext.table.selected_row, Some(1));
        assert_eq!(ext.focused, Some(31));
    }

    #[test]
    fn tree_roving_tab_stop_falls_back_when_selection_becomes_hidden() {
        let entry = crate::gallery::workbench::entries()
            .iter()
            .find(|entry| entry.renderer() == "tree")
            .expect("registered Tree story");
        let mut ext = ExtendedStory::default();
        assert!(ext.tree.select(1));
        assert!(ext.tree.toggle(0)); // Hide the selected child by collapsing its parent.
        let mut state = StoryState::default();
        let mut text = TextEngine::new();
        text.set_scale(1.0);
        text.set_type_scale(1.5);
        let mut static_text = StaticText::new(None);
        let mut ctx = PageCtx {
            theme: tekes_ui_tokens::Theme::LIGHT,
            text: &mut text,
            static_text: &mut static_text,
        };
        let body = Rect::new(0.0, 0.0, 360.0, 100.0);
        let mut scene = Scene::new();
        let palette = Palette::from(ctx.theme);
        assert!(build(
            &mut ext, entry, &mut state, &mut ctx, &mut scene, body, body, palette,
        ));
        let tree_targets = state
            .logical_focus
            .iter()
            .filter(|target| target.roving_group == Some(3) && target.eligible)
            .collect::<Vec<_>>();
        assert_eq!(tree_targets.len(), 1);
        assert_eq!(
            tree_targets[0].id, 30,
            "collapsed selection falls back to the visible root"
        );
        assert!(tree_targets[0].roving_selected);
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

    #[test]
    fn gallery_code_block_height_uses_its_real_narrow_large_text_slot() {
        let entry = crate::gallery::workbench::entries()
            .iter()
            .find(|entry| entry.renderer() == "code-block")
            .expect("registered CodeBlock");
        let mut ext = ExtendedStory::default();
        ext.code.source =
            "a long source line that must wrap within the padded narrow code surface".into();
        let mut state = StoryState::default();
        let mut text = TextEngine::new();
        text.set_scale(1.0);
        text.set_type_scale(1.5);
        text.begin_frame();
        let mut static_text = StaticText::new(None);
        let mut ctx = PageCtx {
            theme: tekes_ui_tokens::Theme::LIGHT,
            text: &mut text,
            static_text: &mut static_text,
        };
        let body = Rect::new(24.0, 32.0, 172.0, 700.0);
        let mut scene = Scene::new();
        let palette = Palette::from(ctx.theme);
        assert!(build(
            &mut ext, entry, &mut state, &mut ctx, &mut scene, body, body, palette,
        ));
        let layout = ext.collections_views.code.layout();
        assert_eq!(layout.frame.size.width, body.size.width);
        assert!(layout.blocks[0].frame.min_x() >= body.min_x() + 14.0);
        assert!(layout.blocks[0].frame.max_x() <= body.max_x() - 14.0);
        assert!(
            layout.blocks[0].frame.size.height
                > tekes_ui_tokens::proposal::TextRole::Code
                    .style()
                    .line_height
        );
        assert!(state.document_bottom > layout.blocks.last().unwrap().frame.max_y());
        assert!(!scene.is_empty());

        // Preserve an actual raster render of the wrapped production view so
        // review includes text and clipping pixels as well as its layout data.
        let mut renderer = tekes_ui_render::Renderer::new_headless()
            .expect("headless GPU is required for the narrow document specimen");
        let rgba = renderer
            .render_to_rgba8(
                &scene,
                ctx.text.atlas_mut(),
                (196, 700),
                1.0,
                crate::gallery::Gallery::background(ctx.theme),
            )
            .expect("render narrow CodeBlock specimen");
        if let Some(output) = std::env::var_os("TEKESUI_GOLD7_PAGE_OUT") {
            let output = std::path::PathBuf::from(output);
            std::fs::create_dir_all(&output).unwrap();
            let file = std::fs::File::create(output.join("code-block-172pt-150-text.png")).unwrap();
            let mut encoder = png::Encoder::new(file, 196, 700);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder
                .write_header()
                .unwrap()
                .write_image_data(&rgba)
                .unwrap();
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
