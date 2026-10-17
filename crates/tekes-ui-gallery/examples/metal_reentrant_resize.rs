//! Native regression: resize and close from inside a Metal animation callback.
//! Run with TEKES_METAL_DISPLAY_LINK=1. Not a 120Hz performance benchmark.
#[cfg(not(target_os = "macos"))]
fn main() {}

#[cfg(target_os = "macos")]
fn main() {
    use std::{cell::Cell, rc::Rc};
    use tekes_ui_platform_macos::{
        App, AppConfig, Event, MenuBar, Size, Window, WindowConfig, WindowHandler, selftest,
    };
    use tekes_ui_render::Renderer;
    use tekes_ui_scene::{GlyphAtlas, Scene};
    use tekes_ui_tokens::Rgba;
    struct Probe {
        renderer: Option<Renderer>,
        atlas: GlyphAtlas,
        frames: Rc<Cell<u64>>,
        presents: Rc<Cell<usize>>,
    }
    impl WindowHandler for Probe {
        fn handle(&mut self, event: Event, window: &mut Window) {
            match event {
                Event::Created => {
                    // SAFETY: main thread; window owns the layer through renderer lifetime.
                    self.renderer = Some(
                        unsafe {
                            Renderer::new_for_metal_layer(
                                window.metal_layer(),
                                window.drawable_size(),
                                window.scale() as f32,
                            )
                        }
                        .expect("renderer"),
                    );
                    window.set_presentation_timing(true);
                    window.set_animating(true);
                }
                Event::Resized {
                    drawable_size,
                    scale,
                    ..
                } => {
                    self.renderer
                        .as_mut()
                        .expect("renderer")
                        .resize(drawable_size, scale as f32);
                }
                Event::AnimationFrame(_) => {
                    let count = self.frames.get() + 1;
                    self.frames.set(count);
                    if [30, 60, 90].contains(&count) {
                        // Resized/Redraw are queued behind this handler while its
                        // delegate lifetime guard still retains the old Metal link.
                        window.set_content_size(Size {
                            width: 640.0 + count as f64,
                            height: 480.0 + count as f64,
                        });
                    }
                    self.renderer
                        .as_mut()
                        .expect("renderer")
                        .render(
                            &Scene::new(),
                            &mut self.atlas,
                            Rgba::new(0.2, 0.3, 0.4, 1.0),
                        )
                        .expect("render");
                    if count == 150 {
                        self.presents.set(
                            window
                                .presented_frames()
                                .iter()
                                .filter(|frame| frame.presented_at > 0.0)
                                .count(),
                        );
                        window.close();
                    }
                }
                _ => {}
            }
        }
    }
    let frames = Rc::new(Cell::new(0));
    let presents = Rc::new(Cell::new(0));
    selftest::run_script(vec![(10000, selftest::Action::Close)]);
    App::new(AppConfig::default()).run(
        WindowConfig {
            title: "Metal reentrant resize regression".into(),
            ..Default::default()
        },
        MenuBar::default(),
        Probe {
            renderer: None,
            atlas: GlyphAtlas::default(),
            frames: frames.clone(),
            presents: presents.clone(),
        },
    );
    assert_eq!(
        frames.get(),
        150,
        "animation stalled after a handler resized its own window"
    );
    assert!(
        presents.get() >= 120,
        "actual presentation stalled: {}",
        presents.get()
    );
    eprintln!(
        "reentrant resize and callback-close passed: {} callbacks, {} positive presented receipts",
        frames.get(),
        presents.get()
    );
}
