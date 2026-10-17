//! 上屏路径的测试：在一个独立（没有放进窗口）的 `CAMetalLayer` 上建 surface，
//! 验证渲染器对 layer 做了什么、resize 之后哪些属性还在。
//!
//! 像素内容由 `offscreen.rs` 验证（同一套管线和着色器）；drawable 的内容这里读不到，
//! 因为 `framebufferOnly = true`。
#![cfg(target_os = "macos")]

use objc2::rc::Retained;
use objc2_core_foundation::{CGPoint, CGRect, CGSize};
use objc2_core_graphics::CGColorSpace;
use objc2_quartz_core::CAMetalLayer;
use std::{ffi::c_void, ptr::NonNull};
use tekes_ui_render::{RenderError, Renderer};
use tekes_ui_scene::{GlyphAtlas, Quad, Rect, Scene};
use tekes_ui_tokens::Rgba;

fn make_layer(w: f64, h: f64, scale: f64) -> Retained<CAMetalLayer> {
    let layer = CAMetalLayer::new();
    layer.setBounds(CGRect::new(CGPoint::new(0.0, 0.0), CGSize::new(w, h)));
    layer.setContentsScale(scale);
    layer
}

fn ptr(layer: &Retained<CAMetalLayer>) -> NonNull<c_void> {
    NonNull::new(Retained::as_ptr(layer) as *mut c_void).unwrap()
}

fn colorspace_name(layer: &CAMetalLayer) -> Option<String> {
    layer
        .colorspace()
        .and_then(|cs| CGColorSpace::name(Some(&cs)))
        .map(|n| n.to_string())
}

fn scene() -> Scene {
    let mut s = Scene::new();
    s.push_quad(
        Quad::new(Rect::new(10.0, 10.0, 40.0, 30.0), Rgba::rgb8(0, 122, 255))
            .with_corner_radius(6.0),
    );
    s
}

/// 返回 None 表示没有适配器，测试跳过。
fn new_renderer(layer: &Retained<CAMetalLayer>, size: (u32, u32), scale: f32) -> Option<Renderer> {
    match unsafe { Renderer::new_for_metal_layer(ptr(layer), size, scale) } {
        Ok(r) => Some(r),
        Err(RenderError::NoAdapter) => {
            eprintln!("跳过：没有可用的 GPU 适配器");
            None
        }
        Err(e) => panic!("创建渲染器失败：{e}"),
    }
}

#[test]
fn configures_the_layer_and_keeps_srgb_colorspace_across_resizes() {
    let layer = make_layer(100.0, 80.0, 2.0);
    let Some(mut r) = new_renderer(&layer, (200, 160), 2.0) else {
        return;
    };

    // 创建后：drawable 尺寸、像素格式、不透明、drawable 数量、colorspace 都是我们要的。
    let ds = layer.drawableSize();
    assert_eq!((ds.width, ds.height), (200.0, 160.0));
    assert_eq!(
        layer.pixelFormat().0,
        80,
        "应是 Bgra8Unorm（MTLPixelFormat 80），不是 sRGB 格式"
    );
    assert!(layer.isOpaque());
    assert_eq!(
        layer.maximumDrawableCount(),
        3,
        "desired_maximum_frame_latency = 2 prevents compositor drawable starvation"
    );
    assert!(layer.displaySyncEnabled(), "Fifo 对应 displaySyncEnabled");
    assert_eq!(
        colorspace_name(&layer).as_deref(),
        Some("kCGColorSpaceSRGB")
    );

    // contentsScale、bounds 是平台层的，渲染器不改。
    assert_eq!(layer.contentsScale(), 2.0);
    assert_eq!(layer.bounds().size.width, 100.0);

    let s = scene();
    for _ in 0..3 {
        r.render(&s, &mut GlyphAtlas::default(), Rgba::gray(1.0))
            .unwrap();
    }

    // resize：drawableSize 跟着变；wgpu 每次配置都会把 colorspace 重置为 nil，
    // 渲染器要在其后重新设回 sRGB。
    layer.setBounds(CGRect::new(
        CGPoint::new(0.0, 0.0),
        CGSize::new(150.0, 100.0),
    ));
    r.resize((300, 200), 2.0);
    let ds = layer.drawableSize();
    assert_eq!((ds.width, ds.height), (300.0, 200.0));
    assert_eq!(
        colorspace_name(&layer).as_deref(),
        Some("kCGColorSpaceSRGB")
    );
    r.render(&s, &mut GlyphAtlas::default(), Rgba::gray(1.0))
        .unwrap();

    // scale 变化（换到 1x 屏幕）。
    layer.setContentsScale(1.0);
    r.resize((150, 100), 1.0);
    let ds = layer.drawableSize();
    assert_eq!((ds.width, ds.height), (150.0, 100.0));
    assert_eq!(
        colorspace_name(&layer).as_deref(),
        Some("kCGColorSpaceSRGB")
    );
    r.render(&s, &mut GlyphAtlas::default(), Rgba::gray(1.0))
        .unwrap();
}

#[test]
fn zero_size_pauses_rendering_and_recovers() {
    let layer = make_layer(100.0, 80.0, 2.0);
    let Some(mut r) = new_renderer(&layer, (0, 0), 2.0) else {
        return;
    };
    let s = scene();
    // 尺寸为 0：空操作，不报错。
    r.render(&s, &mut GlyphAtlas::default(), Rgba::gray(1.0))
        .unwrap();
    r.resize((200, 160), 2.0);
    r.render(&s, &mut GlyphAtlas::default(), Rgba::gray(1.0))
        .unwrap();
    assert_eq!(
        colorspace_name(&layer).as_deref(),
        Some("kCGColorSpaceSRGB")
    );
    // 缩到 0 再恢复。
    r.resize((0, 0), 2.0);
    r.render(&s, &mut GlyphAtlas::default(), Rgba::gray(1.0))
        .unwrap();
    r.resize((220, 180), 2.0);
    r.render(&s, &mut GlyphAtlas::default(), Rgba::gray(1.0))
        .unwrap();
}

#[test]
fn presents_with_transaction_when_the_layer_asks_for_it() {
    let layer = make_layer(100.0, 80.0, 2.0);
    let Some(mut r) = new_renderer(&layer, (200, 160), 2.0) else {
        return;
    };
    let s = scene();
    // wgpu 每次取 drawable 时读取这个属性：为 true 时 present 走
    // commit + waitUntilScheduled + drawable.present()。这里只验证这条路径不卡死、不报错，
    // 并且渲染器没有把属性改回去。
    layer.setPresentsWithTransaction(true);
    for _ in 0..3 {
        r.render(&s, &mut GlyphAtlas::default(), Rgba::gray(1.0))
            .unwrap();
    }
    assert!(layer.presentsWithTransaction());
    layer.setPresentsWithTransaction(false);
    r.render(&s, &mut GlyphAtlas::default(), Rgba::gray(1.0))
        .unwrap();
    r.resize((240, 180), 2.0);
    assert!(!layer.presentsWithTransaction());
}

#[test]
fn rejects_invalid_scale() {
    let layer = make_layer(100.0, 80.0, 2.0);
    let res = unsafe { Renderer::new_for_metal_layer(ptr(&layer), (200, 160), 0.0) };
    assert!(matches!(res, Err(RenderError::InvalidArgument(_))));
}
