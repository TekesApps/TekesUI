//! 帧时间统计：纯数据和纯函数，不依赖 wgpu 或平台层，所以可以直接单测。
//!
//! 宿主（Gallery 的性能 harness）每画一帧填一个 [`FrameSample`]，跑完后交给
//! [`FrameStats::from_samples`] 得到分位数和掉帧统计。
//!
//! # 各个数字的含义
//!
//! | 字段 | 含义 | 来源 |
//! |---|---|---|
//! | `cpu_ms` | 从平台层把 `AnimationFrame`（或 `Redraw`）交给引擎，到 `Renderer::render` 返回 | 宿主用 `Instant` 量。包含建场景、取 drawable、编码、提交、`present` 调用；不含 GPU 执行 |
//! | `cpu_active_ms` | `cpu_ms` 减去 `acquire_ms`：CPU 真正在干活的时间，不含等 drawable | 派生 |
//! | `build_ms` | 其中建场景（排版结果编进 `Scene`、滚动偏移、裁剪）的部分 | 宿主 |
//! | `acquire_ms` | 其中 `CAMetalLayer.nextDrawable` 的等待 | 渲染器量 `get_current_texture` |
//! | `encode_ms` / `submit_ms` | 取到 drawable 之后：上传 atlas、打包实例、编码 render pass；`queue.submit` 加 `present` | 渲染器 |
//! | `gpu_ms` | render pass 在 GPU 上从开始到结束的时间 | Metal 计数器时间戳（`TIMESTAMP_QUERY`），只覆盖我们这一个 pass，不含合成器 |
//! | `budget_ms` | 回调开始时离这一帧的上屏截止（`target_timestamp`）还剩多少 | 显示链接 |
//!
//! # 掉帧检测
//!
//! wgpu 30 不暴露 `CAMetalDrawable`，接不上 `addPresentedHandler`，拿不到真正的上屏时刻。
//! 所以退而用显示链接的节奏：每次回调带 `target_timestamp`（这一帧预计上屏的刷新时刻），
//! 连续两次回调的 `target_timestamp` 之差超过 [`DROP_THRESHOLD`]（1.5）个刷新间隔，
//! 就认为中间至少有一次刷新没有得到新帧。主线程来不及（CPU 超时）、`nextDrawable` 阻塞
//! （GPU 超时、drawable 没释放）都会表现为回调被跳过，所以这个检测同时覆盖 CPU 和 GPU 两种
//! 掉帧，但它看的是“我们有没有及时被叫到并交出一帧”，不是“合成器实际显示了哪一帧”。

/// 掉帧判定倍数：相邻两次回调的目标时间相差超过 `DROP_THRESHOLD * refresh_interval`。
pub const DROP_THRESHOLD: f64 = 1.5;

/// 已排序数组的 `p` 分位数（`p` 在 0..=1），最近秩法：取第 `ceil(p * n)` 个样本，
/// 结果一定是某个真实样本。空数组返回 0。
pub fn percentile_sorted(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let p = p.clamp(0.0, 1.0);
    let rank = (p * sorted.len() as f64).ceil() as usize;
    sorted[rank.clamp(1, sorted.len()) - 1]
}

/// 一组数值的分布。单位由调用方决定（这里的帧统计一律是毫秒）。
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Distribution {
    pub count: usize,
    pub min: f64,
    pub mean: f64,
    pub p50: f64,
    pub p95: f64,
    pub p99: f64,
    pub max: f64,
}

impl Distribution {
    /// 非有限值（NaN、无穷）被忽略。没有样本时全 0。
    pub fn from_samples(values: &[f64]) -> Distribution {
        let mut v: Vec<f64> = values.iter().copied().filter(|x| x.is_finite()).collect();
        if v.is_empty() {
            return Distribution::default();
        }
        v.sort_by(f64::total_cmp);
        Distribution {
            count: v.len(),
            min: v[0],
            mean: v.iter().sum::<f64>() / v.len() as f64,
            p50: percentile_sorted(&v, 0.50),
            p95: percentile_sorted(&v, 0.95),
            p99: percentile_sorted(&v, 0.99),
            max: v[v.len() - 1],
        }
    }
}

/// 掉帧检测结果。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DropReport {
    /// 参与检测的回调数。
    pub callbacks: usize,
    /// 相邻回调目标时间的间隔，秒；长度 `callbacks - 1`。
    pub gaps: Vec<f64>,
    /// 超过阈值的间隔数（“晚到的帧”）。
    pub late_frames: usize,
    /// 丢掉的刷新总数：每个超阈值的间隔贡献 `round(gap / interval) - 1`。
    pub dropped_refreshes: usize,
    /// 一个间隔里连续丢掉的刷新数的最大值，就是“连续掉帧”的长度。
    /// 门禁要求它 ≤ 1 的含义见 `docs/handoffs/F1-measurements.md`。
    pub longest_drop_run: usize,
    /// 连续晚到的帧（相邻的超阈值间隔）的最长段。比 `longest_drop_run` 更严：
    /// 隔一帧掉一帧也会让它增长，反映节奏不稳。
    pub longest_late_streak: usize,
}

/// 由回调的 `target_timestamp` 序列（秒，按回调顺序）检测掉帧。
///
/// `refresh_interval` 是刷新间隔（秒）；非正或非有限时无法判定，返回只有 `callbacks` 和 `gaps`
/// 的报告（其余为 0）。
pub fn detect_drops(target_timestamps: &[f64], refresh_interval: f64) -> DropReport {
    let gaps: Vec<f64> = target_timestamps.windows(2).map(|w| w[1] - w[0]).collect();
    let mut report = DropReport {
        callbacks: target_timestamps.len(),
        gaps,
        ..DropReport::default()
    };
    if !(refresh_interval.is_finite() && refresh_interval > 0.0) {
        return report;
    }
    let mut streak = 0usize;
    for &gap in &report.gaps {
        if gap > DROP_THRESHOLD * refresh_interval {
            let missed = ((gap / refresh_interval).round() as usize).saturating_sub(1);
            report.late_frames += 1;
            report.dropped_refreshes += missed;
            report.longest_drop_run = report.longest_drop_run.max(missed);
            streak += 1;
            report.longest_late_streak = report.longest_late_streak.max(streak);
        } else {
            streak = 0;
        }
    }
    report
}

/// 一帧的全部数字。宿主每画一帧填一个；时间戳单位是秒，其余是毫秒。
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FrameSample {
    /// 显示链接的 `target_timestamp`。
    pub target_timestamp: f64,
    /// 回调入口的实际时刻。
    pub callback_time: f64,
    /// 显示链接报告的刷新间隔，秒。
    pub refresh_interval: f64,
    pub cpu_ms: f64,
    pub build_ms: f64,
    pub acquire_ms: f64,
    pub encode_ms: f64,
    pub submit_ms: f64,
    /// `None`：这一帧没有 GPU 时间（不支持时间戳查询、被跳过的帧、结果没等到）。
    pub gpu_ms: Option<f64>,
    /// 回调开始时离上屏截止的剩余时间，毫秒。
    pub budget_ms: f64,
    /// 这一帧实际画了（没有被遮挡、超时跳过）。
    pub rendered: bool,
}

/// 一次运行的汇总。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FrameStats {
    pub frames: usize,
    pub rendered_frames: usize,
    /// 样本里刷新间隔的中位数，毫秒。
    pub refresh_interval_ms: f64,
    pub cpu: Distribution,
    /// `cpu_ms - acquire_ms`：不含等 drawable 的 CPU 时间。`nextDrawable` 在队列满时会睡到下一次刷新，
    /// 这段是被显示节奏卡住，不是我们在耗 CPU，门禁的 CPU 项以它为准，`cpu` 作为参考一并给出。
    pub cpu_active: Distribution,
    pub build: Distribution,
    pub acquire: Distribution,
    pub encode: Distribution,
    pub submit: Distribution,
    /// 没有任何样本带 GPU 时间时为 `None`。
    pub gpu: Option<Distribution>,
    pub budget: Distribution,
    /// 相邻回调目标时间的间隔（毫秒）：理想值恒等于刷新间隔。
    pub target_interval: Distribution,
    /// 相邻回调实际发生时刻的间隔（毫秒）。
    pub callback_interval: Distribution,
    pub drops: DropReport,
}

impl FrameStats {
    pub fn from_samples(samples: &[FrameSample]) -> FrameStats {
        let col = |f: fn(&FrameSample) -> f64| -> Vec<f64> { samples.iter().map(f).collect() };
        let rendered: Vec<&FrameSample> = samples.iter().filter(|s| s.rendered).collect();
        let col_r =
            |f: fn(&FrameSample) -> f64| -> Vec<f64> { rendered.iter().map(|s| f(s)).collect() };
        let intervals = Distribution::from_samples(&col(|s| s.refresh_interval * 1e3));
        let gpu_values: Vec<f64> = rendered.iter().filter_map(|s| s.gpu_ms).collect();
        let targets = col(|s| s.target_timestamp);
        let callbacks = col(|s| s.callback_time);
        let drops = detect_drops(&targets, intervals.p50 / 1e3);
        let to_ms = |v: &[f64]| -> Vec<f64> { v.windows(2).map(|w| (w[1] - w[0]) * 1e3).collect() };
        FrameStats {
            frames: samples.len(),
            rendered_frames: rendered.len(),
            refresh_interval_ms: intervals.p50,
            cpu: Distribution::from_samples(&col_r(|s| s.cpu_ms)),
            cpu_active: Distribution::from_samples(&col_r(|s| s.cpu_ms - s.acquire_ms)),
            build: Distribution::from_samples(&col_r(|s| s.build_ms)),
            acquire: Distribution::from_samples(&col_r(|s| s.acquire_ms)),
            encode: Distribution::from_samples(&col_r(|s| s.encode_ms)),
            submit: Distribution::from_samples(&col_r(|s| s.submit_ms)),
            gpu: (!gpu_values.is_empty()).then(|| Distribution::from_samples(&gpu_values)),
            budget: Distribution::from_samples(&col(|s| s.budget_ms)),
            target_interval: Distribution::from_samples(&to_ms(&targets)),
            callback_interval: Distribution::from_samples(&to_ms(&callbacks)),
            drops,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HZ120: f64 = 1.0 / 120.0;

    fn steady(n: usize) -> Vec<f64> {
        (0..n).map(|i| 100.0 + i as f64 * HZ120).collect()
    }

    #[test]
    fn percentile_is_nearest_rank() {
        let v: Vec<f64> = (1..=100).map(f64::from).collect();
        assert_eq!(percentile_sorted(&v, 0.50), 50.0);
        assert_eq!(percentile_sorted(&v, 0.95), 95.0);
        assert_eq!(percentile_sorted(&v, 0.99), 99.0);
        assert_eq!(percentile_sorted(&v, 1.0), 100.0);
        assert_eq!(percentile_sorted(&v, 0.0), 1.0);
        // 101 个样本：ceil(0.99 * 101) = 100，第 100 个。
        let v: Vec<f64> = (1..=101).map(f64::from).collect();
        assert_eq!(percentile_sorted(&v, 0.99), 100.0);
        assert_eq!(percentile_sorted(&[], 0.99), 0.0);
        assert_eq!(percentile_sorted(&[7.0], 0.99), 7.0);
    }

    #[test]
    fn distribution_basics() {
        let d = Distribution::from_samples(&[3.0, 1.0, 2.0, 4.0, f64::NAN, f64::INFINITY]);
        assert_eq!(d.count, 4);
        assert_eq!(d.min, 1.0);
        assert_eq!(d.max, 4.0);
        assert_eq!(d.mean, 2.5);
        assert_eq!(d.p50, 2.0);
        assert_eq!(Distribution::from_samples(&[]), Distribution::default());
    }

    #[test]
    fn steady_cadence_has_no_drops() {
        let r = detect_drops(&steady(1200), HZ120);
        assert_eq!(r.callbacks, 1200);
        assert_eq!(r.gaps.len(), 1199);
        assert_eq!(r.late_frames, 0);
        assert_eq!(r.dropped_refreshes, 0);
        assert_eq!(r.longest_drop_run, 0);
        assert_eq!(r.longest_late_streak, 0);
    }

    #[test]
    fn jitter_below_threshold_is_not_a_drop() {
        // 间隔在 0.5 到 1.49 个刷新间隔之间抖动。
        let mut t = 0.0;
        let mut v = vec![t];
        for i in 0..200 {
            t += HZ120 * if i % 2 == 0 { 1.49 } else { 0.51 };
            v.push(t);
        }
        let r = detect_drops(&v, HZ120);
        assert_eq!(r.late_frames, 0);
        assert_eq!(r.dropped_refreshes, 0);
    }

    #[test]
    fn single_missed_refresh_counts_once() {
        let mut v = steady(10);
        // 去掉第 5 个：第 4 到第 6 个相隔两个刷新间隔。
        v.remove(5);
        let r = detect_drops(&v, HZ120);
        assert_eq!(r.late_frames, 1);
        assert_eq!(r.dropped_refreshes, 1);
        assert_eq!(r.longest_drop_run, 1);
        assert_eq!(r.longest_late_streak, 1);
    }

    #[test]
    fn consecutive_missed_refreshes_in_one_gap() {
        let mut v = steady(20);
        // 连续去掉三个：一个间隔里丢了 3 次刷新。
        v.drain(5..8);
        let r = detect_drops(&v, HZ120);
        assert_eq!(r.late_frames, 1);
        assert_eq!(r.dropped_refreshes, 3);
        assert_eq!(r.longest_drop_run, 3);
        assert_eq!(r.longest_late_streak, 1);
    }

    #[test]
    fn alternating_drops_are_isolated_but_form_a_late_streak() {
        // 间隔：1, 2, 2, 2, 1：三次各丢一个刷新，彼此被已交付的帧隔开。
        let mut t = 0.0;
        let mut v = vec![t];
        for k in [1.0, 2.0, 2.0, 2.0, 1.0] {
            t += HZ120 * k;
            v.push(t);
        }
        let r = detect_drops(&v, HZ120);
        assert_eq!(r.late_frames, 3);
        assert_eq!(r.dropped_refreshes, 3);
        assert_eq!(r.longest_drop_run, 1, "每次只丢一个，没有连续丢");
        assert_eq!(r.longest_late_streak, 3);
    }

    #[test]
    fn two_late_runs_are_separated_by_an_on_time_frame() {
        let mut t = 0.0;
        let mut v = vec![t];
        for k in [2.0, 2.0, 1.0, 2.0] {
            t += HZ120 * k;
            v.push(t);
        }
        let r = detect_drops(&v, HZ120);
        assert_eq!(r.late_frames, 3);
        assert_eq!(r.longest_late_streak, 2);
    }

    #[test]
    fn gap_exactly_at_threshold_is_not_late() {
        let v = [0.0, DROP_THRESHOLD * HZ120];
        assert_eq!(detect_drops(&v, HZ120).late_frames, 0);
        let v = [0.0, DROP_THRESHOLD * HZ120 * 1.001];
        let r = detect_drops(&v, HZ120);
        assert_eq!(r.late_frames, 1);
        assert_eq!(r.dropped_refreshes, 1);
    }

    #[test]
    fn unusable_interval_reports_gaps_only() {
        let r = detect_drops(&steady(5), 0.0);
        assert_eq!(r.gaps.len(), 4);
        assert_eq!(r.late_frames, 0);
        let r = detect_drops(&steady(5), f64::NAN);
        assert_eq!(r.late_frames, 0);
        let r = detect_drops(&[], HZ120);
        assert_eq!(r.callbacks, 0);
        assert!(r.gaps.is_empty());
    }

    fn sample(i: usize, cpu: f64, gpu: Option<f64>) -> FrameSample {
        FrameSample {
            target_timestamp: 100.0 + i as f64 * HZ120,
            callback_time: 100.0 + i as f64 * HZ120 - 0.004,
            refresh_interval: HZ120,
            cpu_ms: cpu,
            gpu_ms: gpu,
            budget_ms: 4.0,
            rendered: true,
            ..FrameSample::default()
        }
    }

    #[test]
    fn frame_stats_aggregate() {
        let samples: Vec<FrameSample> = (0..100)
            .map(|i| sample(i, 1.0 + i as f64 * 0.01, Some(0.5)))
            .collect();
        let st = FrameStats::from_samples(&samples);
        assert_eq!(st.frames, 100);
        assert_eq!(st.rendered_frames, 100);
        assert!((st.refresh_interval_ms - 1000.0 / 120.0).abs() < 1e-9);
        assert_eq!(st.cpu.count, 100);
        assert_eq!(st.cpu_active.max, st.cpu.max, "样本里 acquire 为 0");
        assert!((st.cpu.p99 - 1.98).abs() < 1e-9);
        assert!((st.cpu.max - 1.99).abs() < 1e-9);
        let gpu = st.gpu.expect("有 GPU 样本");
        assert_eq!(gpu.p99, 0.5);
        assert_eq!(st.drops.late_frames, 0);
        assert!((st.target_interval.p50 - 1000.0 / 120.0).abs() < 1e-6);
        assert!((st.budget.p50 - 4.0).abs() < 1e-12);
    }

    #[test]
    fn cpu_active_excludes_the_drawable_wait() {
        let mut s = sample(0, 8.3, Some(0.5));
        s.acquire_ms = 8.0;
        let st = FrameStats::from_samples(&[s]);
        assert!((st.cpu.max - 8.3).abs() < 1e-12);
        assert!((st.cpu_active.max - 0.3).abs() < 1e-12);
    }

    #[test]
    fn frame_stats_without_gpu_and_with_skipped_frames() {
        let mut samples: Vec<FrameSample> = (0..10).map(|i| sample(i, 2.0, None)).collect();
        samples[3].rendered = false;
        samples[3].cpu_ms = 999.0;
        let st = FrameStats::from_samples(&samples);
        assert_eq!(st.frames, 10);
        assert_eq!(st.rendered_frames, 9);
        assert!(st.gpu.is_none());
        assert_eq!(st.cpu.max, 2.0, "没画的帧不进 CPU 分布");
    }

    #[test]
    fn frame_stats_detect_a_missed_refresh() {
        let mut samples: Vec<FrameSample> = (0..30).map(|i| sample(i, 1.0, Some(0.2))).collect();
        samples.remove(10);
        let st = FrameStats::from_samples(&samples);
        assert_eq!(st.drops.dropped_refreshes, 1);
        assert_eq!(st.drops.longest_drop_run, 1);
        assert!((st.target_interval.max - 2000.0 / 120.0).abs() < 1e-6);
    }

    #[test]
    fn empty_stats_are_zero() {
        let st = FrameStats::from_samples(&[]);
        assert_eq!(st.frames, 0);
        assert_eq!(st.cpu, Distribution::default());
        assert!(st.gpu.is_none());
    }
}
