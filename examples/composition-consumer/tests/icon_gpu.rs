#![cfg(all(target_os = "macos", feature = "svg-icons"))]
#[path = "support/coretext.rs"]
mod coretext;
use tekes_ui_controls::{
    composition::*,
    extended::{Axis, Stack},
};
use tekes_ui_scene::{Primitive, Rect, Scene};
use tekes_ui_tokens::{Palette, Theme, surface_palette::resolve_surface};
fn host() -> ContentHost {
    ContentHost::new(Content::new(
        ComponentId(1),
        StackContent {
            model: Stack {
                axis: Axis::Vertical,
                gap: 8.,
            },
            flexible_child: None,
            children: vec![
                Content::new(ComponentId(2), IconContent::new("tekes-settings").unwrap()),
                Content::new(
                    ComponentId(3),
                    Button::new(Content::new(
                        ComponentId(4),
                        IconContent::new("tekes-back").unwrap(),
                    ))
                    .with_accessible_name("Back"),
                ),
            ],
        },
    ))
    .unwrap()
}
#[test]
fn canonical_svg_gpu_resources_reuse_retire_and_never_cross_renderer_namespace() {
    for backing in [1., 2.] {
        for declared_surface in [false, true] {
            let mut gpu = tekes_ui_render::Renderer::new_headless().unwrap();
            let mut cache = SvgIconCache::default();
            let mut host = host();
            host.get_mut(ComponentId(1))
                .unwrap()
                .set_surface(declared_surface.then_some("canvas"))
                .unwrap();
            let mut text = coretext::Text::new(1.);
            text.backing = backing;
            let f = Rect::new(0., 0., 160., 160.);
            let p = Palette::from(Theme::LIGHT);
            let ground = resolve_surface(p, "canvas", "content", "rest")
                .unwrap()
                .background;
            let mut draw = |host: &mut ContentHost,
                            gpu: &mut tekes_ui_render::Renderer,
                            cache: &mut SvgIconCache| {
                text.atlas.begin_frame();
                let mut scene = Scene::new();
                {
                    let mut icons = cache.renderer(gpu, backing);
                    host.render(
                        &mut scene,
                        &mut WithIconRenderer::new(&mut text, &mut icons),
                        f,
                        f,
                        p,
                    )
                    .unwrap();
                }
                let ids = scene
                    .primitives()
                    .iter()
                    .filter_map(|p| {
                        if let Primitive::Image(i) = p {
                            Some(i.image)
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>();
                let pixels = gpu
                    .render_to_rgba8(
                        &scene,
                        &mut text.atlas,
                        ((160. * backing) as u32, (160. * backing) as u32),
                        backing,
                        ground,
                    )
                    .unwrap();
                (ids, pixels)
            };
            let (first, rgba) = draw(&mut host, &mut gpu, &mut cache);
            assert_eq!(first.len(), 2);
            assert!(
                rgba.chunks_exact(4)
                    .any(|p| p[0] < 100 && p[1] < 100 && p[2] < 100)
            );
            let (second, again) = draw(&mut host, &mut gpu, &mut cache);
            assert_eq!(first, second);
            assert_eq!(rgba, again);
            let mut other = tekes_ui_render::Renderer::new_headless().unwrap();
            assert_ne!(gpu.resource_identity(), other.resource_identity());
            let (_, rebound) = draw(&mut host, &mut other, &mut cache);
            assert_eq!(rgba, rebound);
            gpu.collect_retired_images();
            assert_eq!(gpu.image_count(), 0);
            host.get_mut(ComponentId(2))
                .unwrap()
                .component_mut::<IconContent>()
                .unwrap()
                .set_glyph("tekes-back")
                .unwrap();
            let (same, _) = draw(&mut host, &mut other, &mut cache);
            let a = host
                .get(ComponentId(2))
                .unwrap()
                .component::<IconContent>()
                .unwrap()
                .layout();
            let b = host
                .get(ComponentId(4))
                .unwrap()
                .component::<IconContent>()
                .unwrap()
                .layout();
            let ci = resolve_surface(p, "canvas", "content", "rest")
                .unwrap()
                .icon;
            let ai = if declared_surface {
                resolve_surface(p, "canvas", "action-quiet", "rest")
                    .unwrap()
                    .icon
            } else {
                // Main's undeclared original Button shares its label foreground.
                p.color("primary-text").unwrap()
            };
            eprintln!("SVG_SPECS passive={a:?} action={b:?} content={ci:?} action={ai:?}");
            if a.artboard.size == b.artboard.size && ci == ai {
                assert_eq!(same[0], same[1]);
            } else {
                assert_ne!(
                    same[0], same[1],
                    "different resolved pixel variants must not alias"
                );
            }
            assert_eq!(
                other.image_count(),
                same.iter().collect::<std::collections::HashSet<_>>().len()
            );
            host.remove(ComponentId(2)).unwrap();
            host.remove(ComponentId(3)).unwrap();
            let (empty, _) = draw(&mut host, &mut other, &mut cache);
            assert!(empty.is_empty());
            assert_eq!(other.image_count(), 0);
        }
    }
}

#[test]
fn original_svg_rotates_on_gpu_with_one_resource_and_original_clip_at_all_scales() {
    for mode in Theme::ALL {
        for backing in [1., 2.] {
            for typography in [1., 1.5] {
                let mut gpu = tekes_ui_render::Renderer::new_headless().unwrap();
                let mut cache = SvgIconCache::default();
                let mut icon = IconContent::new("tekes-back").unwrap();
                icon.set_indeterminate(true);
                let mut h = ContentHost::new(Content::new(ComponentId(1), icon)).unwrap();
                let mut text = coretext::Text::new(typography);
                text.services = true;
                text.reduced_motion = false;
                text.backing = backing;
                let p = Palette::from(mode);
                let ground = resolve_surface(p, "canvas", "content", "rest")
                    .unwrap()
                    .background;
                let f = Rect::new(8., 8., 32., 32.);
                let clip = Rect::new(10., 9., 20., 24.);
                let mut images = vec![];
                let mut pixels = vec![];
                let clear_rgba = gpu
                    .render_to_rgba8(
                        &Scene::new(),
                        &mut text.atlas,
                        ((48. * backing) as u32, (48. * backing) as u32),
                        backing,
                        ground,
                    )
                    .unwrap();
                for phase in [0., 0.25, 0.5, 1.] {
                    text.now = std::time::Duration::from_secs_f64(
                        f64::from(tekes_ui_tokens::proposal::INDETERMINATE_ROTATION_PERIOD_SECONDS)
                            * phase,
                    );
                    text.requested = false;
                    text.atlas.begin_frame();
                    let mut scene = Scene::new();
                    {
                        let mut icons = cache.renderer(&mut gpu, backing);
                        h.render(
                            &mut scene,
                            &mut WithIconRenderer::new(&mut text, &mut icons),
                            f,
                            clip,
                            p,
                        )
                        .unwrap();
                    }
                    assert!(text.requested && h.animation_frame_requested());
                    images.push(
                        scene
                            .primitives()
                            .iter()
                            .find_map(|p| match p {
                                Primitive::Image(i) => Some(i.image),
                                Primitive::TransformedImage(i) => Some(i.quad.image),
                                _ => None,
                            })
                            .unwrap(),
                    );
                    let rgba = gpu
                        .render_to_rgba8(
                            &scene,
                            &mut text.atlas,
                            ((48. * backing) as u32, (48. * backing) as u32),
                            backing,
                            ground,
                        )
                        .unwrap();
                    // Exact GPU ground outside the original ancestor clip, including rotated corners.
                    let clear = &clear_rgba[..4];
                    let side = (48. * backing) as usize;
                    for y in 0..side {
                        for x in 0..side {
                            if (x as f32 + 0.5) / backing < clip.min_x()
                                || (x as f32 + 0.5) / backing >= clip.max_x()
                                || (y as f32 + 0.5) / backing < clip.min_y()
                                || (y as f32 + 0.5) / backing >= clip.max_y()
                            {
                                let offset = (y * side + x) * 4;
                                for i in 0..4 {
                                    assert!(rgba[offset + i].abs_diff(clear[i]) <= 1);
                                }
                            }
                        }
                    }
                    pixels.push(rgba);
                }
                assert!(images.iter().all(|id| *id == images[0]));
                assert_eq!(gpu.image_count(), 1);
                assert_ne!(pixels[0], pixels[1]);
                assert_ne!(pixels[1], pixels[2]);
                assert_eq!(pixels[0], pixels[3]);
            }
        }
    }
}
