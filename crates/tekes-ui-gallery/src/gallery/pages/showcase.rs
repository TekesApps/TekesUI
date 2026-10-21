//! Fixed state illustrations of the forty proposal components, not interactive controls.
//!
//! Authored design values come from the generated proposal API. Relative specimen
//! geometry (pane ratios, chart data, sample widths) is deliberately a calibration
//! fixture, not a new global token scale. There is no animation, input field model,
//! external icon font, clock, or component-local RGB palette here.

use super::PageCtx;
use crate::gallery::{
    catalog::{self, Component, ShowcaseState},
    text::TextEngine,
};
use tekes_ui_scene::{Point, Quad, Rect, Scene};
use tekes_ui_tokens::{Rgba, Spacing, Stroke, Theme, proposal};

const GAP: f32 = Spacing::Loose.value();
const SMALL: f32 = Spacing::Tight.value();
const INSET: f32 = Spacing::Regular.value();
const TITLE_H: f32 = proposal::TextRole::UiHeading.style().line_height;
fn chip_h() -> f32 {
    metric("size", "size-button-xs")
}
const INTRO_H: f32 = proposal::TextRole::UiCaption.style().line_height + Spacing::Loose.value();

fn color(theme: Theme, id: &str) -> Rgba {
    proposal::render_color(id, theme).expect("registered proposal render color")
}
fn metric(group: &str, id: &str) -> f32 {
    proposal::metric(group, id).expect("registered proposal metric")
}
fn clear(theme: Theme) -> Rgba {
    color(theme, "surface-1").with_alpha(0.0)
}
fn state_label(id: &str) -> &str {
    match id {
        "rest" => "默认",
        "hover" => "悬停",
        "pressed" => "按下",
        "focus" => "焦点",
        "disabled" => "禁用",
        "loading" => "加载",
        "checked" => "选中",
        "selected" => "已选",
        "error" => "错误",
        "mixed" => "混合",
        "read-only" => "只读",
        "composing" => "组字",
        "selection" => "选区",
        "open" => "打开",
        "empty" => "空",
        "expanded" => "展开",
        "closing" => "关闭中",
        "success" => "成功",
        "restricted" => "受限",
        "unverified" => "未验证",
        "cancelled" => "已取消",
        "indeterminate" => "不定进度",
        "dragging" => "拖动",
        "scrolling" => "滚动",
        "at-start" => "起点",
        "at-end" => "终点",
        _ => id,
    }
}

#[derive(Clone, Copy)]
struct State<'a>(&'a str);
impl State<'_> {
    fn is(self, id: &str) -> bool {
        self.0.split('+').any(|s| s == id)
    }
}

/// One source of geometry for rendering and hit testing. No state-dependent movement.
fn tile_rect(category: &str, content: Rect, index: usize) -> Rect {
    let count = catalog::group(category).component_ids.len();
    let columns = if category == "data" { 1 } else { 2 };
    let rows = count.div_ceil(columns);
    let width = (content.size.width - GAP * (columns - 1) as f32) / columns as f32;
    let height = (content.size.height - INTRO_H - GAP * (rows - 1) as f32) / rows as f32;
    Rect::new(
        content.min_x() + (index % columns) as f32 * (width + GAP),
        content.min_y() + INTRO_H + (index / columns) as f32 * (height + GAP),
        width,
        height,
    )
}
fn chip_rects(component: &'static Component, tile: Rect) -> Vec<(&'static str, Rect)> {
    let mut x = tile.min_x();
    let mut y = tile.min_y() + TITLE_H + SMALL;
    let mut out = Vec::new();
    for id in &component.states {
        let label = state_label(id);
        let width = (label.chars().count() as f32 * proposal::TextRole::UiCaption.style().size
            + 2.0 * INSET)
            .max(40.0);
        if x > tile.min_x() && x + width > tile.max_x() {
            x = tile.min_x();
            y += chip_h() + SMALL;
        }
        out.push((
            id.as_str(),
            Rect::new(x, y, width.min(tile.size.width), chip_h()),
        ));
        x += width + SMALL;
    }
    out
}
fn preview_rect(component: &'static Component, tile: Rect) -> Rect {
    let bottom = chip_rects(component, tile)
        .iter()
        .map(|(_, r)| r.max_y())
        .fold(tile.min_y() + TITLE_H, f32::max);
    Rect::new(
        tile.min_x(),
        bottom + INSET,
        tile.size.width,
        (tile.max_y() - bottom - INSET).max(0.0),
    )
}

pub fn hit_state(category: &str, content: Rect, p: Point) -> Option<(&'static str, &'static str)> {
    for (index, id) in catalog::group(category).component_ids.iter().enumerate() {
        let component = catalog::component(id);
        for (state, rect) in chip_rects(component, tile_rect(category, content, index)) {
            if rect.contains(p) {
                return Some((component.id.as_str(), state));
            }
        }
    }
    None
}

/// Scripted native selftest uses the exact same chip geometry as rendering/hit tests.
pub fn state_center(
    category: &str,
    content: Rect,
    component_id: &str,
    state_id: &str,
) -> Option<Point> {
    let group = catalog::group(category);
    let index = group
        .component_ids
        .iter()
        .position(|id| id == component_id)?;
    chip_rects(
        catalog::component(component_id),
        tile_rect(category, content, index),
    )
    .into_iter()
    .find(|(id, _)| *id == state_id)
    .map(|(_, rect)| rect.center())
}

struct Painter<'a> {
    text: &'a mut TextEngine,
    scene: &'a mut Scene,
    theme: Theme,
    clip: Rect,
}
impl Painter<'_> {
    fn quad(&mut self, rect: Rect, fill: Rgba, radius: f32, border: Option<Rgba>) {
        if let Some(visible) = rect.intersection(&self.clip) {
            let mut q = Quad::new(visible, fill).with_corner_radius(radius);
            if let Some(ink) = border {
                q = q.with_border(Stroke::Rule.value(), ink);
            }
            self.scene.push_quad(q);
        }
    }
    fn box_role(&mut self, rect: Rect, fill: &str, radius: &str, border: Option<&str>) {
        self.quad(
            rect,
            color(self.theme, fill),
            metric("radius", radius),
            border.map(|id| color(self.theme, id)),
        );
    }
    fn label(&mut self, rect: Rect, text: &str, role: proposal::TextRole, ink: &str) {
        if let Some(slot) = rect.intersection(&self.clip) {
            self.text.proposal_label_in(
                self.scene,
                self.theme,
                slot,
                text,
                role,
                color(self.theme, ink),
            );
        }
    }
    fn body(&mut self, r: Rect, t: &str) {
        self.label(r, t, proposal::TextRole::UiBody, "primary-text");
    }
    fn caption(&mut self, r: Rect, t: &str) {
        self.label(r, t, proposal::TextRole::UiCaption, "secondary-text");
    }
    fn wrapped(&mut self, r: Rect, text: &str, role: proposal::TextRole, ink: &str) {
        if let Some(slot) = r.intersection(&self.clip) {
            self.text.proposal_wrapped_in(
                self.scene,
                self.theme,
                slot,
                text,
                role,
                color(self.theme, ink),
            );
        }
    }
    fn line(&mut self, a: Point, b: Point, ink: &str) {
        // Preserve the existing line-sample geometry used by charts and other
        // Gallery calibration fixtures. Production icon strokes use the
        // centered `line_with_color` path below.
        let stroke = metric("size", "size-icon-artboard-stroke");
        let dx = b.x - a.x;
        let dy = b.y - a.y;
        if dx.abs() < 0.01 || dy.abs() < 0.01 {
            self.quad(
                Rect::new(
                    a.x.min(b.x),
                    a.y.min(b.y),
                    dx.abs().max(stroke),
                    dy.abs().max(stroke),
                ),
                color(self.theme, ink),
                0.,
                None,
            );
            return;
        }
        let steps = dx.abs().max(dy.abs()).ceil().max(1.) as usize;
        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            self.quad(
                Rect::new(a.x + dx * t, a.y + dy * t, stroke, stroke),
                color(self.theme, ink),
                0.,
                None,
            );
        }
    }
    fn line_with_color(&mut self, a: Point, b: Point, ink: Rgba) {
        let stroke = metric("size", "size-icon-artboard-stroke");
        let dx = b.x - a.x;
        let dy = b.y - a.y;
        if dx.abs() < 0.01 {
            self.quad(
                Rect::new(a.x - stroke * 0.5, a.y.min(b.y), stroke, dy.abs()),
                ink,
                0.,
                None,
            );
            return;
        }
        if dy.abs() < 0.01 {
            self.quad(
                Rect::new(a.x.min(b.x), a.y - stroke * 0.5, dx.abs(), stroke),
                ink,
                0.,
                None,
            );
            return;
        }
        let steps = dx.abs().max(dy.abs()).ceil().max(1.) as usize;
        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            let center = Point::new(a.x + dx * t, a.y + dy * t);
            self.quad(
                Rect::new(
                    center.x - stroke * 0.5,
                    center.y - stroke * 0.5,
                    stroke,
                    stroke,
                ),
                ink,
                0.,
                None,
            );
        }
    }
    fn icon(&mut self, r: Rect, kind: Icon, ink: &str) {
        self.icon_with_color(r, kind, color(self.theme, ink));
    }
    fn icon_with_color(&mut self, r: Rect, kind: Icon, ink: Rgba) {
        let p = |x: f32, y: f32| {
            Point::new(r.min_x() + r.size.width * x, r.min_y() + r.size.height * y)
        };
        match kind {
            Icon::Check => {
                self.line_with_color(p(0.15, 0.5), p(0.4, 0.75), ink);
                self.line_with_color(p(0.4, 0.75), p(0.85, 0.2), ink);
            }
            Icon::Chevron => {
                self.line_with_color(p(0.3, 0.2), p(0.65, 0.5), ink);
                self.line_with_color(p(0.65, 0.5), p(0.3, 0.8), ink);
            }
            Icon::Down => {
                self.line_with_color(p(0.2, 0.35), p(0.5, 0.65), ink);
                self.line_with_color(p(0.5, 0.65), p(0.8, 0.35), ink);
            }
            Icon::Close => {
                self.line_with_color(p(0.25, 0.25), p(0.75, 0.75), ink);
                self.line_with_color(p(0.75, 0.25), p(0.25, 0.75), ink);
            }
            Icon::Plus => {
                self.line_with_color(p(0.5, 0.2), p(0.5, 0.8), ink);
                self.line_with_color(p(0.2, 0.5), p(0.8, 0.5), ink);
            }
            Icon::Search => {
                self.quad(
                    Rect::new(
                        p(0.1, 0.1).x,
                        p(0.1, 0.1).y,
                        r.size.width * 0.55,
                        r.size.height * 0.55,
                    ),
                    clear(self.theme),
                    r.size.width,
                    Some(ink),
                );
                self.line_with_color(p(0.57, 0.57), p(0.9, 0.9), ink);
            }
            Icon::Folder => {
                self.quad(
                    Rect::new(
                        p(0.1, 0.3).x,
                        p(0.1, 0.3).y,
                        r.size.width * 0.8,
                        r.size.height * 0.6,
                    ),
                    clear(self.theme),
                    metric("radius", "radius-agent-marker"),
                    Some(ink),
                );
                self.line_with_color(p(0.1, 0.3), p(0.1, 0.15), ink);
                self.line_with_color(p(0.1, 0.15), p(0.45, 0.15), ink);
                self.line_with_color(p(0.45, 0.15), p(0.6, 0.3), ink);
            }
            Icon::File => {
                self.quad(
                    Rect::new(
                        p(0.2, 0.1).x,
                        p(0.2, 0.1).y,
                        r.size.width * 0.6,
                        r.size.height * 0.8,
                    ),
                    clear(self.theme),
                    metric("radius", "radius-agent-marker"),
                    Some(ink),
                );
                self.line_with_color(p(0.35, 0.4), p(0.65, 0.4), ink);
                self.line_with_color(p(0.35, 0.6), p(0.65, 0.6), ink);
            }
            Icon::Info => {
                self.quad(r, clear(self.theme), r.size.width, Some(ink));
                self.line_with_color(p(0.5, 0.42), p(0.5, 0.75), ink);
                self.quad(
                    Rect::new(
                        p(0.5, 0.23).x - Stroke::Emphasis.value() * 0.5,
                        p(0.5, 0.23).y - Stroke::Emphasis.value() * 0.5,
                        Stroke::Emphasis.value(),
                        Stroke::Emphasis.value(),
                    ),
                    ink,
                    0.,
                    None,
                );
            }
            Icon::Spinner => {
                for (x, y) in [(0.2, 0.2), (0.5, 0.1), (0.8, 0.3), (0.8, 0.7), (0.5, 0.85)] {
                    self.quad(
                        Rect::new(
                            p(x, y).x - Stroke::Emphasis.value() * 0.5,
                            p(x, y).y - INSET / 4.,
                            Stroke::Emphasis.value(),
                            INSET / 2.,
                        ),
                        ink,
                        Stroke::Rule.value(),
                        None,
                    );
                }
            }
        }
    }
    fn control(&mut self, r: Rect, s: State<'_>, fill: &str, radius: &str) {
        let bg = if s.is("pressed") {
            "button-pressed"
        } else if s.is("hover") {
            "button-hover"
        } else {
            fill
        };
        let edge = if s.is("error") {
            "status-failed"
        } else if s.is("focus") {
            "border-focused"
        } else {
            "border"
        };
        self.box_role(r, bg, radius, Some(edge));
    }
    fn status_ink(&self, s: State<'_>) -> &'static str {
        if s.is("disabled") {
            "disabled-text"
        } else if s.is("error") {
            "status-failed"
        } else {
            "primary-text"
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub(crate) enum Icon {
    Check,
    Chevron,
    Down,
    Close,
    Plus,
    Search,
    Folder,
    File,
    Info,
    Spinner,
}
impl Icon {
    pub(super) const ALL: [(Self, &'static str); 10] = [
        (Self::Check, "check"),
        (Self::Chevron, "chevron-right"),
        (Self::Down, "chevron-down"),
        (Self::Close, "close"),
        (Self::Plus, "plus"),
        (Self::Search, "search"),
        (Self::Folder, "folder"),
        (Self::File, "file"),
        (Self::Info, "info"),
        (Self::Spinner, "spinner"),
    ];
}
pub(crate) fn draw_icon_tinted(
    ctx: &mut PageCtx<'_>,
    scene: &mut Scene,
    rect: Rect,
    icon: Icon,
    tint: Rgba,
) {
    Painter {
        text: ctx.text,
        scene,
        theme: ctx.theme,
        clip: rect.inset(-Stroke::Emphasis.value()),
    }
    .icon_with_color(rect, icon, tint);
}
fn at(r: Rect, x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect::new(r.min_x() + x, r.min_y() + y, w.max(0.), h.max(0.))
}
fn center(r: Rect, w: f32, h: f32) -> Rect {
    Rect::new(
        r.center().x - w.min(r.size.width) * 0.5,
        r.center().y - h.min(r.size.height) * 0.5,
        w.min(r.size.width),
        h.min(r.size.height),
    )
}
fn text_slot(r: Rect, inset: f32) -> Rect {
    Rect::new(
        r.min_x() + inset,
        r.min_y(),
        (r.size.width - 2. * inset).max(0.),
        r.size.height,
    )
}
fn line_slot(r: Rect, line: usize) -> Rect {
    at(
        r,
        INSET,
        INSET + line as f32 * (proposal::TextRole::UiBody.style().line_height + INSET),
        r.size.width - 2. * INSET,
        TITLE_H,
    )
}

pub fn build(
    category: &str,
    ctx: &mut PageCtx<'_>,
    content: Rect,
    scene: &mut Scene,
    state: &ShowcaseState,
) {
    let mut p = Painter {
        text: &mut *ctx.text,
        scene,
        theme: ctx.theme,
        clip: content,
    };
    p.caption(
        at(content, 0., 0., content.size.width, INTRO_H - INSET),
        "固定状态样本 · 点击状态切换 · 非完整控件验收",
    );
    for (index, id) in catalog::group(category).component_ids.iter().enumerate() {
        let component = catalog::component(id);
        let tile = tile_rect(category, content, index);
        let selected = state.selected(component);
        p.clip = tile;
        p.label(
            at(tile, 0., 0., tile.size.width, TITLE_H),
            &component.name,
            proposal::TextRole::UiHeading,
            "primary-text",
        );
        for (id, rect) in chip_rects(component, tile) {
            let active = id == selected;
            p.box_role(
                rect,
                if active { "row-selection" } else { "surface-1" },
                "radius-badge",
                None,
            );
            p.label(
                rect.inset(SMALL),
                state_label(id),
                proposal::TextRole::UiCaption,
                if active {
                    "primary-text"
                } else {
                    "secondary-text"
                },
            );
        }
        let preview = preview_rect(component, tile);
        p.clip = preview;
        draw_component(&mut p, &component.id, State(selected), preview);
    }
}

fn draw_component(p: &mut Painter<'_>, id: &str, s: State<'_>, r: Rect) {
    match id {
        "tui.text" => {
            p.label(
                line_slot(r, 0),
                "清晰的文字与信息层级",
                proposal::TextRole::UiHeading,
                "primary-text",
            );
            p.body(line_slot(r, 1), "你好，世界 · Make something useful.");
            p.caption(line_slot(r, 2), "正文 13 / 16 · 辅助信息保持可读");
        }
        "tui.icon" => {
            let size = metric("size", "size-icon-preview");
            for (i, kind) in [
                Icon::Search,
                Icon::Folder,
                Icon::File,
                Icon::Plus,
                Icon::Check,
                Icon::Close,
            ]
            .into_iter()
            .enumerate()
            {
                p.icon(
                    at(r, INSET + i as f32 * (size + GAP), INSET, size, size),
                    kind,
                    "primary-text",
                );
            }
            p.caption(line_slot(r, 2), "原生图元绘制 · 不使用 emoji 或图标字体");
        }
        "tui.separator" => {
            p.body(line_slot(r, 0), "文档与设置");
            let y = r.min_y() + INSET + proposal::TextRole::UiBody.style().line_height + INSET;
            p.quad(
                Rect::new(
                    r.min_x() + INSET,
                    y,
                    r.size.width - 2. * INSET,
                    Stroke::Rule.value(),
                ),
                color(p.theme, "separator"),
                0.,
                None,
            );
            p.caption(line_slot(r, 2), "装饰分隔不承担交互边界");
        }
        "tui.stack" => {
            let labels = ["标题", "内容", "操作"];
            let width = (r.size.width - 4. * INSET) / 3.;
            for (i, label) in labels.iter().enumerate() {
                let q = at(
                    r,
                    INSET + i as f32 * (width + INSET),
                    INSET,
                    width,
                    metric("size", "size-button-sm"),
                );
                p.box_role(q, "surface-0", "radius-row", None);
                p.body(text_slot(q, INSET), label);
            }
            p.caption(line_slot(r, 2), "水平堆栈 · 统一间距与垂直对齐");
        }
        "tui.grid" => {
            let width = (r.size.width - 4. * INSET) / 3.;
            let height =
                ((r.size.height - INSET) / 2. - INSET).min(metric("size", "size-row-height"));
            for row in 0..2 {
                for col in 0..3 {
                    let q = at(
                        r,
                        INSET + col as f32 * (width + INSET),
                        INSET + row as f32 * (height + INSET),
                        width,
                        height,
                    );
                    p.box_role(q, "surface-0", "radius-badge", None);
                    p.body(
                        text_slot(q, INSET),
                        ["概览", "项目", "文档", "收藏", "草稿", "归档"][row * 3 + col],
                    );
                }
            }
        }
        "tui.split-view" => {
            let q = r.inset(INSET);
            let ratio = if s.is("dragging") { 0.6 } else { 0.36 };
            let left = Rect::new(
                q.min_x(),
                q.min_y(),
                q.size.width * ratio - INSET / 2.,
                q.size.height,
            );
            let right = Rect::new(
                q.min_x() + q.size.width * ratio + INSET / 2.,
                q.min_y(),
                q.size.width * (1. - ratio) - INSET / 2.,
                q.size.height,
            );
            p.box_role(left, "surface-sidebar", "radius-panel", None);
            p.box_role(
                right,
                "surface-1",
                "radius-panel",
                Some(if s.is("focus") {
                    "border-focused"
                } else {
                    "border"
                }),
            );
            p.label(
                line_slot(left, 0),
                "资源目录",
                proposal::TextRole::UiBody,
                p.status_ink(s),
            );
            p.label(
                line_slot(right, 0),
                if s.is("disabled") {
                    "分隔调整已禁用"
                } else {
                    "文档编辑区"
                },
                proposal::TextRole::UiBody,
                p.status_ink(s),
            );
            let divider = Rect::new(
                q.min_x() + q.size.width * ratio - Stroke::Rule.value(),
                q.min_y(),
                Stroke::Emphasis.value(),
                q.size.height,
            );
            p.quad(
                divider,
                color(
                    p.theme,
                    if s.is("pressed") {
                        "accent-pressed"
                    } else if s.is("hover") {
                        "accent-hover"
                    } else if s.is("dragging") {
                        "accent-outline"
                    } else {
                        "separator"
                    },
                ),
                0.,
                None,
            );
            if s.is("dragging") {
                p.caption(line_slot(right, 2), "拖动位置 · 静态帧");
            }
        }
        "tui.scroll-area" => {
            let q = r.inset(INSET);
            p.control(q, s, "surface-1", "radius-panel");
            let rows = ((q.size.height - INSET) / 24.).floor() as usize;
            let start = if s.is("at-end") {
                7
            } else if s.is("scrolling") {
                3
            } else {
                0
            };
            for i in 0..rows.min(if s.is("at-start") || s.is("at-end") {
                3
            } else {
                4
            }) {
                p.body(
                    at(
                        q,
                        INSET,
                        SMALL + i as f32 * 24.,
                        q.size.width - 3. * INSET,
                        24.,
                    ),
                    [
                        "项目概览",
                        "工作记录",
                        "设计规范",
                        "会议纪要",
                        "版本说明",
                        "文档索引",
                        "历史归档",
                        "最后一项",
                        "已到列表末尾",
                        "后续无内容",
                        "后续无内容",
                    ][start + i],
                );
            }
            let thumb = metric("size", "size-row-height").min(q.size.height * 0.4);
            let y = if s.is("at-end") {
                q.max_y() - thumb - INSET
            } else if s.is("scrolling") {
                q.center().y - thumb * 0.5
            } else {
                q.min_y() + INSET
            };
            p.quad(
                Rect::new(q.max_x() - INSET, y, Stroke::Emphasis.value(), thumb),
                color(
                    p.theme,
                    if s.is("disabled") {
                        "disabled-text"
                    } else {
                        "secondary-text"
                    },
                ),
                Stroke::Rule.value(),
                None,
            );
            if s.is("at-start") || s.is("at-end") {
                p.caption(
                    Rect::new(
                        q.min_x() + INSET,
                        q.max_y() - 24.,
                        q.size.width - 3. * INSET,
                        24.,
                    ),
                    if s.is("at-start") {
                        "已到起点 · 无更早内容"
                    } else {
                        "已到末尾 · 无后续内容"
                    },
                );
            }
        }
        "tui.disclosure" => {
            let header = at(
                r,
                INSET,
                INSET,
                r.size.width - 2. * INSET,
                metric("size", "size-row-height"),
            );
            p.control(header, s, "surface-1", "radius-row");
            let expanded = s.is("expanded");
            p.icon(
                at(header, INSET, INSET, 16., 16.),
                if expanded { Icon::Down } else { Icon::Chevron },
                p.status_ink(s),
            );
            p.body(
                at(header, 32., 0., header.size.width - 40., header.size.height),
                "项目详情",
            );
            if expanded {
                p.body(line_slot(r, 2), "  负责人：示例团队 · 3 个任务");
            } else if s.is("loading") {
                p.caption(line_slot(r, 2), "详情加载 · 静态帧");
            } else if s.is("error") {
                p.caption(line_slot(r, 2), "详情暂不可用，请重试");
            } else if s.is("empty") {
                p.caption(line_slot(r, 2), "暂无详情");
            }
        }
        "tui.button" => button(p, s, r, false),
        "tui.icon-button" => button(p, s, r, true),
        "tui.link" => {
            let q = center(r, 220., metric("size", "size-button-sm"));
            let ink = if s.is("disabled") {
                "disabled-text"
            } else if s.is("pressed") {
                "accent-pressed"
            } else if s.is("hover") {
                "accent-hover"
            } else {
                "accent"
            };
            p.label(q, "查看设计规范", proposal::TextRole::UiBody, ink);
            if s.is("hover") || s.is("pressed") {
                p.line(
                    Point::new(q.min_x(), q.max_y() - SMALL),
                    Point::new(q.min_x() + 92., q.max_y() - SMALL),
                    ink,
                );
            }
            if s.is("focus") {
                p.quad(
                    q,
                    clear(p.theme),
                    metric("radius", "radius-badge"),
                    Some(color(p.theme, "border-focused")),
                );
            }
        }
        "tui.checkbox" => choice(p, s, r, Choice::Checkbox),
        "tui.radio-group" => choice(p, s, r, Choice::Radio),
        "tui.switch" => choice(p, s, r, Choice::Switch),
        "tui.text-field" => field(p, s, r, Field::Text),
        "tui.text-area" => field(p, s, r, Field::Area),
        "tui.search-field" => field(p, s, r, Field::Search),
        "tui.combo-box" => combo(p, s, r),
        "tui.slider" => {
            let q = center(
                r,
                r.size.width - 4. * INSET,
                metric("size", "size-row-height"),
            );
            let ratio = if s.is("dragging") {
                0.72
            } else if s.is("pressed") {
                0.6
            } else {
                0.4
            };
            let y = q.center().y;
            p.quad(
                Rect::new(q.min_x(), y, q.size.width, Stroke::Emphasis.value()),
                color(p.theme, "separator"),
                Stroke::Rule.value(),
                None,
            );
            p.quad(
                Rect::new(q.min_x(), y, q.size.width * ratio, Stroke::Emphasis.value()),
                color(
                    p.theme,
                    if s.is("disabled") {
                        "disabled-text"
                    } else {
                        "progress-active"
                    },
                ),
                Stroke::Rule.value(),
                None,
            );
            let knob = metric("size", "size-selector-knob");
            p.control(
                Rect::new(
                    q.min_x() + q.size.width * ratio - knob / 2.,
                    y - knob / 2.,
                    knob,
                    knob,
                ),
                s,
                "surface-2",
                "radius-pill",
            );
            p.caption(
                at(
                    r,
                    INSET,
                    r.max_y() - r.min_y() - 32.,
                    r.size.width - 2. * INSET,
                    24.,
                ),
                if s.is("dragging") {
                    "72% · 拖动静态帧"
                } else {
                    "40% · 数值状态样本"
                },
            );
        }
        "tui.tabs" => tabs(p, s, r, false),
        "tui.segmented-control" => tabs(p, s, r, true),
        "tui.list" => listing(p, s, r, false),
        "tui.tree" => listing(p, s, r, true),
        "tui.breadcrumb" => breadcrumb(p, s, r),
        "tui.command-palette" => command_palette(p, s, r),
        "tui.menu" => menu(p, s, r),
        "tui.tooltip" => overlay(p, s, r, Overlay::Tooltip),
        "tui.popover" => overlay(p, s, r, Overlay::Popover),
        "tui.dialog" => overlay(p, s, r, Overlay::Dialog),
        "tui.toast" => feedback(p, s, r, Feedback::Toast),
        "tui.banner" => feedback(p, s, r, Feedback::Banner),
        "tui.status-badge" => feedback(p, s, r, Feedback::Badge),
        "tui.progress" => progress(p, s, r),
        "tui.empty-state" => feedback(p, s, r, Feedback::Empty),
        "tui.selection-surface" => document(p, s, r, Document::Selection),
        "tui.markdown" => document(p, s, r, Document::Markdown),
        "tui.code-block" => document(p, s, r, Document::Code),
        "tui.diff-view" => document(p, s, r, Document::Diff),
        "tui.data-table" => table(p, s, r),
        "tui.chart" => chart(p, s, r),
        _ => panic!("component has no illustration branch: {id}"),
    }
}

fn button(p: &mut Painter<'_>, s: State<'_>, r: Rect, icon_only: bool) {
    let h = metric(
        "size",
        if icon_only {
            "size-icon-button"
        } else {
            "size-button-md"
        },
    );
    let q = center(r, if icon_only { h } else { 180. }, h);
    let fill = if s.is("disabled") {
        "button-hover"
    } else if s.is("pressed") {
        "action-primary-pressed"
    } else if s.is("hover") {
        "action-primary-hover"
    } else {
        "action-primary-ground"
    };
    let ink = if s.is("disabled") {
        "disabled-text"
    } else {
        "action-primary-foreground"
    };
    p.box_role(
        q,
        fill,
        "radius-button-md",
        if s.is("focus") {
            Some("action-primary-focus-outline")
        } else {
            None
        },
    );
    if icon_only {
        let icon = center(
            q,
            metric("size", "size-icon-standard"),
            metric("size", "size-icon-standard"),
        );
        p.icon(
            icon,
            if s.is("loading") {
                Icon::Spinner
            } else {
                Icon::Plus
            },
            ink,
        );
    } else {
        let label = if s.is("loading") {
            "保存中 · 静态帧"
        } else {
            "保存更改"
        };
        p.label(
            text_slot(q, INSET),
            label,
            proposal::TextRole::ButtonMd,
            ink,
        );
    }
    if icon_only && s.is("loading") {
        p.caption(
            at(
                r,
                INSET,
                q.max_y() - r.min_y() + INSET,
                r.size.width - 2. * INSET,
                24.,
            ),
            "加载图标 · 无动画",
        );
    }
}
#[derive(Clone, Copy)]
enum Choice {
    Checkbox,
    Radio,
    Switch,
}
fn choice(p: &mut Painter<'_>, s: State<'_>, r: Rect, kind: Choice) {
    let q = center(
        r,
        r.size.width - 4. * INSET,
        metric("size", "size-row-height"),
    );
    let checked = s.is("checked");
    let ink = p.status_ink(s);
    let side = metric("size", "size-inline-badge");
    let w = if matches!(kind, Choice::Switch) {
        metric("size", "size-selector-track") * 2.
    } else {
        side
    };
    let control = at(q, 0., (q.size.height - side) / 2., w, side);
    let fill = if checked {
        "control-checked-ground"
    } else if s.is("pressed") {
        "button-pressed"
    } else if s.is("hover") {
        "button-hover"
    } else {
        "surface-1"
    };
    let border = if s.is("error") {
        "status-failed"
    } else if s.is("focus") {
        "border-focused"
    } else {
        "border"
    };
    p.box_role(
        control,
        fill,
        if matches!(kind, Choice::Checkbox) {
            "radius-badge"
        } else {
            "radius-pill"
        },
        Some(border),
    );
    match kind {
        Choice::Checkbox => {
            if checked {
                p.icon(
                    control.inset(Spacing::Micro.value()),
                    Icon::Check,
                    if s.is("disabled") {
                        "disabled-text"
                    } else {
                        "control-checked-mark"
                    },
                );
            }
            if s.is("mixed") {
                p.line(
                    Point::new(control.min_x() + SMALL, control.center().y),
                    Point::new(control.max_x() - SMALL, control.center().y),
                    "primary-text",
                );
            }
        }
        Choice::Radio => {
            if checked {
                p.quad(
                    control.inset(SMALL),
                    color(p.theme, "control-checked-mark"),
                    side,
                    None,
                );
            }
            let other = control.offset(0., 36.);
            if other.max_y() < r.max_y() {
                p.box_role(other, "surface-1", "radius-pill", Some("border"));
                p.body(at(other, side + INSET, 0., 200., side), "仅当前项目");
            }
        }
        Choice::Switch => {
            let thumb = side - SMALL;
            let x = if checked {
                control.max_x() - thumb - Spacing::Micro.value()
            } else {
                control.min_x() + Spacing::Micro.value()
            };
            p.quad(
                Rect::new(x, control.min_y() + Spacing::Micro.value(), thumb, thumb),
                color(
                    p.theme,
                    if checked {
                        "control-checked-mark"
                    } else {
                        "secondary-text"
                    },
                ),
                thumb,
                None,
            );
        }
    }
    p.label(
        at(q, w + INSET, 0., q.size.width - w - INSET, q.size.height),
        match kind {
            Choice::Checkbox => "接收更新通知",
            Choice::Radio => "所有项目",
            Choice::Switch => {
                if checked {
                    "自动保存已开启"
                } else {
                    "自动保存已关闭"
                }
            }
        },
        proposal::TextRole::UiBody,
        ink,
    );
    if s.is("error") {
        p.label(
            at(
                r,
                INSET,
                r.size.height - 24.,
                r.size.width - 2. * INSET,
                24.,
            ),
            "请选择一个有效选项",
            proposal::TextRole::UiCaption,
            "status-failed",
        );
    }
}
#[derive(Clone, Copy)]
enum Field {
    Text,
    Area,
    Search,
}
fn field(p: &mut Painter<'_>, s: State<'_>, r: Rect, kind: Field) {
    let h = if matches!(kind, Field::Area) {
        metric("size", "size-button-md") * 2.
    } else {
        metric("size", "size-button-md")
    };
    let q = Rect::new(
        r.min_x() + INSET,
        r.min_y() + INSET,
        r.size.width - 2. * INSET,
        h.min(r.size.height - 40.),
    );
    p.control(q, s, "transcript-input", "radius-input");
    let inset = if matches!(kind, Field::Search) {
        metric("size", "size-icon-standard") + 2. * INSET
    } else {
        INSET
    };
    if matches!(kind, Field::Search) {
        p.icon(at(q, INSET, INSET, 16., 16.), Icon::Search, p.status_ink(s));
    }
    let label = if s.is("composing") {
        "qing"
    } else if matches!(kind, Field::Search) {
        "设计规范"
    } else if matches!(kind, Field::Area) {
        "记录一个清晰的想法。"
    } else {
        "示例项目名称"
    };
    let role = if matches!(kind, Field::Area) {
        proposal::TextRole::ComposerBody
    } else {
        proposal::TextRole::UiBody
    };
    let text_r = if matches!(kind, Field::Area) {
        at(
            q,
            inset,
            INSET,
            q.size.width - inset - INSET,
            role.style().line_height,
        )
    } else {
        at(q, inset, 0.0, q.size.width - inset - INSET, q.size.height)
    };
    if s.is("selection") {
        p.text
            .proposal_selected_in(p.scene, p.theme, text_r, label, role, false);
    } else {
        p.label(text_r, label, role, p.status_ink(s));
    }
    if matches!(kind, Field::Area) {
        p.caption(
            at(q, INSET, 32.0, q.size.width - 2.0 * INSET, 24.0),
            "第二行内容 · 多行输入预览",
        );
    }
    let paragraph = p.text.proposal_label(label, role, None);
    let origin =
        crate::gallery::text::label_origin(text_r, &paragraph, crate::gallery::text::HAlign::Left);
    if s.is("focus") || s.is("composing") {
        let caret = paragraph
            .caret_rect(paragraph.utf16_len, Stroke::Rule.value())
            .offset(origin.x, origin.y);
        p.quad(caret, color(p.theme, "primary-text"), 0.0, None);
    }
    if s.is("composing") {
        for rect in
            paragraph.selection_rects(0..paragraph.utf16_len, text_r.offset(-origin.x, -origin.y))
        {
            p.line(
                Point::new(origin.x + rect.min_x(), origin.y + rect.max_y()),
                Point::new(origin.x + rect.max_x(), origin.y + rect.max_y()),
                "primary-text",
            );
        }
    }
    p.label(
        at(
            r,
            INSET,
            q.max_y() - r.min_y() + SMALL,
            r.size.width - 2. * INSET,
            24.,
        ),
        if s.is("error") {
            "名称不可为空 · 校验样本"
        } else if s.is("read-only") {
            "只读内容 · 不接收键入"
        } else if s.is("composing") {
            "组字标记样本 · 非真实 IME"
        } else {
            "固定文本内容 · 不接收键入"
        },
        proposal::TextRole::UiCaption,
        if s.is("error") {
            "status-failed"
        } else {
            "secondary-text"
        },
    );
}
fn combo(p: &mut Painter<'_>, s: State<'_>, r: Rect) {
    let q = at(
        r,
        INSET,
        INSET,
        r.size.width - 2. * INSET,
        metric("size", "size-button-md"),
    );
    p.control(q, s, "transcript-input", "radius-input");
    p.label(
        text_slot(q, INSET),
        if s.is("selected") {
            "设计项目"
        } else if s.is("empty") {
            "没有匹配项"
        } else if s.is("loading") {
            "加载选项 · 静态帧"
        } else {
            "选择项目"
        },
        proposal::TextRole::UiBody,
        p.status_ink(s),
    );
    p.icon(
        Rect::new(q.max_x() - 28., q.center().y - 8., 16., 16.),
        Icon::Down,
        p.status_ink(s),
    );
    if s.is("open") {
        let menu = Rect::new(
            q.min_x(),
            q.max_y() + SMALL,
            q.size.width,
            (r.max_y() - q.max_y() - SMALL).min(84.),
        );
        p.box_role(menu, "surface-popover", "radius-floating", Some("border"));
        for (i, label) in ["设计项目", "文档项目"].iter().enumerate() {
            p.body(
                at(
                    menu,
                    INSET,
                    SMALL + i as f32 * 28.,
                    menu.size.width - 2. * INSET,
                    24.,
                ),
                label,
            );
        }
    } else if s.is("error") {
        p.label(
            line_slot(r, 2),
            "无法读取选项，请重试",
            proposal::TextRole::UiCaption,
            "status-failed",
        );
    }
}

fn tabs(p: &mut Painter<'_>, s: State<'_>, r: Rect, segmented: bool) {
    let q = at(
        r,
        INSET,
        INSET,
        r.size.width - 2. * INSET,
        metric("size", "size-button-md"),
    );
    if segmented {
        p.box_role(q, "segmented-track", "radius-pill", None);
    }
    let width = q.size.width / 3.;
    let selected = if s.is("selected") { 1 } else { 0 };
    for (i, label) in ["概览", "活动", "文件"].iter().enumerate() {
        let tab = Rect::new(
            q.min_x() + i as f32 * width,
            q.min_y(),
            width,
            q.size.height,
        );
        if i == selected {
            if segmented {
                p.box_role(
                    tab.inset(Spacing::Micro.value()),
                    "segmented-thumb",
                    "radius-pill",
                    None,
                );
            } else {
                p.quad(
                    Rect::new(
                        tab.min_x(),
                        tab.max_y() - Stroke::Emphasis.value(),
                        tab.size.width,
                        Stroke::Emphasis.value(),
                    ),
                    color(p.theme, "accent-outline"),
                    0.,
                    None,
                );
            }
        } else if i == 1 && (s.is("hover") || s.is("pressed")) {
            p.box_role(
                tab,
                if s.is("pressed") {
                    "button-pressed"
                } else {
                    "button-hover"
                },
                "radius-row",
                None,
            );
        }
        p.label(
            text_slot(tab, INSET),
            label,
            proposal::TextRole::UiBody,
            p.status_ink(s),
        );
        if i == selected && s.is("focus") {
            p.quad(
                tab,
                clear(p.theme),
                metric("radius", "radius-row"),
                Some(color(p.theme, "border-focused")),
            );
        }
    }
    p.caption(
        line_slot(r, 3),
        if s.is("empty") {
            "当前分类没有内容"
        } else if s.is("loading") {
            "内容加载 · 静态帧"
        } else if s.is("error") {
            "内容加载失败"
        } else {
            "概览内容 · 选中标记保持可见"
        },
    );
}
fn listing(p: &mut Painter<'_>, s: State<'_>, r: Rect, tree: bool) {
    if s.is("empty") || s.is("loading") || s.is("error") {
        p.label(
            line_slot(r, 1),
            if s.is("empty") {
                "暂无项目"
            } else if s.is("error") {
                "无法加载列表"
            } else {
                "列表加载中 · 静态帧"
            },
            proposal::TextRole::UiBody,
            if s.is("error") {
                "status-failed"
            } else {
                "secondary-text"
            },
        );
        return;
    }
    let labels = if tree {
        ["设计资源", "颜色与图标", "组件说明"]
    } else {
        ["项目概览", "设计规范", "会议记录"]
    };
    let height = metric("size", "size-compact-row");
    for (i, label) in labels.iter().enumerate() {
        if tree && i > 0 && !s.is("expanded") {
            continue;
        }
        let active = if tree && !s.is("expanded") { 0 } else { 1 };
        let row = at(
            r,
            INSET,
            INSET + i as f32 * (height + SMALL),
            r.size.width - 2. * INSET,
            height,
        );
        if i == active && (s.is("selected") || s.is("focus") || s.is("hover") || s.is("pressed")) {
            p.control(
                row,
                s,
                if s.is("selected") {
                    "row-selection"
                } else {
                    "button-hover"
                },
                "radius-row",
            );
            if s.is("selected") {
                p.quad(
                    at(row, 0., 0., Stroke::Emphasis.value(), row.size.height),
                    color(p.theme, "selection-accent-marker"),
                    0.,
                    None,
                );
            }
        }
        let indent = if tree && i > 0 { GAP } else { 0. };
        if tree {
            p.icon(
                at(row, SMALL + indent, SMALL, 16., 16.),
                if i == 0 {
                    if s.is("expanded") {
                        Icon::Down
                    } else {
                        Icon::Chevron
                    }
                } else {
                    Icon::File
                },
                p.status_ink(s),
            );
            if i > 0 && !s.is("expanded") {
                continue;
            }
        }
        p.label(
            at(
                row,
                if tree { 28. + indent } else { INSET },
                0.,
                row.size.width - 64.,
                height,
            ),
            label,
            proposal::TextRole::UiBody,
            p.status_ink(s),
        );
    }
    if tree && !s.is("expanded") {
        p.caption(line_slot(r, 3), "折叠目录 · 选择展开状态查看子项");
    }
}
fn breadcrumb(p: &mut Painter<'_>, s: State<'_>, r: Rect) {
    let q = center(
        r,
        r.size.width - 2. * INSET,
        metric("size", "size-button-md"),
    );
    let step = q.size.width / 3.;
    for (i, label) in ["工作空间", "设计项目", "组件说明"].iter().enumerate() {
        let part = at(q, i as f32 * step, 0., step - 20., q.size.height);
        if i == 1 && (s.is("hover") || s.is("pressed") || s.is("focus")) {
            p.control(part, s, "surface-1", "radius-row");
        }
        p.label(
            part,
            label,
            proposal::TextRole::UiBody,
            if i < 2 && !s.is("disabled") {
                "accent"
            } else {
                p.status_ink(s)
            },
        );
        if i < 2 {
            p.icon(
                at(q, (i + 1) as f32 * step - 20., INSET, 16., 16.),
                Icon::Chevron,
                "secondary-text",
            );
        }
    }
}
fn command_palette(p: &mut Painter<'_>, s: State<'_>, r: Rect) {
    if s.is("rest") {
        p.body(line_slot(r, 1), "命令面板未打开 · 选择“打开”查看");
        return;
    }
    let q = r.inset(INSET);
    p.box_role(
        q,
        "surface-popover",
        "radius-floating",
        Some(if s.is("focus") {
            "border-focused"
        } else {
            "border"
        }),
    );
    p.icon(
        at(q, INSET, INSET, 16., 16.),
        Icon::Search,
        "secondary-text",
    );
    p.body(
        at(q, 32., SMALL, q.size.width - 40., 28.),
        "查找命令 · 文字为固定样本",
    );
    let labels = if s.is("empty") {
        ["没有匹配命令", "尝试其他关键词", ""]
    } else if s.is("loading") {
        ["搜索命令 · 静态帧", "请稍候", ""]
    } else if s.is("error") {
        ["无法读取命令", "请稍后重试", ""]
    } else {
        ["新建文档", "打开项目", "切换主题"]
    };
    for (i, label) in labels.iter().enumerate() {
        let row = at(
            q,
            SMALL,
            36. + i as f32 * 28.,
            q.size.width - 2. * SMALL,
            24.,
        );
        if row.max_y() > q.max_y() - SMALL {
            break;
        }
        if i == 0 {
            p.control(
                row,
                s,
                if s.is("selected") {
                    "row-selection"
                } else {
                    "button-hover"
                },
                "radius-row",
            );
        }
        p.label(
            row.inset(SMALL),
            label,
            proposal::TextRole::UiBody,
            p.status_ink(s),
        );
    }
}
fn menu(p: &mut Painter<'_>, s: State<'_>, r: Rect) {
    if s.is("rest") {
        let q = center(r, 160., 32.);
        p.control(q, s, "surface-1", "radius-control");
        p.body(text_slot(q, INSET), "操作菜单 · 未打开");
        return;
    }
    let q = center(
        r,
        (r.size.width * 0.7).min(280.),
        (r.size.height - 2. * INSET).min(164.),
    );
    p.box_role(q, "surface-popover", "radius-floating", Some("border"));
    let labels = if s.is("empty") {
        ["暂无操作", "", ""]
    } else if s.is("loading") {
        ["加载操作 · 静态帧", "", ""]
    } else if s.is("error") {
        ["无法读取操作", "", ""]
    } else {
        ["新建副本", "重命名", "移到归档"]
    };
    for (i, label) in labels.iter().enumerate() {
        let row = at(
            q,
            SMALL,
            SMALL + i as f32 * 36.,
            q.size.width - 2. * SMALL,
            32.,
        );
        if row.max_y() > q.max_y() {
            break;
        }
        if i == 1 {
            p.control(
                row,
                s,
                if s.is("selected") {
                    "row-selection"
                } else if s.is("hover") {
                    "button-hover"
                } else {
                    "surface-popover"
                },
                "radius-row",
            );
        }
        p.label(
            text_slot(row, INSET),
            label,
            proposal::TextRole::UiBody,
            p.status_ink(s),
        );
    }
}
#[derive(Clone, Copy)]
enum Overlay {
    Tooltip,
    Popover,
    Dialog,
}
fn overlay(p: &mut Painter<'_>, s: State<'_>, r: Rect, kind: Overlay) {
    let trigger = Rect::new(
        r.min_x() + INSET,
        r.max_y() - 44.,
        (r.size.width * 0.5).min(220.),
        32.,
    );
    p.control(trigger, State("rest"), "surface-1", "radius-control");
    p.body(
        text_slot(trigger, INSET),
        match kind {
            Overlay::Tooltip => "查看提示",
            Overlay::Popover => "共享设置",
            Overlay::Dialog => "打开确认对话框",
        },
    );
    if !s.is("open") {
        p.caption(
            line_slot(r, 0),
            if s.is("closing") {
                "关闭过渡末帧 · 无动画"
            } else {
                "未打开 · 通过状态切换查看"
            },
        );
        return;
    }
    match kind {
        Overlay::Tooltip => {
            let q = Rect::new(
                trigger.min_x(),
                trigger.min_y() - 44.,
                (r.size.width - 2. * INSET).min(320.),
                32.,
            );
            p.box_role(q, "surface-popover", "radius-floating", Some("border"));
            p.body(text_slot(q, INSET), "为当前项目设置共享权限");
        }
        Overlay::Popover => {
            let q = Rect::new(
                r.min_x() + INSET,
                r.min_y() + INSET,
                (r.size.width - 2. * INSET).min(340.),
                (r.size.height - 64.).min(156.),
            );
            p.box_role(q, "surface-popover", "radius-floating", Some("border"));
            p.label(
                line_slot(q, 0),
                "共享设置",
                proposal::TextRole::UiLabel,
                "primary-text",
            );
            p.body(line_slot(q, 1), "仅项目成员可访问");
            p.icon(
                Rect::new(q.max_x() - 28., q.min_y() + INSET, 16., 16.),
                Icon::Close,
                "secondary-text",
            );
        }
        Overlay::Dialog => {
            p.quad(
                r,
                color(p.theme, "scrim"),
                metric("radius", "radius-panel"),
                None,
            );
            let q = center(
                r,
                (r.size.width - 4. * INSET).min(370.),
                (r.size.height - 4. * INSET).min(176.),
            );
            p.box_role(q, "surface-popover", "radius-dialog", Some("border"));
            p.label(
                line_slot(q, 0),
                "将文档移到归档？",
                proposal::TextRole::UiHeading,
                "primary-text",
            );
            p.body(line_slot(q, 1), "此操作保留内容，可随时恢复。");
            let button = Rect::new(q.max_x() - 108., q.max_y() - 44., 92., 32.);
            p.box_role(button, "action-primary-ground", "radius-button-md", None);
            p.label(
                text_slot(button, INSET),
                "确认归档",
                proposal::TextRole::ButtonMd,
                "action-primary-foreground",
            );
            p.body(
                Rect::new(button.min_x() - 80., button.min_y(), 72., 32.),
                "取消",
            );
        }
    }
}
#[derive(Clone, Copy)]
enum Feedback {
    Toast,
    Banner,
    Badge,
    Empty,
}
fn feedback(p: &mut Painter<'_>, s: State<'_>, r: Rect, kind: Feedback) {
    let (ink, message) = if s.is("error") {
        ("status-failed", "无法保存更改")
    } else if s.is("success") {
        ("status-succeeded", "更改已保存")
    } else if s.is("restricted") {
        ("warning-foreground", "当前权限不足")
    } else if s.is("loading") {
        ("status-running", "正在处理 · 静态帧")
    } else if s.is("unverified") {
        ("secondary-text", "尚未验证")
    } else if s.is("cancelled") {
        ("secondary-text", "操作已取消")
    } else {
        ("status-info", "已准备就绪")
    };
    match kind {
        Feedback::Toast => {
            let q = center(
                r,
                (r.size.width - 2. * INSET).min(360.),
                metric("size", "size-button-lg"),
            );
            p.box_role(q, "surface-popover", "radius-floating", Some("border"));
            p.icon(
                at(q, INSET, INSET, 24., 24.),
                if s.is("success") {
                    Icon::Check
                } else if s.is("loading") {
                    Icon::Spinner
                } else {
                    Icon::Info
                },
                ink,
            );
            p.body(at(q, 40., 0., q.size.width - 64., q.size.height), message);
            p.icon(
                Rect::new(q.max_x() - 24., q.center().y - 8., 16., 16.),
                Icon::Close,
                "secondary-text",
            );
        }
        Feedback::Banner => {
            let q = center(
                r,
                r.size.width - 2. * INSET,
                metric("size", "size-button-lg") + GAP,
            );
            p.box_role(
                q,
                if s.is("error") {
                    "diff-removed-surface"
                } else {
                    "surface-0"
                },
                "radius-panel",
                Some("border"),
            );
            p.icon(at(q, INSET, INSET, 24., 24.), Icon::Info, ink);
            p.label(
                at(q, 40., INSET, q.size.width - 48., 24.),
                message,
                proposal::TextRole::UiLabel,
                ink,
            );
            p.caption(
                at(q, 40., 32., q.size.width - 48., 16.),
                "状态同时用文字说明，不只依赖颜色",
            );
        }
        Feedback::Badge => {
            let q = center(r, 240., metric("size", "size-button-sm"));
            p.box_role(q, "surface-0", "radius-pill", None);
            p.quad(
                at(q, INSET, q.size.height * 0.5 - 3., 6., 6.),
                color(p.theme, ink),
                metric("radius", "radius-pill"),
                None,
            );
            p.label(
                at(q, 24., 0., q.size.width - 32., q.size.height),
                message,
                proposal::TextRole::UiBody,
                ink,
            );
        }
        Feedback::Empty => {
            let q = r.inset(INSET);
            p.icon(
                Rect::new(q.center().x - 12., q.min_y() + INSET, 24., 24.),
                if s.is("loading") {
                    Icon::Spinner
                } else {
                    Icon::Folder
                },
                ink,
            );
            p.label(
                at(q, INSET, 44., q.size.width - 2. * INSET, 24.),
                if s.is("rest") {
                    "这里还没有项目"
                } else {
                    message
                },
                proposal::TextRole::UiHeading,
                "primary-text",
            );
            p.caption(
                at(q, INSET, 76., q.size.width - 2. * INSET, 24.),
                if s.is("restricted") {
                    "请联系项目成员获取访问权限"
                } else {
                    "创建第一个项目，开始组织你的工作"
                },
            );
        }
    }
}
fn progress(p: &mut Painter<'_>, s: State<'_>, r: Rect) {
    let q = center(
        r,
        r.size.width - 4. * INSET,
        metric("size", "size-button-md"),
    );
    let y = q.center().y;
    let width = if s.is("success") {
        q.size.width
    } else if s.is("indeterminate") {
        q.size.width * 0.3
    } else {
        q.size.width * 0.58
    };
    p.quad(
        Rect::new(q.min_x(), y, q.size.width, SMALL),
        color(p.theme, "separator"),
        SMALL,
        None,
    );
    p.quad(
        Rect::new(
            q.min_x()
                + if s.is("indeterminate") {
                    q.size.width * 0.3
                } else {
                    0.
                },
            y,
            width,
            SMALL,
        ),
        color(
            p.theme,
            if s.is("error") {
                "status-failed"
            } else {
                "progress-active"
            },
        ),
        SMALL,
        None,
    );
    p.caption(
        at(
            r,
            INSET,
            q.max_y() - r.min_y() + SMALL,
            r.size.width - 2. * INSET,
            24.,
        ),
        if s.is("indeterminate") {
            "不定进度 · 固定动画帧"
        } else if s.is("loading") {
            "58% · 加载静态帧"
        } else if s.is("success") {
            "100% · 已完成"
        } else if s.is("error") {
            "58% · 处理失败"
        } else {
            "58% · 当前进度"
        },
    );
}
#[derive(Clone, Copy)]
enum Document {
    Selection,
    Markdown,
    Code,
    Diff,
}
fn document(p: &mut Painter<'_>, s: State<'_>, r: Rect, kind: Document) {
    let q = r.inset(INSET);
    p.control(
        q,
        s,
        if matches!(kind, Document::Code | Document::Diff) {
            "code-surface"
        } else {
            "document-canvas"
        },
        "radius-code",
    );
    if s.is("empty") || s.is("loading") || s.is("error") {
        p.label(
            line_slot(q, 1),
            if s.is("empty") {
                "文档暂无内容"
            } else if s.is("loading") {
                "文档加载 · 静态帧"
            } else {
                "无法读取文档内容"
            },
            proposal::TextRole::UiBody,
            if s.is("error") {
                "status-failed"
            } else {
                "secondary-text"
            },
        );
        return;
    }
    match kind {
        Document::Selection => {
            p.label(
                line_slot(q, 0),
                "选择与标注",
                proposal::TextRole::UiHeading,
                "primary-text",
            );
            let body = "被选择的文字保持清晰可读。文字按照可用宽度自然换行，使用共享的字体与选区颜色，不通过缩小字号来容纳内容。";
            let slot = at(
                q,
                INSET,
                line_slot(q, 2).min_y() - q.min_y(),
                q.size.width - 2.0 * INSET,
                64.0,
            );
            if s.is("selection") {
                p.text.proposal_selected_in(
                    p.scene,
                    p.theme,
                    slot,
                    body,
                    proposal::TextRole::DocBody,
                    true,
                );
            } else {
                p.wrapped(slot, body, proposal::TextRole::DocBody, "primary-text");
            }
            p.caption(line_slot(q, 6), "原生选区色 · 固定选择快照");
        }
        Document::Markdown => {
            p.label(
                line_slot(q, 0),
                "项目说明",
                proposal::TextRole::DocH3,
                "primary-text",
            );
            let body = "从一个清晰的想法开始。将相关的信息放在一起，保留文字层级、段落间距和自然换行，让说明在不同内容下依然易读。";
            let slot = at(
                q,
                INSET,
                line_slot(q, 2).min_y() - q.min_y(),
                q.size.width - 2.0 * INSET,
                48.0,
            );
            if s.is("selection") {
                p.text.proposal_selected_in(
                    p.scene,
                    p.theme,
                    slot,
                    body,
                    proposal::TextRole::DocBody,
                    true,
                );
            } else {
                p.wrapped(slot, body, proposal::TextRole::DocBody, "primary-text");
            }
            p.label(
                line_slot(q, 5),
                "查看使用指南",
                proposal::TextRole::UiBody,
                if s.is("hover") {
                    "accent-hover"
                } else if s.is("pressed") {
                    "accent-pressed"
                } else {
                    "accent"
                },
            );
            p.body(line_slot(q, 7), "• 先整理结构，再逐步完善内容。");
        }
        Document::Code => {
            p.label(
                line_slot(q, 0),
                "Rust · 示例",
                proposal::TextRole::UiCaption,
                "secondary-text",
            );
            let code = [
                ("fn main() {", "c-code-keyword"),
                ("    let title = \"Hello\";", "c-code-string"),
                ("    println!(\"{title}\");", "primary-text"),
                ("}", "primary-text"),
            ];
            for (i, (line, ink)) in code.iter().enumerate() {
                let slot = at(
                    q,
                    INSET,
                    36.0 + i as f32 * 24.0,
                    q.size.width - 2.0 * INSET,
                    24.0,
                );
                if s.is("selection") && i == 0 {
                    p.text.proposal_selected_in(
                        p.scene,
                        p.theme,
                        slot,
                        line,
                        proposal::TextRole::Code,
                        false,
                    );
                } else {
                    p.label(slot, line, proposal::TextRole::Code, ink);
                }
            }
        }
        Document::Diff => {
            p.caption(line_slot(q, 0), "src/settings.rs · 变更预览");
            for (i, (line, fill, ink)) in [
                (
                    "- let limit = 10;",
                    "diff-removed-surface",
                    "diff-removed-marker",
                ),
                (
                    "+ let limit = 20;",
                    "diff-added-surface",
                    "diff-added-marker",
                ),
                ("  save_settings();", "code-surface", "primary-text"),
            ]
            .iter()
            .enumerate()
            {
                let row = at(
                    q,
                    SMALL,
                    36. + i as f32 * 28.,
                    q.size.width - 2. * SMALL,
                    24.,
                );
                p.box_role(row, fill, "radius-square", None);
                let slot = Rect::new(
                    row.min_x() + SMALL,
                    row.min_y(),
                    row.size.width - 2.0 * SMALL,
                    row.size.height,
                );
                if s.is("selection") && i == 1 {
                    p.text.proposal_selected_in(
                        p.scene,
                        p.theme,
                        slot,
                        line,
                        proposal::TextRole::Code,
                        false,
                    );
                } else {
                    p.label(slot, line, proposal::TextRole::Code, ink);
                }
            }
        }
    }
}
fn table(p: &mut Painter<'_>, s: State<'_>, r: Rect) {
    let q = r.inset(INSET);
    let width = q.size.width;
    let h = metric("size", "size-row-height");
    let headers = [
        ("文档", 0., 0.55),
        ("状态", 0.55, 0.25),
        ("更新时间", 0.8, 0.2),
    ];
    for (label, x, w) in headers {
        p.label(
            at(q, width * x, 0., width * w, h),
            label,
            proposal::TextRole::UiLabel,
            "secondary-text",
        );
    }
    p.quad(
        at(q, 0., h, width, Stroke::Rule.value()),
        color(p.theme, "separator"),
        0.,
        None,
    );
    if s.is("empty") || s.is("loading") || s.is("error") {
        p.label(
            at(q, INSET, h + GAP, width - 2. * INSET, h),
            if s.is("empty") {
                "暂无数据"
            } else if s.is("loading") {
                "表格加载 · 静态帧"
            } else {
                "数据读取失败"
            },
            proposal::TextRole::UiBody,
            if s.is("error") {
                "status-failed"
            } else {
                "secondary-text"
            },
        );
        return;
    }
    for (i, (name, status, time)) in [
        ("设计规范", "已发布", "今天"),
        ("组件计划", "进行中", "昨天"),
        ("使用指南", "草稿", "周一"),
        ("版本说明", "待审核", "周一"),
    ]
    .iter()
    .enumerate()
    {
        let row = at(q, 0., (i + 1) as f32 * h, width, h);
        if row.max_y() > q.max_y() {
            break;
        }
        if i == 1 && (s.is("selected") || s.is("hover") || s.is("pressed") || s.is("focus")) {
            p.control(
                row,
                s,
                if s.is("selected") {
                    "row-selection"
                } else {
                    "button-hover"
                },
                "radius-row",
            );
        }
        for (label, x, w) in [(*name, 0., 0.55), (*status, 0.55, 0.25), (*time, 0.8, 0.2)] {
            p.label(
                at(row, INSET + width * x, 0., width * w - INSET, h),
                label,
                proposal::TextRole::UiBody,
                p.status_ink(s),
            );
        }
        p.quad(
            at(
                row,
                0.,
                h - Stroke::Rule.value(),
                width,
                Stroke::Rule.value(),
            ),
            color(p.theme, "separator"),
            0.,
            None,
        );
    }
}
fn chart(p: &mut Painter<'_>, s: State<'_>, r: Rect) {
    let q = r.inset(GAP);
    p.caption(at(q, 0., 0., q.size.width, 24.), "每周完成任务 · 示例数据");
    if s.is("empty") || s.is("loading") || s.is("error") {
        p.label(
            at(q, 0., 44., q.size.width, 32.),
            if s.is("empty") {
                "没有可绘制的数据"
            } else if s.is("loading") {
                "图表加载 · 静态帧"
            } else {
                "图表数据不可用"
            },
            proposal::TextRole::UiBody,
            if s.is("error") {
                "status-failed"
            } else {
                "secondary-text"
            },
        );
        return;
    }
    let plot = Rect::new(
        q.min_x() + 32.,
        q.min_y() + 44.,
        q.size.width - 48.,
        q.size.height - 76.,
    );
    let max = 32.;
    for tick in [0., 16., 32.] {
        let y = plot.max_y() - plot.size.height * tick / max;
        p.line(
            Point::new(plot.min_x(), y),
            Point::new(plot.max_x(), y),
            "separator",
        );
        p.caption(
            Rect::new(q.min_x(), y - 8., 28., 16.),
            if tick == 0. {
                "0"
            } else if tick == 16. {
                "16"
            } else {
                "32"
            },
        );
    }
    let step = plot.size.width / 5.;
    for (i, value) in [12., 18., 24., 16., 28.].into_iter().enumerate() {
        let bar = Rect::new(
            plot.min_x() + i as f32 * step + INSET,
            plot.max_y() - plot.size.height * value / max,
            step - 2. * INSET,
            plot.size.height * value / max,
        );
        p.box_role(
            bar,
            if i == 2 && (s.is("hover") || s.is("pressed") || s.is("selected")) {
                "c-chart-series-2"
            } else {
                "c-chart-series-1"
            },
            "radius-badge",
            if i == 2 && s.is("focus") {
                Some("border-focused")
            } else {
                None
            },
        );
        p.caption(
            Rect::new(bar.min_x(), plot.max_y() + SMALL, bar.size.width, 20.),
            ["周一", "周二", "周三", "周四", "周五"][i],
        );
        if i == 2 && (s.is("hover") || s.is("pressed") || s.is("selected")) {
            p.caption(
                Rect::new(bar.min_x(), bar.min_y() - 24., bar.size.width, 24.),
                "24 项",
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gallery::layout::PAGE_CONTENT;
    use crate::gallery::static_text::StaticText;
    use tekes_ui_render::Renderer;
    use tekes_ui_scene::Primitive;

    const GROUPS: [&str; 8] = [
        "foundations",
        "actions",
        "forms",
        "navigation",
        "overlays",
        "feedback",
        "documents",
        "data",
    ];

    fn render_icon_pixels(
        renderer: &mut Renderer,
        icon: Icon,
        frame: Rect,
        scale: f32,
        theme: Theme,
        tint: Rgba,
    ) -> Vec<u8> {
        let mut text = TextEngine::new();
        text.set_scale(scale);
        text.begin_frame();
        let mut static_text = StaticText::new(None);
        let mut scene = Scene::new();
        let surface = Rect::new(0.0, 0.0, 48.0, 48.0);
        scene.push_clip(surface);
        {
            let mut ctx = PageCtx {
                theme,
                text: &mut text,
                static_text: &mut static_text,
            };
            draw_icon_tinted(&mut ctx, &mut scene, frame, icon, tint);
        }
        scene.pop_clip();
        let pixels = (48.0 * scale).round() as u32;
        renderer
            .render_to_rgba8(
                &scene,
                text.atlas_mut(),
                (pixels, pixels),
                scale,
                Rgba::gray(0.92),
            )
            .expect("render actual Gallery icon artwork offscreen")
    }

    fn ink_bounds(rgba: &[u8], width: u32, tint: Rgba) -> Option<(u32, u32, u32, u32)> {
        let target = [
            (tint.r * 255.0).round() as i16,
            (tint.g * 255.0).round() as i16,
            (tint.b * 255.0).round() as i16,
        ];
        let mut bounds = (u32::MAX, u32::MAX, 0, 0);
        let mut found = false;
        for (index, pixel) in rgba.chunks_exact(4).enumerate() {
            // Measure solid ink pixels while ignoring antialiased edges, the
            // neutral offscreen canvas, and unrelated text or controls.
            let distance = (0..3)
                .map(|channel| (i16::from(pixel[channel]) - target[channel]).abs())
                .sum::<i16>();
            if distance > 150 {
                continue;
            }
            let x = index as u32 % width;
            let y = index as u32 / width;
            bounds.0 = bounds.0.min(x);
            bounds.1 = bounds.1.min(y);
            bounds.2 = bounds.2.max(x);
            bounds.3 = bounds.3.max(y);
            found = true;
        }
        found.then_some(bounds)
    }

    #[test]
    fn plus_ink_is_mirrored_about_its_actual_center_at_odd_even_frames_and_scales() {
        let mut renderer = Renderer::new_headless().expect("headless renderer");
        let tint = Rgba::rgb8(18, 66, 148);
        for scale in [1.0, 2.0] {
            for side in [17.0, 18.0] {
                let frame = Rect::new(8.0, 10.0, side, side);
                let pixels =
                    render_icon_pixels(&mut renderer, Icon::Plus, frame, scale, Theme::LIGHT, tint);
                let width = (48.0 * scale).round() as u32;
                let center_x = frame.center().x * scale;
                let center_y = frame.center().y * scale;
                // Pixel centers are at n + 0.5; reflect indices around the
                // same subpixel center used by the logical artboard.
                let reflect_x = (2.0 * center_x).round() as i32 - 1;
                let reflect_y = (2.0 * center_y).round() as i32 - 1;
                let radius = ((side * 0.4 + 2.0) * scale).ceil() as i32;
                for y in (center_y.floor() as i32 - radius)..=(center_y.ceil() as i32 + radius) {
                    for x in (center_x.floor() as i32 - radius)..=(center_x.ceil() as i32 + radius)
                    {
                        let reflected_x = reflect_x - x;
                        let reflected_y = reflect_y - y;
                        if x < 0
                            || y < 0
                            || reflected_x < 0
                            || reflected_y < 0
                            || x >= width as i32
                            || y >= width as i32
                            || reflected_x >= width as i32
                            || reflected_y >= width as i32
                        {
                            continue;
                        }
                        let offset =
                            |x: i32, y: i32| (y as usize * width as usize + x as usize) * 4;
                        let a = offset(x, y);
                        let b = offset(reflected_x, reflected_y);
                        assert_eq!(
                            &pixels[a..a + 4],
                            &pixels[b..b + 4],
                            "+ must be pixel symmetric at {side}pt/{scale}x around {center_x},{center_y}"
                        );
                    }
                }
                let bounds = ink_bounds(&pixels, width, tint).expect("plus draws ink");
                let left = (frame.min_x() * scale).floor() as u32;
                let top = (frame.min_y() * scale).floor() as u32;
                let right = (frame.max_x() * scale).ceil() as u32;
                let bottom = (frame.max_y() * scale).ceil() as u32;
                assert!(
                    bounds.0 >= left && bounds.1 >= top,
                    "{bounds:?} outside {frame:?}"
                );
                assert!(
                    bounds.2 < right && bounds.3 < bottom,
                    "{bounds:?} outside {frame:?}"
                );
            }
        }
    }

    #[test]
    fn semantic_icon_tint_is_used_in_light_and_dark_appearances() {
        let mut renderer = Renderer::new_headless().expect("headless renderer");
        let frame = Rect::new(8.0, 10.0, 18.0, 18.0);
        let tint = Rgba::rgb8(18, 66, 148);
        for theme in Theme::ALL {
            for icon in [Icon::Plus, Icon::Check, Icon::Chevron, Icon::Folder] {
                let pixels = render_icon_pixels(&mut renderer, icon, frame, 2.0, theme, tint);
                let bounds = ink_bounds(&pixels, 96, tint)
                    .unwrap_or_else(|| panic!("icon did not use supplied tint in {theme:?}"));
                assert!(bounds.0 > 0 && bounds.1 > 0);
                assert!(bounds.2 < 96 && bounds.3 < 96);
            }
        }
    }

    #[test]
    fn check_chevron_and_folder_ink_bounds_stay_inside_odd_even_artboards() {
        let mut renderer = Renderer::new_headless().expect("headless renderer");
        let tint = Rgba::rgb8(18, 66, 148);
        for scale in [1.0, 2.0] {
            for side in [17.0, 18.0] {
                let frame = Rect::new(8.0, 10.0, side, side);
                for icon in [Icon::Check, Icon::Chevron, Icon::Folder] {
                    let pixels =
                        render_icon_pixels(&mut renderer, icon, frame, scale, Theme::DARK, tint);
                    let width = (48.0 * scale).round() as u32;
                    let bounds = ink_bounds(&pixels, width, tint)
                        .unwrap_or_else(|| panic!("{icon:?} draws no solid ink"));
                    let artboard = (
                        (frame.min_x() * scale).floor() as i32,
                        (frame.min_y() * scale).floor() as i32,
                        (frame.max_x() * scale).ceil() as i32,
                        (frame.max_y() * scale).ceil() as i32,
                    );
                    assert!(
                        bounds.0 as i32 >= artboard.0 - 1
                            && bounds.1 as i32 >= artboard.1 - 1
                            && bounds.2 as i32 <= artboard.2
                            && bounds.3 as i32 <= artboard.3,
                        "{icon:?} ink {bounds:?} escaped {artboard:?} at {side}pt/{scale}x"
                    );
                }
            }
        }
    }

    #[test]
    fn all_forty_tiles_and_every_chip_fit_and_share_hit_geometry() {
        let mut count = 0;
        for group in GROUPS {
            for (index, id) in catalog::group(group).component_ids.iter().enumerate() {
                count += 1;
                let component = catalog::component(id);
                let tile = tile_rect(group, PAGE_CONTENT, index);
                let preview = preview_rect(component, tile);
                assert!(
                    tile.min_x() >= PAGE_CONTENT.min_x() && tile.min_y() >= PAGE_CONTENT.min_y()
                );
                assert!(
                    tile.max_x() <= PAGE_CONTENT.max_x() + 0.001
                        && tile.max_y() <= PAGE_CONTENT.max_y() + 0.001
                );
                assert!(
                    preview.size.height >= 72.,
                    "{id}: insufficient preview height"
                );
                let chips = chip_rects(component, tile);
                assert_eq!(chips.len(), component.states.len());
                for (i, (state, rect)) in chips.iter().enumerate() {
                    assert!(rect.max_x() <= tile.max_x() + 0.001 && rect.max_y() < preview.min_y());
                    assert_eq!(
                        state_center(group, PAGE_CONTENT, id, state),
                        Some(rect.center())
                    );
                    assert_eq!(
                        hit_state(group, PAGE_CONTENT, rect.center()),
                        Some((component.id.as_str(), *state))
                    );
                    for (_, other) in chips.iter().skip(i + 1) {
                        assert!(rect.intersection(other).is_none());
                    }
                }
            }
            assert!(hit_state(group, PAGE_CONTENT, Point::new(-1., -1.)).is_none());
        }
        assert_eq!(count, 40);
        assert!(state_center("actions", PAGE_CONTENT, "tui.button", "not-a-state").is_none());
    }

    fn specimen(text: &mut TextEngine, theme: Theme, id: &str, state: &str, rect: Rect) -> Scene {
        let mut scene = Scene::new();
        let mut painter = Painter {
            text,
            scene: &mut scene,
            theme,
            clip: rect,
        };
        draw_component(&mut painter, id, State(state), rect);
        scene
    }
    fn assert_in_bounds(scene: &Scene, rect: Rect, scale: f32, id: &str, state: &str) {
        assert!(!scene.is_empty(), "{id}/{state} has no actual preview");
        for primitive in scene.primitives() {
            match primitive {
                Primitive::Quad(q) => {
                    assert!(
                        q.rect.min_x() >= rect.min_x() - 0.001
                            && q.rect.min_y() >= rect.min_y() - 0.001,
                        "{id}/{state}: {q:?}"
                    );
                    assert!(
                        q.rect.max_x() <= rect.max_x() + 0.001
                            && q.rect.max_y() <= rect.max_y() + 0.001,
                        "{id}/{state}: {q:?}"
                    );
                }
                Primitive::Glyphs(run) => {
                    for positioned in &run.glyphs {
                        let g = positioned.glyph;
                        let x = (positioned.origin.x * scale).round() + g.left as f32;
                        let y = (positioned.origin.y * scale).round() + g.top as f32;
                        assert!(
                            x >= (rect.min_x() * scale).round()
                                && y >= (rect.min_y() * scale).round(),
                            "{id}/{state}: glyph before preview"
                        );
                        assert!(
                            x + g.width as f32 <= (rect.max_x() * scale).round()
                                && y + g.height as f32 <= (rect.max_y() * scale).round(),
                            "{id}/{state}: glyph outside preview"
                        );
                    }
                }
                Primitive::PushClip(clip) => assert!(
                    clip.intersection(&rect).is_some(),
                    "selection clip must intersect its preview"
                ),
                Primitive::PopClip => {}
                _ => panic!("showcase uses primitive + real text only"),
            }
        }
    }

    #[test]
    fn forty_components_all_base_states_render_in_four_themes_without_metadata_masks() {
        let mut text = TextEngine::new();
        text.set_scale(1.);
        for theme in Theme::ALL {
            for group in GROUPS {
                for (index, id) in catalog::group(group).component_ids.iter().enumerate() {
                    let component = catalog::component(id);
                    let rect = preview_rect(component, tile_rect(group, PAGE_CONTENT, index));
                    // Compare only preview primitives: no title/chip selection can fake state coverage.
                    let rest = specimen(&mut text, theme, id, "rest", rect);
                    for state in &component.states {
                        let scene = specimen(&mut text, theme, id, state, rect);
                        assert_in_bounds(&scene, rect, 1., id, state);
                        if state != "rest" {
                            assert_ne!(
                                rest, scene,
                                "{id}/{state} preview must visibly represent its registered state"
                            );
                        }
                        let shapes = text.shape_count();
                        let repeated = specimen(&mut text, theme, id, state, rect);
                        assert_eq!(scene, repeated, "{id}/{state}: nondeterministic primitives");
                        assert_eq!(
                            text.shape_count(),
                            shapes,
                            "{id}/{state}: repeat rebuild reshaped text"
                        );
                    }
                }
            }
        }
    }
}
