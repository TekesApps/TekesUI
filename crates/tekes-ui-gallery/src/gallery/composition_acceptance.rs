//! Real-backend validation of a separate consumer crate. No production Gallery
//! routes migrate until the three minimal consumer scenarios have been proven.
use super::text::TextEngine;
use tekes_ui_composition_consumer::{IconImages, compound_button};
use tekes_ui_controls::{
    ControlKey,
    composition::{Button, ComponentId, ContentEvent},
};
use tekes_ui_render::Renderer;
use tekes_ui_scene::{Rect, Scene, Size};
use tekes_ui_tokens::{Palette, Theme};

#[test]
fn required_field_metadata_and_real_consumer_keep_markers_at_narrow_widths() {
    use tekes_ui_composition_consumer::field_metadata::{self, DISABLED, ERROR, NORMAL};
    use tekes_ui_controls::composition::TextFieldContent;
    use tekes_ui_controls::{
        Field,
        feedback_view::{FieldPresentation, FieldView},
    };
    use tekes_ui_tokens::proposal::TextRole;
    let mut renderer = Renderer::new_headless().expect("real GPU required");
    let output = std::env::var_os("TEKESUI_FIELD_GPU_OUT").map(std::path::PathBuf::from);
    if let Some(path) = &output {
        std::fs::create_dir_all(path).unwrap();
    }
    let mut cases = Vec::new();
    let mut lost_markers = 0;
    for (theme_index, theme) in Theme::ALL.into_iter().enumerate() {
        let palette = Palette::from(theme);
        for scale in [1_u32, 2] {
            for width in [140_u32, 344] {
                for type_scale in [1., 1.5] {
                    let mut text = TextEngine::new();
                    text.set_scale(scale as f32);
                    text.set_type_scale(type_scale);
                    for presentation in ["body", "heading", "consumer"] {
                        text.begin_frame();
                        let mut scene = Scene::new();
                        let height = if presentation == "consumer" {
                            1800
                        } else {
                            600
                        };
                        let frame = Rect::new(8., 8., width as f32, (height - 16) as f32);
                        let mut markers = Vec::new();
                        if presentation == "consumer" {
                            let mut host = field_metadata::build().unwrap();
                            host.render(&mut scene, &mut text, frame, frame, palette)
                                .unwrap();
                            for id in [NORMAL, ERROR, DISABLED] {
                                let field = host
                                    .get(id)
                                    .unwrap()
                                    .component::<TextFieldContent>()
                                    .unwrap();
                                let layout = field.view.layout();
                                markers.push(layout.required_mark.unwrap());
                                assert_eq!(layout.slot, field.surface.outer);
                                assert!(layout.message.min_y() >= layout.slot.max_y());
                            }
                        } else {
                            let model = Field {
                                label: "Required workspace display name with a wrapping label"
                                    .into(),
                                help: "Use a name that other people can recognize.".into(),
                                required: true,
                                ..Default::default()
                            };
                            let layout = FieldView::default().render_with_presentation(
                                &mut scene,
                                &mut text,
                                &model,
                                frame,
                                Some(frame),
                                palette,
                                TextRole::UiBody,
                                if presentation == "heading" {
                                    FieldPresentation::emphasized_form()
                                } else {
                                    FieldPresentation::default()
                                },
                            );
                            markers.push(layout.required_mark.unwrap());
                            assert!(layout.slot.min_y() >= layout.label.max_y());
                        }
                        for mark in &markers {
                            let valid =
                                mark.size.width > 0. && mark.max_x() <= frame.max_x() + 0.01;
                            if !valid {
                                lost_markers += 1;
                            }
                        }
                        assert!(scene.primitives().iter().any(|p| matches!(p, tekes_ui_scene::Primitive::Glyphs(r) if !r.glyphs.is_empty())));
                        if presentation != "consumer" {
                            let runs = scene
                                .primitives()
                                .iter()
                                .filter_map(|p| {
                                    if let tekes_ui_scene::Primitive::Glyphs(run) = p {
                                        Some(run)
                                    } else {
                                        None
                                    }
                                })
                                .collect::<Vec<_>>();
                            if let (Some(label), Some(mark)) =
                                (runs.first(), runs.iter().find(|r| r.glyphs.len() == 1))
                            {
                                assert!(
                                    (label.glyphs[0].origin.y - mark.glyphs[0].origin.y).abs()
                                        <= 1.,
                                    "same-role required marker must share the first label baseline"
                                );
                            }
                        }
                        let size = ((width + 16) * scale, height * scale);
                        let rgba = renderer
                            .render_to_rgba8(
                                &scene,
                                text.atlas_mut(),
                                size,
                                scale as f32,
                                palette.color("document-canvas").unwrap(),
                            )
                            .unwrap();
                        let file = format!(
                            "field-{theme_index}-{width}@{scale}x-type-{type_scale}-{presentation}.png"
                        );
                        if let Some(path) = &output {
                            let mut encoder = png::Encoder::new(
                                std::fs::File::create(path.join(&file)).unwrap(),
                                size.0,
                                size.1,
                            );
                            encoder.set_color(png::ColorType::Rgba);
                            encoder.set_depth(png::BitDepth::Eight);
                            encoder
                                .write_header()
                                .unwrap()
                                .write_image_data(&rgba)
                                .unwrap();
                        }
                        cases.push(serde_json::json!({"file":file,"theme":theme_index,"scale":scale,"width":width,"type_scale":type_scale,"presentation":presentation,"marker_widths":markers.iter().map(|m|m.size.width).collect::<Vec<_>>()}));
                    }
                }
            }
        }
    }
    assert_eq!(cases.len(), 96);
    if let Some(path) = output {
        std::fs::write(
            path.join("manifest.json"),
            serde_json::to_vec_pretty(&cases).unwrap(),
        )
        .unwrap();
    }
    // Preserve complete real pre-fix image evidence before reporting the failure.
    assert_eq!(
        lost_markers, 0,
        "required indicators lost in actual shaped layouts"
    );
}

#[test]
fn number_field_existing_states_use_actual_glyphs_and_shared_button_presentation() {
    use tekes_ui_controls::{
        ButtonModel, ButtonSize, NumberField,
        feedback_view::{NumberFieldEvent, NumberFieldView},
    };
    use tekes_ui_scene::Primitive;
    use tekes_ui_tokens::proposal::TextRole;
    let mut renderer = Renderer::new_headless().expect("real GPU required");
    let output = std::env::var_os("TEKESUI_NUMBER_GPU_OUT").map(std::path::PathBuf::from);
    if let Some(p) = &output {
        std::fs::create_dir_all(p).unwrap();
    }
    let mut cases = Vec::new();
    for (theme_index, theme) in Theme::ALL.into_iter().enumerate() {
        for scale in [1_u32, 2] {
            for width in [140_u32, 198] {
                for type_scale in [1., 1.5] {
                    for state in ["rest", "hover", "pressed", "focus", "disabled"] {
                        let palette = Palette::from(theme);
                        let mut text = TextEngine::new();
                        text.set_scale(scale as f32);
                        text.set_type_scale(type_scale);
                        text.begin_frame();
                        let model = NumberField::new(2., 0., 4., 1.).unwrap();
                        let mut view = NumberFieldView::default();
                        let button = ButtonModel::measure(
                            &mut text,
                            "+",
                            48.,
                            ButtonSize::Body,
                            TextRole::UiBody,
                        );
                        let number = tekes_ui_controls::ButtonTextRenderer::measure(
                            &mut text,
                            "2",
                            TextRole::UiHeading,
                            80.,
                        );
                        let frame =
                            Rect::new(8., 8., width as f32, button.height.max(number.height));
                        let clip = Rect::new(0., 0., width as f32 + 16., 96.);
                        let first = view.render(
                            &mut Scene::new(),
                            &mut text,
                            &model,
                            frame,
                            Some(clip),
                            palette,
                            TextRole::UiBody,
                        );
                        let point = first.increment.frame.center();
                        match state {
                            "hover" => view.pointer_move(point),
                            "pressed" => assert_eq!(view.pointer_down(point), Some(true)),
                            "focus" => view.focus(true, true),
                            "disabled" => view.set_disabled(true),
                            _ => {}
                        }
                        let mut scene = Scene::new();
                        text.begin_frame();
                        let layout = view.render(
                            &mut scene,
                            &mut text,
                            &model,
                            frame,
                            Some(clip),
                            palette,
                            TextRole::UiBody,
                        );
                        assert_eq!(layout.frame, first.frame);
                        assert_eq!(layout.value, first.value);
                        assert!(layout.increment.frame.max_x() <= frame.max_x() + 0.01);
                        assert!(scene.primitives().iter().any(
                            |p| matches!(p, Primitive::Glyphs(run) if !run.glyphs.is_empty())
                        ));
                        let rgba = renderer
                            .render_to_rgba8(
                                &scene,
                                text.atlas_mut(),
                                ((width + 16) * scale, 96 * scale),
                                scale as f32,
                                palette.color("document-canvas").unwrap(),
                            )
                            .unwrap();
                        assert_eq!(model.value(), 2.);
                        let mut unchanged = model;
                        view.cancel();
                        assert_eq!(
                            view.pointer_up(point, &mut unchanged),
                            NumberFieldEvent::None
                        );
                        assert_eq!(unchanged.value(), 2.);
                        let file = format!(
                            "number-{theme_index}-{width}@{scale}x-type-{type_scale}-{state}.png"
                        );
                        if let Some(p) = &output {
                            let mut enc = png::Encoder::new(
                                std::fs::File::create(p.join(&file)).unwrap(),
                                (width + 16) * scale,
                                96 * scale,
                            );
                            enc.set_color(png::ColorType::Rgba);
                            enc.set_depth(png::BitDepth::Eight);
                            enc.write_header().unwrap().write_image_data(&rgba).unwrap();
                        }
                        cases.push(serde_json::json!({"file":file,"theme":theme_index,"scale":scale,"width":width,"type_scale":type_scale,"state":state,"value":model.value()}));
                    }
                }
            }
        }
    }
    assert_eq!(cases.len(), 160);
    if let Some(p) = output {
        std::fs::write(
            p.join("manifest.json"),
            serde_json::to_vec_pretty(&cases).unwrap(),
        )
        .unwrap();
    }
}

#[test]
fn independent_plain_content_has_no_form_pixels_and_keeps_real_selection_in_four_themes() {
    use tekes_ui_composition_consumer::plain_text::{BODY, SCROLLER, Scenario};
    use tekes_ui_controls::composition::{
        AccessibilityAction, Content, ContentHost, ScrollContent, TextFieldContent,
    };
    use tekes_ui_scene::Point;
    use tekes_ui_tokens::proposal::TextRole;

    let mut renderer = Renderer::new_headless().expect("real GPU required");
    let output = std::env::var_os("TEKESUI_PLAIN_GPU_OUT").map(std::path::PathBuf::from);
    if let Some(p) = &output {
        std::fs::create_dir_all(p).unwrap();
    }
    let mut cases = Vec::new();
    for (theme_index, theme) in Theme::ALL.into_iter().enumerate() {
        let palette = Palette::from(theme);
        let clear = palette.color("document-canvas").unwrap();
        for scale in [1_u32, 2] {
            let mut text = TextEngine::new();
            text.set_scale(scale as f32);
            // Empty content must leave every pixel to its parent, including the
            // semantic name. Ordinary Field must still paint its label/chrome.
            let size = (320 * scale, 160 * scale);
            let blank = renderer
                .render_to_rgba8(&Scene::new(), text.atlas_mut(), size, scale as f32, clear)
                .unwrap();
            for plain in [true, false] {
                let field = if plain {
                    TextFieldContent::read_only_content("Semantic name", "", TextRole::UiBody)
                } else {
                    TextFieldContent::new("Semantic name", "")
                };
                let mut host = ContentHost::new(Content::new(ComponentId(1), field)).unwrap();
                let mut scene = Scene::new();
                text.begin_frame();
                host.render(
                    &mut scene,
                    &mut text,
                    Rect::new(16., 16., 288., 128.),
                    Rect::new(0., 0., 320., 160.),
                    palette,
                )
                .unwrap();
                let pixels = renderer
                    .render_to_rgba8(&scene, text.atlas_mut(), size, scale as f32, clear)
                    .unwrap();
                if plain {
                    assert_eq!(pixels, blank, "content must not paint a form or label");
                } else {
                    assert_ne!(pixels, blank, "ordinary Field must retain its presentation");
                }
            }
            for width in [360_u32, 720] {
                for type_scale in [1., 1.5] {
                    text.set_type_scale(type_scale);
                    let mut consumer = Scenario::new().unwrap();
                    let frame = Rect::new(8., 8., width as f32 - 16., 464.);
                    let clip = Rect::new(0., 0., width as f32, 480.);
                    text.begin_frame();
                    consumer
                        .host
                        .render(&mut Scene::new(), &mut text, frame, clip, palette)
                        .unwrap();
                    assert!(
                        consumer
                            .host
                            .accessibility_action(
                                BODY,
                                None,
                                AccessibilityAction::SetSelection(8..8)
                            )
                            .consumed
                    );
                    let mut unselected = Vec::new();
                    let mut selection_rects = Vec::new();
                    let mut initial_width = None;
                    let mut initial_height = 0.;
                    for phase in ["focused", "selected", "scrolled"] {
                        if phase == "selected" {
                            assert!(
                                consumer
                                    .host
                                    .accessibility_action(
                                        BODY,
                                        None,
                                        AccessibilityAction::SetSelection(8..12)
                                    )
                                    .consumed
                            );
                        } else if phase == "scrolled" {
                            let point = consumer
                                .host
                                .text_surface(BODY)
                                .unwrap()
                                .visible_frame()
                                .unwrap()
                                .center();
                            consumer.host.event(&ContentEvent::Scroll {
                                position: point,
                                delta: Point::new(0., 100.),
                                precise: true,
                                begin: true,
                                end: false,
                                momentum: false,
                                time: 1.,
                            });
                        }
                        text.begin_frame();
                        let mut scene = Scene::new();
                        consumer
                            .host
                            .render(&mut scene, &mut text, frame, clip, palette)
                            .unwrap();
                        let body = consumer.host.text_surface(BODY).unwrap();
                        assert_eq!(body.inner, body.outer, "no form padding or metadata gap");
                        assert_eq!(body.scroll.offset, Point::new(0., 0.));
                        if phase == "focused" {
                            initial_width = body.width;
                            initial_height = body.outer.size.height;
                        } else {
                            assert_eq!(body.width, initial_width);
                            assert_eq!(body.outer.size.height, initial_height);
                            assert_eq!(body.input.selected_text(), "中文😀");
                            assert_eq!(body.input.selection(), 8..12);
                        }
                        if phase == "selected" {
                            selection_rects = body.selection_rects(8..12);
                        }
                        let pixels = renderer
                            .render_to_rgba8(
                                &scene,
                                text.atlas_mut(),
                                (width * scale, 480 * scale),
                                scale as f32,
                                clear,
                            )
                            .unwrap();
                        if phase == "focused" {
                            unselected = pixels.clone();
                        } else if phase == "selected" {
                            // Compare within the shaped UTF-16 selection, not a
                            // generic whole-image change caused by focus ink.
                            assert!(
                                selection_rects.iter().any(|r| {
                                    let x0 = (r.min_x().max(0.) * scale as f32).ceil() as u32;
                                    let x1 =
                                        (r.max_x().min(width as f32) * scale as f32).floor() as u32;
                                    let y0 = (r.min_y().max(0.) * scale as f32).ceil() as u32;
                                    let y1 = (r.max_y().min(480.) * scale as f32).floor() as u32;
                                    (y0..y1).any(|y| {
                                        (x0..x1).any(|x| {
                                            let i = ((y * width * scale + x) * 4) as usize;
                                            pixels[i..i + 4] != unselected[i..i + 4]
                                        })
                                    })
                                }),
                                "actual selected glyph region must change pixels"
                            );
                        }
                        let offset = consumer
                            .host
                            .get(SCROLLER)
                            .unwrap()
                            .component::<ScrollContent>()
                            .unwrap()
                            .scroll
                            .offset
                            .y;
                        if phase == "scrolled" {
                            assert!(offset > 0., "parent owns document scrolling");
                        }
                        let file = format!(
                            "plain-{theme_index}-{width}@{scale}x-type-{type_scale}-{phase}.png"
                        );
                        if let Some(p) = &output {
                            let mut enc = png::Encoder::new(
                                std::fs::File::create(p.join(&file)).unwrap(),
                                width * scale,
                                480 * scale,
                            );
                            enc.set_color(png::ColorType::Rgba);
                            enc.set_depth(png::BitDepth::Eight);
                            enc.write_header()
                                .unwrap()
                                .write_image_data(&pixels)
                                .unwrap();
                        }
                        cases.push(serde_json::json!({"file":file,"theme":theme_index,"width":width,"scale":scale,"type_scale":type_scale,"phase":phase,"parent_offset":offset,"nested_offset":0}));
                    }
                }
            }
        }
    }
    assert_eq!(cases.len(), 96);
    if let Some(p) = output {
        std::fs::write(
            p.join("manifest.json"),
            serde_json::to_vec_pretty(&cases).unwrap(),
        )
        .unwrap();
    }
}

#[test]
fn independent_consumer_compound_button_uses_real_icon_text_and_gpu_in_four_themes() {
    let mut renderer = Renderer::new_headless().expect("real GPU required");
    let output = std::env::var_os("TEKESUI_COMPOSITION_OUT").map(std::path::PathBuf::from);
    if let Some(p) = &output {
        std::fs::create_dir_all(p).unwrap()
    }
    for (theme_index, theme) in Theme::ALL.into_iter().enumerate() {
        let palette = Palette::from(theme);
        let ink = palette.color("primary-text").unwrap();
        let color = format!(
            "#{:02x}{:02x}{:02x}",
            (ink.r * 255.).round() as u8,
            (ink.g * 255.).round() as u8,
            (ink.b * 255.).round() as u8
        );
        let svg = tekes_ui_icons::Icon::Settings
            .svg()
            .replace("currentColor", &color);
        let tree = resvg::usvg::Tree::from_str(&svg, &resvg::usvg::Options::default()).unwrap();
        let mut pixmap = resvg::tiny_skia::Pixmap::new(48, 48).unwrap();
        resvg::render(
            &tree,
            resvg::tiny_skia::Transform::from_scale(2., 2.),
            &mut pixmap.as_mut(),
        );
        let image = renderer.register_image(pixmap.data(), 48, 48).unwrap();
        for width in [360_u32, 800] {
            for scale in [1_u32, 2] {
                for type_scale in [1., 1.5] {
                    let mut text = TextEngine::new();
                    text.set_scale(scale as f32);
                    text.set_type_scale(type_scale);
                    text.begin_frame();
                    let mut host = compound_button(IconImages(vec![(ink, image)])).unwrap();
                    let extent = host
                        .get_mut(ComponentId(1))
                        .unwrap()
                        .measure(&mut text, Size::new(width as f32 - 32., 220.))
                        .size;
                    let frame = Rect::new(16., 16., extent.width, extent.height);
                    let clip = Rect::new(0., 0., width as f32, 240.);
                    let mut scene = Scene::new();
                    host.render(&mut scene, &mut text, frame, clip, palette)
                        .unwrap();
                    assert_eq!(host.focus_targets().len(), 1);
                    assert_eq!(host.semantics().len(), 1);
                    let badge = host.get(ComponentId(7)).unwrap().frame();
                    assert!(
                        badge.max_x() <= frame.max_x() + 0.1,
                        "badge must remain within the compound action at narrow/large text"
                    );
                    let caption = host.get(ComponentId(6)).unwrap().frame();
                    assert!(caption.max_y() <= frame.max_y() + 0.1);
                    host.focus(Some(ComponentId(1)), true);
                    host.event(&ContentEvent::Key {
                        key: ControlKey::Enter,
                        down: true,
                    });
                    assert_eq!(
                        host.get(ComponentId(1))
                            .unwrap()
                            .component::<Button>()
                            .unwrap()
                            .activations,
                        1
                    );
                    scene.clear();
                    text.begin_frame();
                    host.render(&mut scene, &mut text, frame, clip, palette)
                        .unwrap();
                    let rgba = renderer
                        .render_to_rgba8(
                            &scene,
                            text.atlas_mut(),
                            (width * scale, 240 * scale),
                            scale as f32,
                            palette.color("surface-1").unwrap(),
                        )
                        .unwrap();
                    if let Some(p) = &output {
                        let f = std::fs::File::create(p.join(format!(
                            "button-theme-{theme_index}-{width}@{scale}x-type-{type_scale}.png"
                        )))
                        .unwrap();
                        let mut enc = png::Encoder::new(f, width * scale, 240 * scale);
                        enc.set_color(png::ColorType::Rgba);
                        enc.set_depth(png::BitDepth::Eight);
                        enc.write_header().unwrap().write_image_data(&rgba).unwrap();
                    }
                }
            }
        }
        renderer.unregister_image(image);
    }
}

#[test]
fn independent_list_and_rich_dialog_use_production_views_and_real_gpu() {
    use tekes_ui_composition_consumer::scenarios::{self, ids};
    use tekes_ui_controls::composition::{DialogContent, MenuContent, TextFieldContent};
    let mut renderer = Renderer::new_headless().expect("real GPU required");
    let output = std::env::var_os("TEKESUI_COMPOSITION_OUT").map(std::path::PathBuf::from);
    if let Some(p) = &output {
        std::fs::create_dir_all(p).unwrap();
    }
    let mut count = 0;
    for (theme_index, theme) in Theme::ALL.into_iter().enumerate() {
        let palette = Palette::from(theme);
        for width in [360u32, 800] {
            for scale in [1u32, 2] {
                for type_scale in [1., 1.5] {
                    for kind in ["list", "dialog"] {
                        let mut host = if kind == "list" {
                            scenarios::heterogeneous_list().unwrap().host
                        } else {
                            scenarios::rich_dialog().unwrap()
                        };
                        let mut text = TextEngine::new();
                        text.set_scale(scale as f32);
                        text.set_type_scale(type_scale);
                        let frame = Rect::new(16., 16., width as f32 - 32., 608.);
                        let clip = Rect::new(0., 0., width as f32, 640.);
                        if kind == "dialog" {
                            host.show_dialog(ids::DIALOG).unwrap();
                        }
                        for state in ["base", "active"] {
                            if state == "active" {
                                if kind == "list" {
                                    host.focus(Some(ids::LIST_MENU), true);
                                    host.event(&ContentEvent::Key {
                                        key: ControlKey::Enter,
                                        down: true,
                                    });
                                } else {
                                    host.get_mut(ids::DIALOG_FIELD)
                                        .unwrap()
                                        .component_mut::<TextFieldContent>()
                                        .unwrap()
                                        .surface
                                        .input
                                        .text
                                        .clear();
                                    assert!(!scenarios::validate_dialog(&mut host));
                                }
                            }
                            let mut scene = Scene::new();
                            text.begin_frame();
                            host.render(&mut scene, &mut text, frame, clip, palette)
                                .unwrap();
                            if kind == "list" {
                                let field = host
                                    .get(ids::LIST_FIELD)
                                    .unwrap()
                                    .component::<TextFieldContent>()
                                    .unwrap();
                                assert!(field.surface.paragraph.is_some());
                                assert_eq!(field.surface.shaped, "Tekes");
                                let field_height =
                                    host.get(ids::LIST_FIELD).unwrap().frame().size.height;
                                assert!(
                                    field_height
                                        > host.get(ids::LIST_TEXT).unwrap().frame().size.height
                                );
                                if state == "active" {
                                    let menu = host
                                        .get(ids::LIST_MENU)
                                        .unwrap()
                                        .component::<MenuContent>()
                                        .unwrap();
                                    assert!(menu.popup.as_ref().unwrap().visible.is_some());
                                }
                            } else {
                                let modal = host
                                    .get(ids::DIALOG)
                                    .unwrap()
                                    .component::<DialogContent>()
                                    .unwrap()
                                    .modal_frame();
                                let footer = host.get(ids::DIALOG_FOOTER).unwrap().frame();
                                assert!(footer.max_y() <= modal.max_y());
                                let surface = host
                                    .get(ids::DIALOG_FIELD)
                                    .unwrap()
                                    .component::<TextFieldContent>()
                                    .unwrap();
                                assert!(surface.surface.paragraph.is_some());
                                assert_eq!(surface.field.error.is_some(), state == "active");
                                assert!(
                                    !host
                                        .focus_targets()
                                        .iter()
                                        .any(|(id, _)| *id == ids::DIALOG_INVOKER)
                                );
                                assert_eq!(
                                    host.semantics()[0].semantics.role,
                                    tekes_ui_controls::composition::ContentRole::Dialog
                                );
                            }
                            let rgba = renderer
                                .render_to_rgba8(
                                    &scene,
                                    text.atlas_mut(),
                                    (width * scale, 640 * scale),
                                    scale as f32,
                                    palette.color("surface-1").unwrap(),
                                )
                                .unwrap();
                            if let Some(p) = &output {
                                let path=p.join(format!("{kind}-{state}-theme-{theme_index}-{width}@{scale}x-type-{type_scale}.png"));
                                let mut encoder = png::Encoder::new(
                                    std::fs::File::create(path).unwrap(),
                                    width * scale,
                                    640 * scale,
                                );
                                encoder.set_color(png::ColorType::Rgba);
                                encoder.set_depth(png::BitDepth::Eight);
                                encoder
                                    .write_header()
                                    .unwrap()
                                    .write_image_data(&rgba)
                                    .unwrap();
                            }
                            count += 1;
                        }
                    }
                }
            }
        }
    }
    assert_eq!(count, 128);
}

#[test]
fn independent_rich_dialog_presence_rebuilds_gpu_frames_and_reduces_motion() {
    use std::time::Duration;
    use tekes_ui_composition_consumer::scenarios::{self, ids};
    use tekes_ui_controls::composition::DialogContent;
    let mut renderer = Renderer::new_headless().expect("real GPU required");
    let output = std::env::var_os("TEKESUI_PRESENCE_OUT").map(std::path::PathBuf::from);
    if let Some(p) = &output {
        std::fs::create_dir_all(p).unwrap();
    }
    for (theme_index, theme) in Theme::ALL.into_iter().enumerate() {
        for scale in [1u32, 2] {
            let palette = Palette::from(theme);
            let mut host = scenarios::rich_dialog().unwrap();
            let mut text = TextEngine::new();
            text.set_scale(scale as f32);
            text.set_motion_enabled(true);
            let frame = Rect::new(16., 16., 688., 608.);
            let clip = Rect::new(0., 0., 720., 640.);
            // Establish real invoker geometry and focus before opening the dialog.
            text.begin_frame();
            host.render(&mut Scene::new(), &mut text, frame, clip, palette)
                .unwrap();
            host.focus(Some(ids::DIALOG_INVOKER), true);
            host.show_dialog(ids::DIALOG).unwrap();
            for (name, ms, reduced) in [
                ("enter-start", 0, false),
                ("enter-mid", 50, false),
                ("entered", 1000, false),
                ("exit-start", 1001, false),
                ("exit-mid", 1050, false),
                ("exited", 2000, false),
                ("reverse-start", 2100, false),
                ("reverse-mid", 2150, false),
                ("reduce-open", 2151, true),
                ("reduce-exit", 2152, true),
            ] {
                if name == "exit-start" || name == "reduce-exit" {
                    host.close_dialog();
                }
                if name == "reverse-start" {
                    host.show_dialog(ids::DIALOG).unwrap();
                }
                text.set_motion_context(Duration::from_millis(ms), reduced);
                // Deliberately invalidate old glyph handles: the exit subtree must
                // shape/push its glyphs again in this frame, as existing overlays do.
                text.atlas_mut().clear();
                text.begin_frame();
                let mut scene = Scene::new();
                host.render(&mut scene, &mut text, frame, clip, palette)
                    .unwrap();
                if name.starts_with("exit") || name == "reduce-exit" {
                    assert_eq!(host.focused(), Some(ids::DIALOG_INVOKER));
                    assert!(host.semantics().iter().all(|node| node.id != ids::DIALOG));
                    assert!(host.focused_text().is_none());
                }
                let dialog = host
                    .get(ids::DIALOG)
                    .unwrap()
                    .component::<DialogContent>()
                    .unwrap();
                if name == "exited" || name == "reduce-exit" {
                    assert!(!dialog.is_presented());
                    assert!(!dialog.needs_animation());
                }
                if name == "exit-mid" {
                    assert!(dialog.is_presented());
                    assert!(text.motion_frame_requested());
                    assert!(
                        scene
                            .primitives()
                            .iter()
                            .any(|p| matches!(p, tekes_ui_scene::Primitive::Glyphs(_)))
                    );
                }
                let rgba = renderer
                    .render_to_rgba8(
                        &scene,
                        text.atlas_mut(),
                        (720 * scale, 640 * scale),
                        scale as f32,
                        palette.color("surface-1").unwrap(),
                    )
                    .unwrap();
                if let Some(p) = &output {
                    let path = p.join(format!("dialog-{name}-theme-{theme_index}@{scale}x.png"));
                    let mut e = png::Encoder::new(
                        std::fs::File::create(path).unwrap(),
                        720 * scale,
                        640 * scale,
                    );
                    e.set_color(png::ColorType::Rgba);
                    e.set_depth(png::BitDepth::Eight);
                    e.write_header().unwrap().write_image_data(&rgba).unwrap();
                }
            }
        }
    }
}

#[test]
fn rejected_tree_removal_keeps_rich_dialog_pixels_and_input() {
    use std::time::Duration;
    use tekes_ui_composition_consumer::scenarios::{self, ids};
    use tekes_ui_controls::composition::{CompositionError, DialogContent};
    let mut renderer = Renderer::new_headless().expect("real GPU required");
    let output = std::env::var_os("TEKESUI_REMOVAL_OUT").map(std::path::PathBuf::from);
    if let Some(p) = &output {
        std::fs::create_dir_all(p).unwrap();
    }
    let mut count = 0;
    for (theme_index, theme) in Theme::ALL.into_iter().enumerate() {
        for scale in [1_u32, 2] {
            for width in [640_u32, 960] {
                let palette = Palette::from(theme);
                let mut host = scenarios::rich_dialog().unwrap();
                let mut text = TextEngine::new();
                text.set_scale(scale as f32);
                text.set_motion_enabled(true);
                text.set_motion_context(Duration::ZERO, true);
                let frame = Rect::new(0., 0., width as f32, 520.);
                host.show_dialog(ids::DIALOG).unwrap();
                let mut baseline = None;
                for stage in ["before", "rejected-removal"] {
                    if stage == "rejected-removal" {
                        assert!(matches!(
                            host.remove(ids::DIALOG_SCREEN),
                            Err(CompositionError::MissingId(_))
                        ));
                    }
                    text.begin_frame();
                    let mut scene = Scene::new();
                    host.render(&mut scene, &mut text, frame, frame, palette)
                        .unwrap();
                    assert!(
                        host.get(ids::DIALOG)
                            .unwrap()
                            .component::<DialogContent>()
                            .unwrap()
                            .is_open()
                    );
                    assert!(host.semantics().iter().any(|n| n.id == ids::DIALOG));
                    let pixels = renderer
                        .render_to_rgba8(
                            &scene,
                            text.atlas_mut(),
                            (width * scale, 520 * scale),
                            scale as f32,
                            palette.color("surface-0").unwrap(),
                        )
                        .unwrap();
                    if let Some(expected) = &baseline {
                        assert_eq!(
                            &pixels, expected,
                            "rejected mutation must preserve the live modal presentation"
                        );
                    } else {
                        baseline = Some(pixels.clone());
                    }
                    if let Some(output) = &output {
                        let file = std::fs::File::create(
                            output
                                .join(format!("dialog-{theme_index}-{width}-{stage}@{scale}x.png")),
                        )
                        .unwrap();
                        let mut encoder = png::Encoder::new(file, width * scale, 520 * scale);
                        encoder.set_color(png::ColorType::Rgba);
                        encoder.set_depth(png::BitDepth::Eight);
                        encoder
                            .write_header()
                            .unwrap()
                            .write_image_data(&pixels)
                            .unwrap();
                    }
                    count += 1;
                }
            }
        }
    }
    assert_eq!(count, 32);
}

#[test]
fn independent_navigation_content_statuses_render_real_text_and_gpu() {
    use super::{PageCtx, static_text::StaticText, workbench::content_status};
    use tekes_ui_composition_consumer::content_status::{EDITOR, FEEDBACK, HEADER, Kind};
    let mut renderer = Renderer::new_headless().expect("real GPU required");
    let output = std::env::var_os("TEKESUI_CONTENT_STATUS_OUT").map(std::path::PathBuf::from);
    if let Some(path) = &output {
        std::fs::create_dir_all(path).unwrap();
    }
    let mut cases = Vec::new();
    for kind in [Kind::Tabs, Kind::Disclosure] {
        for (status, focus, title) in content_status::CASES {
            for (theme_index, theme) in Theme::ALL.into_iter().enumerate() {
                for brand in 0..3 {
                    for (width, type_scale) in [(360u32, 1.5f32), (800, 1.)] {
                        for scale in [1u32, 2] {
                            let palette = Palette::new(theme, brand).unwrap();
                            let mut text = TextEngine::new();
                            text.set_scale(scale as f32);
                            text.set_type_scale(type_scale);
                            text.set_motion_enabled(true);
                            text.set_motion_context(std::time::Duration::ZERO, true);
                            text.begin_frame();
                            let mut static_text = StaticText::new(None);
                            let mut ctx = PageCtx {
                                theme,
                                text: &mut text,
                                static_text: &mut static_text,
                            };
                            let frame = Rect::new(24., 24., width as f32 - 48., 360.);
                            let clip = Rect::new(0., 0., width as f32, 420.);
                            let mut probe = Scene::new();
                            let mut scenario = content_status::draw_case(
                                &mut ctx, &mut probe, kind, status, focus, frame, clip, palette,
                            )
                            .unwrap();
                            let mut scene = Scene::new();
                            scenario
                                .host
                                .render(&mut scene, ctx.text, frame, clip, palette)
                                .unwrap();
                            scenario.draw_disclosure_icon(
                                &mut scene,
                                &mut super::pages::icons::GalleryIconRenderer::native(&mut ctx),
                                clip,
                                palette,
                            );
                            let semantics = scenario.host.semantics();
                            assert!(semantics.iter().any(|n| n.id == FEEDBACK));
                            assert!(!semantics.iter().any(|n| n.id == EDITOR));
                            assert_eq!(scenario.host.focused(), focus.then_some(HEADER));
                            assert!(
                                scene
                                    .primitives()
                                    .iter()
                                    .any(|p| matches!(p, tekes_ui_scene::Primitive::Glyphs(_))),
                                "actual shaped text must reach the renderer"
                            );
                            let rgba = renderer
                                .render_to_rgba8(
                                    &scene,
                                    text.atlas_mut(),
                                    (width * scale, 420 * scale),
                                    scale as f32,
                                    palette.color("surface-1").unwrap(),
                                )
                                .unwrap();
                            let name = format!(
                                "{kind:?}-{status:?}-focus-{focus}-theme-{theme_index}-brand-{brand}-{width}@{scale}x-type-{type_scale}.png"
                            );
                            if let Some(path) = &output {
                                let file = std::fs::File::create(path.join(&name)).unwrap();
                                let mut enc = png::Encoder::new(file, width * scale, 420 * scale);
                                enc.set_color(png::ColorType::Rgba);
                                enc.set_depth(png::BitDepth::Eight);
                                enc.write_header().unwrap().write_image_data(&rgba).unwrap();
                            }
                            cases.push(serde_json::json!({"file":name,"kind":format!("{kind:?}"),"status":format!("{status:?}"),"header_focus":focus,"theme":theme_index,"brand":brand,"width":width,"scale":scale,"type_scale":type_scale,"title":title}));
                        }
                    }
                }
            }
        }
    }
    assert_eq!(cases.len(), 480);
    if let Some(path) = output {
        std::fs::write(
            path.join("manifest.json"),
            serde_json::to_vec_pretty(&cases).unwrap(),
        )
        .unwrap();
    }
}
