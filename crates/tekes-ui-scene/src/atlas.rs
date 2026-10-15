//! 字形 atlas：把字形位图装进固定大小的页，缓存、淘汰、记录要上传的脏区。
//!
//! atlas 在 CPU 内存里持有每一页的像素，这份内存是唯一的真值；GPU 纹理只是它的拷贝，
//! 由渲染层按 [`GlyphAtlas::take_dirty`] 给出的脏区上传。这一层不认识 GPU API。
//!
//! # 借用的思路
//!
//! 只借 TekesMetalTextKit 的思路，没有搬代码：
//!
//! - 固定大小的页（默认 1024 x 1024，BGRA8，每页 4 MiB），页数有上限；
//! - 每页内用 shelf（行式）装箱，位图之间留 1 像素缝隙；
//! - 缓存键是（字体，字形，scale 的千分之一取整），前景色不进键；
//! - 未命中时的顺序：先试现有页，再加新页（直到 `max_pages`），最后**整页**按 LRU 淘汰；
//! - 淘汰粒度是整页，不做逐字形淘汰：整页作废后，该页所有字形从缓存里一起删掉；
//! - 本帧用过的页被"固定"，不会被淘汰，否则同一帧里先画的字形会指向被覆盖的位置。
//!
//! # 一帧的用法
//!
//! ```text
//! atlas.begin_frame();
//! // 编 scene：对每个字形调用 get_or_insert_with，把返回的 AtlasGlyph 放进 GlyphRun
//! renderer.render(&scene, &mut atlas, clear)   // 上传脏区，然后画
//! ```
//!
//! scene 里的 [`AtlasGlyph`] 只在**编出它的那一帧**里有效：下一次 `begin_frame` 之后，
//! 它所在的页可能被淘汰重用。要跨帧缓存 scene 的话，必须在 atlas 淘汰过（
//! [`AtlasStats::evictions`] 变了）时重编。
//!
//! 一帧里用到的页数超过 `max_pages` 时，放不下的字形返回 `None`（不画），也不进缓存；
//! 下一帧会重试，并记入 [`AtlasStats::dropped`]。
//!
//! # 为什么不需要固定"GPU 飞行中"的页
//!
//! Metal 版的参照实现要避开仍被在途命令缓冲读取的页，因为它是 CPU 直接 `replace` 写纹理。
//! 这里走 wgpu 的 `Queue::write_texture`，不是立即写入：wgpu-core 把它排进
//! "pending writes" 命令缓冲，并在下一次 `submit` 时**放在该次提交所有命令缓冲的最前面**
//! （wgpu-core 30 的 `queue.rs`，`pre_submit` 的结果 `executions.insert(0, ..)`）。
//! 也就是说写入在时间线上排在它之前的所有提交**之后**、本次提交的绘制**之前**，
//! 前一帧仍在读这页的绘制不会看到新数据；Metal 队列按提交顺序执行，
//! wgpu 创建的纹理使用默认的资源冒险跟踪，后一个命令缓冲对同一纹理的写入会等前一个读完。
//! 所以只需要固定"本帧用过的页"（保证同一个 scene 内部自洽），不需要再跟踪 GPU 飞行。
//!
//! # 空字形
//!
//! 光栅化回调返回 `None`、或位图宽高为 0（空格等）时，缓存为"没有位图"，
//! 之后同一个键直接返回 `None`，不会每帧重新光栅化。这类记录不占页，不随页淘汰。

use crate::glyph::{AtlasGlyph, GlyphKey, RasterGlyph};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_ATLAS_ID: AtomicU64 = AtomicU64::new(1);

/// atlas 的配置。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AtlasConfig {
    /// 页的边长（像素），页是正方形。
    pub page_size: u32,
    /// 页数上限。默认 16 页，1024 的页共 64 MiB。
    pub max_pages: u32,
    /// 页内相邻位图之间的缝隙（像素）。位图自带的边距由光栅化器负责，不算在这里。
    pub padding: u32,
}

impl Default for AtlasConfig {
    fn default() -> AtlasConfig {
        AtlasConfig {
            page_size: 1024,
            max_pages: 16,
            padding: 1,
        }
    }
}

/// atlas 的统计数字。其中 `pages`、`bytes` 对应对比方法文档 2.3 节的"atlas 内存"。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AtlasStats {
    /// 当前已有的页数。
    pub pages: u32,
    /// 当前 CPU 侧页缓冲的总字节数（`pages * page_size^2 * 4`）。GPU 纹理按同样的大小分配。
    pub bytes: u64,
    /// 当前缓存着位图的字形数（不含"没有位图"的记录）。
    pub glyphs: u32,
    /// 累计被淘汰的页数。
    pub evictions: u64,
    /// 累计被渲染层取走的脏区数。每个脏区对应一次 `write_texture`。
    pub uploads: u64,
    /// 累计被取走的脏区像素字节数。
    pub upload_bytes: u64,
    /// 累计命中次数。
    pub hits: u64,
    /// 累计未命中次数（含空字形、放不下的）。
    pub misses: u64,
    /// 累计"放不下而没画"的次数：位图比页大，或本帧所有页都已被固定。
    pub dropped: u64,
}

/// 一页里需要上传到 GPU 的矩形（像素）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DirtyRect {
    pub page: u32,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Copy, Debug)]
struct Shelf {
    y: u32,
    height: u32,
    /// 下一个位图的 x。
    x: u32,
}

#[derive(Clone, Copy, Debug)]
struct Bounds {
    x0: u32,
    y0: u32,
    x1: u32,
    y1: u32,
}

struct Page {
    /// BGRA8，`page_size * page_size * 4` 字节。
    pixels: Vec<u8>,
    shelves: Vec<Shelf>,
    /// 下一个新 shelf 的 y。
    next_shelf_y: u32,
    /// 这页上放着的字形，淘汰时据此清缓存。
    keys: Vec<GlyphKey>,
    /// 最后一次被用到（放入或命中）时的帧号。
    last_used: u64,
    dirty: Option<Bounds>,
}

/// 字形 atlas。见模块文档。
pub struct GlyphAtlas {
    config: AtlasConfig,
    id: u64,
    frame: u64,
    pages: Vec<Page>,
    /// `None` 表示"没有位图"。
    map: HashMap<GlyphKey, Option<AtlasGlyph>>,
    /// 本帧已经确认放不下的键，避免同一帧里重复光栅化。
    dropped_this_frame: HashSet<GlyphKey>,
    /// 上一次取走脏区的消费者。换了消费者就整页重传。
    drained_by: Option<u64>,
    stats: AtlasStats,
}

impl std::fmt::Debug for GlyphAtlas {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GlyphAtlas")
            .field("config", &self.config)
            .field("frame", &self.frame)
            .field("stats", &self.stats())
            .finish_non_exhaustive()
    }
}

impl Default for GlyphAtlas {
    fn default() -> GlyphAtlas {
        GlyphAtlas::new(AtlasConfig::default())
    }
}

impl GlyphAtlas {
    /// 新建 atlas。`page_size` 和 `max_pages` 至少按 1 处理。不分配页，页在第一次需要时才建。
    pub fn new(mut config: AtlasConfig) -> GlyphAtlas {
        config.page_size = config.page_size.max(1);
        config.max_pages = config.max_pages.max(1);
        GlyphAtlas {
            config,
            id: NEXT_ATLAS_ID.fetch_add(1, Ordering::Relaxed),
            frame: 0,
            pages: Vec::new(),
            map: HashMap::new(),
            dropped_this_frame: HashSet::new(),
            drained_by: None,
            stats: AtlasStats::default(),
        }
    }

    pub fn config(&self) -> AtlasConfig {
        self.config
    }

    /// 进程内唯一的 atlas 编号。
    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn page_size(&self) -> u32 {
        self.config.page_size
    }

    pub fn page_count(&self) -> u32 {
        self.pages.len() as u32
    }

    /// 一页的像素（BGRA8，预乘 alpha，行紧密排列，`page_size * 4` 字节一行）。
    pub fn page_pixels(&self, page: u32) -> Option<&[u8]> {
        self.pages.get(page as usize).map(|p| p.pixels.as_slice())
    }

    /// 开始新的一帧。之前的帧用过的页不再被固定，可以被淘汰。
    ///
    /// 不调用的话，所有页永远是"本帧用过"的，atlas 满了以后新字形放不下。
    pub fn begin_frame(&mut self) {
        self.frame += 1;
        self.dropped_this_frame.clear();
    }

    pub fn stats(&self) -> AtlasStats {
        let mut s = self.stats;
        s.pages = self.pages.len() as u32;
        s.bytes = self.pages.len() as u64 * self.page_bytes();
        s
    }

    fn page_bytes(&self) -> u64 {
        self.config.page_size as u64 * self.config.page_size as u64 * 4
    }

    /// 取已缓存的字形，没有就调用 `raster` 光栅化并放进 atlas。
    ///
    /// 返回 `None` 表示这个字形不画：没有位图（空格等，`raster` 返回 `None` 或宽高为 0）、
    /// 位图比页还大、或者本帧所有页都被固定而放不下。前两种会缓存，不会再调用 `raster`；
    /// 最后一种不缓存，下一帧重试。命中或放入都会把所在页标为本帧已用。
    pub fn get_or_insert_with<F>(&mut self, key: GlyphKey, raster: F) -> Option<AtlasGlyph>
    where
        F: FnOnce() -> Option<RasterGlyph>,
    {
        if let Some(slot) = self.map.get(&key) {
            self.stats.hits += 1;
            let slot = *slot;
            if let Some(g) = slot {
                self.pages[g.page as usize].last_used = self.frame;
            }
            return slot;
        }
        self.stats.misses += 1;
        if self.dropped_this_frame.contains(&key) {
            self.stats.dropped += 1;
            return None;
        }
        let Some(r) = raster() else {
            self.map.insert(key, None);
            return None;
        };
        if r.width == 0 || r.height == 0 {
            self.map.insert(key, None);
            return None;
        }
        let expected = r.width as usize * r.height as usize * 4;
        if r.pixels.len() != expected {
            debug_assert!(false, "RasterGlyph.pixels 长度与宽高不符");
            self.map.insert(key, None);
            self.stats.dropped += 1;
            return None;
        }
        if r.width > self.config.page_size || r.height > self.config.page_size {
            // 永远放不下：缓存成"没有位图"，不再重试。
            self.map.insert(key, None);
            self.stats.dropped += 1;
            return None;
        }
        let Some((page, x, y)) = self.allocate(r.width, r.height) else {
            self.dropped_this_frame.insert(key);
            self.stats.dropped += 1;
            return None;
        };
        self.blit(page, x, y, &r);
        let p = &mut self.pages[page];
        p.keys.push(key);
        p.last_used = self.frame;
        let glyph = AtlasGlyph {
            page: page as u32,
            x,
            y,
            width: r.width,
            height: r.height,
            left: r.left,
            top: r.top,
            format: r.format,
        };
        self.map.insert(key, Some(glyph));
        self.stats.glyphs += 1;
        Some(glyph)
    }

    /// 找一块 `w` x `h` 的空位：现有页，然后新页，然后淘汰最久没用的未固定页。
    fn allocate(&mut self, w: u32, h: u32) -> Option<(usize, u32, u32)> {
        for i in 0..self.pages.len() {
            if let Some((x, y)) = self.pages[i].try_place(w, h, &self.config) {
                return Some((i, x, y));
            }
        }
        if (self.pages.len() as u32) < self.config.max_pages {
            let mut page = Page::new(&self.config, self.frame);
            // 空页一定放得下（w、h 已检查不超过页大小）。
            let (x, y) = page.try_place(w, h, &self.config)?;
            self.pages.push(page);
            return Some((self.pages.len() - 1, x, y));
        }
        // 淘汰：未固定（本帧没用过）的页里 last_used 最小的，并列取页号小的。
        let victim = self
            .pages
            .iter()
            .enumerate()
            .filter(|(_, p)| p.last_used != self.frame)
            .min_by_key(|(i, p)| (p.last_used, *i))
            .map(|(i, _)| i)?;
        self.evict(victim);
        let (x, y) = self.pages[victim].try_place(w, h, &self.config)?;
        Some((victim, x, y))
    }

    fn evict(&mut self, page: usize) {
        let keys = std::mem::take(&mut self.pages[page].keys);
        for k in keys {
            if self.map.remove(&k).is_some_and(|g| g.is_some()) {
                self.stats.glyphs -= 1;
            }
        }
        let p = &mut self.pages[page];
        p.shelves.clear();
        p.next_shelf_y = 0;
        self.stats.evictions += 1;
    }

    fn blit(&mut self, page: usize, x: u32, y: u32, r: &RasterGlyph) {
        let size = self.config.page_size as usize;
        let p = &mut self.pages[page];
        let row_bytes = r.width as usize * 4;
        for row in 0..r.height as usize {
            let dst = ((y as usize + row) * size + x as usize) * 4;
            let src = row * row_bytes;
            p.pixels[dst..dst + row_bytes].copy_from_slice(&r.pixels[src..src + row_bytes]);
        }
        let b = Bounds {
            x0: x,
            y0: y,
            x1: x + r.width,
            y1: y + r.height,
        };
        p.dirty = Some(match p.dirty {
            None => b,
            Some(d) => Bounds {
                x0: d.x0.min(b.x0),
                y0: d.y0.min(b.y0),
                x1: d.x1.max(b.x1),
                y1: d.y1.max(b.y1),
            },
        });
    }

    /// 把所有页标为整页脏，下一次 [`take_dirty`](GlyphAtlas::take_dirty) 会全部给出。
    pub fn mark_all_dirty(&mut self) {
        let n = self.config.page_size;
        for p in &mut self.pages {
            p.dirty = Some(Bounds {
                x0: 0,
                y0: 0,
                x1: n,
                y1: n,
            });
        }
    }

    /// 取走要上传的脏区（每页最多一个，是该页上所有新写入位图的外接矩形）。
    ///
    /// `consumer` 标识取走的渲染器。atlas 记着上一次是谁取走的：换了消费者，
    /// 或 `force_all` 为真（消费者的 GPU 纹理刚重建），就把所有页整页给出，
    /// 保证每个消费者的 GPU 纹理最终与 CPU 页一致。
    pub fn take_dirty(&mut self, consumer: u64, force_all: bool) -> Vec<DirtyRect> {
        if force_all || self.drained_by != Some(consumer) {
            self.mark_all_dirty();
        }
        self.drained_by = Some(consumer);
        let mut out = Vec::new();
        for (i, p) in self.pages.iter_mut().enumerate() {
            if let Some(d) = p.dirty.take() {
                let r = DirtyRect {
                    page: i as u32,
                    x: d.x0,
                    y: d.y0,
                    width: d.x1 - d.x0,
                    height: d.y1 - d.y0,
                };
                self.stats.uploads += 1;
                self.stats.upload_bytes += r.width as u64 * r.height as u64 * 4;
                out.push(r);
            }
        }
        out
    }

    /// 丢掉所有缓存的字形（页保留并清空）。例如字体集合整体变了。
    /// 不计入 `evictions`。已经编进 scene 的 [`AtlasGlyph`] 全部作废。
    pub fn clear(&mut self) {
        self.map.clear();
        self.dropped_this_frame.clear();
        self.stats.glyphs = 0;
        for p in &mut self.pages {
            p.keys.clear();
            p.shelves.clear();
            p.next_shelf_y = 0;
        }
    }
}

impl Page {
    fn new(config: &AtlasConfig, frame: u64) -> Page {
        let n = config.page_size as usize;
        Page {
            pixels: vec![0; n * n * 4],
            shelves: Vec::new(),
            next_shelf_y: 0,
            keys: Vec::new(),
            last_used: frame,
            dirty: None,
        }
    }

    /// shelf 装箱：在够高、够宽的 shelf 里选最矮的一条；没有就在下面开一条新 shelf。
    fn try_place(&mut self, w: u32, h: u32, config: &AtlasConfig) -> Option<(u32, u32)> {
        let size = config.page_size;
        let best = self
            .shelves
            .iter()
            .enumerate()
            .filter(|(_, s)| s.height >= h && s.x + w <= size)
            .min_by_key(|(i, s)| (s.height, *i))
            .map(|(i, _)| i);
        if let Some(i) = best {
            let s = &mut self.shelves[i];
            let pos = (s.x, s.y);
            s.x += w + config.padding;
            return Some(pos);
        }
        if self.next_shelf_y + h <= size {
            let y = self.next_shelf_y;
            self.shelves.push(Shelf {
                y,
                height: h,
                x: w + config.padding,
            });
            self.next_shelf_y = y + h + config.padding;
            return Some((0, y));
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::glyph::{FontKey, RasterFormat};

    fn key(glyph: u16) -> GlyphKey {
        GlyphKey::new(FontKey(1), glyph, 2000)
    }

    /// w x h 的位图，每个字节都填 `fill`。
    fn bitmap(w: u32, h: u32, fill: u8) -> RasterGlyph {
        RasterGlyph {
            left: -1,
            top: -(h as i32),
            width: w,
            height: h,
            format: RasterFormat::Mask,
            pixels: vec![fill; (w * h * 4) as usize],
        }
    }

    fn small(max_pages: u32) -> GlyphAtlas {
        GlyphAtlas::new(AtlasConfig {
            page_size: 64,
            max_pages,
            padding: 1,
        })
    }

    fn insert(a: &mut GlyphAtlas, glyph: u16, w: u32, h: u32, fill: u8) -> Option<AtlasGlyph> {
        a.get_or_insert_with(key(glyph), || Some(bitmap(w, h, fill)))
    }

    fn overlaps(a: &AtlasGlyph, b: &AtlasGlyph) -> bool {
        a.page == b.page
            && a.x < b.x + b.width
            && b.x < a.x + a.width
            && a.y < b.y + b.height
            && b.y < a.y + a.height
    }

    #[test]
    fn default_config_matches_the_reference_pages() {
        let c = AtlasConfig::default();
        assert_eq!((c.page_size, c.max_pages, c.padding), (1024, 16, 1));
        let a = GlyphAtlas::default();
        assert_eq!(a.stats(), AtlasStats::default());
    }

    #[test]
    fn packs_into_shelves_with_one_pixel_gaps() {
        let mut a = small(1);
        a.begin_frame();
        let g0 = insert(&mut a, 0, 20, 10, 1).unwrap();
        let g1 = insert(&mut a, 1, 20, 10, 2).unwrap();
        let g2 = insert(&mut a, 2, 20, 10, 3).unwrap();
        // 同一条 shelf：x 依次为 0、21、42，缝隙 1。
        assert_eq!((g0.x, g0.y), (0, 0));
        assert_eq!((g1.x, g1.y), (21, 0));
        assert_eq!((g2.x, g2.y), (42, 0));
        // 第四个宽度放不下（63 + 20 > 64），换新 shelf：y = 10 + 1。
        let g3 = insert(&mut a, 3, 20, 10, 4).unwrap();
        assert_eq!((g3.x, g3.y), (0, 11));
        for (i, x) in [&g0, &g1, &g2, &g3].iter().enumerate() {
            for y in [&g0, &g1, &g2, &g3].iter().skip(i + 1) {
                assert!(!overlaps(x, y));
            }
        }
        // 页内像素就是传入的位图。
        let page = a.page_pixels(0).unwrap();
        let px = |x: u32, y: u32| page[((y * 64 + x) * 4) as usize];
        assert_eq!(px(0, 0), 1);
        assert_eq!(px(21, 9), 2);
        assert_eq!(px(42, 0), 3);
        assert_eq!(px(0, 11), 4);
        // 缝隙没被写。
        assert_eq!(px(20, 0), 0);
        assert_eq!(px(0, 10), 0);
    }

    #[test]
    fn short_glyphs_reuse_a_taller_shelf_with_best_fit() {
        let mut a = small(1);
        a.begin_frame();
        let tall = insert(&mut a, 0, 30, 20, 1).unwrap();
        let short = insert(&mut a, 1, 20, 8, 2).unwrap();
        // 矮字形放进第一条（20 高）shelf 的剩余宽度，而不是开新 shelf。
        assert_eq!(short.y, tall.y);
        assert_eq!(short.x, 31);
        // 太高的才开新 shelf。
        let big = insert(&mut a, 2, 10, 25, 3).unwrap();
        assert_eq!(big.y, 21);
    }

    #[test]
    fn reuses_keys_without_rasterizing_again() {
        let mut a = small(1);
        a.begin_frame();
        let mut calls = 0;
        let mut get = |a: &mut GlyphAtlas| {
            a.get_or_insert_with(key(7), || {
                calls += 1;
                Some(bitmap(5, 7, 9))
            })
        };
        let first = get(&mut a).unwrap();
        let second = get(&mut a).unwrap();
        assert_eq!(first, second);
        assert_eq!(calls, 1);
        let s = a.stats();
        assert_eq!((s.hits, s.misses, s.glyphs), (1, 1, 1));
    }

    #[test]
    fn empty_glyphs_are_cached_as_no_bitmap() {
        let mut a = small(1);
        a.begin_frame();
        let mut calls = 0;
        for _ in 0..3 {
            let r = a.get_or_insert_with(key(32), || {
                calls += 1;
                None
            });
            assert!(r.is_none());
        }
        // 位图宽或高为 0 同样缓存。
        let mut zero_calls = 0;
        for _ in 0..3 {
            let r = a.get_or_insert_with(key(33), || {
                zero_calls += 1;
                Some(bitmap(0, 5, 0))
            });
            assert!(r.is_none());
        }
        assert_eq!((calls, zero_calls), (1, 1));
        let s = a.stats();
        assert_eq!((s.pages, s.glyphs), (0, 0), "空字形不占页");
    }

    #[test]
    fn oversize_glyph_is_rejected_and_cached() {
        let mut a = small(1);
        a.begin_frame();
        let mut calls = 0;
        for _ in 0..2 {
            let r = a.get_or_insert_with(key(1), || {
                calls += 1;
                Some(bitmap(65, 4, 1))
            });
            assert!(r.is_none());
        }
        assert_eq!(calls, 1);
        assert_eq!(a.stats().dropped, 1);
    }

    #[test]
    fn adds_pages_up_to_max_pages() {
        let mut a = small(3);
        a.begin_frame();
        // 每页只能放一个 40x40。
        for g in 0..3 {
            assert!(insert(&mut a, g, 40, 40, g as u8 + 1).is_some());
        }
        let s = a.stats();
        assert_eq!(s.pages, 3);
        assert_eq!(s.bytes, 3 * 64 * 64 * 4);
        assert_eq!(s.evictions, 0);
        // 第四个：三页本帧都用过（被固定），放不下。
        assert!(insert(&mut a, 3, 40, 40, 4).is_none());
        let s = a.stats();
        assert_eq!(s.pages, 3, "不超过 max_pages");
        assert_eq!(s.evictions, 0, "固定的页不被淘汰");
        assert_eq!(s.dropped, 1);
        // 同一帧再要同一个键：不重复光栅化。
        let mut called = false;
        let r = a.get_or_insert_with(key(3), || {
            called = true;
            Some(bitmap(40, 40, 4))
        });
        assert!(r.is_none() && !called);
    }

    #[test]
    fn evicts_the_least_recently_used_unpinned_page() {
        let mut a = small(3);
        a.begin_frame(); // 帧 1
        let g0 = insert(&mut a, 0, 40, 40, 1).unwrap(); // 页 0
        a.begin_frame(); // 帧 2
        let g1 = insert(&mut a, 1, 40, 40, 2).unwrap(); // 页 1
        a.begin_frame(); // 帧 3
        let g2 = insert(&mut a, 2, 40, 40, 3).unwrap(); // 页 2
        assert_eq!((g0.page, g1.page, g2.page), (0, 1, 2));
        a.begin_frame(); // 帧 4
        // 命中页 0，使它比页 1 新。
        assert!(insert(&mut a, 0, 40, 40, 1).is_some());
        // 新字形放不下 -> 淘汰 last_used 最小的未固定页：页 1（帧 2）。
        let g3 = insert(&mut a, 3, 40, 40, 4).unwrap();
        assert_eq!(g3.page, 1);
        assert_eq!(a.stats().evictions, 1);
        assert_eq!(a.stats().pages, 3);
        // 页 1 上的旧字形失效：再要它得重新光栅化。
        let mut called = false;
        let again = a.get_or_insert_with(key(1), || {
            called = true;
            Some(bitmap(40, 40, 2))
        });
        // 这时页 0 已在本帧用过（固定）、页 1 固定（g3）、页 2 未固定 -> 淘汰页 2。
        assert!(called);
        assert_eq!(again.unwrap().page, 2);
        assert_eq!(a.stats().evictions, 2);
        // 页 0 的字形仍然有效，没被碰。
        let mut touched = false;
        let kept = a.get_or_insert_with(key(0), || {
            touched = true;
            None
        });
        assert!(!touched);
        assert_eq!(kept, Some(g0));
    }

    #[test]
    fn never_evicts_a_page_used_this_frame() {
        let mut a = small(2);
        a.begin_frame();
        let g0 = insert(&mut a, 0, 40, 40, 1).unwrap();
        let g1 = insert(&mut a, 1, 40, 40, 2).unwrap();
        a.begin_frame();
        // 只碰页 0。
        assert!(insert(&mut a, 0, 40, 40, 1).is_some());
        // 需要新位置：页 0 固定，只能淘汰页 1。
        let g2 = insert(&mut a, 2, 40, 40, 3).unwrap();
        assert_eq!(g2.page, g1.page);
        // 再来一个：页 0、页 1 都固定了，放不下，而且 g0、g2 都还有效。
        assert!(insert(&mut a, 3, 40, 40, 4).is_none());
        assert_eq!(a.stats().evictions, 1);
        assert_eq!(insert(&mut a, 0, 40, 40, 1), Some(g0));
        assert_eq!(insert(&mut a, 2, 40, 40, 3), Some(g2));
    }

    #[test]
    fn dropped_glyph_is_retried_next_frame() {
        let mut a = small(1);
        a.begin_frame();
        assert!(insert(&mut a, 0, 40, 40, 1).is_some());
        assert!(insert(&mut a, 1, 40, 40, 2).is_none());
        a.begin_frame();
        // 页 0 不再固定，新字形淘汰它。
        let g1 = insert(&mut a, 1, 40, 40, 2).unwrap();
        assert_eq!(g1.page, 0);
        assert_eq!(a.stats().evictions, 1);
        assert_eq!(a.stats().glyphs, 1);
    }

    #[test]
    fn dirty_rects_cover_new_bitmaps_and_are_drained_once() {
        let mut a = small(1);
        a.begin_frame();
        let consumer = 42;
        assert!(a.take_dirty(consumer, false).is_empty());
        insert(&mut a, 0, 10, 8, 1).unwrap();
        insert(&mut a, 1, 12, 5, 2).unwrap();
        let d = a.take_dirty(consumer, false);
        // 两个位图在同一条 shelf：外接矩形 x 0..23，y 0..8。
        assert_eq!(
            d,
            vec![DirtyRect {
                page: 0,
                x: 0,
                y: 0,
                width: 23,
                height: 8
            }]
        );
        assert!(a.take_dirty(consumer, false).is_empty());
        let s = a.stats();
        assert_eq!(s.uploads, 1);
        assert_eq!(s.upload_bytes, 23 * 8 * 4);
    }

    #[test]
    fn a_new_consumer_or_force_gets_whole_pages() {
        let mut a = small(2);
        a.begin_frame();
        insert(&mut a, 0, 40, 40, 1).unwrap();
        insert(&mut a, 1, 40, 40, 2).unwrap();
        assert_eq!(a.take_dirty(1, false).len(), 2);
        assert!(a.take_dirty(1, false).is_empty());
        // 另一个渲染器来取：整页重传。
        let d = a.take_dirty(2, false);
        assert_eq!(d.len(), 2);
        assert!(
            d.iter()
                .all(|r| (r.x, r.y, r.width, r.height) == (0, 0, 64, 64))
        );
        // 原来的消费者回来，同样整页重传（它错过了期间的变化）。
        assert_eq!(a.take_dirty(1, false).len(), 2);
        // force_all：纹理重建。
        assert_eq!(a.take_dirty(1, true).len(), 2);
    }

    #[test]
    fn eviction_then_reinsert_exposes_only_the_new_pixels() {
        let mut a = small(1);
        a.begin_frame();
        insert(&mut a, 0, 40, 40, 0xAA).unwrap();
        a.take_dirty(1, false);
        a.begin_frame();
        // 放不下第二个，淘汰整页后，新字形放在同一位置（0, 0）。
        let g = insert(&mut a, 1, 40, 40, 0x55).unwrap();
        assert_eq!((g.page, g.x, g.y), (0, 0, 0));
        let d = a.take_dirty(1, false);
        assert_eq!(
            d,
            vec![DirtyRect {
                page: 0,
                x: 0,
                y: 0,
                width: 40,
                height: 40
            }]
        );
        let page = a.page_pixels(0).unwrap();
        assert!(page[..4].iter().all(|&b| b == 0x55));
        assert_eq!(a.stats().glyphs, 1);
        // 旧键作废：再要它会重新调用光栅化回调。
        let mut called = false;
        a.get_or_insert_with(key(0), || {
            called = true;
            None
        });
        assert!(called, "旧字形随页一起失效");
    }

    #[test]
    fn clear_drops_glyphs_but_keeps_pages() {
        let mut a = small(2);
        a.begin_frame();
        insert(&mut a, 0, 10, 10, 1).unwrap();
        a.clear();
        let s = a.stats();
        assert_eq!((s.pages, s.glyphs, s.evictions), (1, 0, 0));
        let g = insert(&mut a, 0, 10, 10, 2).unwrap();
        assert_eq!((g.page, g.x, g.y), (0, 0, 0));
    }

    #[test]
    fn color_format_is_preserved() {
        let mut a = small(1);
        a.begin_frame();
        let g = a
            .get_or_insert_with(key(9), || {
                let mut r = bitmap(4, 4, 255);
                r.format = RasterFormat::Color;
                Some(r)
            })
            .unwrap();
        assert_eq!(g.format, RasterFormat::Color);
        assert_eq!((g.left, g.top), (-1, -4));
    }
}
