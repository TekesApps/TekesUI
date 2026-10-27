#![cfg(target_os = "macos")]
#[path = "support/coretext.rs"]
mod coretext;
use tekes_ui_controls::{TextSurface, TextSurfaceHost, TextSurfacePresentation, composition::*};
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
fn assert_readonly_scene(scene: &Scene, surface: &TextSurface, palette: Palette, backing: f32) {
    let (index, ground) = scene
        .primitives()
        .iter()
        .enumerate()
        .rev()
        .find_map(|(i, p)| match p {
            Primitive::Quad(q) if q.rect == surface.outer => Some((i, *q)),
            _ => None,
        })
        .expect("actual readonly TextSurface own Field ground");
    assert_eq!(ground.fill, palette.color("surface-1").unwrap());
    assert_eq!(ground.fill.a, 1.);
    assert_eq!(
        ground.corner_radii,
        tekes_ui_scene::CornerRadii::uniform(proposal::metric("radius", "radius-input").unwrap())
    );
    let first_glyph = index
        + 1
        + scene.primitives()[index + 1..]
            .iter()
            .position(|p| matches!(p,Primitive::Glyphs(run)if !run.glyphs.is_empty()))
            .expect("real body glyphs after own ground");
    let selections: Vec<_> = scene.primitives()[index + 1..first_glyph]
        .iter()
        .filter_map(|p| match p {
            Primitive::Quad(q)
                if q.fill.a > 0.
                    && q.rect.min_x() >= surface.inner.min_x()
                    && q.rect.max_x() <= surface.inner.max_x()
                    && q.rect.min_y() >= surface.inner.min_y()
                    && q.rect.max_y() <= surface.inner.max_y() =>
            {
                Some(*q)
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        selections.len(),
        surface.selection_rects(surface.input.selection()).len()
    );
    assert!(!selections.is_empty());
    for q in &selections {
        assert_eq!(q.fill, palette.color("text-selection").unwrap());
    }
    // Existing non-Srgb target + premultiplied blend operates on encoded sRGB components.
    let overlay = selections[0].fill;
    let base = ground.fill;
    let selected_ground = Rgba::new(
        overlay.r * overlay.a + base.r * (1. - overlay.a),
        overlay.g * overlay.a + base.g * (1. - overlay.a),
        overlay.b * overlay.a + base.b * (1. - overlay.a),
        1.,
    );
    let inks: Vec<_> = scene.primitives()[index + 1..]
        .iter()
        .filter_map(|p| match p {
            Primitive::Glyphs(run)
                if run.glyphs.iter().any(|g| {
                    let a = g.glyph;
                    surface
                        .inner
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
    assert!(!inks.is_empty());
    let primary = palette.color("primary-text").unwrap();
    let selected = palette.color("selected-text").unwrap();
    assert!(inks.contains(&primary));
    assert!(inks.contains(&selected));
    for ink in inks {
        assert!(
            ink == primary || ink == selected,
            "unexpected actual TextSurface glyph ink"
        );
        let adjacent = if ink == selected {
            selected_ground
        } else {
            ground.fill
        };
        assert!(
            contrast(ink, adjacent)
                >= if palette.theme.is_increased() {
                    7.
                } else {
                    4.5
                },
            "actual text/own ground or composited selection contrast"
        );
    }
}

fn render(surface: &mut TextSurface, text: &mut coretext::Text, theme: Theme, width: f32) -> Scene {
    let role = surface.role;
    let backing = text.backing;
    let scale = text.scale;
    surface.layout_with(
        Rect::new(20., 30., width, 180.),
        Some(Rect::new(0., 0., 600., 300.)),
        backing,
        scale,
        |source, width| text.shape_surface(source, role, width),
    );
    let mut scene = Scene::new();
    surface.draw(text, &mut scene, theme, true, false);
    scene
}
#[test]
fn text_surface_retained_themes_preserve_readonly_source_selection_copy_and_real_ink() {
    let source = (0..16)
        .map(|i| format!("Read-only line {i}: 中文 and selectable content remains caller-owned.\n"))
        .collect::<String>();
    let mut surface = TextSurface::new(&source, true, true);
    let mut text = coretext::Text::new(1.);
    assert_eq!(surface.presentation, TextSurfacePresentation::Field);
    assert_eq!(surface.role, proposal::TextRole::ComposerBody);
    render(&mut surface, &mut text, Theme::LIGHT, 420.);
    assert!(surface.input.select_range(0..4));
    let mut initial = None;
    for theme in Theme::ALL {
        let scene = render(&mut surface, &mut text, theme, 420.);
        let palette: Palette = theme.into();
        assert_eq!(surface.input.text, source);
        assert!(surface.input.read_only);
        assert_eq!(surface.input.selection(), 0..4);
        assert_eq!(surface.input.selected_text(), "Read");
        if let Some(before) = initial {
            assert_eq!((surface.outer, surface.inner), before);
        } else {
            initial = Some((surface.outer, surface.inner));
        }
        assert_readonly_scene(&scene, &surface, palette, text.backing);
        assert!(scene.primitives().iter().any(|p|matches!(p,Primitive::Quad(q)if q.fill.a==0. && q.border_width>0.
            && q.rect.center()==surface.outer.center() && q.border_color==palette.color("border-focused").unwrap())));
    }
    let before = surface.paragraph.as_ref().unwrap().height;
    text.scale = 1.5;
    let scene = render(&mut surface, &mut text, Theme::DARK_INCREASED, 260.);
    assert!(surface.paragraph.as_ref().unwrap().height > before);
    assert_eq!(surface.role, proposal::TextRole::ComposerBody);
    assert_eq!(surface.type_scale, 1.5);
    assert_eq!(surface.input.selection(), 0..4);
    assert_eq!(surface.input.selected_text(), "Read");
    assert_readonly_scene(&scene, &surface, Theme::DARK_INCREASED.into(), text.backing);
    assert!(!surface.input.insert("forbidden", None));
    assert_eq!(surface.command("deleteBackward:"), Some(false));
    assert_eq!(surface.input.text, source);
    assert!(surface.scroll.scroll_by(0., 100.));
    assert!(surface.scroll.offset.y > 0.);
    surface.cancel_interaction();
    assert_eq!(surface.input.selected_text(), "Read");
    assert_eq!(surface.input.text, source);
}
#[test]
fn text_content_retained_themes_keep_semantic_text_role_and_actual_canvas_ink() {
    let source = "Read-only adapter · 中文 content";
    let mut host = ContentHost::new(
        Content::new(ComponentId(1), TextContent::new(source))
            .with_surface("canvas")
            .unwrap(),
    )
    .unwrap();
    let mut text = coretext::Text::new(1.);
    let mut frames = Vec::new();
    for theme in Theme::ALL {
        let palette: Palette = theme.into();
        let mut scene = Scene::new();
        let ground = palette.color("document-canvas").unwrap();
        scene.push_quad(Quad::new(Rect::new(0., 0., 600., 240.), ground));
        host.render(
            &mut scene,
            &mut text,
            Rect::new(20., 30., 500., 100.),
            Rect::new(0., 0., 600., 240.),
            palette,
        )
        .unwrap();
        let body = host
            .get(ComponentId(1))
            .unwrap()
            .component::<TextContent>()
            .unwrap();
        assert_eq!(body.text, source);
        assert_eq!(body.role, proposal::TextRole::UiBody);
        let semantics = host.semantics();
        assert_eq!(semantics.len(), 1);
        assert_eq!(semantics[0].semantics.role, ContentRole::Text);
        assert_eq!(semantics[0].semantics.name, source);
        frames.push(semantics[0].frame);
        assert!(host.focus_targets().is_empty());
        let runs: Vec<_> = scene
            .primitives()
            .iter()
            .filter_map(|p| match p {
                Primitive::Glyphs(run) if !run.glyphs.is_empty() => Some(run),
                _ => None,
            })
            .collect();
        assert!(!runs.is_empty());
        for run in runs {
            assert_eq!(run.color, palette.color("primary-text").unwrap());
            assert!(contrast(run.color, ground) >= if theme.is_increased() { 7. } else { 4.5 });
        }
    }
    assert!(frames.windows(2).all(|pair| pair[0] == pair[1]));
}
