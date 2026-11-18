#![cfg(target_os = "macos")]

use tekes_ui_composition_consumer::reading_prepend::{EDITOR, ReadingFixture};
use tekes_ui_controls::composition::{
    ContentEvent as C, PointerButton, PointerEvent, PointerModifiers, PointerPhase,
};
use tekes_ui_macos_bridge::coretext_host::CoreTextHost;
use tekes_ui_render::Renderer;
use tekes_ui_scene::{Point, Rect, Scene};
use tekes_ui_tokens::{Palette, Spacing, Theme};

fn save_png(path: std::path::PathBuf, width: u32, height: u32, rgba: Vec<u8>) {
    resvg::tiny_skia::Pixmap::from_vec(
        rgba,
        resvg::tiny_skia::IntSize::from_wh(width, height).unwrap(),
    )
    .unwrap()
    .save_png(path)
    .unwrap();
}

#[test]
fn independent_prepend_fixture_keeps_real_gpu_reading_position_in_four_themes() {
    let mut renderer = Renderer::new_headless().expect("real GPU required");
    let out = std::env::var_os("TEKESUI_PREPEND_GPU_OUT").map(std::path::PathBuf::from);
    if let Some(p) = &out {
        std::fs::create_dir_all(p).unwrap();
    }
    let mut reports = Vec::new();
    for (theme_id, theme) in Theme::ALL.into_iter().enumerate() {
        for width in [360u32, 800] {
            for scale in [1u32, 2] {
                let mut fixture = ReadingFixture::new();
                let mut text = CoreTextHost::new()
                    .with_backing_scale(scale as f32)
                    .with_type_scale(if width == 360 { 1.5 } else { 1. });
                let frame = Rect::new(0., 0., width as f32, 640.);
                let mut anchor: Option<(usize, f32)> = None;
                for stage in 0..3 {
                    if stage > 0 {
                        let len = fixture.surface.input().len();
                        fixture.prepend().unwrap();
                        let delta = fixture.surface.input().len() - len;
                        if let Some((i, y)) = anchor {
                            anchor = Some((i + delta, y));
                        }
                    }
                    text.atlas.begin_frame();
                    text.begin_frame(std::time::Duration::ZERO, false);
                    let mut scene = Scene::new();
                    fixture.paint(
                        &mut text,
                        &mut scene,
                        frame.inset(Spacing::Loose.value()),
                        frame,
                        theme.into(),
                    );
                    let a = fixture.surface.reflow_anchor().unwrap();
                    let y = fixture.surface.caret_doc(a.utf16).min_y()
                        - fixture.surface.scroll().offset.y;
                    if let Some((i, old_y)) = anchor {
                        assert_eq!(a.utf16, i);
                        assert!((y - old_y).abs() <= 1.);
                        reports.push(serde_json::json!({
                            "theme": theme_id,
                            "width": width,
                            "scale": scale,
                            "stage": stage,
                            "error_pt": (y - old_y).abs()
                        }));
                    } else {
                        anchor = Some((a.utf16, y));
                    }
                    assert!(fixture.controls.get(EDITOR).unwrap().frame().max_y() <= frame.max_y());
                    let rgba = renderer
                        .render_to_rgba8(
                            &scene,
                            &mut text.atlas,
                            (width * scale, 640 * scale),
                            scale as f32,
                            Palette::from(theme).color("surface-1").unwrap(),
                        )
                        .unwrap();
                    if let Some(p) = &out {
                        save_png(
                            p.join(format!(
                                "reading-{theme_id}-{width}@{scale}x-stage{stage}.png"
                            )),
                            width * scale,
                            640 * scale,
                            rgba,
                        );
                    }
                }
            }
        }
    }
    if let Some(p) = out {
        std::fs::write(
            p.join("geometry.json"),
            serde_json::to_vec_pretty(&reports).unwrap(),
        )
        .unwrap();
    }
}

#[test]
fn full_pointer_shift_selection_uses_original_gpu_paint_in_four_themes() {
    let out = std::env::var_os("TEKESUI_POINTER_GPU_OUT").map(std::path::PathBuf::from);
    if let Some(p) = &out {
        std::fs::create_dir_all(p).unwrap();
    }
    let mut renderer = Renderer::new_headless().expect("real GPU required");
    for (theme_id, theme) in Theme::ALL.into_iter().enumerate() {
        for scale in [1u32, 2] {
            let mut fixture = ReadingFixture::new();
            let mut text = CoreTextHost::new().with_backing_scale(scale as f32);
            let frame = Rect::new(0., 0., 800., 640.);
            let mut baseline: Option<Vec<u8>> = None;
            for stage in 0..2 {
                if stage == 1 {
                    let field = fixture.controls.text_surface(EDITOR).unwrap();
                    let point = |index| {
                        let rect = field.caret_doc(index);
                        Point::new(
                            field.origin().x + rect.min_x() + 0.1,
                            field.caret().center().y,
                        )
                    };
                    let a = point(2);
                    let b = point(12);
                    for (phase, position, shift) in [
                        (PointerPhase::Down, a, false),
                        (PointerPhase::Up, a, false),
                        (PointerPhase::Down, b, true),
                        (PointerPhase::Up, b, true),
                    ] {
                        fixture.controls.event(&C::Pointer(PointerEvent {
                            position,
                            phase,
                            button: Some(PointerButton::Primary),
                            modifiers: PointerModifiers {
                                shift,
                                ..Default::default()
                            },
                            click_count: 1,
                        }));
                    }
                    assert_eq!(fixture.controls.focused(), Some(EDITOR));
                    assert!(
                        !fixture
                            .controls
                            .text_surface(EDITOR)
                            .unwrap()
                            .input()
                            .selection()
                            .is_empty()
                    );
                }
                text.atlas.begin_frame();
                text.begin_frame(std::time::Duration::ZERO, false);
                let mut scene = Scene::new();
                fixture.paint(&mut text, &mut scene, frame.inset(16.), frame, theme.into());
                let bytes = renderer
                    .render_to_rgba8(
                        &scene,
                        &mut text.atlas,
                        (800 * scale, 640 * scale),
                        scale as f32,
                        Palette::from(theme).color("surface-1").unwrap(),
                    )
                    .unwrap();
                if let Some(before) = &baseline {
                    assert_ne!(before, &bytes, "selection must reach real GPU painter");
                } else {
                    baseline = Some(bytes.clone());
                }
                if let Some(p) = &out {
                    save_png(
                        p.join(format!("pointer-{theme_id}@{scale}x-stage{stage}.png")),
                        800 * scale,
                        640 * scale,
                        bytes,
                    );
                }
            }
        }
    }
}
