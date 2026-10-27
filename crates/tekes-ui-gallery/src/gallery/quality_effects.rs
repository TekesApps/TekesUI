//! GPU checks of production overlay views, using the Gallery text adapter.
use super::text::TextEngine;
use tekes_ui_controls::extended::Popover;
use tekes_ui_controls::feedback_view::PopoverView;
use tekes_ui_controls::{DialogView, MenuView};
use tekes_ui_render::Renderer;
use tekes_ui_scene::{Primitive, Rect, Scene, Size};
use tekes_ui_tokens::{Palette, Theme, proposal};

#[test]
fn production_overlays_render_json_shadows_outside_surface_but_inside_ancestor() {
    let mut renderer =
        Renderer::new_headless().expect("actual GPU required for overlay verification");
    let output = std::env::var_os("TEKESUI_GOLD7_EFFECTS_OUT").map(std::path::PathBuf::from);
    if let Some(path) = &output {
        std::fs::create_dir_all(path).unwrap();
    }
    for theme in Theme::ALL {
        for scale in [1_u32, 2] {
            for kind in ["menu", "popover", "dialog"] {
                let mut text = TextEngine::new();
                text.set_scale(scale as f32);
                text.begin_frame();
                let mut scene = Scene::new();
                let viewport = Rect::new(16., 16., 608., 448.);
                let palette = Palette::from(theme);
                let role = proposal::TextRole::UiBody;
                let frame = match kind {
                    "menu" => {
                        let mut view = MenuView::new(3);
                        view.show(0);
                        view.render(
                            &mut scene,
                            &mut text,
                            &["Open", "Duplicate", "Archive"],
                            Rect::new(180., 100., 160., 36.),
                            viewport,
                            Some(viewport),
                            palette,
                            role,
                            false,
                        )
                        .unwrap()
                        .popup
                    }
                    "popover" => {
                        let mut view = PopoverView::default();
                        let mut model = Popover::default();
                        view.show(&mut model, 0);
                        view.render(
                            &mut scene,
                            &mut text,
                            &model,
                            Rect::new(180., 100., 160., 36.),
                            Size::new(240., 160.),
                            viewport,
                            Some(viewport),
                            palette,
                            role,
                        )
                        .frame
                    }
                    _ => {
                        let mut view = DialogView::default();
                        view.show(0);
                        view.key(tekes_ui_controls::ControlKey::Tab { reverse: false }, true);
                        view.render(
                            &mut scene,
                            &mut text,
                            "Confirm changes",
                            viewport,
                            Some(viewport),
                            palette,
                            role,
                            false,
                        )
                        .unwrap()
                        .modal
                    }
                };
                let shadows = scene
                    .primitives()
                    .iter()
                    .filter(|p| matches!(p, Primitive::Shadow(_)))
                    .count();
                assert_eq!(
                    shadows,
                    if theme.is_increased() { 0 } else { 2 },
                    "{kind}/{theme:?}"
                );
                let mut without_shadows = Scene::new();
                for primitive in scene.primitives() {
                    if !matches!(primitive, Primitive::Shadow(_)) {
                        without_shadows.push(primitive.clone());
                    }
                }
                let size = (640 * scale, 480 * scale);
                let clear = super::Gallery::background(theme);
                let actual = renderer
                    .render_to_rgba8(&scene, text.atlas_mut(), size, scale as f32, clear)
                    .unwrap();
                let baseline = renderer
                    .render_to_rgba8(
                        &without_shadows,
                        text.atlas_mut(),
                        size,
                        scale as f32,
                        clear,
                    )
                    .unwrap();
                let mut changed_outside_surface = 0;
                for y in 0..size.1 {
                    for x in 0..size.0 {
                        let pt = tekes_ui_scene::Point::new(
                            (x as f32 + 0.5) / scale as f32,
                            (y as f32 + 0.5) / scale as f32,
                        );
                        let i = ((y * size.0 + x) * 4) as usize;
                        if actual[i..i + 4] != baseline[i..i + 4] {
                            assert!(
                                viewport.contains(pt),
                                "shadow leaked outside ancestor: {kind}/{theme:?}/{pt:?}"
                            );
                            if !frame.inset(-1.).contains(pt) {
                                changed_outside_surface += 1;
                            }
                        }
                    }
                }
                if theme.is_increased() {
                    assert_eq!(actual, baseline);
                } else {
                    assert!(
                        changed_outside_surface > 20 * scale * scale,
                        "missing real external shadow: {kind}/{theme:?}/{scale}x"
                    );
                }
                if let Some(path) = &output {
                    let name = format!(
                        "{kind}-{}-{}-{scale}x.png",
                        if theme.is_dark() { "dark" } else { "light" },
                        if theme.is_increased() {
                            "ic"
                        } else {
                            "standard"
                        }
                    );
                    let mut encoder = png::Encoder::new(
                        std::fs::File::create(path.join(name)).unwrap(),
                        size.0,
                        size.1,
                    );
                    encoder.set_color(png::ColorType::Rgba);
                    encoder.set_depth(png::BitDepth::Eight);
                    encoder
                        .write_header()
                        .unwrap()
                        .write_image_data(&actual)
                        .unwrap();
                }
            }
        }
    }
}

#[test]
fn real_control_motion_changes_gpu_pixels_without_moving_hit_geometry() {
    use std::time::Duration;
    use tekes_ui_controls::extended::{Disclosure, Switch};
    use tekes_ui_controls::layout_view::DisclosureView;
    use tekes_ui_controls::{ButtonModel, ButtonSize, ButtonTextAlign, SwitchView, paint};
    let mut renderer = Renderer::new_headless().expect("actual GPU required for component motion");
    let mut text = TextEngine::new();
    text.set_scale(2.);
    text.set_motion_enabled(true);
    let palette = Palette::from(Theme::LIGHT);
    let mut button = ButtonModel::default();
    let button_frame = Rect::new(24., 24., 200., 40.);
    let switch_frame = Rect::new(24., 92., 280., 36.);
    let header = Rect::new(24., 152., 280., 40.);
    let body = Rect::new(24., 200., 280., 92.);
    let mut toggle = Switch::default();
    let mut switch = SwitchView::default();
    let mut disclosure = Disclosure::default();
    let mut disclosure_view = DisclosureView::default();
    let mut previous = Vec::new();
    let output = std::env::var_os("TEKESUI_GOLD7_EFFECTS_OUT").map(std::path::PathBuf::from);
    for (step, millis) in [0, 0, 50, 400].into_iter().enumerate() {
        if step == 1 {
            button.pointer_move_at(button_frame.center());
            assert!(switch.pointer_down(switch.layout().frame().center(), &toggle));
            assert_eq!(
                switch.pointer_up(switch.layout().frame().center(), &mut toggle),
                tekes_ui_controls::SelectionEvent::Changed
            );
            assert!(toggle.checked);
            assert!(disclosure_view.pointer_down(header.center()));
            assert!(disclosure_view.pointer_up(header.center(), &mut disclosure));
            assert!(disclosure.open);
        }
        text.set_motion_context(Duration::from_millis(millis), false);
        text.begin_frame();
        let mut scene = Scene::new();
        let layout = button.render(
            &mut scene,
            &mut text,
            "Hover feedback",
            button_frame,
            None,
            palette,
            paint::ButtonVariant::Quiet,
            ButtonSize::Body,
            proposal::TextRole::UiBody,
            ButtonTextAlign::default(),
        );
        assert_eq!(layout.frame, button_frame);
        assert!(layout.hit_test(button_frame.center()));
        switch.render_labeled(
            &mut scene,
            &mut text,
            toggle,
            switch_frame,
            None,
            palette,
            "Selection",
        );
        disclosure_view.render(
            &mut scene,
            &mut text,
            &disclosure,
            "Details",
            header,
            None,
            palette,
            proposal::TextRole::UiBody,
        );
        let reveal = disclosure_view.layout_body(&mut text, &disclosure, body, None);
        if step == 2 {
            assert!(reveal.progress > 0. && reveal.progress < 1.);
        }
        if let Some(clip) = reveal.visible_clip {
            scene.push_clip(clip);
            paint::surface_at_scale(&mut scene, reveal.body_rect, palette, true, text.scale());
            scene.pop_clip();
        }
        let pixels = renderer
            .render_to_rgba8(
                &scene,
                text.atlas_mut(),
                (720, 640),
                2.,
                super::Gallery::background(Theme::LIGHT),
            )
            .unwrap();
        if step >= 2 {
            // Inspect each production control's actual pixels, not just its model.
            for (top, bottom) in [(24, 64), (92, 128), (200, 292)] {
                let changed = (top * 2..bottom * 2).any(|y| {
                    let start = (y * 720 * 4) as usize;
                    pixels[start..start + 720 * 4] != previous[start..start + 720 * 4]
                });
                assert!(
                    changed,
                    "component rows {top}..{bottom} failed to paint motion at {millis}ms"
                );
            }
        }
        if let Some(path) = &output {
            std::fs::create_dir_all(path).unwrap();
            let mut encoder = png::Encoder::new(
                std::fs::File::create(path.join(format!("controls-step{step}-{millis}ms.png")))
                    .unwrap(),
                720,
                640,
            );
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder
                .write_header()
                .unwrap()
                .write_image_data(&pixels)
                .unwrap();
        }
        previous = pixels;
    }
    assert!(!text.motion_frame_requested());
    assert!(!button.needs_animation());
    assert!(!switch.needs_animation());
    // Change real state under Reduced Motion; no ornamental frames remain.
    toggle.checked = false;
    disclosure.open = false;
    text.set_motion_context(Duration::from_millis(410), true);
    text.begin_frame();
    let mut scene = Scene::new();
    switch.render_labeled(
        &mut scene,
        &mut text,
        toggle,
        switch_frame,
        None,
        palette,
        "Selection",
    );
    let reveal = disclosure_view.layout_body(&mut text, &disclosure, body, None);
    assert_eq!(reveal.progress, 0.);
    assert!(!reveal.interactive);
    assert!(!text.motion_frame_requested());
}

#[test]
fn production_overlay_enter_exit_rebuilds_live_glyphs_and_removes_closed_hits() {
    use std::time::Duration;
    use tekes_ui_controls::ControlKey;
    use tekes_ui_scene::Point;
    let mut renderer = Renderer::new_headless().expect("actual GPU required for overlay lifecycle");
    let output = std::env::var_os("TEKESUI_GOLD7_EFFECTS_OUT").map(std::path::PathBuf::from);
    for theme in Theme::ALL {
        let theme_name = format!(
            "{}-{}",
            if theme.is_dark() { "dark" } else { "light" },
            if theme.is_increased() {
                "ic"
            } else {
                "standard"
            }
        );
        for kind in ["menu", "dialog", "popover"] {
            let mut text = TextEngine::new();
            text.set_scale(2.0);
            text.set_motion_enabled(true);
            let mut menu = MenuView::new(3);
            let mut dialog = DialogView::default();
            let mut popover = PopoverView::default();
            let mut model = Popover::default();
            menu.show(7);
            dialog.show(7);
            popover.show(&mut model, 7);
            let viewport = Rect::new(16.0, 16.0, 608.0, 448.0);
            let anchor = Rect::new(180.0, 100.0, 160.0, 36.0);
            let palette = Palette::from(theme);
            let mut frames: Vec<Vec<u8>> = Vec::new();
            for (step, millis) in [0, 60, 200, 200, 260, 400].into_iter().enumerate() {
                if step == 3 {
                    menu.key(ControlKey::Escape, true);
                    dialog.close();
                    popover.close(&mut model);
                    assert!(!menu.is_open() && !dialog.is_open() && !model.open);
                    assert!(!dialog.blocks_background());
                    assert!(popover.focus_target().is_none());
                    assert_eq!(
                        menu.pointer_down_at(Point::new(200.0, 180.0)),
                        tekes_ui_controls::MenuEvent::None
                    );
                    assert_eq!(
                        dialog.pointer_down_at(Point::new(200.0, 180.0)),
                        tekes_ui_controls::DialogEvent::None
                    );
                    assert!(!popover.pointer_down(&mut model, Point::new(200.0, 180.0)));
                }
                if step == 4 {
                    // All retained AtlasGlyph coordinates become invalid here.
                    // Exit paint must shape/rasterize against the current atlas.
                    text.atlas_mut().clear();
                }
                text.set_motion_context(Duration::from_millis(millis), false);
                text.begin_frame();
                let mut scene = Scene::new();
                let layout_visible = match kind {
                    "menu" => menu
                        .render(
                            &mut scene,
                            &mut text,
                            &["Open", "Duplicate", "Archive"],
                            anchor,
                            viewport,
                            Some(viewport),
                            palette,
                            proposal::TextRole::UiBody,
                            false,
                        )
                        .is_some(),
                    "dialog" => dialog
                        .render(
                            &mut scene,
                            &mut text,
                            "Confirm changes",
                            viewport,
                            Some(viewport),
                            palette,
                            proposal::TextRole::UiBody,
                            false,
                        )
                        .is_some(),
                    _ => popover
                        .render_with_text_content(
                            &mut scene,
                            &mut text,
                            &model,
                            anchor,
                            Size::new(240.0, 160.0),
                            viewport,
                            Some(viewport),
                            palette,
                            proposal::TextRole::UiBody,
                            "live content",
                            |scene, text, bounds| {
                                use tekes_ui_controls::ButtonTextRenderer;
                                text.draw(
                                    scene,
                                    "Live popover content",
                                    proposal::TextRole::UiBody,
                                    bounds,
                                    tekes_ui_controls::ButtonTextAlign::Leading,
                                    palette.color("primary-text").unwrap(),
                                );
                            },
                        )
                        .visible
                        .is_some(),
                };
                assert_eq!(
                    layout_visible,
                    step < 3,
                    "{kind} published closed hit geometry"
                );
                if step == 4 {
                    assert!(
                        scene
                            .primitives()
                            .iter()
                            .any(|p| matches!(p, Primitive::Glyphs(_))),
                        "{kind} lost exit text after atlas reset"
                    );
                    assert!(
                        text.atlas_mut().stats().glyphs > 0,
                        "{kind} reused invalid retained glyphs"
                    );
                }
                let pixels = renderer
                    .render_to_rgba8(
                        &scene,
                        text.atlas_mut(),
                        (1280, 960),
                        2.0,
                        super::Gallery::background(theme),
                    )
                    .unwrap();
                if step == 4 {
                    let mut without_glyphs = Scene::new();
                    for primitive in scene.primitives() {
                        if !matches!(primitive, Primitive::Glyphs(_)) {
                            without_glyphs.push(primitive.clone());
                        }
                    }
                    let glyph_free = renderer
                        .render_to_rgba8(
                            &without_glyphs,
                            text.atlas_mut(),
                            (1280, 960),
                            2.0,
                            super::Gallery::background(theme),
                        )
                        .unwrap();
                    let visible_glyph_pixels = pixels
                        .chunks_exact(4)
                        .zip(glyph_free.chunks_exact(4))
                        .filter(|(actual, baseline)| actual != baseline)
                        .count();
                    assert!(
                        visible_glyph_pixels > 20,
                        "{kind}/{theme_name} exit glyphs disappeared after atlas reset: {visible_glyph_pixels} pixels"
                    );
                }
                if step == 1 || step == 4 {
                    let blank = &frames[0];
                    for y in 0..960 {
                        for x in 0..1280 {
                            let point = Point::new((x as f32 + 0.5) / 2.0, (y as f32 + 0.5) / 2.0);
                            if !viewport.contains(point) {
                                let offset = ((y * 1280 + x) * 4) as usize;
                                assert_eq!(
                                    pixels[offset..offset + 4],
                                    blank[offset..offset + 4],
                                    "{kind}/{theme_name} mid-frame paint escaped ancestor"
                                );
                            }
                        }
                    }
                }
                if let Some(path) = &output {
                    std::fs::create_dir_all(path).unwrap();
                    let mut encoder = png::Encoder::new(
                        std::fs::File::create(path.join(format!(
                            "overlay-{kind}-{theme_name}-step{step}-{millis}ms-2x.png"
                        )))
                        .unwrap(),
                        1280,
                        960,
                    );
                    encoder.set_color(png::ColorType::Rgba);
                    encoder.set_depth(png::BitDepth::Eight);
                    encoder
                        .write_header()
                        .unwrap()
                        .write_image_data(&pixels)
                        .unwrap();
                }
                frames.push(pixels);
            }
            assert_eq!(frames[0], frames[5], "{kind} left ghost pixels after exit");
            assert_ne!(frames[1], frames[0], "{kind} enter did not paint");
            assert_ne!(
                frames[1], frames[2],
                "{kind} enter skipped intermediate pixels"
            );
            assert_ne!(frames[4], frames[2], "{kind} exit did not fade");
            assert_ne!(
                frames[4], frames[5],
                "{kind} exit skipped intermediate pixels"
            );
            match kind {
                "menu" => assert!(!menu.is_presented()),
                "dialog" => assert!(!dialog.is_presented()),
                _ => assert!(!popover.is_presented()),
            }
        }
    }
}

/// Test-only recorder: keep production geometry, text and clipping intact and
/// normalize only decorative panel borders to the existing shared rule.
pub(crate) fn record_panel_host_scale(
    scene: &Scene,
    text: &mut TextEngine,
    palette: Palette,
    name: &str,
    gpu: Option<&mut Renderer>,
) -> usize {
    let scale = text.scale();
    let expected = tekes_ui_controls::paint::decorative_border(palette, scale).0;
    let is_panel = |q: &tekes_ui_scene::Quad| {
        q.fill == palette.color("surface-panel").unwrap()
            && q.border_color == palette.color("separator").unwrap()
            && q.border_width > 0.
    };
    let panels = scene
        .primitives()
        .iter()
        .filter_map(|p| match p {
            Primitive::Quad(q) if is_panel(q) => Some(q),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(
        !panels.is_empty(),
        "actual production panel expected: {name}"
    );
    let wrong = panels.iter().filter(|q| q.border_width != expected).count();
    if let Some(gpu) = gpu {
        let out = std::path::PathBuf::from(std::env::var_os("TEKESUI_PANEL_SCALE_OUT").unwrap());
        std::fs::create_dir_all(&out).unwrap();
        let mut canonical = Scene::new();
        for p in scene.primitives() {
            let mut p = p.clone();
            if let Primitive::Quad(q) = &mut p
                && is_panel(q)
            {
                q.border_width = expected;
            }
            canonical.push(p);
        }
        let size = ((720. * scale) as u32, (560. * scale) as u32);
        let clear = palette.color("surface-1").unwrap();
        let actual = gpu
            .render_to_rgba8(scene, text.atlas_mut(), size, scale, clear)
            .unwrap();
        let reference = gpu
            .render_to_rgba8(&canonical, text.atlas_mut(), size, scale, clear)
            .unwrap();
        let mut before_changed = None;
        if let Some(before) = std::env::var_os("TEKESUI_PANEL_SCALE_BEFORE") {
            let before = std::path::PathBuf::from(before);
            let decode = |p| {
                let mut r = png::Decoder::new(std::fs::File::open(p).unwrap())
                    .read_info()
                    .unwrap();
                let mut b = vec![0; r.output_buffer_size()];
                let info = r.next_frame(&mut b).unwrap();
                b.truncate(info.buffer_size());
                b
            };
            assert_eq!(
                actual,
                decode(before.join(format!("{name}-reference.png"))),
                "actual caller output must equal prior existing-rule reference: {name}"
            );
            let old = decode(before.join(format!("{name}-actual.png")));
            before_changed = Some(
                actual
                    .chunks_exact(4)
                    .zip(old.chunks_exact(4))
                    .filter(|(a, b)| a != b)
                    .count(),
            );
        }
        for (kind, bytes) in [("actual", &actual), ("reference", &reference)] {
            let mut png = png::Encoder::new(
                std::fs::File::create(out.join(format!("{name}-{kind}.png"))).unwrap(),
                size.0,
                size.1,
            );
            png.set_color(png::ColorType::Rgba);
            png.set_depth(png::BitDepth::Eight);
            png.write_header().unwrap().write_image_data(bytes).unwrap();
        }
        std::fs::write(out.join(format!("{name}.json")), serde_json::to_vec_pretty(&serde_json::json!({"case":name,"scale":scale,"panels":panels.len(),"expected_width":expected,"wrong_panels":wrong,"actual_reference_equal":actual==reference,"before_after_changed_pixels":before_changed,"scope":"actual production Gallery scene, bounded panel border only; not whole page acceptance"})).unwrap()).unwrap();
    }
    wrong
}
