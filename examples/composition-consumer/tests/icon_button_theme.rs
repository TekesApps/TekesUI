//! Same registered resource and original Button across Theme changes.
#![cfg(target_os = "macos")]
#[path = "support/coretext.rs"]
mod coretext;
use tekes_ui_controls::{Button, ControlKey, composition::*};
use tekes_ui_scene::{ImageQuad, Primitive, Quad, Rect, Scene};
use tekes_ui_tokens::{Rgba, Theme, proposal};

fn artwork(ink: Rgba, side: u32) -> Vec<u8> {
    let glyph = tekes_ui_icons::Glyph::ALL
        .iter()
        .find(|g| g.name == "tekes-settings")
        .expect("same approved Regular SVG as unified_button_gpu");
    let channel = |v: f32| (v.clamp(0., 1.) * 255.).round() as u8;
    let options = resvg::usvg::Options {
        style_sheet: Some(format!(
            "svg {{ color: rgb({}, {}, {}); }}",
            channel(ink.r),
            channel(ink.g),
            channel(ink.b)
        )),
        ..Default::default()
    };
    let tree = resvg::usvg::Tree::from_str(glyph.svg, &options).unwrap();
    let mut pixels = resvg::tiny_skia::Pixmap::new(side, side).unwrap();
    let factor = side as f32 / tree.size().width();
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(factor, factor),
        &mut pixels.as_mut(),
    );
    pixels.data().to_vec()
}
fn draw(host: &mut ContentHost, text: &mut coretext::Text, theme: Theme) -> Scene {
    let mut scene = Scene::new();
    host.render(
        &mut scene,
        text,
        Rect::new(20., 20., 100., 64.),
        Rect::new(0., 0., 160., 110.),
        theme.into(),
    )
    .unwrap();
    scene
}
fn sample(scene: &Scene, frame: Rect) -> (usize, Quad, ImageQuad) {
    let (index, ground) = scene
        .primitives()
        .iter()
        .enumerate()
        .find_map(|(i, p)| match p {
            Primitive::Quad(q) if q.rect == frame => Some((i, *q)),
            _ => None,
        })
        .expect("original image Button's own background");
    let images: Vec<_> = scene.primitives()[index + 1..]
        .iter()
        .filter_map(|p| match p {
            Primitive::Image(q) if frame.intersection(&q.rect) == Some(q.rect) => Some(*q),
            _ => None,
        })
        .collect();
    assert_eq!(
        images.len(),
        1,
        "one actual resource after its own Button ground"
    );
    (index, ground, images[0])
}
#[test]
fn icon_button_same_registered_image_repaints_themes_and_keeps_unavailable_contracts() {
    let mut renderer =
        tekes_ui_render::Renderer::new_headless().expect("actual registered image renderer");
    let side = (proposal::metric("size", "size-icon-standard").unwrap() * 2.).ceil() as u32;
    let initial = artwork(
        tekes_ui_tokens::Palette::from(Theme::LIGHT)
            .color("primary-text")
            .unwrap(),
        side,
    );
    let image = renderer.register_image(&initial, side, side).unwrap();
    let mut host = ContentHost::new(Content::new(
        ComponentId(1),
        Button::new(image).with_accessible_name("Settings"),
    ))
    .unwrap();
    let mut text = coretext::Text::new(1.);
    text.services = true;
    let mut original_geometry = None;
    let mut grounds = Vec::new();
    for theme in Theme::ALL {
        let palette: tekes_ui_tokens::Palette = theme.into();
        // Artwork is caller-owned; update the registered resource without rebuilding Button or ID.
        let pixels = artwork(palette.color("primary-text").unwrap(), side);
        renderer.update_image(image, &pixels).unwrap();
        let scene = draw(&mut host, &mut text, theme);
        assert_eq!(host.semantics().len(), 1);
        assert_eq!(host.semantics()[0].semantics.name, "Settings");
        let frame = host.semantics()[0].frame;
        let (_, ground, actual) = sample(&scene, frame);
        assert_eq!(ground.fill, palette.color("surface-2").unwrap());
        assert_eq!(actual.image, image);
        assert_eq!(actual.opacity, 1.);
        if let Some(geometry) = original_geometry {
            assert_eq!((frame, actual.rect), geometry);
        } else {
            original_geometry = Some((frame, actual.rect));
        }
        grounds.push(ground.fill);
        let source = pixels
            .chunks_exact(4)
            .find(|p| p[3] == 255)
            .expect("opaque SVG stroke pixel");
        let ink = Rgba::rgb8(source[0], source[1], source[2]);
        let luminance = |c: Rgba| {
            let c = c.to_linear();
            0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
        };
        let (a, b) = (luminance(ink), luminance(ground.fill));
        assert!(
            (a.max(b) + 0.05) / (a.min(b) + 0.05) >= 3.,
            "actual source ink versus emitted Button ground"
        );
        let mut bare = Scene::new();
        for primitive in scene.primitives() {
            if !matches!(primitive, Primitive::Image(q) if q.image==image) {
                bare.push(primitive.clone());
            }
        }
        let clear = palette.color("surface-1").unwrap();
        let rgba = renderer
            .render_to_rgba8(&scene, &mut text.atlas, (320, 220), 2., clear)
            .unwrap();
        let no_image = renderer
            .render_to_rgba8(&bare, &mut text.atlas, (320, 220), 2., clear)
            .unwrap();
        let mut changed = 0;
        for y in (actual.rect.min_y() * 2.) as usize..(actual.rect.max_y() * 2.) as usize {
            for x in (actual.rect.min_x() * 2.) as usize..(actual.rect.max_x() * 2.) as usize {
                let i = (y * 320 + x) * 4;
                changed += usize::from(rgba[i..i + 4] != no_image[i..i + 4]);
            }
        }
        assert!(
            changed > 0,
            "registered image really paints over its own ground"
        );
    }
    assert_ne!(grounds[0], grounds[1]);
    assert_ne!(grounds[2], grounds[3]);
    host.focus(Some(ComponentId(1)), true);
    let scene = draw(&mut host, &mut text, Theme::DARK_INCREASED);
    assert!(
        scene
            .primitives()
            .iter()
            .any(|p| matches!(p,Primitive::Quad(q) if q.fill.a==0. && q.border_width>0.))
    );
    let point = host.semantics()[0].frame.center();
    assert!(
        host.event(&ContentEvent::PointerDown(point))
            .capture_pointer
    );
    host.get_mut(ComponentId(1))
        .unwrap()
        .component_mut::<Button>()
        .unwrap()
        .button
        .state
        .disabled = true;
    let scene = draw(&mut host, &mut text, Theme::DARK_INCREASED);
    let (_, _, actual) = sample(&scene, host.semantics()[0].frame);
    assert_eq!(actual.image, image);
    assert_eq!(
        actual.opacity,
        proposal::metric("opacity", "alpha-wash").unwrap()
    );
    assert!(!host.event(&ContentEvent::PointerUp(point)).activated);
    assert!(
        !host
            .accessibility_action(ComponentId(1), None, AccessibilityAction::Press)
            .activated
    );
    assert!(
        !scene
            .primitives()
            .iter()
            .any(|p| matches!(p,Primitive::Quad(q) if q.fill.a==0. && q.border_width>0.))
    );
    let button = host
        .get_mut(ComponentId(1))
        .unwrap()
        .component_mut::<Button>()
        .unwrap();
    button.button.state.disabled = false;
    button.button.state.loading = true;
    draw(&mut host, &mut text, Theme::DARK_INCREASED);
    assert!(
        !host
            .event(&ContentEvent::Key {
                key: ControlKey::Enter,
                down: true
            })
            .activated
    );
    assert!(
        !host
            .accessibility_action(ComponentId(1), None, AccessibilityAction::Press)
            .activated
    );
    assert_eq!(
        host.get(ComponentId(1))
            .unwrap()
            .component::<Button>()
            .unwrap()
            .activations,
        0
    );
    assert!(renderer.unregister_image(image));
}
