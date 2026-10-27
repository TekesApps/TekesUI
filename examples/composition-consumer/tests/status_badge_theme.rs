//! #125: retained original passive badge; prior external/brand/GPU matrices are reused.
#![cfg(target_os = "macos")]
#[path = "support/coretext.rs"]
mod coretext;
use tekes_ui_controls::{
    composition::{ComponentId, Content, ContentHost, ContentRole, StatusBadgeContent},
    extended::StatusBadge,
};
use tekes_ui_scene::{CornerRadii, Primitive, Quad, Rect, Scene};
use tekes_ui_tokens::{Palette, Rgba, Theme, surface_palette::resolve_surface};
fn contrast(a: Rgba, b: Rgba) -> f32 {
    let luminance = |c: Rgba| {
        let v = c.to_linear();
        0.2126 * v[0] + 0.7152 * v[1] + 0.0722 * v[2]
    };
    let (a, b) = (luminance(a), luminance(b));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}
#[test]
fn status_badge_retained_themes_preserve_tone_solid_label_contrast_and_passivity() {
    const BADGE: ComponentId = ComponentId(125_003);
    let mut host = ContentHost::new(Content::new(
        BADGE,
        StatusBadgeContent::new(StatusBadge::Info, "Info").unwrap(),
    ))
    .unwrap();
    let mut text = coretext::Text::new(1.);
    text.services = true;
    let frame = Rect::new(20.25, 24.5, 124., 28.);
    let clip = Rect::new(0., 0., 200., 90.);
    for (tone, usage, label) in [
        (StatusBadge::Info, "status-badge-info", "Info"),
        (StatusBadge::Running, "status-badge-running", "Running"),
        (
            StatusBadge::Succeeded,
            "status-badge-succeeded",
            "Succeeded",
        ),
        (StatusBadge::Failed, "status-badge-failed", "Failed"),
    ] {
        host.get_mut(BADGE)
            .unwrap()
            .component_mut::<StatusBadgeContent>()
            .unwrap()
            .update(tone, label, None, None, false)
            .unwrap();
        let mut fills = Vec::new();
        // Same ContentHost/Content/View throughout, with no brand/Surface Cartesian repeat.
        for theme in Theme::ALL {
            let palette = Palette::new(theme, 0).unwrap();
            let packet = resolve_surface(palette, "canvas", usage, "rest").unwrap();
            let ground = resolve_surface(palette, "canvas", "content", "rest")
                .unwrap()
                .background;
            text.atlas.begin_frame();
            let mut scene = Scene::new();
            scene.push_quad(Quad::new(clip, ground));
            host.render(&mut scene, &mut text, frame, clip, palette)
                .unwrap();
            let solid = scene
                .primitives()
                .iter()
                .find_map(|p| match p {
                    Primitive::Quad(q) if q.rect == frame => Some(*q),
                    _ => None,
                })
                .unwrap();
            assert_eq!(solid.fill, packet.background);
            assert_eq!(
                solid.corner_radii,
                CornerRadii::uniform(frame.size.height * 0.5)
            );
            assert_eq!(solid.border_width, 0.);
            let inks: Vec<_> = scene
                .primitives()
                .iter()
                .filter_map(|p| match p {
                    Primitive::Glyphs(run) if !run.glyphs.is_empty() => Some(run.color),
                    _ => None,
                })
                .collect();
            assert!(!inks.is_empty(), "real normal-label CoreText glyphs");
            for ink in inks {
                assert_eq!(ink, packet.text);
                assert_eq!(ink, palette.color("c-status-badge-foreground").unwrap());
                assert!(
                    contrast(ink, solid.fill) >= if theme.is_increased() { 7. } else { 4.5 },
                    "{theme:?} {tone:?} label contrast={}",
                    contrast(ink, solid.fill)
                );
            }
            assert!(
                contrast(solid.fill, ground) >= 3.,
                "{theme:?} {tone:?} necessary status fill/parent contrast={}",
                contrast(solid.fill, ground)
            );
            let badge = host
                .get(BADGE)
                .unwrap()
                .component::<StatusBadgeContent>()
                .unwrap();
            assert_eq!(badge.model(), tone);
            assert_eq!(badge.label(), label);
            assert_eq!(badge.layout().frame, frame);
            assert_eq!(badge.layout().visible, Some(frame));
            assert!(host.focus_targets().is_empty());
            let semantics = host.semantics();
            assert_eq!(semantics[0].semantics.role, ContentRole::Text);
            assert_eq!(semantics[0].semantics.name, label);
            assert!(!semantics[0].semantics.disabled);
            fills.push(solid.fill);
        }
        assert!(
            fills.windows(2).any(|w| w[0] != w[1]),
            "same retained {tone:?} consumes changed Theme fill"
        );
    }
}
