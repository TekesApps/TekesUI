//! 渲染器逐帧计时的测试（离屏路径，没有 GPU 适配器时跳过）。
//!
//! 重点是校验 GPU 时间戳的换算：wgpu 30 的 Metal 后端把 `timestamp_period` 写死成 1.0 ns/tick，
//! 这里用一个足够重的离屏帧，对照整个调用的墙钟时间，换算错一个数量级就会被抓到。

use std::time::Instant;
use tekes_ui_render::{FrameTiming, RenderError, Renderer};
use tekes_ui_scene::{GlyphAtlas, Quad, Rect, Scene};
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

/// `n` 个铺满 `w x h` 的半透明矩形：全是混合，填充率压力大。
fn heavy_scene(n: usize, w: f32, h: f32) -> Scene {
    let mut scene = Scene::new();
    for i in 0..n {
        let a = 0.01 + (i % 3) as f32 * 0.01;
        scene.push_quad(Quad::new(
            Rect::new(0.0, 0.0, w, h),
            Rgba::new(0.2, 0.4, 0.8, a),
        ));
    }
    scene
}

#[test]
fn timing_is_off_by_default_and_costs_nothing_to_drain() {
    let Some(mut r) = renderer() else { return };
    let mut atlas = GlyphAtlas::default();
    let scene = heavy_scene(2, 64.0, 64.0);
    r.render_to_rgba8(&scene, &mut atlas, (64, 64), 1.0, Rgba::gray(1.0))
        .unwrap();
    let mut out = Vec::new();
    r.drain_frame_timings(&mut out, true);
    assert!(out.is_empty());
    assert_eq!(r.last_timed_frame(), None);
    assert_eq!(r.pending_frame_timings(), 0);
}

#[test]
fn cpu_segments_are_consistent_and_frames_are_numbered() {
    let Some(mut r) = renderer() else { return };
    r.set_frame_timing(true);
    let mut atlas = GlyphAtlas::default();
    let scene = heavy_scene(10, 256.0, 256.0);
    for expect in 0..5u64 {
        r.render_to_rgba8(&scene, &mut atlas, (256, 256), 1.0, Rgba::gray(1.0))
            .unwrap();
        assert_eq!(r.last_timed_frame(), Some(expect));
    }
    let mut out = Vec::new();
    r.drain_frame_timings(&mut out, true);
    assert_eq!(out.len(), 5, "每帧恰好一条");
    for (i, t) in out.iter().enumerate() {
        assert_eq!(t.frame, i as u64, "按帧顺序");
        assert!(t.acquire.is_zero(), "离屏没有 drawable");
        assert_eq!(
            t.gpu.is_some(),
            r.gpu_timing_supported(),
            "帧 {i} 的 GPU 时间"
        );
        assert!(t.encode + t.submit <= t.total + std::time::Duration::from_micros(50));
    }
    // 再取一次什么都没有。
    let mut again = Vec::new();
    r.drain_frame_timings(&mut again, false);
    assert!(again.is_empty());
    // 关掉之后不再产生。
    r.set_frame_timing(false);
    r.render_to_rgba8(&scene, &mut atlas, (256, 256), 1.0, Rgba::gray(1.0))
        .unwrap();
    r.drain_frame_timings(&mut again, true);
    assert!(again.is_empty());
}

#[test]
fn gpu_time_agrees_with_wall_clock_on_a_heavy_frame() {
    let Some(mut r) = renderer() else { return };
    if !r.gpu_timing_supported() {
        eprintln!("跳过：设备不支持 TIMESTAMP_QUERY");
        return;
    }
    r.set_frame_timing(true);
    let mut atlas = GlyphAtlas::default();
    let (w, h) = (512u32, 512u32);
    let scene = heavy_scene(20_000, w as f32, h as f32);
    // 预热一次（管线、缓冲的首次分配不计入）。
    r.render_to_rgba8(&scene, &mut atlas, (w, h), 1.0, Rgba::gray(1.0))
        .unwrap();
    let mut out = Vec::new();
    r.drain_frame_timings(&mut out, true);
    out.clear();

    let t0 = Instant::now();
    r.render_to_rgba8(&scene, &mut atlas, (w, h), 1.0, Rgba::gray(1.0))
        .unwrap();
    let wall = t0.elapsed();
    r.drain_frame_timings(&mut out, true);
    assert_eq!(out.len(), 1);
    let t: &FrameTiming = &out[0];
    let gpu = t.gpu.expect("支持时间戳时必须有 GPU 时间");
    eprintln!("wall {wall:?}  gpu {gpu:?}  encode {:?}", t.encode);
    // 墙钟包含编码、提交、GPU 执行和读回：GPU 时间不可能比它长，
    // 也不应该短到只占一小部分（这帧约 5 Gpixel 的混合、读回只有 1 MB，GPU 是主要成本）。
    assert!(gpu <= wall, "gpu {gpu:?} > wall {wall:?}");
    assert!(
        gpu.as_secs_f64() >= 0.5 * wall.as_secs_f64(),
        "gpu {gpu:?} 相对 wall {wall:?} 太短，时间戳换算可能不对"
    );
}
