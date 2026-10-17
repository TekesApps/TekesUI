//! 字形与图像管线的像素验证（离屏）。没有 GPU 适配器时打印原因后跳过。
//!
//! 全部用合成的位图，不依赖 CoreText：棋盘格 mask、纯色 tile、渐变覆盖率。

use tekes_ui_render::{RenderError, Renderer};
use tekes_ui_scene::{
    AtlasConfig, AtlasGlyph, FontKey, GlyphAtlas, GlyphKey, GlyphRun, ImageId, ImageQuad, Point,
    PositionedGlyph, Primitive, Quad, RasterFormat, RasterGlyph, Rect, Scene,
};
use tekes_ui_tokens::Rgba;

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

fn render(
    r: &mut Renderer,
    scene: &Scene,
    atlas: &mut GlyphAtlas,
    size: (u32, u32),
    scale: f32,
    clear: Rgba,
) -> Image {
    let data = r.render_to_rgba8(scene, atlas, size, scale, clear).unwrap();
    Image {
        w: size.0,
        h: size.1,
        data,
    }
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
const W8: [u8; 4] = [255, 255, 255, 255];
const K8: [u8; 4] = [0, 0, 0, 255];

fn key(glyph: u16, scale: f32) -> GlyphKey {
    GlyphKey::new(FontKey(1), glyph, GlyphKey::scale_key_for(scale))
}

/// BGRA 预乘像素。
fn bgra(b: u8, g: u8, r: u8, a: u8) -> [u8; 4] {
    [b, g, r, a]
}

/// 5 x 7 的棋盘格 mask：(x + y) 为偶数处覆盖率 255，其余 0。
/// rgb 故意填成非零垃圾值，证明 Mask 只看 alpha。
fn checker_mask(left: i32, top: i32) -> RasterGlyph {
    let (w, h) = (5u32, 7u32);
    let mut pixels = Vec::new();
    for y in 0..h {
        for x in 0..w {
            let a = if (x + y).is_multiple_of(2) { 255 } else { 0 };
            pixels.extend_from_slice(&bgra(0x33, 0x66, 0x99, a));
        }
    }
    RasterGlyph {
        left,
        top,
        width: w,
        height: h,
        format: RasterFormat::Mask,
        pixels,
    }
}

fn is_checker_on(x: u32, y: u32) -> bool {
    (x + y).is_multiple_of(2)
}

/// 整块同一覆盖率的 mask。
fn flat_mask(w: u32, h: u32, coverage: u8) -> RasterGlyph {
    let mut pixels = Vec::new();
    for _ in 0..w * h {
        pixels.extend_from_slice(&bgra(0, 0, 0, coverage));
    }
    RasterGlyph {
        left: 0,
        top: 0,
        width: w,
        height: h,
        format: RasterFormat::Mask,
        pixels,
    }
}

/// 4 x 4 的彩色 tile，已预乘。
/// 行 0：红、绿、蓝、白（不透明）；行 1：半透明红（预乘）；行 2、3：纯色黄。
fn color_tile(left: i32, top: i32) -> RasterGlyph {
    let mut pixels = Vec::new();
    let row0 = [
        bgra(0, 0, 255, 255),
        bgra(0, 255, 0, 255),
        bgra(255, 0, 0, 255),
        bgra(255, 255, 255, 255),
    ];
    for p in row0 {
        pixels.extend_from_slice(&p);
    }
    for _ in 0..4 {
        pixels.extend_from_slice(&bgra(0, 0, 128, 128));
    }
    for _ in 0..8 {
        pixels.extend_from_slice(&bgra(0, 255, 255, 255));
    }
    RasterGlyph {
        left,
        top,
        width: 4,
        height: 4,
        format: RasterFormat::Color,
        pixels,
    }
}

fn solid_color(w: u32, h: u32, b: u8, g: u8, r: u8) -> RasterGlyph {
    let mut pixels = Vec::new();
    for _ in 0..w * h {
        pixels.extend_from_slice(&bgra(b, g, r, 255));
    }
    RasterGlyph {
        left: 0,
        top: 0,
        width: w,
        height: h,
        format: RasterFormat::Color,
        pixels,
    }
}

fn put(atlas: &mut GlyphAtlas, k: GlyphKey, raster: RasterGlyph) -> AtlasGlyph {
    atlas
        .get_or_insert_with(k, || Some(raster))
        .expect("atlas 放得下")
}

fn run(color: Rgba, glyphs: &[(f32, f32, AtlasGlyph)]) -> GlyphRun {
    GlyphRun {
        color,
        glyphs: glyphs
            .iter()
            .map(|&(x, y, glyph)| PositionedGlyph {
                origin: Point::new(x, y),
                glyph,
            })
            .collect(),
    }
}

fn scene_with(runs: Vec<GlyphRun>) -> Scene {
    let mut s = Scene::new();
    for r in runs {
        s.push_glyphs(r);
    }
    s
}

#[test]
fn checker_mask_lands_on_the_rounded_pixel_at_scale_1() {
    let Some(mut r) = renderer() else { return };
    let mut atlas = GlyphAtlas::default();
    atlas.begin_frame();
    let g = put(&mut atlas, key(1, 1.0), checker_mask(2, -3));
    // 原点 (10.3, 12.6) pt -> 像素 (10, 13)，位图左上角 = (10 + 2, 13 - 3) = (12, 10)。
    let scene = scene_with(vec![run(BLACK, &[(10.3, 12.6, g)])]);
    let img = render(&mut r, &scene, &mut atlas, (30, 24), 1.0, WHITE);
    for y in 0..24 {
        for x in 0..30 {
            let inside = (12..17).contains(&x) && (10..17).contains(&y);
            let expect = if inside && is_checker_on(x - 12, y - 10) {
                K8
            } else {
                W8
            };
            assert_eq!(img.px(x, y), expect, "像素 ({x}, {y})");
        }
    }
}

#[test]
fn half_pixel_origins_round_away_from_zero() {
    let Some(mut r) = renderer() else { return };
    let mut atlas = GlyphAtlas::default();
    atlas.begin_frame();
    let g = put(&mut atlas, key(1, 1.0), flat_mask(2, 2, 255));
    // 5.5 -> 6，7.5 -> 8（四舍五入，0.5 进位）；4.49 -> 4。
    let scene = scene_with(vec![run(BLACK, &[(5.5, 7.5, g), (20.49, 7.49, g)])]);
    let img = render(&mut r, &scene, &mut atlas, (30, 20), 1.0, WHITE);
    for (x0, y0) in [(6u32, 8u32), (20, 7)] {
        for y in 0..20 {
            for x in 0..30 {
                let inside = (x0..x0 + 2).contains(&x) && (y0..y0 + 2).contains(&y);
                // 另一个字形的区域不在这里检查。
                let other = [(6u32, 8u32), (20, 7)].iter().any(|&(a, b)| {
                    (a, b) != (x0, y0) && (a..a + 2).contains(&x) && (b..b + 2).contains(&y)
                });
                if other {
                    continue;
                }
                let expect = if inside { K8 } else { W8 };
                assert_eq!(img.px(x, y), expect, "字形 ({x0}, {y0}) 像素 ({x}, {y})");
            }
        }
    }
}

#[test]
fn checker_mask_at_scale_2_is_not_scaled() {
    let Some(mut r) = renderer() else { return };
    let mut atlas = GlyphAtlas::default();
    atlas.begin_frame();
    let g = put(&mut atlas, key(1, 2.0), checker_mask(1, -4));
    // 原点 (10.3, 12.6) pt -> (20.6, 25.2) px -> (21, 25)；位图左上角 (22, 21)；仍是 5 x 7 像素。
    let scene = scene_with(vec![run(BLACK, &[(10.3, 12.6, g)])]);
    let img = render(&mut r, &scene, &mut atlas, (60, 50), 2.0, WHITE);
    for y in 0..50 {
        for x in 0..60 {
            let inside = (22..27).contains(&x) && (21..28).contains(&y);
            let expect = if inside && is_checker_on(x - 22, y - 21) {
                K8
            } else {
                W8
            };
            assert_eq!(img.px(x, y), expect, "像素 ({x}, {y})");
        }
    }
}

#[test]
fn mask_coverage_blends_in_gamma_space() {
    let Some(mut r) = renderer() else { return };
    let mut atlas = GlyphAtlas::default();
    atlas.begin_frame();
    let full = put(&mut atlas, key(1, 1.0), flat_mask(4, 4, 255));
    let half = put(&mut atlas, key(2, 1.0), flat_mask(4, 4, 128));
    let red = Rgba::new(1.0, 0.0, 0.0, 1.0);
    let faded = Rgba::new(0.0, 0.0, 0.0, 0.5);
    let scene = scene_with(vec![
        run(BLACK, &[(2.0, 2.0, full), (10.0, 2.0, half)]),
        run(red, &[(18.0, 2.0, full), (26.0, 2.0, half)]),
        // 颜色 alpha 0.5 乘覆盖率 255。
        run(faded, &[(34.0, 2.0, full)]),
    ]);
    let img = render(&mut r, &scene, &mut atlas, (44, 8), 1.0, WHITE);
    // 覆盖率 255：精确等于文字色。
    assert_px(&img, 3, 3, K8, 0);
    assert_px(&img, 19, 3, [255, 0, 0, 255], 0);
    // 覆盖率 128/255 的黑色盖在白底上：255 * (1 - 128/255) = 127，gamma 空间直接混合。
    assert_px(&img, 11, 3, [127, 127, 127, 255], 1);
    // 红色：R = 255（红 128/255 + 白 127/255），G = B = 127。
    assert_px(&img, 27, 3, [255, 127, 127, 255], 1);
    // 黑色 50% 不透明度：127 / 128。
    assert_px(&img, 35, 3, [127, 127, 127, 255], 1);
    // 位图之外不受影响。
    assert_px(&img, 6, 3, W8, 0);
}

#[test]
fn color_glyph_keeps_its_own_colors() {
    let Some(mut r) = renderer() else { return };
    let mut atlas = GlyphAtlas::default();
    atlas.begin_frame();
    let g = put(&mut atlas, key(1, 1.0), color_tile(0, 0));
    // 与 mask 不同：run 的 rgb 被忽略，alpha 只当不透明度。
    let green = Rgba::new(0.0, 1.0, 0.0, 1.0);
    let half = Rgba::new(0.0, 0.0, 1.0, 0.5);
    let scene = scene_with(vec![
        run(green, &[(2.0, 2.0, g)]),
        run(half, &[(12.0, 2.0, g)]),
    ]);
    let img = render(&mut r, &scene, &mut atlas, (20, 8), 1.0, WHITE);
    // 不透明度 1：每个像素等于自己的颜色（预乘值盖在白底上）。
    assert_px(&img, 2, 2, [255, 0, 0, 255], 0);
    assert_px(&img, 3, 2, [0, 255, 0, 255], 0);
    assert_px(&img, 4, 2, [0, 0, 255, 255], 0);
    assert_px(&img, 5, 2, [255, 255, 255, 255], 0);
    // 半透明红（预乘 128, 0, 0, a=128）盖在白底：R = 128 + 127 = 255，G = B = 127。
    assert_px(&img, 2, 3, [255, 127, 127, 255], 1);
    assert_px(&img, 2, 4, [255, 255, 0, 255], 0);
    // 不透明度 0.5：红色与白底各一半。
    assert_px(&img, 12, 2, [255, 128, 128, 255], 1);
    assert_px(&img, 13, 2, [128, 255, 128, 255], 1);
    assert_px(&img, 14, 2, [128, 128, 255, 255], 1);
    assert_px(&img, 12, 4, [255, 255, 128, 255], 1);
    // tile 之外是白底。
    assert_px(&img, 6, 2, W8, 0);
    assert_px(&img, 2, 6, W8, 0);
}

#[test]
fn rendering_twice_is_byte_identical() {
    let Some(mut r) = renderer() else { return };
    let mut atlas = GlyphAtlas::default();
    atlas.begin_frame();
    let a = put(&mut atlas, key(1, 2.0), checker_mask(1, -4));
    let b = put(&mut atlas, key(2, 2.0), color_tile(0, -2));
    let scene = scene_with(vec![
        run(Rgba::rgb8(20, 80, 200), &[(3.3, 9.7, a), (9.1, 9.7, a)]),
        run(WHITE, &[(14.5, 8.0, b)]),
    ]);
    let first = render(&mut r, &scene, &mut atlas, (60, 40), 2.0, WHITE);
    let second = render(&mut r, &scene, &mut atlas, (60, 40), 2.0, WHITE);
    assert_eq!(first.data, second.data);
    // 新渲染器、新 atlas、同样的输入，同样的字节。
    let mut r2 = Renderer::new_headless().unwrap();
    let mut atlas2 = GlyphAtlas::default();
    atlas2.begin_frame();
    let a2 = put(&mut atlas2, key(1, 2.0), checker_mask(1, -4));
    let b2 = put(&mut atlas2, key(2, 2.0), color_tile(0, -2));
    let scene2 = scene_with(vec![
        run(Rgba::rgb8(20, 80, 200), &[(3.3, 9.7, a2), (9.1, 9.7, a2)]),
        run(WHITE, &[(14.5, 8.0, b2)]),
    ]);
    let third = render(&mut r2, &scene2, &mut atlas2, (60, 40), 2.0, WHITE);
    assert_eq!(first.data, third.data);
    // 确实画上了东西。
    assert!(first.data.chunks(4).any(|p| p != W8));
}

#[test]
fn eviction_then_reinsert_uploads_the_new_pixels() {
    let Some(mut r) = renderer() else { return };
    let mut atlas = GlyphAtlas::new(AtlasConfig {
        page_size: 32,
        max_pages: 1,
        padding: 1,
    });
    let red = key(1, 1.0);
    let blue = key(2, 1.0);

    atlas.begin_frame();
    let g = put(&mut atlas, red, solid_color(24, 24, 0, 0, 255));
    let scene = scene_with(vec![run(WHITE, &[(4.0, 4.0, g)])]);
    let img = render(&mut r, &scene, &mut atlas, (32, 32), 1.0, WHITE);
    assert_px(&img, 10, 10, [255, 0, 0, 255], 0);
    assert_eq!(atlas.stats().uploads, 1);

    // 第二帧：放不下，整页淘汰，蓝色放在同一个位置。
    atlas.begin_frame();
    let g2 = put(&mut atlas, blue, solid_color(24, 24, 255, 0, 0));
    assert_eq!((g2.page, g2.x, g2.y), (0, 0, 0));
    assert_eq!(atlas.stats().evictions, 1);
    let scene = scene_with(vec![run(WHITE, &[(4.0, 4.0, g2)])]);
    let img = render(&mut r, &scene, &mut atlas, (32, 32), 1.0, WHITE);
    assert_px(&img, 10, 10, [0, 0, 255, 255], 0);
    assert_px(&img, 4, 4, [0, 0, 255, 255], 0);
    assert_px(&img, 27, 27, [0, 0, 255, 255], 0);
    assert_eq!(atlas.stats().uploads, 2);

    // 第三帧：没有新字形，不再上传，画面不变。
    atlas.begin_frame();
    let again = put(&mut atlas, blue, solid_color(24, 24, 255, 0, 0));
    assert_eq!(again, g2);
    let img2 = render(&mut r, &scene, &mut atlas, (32, 32), 1.0, WHITE);
    assert_eq!(atlas.stats().uploads, 2);
    assert_eq!(img.data, img2.data);

    // 红色回来：蓝色被淘汰，画面是红色，不是旧的蓝色。
    atlas.begin_frame();
    let back = put(&mut atlas, red, solid_color(24, 24, 0, 0, 255));
    assert_eq!(atlas.stats().evictions, 2);
    let scene = scene_with(vec![run(WHITE, &[(4.0, 4.0, back)])]);
    let img = render(&mut r, &scene, &mut atlas, (32, 32), 1.0, WHITE);
    assert_px(&img, 10, 10, [255, 0, 0, 255], 0);
}

#[test]
fn atlas_texture_grows_with_pages_and_keeps_old_pages() {
    let Some(mut r) = renderer() else { return };
    let mut atlas = GlyphAtlas::new(AtlasConfig {
        page_size: 32,
        max_pages: 4,
        padding: 1,
    });
    // 帧 1：一页。
    atlas.begin_frame();
    let a = put(&mut atlas, key(1, 1.0), solid_color(24, 24, 0, 0, 255));
    let scene = scene_with(vec![run(WHITE, &[(0.0, 0.0, a)])]);
    let img = render(&mut r, &scene, &mut atlas, (100, 30), 1.0, WHITE);
    assert_px(&img, 10, 10, [255, 0, 0, 255], 0);
    assert_eq!(r.atlas_texture_creations(), 1);

    // 帧 2：再加两页（共 3 页），纹理要长到 4 层；第 0 页要整页重传。
    atlas.begin_frame();
    let a = put(&mut atlas, key(1, 1.0), solid_color(24, 24, 0, 0, 255));
    let b = put(&mut atlas, key(2, 1.0), solid_color(24, 24, 0, 255, 0));
    let c = put(&mut atlas, key(3, 1.0), solid_color(24, 24, 255, 0, 0));
    assert_eq!((a.page, b.page, c.page), (0, 1, 2));
    let scene = scene_with(vec![run(
        WHITE,
        &[(0.0, 0.0, a), (30.0, 0.0, b), (60.0, 0.0, c)],
    )]);
    let img = render(&mut r, &scene, &mut atlas, (100, 30), 1.0, WHITE);
    assert_eq!(r.atlas_texture_creations(), 2);
    assert_px(&img, 10, 10, [255, 0, 0, 255], 0);
    assert_px(&img, 40, 10, [0, 255, 0, 255], 0);
    assert_px(&img, 70, 10, [0, 0, 255, 255], 0);

    // 帧 3：第 4 页，仍在 4 层以内，不重建。
    atlas.begin_frame();
    let d = put(&mut atlas, key(4, 1.0), solid_color(24, 24, 0, 255, 255));
    assert_eq!(d.page, 3);
    let scene = scene_with(vec![run(WHITE, &[(0.0, 0.0, a), (70.0, 0.0, d)])]);
    let img = render(&mut r, &scene, &mut atlas, (100, 30), 1.0, WHITE);
    assert_eq!(r.atlas_texture_creations(), 2);
    assert_px(&img, 10, 10, [255, 0, 0, 255], 0);
    assert_px(&img, 80, 10, [255, 255, 0, 255], 0);
    let s = atlas.stats();
    assert_eq!((s.pages, s.glyphs, s.evictions), (4, 4, 0));
    assert_eq!(s.bytes, 4 * 32 * 32 * 4);
}

#[test]
fn two_renderers_can_share_one_atlas() {
    let Some(mut r1) = renderer() else { return };
    let mut r2 = Renderer::new_headless().unwrap();
    let mut atlas = GlyphAtlas::new(AtlasConfig {
        page_size: 32,
        max_pages: 2,
        padding: 1,
    });
    atlas.begin_frame();
    let a = put(&mut atlas, key(1, 1.0), solid_color(24, 24, 0, 0, 255));
    let sa = scene_with(vec![run(WHITE, &[(0.0, 0.0, a)])]);
    let i1 = render(&mut r1, &sa, &mut atlas, (30, 30), 1.0, WHITE);
    assert_px(&i1, 10, 10, [255, 0, 0, 255], 0);
    // 第二个渲染器从没见过这个 atlas，脏区已被第一个取走：它仍要得到完整内容。
    let i2 = render(&mut r2, &sa, &mut atlas, (30, 30), 1.0, WHITE);
    assert_eq!(i1.data, i2.data);
    // 之后加字形，再交替渲染，两边都是新的。
    atlas.begin_frame();
    let a = put(&mut atlas, key(1, 1.0), solid_color(24, 24, 0, 0, 255));
    let b = put(&mut atlas, key(2, 1.0), solid_color(24, 24, 0, 255, 0));
    let sb = scene_with(vec![run(WHITE, &[(0.0, 0.0, a), (0.0, 0.0, b)])]);
    let j1 = render(&mut r1, &sb, &mut atlas, (30, 30), 1.0, WHITE);
    let j2 = render(&mut r2, &sb, &mut atlas, (30, 30), 1.0, WHITE);
    assert_px(&j1, 10, 10, [0, 255, 0, 255], 0);
    assert_eq!(j1.data, j2.data);
}

#[test]
fn painters_order_holds_across_primitive_kinds() {
    let Some(mut r) = renderer() else { return };
    let mut atlas = GlyphAtlas::default();
    atlas.begin_frame();
    let g = put(&mut atlas, key(1, 1.0), flat_mask(8, 8, 255));
    let blue = Rgba::new(0.0, 0.0, 1.0, 1.0);
    let cover = Quad::new(Rect::new(0.0, 0.0, 30.0, 12.0), blue);

    // 字形压在 quad 上：黑色。
    let mut s = Scene::new();
    s.push_quad(cover);
    s.push_glyphs(run(BLACK, &[(2.0, 2.0, g)]));
    let img = render(&mut r, &s, &mut atlas, (30, 12), 1.0, WHITE);
    assert_px(&img, 4, 4, K8, 0);
    assert_px(&img, 20, 4, [0, 0, 255, 255], 0);

    // quad 压在字形上：字形被盖住。
    let mut s = Scene::new();
    s.push_glyphs(run(BLACK, &[(2.0, 2.0, g)]));
    s.push_quad(cover);
    let img = render(&mut r, &s, &mut atlas, (30, 12), 1.0, WHITE);
    assert_px(&img, 4, 4, [0, 0, 255, 255], 0);

    // 夹在中间：quad - 字形 - 半透明 quad，最上面的半透明盖在黑字上。
    let mut s = Scene::new();
    s.push_quad(cover);
    s.push_glyphs(run(BLACK, &[(2.0, 2.0, g)]));
    s.push_quad(Quad::new(
        Rect::new(0.0, 0.0, 30.0, 12.0),
        Rgba::new(1.0, 1.0, 1.0, 0.5),
    ));
    let img = render(&mut r, &s, &mut atlas, (30, 12), 1.0, WHITE);
    assert_px(&img, 4, 4, [128, 128, 128, 255], 1);
    assert_px(&img, 20, 4, [128, 128, 255, 255], 1);
}

#[test]
fn consecutive_primitives_of_a_kind_form_one_draw() {
    let Some(mut r) = renderer() else { return };
    let mut atlas = GlyphAtlas::default();
    atlas.begin_frame();
    let g = put(&mut atlas, key(1, 1.0), flat_mask(2, 2, 255));
    let img_id = r.register_image(&[255; 4], 1, 1).unwrap();
    let q = Quad::new(Rect::new(0.0, 0.0, 4.0, 4.0), BLACK);
    let im = |id| {
        Primitive::Image(ImageQuad {
            rect: Rect::new(0.0, 0.0, 4.0, 4.0),
            image: id,
            opacity: 1.0,
        })
    };
    let mut s = Scene::new();
    s.push_quad(q);
    s.push_quad(q);
    s.push_glyphs(run(BLACK, &[(1.0, 1.0, g), (5.0, 1.0, g)]));
    s.push_glyphs(run(WHITE, &[(9.0, 1.0, g)]));
    s.push_quad(q);
    s.push(im(img_id));
    s.push(im(img_id));
    s.push_glyphs(run(BLACK, &[(1.0, 1.0, g)]));
    render(&mut r, &s, &mut atlas, (16, 16), 1.0, WHITE);
    // quad x2 | glyph x3 | quad | image x2 | glyph：5 次 draw。
    assert_eq!(r.last_frame_counts(), (3, 4, 2, 5));
}

#[test]
fn empty_and_missing_glyph_inputs_draw_nothing() {
    let Some(mut r) = renderer() else { return };
    let mut atlas = GlyphAtlas::default();
    atlas.begin_frame();
    // 空格：不占位，get_or_insert_with 返回 None，不会进 scene。
    assert!(atlas.get_or_insert_with(key(32, 1.0), || None).is_none());
    let mut s = Scene::new();
    s.push_glyphs(GlyphRun {
        color: BLACK,
        glyphs: vec![],
    });
    let img = render(&mut r, &s, &mut atlas, (8, 8), 1.0, WHITE);
    assert!(img.data.chunks(4).all(|p| p == W8));
    assert_eq!(r.last_frame_counts(), (0, 0, 0, 0));
    // atlas 一页都没有时，其他 atlas 的字形（页号越界）被跳过而不是崩溃。
    let mut other = GlyphAtlas::default();
    other.begin_frame();
    let foreign = put(&mut other, key(1, 1.0), flat_mask(2, 2, 255));
    let mut s = Scene::new();
    s.push_glyphs(run(BLACK, &[(1.0, 1.0, foreign)]));
    let img = render(&mut r, &s, &mut atlas, (8, 8), 1.0, WHITE);
    assert!(img.data.chunks(4).all(|p| p == W8));
}

#[test]
fn atlas_stats_follow_the_frames() {
    let Some(mut r) = renderer() else { return };
    let mut atlas = GlyphAtlas::default();
    atlas.begin_frame();
    for i in 0..10u16 {
        put(&mut atlas, key(i, 2.0), solid_color(10, 12, 1, 2, 3));
    }
    assert!(atlas.get_or_insert_with(key(100, 2.0), || None).is_none());
    let scene = Scene::new();
    render(&mut r, &scene, &mut atlas, (8, 8), 2.0, WHITE);
    let s = atlas.stats();
    assert_eq!((s.pages, s.glyphs), (1, 10));
    assert_eq!(s.bytes, 1024 * 1024 * 4);
    assert_eq!((s.misses, s.hits, s.evictions), (11, 0, 0));
    // 渲染器第一次见到这个 atlas：纹理是新建的，整页上传。
    assert_eq!(s.uploads, 1);
    assert_eq!(s.upload_bytes, 1024 * 1024 * 4);
    // 命中，不产生上传。
    atlas.begin_frame();
    put(&mut atlas, key(3, 2.0), solid_color(10, 12, 1, 2, 3));
    assert_eq!(atlas.stats().hits, 1);
    render(&mut r, &scene, &mut atlas, (8, 8), 2.0, WHITE);
    assert_eq!(atlas.stats().uploads, 1);
    // 新字形：只上传它所在的矩形（接在上一个后面，同一条 shelf，宽 10 高 12）。
    put(&mut atlas, key(50, 2.0), solid_color(10, 12, 4, 5, 6));
    render(&mut r, &scene, &mut atlas, (8, 8), 2.0, WHITE);
    let s = atlas.stats();
    assert_eq!(s.uploads, 2);
    assert_eq!(s.upload_bytes, 1024 * 1024 * 4 + 10 * 12 * 4);
}

// ---- 图像 ----

/// 2 x 2 的图：左上红、右上绿、左下蓝、右下白，不透明，RGBA 顺序。
fn four_color_image() -> Vec<u8> {
    vec![
        255, 0, 0, 255, 0, 255, 0, 255, //
        0, 0, 255, 255, 255, 255, 255, 255,
    ]
}

#[test]
fn image_draws_into_its_rect() {
    let Some(mut r) = renderer() else { return };
    let mut atlas = GlyphAtlas::default();
    let id = r.register_image(&four_color_image(), 2, 2).unwrap();
    assert_eq!(r.image_count(), 1);
    let mut s = Scene::new();
    s.push(Primitive::Image(ImageQuad {
        rect: Rect::new(5.0, 5.0, 10.0, 10.0),
        image: id,
        opacity: 1.0,
    }));
    let gray = Rgba::gray(0.5);
    let img = render(&mut r, &s, &mut atlas, (20, 20), 1.0, gray);
    let bg = [128, 128, 128, 255];
    // 矩形之外是背景，边界精确。
    assert_px(&img, 4, 10, bg, 0);
    assert_px(&img, 15, 10, bg, 0);
    assert_px(&img, 10, 4, bg, 0);
    assert_px(&img, 10, 15, bg, 0);
    // 四个角落靠近边缘的像素是各自的纯色（线性采样钳到边缘 texel）。
    assert_px(&img, 5, 5, [255, 0, 0, 255], 2);
    assert_px(&img, 14, 5, [0, 255, 0, 255], 2);
    assert_px(&img, 5, 14, [0, 0, 255, 255], 2);
    assert_px(&img, 14, 14, [255, 255, 255, 255], 2);
    // 中心是线性混合，不是任何一个纯色。
    let c = img.px(10, 10);
    assert!(c[0] > 60 && c[1] > 60 && c[2] > 60, "中心 {c:?}");
}

#[test]
fn image_scales_with_scale_and_respects_opacity() {
    let Some(mut r) = renderer() else { return };
    let mut atlas = GlyphAtlas::default();
    let id = r.register_image(&[255, 0, 0, 255], 1, 1).unwrap();
    let mut s = Scene::new();
    s.push(Primitive::Image(ImageQuad {
        rect: Rect::new(2.0, 2.0, 4.0, 4.0),
        image: id,
        opacity: 0.5,
    }));
    let img = render(&mut r, &s, &mut atlas, (16, 16), 2.0, WHITE);
    // scale 2：矩形占像素 4..12。红色 50%（预乘值 * 0.5）盖在白底：R=255，G=B=128。
    assert_px(&img, 4, 4, [255, 128, 128, 255], 1);
    assert_px(&img, 11, 11, [255, 128, 128, 255], 1);
    assert_px(&img, 3, 4, W8, 0);
    assert_px(&img, 12, 4, W8, 0);
    assert_px(&img, 4, 12, W8, 0);
}

#[test]
fn premultiplied_image_alpha_blends_over_the_background() {
    let Some(mut r) = renderer() else { return };
    let mut atlas = GlyphAtlas::default();
    // 预乘的半透明黑 (0, 0, 0, 128)。
    let id = r.register_image(&[0, 0, 0, 128], 1, 1).unwrap();
    let mut s = Scene::new();
    s.push(Primitive::Image(ImageQuad {
        rect: Rect::new(0.0, 0.0, 4.0, 4.0),
        image: id,
        opacity: 1.0,
    }));
    let img = render(&mut r, &s, &mut atlas, (4, 4), 1.0, WHITE);
    assert_px(&img, 2, 2, [127, 127, 127, 255], 1);
}

#[test]
fn unregistered_images_are_skipped_and_bad_input_is_rejected() {
    let Some(mut r) = renderer() else { return };
    let mut atlas = GlyphAtlas::default();
    let id = r.register_image(&[0, 0, 0, 255], 1, 1).unwrap();
    assert!(r.unregister_image(id));
    assert!(!r.unregister_image(id), "第二次注销返回 false");
    assert_eq!(r.image_count(), 0);
    let mut s = Scene::new();
    s.push(Primitive::Image(ImageQuad {
        rect: Rect::new(0.0, 0.0, 4.0, 4.0),
        image: id,
        opacity: 1.0,
    }));
    s.push(Primitive::Image(ImageQuad {
        rect: Rect::new(0.0, 0.0, 4.0, 4.0),
        image: ImageId(9999),
        opacity: 1.0,
    }));
    let img = render(&mut r, &s, &mut atlas, (4, 4), 1.0, WHITE);
    assert!(img.data.chunks(4).all(|p| p == W8));
    assert_eq!(r.last_frame_counts(), (0, 0, 0, 0));
    assert!(matches!(
        r.register_image(&[0; 4], 0, 1),
        Err(RenderError::InvalidArgument(_))
    ));
    assert!(matches!(
        r.register_image(&[0; 7], 1, 2),
        Err(RenderError::InvalidArgument(_))
    ));
    // id 不复用。
    let a = r.register_image(&[0; 4], 1, 1).unwrap();
    let b = r.register_image(&[0; 4], 1, 1).unwrap();
    assert_ne!(a, b);
    assert_ne!(a, id);
}

#[test]
fn transformed_images_rotate_select_atlas_region_and_keep_zero_angle_pixels() {
    use tekes_ui_scene::TransformedImageQuad;
    let Some(mut r) = renderer() else {
        return;
    };
    let mut atlas = GlyphAtlas::default();
    let id = r.register_image(&four_color_image(), 2, 2).unwrap();
    let quad = ImageQuad {
        rect: Rect::new(5.0, 5.0, 10.0, 10.0),
        image: id,
        opacity: 1.0,
    };
    let mut legacy = Scene::new();
    legacy.push_image(quad);
    let mut transformed = Scene::new();
    transformed.push_transformed_image(TransformedImageQuad::new(quad, 0.0));
    assert_eq!(
        render(&mut r, &legacy, &mut atlas, (20, 20), 1.0, Rgba::gray(0.0)).data,
        render(
            &mut r,
            &transformed,
            &mut atlas,
            (20, 20),
            1.0,
            Rgba::gray(0.0)
        )
        .data
    );
    transformed.clear();
    transformed
        .push_transformed_image(TransformedImageQuad::new(quad, std::f32::consts::FRAC_PI_2));
    let image = render(
        &mut r,
        &transformed,
        &mut atlas,
        (20, 20),
        1.0,
        Rgba::gray(0.0),
    );
    assert_px(&image, 5, 5, [0, 0, 255, 255], 2);
    assert_px(&image, 14, 5, [255, 0, 0, 255], 2);
    assert_px(&image, 14, 14, [0, 255, 0, 255], 2);
    let id = r
        .register_image(&[255, 0, 0, 255, 0, 255, 0, 255], 2, 1)
        .unwrap();
    transformed.clear();
    transformed.push_transformed_image(TransformedImageQuad {
        quad: ImageQuad { image: id, ..quad },
        rotation: 0.0,
        uv: Rect::new(0.5, 0.0, 0.5, 1.0),
    });
    let image = render(
        &mut r,
        &transformed,
        &mut atlas,
        (20, 20),
        1.0,
        Rgba::gray(0.0),
    );
    assert_px(&image, 13, 10, [0, 255, 0, 255], 0);
}
