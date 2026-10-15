//! 把一个示例 scene 渲染成 PNG，用来目测抗锯齿和混合效果。
//!
//! 用法：`cargo run -p tekes-ui-render --example dump_sample -- <输出目录>`
//!
//! 输出 `sample_1x.png`、`sample_2x.png`，以及圆角和描边局部的最近邻放大图
//! `zoom_1x.png`、`zoom_2x.png`（点区域 92..172 x 12..116，看单个像素的覆盖率）。

use std::{fs::File, io::BufWriter, path::Path};
use tekes_ui_render::Renderer;
use tekes_ui_scene::{CornerRadii, GlyphAtlas, Quad, Rect, Scene};
use tekes_ui_tokens::{ColorRole, Rgba, Theme};

const W: u32 = 360;
const H: u32 = 240;

fn sample_scene() -> Scene {
    let theme = Theme::LIGHT;
    let accent = ColorRole::Accent.resolve(theme);
    let red = ColorRole::StatusFailed.resolve(theme);
    let green = ColorRole::StatusSucceeded.resolve(theme);
    let orange = ColorRole::StatusRunning.resolve(theme);
    let wash = ColorRole::HoverWash.resolve(theme);
    let border = ColorRole::Border.resolve(theme);
    let mut s = Scene::new();

    // 第一行：直角、圆角、胶囊、圆。
    s.push_quad(Quad::new(Rect::new(16.0, 16.0, 64.0, 40.0), accent));
    s.push_quad(Quad::new(Rect::new(96.0, 16.0, 64.0, 40.0), red).with_corner_radius(8.0));
    s.push_quad(Quad::new(Rect::new(176.0, 16.0, 80.0, 40.0), green).with_corner_radius(20.0));
    s.push_quad(Quad::new(Rect::new(272.0, 16.0, 40.0, 40.0), orange).with_corner_radius(20.0));
    // 只圆上面两个角。
    s.push_quad(
        Quad::new(Rect::new(320.0, 16.0, 28.0, 40.0), accent)
            .with_corner_radii(CornerRadii::top(10.0)),
    );

    // 第二行：描边。
    let white = Rgba::gray(1.0);
    s.push_quad(Quad::new(Rect::new(16.0, 72.0, 64.0, 40.0), white).with_border(1.0, border));
    s.push_quad(
        Quad::new(Rect::new(96.0, 72.0, 64.0, 40.0), white)
            .with_corner_radius(8.0)
            .with_border(1.0, accent),
    );
    s.push_quad(
        Quad::new(Rect::new(176.0, 72.0, 80.0, 40.0), white)
            .with_corner_radius(20.0)
            .with_border(3.0, red),
    );
    s.push_quad(
        Quad::new(Rect::new(272.0, 72.0, 40.0, 40.0), accent)
            .with_corner_radius(10.0)
            .with_border(4.0, Rgba::gray(0.0).with_alpha(0.5)),
    );

    // 第三行：半透明叠加（A 与 B 重叠处看混合）、hoverWash、非整数坐标。
    s.push_quad(
        Quad::new(Rect::new(16.0, 128.0, 56.0, 56.0), red.with_alpha(0.5)).with_corner_radius(10.0),
    );
    s.push_quad(
        Quad::new(Rect::new(44.0, 144.0, 56.0, 56.0), accent.with_alpha(0.5))
            .with_corner_radius(10.0),
    );
    s.push_quad(Quad::new(
        Rect::new(120.0, 128.0, 80.0, 28.0),
        Rgba::gray(0.0).with_alpha(0.5),
    ));
    s.push_quad(Quad::new(Rect::new(120.0, 160.0, 80.0, 28.0), wash).with_corner_radius(6.0));
    s.push_quad(
        Quad::new(Rect::new(216.0, 128.5, 40.5, 40.5), Rgba::gray(0.0)).with_corner_radius(12.0),
    );
    s.push_quad(
        Quad::new(Rect::new(272.25, 128.25, 40.0, 40.0), Rgba::gray(0.0)).with_corner_radius(12.0),
    );
    // 一条 1pt 的分隔线，以及 0.5pt 对齐到半像素的细线。
    s.push_quad(Quad::new(
        Rect::new(216.0, 188.0, 128.0, 1.0),
        Rgba::gray(0.0).with_alpha(0.3),
    ));
    s.push_quad(Quad::new(
        Rect::new(216.0, 196.5, 128.0, 1.0),
        Rgba::gray(0.0).with_alpha(0.3),
    ));
    s
}

fn write_png(path: &Path, w: u32, h: u32, rgba: &[u8]) {
    let mut enc = png::Encoder::new(BufWriter::new(File::create(path).unwrap()), w, h);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header().unwrap().write_image_data(rgba).unwrap();
}

/// 最近邻放大 `k` 倍的局部。
fn zoom(rgba: &[u8], w: u32, x0: u32, y0: u32, cw: u32, ch: u32, k: u32) -> Vec<u8> {
    let mut out = Vec::with_capacity((cw * ch * k * k * 4) as usize);
    for y in 0..ch * k {
        for x in 0..cw * k {
            let i = (((y0 + y / k) * w + x0 + x / k) * 4) as usize;
            out.extend_from_slice(&rgba[i..i + 4]);
        }
    }
    out
}

fn main() {
    let dir = std::env::args()
        .nth(1)
        .expect("用法：dump_sample <输出目录>");
    let dir = Path::new(&dir);
    std::fs::create_dir_all(dir).unwrap();
    let mut r = Renderer::new_headless().expect("没有可用的 GPU");
    let scene = sample_scene();
    let clear = ColorRole::DocumentCanvas.resolve(Theme::LIGHT);
    for scale in [1u32, 2] {
        let (w, h) = (W * scale, H * scale);
        let rgba = r
            .render_to_rgba8(
                &scene,
                &mut GlyphAtlas::default(),
                (w, h),
                scale as f32,
                clear,
            )
            .unwrap();
        write_png(&dir.join(format!("sample_{scale}x.png")), w, h, &rgba);
        // 放大后两张图的屏幕大小一样（1x 放大 6 倍，2x 放大 3 倍）。
        let k = 6 / scale;
        let (cx, cy, cw, ch) = (92 * scale, 12 * scale, 80 * scale, 104 * scale);
        let z = zoom(&rgba, w, cx, cy, cw, ch, k);
        write_png(&dir.join(format!("zoom_{scale}x.png")), cw * k, ch * k, &z);
    }
    println!("已写入 {}", dir.display());
}
