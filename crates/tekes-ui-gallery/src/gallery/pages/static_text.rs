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

use tekes_ui_scene::{ImageQuad, Point, Quad, Rect, Scene, Size};
use tekes_ui_tokens::{ColorRole, Contrast, CornerRadius, Stroke, TextRole, Theme};

use crate::gallery::containers::{META_STYLE, NOTE_STYLE, PANE_TITLES, ReferenceSlot};
use crate::gallery::pages::PageCtx;
use crate::gallery::static_text::{
    Computed, FIXTURES, Pick, REFERENCE_ENV, ReferenceStatus, StaticText, VIEW_H, is_dark,
    load_colors, scale_to_int,
};
use crate::gallery::text::LabelStyle;

// ---------------------------------------------------------------- 布局

/// 选择器行的标签槽宽度（“夹具”“宽度”）。
const PICKER_LABEL_W: f32 = 44.0;
const BUTTON_H: f32 = 20.0;
const BUTTON_GAP: f32 = 6.0;
/// 夹具按钮的宽度（放得下 `mixed-cjk-latin`）。
const FIXTURE_BUTTON_W: f32 = 112.0;
const WIDTH_BUTTON_W: f32 = 56.0;
/// 两行选择器的行距。
const PICKER_PITCH: f32 = 28.0;
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

/// 参照槽：上下叠放，窗格宽度 = 所选宽度，高 [`VIEW_H`]。
pub fn slot(content: Rect, width: u32) -> ReferenceSlot {
    ReferenceSlot::vertical(
        Point::new(content.min_x(), content.min_y() + SLOT_TOP),
        Size::new(width as f32, VIEW_H),
    )
}

/// 差异数字那一行的槽（第三个窗格正下方）。
pub fn stats_slot(content: Rect, width: u32) -> Rect {
    let pane = slot(content, width).pane_rect(2);
    Rect::new(
        pane.min_x(),
        pane.max_y() + STATS_GAP,
        content.max_x() - pane.min_x(),
        14.0,
    )
}

/// 选择器上的点击。
pub fn hit_test(content: Rect, st: &StaticText, p: Point) -> Option<Pick> {
    for i in 0..FIXTURES.len() {
        if fixture_button(content, i).contains(p) {
            return Some(Pick::Fixture(i));
        }
    }
    for (j, w) in st.widths().iter().enumerate() {
        if width_button(content, j).contains(p) {
            return Some(Pick::Width(*w));
        }
    }
    None
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
    let color = if selected {
        ColorRole::PrimaryText
    } else {
        ColorRole::SecondaryText
    };
    text.label_in(
        scene,
        theme,
        rect,
        label,
        LabelStyle::new(TextRole::RowMeta, color).centered(),
    );
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
    let theme = ctx.theme;
    let scale = ctx.text.scale();
    let scale_i = scale_to_int(scale);
    let dark = is_dark(theme);
    let st = &mut *ctx.static_text;
    let text = &mut *ctx.text;

    // 1. 选择器。
    let label_slot = |row: f32| {
        Rect::new(
            content.min_x(),
            content.min_y() + row * PICKER_PITCH,
            PICKER_LABEL_W,
            BUTTON_H,
        )
    };
    text.label_in(scene, theme, label_slot(0.0), "夹具", META_STYLE);
    for (i, f) in FIXTURES.iter().enumerate() {
        button(
            scene,
            theme,
            text,
            fixture_button(content, i),
            f.name,
            i == st.fixture(),
        );
    }
    text.label_in(scene, theme, label_slot(1.0), "宽度", META_STYLE);
    for (j, w) in st.widths().iter().enumerate() {
        button(
            scene,
            theme,
            text,
            width_button(content, j),
            &w.to_string(),
            *w == st.width(),
        );
    }

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
    let ref_slot = slot(content, st.width());
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
    let line = stats_slot(content, st.width());
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
