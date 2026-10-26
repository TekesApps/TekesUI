//! 静态文字页：固定夹具的文字，TekesUI 实时渲染、参照图、差异热图三个窗格，外加夹具与宽度选择器。
//!
//! # 布局（固定）
//!
//! 文字最宽 700 pt，三个窗格并排放不下（页面内容区宽 932 pt），所以这一页把参照槽改成**上下叠放**
//! （`SlotLayout::Vertical`）：每个窗格宽度等于所选宽度（320 / 500 / 700 pt），高度固定 170 pt。
//! 文字比窗格高时，三个窗格都只显示放得下的前几行整行（裁切线就是窗格边框，标题行右侧写着
//! “显示前 N 行 / 共 M 行”）；差异数字是**整张画布**的结果，不受裁切影响。
//!
//! ```text
//! 夹具  [sentences-a] [mixed-cjk-latin] [kinsoku] [emoji] [combining] [rtl] [long-paragraph]
//! 宽度  [700] [500] [320]
//! TekesUI 实时   显示前 10 行 / 共 101 行
//! +--------------------------------------+   <- 窗格 = 文字画布，原点 (0, 0)，边框画在外面 1pt
//! 参照图         文件名
//! +--------------------------------------+   <- 参照图 1:1 画在设备像素上
//! 差异热图
//! +--------------------------------------+
//! [通过] 超阈值比例 …  最大连通域 …  最大单像素差 …
//! ```
//!
//! # 内容
//!
//! - 窗格 1：夹具文本原样（含文件末尾的换行），系统界面字体 14pt，行间距 0，段落原点是窗格左上角
//!   （整数 pt）。颜色和底色取参照侧 `appearances.json` 里的值，没有就用默认值
//!   （浅 (0,0,0)/(255,255,255)，深 (230,230,230)/(30,30,30)），这样三个窗格的底色一致；
//! - 窗格 2：参照图（见 `static_text.rs` 的说明），没有时写明原因；
//! - 窗格 3：差异热图，下面是三个数字和通过与否（`DiffOptions::for_scale`）。

use tekes_ui_controls::{Button, ButtonSize, ButtonTextAlign, paint};
use tekes_ui_scene::{ImageQuad, Point, Quad, Rect, Scene, Size};
use tekes_ui_tokens::{ColorRole, Contrast, CornerRadius, Stroke, TextRole, Theme, proposal};

use crate::gallery::containers::{META_STYLE, NOTE_STYLE, PANE_TITLES, ReferenceSlot};
use crate::gallery::pages::PageCtx;
use crate::gallery::static_text::{
    Computed, FIXTURES, REFERENCE_ENV, ReferenceStatus, StaticText, VIEW_H, is_dark, load_colors,
    scale_to_int,
};
use crate::gallery::text::LabelStyle;

// ---------------------------------------------------------------- 布局

/// 选择器行的标签槽宽度（“夹具”“宽度”）。
const PICKER_LABEL_W: f32 = 44.0;
/// Small breathing room between selector labels and their first button.
const PICKER_LABEL_GAP: f32 = 6.0;
const BUTTON_H: f32 = 20.0;
const BUTTON_GAP: f32 = 6.0;
/// 夹具按钮的宽度（放得下 `mixed-cjk-latin`）。
const FIXTURE_BUTTON_W: f32 = 112.0;
const WIDTH_BUTTON_W: f32 = 56.0;
/// 两行选择器的行距。
const PICKER_PITCH: f32 = 28.0;
const PICKER_ROW_GAP: f32 = 8.0;
/// 参照槽起点距内容区顶。
const SLOT_TOP: f32 = 64.0;
/// 差异数字距第三个窗格下沿。
const STATS_GAP: f32 = 8.0;
/// 标题行里右侧说明文字的起点（距窗格左缘）。
const NOTE_X: f32 = 120.0;

/// 第 `i` 个夹具按钮。
pub fn fixture_button(content: Rect, i: usize) -> Rect {
    Rect::new(
        content.min_x() + PICKER_LABEL_W + i as f32 * (FIXTURE_BUTTON_W + BUTTON_GAP),
        content.min_y(),
        FIXTURE_BUTTON_W,
        BUTTON_H,
    )
}

/// 第 `j` 个宽度按钮（`j` 是当前夹具宽度列表里的下标）。
pub fn width_button(content: Rect, j: usize) -> Rect {
    Rect::new(
        content.min_x() + PICKER_LABEL_W + j as f32 * (WIDTH_BUTTON_W + BUTTON_GAP),
        content.min_y() + PICKER_PITCH,
        WIDTH_BUTTON_W,
        BUTTON_H,
    )
}

fn slot_at(content: Rect, width: u32, top: f32) -> ReferenceSlot {
    ReferenceSlot::vertical(
        Point::new(content.min_x(), top),
        Size::new(width as f32, VIEW_H),
    )
}

fn columns_for(available: f32, item_width: f32, count: usize) -> usize {
    if count == 0 || available <= 0.0 || item_width <= 0.0 {
        return 1;
    }
    (((available + BUTTON_GAP) / (item_width + BUTTON_GAP)).floor() as usize)
        .max(1)
        .min(count)
}

struct LiveSelectorLayout {
    label_width: f32,
    fixture_width: f32,
    fixture_columns: usize,
    fixture_row_heights: Vec<f32>,
    fixture_height: f32,
    width_button_width: f32,
    width_columns: usize,
    width_row_heights: Vec<f32>,
    width_height: f32,
    pane_top_offset: f32,
}

/// Measure the live selector rows once; both drawing and the host's document prepass use these
/// exact Token-backed Button heights and the same pane geometry below them.
fn live_selector_layout(
    content_width: f32,
    st: &StaticText,
    text: &mut crate::gallery::text::TextEngine,
) -> LiveSelectorLayout {
    // Size this column from the same localized, type-scaled shaping metrics used to draw the
    // labels. Preserve the 44pt baseline slot when it fits; grow only when the copy needs room.
    let fixture_label_width = text.label("夹具", META_STYLE.role, None).width as f32;
    let width_label_width = text.label("宽度", META_STYLE.role, None).width as f32;
    let label_width =
        PICKER_LABEL_W.max(fixture_label_width.max(width_label_width) + PICKER_LABEL_GAP);
    let available = (content_width - label_width).max(0.0);
    let fixture_width = FIXTURE_BUTTON_W.min(available);
    let fixture_columns = columns_for(available, fixture_width, FIXTURES.len());
    let fixture_rows = FIXTURES.len().div_ceil(fixture_columns);
    let fixture_row_heights: Vec<f32> = (0..fixture_rows)
        .map(|row| {
            (row * fixture_columns..((row + 1) * fixture_columns).min(FIXTURES.len()))
                .map(|index| {
                    text.measure_button(
                        FIXTURES[index].name,
                        fixture_width,
                        ButtonSize::Xs,
                        proposal::TextRole::UiCaption,
                    )
                    .height
                })
                .fold(BUTTON_H, f32::max)
        })
        .collect();
    let fixture_height = fixture_row_heights.iter().sum::<f32>()
        + PICKER_ROW_GAP * fixture_rows.saturating_sub(1) as f32;

    let widths = st.widths();
    let width_button_width = WIDTH_BUTTON_W.min(available);
    let width_columns = columns_for(available, width_button_width, widths.len());
    let width_rows = widths.len().div_ceil(width_columns);
    let width_row_heights: Vec<f32> = (0..width_rows)
        .map(|row| {
            (row * width_columns..((row + 1) * width_columns).min(widths.len()))
                .map(|index| {
                    text.measure_button(
                        &widths[index].to_string(),
                        width_button_width,
                        ButtonSize::Xs,
                        proposal::TextRole::UiCaption,
                    )
                    .height
                })
                .fold(BUTTON_H, f32::max)
        })
        .collect();
    let width_height = width_row_heights.iter().sum::<f32>()
        + PICKER_ROW_GAP * width_rows.saturating_sub(1) as f32;
    let width_y = fixture_height + PICKER_ROW_GAP;
    let pane_top_offset = (width_y + width_height + 16.0).max(SLOT_TOP);

    LiveSelectorLayout {
        label_width,
        fixture_width,
        fixture_columns,
        fixture_row_heights,
        fixture_height,
        width_button_width,
        width_columns,
        width_row_heights,
        width_height,
        pane_top_offset,
    }
}

/// Required vertical document extent for the calibration body, relative to its content origin.
/// Selector measurements and the final three-pane/stat line geometry are shared with `build`.
pub fn document_extent(
    content_width: f32,
    text: &mut crate::gallery::text::TextEngine,
    st: &StaticText,
) -> f32 {
    let pane_top = if text.locale() == crate::gallery::locale::Locale::Reference {
        SLOT_TOP
    } else {
        live_selector_layout(content_width, st, text).pane_top_offset
    };
    let content = Rect::new(0.0, 0.0, content_width.max(0.0), 0.0);
    let slot = slot_at(content, st.width(), pane_top);
    slot.pane_rect(2).max_y() + STATS_GAP + 14.0
}

// ---------------------------------------------------------------- 文案

/// 窗格 2 没有图时写什么。
pub fn status_message(status: &ReferenceStatus) -> String {
    match status {
        ReferenceStatus::NoDir => format!("未提供参照图（设置 {REFERENCE_ENV}）"),
        ReferenceStatus::Missing { file } => format!("参照目录里没有 {file}"),
        ReferenceStatus::Invalid { file, reason } => format!("读不了 {file}：{reason}"),
        ReferenceStatus::SizeMismatch {
            file,
            got,
            expected,
        } => format!(
            "{file} 的尺寸是 {}x{} 像素，预期 {}x{}",
            got.0, got.1, expected.0, expected.1
        ),
        ReferenceStatus::RenderFailed { reason } => format!("差异计算失败：{reason}"),
        ReferenceStatus::Loaded => String::new(),
    }
}

/// 差异数字那一行的文字（三个数字，按 超阈值比例、最大连通域、最大单像素差 的顺序）。
pub fn numbers_line(n: &crate::gallery::static_text::Numbers) -> String {
    format!(
        "超阈值比例 {:.4}%（上限 {:.2}%）   最大连通域 {} 像素（上限 {}）   最大单像素差 {}/255",
        n.ratio * 100.0,
        n.max_ratio * 100.0,
        n.max_component,
        n.max_component_limit,
        n.max_pixel_diff
    )
}

fn button(
    scene: &mut Scene,
    renderer: (&mut crate::gallery::text::TextEngine, Theme),
    button: &mut Button,
    rect: Rect,
    clip: Rect,
    label: &str,
    selected: bool,
) {
    button.state.selected = selected;
    renderer.0.render_button(
        scene,
        button,
        label,
        rect,
        Some(clip),
        tekes_ui_tokens::Palette::new(renderer.1, 0).expect("registered Gallery brand"),
        paint::ButtonVariant::Quiet,
        ButtonSize::Xs,
        proposal::TextRole::UiCaption,
        ButtonTextAlign::Center,
    );
}

/// Draw the frozen selector artwork used by the historical offline reference snapshots.
/// This path deliberately has no Button model, hit target, or keyboard behavior; the live
/// English and Chinese calibration page below uses the production Button component.
fn draw_reference_selector(
    scene: &mut Scene,
    theme: Theme,
    text: &mut crate::gallery::text::TextEngine,
    rect: Rect,
    label: &str,
    selected: bool,
) {
    let fill = if selected {
        ColorRole::RowSelection.resolve(theme)
    } else {
        tekes_ui_tokens::Rgba::new(0.0, 0.0, 0.0, 0.0)
    };
    scene.push_quad(
        Quad::new(rect, fill)
            .with_corner_radius(CornerRadius::Row.value())
            .with_border(Stroke::Hairline.value(), ColorRole::Border.resolve(theme)),
    );
    let ink = if selected {
        ColorRole::PrimaryText
    } else {
        ColorRole::SecondaryText
    };
    text.label_in(
        scene,
        theme,
        rect,
        label,
        LabelStyle::new(TextRole::RowMeta, ink).centered(),
    );
}

fn draw_reference_selectors(
    st: &mut StaticText,
    content: Rect,
    scene: &mut Scene,
    theme: Theme,
    text: &mut crate::gallery::text::TextEngine,
) {
    // Offline golden rendering must not leave stale production hit geometry or focus behind.
    st.blur();
    for button in st.fixture_buttons_mut() {
        button.clear_layout();
    }
    for button in st.width_buttons_mut() {
        button.clear_layout();
    }

    let label_slot = |row: f32| {
        Rect::new(
            content.min_x(),
            content.min_y() + row * PICKER_PITCH,
            PICKER_LABEL_W,
            BUTTON_H,
        )
    };
    text.label_in(scene, theme, label_slot(0.0), "夹具", META_STYLE);
    for (i, fixture) in FIXTURES.iter().take(7).enumerate() {
        draw_reference_selector(
            scene,
            theme,
            text,
            fixture_button(content, i),
            fixture.name,
            i == st.fixture(),
        );
    }
    text.label_in(scene, theme, label_slot(1.0), "宽度", META_STYLE);
    for (j, width) in st.widths().iter().enumerate() {
        draw_reference_selector(
            scene,
            theme,
            text,
            width_button(content, j),
            &width.to_string(),
            *width == st.width(),
        );
    }
}

/// 一个窗格里的图像：1:1 画在设备像素上，矩形 = 像素尺寸 / 倍率。
fn image_rect(pane: Rect, px: (u32, u32), scale: u32) -> Rect {
    Rect::new(
        pane.min_x(),
        pane.min_y(),
        px.0 as f32 / scale as f32,
        px.1 as f32 / scale as f32,
    )
}

pub fn build(ctx: &mut PageCtx<'_>, content: Rect, scene: &mut Scene) {
    build_clipped(ctx, content, content, scene);
}

/// Build within translated document coordinates while retaining the host's fixed viewport clip.
pub fn build_clipped(ctx: &mut PageCtx<'_>, content: Rect, viewport_clip: Rect, scene: &mut Scene) {
    let theme = ctx.theme;
    let scale = ctx.text.scale();
    let scale_i = scale_to_int(scale);
    let dark = is_dark(theme);
    let st = &mut *ctx.static_text;
    let text = &mut *ctx.text;
    let button_clip = content.intersection(&viewport_clip).unwrap_or(Rect::new(
        viewport_clip.min_x(),
        viewport_clip.min_y(),
        0.0,
        0.0,
    ));

    // 1. 选择器。
    let frozen_reference = text.locale() == crate::gallery::locale::Locale::Reference;
    let live_slot_top = if frozen_reference {
        draw_reference_selectors(st, content, scene, theme, text);
        content.min_y() + SLOT_TOP
    } else {
        let layout = live_selector_layout(content.size.width, st, text);
        let label_slot =
            |top: f32, height: f32| Rect::new(content.min_x(), top, layout.label_width, height);
        let count = FIXTURES.len();
        for button in st.fixture_buttons_mut().iter_mut().skip(count) {
            button.clear_layout();
        }
        let widths = st.widths().to_vec();
        for button in st.width_buttons_mut().iter_mut().skip(widths.len()) {
            button.clear_layout();
        }
        let fixture_y = content.min_y();
        text.label_in(
            scene,
            theme,
            label_slot(fixture_y, layout.fixture_height),
            "夹具",
            META_STYLE,
        );
        let mut row_y = fixture_y;
        for (row, row_height) in layout.fixture_row_heights.iter().copied().enumerate() {
            for column in 0..layout.fixture_columns {
                let i = row * layout.fixture_columns + column;
                if i >= count {
                    break;
                }
                let fixture = &FIXTURES[i];
                let selected = i == st.fixture();
                let rect = Rect::new(
                    content.min_x()
                        + layout.label_width
                        + column as f32 * (layout.fixture_width + BUTTON_GAP),
                    row_y,
                    layout.fixture_width,
                    row_height,
                );
                let model = st.fixture_button_mut(i).expect("fixture button model");
                button(
                    scene,
                    (text, theme),
                    model,
                    rect,
                    button_clip,
                    fixture.name,
                    selected,
                );
            }
            row_y += row_height + PICKER_ROW_GAP;
        }
        let width_y = fixture_y + layout.fixture_height + PICKER_ROW_GAP;
        text.label_in(
            scene,
            theme,
            label_slot(width_y, layout.width_height),
            "宽度",
            META_STYLE,
        );
        let mut row_y = width_y;
        for (row, row_height) in layout.width_row_heights.iter().copied().enumerate() {
            for column in 0..layout.width_columns {
                let j = row * layout.width_columns + column;
                if j >= widths.len() {
                    break;
                }
                let width = widths[j];
                let selected = width == st.width();
                let rect = Rect::new(
                    content.min_x()
                        + layout.label_width
                        + column as f32 * (layout.width_button_width + BUTTON_GAP),
                    row_y,
                    layout.width_button_width,
                    row_height,
                );
                let model = st.width_button_mut(j).expect("width button model");
                button(
                    scene,
                    (text, theme),
                    model,
                    rect,
                    button_clip,
                    &width.to_string(),
                    selected,
                );
            }
            row_y += row_height + PICKER_ROW_GAP;
        }
        content.min_y() + layout.pane_top_offset
    };

    // 2. 三个窗格。
    let shaped = st.shaped(text);
    let result: Option<Computed> = st.result(theme, scale).cloned();
    let colors = match &result {
        Some(c) => c.colors,
        None => load_colors(st.reference_dir(), dark),
    };
    let status = match &result {
        Some(c) => c.status.clone(),
        None if st.reference_dir().is_none() => ReferenceStatus::NoDir,
        None => ReferenceStatus::Missing {
            file: "（尚未加载）".into(),
        },
    };
    let slot_top = live_slot_top;
    let focus_count = st.focus_order().len();
    if st.focus_index().is_some_and(|index| index >= focus_count) {
        st.set_focus(None, false);
    }
    let ref_slot = slot_at(content, st.width(), slot_top);
    let pane = |i| ref_slot.pane_rect(i);
    ref_slot.draw(scene, theme, text, &PANE_TITLES, |scene, text, rect| {
        scene.push_quad(Quad::new(rect, colors.bg()));
        text.push(scene, &shaped.cropped, rect.origin, colors.fg());
    });

    // 标题行右侧的说明。
    let note =
        |scene: &mut Scene, text: &mut crate::gallery::text::TextEngine, i: usize, s: &str| {
            let h = ref_slot.header_slot(i);
            let r = Rect::new(
                h.min_x() + NOTE_X,
                h.min_y(),
                h.size.width.max(400.0) - NOTE_X,
                h.size.height,
            );
            text.label_in(scene, theme, r, s, NOTE_STYLE);
        };
    let crop_note = if shaped.shown_lines < shaped.total_lines {
        format!(
            "显示前 {} 行 / 共 {} 行（画布 {}x{} pt，{}x）",
            shaped.shown_lines,
            shaped.total_lines,
            st.width(),
            shaped.canvas_h,
            scale_i
        )
    } else {
        format!(
            "共 {} 行（画布 {}x{} pt，{}x）",
            shaped.total_lines,
            st.width(),
            shaped.canvas_h,
            scale_i
        )
    };
    note(scene, text, 0, &crop_note);

    // 窗格 2：参照图。
    let mut ref_note = String::new();
    if let Some(c) = &result
        && c.status == ReferenceStatus::Loaded
    {
        ref_note = crate::gallery::static_text::reference_file_name(
            FIXTURES[c.key.fixture].name,
            c.key.width,
            c.key.scale,
            c.key.dark,
        );
    }
    if theme.contrast == Contrast::Increased {
        if !ref_note.is_empty() {
            ref_note.push('；');
        }
        ref_note.push_str("没有 IC 版，显示普通对比度的图");
    }
    if !ref_note.is_empty() {
        note(scene, text, 1, &ref_note);
    }
    let message_slot = |r: Rect| {
        Rect::new(
            r.min_x() + 12.0,
            r.min_y() + 12.0,
            r.size.width - 24.0,
            r.size.height - 24.0,
        )
    };
    match &result {
        Some(c) if c.status == ReferenceStatus::Loaded => {
            if let Some(id) = c.reference_image {
                scene.push_image(ImageQuad {
                    rect: image_rect(pane(1), c.image_px, scale_i),
                    image: id,
                    opacity: 1.0,
                });
            }
        }
        _ => {
            text.wrapped_label_in(
                scene,
                theme,
                message_slot(pane(1)),
                &status_message(&status),
                META_STYLE,
            );
        }
    }

    // 窗格 3：差异热图和数字。
    let mut stats = None;
    match &result {
        Some(c) if c.heat_image.is_some() => {
            if let Some(id) = c.heat_image {
                scene.push_image(ImageQuad {
                    rect: image_rect(pane(2), c.image_px, scale_i),
                    image: id,
                    opacity: 1.0,
                });
            }
            stats = c.numbers;
        }
        _ => {
            text.wrapped_label_in(
                scene,
                theme,
                message_slot(pane(2)),
                "需要参照图才能比较",
                NOTE_STYLE,
            );
        }
    }
    let pane = ref_slot.pane_rect(2);
    let line = Rect::new(
        pane.min_x(),
        pane.max_y() + STATS_GAP,
        content.max_x() - pane.min_x(),
        14.0,
    );
    match stats {
        Some(n) => {
            let mark = Rect::new(line.min_x(), line.min_y() + 2.0, 10.0, 10.0);
            let color = if n.passed {
                ColorRole::StatusSucceeded
            } else {
                ColorRole::StatusFailed
            };
            scene.push_quad(
                Quad::new(mark, color.resolve(theme))
                    .with_corner_radius(CornerRadius::Badge.value()),
            );
            let verdict = Rect::new(line.min_x() + 16.0, line.min_y(), 44.0, line.size.height);
            text.label_in(
                scene,
                theme,
                verdict,
                if n.passed { "通过" } else { "未通过" },
                LabelStyle::new(TextRole::SectionHeader, ColorRole::PrimaryText),
            );
            let nums = Rect::new(
                line.min_x() + 64.0,
                line.min_y(),
                line.size.width - 64.0,
                line.size.height,
            );
            text.label_in(scene, theme, nums, &numbers_line(&n), META_STYLE);
        }
        None => {
            text.label_in(scene, theme, line, "未比较", NOTE_STYLE);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_picker(st: &mut StaticText, text: &mut crate::gallery::text::TextEngine) {
        text.begin_frame();
        let mut ctx = PageCtx {
            theme: Theme::LIGHT,
            text,
            static_text: st,
        };
        build(
            &mut ctx,
            Rect::new(0.0, 0.0, 1400.0, 900.0),
            &mut Scene::new(),
        );
    }

    #[test]
    fn calibration_selectors_activate_through_the_retained_button_geometry() {
        let mut st = StaticText::new(None);
        let mut text = crate::gallery::text::TextEngine::new();

        build_picker(&mut st, &mut text);
        let fixture = st.fixture_buttons()[1].layout().unwrap();
        let point = fixture.frame.center();
        assert!(st.pointer_move(point));
        assert!(st.fixture_buttons()[1].state.hovered);
        assert!(st.pointer_down(point));
        assert!(st.fixture_buttons()[1].state.pressed);
        assert!(st.pointer_up(point));
        assert_eq!(st.fixture(), 1);

        build_picker(&mut st, &mut text);
        let width = st.width_buttons()[1].layout().unwrap().frame.center();
        assert!(st.pointer_down(width));
        assert!(st.pointer_up(width));
        assert_eq!(st.width(), 500);

        build_picker(&mut st, &mut text);
        let fixture = st.fixture_buttons()[2].layout().unwrap().frame.center();
        assert!(st.pointer_down(fixture));
        assert!(!st.pointer_up(Point::new(fixture.x + 500.0, fixture.y)));
        assert_eq!(st.fixture(), 1);
        st.cancel_pointer();
    }

    #[test]
    fn scrolled_selector_uses_fixed_viewport_for_hit_and_paint() {
        let mut st = StaticText::new(None);
        let mut text = crate::gallery::text::TextEngine::new();
        let content = Rect::new(20.0, 188.0, 900.0, 600.0);
        let viewport = Rect::new(20.0, 100.0, 900.0, 100.0);
        text.begin_frame();
        let mut ctx = PageCtx {
            theme: Theme::LIGHT,
            text: &mut text,
            static_text: &mut st,
        };
        build_clipped(&mut ctx, content, viewport, &mut Scene::new());

        let layout = st.fixture_buttons()[1].layout().unwrap();
        let visible = layout.visible.unwrap();
        assert!((visible.min_y() - viewport.min_y().max(layout.frame.min_y())).abs() < 0.01);
        assert!((visible.max_y() - viewport.max_y()).abs() < 0.01);
        assert!(visible.size.height < layout.frame.size.height);
        let outside = Point::new(layout.frame.center().x, layout.frame.max_y() - 1.0);
        assert!(!layout.hit_test(outside));
        assert!(!st.pointer_down(outside));
        assert!(!st.pointer_up(outside));
        assert_eq!(st.fixture(), 0);

        let inside = visible.center();
        assert!(st.pointer_down(inside));
        assert!(st.pointer_up(inside));
        assert_eq!(st.fixture(), 1);
    }

    #[test]
    fn historical_reference_selectors_are_read_only_and_live_locale_uses_buttons() {
        let mut st = StaticText::new(None);
        let mut text = crate::gallery::text::TextEngine::new();
        text.set_locale(crate::gallery::locale::Locale::Reference);
        build_picker(&mut st, &mut text);
        assert!(
            st.fixture_buttons()
                .iter()
                .all(|button| button.layout().is_none())
        );
        assert!(
            st.width_buttons()
                .iter()
                .all(|button| button.layout().is_none())
        );
        assert!(st.focus_order().is_empty());

        text.set_locale(crate::gallery::locale::Locale::En);
        build_picker(&mut st, &mut text);
        assert_eq!(
            st.fixture_buttons()
                .iter()
                .filter(|button| button.layout().is_some())
                .count(),
            FIXTURES.len()
        );
        assert_eq!(
            st.width_buttons()
                .iter()
                .filter(|button| button.layout().is_some())
                .count(),
            st.widths().len()
        );
    }

    #[test]
    fn large_type_selector_rows_grow_before_content_and_stay_inside_the_viewport() {
        for scale in [1.0, 1.25, 1.5] {
            for width in [1400.0, 420.0, 240.0] {
                let mut st = StaticText::new(None);
                let mut text = crate::gallery::text::TextEngine::new();
                text.set_type_scale(scale);
                let content = Rect::new(0.0, 0.0, width, 2000.0);
                text.begin_frame();
                let mut ctx = PageCtx {
                    theme: Theme::LIGHT,
                    text: &mut text,
                    static_text: &mut st,
                };
                build(&mut ctx, content, &mut Scene::new());

                let fixtures: Vec<_> = st
                    .fixture_buttons()
                    .iter()
                    .filter_map(Button::layout)
                    .collect();
                let widths: Vec<_> = st
                    .width_buttons()
                    .iter()
                    .filter_map(Button::layout)
                    .collect();
                assert_eq!(fixtures.len(), FIXTURES.len());
                assert!(!widths.is_empty());
                for (index, layout) in fixtures.iter().chain(&widths).enumerate() {
                    assert_eq!(layout.visible, Some(layout.frame));
                    assert!(layout.frame.min_x() >= content.min_x());
                    assert!(
                        layout.frame.max_x() <= content.max_x() + 0.01,
                        "width={width}, scale={scale}, index={index}"
                    );
                    assert!(layout.frame.min_y() >= content.min_y());
                    assert!(layout.frame.max_y() <= content.max_y());
                    for other in fixtures.iter().chain(&widths).skip(index + 1) {
                        assert!(
                            layout.frame.intersection(&other.frame).is_none(),
                            "width={width}, scale={scale}"
                        );
                    }
                }

                let fixture_bottom = fixtures
                    .iter()
                    .map(|layout| layout.frame.max_y())
                    .fold(0.0, f32::max);
                let width_top = widths[0].frame.min_y();
                assert!(
                    width_top >= fixture_bottom + 8.0 - 0.01,
                    "width={width}, scale={scale}"
                );
                let width_bottom = widths
                    .iter()
                    .map(|layout| layout.frame.max_y())
                    .fold(0.0, f32::max);
                let pane_top = (width_bottom + 16.0).max(content.min_y() + SLOT_TOP);
                // `build` keeps the original 64pt minimum breathing room and pushes content lower
                // when the measured button rows need more room at large type scales.
                assert!(
                    pane_top >= width_bottom + 16.0 - 0.01,
                    "width={width}, scale={scale}"
                );
            }
        }
    }

    #[test]
    fn live_selector_label_column_tracks_localized_scaled_text_and_keeps_baseline_when_it_fits() {
        for locale in [
            crate::gallery::locale::Locale::En,
            crate::gallery::locale::Locale::ZhCn,
        ] {
            for scale in [1.0, 1.5] {
                let mut st = StaticText::new(None);
                let mut text = crate::gallery::text::TextEngine::new();
                text.set_locale(locale);
                text.set_type_scale(scale);
                text.begin_frame();

                let fixture_label = text.label("夹具", META_STYLE.role, None).width as f32;
                let width_label = text.label("宽度", META_STYLE.role, None).width as f32;
                let expected =
                    PICKER_LABEL_W.max(fixture_label.max(width_label) + PICKER_LABEL_GAP);
                let layout = live_selector_layout(720.0, &st, &mut text);
                assert_eq!(
                    layout.label_width, expected,
                    "locale={locale:?}, scale={scale}"
                );

                let content = Rect::new(0.0, 0.0, 720.0, 1200.0);
                let mut ctx = PageCtx {
                    theme: Theme::LIGHT,
                    text: &mut text,
                    static_text: &mut st,
                };
                build(&mut ctx, content, &mut Scene::new());
                let first_button = st.fixture_buttons()[0].layout().unwrap().frame;
                assert_eq!(first_button.min_x(), layout.label_width);
                assert!(first_button.min_x() - fixture_label >= PICKER_LABEL_GAP - 0.01);
                assert_eq!(layout.label_width, PICKER_LABEL_W.max(expected));
            }
        }
    }

    #[test]
    fn measured_calibration_extent_contains_wrapped_selector_rows_and_all_panes() {
        for width in [720.0, 384.0] {
            let mut st = StaticText::new(None);
            let mut text = crate::gallery::text::TextEngine::new();
            text.set_type_scale(1.5);
            text.begin_frame();
            let content = Rect::new(0.0, 0.0, width, 2400.0);
            let selector = live_selector_layout(width, &st, &mut text);
            {
                let mut ctx = PageCtx {
                    theme: Theme::LIGHT,
                    text: &mut text,
                    static_text: &mut st,
                };
                build(&mut ctx, content, &mut Scene::new());
            }

            let pane_bottom = slot_at(
                content,
                st.width(),
                content.min_y() + selector.pane_top_offset,
            )
            .pane_rect(2)
            .max_y();
            let actual_body_bottom = pane_bottom + STATS_GAP + 14.0;
            let document_bottom = document_extent(width, &mut text, &st);
            assert!(
                actual_body_bottom <= document_bottom + 0.01,
                "width={width}: body ends at {actual_body_bottom}, document extent is {document_bottom}"
            );
            let selector_bottom = st
                .fixture_buttons()
                .iter()
                .chain(st.width_buttons())
                .filter_map(Button::layout)
                .map(|layout| layout.frame.max_y())
                .fold(0.0, f32::max);
            assert!(selector_bottom < pane_bottom, "width={width}");
        }
    }

    #[test]
    fn calibration_selectors_have_real_tab_enter_and_space_behavior() {
        use tekes_ui_controls::ControlKey;

        let mut st = StaticText::new(None);
        let mut text = crate::gallery::text::TextEngine::new();
        build_picker(&mut st, &mut text);
        assert!(st.handle_key(ControlKey::Tab { reverse: false }, true));
        assert!(st.fixture_buttons()[0].state.focus_visible);
        assert!(st.handle_key(ControlKey::Tab { reverse: false }, true));
        assert!(st.fixture_buttons()[1].state.focus_visible);
        assert!(st.handle_key(ControlKey::Enter, true));
        assert_eq!(st.fixture(), 1);

        build_picker(&mut st, &mut text);
        // From fixture 1, advance to the second width option (500pt).
        for _ in 0..st
            .fixture_buttons()
            .iter()
            .filter(|b| b.layout().is_some())
            .count()
        {
            assert!(st.handle_key(ControlKey::Tab { reverse: false }, true));
        }
        assert!(st.width_buttons()[1].state.focus_visible);
        assert!(st.handle_key(ControlKey::Space, true));
        assert!(st.width_buttons()[1].state.pressed);
        assert!(!st.pointer_up(Point::new(f32::NAN, f32::NAN)));
        assert!(st.width_buttons()[1].state.pressed);
        assert!(st.handle_key(ControlKey::Space, false));
        assert_eq!(st.width(), 500);
        assert!(!st.width_buttons()[1].state.pressed);
    }
}
