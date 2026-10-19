//! 性能 harness：`TEKES_PERF=scroll[:秒数]`，F1 门禁“120Hz 下滚动不掉帧”的测量入口。
//!
//! 做的事（对应 docs/comparison-method.md 第 2.3 节）：
//!
//! 1. 窗口创建后打开持续动画（显示链接），先预热字形 atlas，再空转 [`WARMUP_S`] 秒不记录；
//! 2. 之后 `秒数`（默认 10）秒里，每个显示链接回调画一帧，内容以恒定速度（默认 1200 pt/s，
//!    到头折返）垂直滚动，滚动位置按回调的 `target_timestamp` 算，所以掉帧会表现为画面跳变而不是被时间平滑掉；
//! 3. 每帧记录 CPU 时间（宿主量）、`nextDrawable` 等待、编码、提交（渲染器量）、GPU 时间（时间戳查询）、
//!    显示链接的时间戳，见 `tekes_ui_render::stats` 的说明；
//! 4. 到时间停止提交新帧，收齐在途呈现收据（最多 500ms），再关闭动画，**等 5 秒**确认空闲。
//!
//! 环境变量：
//! - `TEKES_PERF=scroll` 或 `scroll:10`：开启并指定测量秒数；
//! - `TEKES_PERF_OUT=路径`：JSON 报告的路径，默认当前目录 `tekes-perf-<unix 秒>.json`；
//! - `TEKES_PERF_CONTENT=quads|text|mixed|editor|scroll-area|icons|motion|scene3d`：滚动的内容，默认 `mixed`；
//! - `TEKES_PERF_SPEED=点/秒`：滚动速度，默认 1200；
//! - `TEKES_PERF_COMMIT=...`：写进报告的提交标识（`scripts/f1-scroll-gate.sh` 会设置）。
//! - `TEKES_PERF_DRAIN_TIMER_ONLY=1`：仅诊断停止阶段的独立唤醒，不用于性能验收。
//! - `TEKES_PERF_TEST_EARLY_CLOSE_MS=...`：仅诊断提前关闭的失败边界。
//!
//! 窗口放哪块屏幕：perf 模式请求 120Hz，摆放规则选内建屏；`TEKESUI_WINDOW_PLACEMENT=builtin` 也可以显式指定。
//!
//! 内容：
//! - `quads`：把 Gallery 的各页（每页一个 1200x800 的整屏，含目录列）上下叠起来，共 8 屏；
//! - `text`：`fixtures/text/sentences-a.txt` 每行一个段落、`long-paragraph.txt` 一个长段落，
//!   循环排下去，约 8 屏高，用 CoreText 排一次，每帧 `push_paragraph`（atlas 命中，不光栅化）；
//! - `mixed`：整屏和文字段交替。
//!
//! 每帧只编可见部分：按段落和图元的包围盒做裁剪（真实的虚拟化也会这样做），
//! 但整屏的 Gallery 页每帧都重新 `build_scene`（计入 CPU 时间），不缓存。
//! 文字的排版结果缓存，每帧只做 `push_paragraph`。

use std::collections::{HashMap, HashSet};
use std::ops::Range;
use std::path::PathBuf;
use std::sync::atomic::{AtomicI32, Ordering};
use std::time::Instant;

use serde_json::{Value, json};
use tekes_ui_platform_macos::selftest::{self, Action};
use tekes_ui_platform_macos::{AnimationFrame, Window};
use tekes_ui_render::{Distribution, FrameSample, FrameStats, FrameTiming, Renderer};
use tekes_ui_scene::{AtlasStats, GlyphRun, ImageQuad, Point, Primitive, Quad, Scene};
use tekes_ui_text::model::{Paragraph, ShapeOptions};
use tekes_ui_text::style::TextStyle;
use tekes_ui_tokens::{ColorRole, Rgba, Theme};

use crate::gallery::{Command, Gallery, Page};

/// 预热（动画开着、不记录）的秒数：让显示链接落到稳定的 120Hz、drawable 池就位。
pub const WARMUP_S: f64 = 1.5;
/// 测量结束后等多久再确认显示链接已静默。
pub const IDLE_WAIT_MS: u64 = 5000;
/// 整屏的高度（pt），等于 Gallery 固定框。
const SCREEN_H: f32 = crate::gallery::CONTENT_HEIGHT;
/// 文字段的左边距和换行宽度（pt）。
const TEXT_X: f32 = 50.0;
const TEXT_WIDTH: f64 = 1100.0;
/// 段落之间的空隙（pt）。
const PARAGRAPH_GAP: f32 = 6.0;
/// 文档由几段（整屏或文字段）组成。
const SEGMENTS: usize = 8;

/// 进程退出码：0 完整跑完；2 中途被关或没有跑起来。
static EXIT_CODE: AtomicI32 = AtomicI32::new(2);

pub fn exit_code() -> i32 {
    EXIT_CODE.load(Ordering::Relaxed)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContentKind {
    Quads,
    Text,
    Mixed,
    Editor,
    ScrollArea,
    Icons,
    Motion,
    Symbols,
    Scene3d,
}

impl ContentKind {
    fn name(self) -> &'static str {
        match self {
            ContentKind::Quads => "quads",
            ContentKind::Text => "text",
            ContentKind::Mixed => "mixed",
            ContentKind::Editor => "editor",
            ContentKind::ScrollArea => "scroll-area",
            ContentKind::Icons => "icons",
            ContentKind::Motion => "motion",
            ContentKind::Symbols => "symbols",
            ContentKind::Scene3d => "scene3d",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PerfConfig {
    /// 测量秒数（不含预热）。
    pub seconds: f64,
    /// 滚动速度，pt/s。
    pub speed: f64,
    pub content: ContentKind,
    pub out: Option<PathBuf>,
}

impl PerfConfig {
    /// 读环境变量；没有 `TEKES_PERF` 返回 `None`，值不合法时打印原因并退出（2）。
    pub fn from_env() -> Option<PerfConfig> {
        let spec = std::env::var("TEKES_PERF").ok()?;
        let get = |k: &str| std::env::var(k).ok();
        match PerfConfig::parse(
            &spec,
            get("TEKES_PERF_OUT").as_deref(),
            get("TEKES_PERF_CONTENT").as_deref(),
            get("TEKES_PERF_SPEED").as_deref(),
        ) {
            Ok(c) => Some(c),
            Err(e) => {
                eprintln!("TEKES_PERF：{e}");
                std::process::exit(2);
            }
        }
    }

    pub fn parse(
        spec: &str,
        out: Option<&str>,
        content: Option<&str>,
        speed: Option<&str>,
    ) -> Result<PerfConfig, String> {
        let (kind, secs) = match spec.split_once(':') {
            Some((k, s)) => (k, Some(s)),
            None => (spec, None),
        };
        if kind != "scroll" {
            return Err(format!("不认识的模式 {kind:?}，目前只有 scroll"));
        }
        let seconds = match secs {
            None => 10.0,
            Some(s) => s
                .parse::<f64>()
                .map_err(|_| format!("秒数 {s:?} 不是数字"))?,
        };
        if !(seconds.is_finite() && seconds > 0.0) {
            return Err("秒数必须是正数".into());
        }
        let speed = match speed {
            None => 1200.0,
            Some(s) => s
                .parse::<f64>()
                .map_err(|_| format!("速度 {s:?} 不是数字"))?,
        };
        if !(speed.is_finite() && speed > 0.0) {
            return Err("速度必须是正数".into());
        }
        let content = match content {
            None | Some("mixed") => ContentKind::Mixed,
            Some("quads") => ContentKind::Quads,
            Some("text") => ContentKind::Text,
            Some("editor") => ContentKind::Editor,
            Some("scroll-area") => ContentKind::ScrollArea,
            Some("icons") => ContentKind::Icons,
            Some("motion") => ContentKind::Motion,
            Some("symbols") => ContentKind::Symbols,
            Some("scene3d") => ContentKind::Scene3d,
            Some(other) => {
                return Err(format!(
                    "TEKES_PERF_CONTENT 取 quads、text、mixed、editor、scroll-area、icons、motion、symbols、scene3d，收到 {other:?}"
                ));
            }
        };
        Ok(PerfConfig {
            seconds,
            speed,
            content,
            out: out.map(PathBuf::from),
        })
    }
}

/// 位置按“三角波”折返：`pos` 单调增长，结果在 `0..=range` 之间来回。
pub fn bounce(pos: f64, range: f64) -> f64 {
    if range <= 0.0 {
        return 0.0;
    }
    let period = 2.0 * range;
    let p = pos.rem_euclid(period);
    if p <= range { p } else { period - p }
}

// ---------------------------------------------------------------------------------------------
// 文档规划（纯函数）

#[derive(Clone, Debug, PartialEq)]
enum Kind {
    /// 第 `n` 个 Gallery 整屏。
    Screen(usize),
    /// `placed` 里的一段文字。
    Text(Range<usize>),
}

#[derive(Clone, Debug, PartialEq)]
struct Segment {
    y: f32,
    h: f32,
    kind: Kind,
}

/// 文字段里的一个段落：下标和文档绝对 y。
#[derive(Clone, Copy, Debug, PartialEq)]
struct Placed {
    para: usize,
    y: f32,
}

#[derive(Clone, Debug, Default, PartialEq)]
struct Plan {
    segments: Vec<Segment>,
    placed: Vec<Placed>,
    height: f32,
}

/// 把 `SEGMENTS` 段竖着排起来。`para_heights` 是已排好版的段落高度，文字段按顺序循环取用，
/// 每段取到累计高度达到一屏为止。
fn plan_document(content: ContentKind, para_heights: &[f32]) -> Plan {
    let mut plan = Plan::default();
    let mut y = 0.0f32;
    let mut next_para = 0usize;
    let mut screens = 0usize;
    for i in 0..SEGMENTS {
        let want_text = match content {
            ContentKind::Quads => false,
            ContentKind::Text
            | ContentKind::Editor
            | ContentKind::ScrollArea
            | ContentKind::Icons
            | ContentKind::Motion
            | ContentKind::Symbols
            | ContentKind::Scene3d => true,
            ContentKind::Mixed => i % 2 == 1,
        };
        if !want_text || para_heights.is_empty() {
            plan.segments.push(Segment {
                y,
                h: SCREEN_H,
                kind: Kind::Screen(screens),
            });
            screens += 1;
            y += SCREEN_H;
            continue;
        }
        let start = plan.placed.len();
        let seg_y = y;
        while y - seg_y < SCREEN_H {
            let p = next_para % para_heights.len();
            next_para += 1;
            plan.placed.push(Placed { para: p, y });
            y += para_heights[p] + PARAGRAPH_GAP;
        }
        plan.segments.push(Segment {
            y: seg_y,
            h: y - seg_y,
            kind: Kind::Text(start..plan.placed.len()),
        });
    }
    plan.height = y;
    plan
}

/// 把 `src` 里的图元平移 `dy` 后追加到 `dst`，丢掉完全落在 `0..view_h` 之外的。
/// 字形按整个 run 判断：run 内最上和最下的字形原点都在可见范围外同一侧才丢。
fn translate_into(dst: &mut Scene, src: &Scene, dy: f32, view_h: f32) {
    for p in src.primitives() {
        match p {
            Primitive::Shadow(shadow) => {
                let mut moved = *shadow;
                moved.rect = moved.rect.offset(0.0, dy);
                let tail = moved.outset.max(0.0) + moved.blur_radius * 2.5 + 1.0;
                let bounds = moved
                    .rect
                    .offset(moved.offset.x, moved.offset.y)
                    .inset(-tail);
                if bounds.max_y() >= 0.0 && bounds.min_y() <= view_h {
                    dst.push_shadow(moved);
                }
            }
            Primitive::Quad(q) => {
                let rect = q.rect.offset(0.0, dy);
                if rect.max_y() < 0.0 || rect.min_y() > view_h {
                    continue;
                }
                dst.push_quad(Quad { rect, ..*q });
            }
            Primitive::Glyphs(run) => {
                // 字形原点是基线，上下各留 64pt 余量覆盖字形的上伸和下伸。
                let (lo, hi) = run.glyphs.iter().fold((f32::MAX, f32::MIN), |(lo, hi), g| {
                    (lo.min(g.origin.y + dy), hi.max(g.origin.y + dy))
                });
                if run.glyphs.is_empty() || hi < -64.0 || lo > view_h + 64.0 {
                    continue;
                }
                let mut moved = GlyphRun {
                    color: run.color,
                    glyphs: run.glyphs.clone(),
                };
                for g in &mut moved.glyphs {
                    g.origin = Point::new(g.origin.x, g.origin.y + dy);
                }
                dst.push_glyphs(moved);
            }
            Primitive::Image(im) => {
                let rect = im.rect.offset(0.0, dy);
                if rect.max_y() < 0.0 || rect.min_y() > view_h {
                    continue;
                }
                dst.push_image(ImageQuad { rect, ..*im });
            }
            Primitive::TransformedImage(im) => {
                let mut moved = *im;
                moved.quad.rect = moved.quad.rect.offset(0.0, dy);
                let bounds = moved.bounds();
                if bounds.max_y() >= 0.0 && bounds.min_y() <= view_h {
                    dst.push_transformed_image(moved);
                }
            }
            Primitive::PushClip(rect) => dst.push_clip(rect.offset(0.0, dy)),
            Primitive::PopClip => dst.pop_clip(),
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------------------------
// 进程信息

/// 本进程累计 CPU 时间（用户加系统），秒。
fn process_cpu_seconds() -> f64 {
    #[repr(C)]
    struct Timespec {
        tv_sec: i64,
        tv_nsec: i64,
    }
    unsafe extern "C" {
        fn clock_gettime(clock: u32, ts: *mut Timespec) -> i32;
    }
    // macOS 的 CLOCK_PROCESS_CPUTIME_ID。
    const CLOCK_PROCESS_CPUTIME_ID: u32 = 12;
    let mut ts = Timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // SAFETY: ts 是有效的可写 timespec，布局与 C 一致。
    let rc = unsafe { clock_gettime(CLOCK_PROCESS_CPUTIME_ID, &mut ts) };
    if rc != 0 {
        return 0.0;
    }
    ts.tv_sec as f64 + ts.tv_nsec as f64 * 1e-9
}

fn command_output(program: &str, args: &[&str]) -> String {
    std::process::Command::new(program)
        .args(args)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

fn machine_info() -> Value {
    let low_power = command_output("pmset", &["-g"])
        .lines()
        .find(|l| l.contains("lowpowermode"))
        .map(|l| l.split_whitespace().last().unwrap_or("").to_string());
    json!({
        "model": command_output("sysctl", &["-n", "hw.model"]),
        "chip": command_output("sysctl", &["-n", "machdep.cpu.brand_string"]),
        "os": format!("macOS {}", command_output("sw_vers", &["-productVersion"])),
        "os_build": command_output("sw_vers", &["-buildVersion"]),
        "low_power_mode": low_power,
    })
}

// ---------------------------------------------------------------------------------------------
// harness

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    /// 动画已开，等第一帧或预热时间未到。
    WarmUp,
    /// 记录样本。
    Measure,
    /// 不再提交新帧；让最后的已提交 drawable 完成呈现，避免 invalidate 取消它。
    PresentationDrain,
    /// 动画已关，等 5 秒的空闲确认。
    Cooldown,
    Done,
}

struct Stop {
    callbacks: u64,
    drawable_counts: (u64, u64),
    cpu: f64,
    at: Instant,
}

fn measurement_complete(phase: Phase, idle: Option<&Value>) -> bool {
    phase == Phase::Cooldown
        && idle
            .and_then(|idle| idle["wait_s"].as_f64())
            .is_some_and(|wait| wait.is_finite() && wait >= IDLE_WAIT_MS as f64 / 1000.0)
}

const PRESENTATION_DRAIN_TIMEOUT_S: f64 = 0.5;

struct PresentationDrain {
    started: Instant,
    expected: HashSet<u64>,
    received: usize,
    unpresented: usize,
    timed_out: bool,
    wait_ms: Option<f64>,
}

impl PresentationDrain {
    fn new(expected: impl IntoIterator<Item = u64>) -> Self {
        Self {
            started: Instant::now(),
            expected: expected.into_iter().collect(),
            received: 0,
            unpresented: 0,
            timed_out: false,
            wait_ms: None,
        }
    }

    fn poll(
        &mut self,
        receipts: &[tekes_ui_platform_macos::PresentedFrame],
        elapsed_s: f64,
    ) -> bool {
        let matching: HashMap<_, _> = receipts
            .iter()
            .filter(|receipt| self.expected.contains(&receipt.serial))
            .map(|receipt| (receipt.serial, receipt.presented_at))
            .collect();
        self.received = matching.len();
        self.unpresented = matching
            .values()
            .filter(|time| !time.is_finite() || **time <= 0.0)
            .count();
        self.timed_out =
            self.received < self.expected.len() && elapsed_s >= PRESENTATION_DRAIN_TIMEOUT_S;
        if self.received == self.expected.len() || self.timed_out {
            self.wait_ms = Some(elapsed_s * 1000.0);
            return true;
        }
        false
    }
}

/// 每帧要用到的宿主部件。
pub struct FrameCtx<'a> {
    pub scene: &'a mut Scene,
    pub renderer: &'a mut Renderer,
    pub window: &'a mut Window,
    pub theme: Theme,
}

#[path = "symbol_perf.rs"]
mod symbol_perf;

pub struct Perf {
    symbols: Option<symbol_perf::SymbolWorkload>,
    cfg: PerfConfig,
    measure_gpu: bool,
    plan: Plan,
    paragraphs: Vec<Paragraph>,
    /// 私有的 Gallery（自带文字引擎和 atlas，窗口里的那个 Gallery 不参与），用来逐页编整屏，不影响窗口里的那个。
    gallery: Gallery,
    tmp: Scene,
    phase: Phase,
    first_target: Option<f64>,
    measure_start_target: Option<f64>,
    measure_start_cpu: f64,
    measure_start: Option<Instant>,
    measure_end_cpu: f64,
    measure_end: Option<Instant>,
    cpu_rate_last: Option<(Instant, f64)>,
    cpu_rates: Vec<[f64; 2]>,
    prev_render_id: Option<u64>,
    samples: Vec<FrameSample>,
    dispatch_delays: Vec<f64>,
    render_ids: Vec<Option<u64>>,
    drawable_serials: Vec<u64>,
    visibility_counts: [usize; 2],
    visibility_changes: Vec<(f64, bool, bool)>,
    scroll_cycle: Option<u64>,
    scroll_stage: u8,
    /// 每帧（quads, glyphs, images, draws）。
    counts: Vec<[u32; 4]>,
    timings: Vec<FrameTiming>,
    stop: Option<Stop>,
    presentation_drain: Option<PresentationDrain>,
    drain_timer_only: bool,
    prewarm_ms: f64,
    started: Instant,
    motion_step: Option<u64>,
    scene3d_start: Value,
    scene3d_pose_bytes: u64,
    scene3d_deformed_meshes: u64,
    animation_target: Option<f64>,
    path: PathBuf,
    /// 窗口信息（开始时记下）。
    window_info: Value,
    scale: f32,
    view_h: f32,
}

impl std::fmt::Debug for Perf {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Perf")
            .field("phase", &self.phase)
            .field("samples", &self.samples.len())
            .finish_non_exhaustive()
    }
}

const SENTENCES_A: &str = include_str!("../../../fixtures/text/sentences-a.txt");
const LONG_PARAGRAPH: &str = include_str!("../../../fixtures/text/long-paragraph.txt");

impl Perf {
    /// 排版文字（一次，不计入测量）。
    pub fn new(cfg: PerfConfig) -> Perf {
        let mut gallery = Gallery::new();
        let style = TextStyle::system(TextStyle::DEFAULT_SIZE);
        let options = ShapeOptions::with_width(TEXT_WIDTH);
        let mut paragraphs: Vec<Paragraph> = SENTENCES_A
            .lines()
            .filter(|l| !l.is_empty())
            .map(|l| gallery.text_mut().shape(l, &style, &options))
            .collect();
        paragraphs.extend(
            LONG_PARAGRAPH
                .lines()
                .filter(|l| !l.is_empty())
                .map(|l| gallery.text_mut().shape(l, &style, &options)),
        );
        let heights: Vec<f32> = paragraphs.iter().map(|p| p.height as f32).collect();
        let plan = plan_document(cfg.content, &heights);
        let path = cfg.out.clone().unwrap_or_else(|| {
            let t = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_secs());
            PathBuf::from(format!("tekes-perf-{t}.json"))
        });
        let capacity = (cfg.seconds.mul_add(132.0, 240.0).min(100_000.0)) as usize;
        let symbols =
            (cfg.content == ContentKind::Symbols).then(symbol_perf::SymbolWorkload::from_env);
        Perf {
            symbols,
            measure_gpu: std::env::var("TEKES_PERF_GPU_TIMING").as_deref() != Ok("0"),
            cfg,
            plan,
            paragraphs,
            gallery,
            tmp: Scene::new(),
            phase: Phase::WarmUp,
            first_target: None,
            measure_start_target: None,
            measure_start_cpu: 0.0,
            measure_start: None,
            measure_end_cpu: 0.0,
            measure_end: None,
            cpu_rate_last: None,
            cpu_rates: Vec::new(),
            prev_render_id: None,
            samples: Vec::with_capacity(capacity),
            dispatch_delays: Vec::with_capacity(capacity),
            render_ids: Vec::with_capacity(capacity),
            drawable_serials: Vec::with_capacity(capacity),
            visibility_counts: [0; 2],
            visibility_changes: Vec::new(),
            scroll_cycle: None,
            scroll_stage: 0,
            counts: Vec::with_capacity(capacity),
            timings: Vec::with_capacity(capacity),
            stop: None,
            presentation_drain: None,
            drain_timer_only: std::env::var("TEKES_PERF_DRAIN_TIMER_ONLY").as_deref() == Ok("1"),
            prewarm_ms: 0.0,
            started: Instant::now(),
            motion_step: None,
            scene3d_start: Value::Null,
            scene3d_pose_bytes: 0,
            scene3d_deformed_meshes: 0,
            animation_target: None,
            path,
            window_info: Value::Null,
            scale: 2.0,
            view_h: SCREEN_H,
        }
    }

    pub fn set_reduced_motion(&mut self, reduced: bool) {
        self.gallery.set_reduced_motion(reduced);
    }
    pub fn set_system_theme(&mut self, theme: Theme) {
        self.gallery.set_system_theme(theme);
    }
    pub fn is_active(&self) -> bool {
        self.phase != Phase::Done
    }

    /// 窗口和渲染器就绪后调用：开计时、预热 atlas、打开动画，并装一个看门狗，
    /// 防止窗口被遮挡等原因导致显示链接不回调时无限等下去。
    /// Window placement may resize after Created. The benchmark must use the
    /// same live content viewport as normal Gallery rendering and hit testing.
    pub fn resize(&mut self, width: f32, height: f32) {
        self.view_h = height;
        self.gallery.set_view_size(width, height);
    }

    pub fn begin(&mut self, window: &mut Window, renderer: &mut Renderer) {
        window.set_presentation_timing(true);
        self.scale = window.scale() as f32;
        self.view_h = window.size().height as f32;
        if self.measure_gpu {
            renderer.set_frame_timing(true);
        } else {
            renderer.set_frame_timing_cpu_only();
        }
        let t = Instant::now();
        self.prewarm(renderer);
        if let Some(page) = match self.cfg.content {
            ContentKind::Editor => Some(Page::TextEditing),
            ContentKind::ScrollArea => Some(Page::ScrollArea),
            ContentKind::Icons => Some(Page::Icons),
            ContentKind::Motion => Some(Page::Motion),
            _ => None,
        } {
            if std::env::var("TEKES_PERF_WORKBENCH").as_deref() == Ok("1") {
                let route = match self.cfg.content {
                    ContentKind::Editor => "lab-internal-text",
                    ContentKind::ScrollArea => "component-scroll-area",
                    ContentKind::Icons => "component-icon",
                    ContentKind::Motion => "foundation-motion",
                    _ => unreachable!(),
                };
                assert!(
                    self.gallery.open_route(route),
                    "benchmark must exercise a real registered route"
                );
            } else {
                self.gallery.apply(Command::Page(page));
            }
            self.gallery
                .set_view_size(window.size().width as f32, window.size().height as f32);
            self.gallery.prepare(renderer, self.scale);
            self.gallery.build_scene(&mut Scene::new());
        }
        if self.cfg.content == ContentKind::Scene3d {
            assert!(self.gallery.open_route("component-scene3d/playground"));
            self.gallery.set_view_size(window.size().width as f32,window.size().height as f32);
            // Layout chooses the physical viewport; settle it before timing.
            for _ in 0..2 {
                self.gallery.prepare(renderer,self.scale);
                self.gallery.build_scene(&mut Scene::new());
            }
            self.gallery.start_scene3d_rotation(std::env::var("TEKES_PERF_SCENE3D_ANIMATION").as_deref()==Ok("1"));
        }
        self.prewarm_ms = t.elapsed().as_secs_f64() * 1e3;
        self.window_info = match window.screen_info() {
            Some(s) => json!({
                "screen": s.name,
                "builtin": s.builtin,
                "scale": s.scale,
                "max_fps": s.max_fps,
            }),
            None => Value::Null,
        };
        let on = window.set_animating(true);
        eprintln!(
            "perf：内容 {}，文档高 {:.0} pt；预热 atlas {:.0} ms；动画 {}；GPU 计时 {}",
            self.cfg.content.name(),
            self.document_height(),
            self.prewarm_ms,
            if on {
                "已开"
            } else {
                "开不起来（需要 macOS 14）"
            },
            if renderer.gpu_timing_supported() {
                "支持"
            } else {
                "不支持"
            },
        );
        let limit_ms = ((self.cfg.seconds + WARMUP_S + 30.0) * 1000.0) as u64;
        selftest::run_script(vec![(limit_ms, Action::Close)]);
        if let Ok(delay) = std::env::var("TEKES_PERF_TEST_EARLY_CLOSE_MS") {
            let delay = delay
                .parse()
                .expect("TEKES_PERF_TEST_EARLY_CLOSE_MS must be unsigned milliseconds");
            selftest::run_script(vec![(delay, Action::Close)]);
        }
        if !on {
            window.close();
        }
    }

    /// 把每一页和全部段落过一遍，让字形都进 atlas（首次光栅化的成本不进滚动测量）。
    fn prewarm(&mut self, renderer: &mut Renderer) {
        let mut scene = Scene::new();
        for page in Page::ALL {
            self.tmp.clear();
            self.screen(page.index(), Theme::default(), renderer);
        }
        self.gallery.text_mut().begin_frame();
        for p in &self.paragraphs {
            self.gallery
                .text_mut()
                .push(&mut scene, p, Point::new(0.0, 0.0), Rgba::gray(0.0));
        }
    }

    fn document_height(&self) -> f32 {
        match self.cfg.content {
            ContentKind::Editor | ContentKind::ScrollArea => {
                let (limit, view) = self.gallery.editor_scroll_metrics();
                limit + view
            }
            ContentKind::Symbols => self.view_h,
            ContentKind::Icons | ContentKind::Motion => self.gallery.page_document_height(),
            ContentKind::Scene3d => self.view_h,
            _ => self.plan.height,
        }
    }
    fn scroll_at(&self, elapsed: f64) -> f32 {
        if self.cfg.content==ContentKind::Scene3d {return 0.;}
        let range = if matches!(
            self.cfg.content,
            ContentKind::Editor | ContentKind::ScrollArea
        ) {
            self.gallery.editor_scroll_metrics().0 as f64
        } else if self.cfg.content == ContentKind::Icons {
            self.gallery.page_scroll_range() as f64
        } else {
            (self.plan.height - self.view_h).max(0.0) as f64
        };
        bounce(self.cfg.speed * elapsed, range) as f32
    }

    /// 编这一帧的可见内容。
    fn build(&mut self, ctx: &mut FrameCtx<'_>, scroll: f32) {
        if self.cfg.content == ContentKind::Scene3d {
            self.gallery.prepare(ctx.renderer,self.scale);
            self.gallery.build_scene(ctx.scene);
            return;
        }
        if let Some(symbols) = &mut self.symbols {
            symbols.build(
                &mut self.gallery,
                ctx.renderer,
                ctx.scene,
                ctx.theme,
                self.scale,
            );
            return;
        }
        if matches!(
            self.cfg.content,
            ContentKind::Editor
                | ContentKind::ScrollArea
                | ContentKind::Icons
                | ContentKind::Motion
        ) {
            if self.cfg.content == ContentKind::Editor {
                self.gallery.scroll_editor_to(scroll);
            } else if self.cfg.content == ContentKind::Icons {
                self.gallery.scroll_page_to(scroll);
                self.gallery.prepare(ctx.renderer, self.scale);
            }
            self.gallery.build_scene(ctx.scene);
            return;
        }
        let view_h = self.view_h;
        let color = ColorRole::PrimaryText.resolve(ctx.theme);
        for si in 0..self.plan.segments.len() {
            let (y, h, kind) = {
                let s = &self.plan.segments[si];
                (s.y, s.h, s.kind.clone())
            };
            if y + h < scroll || y > scroll + view_h {
                continue;
            }
            match kind {
                Kind::Screen(n) => {
                    self.tmp.clear();
                    self.screen(n, ctx.theme, ctx.renderer);
                    translate_into(ctx.scene, &self.tmp, y - scroll, view_h);
                }
                Kind::Text(range) => {
                    for k in range {
                        let pl = self.plan.placed[k];
                        let para = &self.paragraphs[pl.para];
                        let top = pl.y - scroll;
                        if top + para.height as f32 + 8.0 < 0.0 || top > view_h {
                            continue;
                        }
                        self.gallery.text_mut().push(
                            ctx.scene,
                            para,
                            Point::new(TEXT_X, top),
                            color,
                        );
                    }
                }
            }
        }
    }

    /// 编第 `n` 个整屏：按页轮流，写进 `self.tmp`。依赖 Gallery 的 `apply`、`prepare` 与 `build_scene`
    /// （`build_scene` 自己会 `begin_frame`，一帧里可见的整屏最多两个，atlas 不淘汰所以无碍）。
    fn screen(&mut self, n: usize, theme: Theme, renderer: &mut Renderer) {
        self.gallery.set_system_theme(theme);
        self.gallery
            .apply(Command::Page(Page::ALL[n % Page::ALL.len()]));
        self.gallery.prepare(renderer, self.scale);
        self.gallery.build_scene(&mut self.tmp);
    }

    /// 处理一个显示链接回调：画一帧，记录样本，推进阶段。
    pub fn frame(&mut self, f: AnimationFrame, ctx: &mut FrameCtx<'_>) {
        if self.phase == Phase::PresentationDrain {
            if !self.drain_timer_only {
                self.poll_presentation_drain(ctx);
            }
            return;
        }
        if self.phase == Phase::Cooldown || self.phase == Phase::Done {
            return;
        }
        let t0 = Instant::now();
        let first = *self.first_target.get_or_insert(f.target_timestamp);
        let elapsed = f.target_timestamp - first;
        let scroll = self.scroll_at(elapsed);
        if let Some(symbols) = &mut self.symbols {
            let dt = self.animation_target.map_or(f.refresh_interval, |last| {
                (f.target_timestamp - last).max(0.0)
            });
            self.animation_target = Some(f.target_timestamp);
            symbols.tick(elapsed, dt);
        }
        if self.cfg.content == ContentKind::Scene3d {
            let dt=self.animation_target.map_or(f.refresh_interval,|last|(f.target_timestamp-last).max(0.));
            self.animation_target=Some(f.target_timestamp);
            self.gallery.animate(dt as f32);
        }
        if self.cfg.content == ContentKind::Motion {
            let dt = self
                .animation_target
                .map_or(f.refresh_interval, |previous| {
                    (f.target_timestamp - previous).max(0.0)
                });
            self.animation_target = Some(f.target_timestamp);
            self.gallery.advance_motion(dt as f32);
            let step = (elapsed / 0.3).floor() as u64;
            if self.motion_step != Some(step) {
                self.gallery
                    .motion_retarget(if step.is_multiple_of(2) { 0.85 } else { 0.15 });
                self.motion_step = Some(step);
            }
        }

        if self.cfg.content == ContentKind::ScrollArea {
            use tekes_ui_platform_macos::{
                Modifiers, Point as Pt, ScrollEvent, ScrollPhase as P, Vec2,
            };
            let dt = self
                .animation_target
                .map_or(f.refresh_interval, |t| (f.target_timestamp - t).max(0.0));
            self.animation_target = Some(f.target_timestamp);
            self.gallery.animate(dt as f32);
            let cycle = (elapsed / 2.0).floor() as u64;
            let t = elapsed % 2.0;
            let direction = if cycle.is_multiple_of(2) { 1.0 } else { -1.0 };
            let event = |delta, phase, momentum_phase| {
                tekes_ui_platform_macos::Event::Scroll(ScrollEvent {
                    timestamp: f.target_timestamp,
                    delta: Vec2 { x: 0.0, y: delta },
                    precise: true,
                    phase,
                    momentum_phase,
                    inverted: false,
                    position: Pt { x: 600.0, y: 400.0 },
                    modifiers: Modifiers::NONE,
                })
            };
            if self.scroll_cycle != Some(cycle) {
                self.scroll_cycle = Some(cycle);
                self.scroll_stage = 0;
                let limit = self.gallery.editor_scroll_metrics().0;
                self.gallery.scroll_editor_to(if direction > 0.0 {
                    (limit - 200.0).max(0.0)
                } else {
                    200.0
                });
                self.gallery.handle_input(&event(0.0, P::Began, P::None));
            }
            if t < 0.35 {
                self.gallery.handle_input(&event(
                    -direction * self.cfg.speed * dt,
                    P::Changed,
                    P::None,
                ));
            } else if t < 1.0 {
                if self.scroll_stage == 0 {
                    self.gallery.handle_input(&event(0.0, P::Ended, P::None));
                    self.scroll_stage = 1;
                }
                self.gallery.handle_input(&event(
                    -direction * self.cfg.speed * (-8.0 * (t - 0.35)).exp() * dt,
                    P::None,
                    P::Changed,
                ));
            } else if self.scroll_stage == 1 {
                self.gallery.handle_input(&event(0.0, P::None, P::Ended));
                self.scroll_stage = 2;
            }
        }
        self.gallery.text_mut().set_scale(self.scale);
        self.gallery.text_mut().begin_frame();
        ctx.scene.clear();
        self.build(ctx, scroll);
        let t_built = Instant::now();
        let clear = ColorRole::DocumentCanvas.resolve(ctx.theme);
        if let Err(e) = ctx
            .renderer
            .render(ctx.scene, self.gallery.atlas_mut(), clear)
        {
            eprintln!("渲染失败：{e}");
        }
        let t_end = Instant::now();

        let id = ctx.renderer.last_timed_frame();
        let rendered = id != self.prev_render_id;
        self.prev_render_id = id;

        if elapsed < WARMUP_S {
            return;
        }
        if self.phase == Phase::WarmUp {
            self.phase = Phase::Measure;
            if self.cfg.content==ContentKind::Scene3d {self.scene3d_start=self.gallery.scene3d_diagnostics();}
            self.measure_start_target = Some(f.target_timestamp);
            self.measure_start_cpu = process_cpu_seconds();
            self.measure_start = Some(Instant::now());
            self.cpu_rate_last = Some((Instant::now(), process_cpu_seconds()));
        }
        if let Some((at, cpu)) = self.cpu_rate_last
            && at.elapsed().as_secs_f64() >= 0.25
        {
            let now = Instant::now();
            let current = process_cpu_seconds();
            self.cpu_rates.push([
                self.measure_start.unwrap().elapsed().as_secs_f64(),
                (current - cpu) / now.duration_since(at).as_secs_f64() * 100.0,
            ]);
            self.cpu_rate_last = Some((now, current));
        }
        if self.cfg.content==ContentKind::Scene3d {
            let (bytes,meshes)=self.gallery.scene3d_frame_work();
            self.scene3d_pose_bytes+=bytes; self.scene3d_deformed_meshes+=meshes as u64;
        }
        let c = ctx.renderer.last_frame_counts();
        self.counts
            .push([c.0 as u32, c.1 as u32, c.2 as u32, c.3 as u32]);
        self.render_ids.push(if rendered { id } else { None });
        self.drawable_serials.push(if rendered {
            ctx.window.drawable_serial()
        } else {
            0
        });
        let occluded = ctx.window.is_occluded();
        let key = ctx.window.is_key_window();
        self.visibility_counts[0] += usize::from(occluded);
        self.visibility_counts[1] += usize::from(!key);
        if self
            .visibility_changes
            .last()
            .is_none_or(|v| (v.1, v.2) != (occluded, key))
        {
            self.visibility_changes
                .push((f.target_timestamp, occluded, key));
        }
        self.dispatch_delays.push(f.queue_delay() * 1000.0);
        self.samples.push(FrameSample {
            target_timestamp: f.target_timestamp,
            callback_time: f.callback_time,
            refresh_interval: f.refresh_interval,
            cpu_ms: (t_end - t0).as_secs_f64() * 1e3,
            build_ms: (t_built - t0).as_secs_f64() * 1e3,
            budget_ms: f.budget() * 1e3,
            rendered,
            ..FrameSample::default()
        });
        let start = self.measure_start_target.unwrap_or(f.target_timestamp);
        if f.target_timestamp - start >= self.cfg.seconds {
            self.finish_measure();
        }
    }

    fn finish_measure(&mut self) {
        if let Some(symbols) = &mut self.symbols {
            symbols.stop();
        }
        self.measure_end_cpu = process_cpu_seconds();
        self.measure_end = Some(Instant::now());
        self.presentation_drain = Some(PresentationDrain::new(
            self.drawable_serials
                .iter()
                .zip(&self.samples)
                .filter(|(_, sample)| sample.rendered)
                .map(|(serial, _)| *serial),
        ));
        self.phase = Phase::PresentationDrain;
        // Metal may stop issuing updates when no more supplied drawables are
        // consumed. The timeout must therefore have an independent wake source.
        selftest::run_script(vec![(
            (PRESENTATION_DRAIN_TIMEOUT_S * 1000.0) as u64,
            Action::WakeRedraw,
        )]);
        eprintln!(
            "perf：测量结束（{} 帧），停止提交新帧，等待在途呈现收据……",
            self.samples.len()
        );
    }

    pub fn poll_presentation_drain(&mut self, ctx: &mut FrameCtx<'_>) {
        if self.phase != Phase::PresentationDrain {
            return;
        }
        let drain = self
            .presentation_drain
            .as_mut()
            .expect("drain phase has state");
        if drain.poll(
            &ctx.window.presented_frames(),
            drain.started.elapsed().as_secs_f64(),
        ) {
            self.begin_cooldown(ctx);
        }
    }

    fn begin_cooldown(&mut self, ctx: &mut FrameCtx<'_>) {
        ctx.window.set_animating(false);
        // 等 GPU 做完并收齐所有结果。
        ctx.renderer.drain_frame_timings(&mut self.timings, true);
        self.stop = Some(Stop {
            callbacks: ctx.window.display_link_callbacks(),
            drawable_counts: ctx.window.drawable_acquisition_counts(),
            cpu: process_cpu_seconds(),
            at: Instant::now(),
        });
        self.phase = Phase::Cooldown;
        selftest::run_script(vec![(IDLE_WAIT_MS, Action::Close)]);
        eprintln!(
            "perf：测量结束（{} 帧），动画已关，等 {} 秒确认显示链接静默……",
            self.samples.len(),
            IDLE_WAIT_MS / 1000
        );
    }

    /// 窗口收到关闭请求（空闲确认到时、看门狗、用户手动关）。写报告并关窗。
    pub fn on_close_requested(&mut self, window: &mut Window, renderer: &mut Renderer) {
        let idle = self.stop.as_ref().map(|s| {
            json!({
                "wait_s": s.at.elapsed().as_secs_f64(),
                "display_link_callbacks": window.display_link_callbacks().saturating_sub(s.callbacks),
                "provided_drawable_consumptions": window.drawable_acquisition_counts().0.saturating_sub(s.drawable_counts.0),
                "ordinary_drawable_acquisitions": window.drawable_acquisition_counts().1.saturating_sub(s.drawable_counts.1),
                "cpu_ms": (process_cpu_seconds() - s.cpu) * 1e3,
                "cpu_percent": (process_cpu_seconds() - s.cpu) / s.at.elapsed().as_secs_f64() * 100.0,
            })
        });
        let completed = measurement_complete(self.phase, idle.as_ref());
        window.set_animating(false);
        renderer.drain_frame_timings(&mut self.timings, true);
        let atlas_stats = self.gallery.atlas_mut().stats();
        let report = self.report(completed, idle.clone(), window, renderer, atlas_stats);
        match serde_json::to_string_pretty(&report)
            .map_err(|e| e.to_string())
            .and_then(|s| std::fs::write(&self.path, s).map_err(|e| e.to_string()))
        {
            Ok(()) => eprintln!("perf：报告写到 {}", self.path.display()),
            Err(e) => eprintln!("perf：写报告失败：{e}"),
        }
        eprintln!("{}", summary(&report));
        let idle_ok = idle
            .as_ref()
            .is_some_and(|i| i["display_link_callbacks"] == 0);
        EXIT_CODE.store(if completed && idle_ok { 0 } else { 2 }, Ordering::Relaxed);
        self.phase = Phase::Done;
        window.close();
    }

    /// 把渲染器的计时并进样本。
    fn joined_samples(&self) -> Vec<FrameSample> {
        let by_frame: std::collections::HashMap<u64, &FrameTiming> =
            self.timings.iter().map(|t| (t.frame, t)).collect();
        self.samples
            .iter()
            .zip(&self.render_ids)
            .map(|(s, id)| {
                let mut s = *s;
                if let Some(t) = id.and_then(|id| by_frame.get(&id)) {
                    s.acquire_ms = t.acquire.as_secs_f64() * 1e3;
                    s.encode_ms = t.encode.as_secs_f64() * 1e3;
                    s.submit_ms = t.submit.as_secs_f64() * 1e3;
                    s.gpu_ms = t.gpu.map(|d| d.as_secs_f64() * 1e3);
                }
                s
            })
            .collect()
    }

    fn report(
        &self,
        completed: bool,
        idle: Option<Value>,
        window: &Window,
        renderer: &Renderer,
        st: AtlasStats,
    ) -> Value {
        let samples = self.joined_samples();
        let stats = FrameStats::from_samples(&samples);
        let wall = match (self.measure_start, self.measure_end) {
            (Some(a), Some(b)) => (b - a).as_secs_f64(),
            _ => 0.0,
        };
        let cpu_busy = if wall > 0.0 {
            (self.measure_end_cpu - self.measure_start_cpu) / wall * 100.0
        } else {
            0.0
        };
        let med = |f: fn(&[u32; 4]) -> u32| -> u32 {
            let mut v: Vec<u32> = self.counts.iter().map(f).collect();
            v.sort_unstable();
            v.get(v.len() / 2).copied().unwrap_or(0)
        };
        let dist = |d: &Distribution| {
            json!({
                "count": d.count, "min": d.min, "mean": d.mean,
                "p50": d.p50, "p95": d.p95, "p99": d.p99, "max": d.max,
            })
        };
        let receipts: std::collections::HashMap<_, _> = window
            .presented_frames()
            .into_iter()
            .map(|r| (r.serial, r))
            .collect();
        let mut unique_serials = std::collections::HashSet::new();
        let duplicate_serials = self
            .drawable_serials
            .iter()
            .zip(&samples)
            .filter(|(serial, sample)| {
                sample.rendered && **serial != 0 && !unique_serials.insert(**serial)
            })
            .count();
        let actual = presentation_rows(&self.drawable_serials, &samples, &receipts);
        let positive: Vec<_> = actual.iter().filter(|frame| frame.1 > 0.0).collect();
        let intervals: Vec<_> = positive
            .windows(2)
            .map(|p| (p[1].1 - p[0].1) * 1000.0)
            .collect();
        let late: Vec<_> = actual
            .iter()
            .filter(|r| r.1 > 0.0)
            .map(|r| (r.1 - r.0) * 1000.0)
            .collect();
        let presentation = json!({"method":"public CAMetalLayer subclass nextDrawable + MTLDrawable.addPresentedHandler; optional benchmark-only receipts",
            "receipts":actual.len(),"missing_receipts":samples.iter().filter(|s|s.rendered).count().saturating_sub(actual.len()),
            "skipped_render_callbacks": samples.iter().filter(|s|!s.rendered).count(),
            "duplicate_drawable_serials": duplicate_serials,"not_presented":actual.iter().filter(|r|r.1<=0.0).count(),
            "interval_ms":dist(&Distribution::from_samples(&intervals)),"target_lateness_ms":dist(&Distribution::from_samples(&late)),
            "gaps_over_1_5_refresh":intervals.iter().filter(|&&v|v>stats.refresh_interval_ms*1.5).count(),
            "columns":["target_s","presented_s","acquired_s"],"samples":actual.iter().map(|r|[r.0,r.1,r.2]).collect::<Vec<_>>()});
        let rows: Vec<Value> = samples
            .iter()
            .zip(&self.counts)
            .enumerate()
            .map(|(index, (s, c))| {
                json!([
                    s.target_timestamp,
                    s.callback_time,
                    s.refresh_interval * 1e3,
                    s.budget_ms,
                    s.cpu_ms,
                    s.build_ms,
                    s.acquire_ms,
                    s.encode_ms,
                    s.submit_ms,
                    s.gpu_ms,
                    s.rendered,
                    c[0],
                    c[1],
                    c[2],
                    c[3],
                    self.dispatch_delays[index],
                ])
            })
            .collect();
        let scene3d_workload = (self.cfg.content==ContentKind::Scene3d).then(||json!({
                "start":self.scene3d_start,"end":self.gallery.scene3d_diagnostics(),
                "measurement_pose_upload_bytes":self.scene3d_pose_bytes,
                "measurement_deformed_meshes":self.scene3d_deformed_meshes,
                "pixel_readback_per_frame":false,
                "gpu_timing_scope":"existing UI pass counters only; 3D compute/render are included in process CPU and actual presentation, not these GPU timestamps",
            }));
        let mut report = json!({
            "schema": "tekesui-perf/0",
            "kind": "scroll",
            "completed": completed,
            "tekesui": {
                "version": env!("CARGO_PKG_VERSION"),
                "commit": std::env::var("TEKES_PERF_COMMIT").ok(),
            },
            "machine": machine_info(),
            "display": self.window_info,
            "display_driver": if std::env::var("TEKES_METAL_DISPLAY_LINK").as_deref() == Ok("1") { "CAMetalDisplayLink" } else { "NSView CADisplayLink" },
            "metal_reentrant_updates_queued": window.metal_reentrant_queued(),
            "drawable_acquisitions_including_warmup": {"provided":window.drawable_acquisition_counts().0,"ordinary":window.drawable_acquisition_counts().1},
            "visibility": {"occluded_callbacks": self.visibility_counts[0], "non_key_callbacks": self.visibility_counts[1], "changes_target_s_occluded_key": self.visibility_changes},
            "render_queue": {
                "present_mode": "Fifo",
                "requested_maximum_frame_latency": match std::env::var("TEKES_PERF_FRAME_LATENCY").as_deref() { Ok("1") => 1, _ => 2 },
                "configured_metal_drawables_before_display_link": match std::env::var("TEKES_PERF_FRAME_LATENCY").as_deref() { Ok("1") => 2, _ => 3 },
                "metal_display_link_requested_frame_latency": if std::env::var("TEKES_METAL_DISPLAY_LINK").as_deref() == Ok("1") { Some(1.0) } else { None },
                "metal_display_link_pool_size_measured": false,
            },
            "config": {
                "seconds": self.cfg.seconds,
                "warmup_s": WARMUP_S,
                "speed_pt_s": self.cfg.speed,
                "content": self.cfg.content.name(),
                "gallery_route": self.gallery.page().slug(),
                "scroll_area_workload": "2s alternating top/bottom cycles: 0.35s finger, 0.65s OS-style momentum, 1s return/settle; routed through actual Gallery input",
                "view_pt": [window.size().width, window.size().height],
                "drawable_px": [window.drawable_size().0, window.drawable_size().1],
                "scale": window.scale(),
                "present_mode": "Fifo",
                "max_drawable_count": match std::env::var("TEKES_PERF_FRAME_LATENCY").as_deref() { Ok("1") => 2, _ => 3 },
                "max_drawable_count_scope": "wgpu configuration before optional CAMetalDisplayLink; not a measured Metal display-link pool size",
                "document_height_pt": self.document_height(),
                "scroll_range_pt": if matches!(self.cfg.content,ContentKind::Editor|ContentKind::ScrollArea) {self.gallery.editor_scroll_metrics().0}else if self.cfg.content==ContentKind::Icons {self.gallery.page_scroll_range()}else if matches!(self.cfg.content,ContentKind::Motion|ContentKind::Symbols|ContentKind::Scene3d) {0.0}else{(self.plan.height-self.view_h).max(0.0)},
                "segments": if matches!(self.cfg.content,ContentKind::Editor|ContentKind::ScrollArea|ContentKind::Icons|ContentKind::Motion|ContentKind::Symbols|ContentKind::Scene3d) {Value::Null}else{json!(self.plan.segments.len())},
            },
            "gpu_timing": {
                "supported": renderer.gpu_timing_supported(),
                "enabled": self.measure_gpu,
                "method": "wgpu TIMESTAMP_QUERY：render pass 开始/结束时间戳，Metal stage boundary 计数器采样",
                "covers": "只有我们的 render pass；不含合成器和 drawable 等待",
            },
            "drop_detection": {
                "method": "显示链接 target_timestamp 间隔 > 1.5 x 刷新间隔",
                "presented_time": "见 presentation：owned CAMetalLayer 的公开 addPresentedHandler 收据，另行统计实际呈现间隔",
            },
            "scene_per_frame_median": {
                "quads": med(|c| c[0]), "glyphs": med(|c| c[1]),
                "images": med(|c| c[2]), "draws": med(|c| c[3]),
            },
            "atlas": {
                "pages": st.pages, "bytes": st.bytes, "glyphs": st.glyphs,
                "evictions": st.evictions, "uploads": st.uploads, "upload_bytes": st.upload_bytes,
                "hits": st.hits, "misses": st.misses, "dropped": st.dropped,
                "prewarm_ms": self.prewarm_ms,
            },
            "presentation": presentation,
            "presentation_drain": self.presentation_drain.as_ref().map(|drain| json!({
                "expected_unique_serials": drain.expected.len(),
                "received": drain.received,
                "unpresented": drain.unpresented,
                "timed_out": drain.timed_out,
                "finished": drain.wait_ms.is_some(),
                "wait_ms": drain.wait_ms,
                "timeout_ms": PRESENTATION_DRAIN_TIMEOUT_S * 1000.0,
                "timer_only_diagnostic": self.drain_timer_only,
            })),
            "symbols_workload": self.symbols.as_ref().map(|s|s.report()),
            "process": {
                "cpu_percent_while_scrolling": cpu_busy,
                "measure_wall_s": wall,
                "cpu_rate_unit": "100% = one logical CPU core; getrusage user + system, sampled every 250ms",
                "cpu_rate_peak_250ms_percent": self.cpu_rates.iter().map(|s|s[1]).fold(0.0,f64::max),
                "cpu_rate_samples_elapsed_s_percent": self.cpu_rates,
            },
            "stats": {
                "frames": stats.frames,
                "rendered_frames": stats.rendered_frames,
                "refresh_interval_ms": stats.refresh_interval_ms,
                "cpu_ms": dist(&stats.cpu),
                "dispatch_delay_ms": dist(&Distribution::from_samples(&self.dispatch_delays)),
                "cpu_active_ms": dist(&stats.cpu_active),
                "build_ms": dist(&stats.build),
                "acquire_ms": dist(&stats.acquire),
                "encode_ms": dist(&stats.encode),
                "submit_ms": dist(&stats.submit),
                "gpu_ms": stats.gpu.as_ref().map(&dist),
                "budget_ms": dist(&stats.budget),
                "target_interval_ms": dist(&stats.target_interval),
                "callback_interval_ms": dist(&stats.callback_interval),
                "drops": {
                    "callbacks": stats.drops.callbacks,
                    "late_frames": stats.drops.late_frames,
                    "dropped_refreshes": stats.drops.dropped_refreshes,
                    "longest_drop_run": stats.drops.longest_drop_run,
                    "longest_late_streak": stats.drops.longest_late_streak,
                },
            },
            "idle_after": idle,
            "samples": {
                "columns": [
                    "target_timestamp_s", "callback_time_s", "refresh_interval_ms", "budget_ms",
                    "cpu_ms", "build_ms", "acquire_ms", "encode_ms", "submit_ms", "gpu_ms",
                    "rendered", "quads", "glyphs", "images", "draws", "dispatch_delay_ms",
                ],
                "rows": rows,
            },
            "total_run_s": self.started.elapsed().as_secs_f64(),
        });
        report["scene3d_workload"] = scene3d_workload.unwrap_or(Value::Null);
        report
    }
}

// A skipped render has no acquisition. Never duplicate its previous drawable's
// receipt, even if an older harness accidentally supplied a stale serial.
fn presentation_rows(
    serials: &[u64],
    samples: &[FrameSample],
    receipts: &std::collections::HashMap<u64, tekes_ui_platform_macos::PresentedFrame>,
) -> Vec<(f64, f64, f64)> {
    let mut seen = std::collections::HashSet::new();
    serials
        .iter()
        .zip(samples)
        .filter_map(|(&serial, sample)| {
            if !sample.rendered || serial == 0 || !seen.insert(serial) {
                return None;
            }
            receipts
                .get(&serial)
                .map(|r| (sample.target_timestamp, r.presented_at, r.acquired_at))
        })
        .collect()
}

/// 报告的一屏摘要（打印到 stderr）。
fn summary(r: &Value) -> String {
    let s = &r["stats"];
    let line = |name: &str, key: &str| -> String {
        let d = &s[key];
        if d.is_null() {
            return format!("  {name:<10} 无\n");
        }
        format!(
            "  {name:<10} p50 {:>6.3}  p95 {:>6.3}  p99 {:>6.3}  max {:>6.3}\n",
            d["p50"].as_f64().unwrap_or(0.0),
            d["p95"].as_f64().unwrap_or(0.0),
            d["p99"].as_f64().unwrap_or(0.0),
            d["max"].as_f64().unwrap_or(0.0)
        )
    };
    let mut out = format!(
        "perf 摘要（ms）：完成={} 帧={} 刷新间隔 {:.3} ms\n",
        r["completed"],
        s["frames"],
        s["refresh_interval_ms"].as_f64().unwrap_or(0.0)
    );
    out += &line("CPU(含等)", "cpu_ms");
    out += &line("CPU(干活)", "cpu_active_ms");
    out += &line("建场景", "build_ms");
    out += &line("取drawable", "acquire_ms");
    out += &line("编码", "encode_ms");
    out += &line("提交", "submit_ms");
    out += &line("GPU", "gpu_ms");
    out += &line("回调间隔", "target_interval_ms");
    let d = &s["drops"];
    out += &format!(
        "  掉帧：晚到 {} 次，丢刷新 {} 次，最长连续丢 {}，最长连续晚到 {}\n",
        d["late_frames"], d["dropped_refreshes"], d["longest_drop_run"], d["longest_late_streak"]
    );
    out += &format!("  空闲确认：{}\n", r["idle_after"]);
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn early_close_cannot_complete_the_five_second_idle_observation() {
        assert!(!measurement_complete(
            Phase::Cooldown,
            Some(&json!({"wait_s":4.999}))
        ));
        assert!(!measurement_complete(Phase::Cooldown, None));
        assert!(!measurement_complete(
            Phase::PresentationDrain,
            Some(&json!({"wait_s":5.0}))
        ));
        assert!(measurement_complete(
            Phase::Cooldown,
            Some(&json!({"wait_s":5.0}))
        ));
    }

    fn drain_receipt(serial: u64, presented_at: f64) -> tekes_ui_platform_macos::PresentedFrame {
        tekes_ui_platform_macos::PresentedFrame {
            serial,
            acquired_at: 1.0,
            presented_at,
        }
    }

    #[test]
    fn presentation_drain_waits_for_the_final_submitted_drawable() {
        let mut drain = PresentationDrain::new([11, 12]);
        assert!(!drain.poll(&[drain_receipt(11, 1.01)], 0.01));
        assert_eq!(drain.wait_ms, None);
        assert!(drain.poll(&[drain_receipt(11, 1.01), drain_receipt(12, 1.02)], 0.04));
        assert_eq!(drain.received, 2);
        assert_eq!(drain.unpresented, 0);
        assert!(!drain.timed_out);
    }

    #[test]
    fn presentation_drain_does_not_count_duplicates_or_unrelated_receipts() {
        let mut drain = PresentationDrain::new([11, 12]);
        assert!(!drain.poll(
            &[
                drain_receipt(11, 1.01),
                drain_receipt(11, 1.01),
                drain_receipt(99, 1.02)
            ],
            0.04
        ));
        assert_eq!(drain.received, 1);
    }

    #[test]
    fn presentation_drain_preserves_a_terminal_unpresented_receipt() {
        let mut drain = PresentationDrain::new([11, 12]);
        assert!(drain.poll(&[drain_receipt(11, 1.01), drain_receipt(12, 0.0)], 0.04));
        assert_eq!(drain.unpresented, 1);
        assert!(!drain.timed_out);
    }

    #[test]
    fn presentation_drain_timeout_is_explicit_and_keeps_missing_evidence() {
        let mut drain = PresentationDrain::new([11, 12]);
        assert!(drain.poll(&[drain_receipt(11, 1.01)], PRESENTATION_DRAIN_TIMEOUT_S));
        assert!(drain.timed_out);
        assert_eq!(drain.received, 1);
        assert_eq!(drain.expected.len(), 2);
    }

    #[test]
    fn presentation_receipts_exclude_skipped_and_duplicate_acquisitions() {
        use tekes_ui_platform_macos::PresentedFrame;
        let samples: Vec<_> = [
            (1.0, true),
            (2.0, false),
            (3.0, true),
            (4.0, true),
            (5.0, true),
        ]
        .into_iter()
        .map(|(target_timestamp, rendered)| FrameSample {
            target_timestamp,
            rendered,
            ..Default::default()
        })
        .collect();
        let receipts = [
            (
                7,
                PresentedFrame {
                    serial: 7,
                    acquired_at: 1.0,
                    presented_at: 1.01,
                },
            ),
            (
                8,
                PresentedFrame {
                    serial: 8,
                    acquired_at: 3.0,
                    presented_at: 3.01,
                },
            ),
        ]
        .into_iter()
        .collect();
        let rows = presentation_rows(&[7, 7, 8, 8, 0], &samples, &receipts);
        assert_eq!(rows, vec![(1.0, 1.01, 1.0), (3.0, 3.01, 3.0)]);
    }

    use super::*;

    #[test]
    fn perf_scroll_area_tracks_the_post_placement_window_viewport() {
        let config = PerfConfig::parse("scroll:1", None, Some("scroll-area"), None).unwrap();
        let mut perf = Perf::new(config);
        perf.gallery.apply(Command::Page(Page::ScrollArea));
        for (width, height) in [(1440.0, 960.0), (1440.0, 827.0), (720.0, 480.0)] {
            perf.resize(width, height);
            perf.gallery.build_scene(&mut Scene::new());
            let regular = tekes_ui_tokens::Spacing::Regular.value();
            let expected = height
                - crate::gallery::layout::PAGE_CONTENT.min_y()
                - tekes_ui_tokens::Spacing::Section.value()
                - tekes_ui_tokens::proposal::TextRole::UiHeading
                    .style()
                    .line_height
                - regular
                - 2.0 * tekes_ui_tokens::proposal::SCROLL_CONTENT_PADDING;
            assert!((perf.gallery.editor_scroll_metrics().1 - expected).abs() < 0.01);
            assert_eq!(perf.view_h, height);
        }
    }

    #[test]
    fn parse_spec() {
        let c = PerfConfig::parse("scroll", None, None, None).unwrap();
        assert_eq!(c.seconds, 10.0);
        assert_eq!(c.speed, 1200.0);
        assert_eq!(c.content, ContentKind::Mixed);
        assert_eq!(c.out, None);
        let c = PerfConfig::parse("scroll:3.5", Some("/tmp/x.json"), Some("text"), Some("600"))
            .unwrap();
        assert_eq!(c.seconds, 3.5);
        assert_eq!(c.speed, 600.0);
        assert_eq!(c.content, ContentKind::Text);
        assert_eq!(c.out, Some(PathBuf::from("/tmp/x.json")));
        assert_eq!(PerfConfig::parse("scroll:60",None,Some("scene3d"),None).unwrap().content,ContentKind::Scene3d);
        assert!(PerfConfig::parse("zoom", None, None, None).is_err());
        assert!(PerfConfig::parse("scroll:abc", None, None, None).is_err());
        assert!(PerfConfig::parse("scroll:0", None, None, None).is_err());
        assert!(PerfConfig::parse("scroll:-1", None, None, None).is_err());
        assert!(PerfConfig::parse("scroll", None, Some("wat"), None).is_err());
        assert!(PerfConfig::parse("scroll", None, None, Some("0")).is_err());
    }

    #[test]
    fn bounce_is_a_triangle_wave() {
        assert_eq!(bounce(0.0, 100.0), 0.0);
        assert_eq!(bounce(60.0, 100.0), 60.0);
        assert_eq!(bounce(100.0, 100.0), 100.0);
        assert_eq!(bounce(140.0, 100.0), 60.0);
        assert_eq!(bounce(200.0, 100.0), 0.0);
        assert_eq!(bounce(260.0, 100.0), 60.0);
        assert_eq!(bounce(5.0, 0.0), 0.0);
        // 速度恒定：相邻采样的位移绝对值相等（除折返点）。
        let step = 12.0;
        let mut prev = bounce(0.0, 1000.0);
        for i in 1..300 {
            let p = bounce(i as f64 * step, 1000.0);
            let d = (p - prev).abs();
            assert!(d <= step + 1e-9);
            prev = p;
        }
    }

    #[test]
    fn plan_quads_is_screens_only() {
        let plan = plan_document(ContentKind::Quads, &[17.0]);
        assert_eq!(plan.segments.len(), SEGMENTS);
        assert_eq!(plan.height, SEGMENTS as f32 * SCREEN_H);
        assert!(plan.placed.is_empty());
        for (i, s) in plan.segments.iter().enumerate() {
            assert_eq!(s.kind, Kind::Screen(i));
            assert_eq!(s.y, i as f32 * SCREEN_H);
        }
    }

    #[test]
    fn plan_text_fills_segments_to_a_screen_and_cycles() {
        let heights = [17.0, 17.0, 250.0];
        let plan = plan_document(ContentKind::Text, &heights);
        assert_eq!(plan.segments.len(), SEGMENTS);
        let mut y = 0.0;
        for s in &plan.segments {
            assert_eq!(s.y, y);
            assert!(s.h >= SCREEN_H, "每段至少一屏高");
            let Kind::Text(r) = &s.kind else {
                panic!("应是文字段")
            };
            assert!(!r.is_empty());
            y += s.h;
        }
        assert_eq!(plan.height, y);
        // 段落按 0,1,2,0,1,2… 循环取用，y 单调递增、间隔等于高度加空隙。
        for (k, w) in plan.placed.windows(2).enumerate() {
            assert_eq!(plan.placed[k].para, k % 3);
            assert!((w[1].y - w[0].y - (heights[w[0].para] + PARAGRAPH_GAP)).abs() < 1e-3);
        }
    }

    #[test]
    fn plan_mixed_alternates_and_without_text_degrades_to_screens() {
        let plan = plan_document(ContentKind::Mixed, &[17.0, 40.0]);
        for (i, s) in plan.segments.iter().enumerate() {
            match (&s.kind, i % 2) {
                (Kind::Screen(n), 0) => assert_eq!(*n, i / 2),
                (Kind::Text(_), 1) => {}
                other => panic!("第 {i} 段类型不对：{other:?}"),
            }
        }
        // 连续：下一段从上一段末尾开始。
        for w in plan.segments.windows(2) {
            assert!((w[0].y + w[0].h - w[1].y).abs() < 1e-3);
        }
        let plan = plan_document(ContentKind::Mixed, &[]);
        assert!(
            plan.segments
                .iter()
                .all(|s| matches!(s.kind, Kind::Screen(_)))
        );
    }

    #[test]
    fn scene_translation_preserves_visible_shadow_tails_and_offsets() {
        use tekes_ui_scene::{Rect, ShadowQuad};
        let shadow = ShadowQuad::new(Rect::new(20., -30., 40., 10.), Rgba::new(0., 0., 0., 0.2))
            .with_blur_radius(16.)
            .with_offset(Point::new(2., 3.));
        let mut source = Scene::new();
        source.push_shadow(shadow);
        let mut visible = Scene::new();
        translate_into(&mut visible, &source, 5., 100.);
        assert_eq!(visible.len(), 1, "visible blur tail must survive culling");
        let Primitive::Shadow(moved) = visible.primitives()[0] else {
            panic!("shadow lost")
        };
        assert_eq!(moved.rect, shadow.rect.offset(0., 5.));
        assert_eq!(moved.offset, shadow.offset);
        let mut invisible = Scene::new();
        translate_into(&mut invisible, &source, -100., 100.);
        assert!(invisible.is_empty());
    }

    #[test]
    fn translate_culls_offscreen_primitives() {
        let mut src = Scene::new();
        let black = Rgba::gray(0.0);
        src.push_quad(Quad::new(
            tekes_ui_scene::Rect::new(0.0, 0.0, 10.0, 10.0),
            black,
        ));
        src.push_quad(Quad::new(
            tekes_ui_scene::Rect::new(0.0, 700.0, 10.0, 10.0),
            black,
        ));
        src.push_quad(Quad::new(
            tekes_ui_scene::Rect::new(0.0, 790.0, 10.0, 10.0),
            black,
        ));
        let mut dst = Scene::new();
        // 向上移 500：第一个在 -500 以外丢掉，第二个落在 200，第三个落在 290。
        translate_into(&mut dst, &src, -500.0, 800.0);
        assert_eq!(dst.len(), 2);
        let ys: Vec<f32> = dst
            .primitives()
            .iter()
            .map(|p| match p {
                Primitive::Quad(q) => q.rect.min_y(),
                _ => unreachable!(),
            })
            .collect();
        assert_eq!(ys, vec![200.0, 290.0]);
        // 向下移 790：只有第一个落在 790，其余超出。
        let mut dst = Scene::new();
        translate_into(&mut dst, &src, 790.0, 800.0);
        assert_eq!(dst.len(), 1);
    }
}
