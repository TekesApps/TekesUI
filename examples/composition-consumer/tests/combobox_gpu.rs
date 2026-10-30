//! Same-source original editor + ComboBoxView versus public Content, real GPU.
#![cfg(target_os = "macos")]
#[path = "support/coretext.rs"]
mod coretext;
use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, ControlKey,
    composition::*,
    extended::ComboBox,
    extended::{Axis, Stack},
    layout::MeasuredChild,
    layout_view::ComboBoxView,
};
use tekes_ui_scene::{Primitive, Quad, Rect, Scene, Size};
use tekes_ui_tokens::{Palette, Theme, proposal, surface_palette::resolve_surface};
#[derive(Debug)]
struct Background;
impl Component for Background {
    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, a: Size) -> MeasuredChild {
        MeasuredChild::new(Size::new(a.width, 120.))
    }
    fn paint(
        &mut self,
        s: &mut Scene,
        t: &mut dyn ButtonTextRenderer,
        f: Rect,
        c: Rect,
        theme: ContentTheme,
    ) {
        let colors = resolve_surface(theme.palette, "panel", "content", "rest").unwrap();
        s.push_clip(c);
        s.push_quad(Quad::new(f, colors.background));
        t.draw(
            s,
            "Background sibling",
            proposal::TextRole::UiBody,
            f,
            ButtonTextAlign::Leading,
            colors.text,
        );
        s.pop_clip();
    }
}
fn configure(h: &mut ContentHost, state: &str) {
    let id = ComponentId(90);
    h.focus(Some(id), true);
    h.event(&ContentEvent::Commit {
        text: if state == "no-candidates" {
            "not present"
        } else {
            "Com"
        }
        .into(),
        replacement: None,
    });
    if state == "marked" {
        h.event(&ContentEvent::Preedit {
            text: "Com".into(),
            selection: 1..2,
            replacement: Some(0..3),
            clauses: vec![0..1, 1..3],
        });
    }
    if state == "selected" {
        h.event(&ContentEvent::Key {
            key: ControlKey::Enter,
            down: true,
        });
    }
}
#[test]
#[ignore = "explicit original review lock, fresh SSD output and actual GPU required"]
fn original_view_and_public_combobox_pixels() {
    let out =
        std::path::PathBuf::from(std::env::var_os("TEKESUI_COMBO_GPU_OUT").expect("fresh output"));
    assert!(!out.exists());
    std::fs::create_dir_all(&out).unwrap();
    let mut gpu = tekes_ui_render::Renderer::new_headless().unwrap();
    let mut cases = Vec::new();
    for (mode, theme) in [Theme::LIGHT, Theme::DARK].into_iter().enumerate() {
        let palette = std::env::var("TEKESUI_COMBO_THEME_ID").ok().map_or_else(
            || Palette::from(theme),
            |id| Palette::for_theme_definition(theme, &id).unwrap(),
        );
        for backing in [1., 2.] {
            for partial in [false, true] {
                for state in ["open", "marked", "selected", "no-candidates"] {
                    let width = if partial { 160. } else { 320. };
                    let frame = Rect::new(16., 16., width, 250.);
                    let clip = if partial {
                        Rect::new(24., 12., width - 24., 180.)
                    } else {
                        Rect::new(0., 0., 384., 300.)
                    };
                    let items = ["Automatic", "Comfortable", "Compact", "", "Compact"]
                        .map(str::to_string)
                        .to_vec();
                    let mut host = ContentHost::new(
                        Content::new(
                            ComponentId(89),
                            StackContent {
                                model: Stack {
                                    axis: Axis::Vertical,
                                    gap: 0.,
                                },
                                flexible_child: None,
                                children: vec![
                                    Content::new(
                                        ComponentId(90),
                                        ComboBoxContent::new("Layout", items),
                                    ),
                                    Content::new(ComponentId(91), Background),
                                ],
                            },
                        )
                        .with_surface("panel")
                        .unwrap(),
                    )
                    .unwrap();
                    let mut text = coretext::Text::new(1.);
                    text.services = true;
                    text.backing = backing;
                    text.reduced_motion = true;
                    host.render(&mut Scene::new(), &mut text, frame, clip, palette)
                        .unwrap();
                    configure(&mut host, state);
                    text.atlas.begin_frame();
                    let mut scene = Scene::new();
                    host.render(&mut scene, &mut text, frame, clip, palette)
                        .unwrap();
                    let original = host.text_surface(ComponentId(90)).unwrap();
                    let mut reference = TextFieldContent::new("Layout", &original.input.text);
                    if let Some(mark) = &original.input.marked {
                        let utf16 = original.input.text.encode_utf16().collect::<Vec<_>>();
                        reference.surface.input.preedit(
                            &String::from_utf16(&utf16[mark.clone()]).unwrap(),
                            original.input.selection().start - mark.start
                                ..original.input.selection().end - mark.start,
                            Some(mark.clone()),
                        );
                        reference.surface.clauses = original.clauses.clone();
                    } else {
                        reference
                            .surface
                            .input
                            .select_range(original.input.selection());
                    }
                    reference.set_focus(true, true);
                    let mut ref_text = coretext::Text::new(1.);
                    ref_text.services = true;
                    ref_text.backing = backing;
                    ref_text.reduced_motion = true;
                    let editor_frame = host.get(ComponentId(90)).unwrap().frame();
                    reference.arrange(&mut ref_text, editor_frame, clip);
                    reference.surface.reveal();
                    let mut ref_scene = Scene::new();
                    let ct = ContentTheme::from(palette).with_surface("panel").unwrap();
                    reference.paint(&mut ref_scene, &mut ref_text, editor_frame, clip, ct);
                    Background.paint(
                        &mut ref_scene,
                        &mut ref_text,
                        host.get(ComponentId(91)).unwrap().frame(),
                        clip,
                        ct,
                    );
                    let mut model: ComboBox = host
                        .get(ComponentId(90))
                        .unwrap()
                        .component::<ComboBoxContent>()
                        .unwrap()
                        .model()
                        .clone();
                    let mut view = ComboBoxView::default();
                    view.options.set_reduced_motion(true);
                    view.render(
                        &mut ref_scene,
                        &mut ref_text,
                        &mut model,
                        reference.surface.outer,
                        clip,
                        Some(clip),
                        palette,
                        proposal::TextRole::UiBody,
                        false,
                    );
                    let size = ((384. * backing) as u32, (300. * backing) as u32);
                    let bg = resolve_surface(palette, "panel", "content", "rest")
                        .unwrap()
                        .background;
                    let pixels = gpu
                        .render_to_rgba8(&scene, &mut text.atlas, size, backing, bg)
                        .unwrap();
                    let reference_pixels = gpu
                        .render_to_rgba8(&ref_scene, &mut ref_text.atlas, size, backing, bg)
                        .unwrap();
                    let mut ink_scene = Scene::new();
                    let mut glyph_runs = 0;
                    for p in scene.primitives() {
                        if matches!(
                            p,
                            Primitive::Glyphs(_) | Primitive::PushClip(_) | Primitive::PopClip
                        ) {
                            ink_scene.push(p.clone());
                        }
                        if matches!(p, Primitive::Glyphs(_)) {
                            glyph_runs += 1;
                        }
                    }
                    let ink = gpu
                        .render_to_rgba8(&ink_scene, &mut text.atlas, size, backing, bg)
                        .unwrap();
                    let empty = gpu
                        .render_to_rgba8(&Scene::new(), &mut text.atlas, size, backing, bg)
                        .unwrap();
                    let positive = ink
                        .chunks_exact(4)
                        .zip(empty.chunks_exact(4))
                        .filter(|(a, b)| a != b)
                        .count();
                    let prefix = format!("mode{mode}-scale{backing}-partial{partial}-{state}");
                    for (label, data) in [
                        ("content", pixels.clone()),
                        ("original", reference_pixels.clone()),
                        ("ink", ink.clone()),
                    ] {
                        resvg::tiny_skia::Pixmap::from_vec(
                            data,
                            resvg::tiny_skia::IntSize::from_wh(size.0, size.1).unwrap(),
                        )
                        .unwrap()
                        .save_png(out.join(format!("{prefix}-{label}.png")))
                        .unwrap();
                    }
                    let differences = pixels
                        .chunks_exact(4)
                        .zip(reference_pixels.chunks_exact(4))
                        .filter(|(a, b)| a != b)
                        .count();
                    let mut leaked = 0;
                    for y in 0..size.1 {
                        for x in 0..size.0 {
                            let p = tekes_ui_scene::Point::new(
                                (x as f32 + 0.5) / backing,
                                (y as f32 + 0.5) / backing,
                            );
                            let at = ((y * size.0 + x) * 4) as usize;
                            if !clip.contains(p) && ink[at..at + 4] != empty[at..at + 4] {
                                leaked += 1;
                            }
                        }
                    }
                    cases.push(serde_json::json!({"case":prefix,"source_model":format!("{model:?}"),"differences":differences,"positive_glyph_pixels":positive,"glyph_runs":glyph_runs,"outside_clip_ink":leaked}));
                    std::fs::write(
                        out.join("cases.json"),
                        serde_json::to_vec_pretty(&cases).unwrap(),
                    )
                    .unwrap();
                    assert!(
                        glyph_runs > 0 && positive > 0,
                        "missing positive original glyph ink {prefix}"
                    );
                    assert_eq!(leaked, 0, "parent clip leaked {prefix}");
                    assert_eq!(differences, 0, "original View/editor mismatch {prefix}");
                }
            }
        }
    }
}
