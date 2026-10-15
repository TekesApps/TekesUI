//! 离屏渲染的像素验证。
//!
//! 用的是和上屏完全相同的管线与着色器，只是目标换成一张纹理再读回。
//! 没有 GPU 适配器时（例如没有 Metal 的环境）测试打印原因后跳过，不算失败。

use tekes_ui_render::{RenderError, Renderer};
use tekes_ui_scene::{CornerRadii, GlyphAtlas, Quad, Rect, Scene};
use tekes_ui_tokens::{ColorRole, Rgba, Theme};

/// 创建无 surface 的渲染器；没有适配器就返回 `None`，调用方直接 return。
fn renderer() -> Option<Renderer> {
    match Renderer::new_headless() {
        Ok(r) => Some(r),
        Err(RenderError::NoAdapter) => {
            eprintln!("跳过：没有可用的 GPU 适配器");
            None
        }
        Err(e) => panic!("创建渲染器失败：{e}"),
    }
}

/// 一张读回的图。
struct Image {
    w: u32,
    h: u32,
    data: Vec<u8>,
}

impl Image {
    fn px(&self, x: u32, y: u32) -> [u8; 4] {
        assert!(x < self.w && y < self.h, "像素 ({x}, {y}) 越界");
        let i = ((y * self.w + x) * 4) as usize;
        [
            self.data[i],
            self.data[i + 1],
            self.data[i + 2],
            self.data[i + 3],
        ]
    }
}

fn render(r: &mut Renderer, scene: &Scene, size: (u32, u32), scale: f32, clear: Rgba) -> Image {
    let data = r
        .render_to_rgba8(scene, &mut GlyphAtlas::default(), size, scale, clear)
        .unwrap();
    assert_eq!(data.len(), (size.0 * size.1 * 4) as usize);
    Image {
        w: size.0,
        h: size.1,
        data,
    }
}

/// 把 0..=1 的分量四舍五入成 0..=255。
fn b(c: f32) -> u8 {
    (c * 255.0).round() as u8
}

fn rgba8(c: Rgba) -> [u8; 4] {
    [b(c.r), b(c.g), b(c.b), b(c.a)]
}

#[track_caller]
fn assert_px(img: &Image, x: u32, y: u32, expect: [u8; 4], tol: u8) {
    let got = img.px(x, y);
    for i in 0..4 {
        assert!(
            got[i].abs_diff(expect[i]) <= tol,
            "像素 ({x}, {y}) 通道 {i}：得到 {got:?}，期望 {expect:?}（容差 {tol}）"
        );
    }
}

const WHITE: Rgba = Rgba::gray(1.0);
const BLACK: Rgba = Rgba::gray(0.0);

/// 用 token 里的颜色，保证测的是真实使用的数值。
fn token(role: ColorRole) -> Rgba {
    role.resolve(Theme::LIGHT)
}

#[test]
fn empty_scene_is_all_clear_color() {
    let Some(mut r) = renderer() else { return };
    let clear = Rgba::rgb8(0x12, 0x34, 0x56);
    let img = render(&mut r, &Scene::new(), (37, 21), 1.0, clear);
    for y in 0..img.h {
        for x in 0..img.w {
            assert_px(&img, x, y, rgba8(clear), 0);
        }
    }
}

#[test]
fn fill_matches_token_srgb_at_scale_1_and_2() {
    let Some(mut r) = renderer() else { return };
    let clear = token(ColorRole::DocumentCanvas);
    let fill = token(ColorRole::Accent);
    for scale in [1.0_f32, 2.0] {
        let mut scene = Scene::new();
        scene.push_quad(Quad::new(Rect::new(20.0, 10.0, 40.0, 30.0), fill));
        let size = ((100.0 * scale) as u32, (60.0 * scale) as u32);
        let img = render(&mut r, &scene, size, scale, clear);
        let s = scale as u32;
        // 中心：与 token 的 sRGB 值一致，容差 1/255。
        assert_px(&img, 40 * s, 25 * s, rgba8(fill), 1);
        // 外面：清屏色。
        assert_px(&img, 5 * s, 5 * s, rgba8(clear), 0);
        assert_px(&img, 80 * s, 50 * s, rgba8(clear), 0);
        assert_px(&img, 40 * s, 5 * s, rgba8(clear), 0);
    }
}

#[test]
fn ten_point_rect_covers_exactly_20_pixels_at_scale_2() {
    let Some(mut r) = renderer() else { return };
    let mut scene = Scene::new();
    scene.push_quad(Quad::new(Rect::new(5.0, 5.0, 10.0, 10.0), BLACK));
    let img = render(&mut r, &scene, (40, 40), 2.0, WHITE);
    // 10pt 在 scale 2 下是 x、y 都从 10 到 29，共 20 个像素；边界上没有半透明像素。
    let mut filled = 0;
    for y in 0..40 {
        for x in 0..40 {
            let p = img.px(x, y);
            let inside = (10..30).contains(&x) && (10..30).contains(&y);
            if inside {
                assert_eq!(p, [0, 0, 0, 255], "({x}, {y}) 应该被填满");
                filled += 1;
            } else {
                assert_eq!(p, [255, 255, 255, 255], "({x}, {y}) 应该是清屏色");
            }
        }
    }
    assert_eq!(filled, 20 * 20);
}

#[test]
fn pixel_aligned_edges_are_crisp_at_scale_1() {
    let Some(mut r) = renderer() else { return };
    let mut scene = Scene::new();
    scene.push_quad(Quad::new(Rect::new(10.0, 10.0, 10.0, 10.0), BLACK));
    let img = render(&mut r, &scene, (30, 30), 1.0, WHITE);
    assert_eq!(img.px(9, 15), [255, 255, 255, 255]);
    assert_eq!(img.px(10, 15), [0, 0, 0, 255]);
    assert_eq!(img.px(19, 15), [0, 0, 0, 255]);
    assert_eq!(img.px(20, 15), [255, 255, 255, 255]);
    assert_eq!(img.px(15, 9), [255, 255, 255, 255]);
    assert_eq!(img.px(15, 20), [255, 255, 255, 255]);
}

#[test]
fn half_pixel_edges_get_half_coverage() {
    let Some(mut r) = renderer() else { return };
    // 左右边界落在像素中间：x 从 10.5 到 20.5。
    let mut scene = Scene::new();
    scene.push_quad(Quad::new(Rect::new(10.5, 10.0, 10.0, 10.0), BLACK));
    let img = render(&mut r, &scene, (30, 30), 1.0, WHITE);
    assert_eq!(img.px(9, 15), [255, 255, 255, 255]);
    // 覆盖一半：黑色 50% 叠在白色上，gamma 空间里约 128。
    assert_px(&img, 10, 15, [128, 128, 128, 255], 2);
    assert_eq!(img.px(11, 15), [0, 0, 0, 255]);
    assert_eq!(img.px(19, 15), [0, 0, 0, 255]);
    assert_px(&img, 20, 15, [128, 128, 128, 255], 2);
    assert_eq!(img.px(21, 15), [255, 255, 255, 255]);
}

#[test]
fn rounded_corner_is_clear_and_edge_midpoints_are_filled() {
    let Some(mut r) = renderer() else { return };
    let fill = Rgba::rgb8(0xFF, 0x3B, 0x30);
    for scale in [1.0_f32, 2.0] {
        let s = scale as u32;
        let mut scene = Scene::new();
        scene
            .push_quad(Quad::new(Rect::new(10.0, 10.0, 40.0, 40.0), fill).with_corner_radius(12.0));
        let n = 60 * s;
        let img = render(&mut r, &scene, (n, n), scale, WHITE);
        let (x0, y0, x1, y1) = (10 * s, 10 * s, 50 * s - 1, 50 * s - 1);
        // 四个角的最外一个像素：落在圆角外面，仍是清屏色。
        for (x, y) in [(x0, y0), (x1, y0), (x0, y1), (x1, y1)] {
            assert_px(&img, x, y, [255, 255, 255, 255], 0);
        }
        // 四条边的中点：被填满。
        let (mx, my) = (30 * s, 30 * s);
        for (x, y) in [(mx, y0), (mx, y1), (x0, my), (x1, my)] {
            assert_px(&img, x, y, rgba8(fill), 1);
        }
        // 圆角内侧的一点（离角 12pt 半径的圆内）：填满。
        let off = (12.0 * (1.0 - std::f32::consts::FRAC_1_SQRT_2) * scale) as u32 + 2 * s;
        assert_px(&img, x0 + off, y0 + off, rgba8(fill), 1);
    }
}

#[test]
fn rounded_corner_is_symmetric() {
    let Some(mut r) = renderer() else { return };
    let mut scene = Scene::new();
    scene.push_quad(Quad::new(Rect::new(8.0, 8.0, 32.0, 32.0), BLACK).with_corner_radius(10.0));
    let img = render(&mut r, &scene, (48, 48), 1.0, WHITE);
    // 圆角矩形关于中心对称：x -> 47 - x，y -> 47 - y。
    for y in 0..48 {
        for x in 0..48 {
            let a = img.px(x, y);
            assert_px(&img, 47 - x, y, a, 1);
            assert_px(&img, x, 47 - y, a, 1);
        }
    }
}

#[test]
fn per_corner_radii_only_round_the_requested_corners() {
    let Some(mut r) = renderer() else { return };
    let mut scene = Scene::new();
    scene.push_quad(
        Quad::new(Rect::new(10.0, 10.0, 40.0, 40.0), BLACK)
            .with_corner_radii(CornerRadii::top(12.0)),
    );
    let img = render(&mut r, &scene, (60, 60), 1.0, WHITE);
    // 上面两个角被切掉。
    assert_eq!(img.px(10, 10), [255, 255, 255, 255]);
    assert_eq!(img.px(49, 10), [255, 255, 255, 255]);
    // 下面两个角是直角，最外一个像素也是实的。
    assert_eq!(img.px(10, 49), [0, 0, 0, 255]);
    assert_eq!(img.px(49, 49), [0, 0, 0, 255]);
}

#[test]
fn oversized_radius_is_clamped_to_a_pill() {
    let Some(mut r) = renderer() else { return };
    let mut scene = Scene::new();
    // 40x20 的矩形，半径 100 被钳成 10：左右两端是半圆。
    scene.push_quad(Quad::new(Rect::new(10.0, 10.0, 40.0, 20.0), BLACK).with_corner_radius(100.0));
    let img = render(&mut r, &scene, (60, 40), 1.0, WHITE);
    assert_eq!(img.px(10, 10), [255, 255, 255, 255]);
    assert_eq!(img.px(30, 10), [0, 0, 0, 255]);
    assert_eq!(img.px(11, 20), [0, 0, 0, 255]);
    assert_eq!(img.px(48, 20), [0, 0, 0, 255]);
}

#[test]
fn border_band_has_border_color_and_correct_width() {
    let Some(mut r) = renderer() else { return };
    let fill = Rgba::rgb8(0xFF, 0xFF, 0xFF);
    let border = Rgba::rgb8(0x00, 0x7A, 0xFF);
    let clear = Rgba::gray(0.5);
    for scale in [1.0_f32, 2.0] {
        let s = scale as u32;
        let mut scene = Scene::new();
        scene.push_quad(
            Quad::new(Rect::new(10.0, 10.0, 40.0, 30.0), fill)
                .with_border(4.0, border)
                .with_corner_radius(0.0),
        );
        let img = render(&mut r, &scene, (60 * s, 50 * s), scale, clear);
        let y = 25 * s;
        // 外面是清屏色。
        assert_px(&img, 10 * s - 1, y, rgba8(clear), 0);
        // 左边的描边带：4pt 宽，即 4*s 个像素，都是描边色。
        for i in 0..4 * s {
            assert_px(&img, 10 * s + i, y, rgba8(border), 1);
        }
        // 紧接着是 fill。
        assert_px(&img, 10 * s + 4 * s, y, rgba8(fill), 1);
        assert_px(&img, 30 * s, y, rgba8(fill), 1);
        // 右边同样：最后 4*s 个像素是描边。
        for i in 0..4 * s {
            assert_px(&img, 50 * s - 1 - i, y, rgba8(border), 1);
        }
        assert_px(&img, 50 * s - 4 * s - 1, y, rgba8(fill), 1);
        // 上下边。
        let x = 30 * s;
        for i in 0..4 * s {
            assert_px(&img, x, 10 * s + i, rgba8(border), 1);
            assert_px(&img, x, 40 * s - 1 - i, rgba8(border), 1);
        }
        assert_px(&img, x, 10 * s + 4 * s, rgba8(fill), 1);
    }
}

#[test]
fn border_follows_rounded_outline() {
    let Some(mut r) = renderer() else { return };
    let border = Rgba::rgb8(0xFF, 0x00, 0x00);
    let mut scene = Scene::new();
    scene.push_quad(
        Quad::new(Rect::new(10.0, 10.0, 40.0, 40.0), WHITE)
            .with_corner_radius(14.0)
            .with_border(2.0, border),
    );
    let img = render(&mut r, &scene, (60, 60), 1.0, Rgba::gray(0.5));
    // 边中点处的描边带。
    assert_px(&img, 30, 10, rgba8(border), 1);
    assert_px(&img, 30, 11, rgba8(border), 1);
    assert_px(&img, 30, 12, [255, 255, 255, 255], 1);
    assert_px(&img, 10, 30, rgba8(border), 1);
    // 圆角外面仍然是清屏色，没有描边色漏出去。
    assert_px(&img, 10, 10, [128, 128, 128, 255], 1);
    // 沿 45 度对角线：圆角圆心 (24, 24)，半径 14，对角线上外沿离圆心 14 处。
    // 带的中点（半径 13）约在 (24 - 13/√2, 24 - 13/√2) = (14.8, 14.8)，像素 (14, 14) 应是描边色为主。
    let p = img.px(14, 14);
    assert!(p[0] > 200 && p[1] < 80, "对角线上的描边像素 {p:?}");
}

#[test]
fn translucent_fill_blends_in_gamma_space() {
    let Some(mut r) = renderer() else { return };
    // 黑色 50% 叠在白色上：gamma 空间是 127.5 -> 127 或 128；线性空间会是约 188。
    let mut scene = Scene::new();
    scene.push_quad(Quad::new(
        Rect::new(10.0, 10.0, 20.0, 20.0),
        BLACK.with_alpha(0.5),
    ));
    let img = render(&mut r, &scene, (40, 40), 1.0, WHITE);
    let p = img.px(20, 20);
    assert_px(&img, 20, 20, [128, 128, 128, 255], 1);
    assert!(p[0] < 140, "不应是线性空间混合的结果（约 188），得到 {p:?}");

    // 一般情形：每个通道 src*a + dst*(1-a)，都在编码值上算。
    let src = Rgba::new(0.9, 0.2, 0.0, 0.3);
    let dst = Rgba::new(0.1, 0.6, 1.0, 1.0);
    let mut scene = Scene::new();
    scene.push_quad(Quad::new(Rect::new(0.0, 0.0, 8.0, 8.0), src));
    let img = render(&mut r, &scene, (8, 8), 1.0, dst);
    let mix = |s: f32, d: f32| b(s * src.a + d * (1.0 - src.a));
    assert_px(
        &img,
        4,
        4,
        [mix(src.r, dst.r), mix(src.g, dst.g), mix(src.b, dst.b), 255],
        1,
    );
}

#[test]
fn hover_wash_token_over_background() {
    let Some(mut r) = renderer() else { return };
    // 真实的 token 半透明色：hoverWash 叠在 DocumentCanvas 上。
    let bg = token(ColorRole::DocumentCanvas);
    let wash = token(ColorRole::HoverWash);
    let mut scene = Scene::new();
    scene.push_quad(Quad::new(Rect::new(0.0, 0.0, 8.0, 8.0), wash));
    let img = render(&mut r, &scene, (8, 8), 1.0, bg);
    let mix = |s: f32, d: f32| b(s * wash.a + d * (1.0 - wash.a));
    assert_px(
        &img,
        4,
        4,
        [mix(wash.r, bg.r), mix(wash.g, bg.g), mix(wash.b, bg.b), 255],
        1,
    );
}

#[test]
fn translucent_border_shows_the_fill_underneath() {
    let Some(mut r) = renderer() else { return };
    // fill 白、描边黑 50%：描边带下面是白色 fill，结果约 128。
    let mut scene = Scene::new();
    scene.push_quad(
        Quad::new(Rect::new(10.0, 10.0, 20.0, 20.0), WHITE).with_border(3.0, BLACK.with_alpha(0.5)),
    );
    let img = render(&mut r, &scene, (40, 40), 1.0, Rgba::rgb8(255, 0, 0));
    assert_px(&img, 11, 20, [128, 128, 128, 255], 1);
    assert_px(&img, 20, 20, [255, 255, 255, 255], 0);
    // 没有红色（清屏色）透过描边漏出来。
    assert!(img.px(11, 20)[1] > 100);
}

#[test]
fn bordered_outer_edge_is_antialiased_once() {
    let Some(mut r) = renderer() else { return };
    // fill 和描边同色的不透明矩形，外缘只被抗锯齿一次：覆盖 0.5 的像素是 ~128。
    // 如果 fill 与 border 各自抗锯齿再叠加，会得到约 0.75 覆盖（~64），边缘发暗。
    let mut scene = Scene::new();
    scene.push_quad(
        Quad::new(Rect::new(10.5, 10.5, 20.0, 20.0), BLACK)
            .with_corner_radius(6.0)
            .with_border(2.0, BLACK),
    );
    let img = render(&mut r, &scene, (40, 40), 1.0, WHITE);
    // 目标不透明，alpha 恒为 255。
    for y in 0..40 {
        for x in 0..40 {
            assert_eq!(img.px(x, y)[3], 255);
        }
    }
    // 左边中间那一列（x = 10，覆盖 0.5）。
    assert_px(&img, 10, 20, [128, 128, 128, 255], 2);
}

#[test]
fn later_quads_paint_over_earlier_ones() {
    let Some(mut r) = renderer() else { return };
    let red = Rgba::rgb8(255, 0, 0);
    let blue = Rgba::rgb8(0, 0, 255);
    let mut scene = Scene::new();
    scene.push_quad(Quad::new(Rect::new(0.0, 0.0, 20.0, 20.0), red));
    scene.push_quad(Quad::new(Rect::new(10.0, 10.0, 20.0, 20.0), blue));
    let img = render(&mut r, &scene, (40, 40), 1.0, WHITE);
    assert_px(&img, 5, 5, [255, 0, 0, 255], 0);
    assert_px(&img, 15, 15, [0, 0, 255, 255], 0);
    assert_px(&img, 25, 25, [0, 0, 255, 255], 0);
    // 反过来画，顺序决定结果。
    let mut scene = Scene::new();
    scene.push_quad(Quad::new(Rect::new(10.0, 10.0, 20.0, 20.0), blue));
    scene.push_quad(Quad::new(Rect::new(0.0, 0.0, 20.0, 20.0), red));
    let img = render(&mut r, &scene, (40, 40), 1.0, WHITE);
    assert_px(&img, 15, 15, [255, 0, 0, 255], 0);
}

#[test]
fn quads_partially_or_fully_outside_the_target_are_clipped() {
    let Some(mut r) = renderer() else { return };
    let mut scene = Scene::new();
    scene.push_quad(Quad::new(Rect::new(-10.0, -10.0, 20.0, 20.0), BLACK));
    scene.push_quad(Quad::new(Rect::new(100.0, 100.0, 5.0, 5.0), BLACK));
    scene.push_quad(Quad::new(Rect::new(5.0, 5.0, 0.0, 10.0), BLACK)); // 零宽
    let img = render(&mut r, &scene, (20, 20), 1.0, WHITE);
    assert_px(&img, 0, 0, [0, 0, 0, 255], 0);
    assert_px(&img, 9, 9, [0, 0, 0, 255], 0);
    assert_px(&img, 10, 10, [255, 255, 255, 255], 0);
    assert_px(&img, 5, 8, [0, 0, 0, 255], 0); // 被第一个矩形盖住的位置
    assert_px(&img, 15, 15, [255, 255, 255, 255], 0);
}

#[test]
fn instance_buffer_is_reused_across_frames_and_grows_when_needed() {
    let Some(mut r) = renderer() else { return };
    let base_allocs = r.instance_buffer_allocations();
    let base_cap = r.instance_capacity();
    assert!(base_cap >= 1);

    // 在容量内反复渲染：不重新分配。
    let mut scene = Scene::new();
    for i in 0..base_cap {
        scene.push_quad(Quad::new(Rect::new(i as f32 % 16.0, 0.0, 1.0, 1.0), BLACK));
    }
    for _ in 0..5 {
        render(&mut r, &scene, (16, 16), 1.0, WHITE);
    }
    assert_eq!(r.instance_buffer_allocations(), base_allocs);
    assert_eq!(r.instance_capacity(), base_cap);

    // 超出容量：增长一次，至少翻倍，之后同样规模不再分配。
    scene.clear();
    let n = base_cap * 3 + 1;
    for i in 0..n {
        let x = (i % 64) as f32;
        let y = (i / 64) as f32;
        scene.push_quad(Quad::new(Rect::new(x, y, 1.0, 1.0), BLACK));
    }
    let img = render(&mut r, &scene, (64, 64), 1.0, WHITE);
    assert_eq!(r.instance_buffer_allocations(), base_allocs + 1);
    assert!(r.instance_capacity() >= n);
    assert!(r.instance_capacity() >= base_cap * 2);
    // 增长后的内容正确：每个 1x1 的 quad 恰好占一个像素。
    for i in 0..n {
        assert_eq!(
            img.px((i % 64) as u32, (i / 64) as u32),
            [0, 0, 0, 255],
            "quad {i}"
        );
    }
    let cap = r.instance_capacity();
    for _ in 0..3 {
        render(&mut r, &scene, (64, 64), 1.0, WHITE);
    }
    assert_eq!(r.instance_buffer_allocations(), base_allocs + 1);
    // 场景变小时不缩小。
    render(&mut r, &Scene::new(), (64, 64), 1.0, WHITE);
    assert_eq!(r.instance_capacity(), cap);
}

#[test]
fn invalid_arguments_are_errors_not_panics() {
    let Some(mut r) = renderer() else { return };
    let s = Scene::new();
    assert!(matches!(
        r.render_to_rgba8(&s, &mut GlyphAtlas::default(), (0, 10), 1.0, WHITE),
        Err(RenderError::InvalidArgument(_))
    ));
    assert!(matches!(
        r.render_to_rgba8(&s, &mut GlyphAtlas::default(), (10, 10), 0.0, WHITE),
        Err(RenderError::InvalidArgument(_))
    ));
    assert!(matches!(
        r.render_to_rgba8(&s, &mut GlyphAtlas::default(), (10, 10), f32::NAN, WHITE),
        Err(RenderError::InvalidArgument(_))
    ));
    // 没有 surface 的渲染器不能 render。
    assert!(matches!(
        r.render(&s, &mut GlyphAtlas::default(), WHITE),
        Err(RenderError::Surface(_))
    ));
}

#[test]
fn non_multiple_of_64_widths_read_back_correctly() {
    let Some(mut r) = renderer() else { return };
    // 行字节数不是 256 的倍数时，读回要去掉行填充。
    let mut scene = Scene::new();
    scene.push_quad(Quad::new(
        Rect::new(0.0, 0.0, 1.0, 1.0),
        Rgba::rgb8(10, 20, 30),
    ));
    scene.push_quad(Quad::new(
        Rect::new(36.0, 4.0, 1.0, 1.0),
        Rgba::rgb8(40, 50, 60),
    ));
    let img = render(&mut r, &scene, (37, 5), 1.0, WHITE);
    assert_eq!(img.px(0, 0), [10, 20, 30, 255]);
    assert_eq!(img.px(36, 4), [40, 50, 60, 255]);
    assert_eq!(img.px(1, 0), [255, 255, 255, 255]);
    assert_eq!(img.px(36, 3), [255, 255, 255, 255]);
}
