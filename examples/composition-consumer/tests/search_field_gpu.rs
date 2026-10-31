//! Explicit actual-GPU probe. No GPU acceptance until this ignored test is run
//! under the original GPU lock with #101's complete matched role dependency.
#![cfg(target_os = "macos")]
#[path = "support/coretext.rs"]
mod coretext;
use tekes_ui_controls::{TextSurface, composition::*};
use tekes_ui_scene::{Rect, Scene};
use tekes_ui_tokens::{Theme, surface_palette::resolve_surface};

fn configure(host: &mut ContentHost, state: &str) {
    let id = ComponentId(1);
    if state != "empty" {
        host.focus(Some(id), true);
    }
    match state {
        "selection" => {
            host.accessibility_action(id, None, AccessibilityAction::SetSelection(1..3));
        }
        "marked" => {
            host.event(&ContentEvent::Preedit {
                text: "拼音".into(),
                selection: 1..1,
                clauses: vec![0..1, 1..2],
                replacement: Some(1..3),
            });
        }
        _ => {}
    }
}

#[test]
#[ignore = "requires explicit GPU lock, #101, fresh output and real GPU"]
fn public_search_original_editor_pixels_and_surface_clip() {
    let output = std::path::PathBuf::from(
        std::env::var_os("TEKESUI_SEARCH_GPU_OUT").expect("fresh evidence directory required"),
    );
    assert!(!output.exists(), "never overwrite existing evidence");
    std::fs::create_dir_all(&output).unwrap();
    let mut gpu = tekes_ui_render::Renderer::new_headless().unwrap();
    let mut cases = vec![];
    for (theme_index, theme) in Theme::ALL.into_iter().enumerate() {
        for &surface in tekes_ui_tokens::surface_palette::surface_names() {
            for (zoom, backing, width, partial) in [
                (1., 1., 320., false),
                (1.5, 2., 320., false),
                (1.5, 2., 96., true),
            ] {
                for state in [
                    "empty",
                    "focused",
                    "selection",
                    "marked",
                    "readonly",
                    "error",
                    "disabled",
                    "clear-pressed",
                    "cleared",
                ] {
                    let value = if state == "empty" {
                        ""
                    } else {
                        "A😀B 中文 e\u{301}"
                    };
                    let mut search = SearchFieldContent::new("Search", value);
                    search.set_submit_policy(TextSubmitPolicy::Enter);
                    if state == "error" {
                        search.set_error(Some("Invalid query".into()));
                    }
                    search.set_availability(state == "readonly", state == "disabled");
                    let mut host = ContentHost::new(
                        Content::new(ComponentId(1), search)
                            .with_surface(surface)
                            .unwrap(),
                    )
                    .unwrap();
                    configure(&mut host, state);
                    let mut text = coretext::Text::new(zoom);
                    text.services = true;
                    text.backing = backing;
                    let frame = Rect::new(16., 16., width, 64.);
                    let clip = if partial {
                        Rect::new(20., 16., (width - 8.).max(0.), 48.)
                    } else {
                        Rect::new(0., 0., 352., 96.)
                    };
                    let selected_theme = std::env::var("TEKESUI_SEARCH_THEME_ID").ok();
                    let palette = selected_theme.as_ref().map_or_else(
                        || theme.into(),
                        |id| {
                            tekes_ui_tokens::Palette::for_theme_definition(theme, id)
                                .expect("explicit existing external Theme definition")
                        },
                    );
                    let mut scene = Scene::new();
                    host.render(&mut scene, &mut text, frame, clip, palette)
                        .unwrap();
                    if matches!(state, "clear-pressed" | "cleared") {
                        let clear = host.semantics()[0].parts[0].frame.center();
                        host.event(&ContentEvent::PointerDown(clear));
                        if state == "cleared" {
                            host.event(&ContentEvent::PointerUp(clear));
                        }
                        text.atlas.begin_frame();
                        scene = Scene::new();
                        host.render(&mut scene, &mut text, frame, clip, palette)
                            .unwrap();
                    }
                    let original = host.text_surface(ComponentId(1)).unwrap();
                    let editor = original.outer;
                    let mut reference = TextFieldContent::new("Search", &original.input.text);
                    reference.show_metadata = false;
                    if original.error {
                        reference.field.error = Some("Invalid query".into());
                    }
                    reference.surface =
                        TextSurface::new(&original.input.text, false, original.input.read_only);
                    reference.surface.input.disabled = original.input.disabled;
                    // Original transactions retain the exact direction/marked ranges.
                    if let Some(mark) = &original.input.marked {
                        let units: Vec<_> = original.input.text.encode_utf16().collect();
                        let preedit = String::from_utf16(&units[mark.clone()]).unwrap();
                        reference.surface.input.preedit(
                            &preedit,
                            original.input.selection().start.saturating_sub(mark.start)
                                ..original.input.selection().end.saturating_sub(mark.start),
                            Some(mark.clone()),
                        );
                        reference.surface.clauses = original.clauses.clone();
                    } else {
                        reference
                            .surface
                            .input
                            .select_range(original.input.selection());
                    }
                    if matches!(state, "selection" | "marked") {
                        reference.surface.reveal();
                    }
                    let mut ref_host = ContentHost::new(
                        Content::new(ComponentId(1), reference)
                            .with_surface(surface)
                            .unwrap(),
                    )
                    .unwrap();
                    if host.focused() == Some(ComponentId(1)) {
                        ref_host.focus(Some(ComponentId(1)), true);
                    }
                    let mut ref_text = coretext::Text::new(zoom);
                    ref_text.services = true;
                    ref_text.backing = backing;
                    let mut ref_scene = Scene::new();
                    ref_host
                        .render(&mut ref_scene, &mut ref_text, editor, clip, palette)
                        .unwrap();
                    let pixels_size = ((352. * backing) as u32, (96. * backing) as u32);
                    let bg = resolve_surface(palette, surface, "content", "rest")
                        .unwrap()
                        .background;
                    let pixels = gpu
                        .render_to_rgba8(&scene, &mut text.atlas, pixels_size, backing, bg)
                        .unwrap();
                    let reference_pixels = gpu
                        .render_to_rgba8(&ref_scene, &mut ref_text.atlas, pixels_size, backing, bg)
                        .unwrap();
                    let prefix =
                        format!("{theme_index}-{surface}-{zoom}-{backing}-{width}-{state}");
                    for (suffix, data) in [
                        ("search", pixels.clone()),
                        ("original-field", reference_pixels.clone()),
                    ] {
                        resvg::tiny_skia::Pixmap::from_vec(
                            data,
                            resvg::tiny_skia::IntSize::from_wh(pixels_size.0, pixels_size.1)
                                .unwrap(),
                        )
                        .unwrap()
                        .save_png(output.join(format!("{prefix}-{suffix}.png")))
                        .unwrap();
                    }
                    let reference_surface = ref_host.text_surface(ComponentId(1)).unwrap();
                    std::fs::write(output.join(format!("{prefix}-input.json")),serde_json::to_vec_pretty(&serde_json::json!({
                        "search":format!("outer={:?} inner={:?} origin={:?} scroll={:?} selection={:?} rects={:?}",original.outer,original.inner,original.origin(),original.scroll.offset,original.input.selection(),original.selection_rects(original.input.selection())),
                        "reference":format!("outer={:?} inner={:?} origin={:?} scroll={:?} selection={:?} rects={:?}",reference_surface.outer,reference_surface.inner,reference_surface.origin(),reference_surface.scroll.offset,reference_surface.input.selection(),reference_surface.selection_rects(reference_surface.input.selection()))
                    })).unwrap()).unwrap();
                    // Include the original field chrome and complete focus-ring outsets.
                    // Existing regular spacing separates the independent clear Button.
                    let outset = tekes_ui_tokens::proposal::metric("stroke", "c-focus-width")
                        .unwrap()
                        + tekes_ui_tokens::proposal::metric("spacing", "c-focus-offset").unwrap();
                    let crop = editor.inset(-outset).intersection(&clip);
                    let mut compared = 0;
                    if let Some(crop) = crop {
                        for y in ((crop.min_y() * backing).ceil() as usize)
                            ..((crop.max_y() * backing).floor() as usize)
                        {
                            for x in ((crop.min_x() * backing).ceil() as usize)
                                ..((crop.max_x() * backing).floor() as usize)
                            {
                                let at = (y * pixels_size.0 as usize + x) * 4;
                                assert_eq!(
                                    &pixels[at..at + 4],
                                    &reference_pixels[at..at + 4],
                                    "original editor pixels differ: {prefix} at {x},{y}"
                                );
                                compared += 1;
                            }
                        }
                    }
                    assert!(
                        compared > 0,
                        "case did not compare any actual editor pixels"
                    );

                    let mut clear_compared = 0usize;
                    // Positive full-clip cases compare the actual auxiliary Button
                    // against the original public Button, without Host focus transfer.
                    if !partial && !original.input.text.is_empty() {
                        let clear_frame = host.semantics()[0].parts[0].frame;
                        let mut clear = Button::new("×").with_accessible_name("Clear");
                        clear.button.state.disabled = original.input.read_only
                            || original.input.disabled
                            || original.input.marked.is_some();
                        let mut button_text = coretext::Text::new(zoom);
                        button_text.services = true;
                        button_text.backing = backing;
                        let button_theme =
                            ContentTheme::from(palette).with_surface(surface).unwrap();
                        let mut button_scene = Scene::new();
                        clear.measure(&mut button_text, clear_frame.size);
                        clear.arrange(&mut button_text, clear_frame, clip);
                        clear.paint(
                            &mut button_scene,
                            &mut button_text,
                            clear_frame,
                            clip,
                            button_theme,
                        );
                        if state == "clear-pressed" {
                            clear.event(&ContentEvent::PointerDown(clear_frame.center()));
                            button_text.atlas.begin_frame();
                            button_scene = Scene::new();
                            clear.paint(
                                &mut button_scene,
                                &mut button_text,
                                clear_frame,
                                clip,
                                button_theme,
                            );
                        }
                        let button_pixels = gpu
                            .render_to_rgba8(
                                &button_scene,
                                &mut button_text.atlas,
                                pixels_size,
                                backing,
                                bg,
                            )
                            .unwrap();
                        for y in ((clear_frame.min_y() * backing).ceil() as usize)
                            ..((clear_frame.max_y() * backing).floor() as usize)
                        {
                            for x in ((clear_frame.min_x() * backing).ceil() as usize)
                                ..((clear_frame.max_x() * backing).floor() as usize)
                            {
                                let at = (y * pixels_size.0 as usize + x) * 4;
                                assert_eq!(
                                    &pixels[at..at + 4],
                                    &button_pixels[at..at + 4],
                                    "original clear Button pixels differ: {state}/{surface}"
                                );
                                clear_compared += 1;
                            }
                        }
                        resvg::tiny_skia::Pixmap::from_vec(
                            button_pixels,
                            resvg::tiny_skia::IntSize::from_wh(pixels_size.0, pixels_size.1)
                                .unwrap(),
                        )
                        .unwrap()
                        .save_png(output.join(format!("{prefix}-original-clear.png")))
                        .unwrap();
                    }

                    cases.push(serde_json::json!({"prefix":prefix,"state":state,"theme":theme_index,"surface":surface,"zoom":zoom,"backing":backing,"width":width,"partial_clip":partial,"selected_theme":selected_theme,"palette_source":format!("{:?}",palette.source()),"compared_pixels":compared,"clear_compared_pixels":clear_compared,"query":host.get(ComponentId(1)).unwrap().component::<SearchFieldContent>().unwrap().query()}));
                }
            }
        }
    }
    std::fs::write(output.join("receipt.json"),serde_json::to_vec_pretty(&serde_json::json!({"scope":"independent public Search, original TextField with full focus-ring outsets and positive original Button pixels; Gallery TextDemo reference uses separate probe","cases":cases,"native":false,"physical_ime":false,"voiceover":false,"gold7":false})).unwrap()).unwrap();
}
