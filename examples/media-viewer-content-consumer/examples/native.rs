//! Controlled-window fixture, no automatic input injection. Raw OS events are
//! logged; the operator must separately record whether input was physical/CUA/AX.
#[cfg(target_os = "macos")]
#[path = "../tests/support/coretext.rs"]
mod coretext;
#[cfg(target_os = "macos")]
#[path = "../tests/support/viewer_fixture.rs"]
mod fixture;
#[cfg(target_os = "macos")]
#[path = "../tests/support/native_key.rs"]
mod native_key;
#[cfg(target_os = "macos")]
#[path = "../tests/support/native_pointer.rs"]
mod native_pointer;
#[cfg(not(target_os = "macos"))]
fn main() {
    panic!("macOS native fixture required");
}
#[cfg(target_os = "macos")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::io::Write;
    use tekes_ui_controls::composition::*;
    use tekes_ui_platform_macos::{
        self as p, App, AppConfig, Event, MenuBar, Window, WindowConfig, WindowHandler,
        content_accessibility::ContentAccessibilitySnapshot,
    };
    use tekes_ui_render::Renderer;
    use tekes_ui_scene::{Rect, Scene};
    use tekes_ui_tokens::{Palette, Theme};
    struct Host {
        tree: Option<ContentHost>,
        gpu: Option<Renderer>,
        text: coretext::Text,
        actions: fixture::Actions,
        log: std::fs::File,
        external_display_id: u32,
    }
    impl WindowHandler for Host {
        fn handle(&mut self, event: Event, window: &mut Window) {
            writeln!(self.log, "event={event:?}").unwrap();
            if matches!(&event, Event::Created) {
                writeln!(
                    self.log,
                    "screen={:?} screen_id={:?} placement={:?} window_frame={:?}",
                    window.screen_info(),
                    window.native_screen_id(),
                    window.placement_report(),
                    window.native_window_frame()
                )
                .unwrap();
                self.log.flush().unwrap();
                assert_eq!(
                    window.native_screen_id(),
                    Some(self.external_display_id),
                    "native window must start on the granted external display"
                );
                assert!(
                    window.screen_info().is_some_and(|screen| !screen.builtin),
                    "external display required"
                );
                assert!(window.set_text_key_events(true));
                let mut gpu = unsafe {
                    Renderer::new_for_metal_layer(
                        window.metal_layer(),
                        window.drawable_size(),
                        window.scale() as f32,
                    )
                }
                .unwrap();
                let mut tree = if std::env::var_os("MEDIA_VIEWER_MODAL").is_some() {
                    fixture::modal_fixture(&mut gpu)
                } else {
                    fixture::fixture(&mut gpu)
                };
                if tree.get(fixture::DIALOG).is_some() {
                    let frame = Rect::new(
                        0.,
                        0.,
                        window.size().width as f32,
                        window.size().height as f32,
                    );
                    tree.render(
                        &mut Scene::new(),
                        &mut self.text,
                        frame,
                        frame,
                        Theme::LIGHT.into(),
                    )
                    .unwrap();
                    tree.focus(Some(fixture::INVOKER), true);
                    tree.show_dialog(fixture::DIALOG).unwrap();
                }
                self.tree = Some(tree);
                self.gpu = Some(gpu);
            }
            let Some(tree) = &mut self.tree else {
                return;
            };
            match event {
                Event::Resized {
                    drawable_size,
                    scale,
                    ..
                } => self
                    .gpu
                    .as_mut()
                    .unwrap()
                    .resize(drawable_size, scale as f32),
                Event::Focus(active) => tree.set_window_active(active),
                Event::Mouse(mouse) => {
                    tree.event_with_text(&native_pointer::translate(mouse), &mut self.text)
                        .unwrap();
                }
                Event::Key(key) => {
                    tree.event_with_text(
                        &ContentEvent::KeyInput(native_key::key(key)),
                        &mut self.text,
                    )
                    .unwrap();
                }
                Event::TextKey(key) => {
                    tree.event_with_text(&native_key::translate(key), &mut self.text)
                        .unwrap();
                }
                Event::TextCommand(command) => {
                    tree.event_with_text(&ContentEvent::TextCommand(command), &mut self.text)
                        .unwrap();
                }
                Event::Accessibility { target, action } => {
                    let response = window
                        .content_accessibility_publication()
                        .and_then(|publication| {
                            let owner = publication.owner(target)?;
                            Some(publication.dispatch(owner, tree, target, action))
                        })
                        .unwrap_or_default();
                    writeln!(self.log, "ax_response={response:?}").unwrap();
                }
                Event::CloseRequested => {
                    tree.set_window_active(false);
                    window.set_content_accessibility_publication(None);
                    window.close();
                    return;
                }
                _ => {}
            }
            let (close, retry) = self.actions.dispatch(tree);
            writeln!(
                self.log,
                "close={close} retry={retry} focused={:?} transform={:?} changes={:?}",
                tree.focused(),
                fixture::viewer(tree).transform(),
                fixture::viewer(tree).drain_transform_changes()
            )
            .unwrap();
            if close > 0 && tree.get(fixture::DIALOG).is_some() {
                assert!(tree.close_dialog());
                writeln!(self.log, "modal_close_restored_focus={:?}", tree.focused()).unwrap();
            } else if close > 0 {
                tree.set_window_active(false);
                window.set_content_accessibility_publication(None);
                window.close();
                return;
            }
            // Retry notifies this host once; it performs no I/O or implicit load.
            fixture::refresh_buttons(tree);
            fixture::refresh_status(tree);
            tree.set_window_active(window.is_active());
            self.text.backing = window.scale() as f32;
            self.text.atlas.begin_frame();
            let frame = Rect::new(
                0.,
                0.,
                window.size().width as f32,
                window.size().height as f32,
            );
            let mut scene = Scene::new();
            let palette = Palette::from(Theme::LIGHT);
            tree.render(&mut scene, &mut self.text, frame, frame, palette)
                .unwrap();
            let clear = tekes_ui_tokens::surface_palette::resolve_surface(
                palette, "canvas", "content", "rest",
            )
            .unwrap()
            .background;
            self.gpu
                .as_mut()
                .unwrap()
                .render(&scene, &mut self.text.atlas, clear)
                .unwrap();
            if window.is_active() {
                assert!(window.set_content_accessibility_snapshot(
                    ContentAccessibilitySnapshot::from_host(tree).unwrap()
                ));
            } else {
                window.set_content_accessibility_publication(None);
            }
            writeln!(
                self.log,
                "viewport={:?} image={:?} semantics={:#?}",
                tree.get(fixture::VIEW).unwrap().frame(),
                tree.get(fixture::IMAGE).unwrap().frame(),
                tree.semantics()
            )
            .unwrap();
            self.log.flush().unwrap();
        }
    }
    let directory = std::path::PathBuf::from(
        std::env::var_os("MEDIA_VIEWER_OUTPUT").ok_or("set MEDIA_VIEWER_OUTPUT")?,
    );
    std::fs::create_dir_all(&directory)?;
    if std::fs::read_dir(&directory)?.next().is_some() {
        return Err("use a fresh raw-evidence directory".into());
    }
    let binding =
        std::env::var_os("MEDIA_VIEWER_BINDING").ok_or("set verified run-binding JSON")?;
    std::fs::copy(binding, directory.join("run-binding.json"))?;
    std::fs::write(
        directory.join("process.txt"),
        format!(
            "pid={}\ncurrent_exe={:?}\n",
            std::process::id(),
            std::env::current_exe()?
        ),
    )?;
    let mut log = std::fs::File::create(directory.join("native-events.txt"))?;
    writeln!(
        log,
        "input_source=UNATTESTED; operator must supply provenance; physical IME/VO/performance/pinch NOT_TESTED"
    )?;
    let external_display_id: u32 = std::env::var("MEDIA_VIEWER_EXTERNAL_DISPLAY_ID")?.parse()?;
    App::new(AppConfig {
        name: Some("Media Viewer Consumer 204".into()),
        ..Default::default()
    })
    .run(
        WindowConfig {
            title: "Media Viewer Consumer 204".into(),
            size: p::Size {
                width: 480.,
                height: 720.,
            },
            placement: p::WindowPlacement::Screen(p::ScreenChoice::External),
            ..Default::default()
        },
        MenuBar::default(),
        Host {
            tree: None,
            gpu: None,
            text: coretext::Text::new(1.),
            actions: fixture::Actions::default(),
            log,
            external_display_id,
        },
    );
    Ok(())
}
