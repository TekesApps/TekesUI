use tekes_ui_render::{RenderError, Renderer};
use tekes_ui_scene::{GlyphAtlas, ImageQuad, Rect, Scene};
use tekes_ui_tokens::Rgba;
#[test]
fn owned_registration_releases_on_render_and_rejects_cross_renderer_rebinding() {
    let mut renderer = Renderer::new_headless().unwrap();
    let context = renderer.gpu_context();
    let image = context.create_image((8, 8)).unwrap();
    let registration = renderer.register_owned_gpu_image(&image).unwrap();
    assert_eq!(
        registration.bitmap_lease().unwrap_err(),
        RenderError::InvalidArgument(
            "externally mutable GPU image cannot provide an immutable bitmap lease"
        )
    );
    let old = registration.id();
    assert_eq!(renderer.image_count(), 1);
    let mut other = Renderer::new_headless().unwrap();
    let other_image = other.gpu_context().create_image((8, 8)).unwrap();
    let other_registration = other.register_owned_gpu_image(&other_image).unwrap();
    // IDs can coincide across renderers; the guard must bind ownership as well.
    assert_eq!(old, other_registration.id());
    assert!(
        other
            .replace_owned_gpu_image(&registration, &other_image)
            .is_err()
    );
    assert!(
        renderer
            .replace_owned_gpu_image(&registration, &other_image)
            .is_err()
    );
    renderer
        .replace_owned_gpu_image(&registration, &image)
        .unwrap();
    drop(registration);
    let mut stale = Scene::new();
    stale.push_image(ImageQuad {
        rect: Rect::new(0., 0., 8., 8.),
        image: old,
        opacity: 1.,
    });
    let pixels = renderer
        .render_to_rgba8(
            &stale,
            &mut GlyphAtlas::default(),
            (8, 8),
            1.,
            Rgba::gray(1.),
        )
        .unwrap();
    assert!(pixels.iter().all(|c| *c == 255));
    assert_eq!(renderer.image_count(), 0);
    assert_eq!(renderer.collect_retired_images(), 0);
    assert!(!renderer.unregister_image(old));
    let second = renderer.register_owned_gpu_image(&image).unwrap();
    assert_ne!(second.id(), old);
    drop(second);
    assert_eq!(renderer.collect_retired_images(), 1);
    let last = renderer.register_owned_gpu_image(&image).unwrap();
    drop(renderer);
    drop(last);
    drop(other_registration);
    assert_eq!(other.collect_retired_images(), 1);
}
