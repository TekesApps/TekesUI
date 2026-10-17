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
    use tekes_ui_controls::{Button, ButtonSize, ButtonTextAlign, SwitchView, paint};
    let mut renderer = Renderer::new_headless().expect("actual GPU required for component motion");
    let mut text = TextEngine::new();
    text.set_scale(2.);
    text.set_motion_enabled(true);
    let palette = Palette::from(Theme::LIGHT);
    let mut button = Button::default();
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
            paint::surface(&mut scene, reveal.body_rect, palette, true);
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
