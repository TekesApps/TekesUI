//! #121: retained SurfaceView canvas/panel branches with real public text content.
#![cfg(target_os = "macos")]
#[path = "support/coretext.rs"]
mod coretext;
use tekes_ui_controls::{
    composition::{ComponentId, Content, ContentHost, TextContent},
    feedback_view::SurfaceView,
};
use tekes_ui_scene::{CornerRadii, Primitive, Rect, Scene};
use tekes_ui_tokens::{Palette, Rgba, Theme, proposal};
fn contrast(a: Rgba, b: Rgba) -> f32 {
    let luminance = |c: Rgba| {
        let v = c.to_linear();
        0.2126 * v[0] + 0.7152 * v[1] + 0.0722 * v[2]
    };
    let (a, b) = (luminance(a), luminance(b));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}
#[test]
fn surface_retained_themes_keep_canvas_panel_geometry_and_readable_real_content() {
    let mut view = SurfaceView::default();
    let mut host = ContentHost::new(Content::new(
        ComponentId(121_004),
        TextContent::new("Account settings"),
    ))
    .unwrap();
    let mut text = coretext::Text::new(1.);
    text.services = true;
    let frame = Rect::new(16., 20., 360., 120.);
    let clip = Rect::new(32., 10., 320., 150.);
    let mut fills = Vec::new();
    // Shared Palette brands, #62 inheritance/default/override and scale fallback are reused.
    for theme in Theme::ALL {
        let palette = Palette::new(theme, 0).unwrap();
        for panel in [false, true] {
            let mut scene = Scene::new();
            let layout = view.render_at_scale(&mut scene, frame, Some(clip), palette, panel, 2.);
            assert_eq!(layout.frame, frame);
            assert_eq!(
                layout.content,
                frame.inset(proposal::metric("spacing", "space-loose").unwrap())
            );
            assert_eq!(layout.visible, frame.intersection(&clip));
            assert_eq!(view.layout().content, layout.content);
            let background = scene
                .primitives()
                .iter()
                .find_map(|p| match p {
                    Primitive::Quad(q) if q.rect == frame => Some(*q),
                    _ => None,
                })
                .expect("production Surface background");
            assert_eq!(
                background.fill,
                palette
                    .color(if panel { "surface-panel" } else { "surface-1" })
                    .unwrap()
            );
            assert_eq!(
                background.corner_radii,
                CornerRadii::uniform(proposal::metric("radius", "radius-panel").unwrap())
            );
            assert_eq!(
                background.border_width,
                if panel {
                    if theme.is_increased() {
                        proposal::metric("stroke", "stroke-rule").unwrap()
                    } else {
                        0.5
                    }
                } else {
                    0.
                }
            );
            if panel {
                assert_eq!(background.border_color, palette.color("separator").unwrap());
            }
            assert!(
                scene
                    .primitives()
                    .iter()
                    .any(|p| matches!(p, Primitive::PushClip(r) if *r == clip)),
                "ancestor viewport bounds decorative paint"
            );
            host.render(&mut scene, &mut text, layout.content, clip, palette)
                .unwrap();
            let inks: Vec<_> = scene
                .primitives()
                .iter()
                .filter_map(|p| match p {
                    Primitive::Glyphs(run) if !run.glyphs.is_empty() => Some(run.color),
                    _ => None,
                })
                .collect();
            assert!(
                !inks.is_empty(),
                "public TextContent emits actual CoreText glyphs inside Surface content"
            );
            for ink in inks {
                assert_eq!(ink, palette.color("primary-text").unwrap());
                assert!(
                    contrast(ink, background.fill) >= if theme.is_increased() { 7. } else { 4.5 },
                    "{theme:?} panel={panel}: {}",
                    contrast(ink, background.fill)
                );
            }
            fills.push(background.fill);
        }
    }
    assert!(
        fills.iter().step_by(2).any(|fill| *fill != fills[0]),
        "same View consumes changed Theme surfaces"
    );
}
