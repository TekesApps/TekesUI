//! 带容差的像素对比（comparison-method 第 2.1 节）。
//!
//! 输入是两张同尺寸的 RGBA8 位图（行紧密排列，左上角在前，已合成在不透明背景上）。
//! 判定规则：
//!
//! 1. 四个通道里任何一个的差 ≤ `channel_threshold`（初值 24/255）的像素不算差异；
//! 2. 文字包围盒是"两张图里与背景不同的像素"的并集的外接矩形；超阈值像素数占包围盒
//!    像素数的比例必须 ≤ `max_ratio`（1x 为 0.5%，2x 为 0.3%）；
//! 3. 超阈值像素做 8 连通分析，任何一个连通域的面积超过 `max_component_area`
//!    （约一个字形的面积）就判失败，不看比例。
//!
//! 位置偏移不做自动对齐，偏移本身就是失败。
//!
//! 输出除了数字，还有一张差异热图（RGBA8，同尺寸），Gallery 用它并排展示。

/// 像素矩形，半开区间。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PixelRect {
    pub x0: u32,
    pub y0: u32,
    pub x1: u32,
    pub y1: u32,
}

impl PixelRect {
    pub fn width(&self) -> u32 {
        self.x1 - self.x0
    }

    pub fn height(&self) -> u32 {
        self.y1 - self.y0
    }

    pub fn area(&self) -> usize {
        self.width() as usize * self.height() as usize
    }
}

/// 对比参数。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DiffOptions {
    /// 通道差不超过它的像素不计入差异。
    pub channel_threshold: u8,
    /// 超阈值像素占文字包围盒的最大比例。
    pub max_ratio: f64,
    /// 允许的最大连通差异面积（像素数），约一个字形。
    pub max_component_area: usize,
    /// 背景色（RGBA8），用来找出文字包围盒。
    pub background: [u8; 4],
}

impl DiffOptions {
    /// 按 scale 取门槛：1x 为 0.5%，2x 及以上为 0.3%。`glyph_px` 是字号乘 scale（像素），
    /// 一个字形的面积按 `glyph_px²` 估计。
    pub fn for_scale(scale: f64, glyph_px: f64, background: [u8; 4]) -> DiffOptions {
        DiffOptions {
            channel_threshold: 24,
            max_ratio: if scale >= 1.5 { 0.003 } else { 0.005 },
            max_component_area: (glyph_px * glyph_px).ceil() as usize,
            background,
        }
    }
}

/// 对比结果。
#[derive(Clone, Debug, PartialEq)]
pub struct DiffReport {
    pub width: u32,
    pub height: u32,
    /// 文字包围盒；两张图都是纯背景时为 `None`。
    pub text_bbox: Option<PixelRect>,
    pub bbox_pixels: usize,
    /// 超阈值像素数（整张图）。
    pub over_threshold: usize,
    /// 超阈值像素数 / 包围盒像素数；没有包围盒时为 0。
    pub ratio: f64,
    /// 最大连通差异域的面积。
    pub max_component_area: usize,
    /// 最大单像素通道差（0..=255），含低于阈值的差异。
    pub max_pixel_diff: u8,
    /// 差异热图，RGBA8，不透明。
    pub heatmap: Vec<u8>,
    pub options: DiffOptions,
}

impl DiffReport {
    pub fn ratio_ok(&self) -> bool {
        self.ratio <= self.options.max_ratio
    }

    pub fn components_ok(&self) -> bool {
        self.max_component_area <= self.options.max_component_area
    }

    /// 比例与连通域两项都通过。
    pub fn passed(&self) -> bool {
        self.ratio_ok() && self.components_ok()
    }

    /// 一行摘要，放进对比报告。
    pub fn summary(&self) -> String {
        format!(
            "超阈值比例 {:.4}%（上限 {:.2}%），最大连通域 {} 像素（上限 {}），最大单像素差 {}/255，包围盒 {:?}",
            self.ratio * 100.0,
            self.options.max_ratio * 100.0,
            self.max_component_area,
            self.options.max_component_area,
            self.max_pixel_diff,
            self.text_bbox,
        )
    }
}

/// 对比两张 RGBA8 位图。尺寸或长度不符会 panic（这是调用方的错误）。
pub fn diff_rgba8(a: &[u8], b: &[u8], width: u32, height: u32, opts: DiffOptions) -> DiffReport {
    let (w, h) = (width as usize, height as usize);
    assert_eq!(a.len(), w * h * 4, "位图 a 的长度与尺寸不符");
    assert_eq!(b.len(), w * h * 4, "位图 b 的长度与尺寸不符");

    let mut over = vec![false; w * h];
    let mut diffs = vec![0u8; w * h];
    let (mut min_x, mut min_y, mut max_x, mut max_y) = (u32::MAX, u32::MAX, 0u32, 0u32);
    let mut any_ink = false;
    let mut max_pixel_diff = 0u8;
    let mut over_count = 0usize;
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            let pa = &a[i * 4..i * 4 + 4];
            let pb = &b[i * 4..i * 4 + 4];
            let d = (0..4).map(|c| pa[c].abs_diff(pb[c])).max().unwrap_or(0);
            diffs[i] = d;
            max_pixel_diff = max_pixel_diff.max(d);
            if d > opts.channel_threshold {
                over[i] = true;
                over_count += 1;
            }
            if pa != opts.background || pb != opts.background {
                any_ink = true;
                min_x = min_x.min(x as u32);
                min_y = min_y.min(y as u32);
                max_x = max_x.max(x as u32);
                max_y = max_y.max(y as u32);
            }
        }
    }
    let text_bbox = any_ink.then_some(PixelRect {
        x0: min_x,
        y0: min_y,
        x1: max_x + 1,
        y1: max_y + 1,
    });
    let bbox_pixels = text_bbox.map_or(0, |r| r.area());
    let ratio = if bbox_pixels > 0 {
        over_count as f64 / bbox_pixels as f64
    } else {
        0.0
    };
    let max_component_area = largest_component(&over, w, h);
    let heatmap = heatmap_rgba8(&diffs, &over, opts.channel_threshold);
    DiffReport {
        width,
        height,
        text_bbox,
        bbox_pixels,
        over_threshold: over_count,
        ratio,
        max_component_area,
        max_pixel_diff,
        heatmap,
        options: opts,
    }
}

/// 8 连通域里最大的面积（迭代，不递归）。
fn largest_component(mask: &[bool], w: usize, h: usize) -> usize {
    let mut seen = vec![false; mask.len()];
    let mut best = 0usize;
    let mut stack: Vec<usize> = Vec::new();
    for start in 0..mask.len() {
        if !mask[start] || seen[start] {
            continue;
        }
        let mut area = 0usize;
        seen[start] = true;
        stack.push(start);
        while let Some(i) = stack.pop() {
            area += 1;
            let (x, y) = ((i % w) as isize, (i / w) as isize);
            for dy in -1isize..=1 {
                for dx in -1isize..=1 {
                    if dx == 0 && dy == 0 {
                        continue;
                    }
                    let (nx, ny) = (x + dx, y + dy);
                    if nx < 0 || ny < 0 || nx >= w as isize || ny >= h as isize {
                        continue;
                    }
                    let j = ny as usize * w + nx as usize;
                    if mask[j] && !seen[j] {
                        seen[j] = true;
                        stack.push(j);
                    }
                }
            }
        }
        best = best.max(area);
    }
    best
}

/// 热图配色：无差异是深灰；有差异但在阈值内按深蓝渐变；超阈值按红到黄渐变。
fn heatmap_rgba8(diffs: &[u8], over: &[bool], threshold: u8) -> Vec<u8> {
    let mut out = Vec::with_capacity(diffs.len() * 4);
    for (i, &d) in diffs.iter().enumerate() {
        let px: [u8; 4] = if d == 0 {
            [28, 28, 32, 255]
        } else if !over[i] {
            let t = f32::from(d) / f32::from(threshold.max(1));
            [28, 28, (60.0 + 120.0 * t) as u8, 255]
        } else {
            let t = (f32::from(d) - f32::from(threshold)) / (255.0 - f32::from(threshold)).max(1.0);
            [255, (80.0 + 175.0 * t) as u8, 40, 255]
        };
        out.extend_from_slice(&px);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const BG: [u8; 4] = [255, 255, 255, 255];

    fn blank(w: u32, h: u32) -> Vec<u8> {
        BG.repeat((w * h) as usize)
    }

    fn set(img: &mut [u8], w: u32, x: u32, y: u32, px: [u8; 4]) {
        let i = ((y * w + x) * 4) as usize;
        img[i..i + 4].copy_from_slice(&px);
    }

    fn opts() -> DiffOptions {
        DiffOptions::for_scale(1.0, 4.0, BG)
    }

    #[test]
    fn identical_images_pass_with_zero_numbers() {
        let mut a = blank(16, 16);
        set(&mut a, 16, 3, 3, [0, 0, 0, 255]);
        let r = diff_rgba8(&a, &a.clone(), 16, 16, opts());
        assert!(r.passed());
        assert_eq!(
            (r.over_threshold, r.max_component_area, r.max_pixel_diff),
            (0, 0, 0)
        );
        assert_eq!(
            r.text_bbox,
            Some(PixelRect {
                x0: 3,
                y0: 3,
                x1: 4,
                y1: 4
            })
        );
    }

    #[test]
    fn below_threshold_differences_are_not_counted() {
        let mut a = blank(8, 8);
        let mut b = blank(8, 8);
        set(&mut a, 8, 2, 2, [100, 100, 100, 255]);
        set(&mut b, 8, 2, 2, [124, 100, 100, 255]); // 差正好 24
        let r = diff_rgba8(&a, &b, 8, 8, opts());
        assert_eq!(r.over_threshold, 0);
        assert_eq!(r.max_pixel_diff, 24);
        set(&mut b, 8, 2, 2, [125, 100, 100, 255]); // 差 25
        let r = diff_rgba8(&a, &b, 8, 8, opts());
        assert_eq!(r.over_threshold, 1);
    }

    #[test]
    fn ratio_is_over_threshold_pixels_in_text_bbox() {
        // 包围盒 10x10 = 100 像素，其中 2 个超阈值：2%。
        let mut a = blank(20, 20);
        let mut b = blank(20, 20);
        set(&mut a, 20, 0, 0, [0, 0, 0, 255]);
        set(&mut a, 20, 9, 9, [0, 0, 0, 255]);
        set(&mut b, 20, 0, 0, [255, 255, 255, 255]);
        set(&mut b, 20, 9, 9, [255, 255, 255, 255]);
        let r = diff_rgba8(&a, &b, 20, 20, opts());
        assert_eq!(r.bbox_pixels, 100);
        assert_eq!(r.over_threshold, 2);
        assert!((r.ratio - 0.02).abs() < 1e-12);
        assert!(!r.ratio_ok());
    }

    #[test]
    fn large_connected_difference_fails_even_with_small_ratio() {
        // 大包围盒里一个 6x6 的整块缺失：比例很小，但连通域超过 4x4 的"字形"。
        let mut a = blank(200, 200);
        let b = blank(200, 200);
        for y in 10..16 {
            for x in 10..16 {
                set(&mut a, 200, x, y, [0, 0, 0, 255]);
            }
        }
        set(&mut a, 200, 199, 199, [0, 0, 0, 255]);
        let mut o = opts();
        o.max_ratio = 0.5;
        let r = diff_rgba8(&a, &b, 200, 200, o);
        assert_eq!(r.max_component_area, 36 + 1 - 1); // 6x6；远处的单点是另一个连通域
        assert!(r.ratio_ok());
        assert!(!r.components_ok());
        assert!(!r.passed());
    }

    #[test]
    fn diagonal_neighbours_are_one_component() {
        let mut a = blank(8, 8);
        let b = blank(8, 8);
        for i in 0..4 {
            set(&mut a, 8, i, i, [0, 0, 0, 255]);
        }
        let r = diff_rgba8(&a, &b, 8, 8, opts());
        assert_eq!(r.max_component_area, 4);
    }

    #[test]
    fn heatmap_marks_over_threshold_pixels() {
        let mut a = blank(4, 4);
        let b = blank(4, 4);
        set(&mut a, 4, 1, 1, [0, 0, 0, 255]);
        let r = diff_rgba8(&a, &b, 4, 4, opts());
        assert_eq!(r.heatmap.len(), 4 * 4 * 4);
        let hot = &r.heatmap[20..24]; // 第 1 行第 1 列：(4 + 1) * 4
        assert_eq!(hot[0], 255);
        assert_eq!(hot[3], 255);
        assert_eq!(&r.heatmap[0..4], &[28, 28, 32, 255]);
    }

    #[test]
    fn pure_background_has_no_bbox() {
        let a = blank(4, 4);
        let r = diff_rgba8(&a, &a.clone(), 4, 4, opts());
        assert_eq!(r.text_bbox, None);
        assert_eq!(r.ratio, 0.0);
        assert!(r.passed());
    }

    #[test]
    fn scale_picks_the_threshold() {
        assert_eq!(DiffOptions::for_scale(1.0, 14.0, BG).max_ratio, 0.005);
        assert_eq!(DiffOptions::for_scale(2.0, 28.0, BG).max_ratio, 0.003);
    }
}
