#![cfg(target_os = "macos")]
#[path = "support/coretext.rs"]
mod coretext;
use tekes_ui_controls::{TextSurface, TextSurfaceHost, composition::*};
use tekes_ui_scene::{Primitive, Quad, Rect, Scene};
use tekes_ui_tokens::{Palette, Rgba, Theme, proposal};
fn contrast(a: Rgba, b: Rgba) -> f32 {
    let l = |c: Rgba| {
        let c = c.to_linear();
        0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
    };
    let (a, b) = (l(a), l(b));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}
fn assert_input(scene: &Scene, frame: Rect, palette: Palette, backing: f32) -> usize {
    let (index, ground) = scene
        .primitives()
        .iter()
        .enumerate()
        .rev()
        .find_map(|(i, p)| match p {
            Primitive::Quad(q) if q.rect == frame => Some((i, *q)),
            _ => None,
        })
        .expect("actual original TextArea input ground");
    assert_eq!(ground.fill, palette.color("transcript-input").unwrap());
    assert_eq!(
        ground.corner_radii,
        tekes_ui_scene::CornerRadii::uniform(proposal::metric("radius", "radius-input").unwrap())
    );
    assert_eq!(
        ground.border_color,
        palette
            .color(proposal::control_boundary("required-input", palette.theme).unwrap())
            .unwrap()
    );
    let inks: Vec<_> = scene.primitives()[index + 1..]
        .iter()
        .filter_map(|p| match p {
            Primitive::Glyphs(run)
                if run.glyphs.iter().any(|g| {
                    let a = g.glyph;
                    frame
                        .intersection(&Rect::new(
                            g.origin.x + a.left as f32 / backing,
                            g.origin.y + a.top as f32 / backing,
                            a.width as f32 / backing,
                            a.height as f32 / backing,
                        ))
                        .is_some()
                }) =>
            {
                Some(run.color)
            }
            _ => None,
        })
        .collect();
    assert!(
        !inks.is_empty(),
        "real CoreText multiline glyphs after own input ground"
    );
    let first_glyph = scene.primitives()[index + 1..]
        .iter()
        .position(|p| matches!(p, Primitive::Glyphs(run) if !run.glyphs.is_empty()))
        .unwrap()
        + index
        + 1;
    let overlays: Vec<_> = scene.primitives()[index + 1..first_glyph]
        .iter()
        .filter_map(|p| match p {
            Primitive::Quad(q) if q.fill.a > 0. && q.rect.intersection(&frame).is_some() => {
                Some(q.fill)
            }
            _ => None,
        })
        .collect();
    for overlay in &overlays {
        assert_eq!(*overlay, palette.color("text-selection").unwrap());
    }
    let primary = palette.color("primary-text").unwrap();
    let selected = palette.color("selected-text").unwrap();
    let minimum = if palette.theme.is_increased() {
        7.
    } else {
        4.5
    };
    assert!(inks.contains(&primary));
    for ink in inks {
        assert!(
            ink == primary || ink == selected,
            "actual multiline ink uses its declared role"
        );
        if ink == primary {
            assert!(
                contrast(ink, ground.fill) >= minimum,
                "actual ordinary text/input ground contrast"
            );
        }
        if ink == selected {
            // Equal role colors do not establish selection visibility. Check both grounds when visible.
            assert!(
                ink == primary || !overlays.is_empty(),
                "distinct selected ink requires visible selection paint"
            );
            for overlay in &overlays {
                let adjacent = Rgba::new(
                    overlay.r * overlay.a + ground.fill.r * (1. - overlay.a),
                    overlay.g * overlay.a + ground.fill.g * (1. - overlay.a),
                    overlay.b * overlay.a + ground.fill.b * (1. - overlay.a),
                    1.,
                );
                assert!(
                    contrast(ink, adjacent) >= minimum,
                    "actual selected text/composited own ground contrast"
                );
            }
        }
    }
    overlays.len()
}
fn has_ring(scene: &Scene, frame: Rect, palette: Palette) -> bool {
    scene.primitives().iter().any(|p|matches!(p,Primitive::Quad(q)if q.fill.a==0. && q.border_width>0.
        && q.rect.center()==frame.center() && q.border_color==palette.color("border-focused").unwrap()))
}

fn assert_thumb(scene: &Scene, frame: Rect, palette: Palette) -> Rect {
    let ground_index = scene
        .primitives()
        .iter()
        .rposition(|p| matches!(p, Primitive::Quad(q) if q.rect == frame))
        .expect("own multiline input ground");
    let thumb = scene.primitives()[ground_index + 1..]
        .iter()
        .rev()
        .find_map(|p| match p {
            Primitive::Quad(q)
                if (q.rect.size.width - proposal::SCROLLBAR_WIDTH).abs() < 0.001
                    && (q.rect.min_x()
                        - (frame.max_x()
                            - proposal::SCROLLBAR_WIDTH
                            - proposal::SCROLLBAR_INSET))
                        .abs()
                        < 0.001
                    && q.rect.min_y() >= frame.min_y()
                    && q.rect.max_y() <= frame.max_y() =>
            {
                Some(*q)
            }
            _ => None,
        })
        .expect("actual multiline viewport scrollbar thumb");
    assert_eq!(thumb.fill, palette.color("secondary-text").unwrap());
    assert!(contrast(thumb.fill, palette.color("transcript-input").unwrap()) >= 3.);
    thumb.rect
}
fn source() -> String {
    (0..24)
        .map(|i| {
            format!("Area line {i}: editable multiline content 中文 stays in the original input.\n")
        })
        .collect()
}
fn render(surface: &mut TextSurface, text: &mut coretext::Text, theme: Theme) -> Scene {
    let role = surface.role;
    let scale = text.scale;
    let backing = text.backing;
    surface.layout_with(
        Rect::new(20., 30., 420., 160.),
        Some(Rect::new(0., 0., 540., 260.)),
        backing,
        scale,
        |source, width| TextSurfaceHost::shape_surface(text, source, role, width),
    );
    let mut scene = Scene::new();
    surface.draw(text, &mut scene, theme, true, false);
    scene
}
#[test]
fn text_area_primary_retained_themes_keep_multiline_selection_scroll_and_real_ink() {
    let source = source();
    let mut area = TextSurface::new(&source, true, false);
    let mut text = coretext::Text::new(1.);
    assert!(area.input.multiline);
    assert_eq!(area.role, proposal::TextRole::ComposerBody);
    render(&mut area, &mut text, Theme::LIGHT);
    assert!(area.input.select_range(0..4));
    let frame = area.outer;
    let inner = area.inner;
    for theme in Theme::ALL {
        let scene = render(&mut area, &mut text, theme);
        assert_eq!(area.input.text, source);
        assert_eq!(area.input.selection(), 0..4);
        assert_eq!(area.input.selected_text(), "Area");
        assert_eq!((area.outer, area.inner), (frame, inner));
        assert!(
            assert_input(&scene, frame, theme.into(), text.backing) > 0,
            "first visible selection phase must paint selection"
        );
        assert!(scene.primitives().iter().any(|p|matches!(p,Primitive::Glyphs(run) if !run.glyphs.is_empty() && run.color==Palette::from(theme).color("selected-text").unwrap())));
        assert!(has_ring(&scene, frame, theme.into()));
        assert_thumb(&scene, frame, theme.into());
    }
    assert!(area.scroll.scroll_by(0., 96.));
    let offset = area.scroll.offset;
    assert!(offset.y > 0.);
    for theme in Theme::ALL {
        let scene = render(&mut area, &mut text, theme);
        assert_eq!(area.scroll.offset, offset);
        assert_eq!(area.input.selection(), 0..4);
        assert_eq!(area.input.text, source);
        assert_input(&scene, frame, theme.into(), text.backing);
    }
    area.input.select(area.input.len(), false);
    assert_eq!(area.command("insertNewline:"), Some(true));
    assert!(area.input.text.ends_with("\n\n"));
}
fn render_host(host: &mut ContentHost, text: &mut coretext::Text, theme: Theme) -> Scene {
    let mut scene = Scene::new();
    let palette: Palette = theme.into();
    scene.push_quad(Quad::new(
        Rect::new(0., 0., 640., 360.),
        palette.color("document-canvas").unwrap(),
    ));
    host.render(
        &mut scene,
        text,
        Rect::new(20., 30., 500., 240.),
        Rect::new(0., 0., 640., 360.),
        palette,
    )
    .unwrap();
    scene
}
#[test]
fn text_area_original_content_configuration_retains_multiline_input_and_theme_scroll() {
    let source = source();
    let mut field = TextFieldContent::new("Project notes", &source);
    field.surface = TextSurface::new(&source, true, false);
    field.viewport = TextViewport::Fixed(160.);
    let mut host = ContentHost::new(Content::new(ComponentId(1), field)).unwrap();
    let mut text = coretext::Text::new(1.);
    text.services = true;
    render_host(&mut host, &mut text, Theme::LIGHT);
    host.focus(Some(ComponentId(1)), true);
    let field = host
        .get_mut(ComponentId(1))
        .unwrap()
        .component_mut::<TextFieldContent>()
        .unwrap();
    assert!(field.surface.input.select_range(0..4));
    assert!(field.surface.scroll.scroll_by(0., 80.));
    let offset = field.surface.scroll.offset;
    let frame = field.surface.outer;
    for theme in Theme::ALL {
        let scene = render_host(&mut host, &mut text, theme);
        let field = host
            .get(ComponentId(1))
            .unwrap()
            .component::<TextFieldContent>()
            .unwrap();
        assert!(field.surface.input.multiline);
        assert_eq!(field.surface.role, proposal::TextRole::ComposerBody);
        assert_eq!(field.surface.outer, frame);
        assert_eq!(field.surface.scroll.offset, offset);
        assert_eq!(field.surface.input.selection(), 0..4);
        assert_eq!(field.surface.input.text, source);
        assert_input(&scene, frame, theme.into(), text.backing);
        assert!(has_ring(&scene, frame, theme.into()));
        assert_eq!(
            host.semantics()[0].semantics.value.as_deref(),
            Some(source.as_str())
        );
    }
    assert!(
        host.event(&ContentEvent::Commit {
            text: "Edited\nnotes".into(),
            replacement: Some(0..source.encode_utf16().count())
        })
        .changed
    );
    render_host(&mut host, &mut text, Theme::DARK_INCREASED);
    assert_eq!(
        host.semantics()[0].semantics.value.as_deref(),
        Some("Edited\nnotes")
    );
}
