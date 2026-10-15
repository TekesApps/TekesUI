//! 渲染器内部的逐帧计时（默认关闭，关闭时每帧只多一次 `Option` 判断）。
//!
//! # CPU 部分
//!
//! 用 `Instant` 量 `Renderer::render` 里的三段：取 drawable（`get_current_texture`，
//! Metal 上就是 `CAMetalLayer.nextDrawable`，所有 drawable 都在途时会阻塞）、编码
//! （atlas 脏区上传、实例打包、render pass 编码）、提交（`queue.submit` 加 `present`）。
//!
//! # GPU 部分
//!
//! 用 wgpu 的 `TIMESTAMP_QUERY`（Metal 的 stage boundary 计数器采样）：render pass 的
//! `timestamp_writes` 在 pass 开始和结束各写一个时间戳，`resolve_query_set` 解析到缓冲，
//! 拷到可映射的回读缓冲，提交后 `map_async`；之后每帧非阻塞地 `poll`，结果到了就取。
//! 所以 GPU 时间比 CPU 时间晚一两帧才到，由 [`Renderer::drain_frame_timings`] 交出。
//!
//! 它量的是**我们这一个 render pass 在 GPU 上的执行时间**（顶点加片元加 tile 写回），
//! 不含合成器（WindowServer）、不含 drawable 等待、不含显示扫描。回读缓冲一共 8 个槽，
//! 都在途时（GPU 落后太多）这一帧没有 GPU 时间，`gpu` 为 `None`。
//!
//! 时间戳换算成纳秒用 wgpu 报告的 `timestamp_period`；wgpu 30 的 Metal 后端对非 Intel
//! 一律写死 1.0（ns/tick），所以这里的换算是否对 M 系列新芯片成立，由
//! `tests/frame_timing.rs` 里和墙钟时间对照的测试把关。
//!
//! [`Renderer::drain_frame_timings`]: crate::Renderer::drain_frame_timings

use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::Duration;

/// 回读槽的数量：同时在途的带 GPU 计时的帧数上限。
const SLOTS: usize = 8;

const FREE: u8 = 0;
const PENDING: u8 = 1;
const READY: u8 = 2;
const FAILED: u8 = 3;

/// 渲染器量到的一帧时间。
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FrameTiming {
    /// 开启计时之后第几个被画出的帧，从 0 开始。被遮挡、尺寸为 0 而跳过的帧不占编号。
    pub frame: u64,
    /// 取 drawable 的时间（`nextDrawable` 的等待）。
    pub acquire: Duration,
    /// 取到 drawable 之后到 `submit` 之前：上传 atlas、打包实例、编码 pass。
    pub encode: Duration,
    /// `queue.submit` 加 `present`。
    pub submit: Duration,
    /// 整个 `render` 调用。
    pub total: Duration,
    /// render pass 的 GPU 执行时间。`None`：设备不支持时间戳查询、槽满了、或结果读回失败。
    pub gpu: Option<Duration>,
}

struct Slot {
    resolve: wgpu::Buffer,
    readback: wgpu::Buffer,
    state: Arc<AtomicU8>,
}

struct GpuTimer {
    query_set: wgpu::QuerySet,
    slots: Vec<Slot>,
    next: usize,
    /// 每个 tick 多少纳秒（`Queue::get_timestamp_period`）。
    period_ns: f64,
}

struct Waiting {
    timing: FrameTiming,
    slot: usize,
}

/// 开启计时之后的全部状态。
pub(crate) struct Timing {
    next_frame: u64,
    gpu: Option<GpuTimer>,
    waiting: VecDeque<Waiting>,
    done: Vec<FrameTiming>,
    /// 最近一次画出的帧的编号。
    last_frame: Option<u64>,
}

/// 这一帧占用的槽。
pub(crate) struct SlotTicket(usize);

impl Timing {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, gpu_supported: bool) -> Timing {
        let gpu = gpu_supported.then(|| {
            let query_set = device.create_query_set(&wgpu::QuerySetDescriptor {
                label: Some("tekes frame timing"),
                ty: wgpu::QueryType::Timestamp,
                count: (SLOTS * 2) as u32,
            });
            let slots = (0..SLOTS)
                .map(|_| Slot {
                    resolve: device.create_buffer(&wgpu::BufferDescriptor {
                        label: Some("tekes timing resolve"),
                        size: 16,
                        usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
                        mapped_at_creation: false,
                    }),
                    readback: device.create_buffer(&wgpu::BufferDescriptor {
                        label: Some("tekes timing readback"),
                        size: 16,
                        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                        mapped_at_creation: false,
                    }),
                    state: Arc::new(AtomicU8::new(FREE)),
                })
                .collect();
            GpuTimer {
                query_set,
                slots,
                next: 0,
                period_ns: queue.get_timestamp_period() as f64,
            }
        });
        Timing {
            next_frame: 0,
            gpu,
            waiting: VecDeque::new(),
            done: Vec::new(),
            last_frame: None,
        }
    }

    pub fn last_frame(&self) -> Option<u64> {
        self.last_frame
    }

    /// 为即将编码的 pass 占一个槽并给出 `timestamp_writes`。没有空槽返回 `None`。
    pub fn begin_gpu(&mut self) -> Option<(SlotTicket, wgpu::RenderPassTimestampWrites<'_>)> {
        let g = self.gpu.as_mut()?;
        let i = g.next;
        if g.slots[i].state.load(Ordering::Acquire) != FREE {
            return None;
        }
        g.next = (i + 1) % SLOTS;
        Some((
            SlotTicket(i),
            wgpu::RenderPassTimestampWrites {
                query_set: &g.query_set,
                beginning_of_pass_write_index: Some(2 * i as u32),
                end_of_pass_write_index: Some(2 * i as u32 + 1),
            },
        ))
    }

    /// pass 结束之后：把这个槽的两个时间戳解析并拷到回读缓冲。
    pub fn resolve(&self, encoder: &mut wgpu::CommandEncoder, ticket: &SlotTicket) {
        let Some(g) = &self.gpu else { return };
        let i = ticket.0;
        let slot = &g.slots[i];
        encoder.resolve_query_set(
            &g.query_set,
            2 * i as u32..2 * i as u32 + 2,
            &slot.resolve,
            0,
        );
        encoder.copy_buffer_to_buffer(&slot.resolve, 0, &slot.readback, 0, 16);
    }

    /// `submit` 之后：发起映射，帧进入等待队列。`ticket` 为 `None` 表示这一帧没有 GPU 计时。
    pub fn finish_frame(&mut self, mut timing: FrameTiming, ticket: Option<SlotTicket>) {
        timing.frame = self.next_frame;
        self.next_frame += 1;
        self.last_frame = Some(timing.frame);
        let (Some(t), Some(g)) = (ticket, &self.gpu) else {
            self.done.push(timing);
            return;
        };
        let slot = &g.slots[t.0];
        slot.state.store(PENDING, Ordering::Release);
        let state = slot.state.clone();
        slot.readback
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |r| {
                state.store(if r.is_ok() { READY } else { FAILED }, Ordering::Release);
            });
        self.waiting.push_back(Waiting { timing, slot: t.0 });
    }

    /// 收取映射已完成的槽，转成 GPU 时间。按帧顺序，遇到还没好的就停。
    pub fn collect(&mut self) {
        let Some(g) = &self.gpu else { return };
        while let Some(front) = self.waiting.front() {
            let slot = &g.slots[front.slot];
            let gpu = match slot.state.load(Ordering::Acquire) {
                READY => {
                    let ticks = {
                        let range = slot.readback.slice(..).get_mapped_range();
                        match range {
                            Ok(bytes) => {
                                let b: &[u8] = &bytes;
                                let begin = u64::from_le_bytes(b[0..8].try_into().unwrap());
                                let end = u64::from_le_bytes(b[8..16].try_into().unwrap());
                                Some((begin, end))
                            }
                            Err(_) => None,
                        }
                    };
                    slot.readback.unmap();
                    ticks.and_then(|(begin, end)| {
                        // 时间戳不单调（计数器回绕、驱动返回 0）时丢弃这一帧的 GPU 时间。
                        (end > begin).then(|| {
                            Duration::from_nanos(((end - begin) as f64 * g.period_ns) as u64)
                        })
                    })
                }
                FAILED => None,
                _ => break,
            };
            slot.state.store(FREE, Ordering::Release);
            let w = self.waiting.pop_front().expect("front 存在");
            let mut timing = w.timing;
            timing.gpu = gpu;
            self.done.push(timing);
        }
    }

    pub fn take_done(&mut self, out: &mut Vec<FrameTiming>) {
        out.append(&mut self.done);
    }

    /// 还有多少帧在等 GPU 结果。
    pub fn pending(&self) -> usize {
        self.waiting.len()
    }
}
