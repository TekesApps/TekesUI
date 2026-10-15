//! Gallery 自己的状态与内容。
//!
//! 现在只有一个占位页：几块用 token 画的色块，加一组用来验证 Retina 线宽和缩放布局的标记。
//! 页面、目录、状态矩阵切换、并排参照图都在后续阶段加到这里；`host.rs` 不需要改。
//!
//! 与平台层的耦合只有 `handle_input` 的 `Event` 参数；坐标一律是逻辑点，左上角为原点。

use tekes_ui_platform_macos::Event;
use tekes_ui_scene::{Quad, Rect, Scene, Size};
use tekes_ui_tokens::{ColorRole, Rgba, Theme};

/// 固定的内容区尺寸（pt）。每个页面都按这个尺寸排版；窗口可以缩放，但初始尺寸固定。
pub const CONTENT_WIDTH: f64 = 1200.0;
pub const CONTENT_HEIGHT: f64 = 800.0;

/// 顶栏高度、侧栏宽度（占位）。
const HEADER_H: f32 = 44.0;
const SIDEBAR_W: f32 = 220.0;

/// 左上与右下标记块的边长和边距，自测靠它们验证缩放后的布局。
pub const MARKER: f32 = 40.0;
pub const MARKER_INSET: f32 = 20.0;

/// 线宽验证条在内容区里的位置（pt）。线都对齐到逻辑点，2x 下 1pt = 2 像素，0.5pt = 1 像素。
pub const STRIP_X: f32 = 260.0;
pub const STRIP_Y: f32 = 80.0;

#[derive(Debug, Default)]
pub struct Gallery {}

impl Gallery {
    pub fn new() -> Gallery {
        Gallery {}
    }

    /// 窗口背景色，传给 `Renderer::render` 的 `clear`。
    pub fn background(&self, theme: Theme) -> Rgba {
        ColorRole::DocumentCanvas.resolve(theme)
    }

    /// 把当前内容编译成绘制指令。`size` 是视图尺寸（pt）。
    pub fn build_scene(&self, size: Size, theme: Theme, scene: &mut Scene) {
        let c = |role: ColorRole| role.resolve(theme);
        let w = size.width;
        let h = size.height;

        // 顶栏与侧栏。
        scene.push_quad(Quad::new(
            Rect::new(0.0, 0.0, w, HEADER_H),
            c(ColorRole::TranscriptInput),
        ));
        scene.push_quad(Quad::new(
            Rect::new(0.0, HEADER_H, w, 1.0),
            c(ColorRole::Separator),
        ));
        scene.push_quad(Quad::new(
            Rect::new(
                0.0,
                HEADER_H + 1.0,
                SIDEBAR_W,
                (h - HEADER_H - 1.0).max(0.0),
            ),
            c(ColorRole::RowSelection),
        ));
        scene.push_quad(Quad::new(
            Rect::new(
                SIDEBAR_W,
                HEADER_H + 1.0,
                1.0,
                (h - HEADER_H - 1.0).max(0.0),
            ),
            c(ColorRole::Separator),
        ));

        // 内容卡片：圆角加描边。
        let card = Rect::new(
            SIDEBAR_W + 24.0,
            HEADER_H + 24.0,
            (w - SIDEBAR_W - 48.0).max(0.0),
            (h - HEADER_H - 48.0).max(0.0),
        );
        scene.push_quad(
            Quad::new(card, c(ColorRole::TranscriptInput))
                .with_corner_radius(12.0)
                .with_border(1.0, c(ColorRole::Border)),
        );

        // 线宽验证条：1pt 与 0.5pt 的横线、竖线。
        let ink = c(ColorRole::PrimaryText);
        let (x, y) = (STRIP_X, HEADER_H + STRIP_Y);
        scene.push_quad(Quad::new(Rect::new(x, y, 200.0, 1.0), ink));
        scene.push_quad(Quad::new(Rect::new(x, y + 10.0, 200.0, 0.5), ink));
        scene.push_quad(Quad::new(Rect::new(x + 220.0, y, 1.0, 60.0), ink));
        scene.push_quad(Quad::new(Rect::new(x + 230.0, y, 0.5, 60.0), ink));

        // 左上和右下的标记块：缩放后右下块必须贴着新的右下角。
        let accent = c(ColorRole::Accent);
        scene.push_quad(Quad::new(
            Rect::new(MARKER_INSET, MARKER_INSET, MARKER, MARKER),
            accent,
        ));
        scene.push_quad(
            Quad::new(
                Rect::new(
                    w - MARKER_INSET - MARKER,
                    h - MARKER_INSET - MARKER,
                    MARKER,
                    MARKER,
                ),
                c(ColorRole::StatusSucceeded),
            )
            .with_corner_radius(8.0),
        );
    }

    /// 处理输入事件，返回是否需要重绘。目前没有交互。
    pub fn handle_input(&mut self, _event: &Event) -> bool {
        false
    }
}
