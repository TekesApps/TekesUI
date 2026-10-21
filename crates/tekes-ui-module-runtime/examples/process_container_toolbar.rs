//! Independent public ContentHost + ProcessContainer + local toolbar consumer.
#[cfg(not(target_os = "macos"))]
fn main() {}
#[cfg(target_os = "macos")]
mod review {
    use anyhow::{Result, ensure};
    use serde_json::{Value, json};
    use std::time::{Duration, Instant};
    use tekes_ui_app_protocol::{Request, ToolbarItem};
    use tekes_ui_controls::{ButtonTextAlign, ButtonTextRenderer, ControlKey, composition::*};
    use tekes_ui_module_runtime::{component::*, *};
    use tekes_ui_render::Renderer;
    use tekes_ui_scene::{GlyphAtlas, Point, Quad, Rect, Scene, Size};
    use tekes_ui_text::{
        ShapeOptions, Shaper, TextStyle,
        coretext::{CoreTextFonts, CoreTextRasterizer, CoreTextShaper},
        push_paragraph,
    };
    use tekes_ui_tokens::{Palette, Rgba, Theme, proposal};
    struct Labels {
        shaper: CoreTextShaper,
        raster: CoreTextRasterizer,
        atlas: GlyphAtlas,
    }
    impl Labels {
        fn new() -> Self {
            let fonts = CoreTextFonts::new();
            Self {
                shaper: CoreTextShaper::new(fonts.clone()),
                raster: CoreTextRasterizer::new(fonts),
                atlas: GlyphAtlas::default(),
            }
        }
    }
    impl ButtonTextRenderer for Labels {
        fn measure(&mut self, s: &str, role: proposal::TextRole, _: f32) -> Size {
            let role = role.style();
            let p = self.shaper.shape(
                s,
                &TextStyle::system(role.size).with_weight(role.weight),
                &ShapeOptions::default(),
            );
            Size::new(p.width as f32, p.height as f32)
        }
        fn draw(
            &mut self,
            scene: &mut Scene,
            s: &str,
            role: proposal::TextRole,
            r: Rect,
            align: ButtonTextAlign,
            ink: Rgba,
        ) {
            let role = role.style();
            let p = self.shaper.shape(
                s,
                &TextStyle::system(role.size).with_weight(role.weight),
                &ShapeOptions::default(),
            );
            let x = if align == ButtonTextAlign::Center {
                r.min_x() + (r.size.width - p.width as f32) / 2.
            } else {
                r.min_x()
            };
            let y = r.min_y() + (r.size.height - p.height as f32) / 2.;
            scene.push_clip(r);
            push_paragraph(
                scene,
                &mut self.atlas,
                &mut self.raster,
                &p,
                Point::new(x.round(), y.round()),
                1.,
                ink,
            );
            scene.pop_clip();
        }
    }
    struct Counter {
        value: u64,
        revision: u64,
        bar_revision: u64,
        theme: Theme,
        brand: usize,
        fonts: Option<CoreTextFonts>,
    }
    impl Counter {
        fn toolbar(&mut self, e: &ComponentEmitter, scope: ToolbarScope, icon: bool) -> Result<()> {
            self.bar_revision += 1;
            e.emit(ComponentEvent::Toolbar {
                revision: self.bar_revision,
                scope,
                items: vec![ToolbarAction {
                    command: ToolbarItem {
                        id: 7,
                        title: if icon {
                            String::new()
                        } else {
                            "Increment · 增加".into()
                        },
                        enabled: true,
                        geometry_dependent: false,
                    },
                    icon: icon.then(|| "review-icon".into()),
                    accessible_name: Some("Increment · 增加".into()),
                    tooltip: None,
                }],
            })
        }
    }
    impl PanelModule for Counter {
        fn update(
            &mut self,
            r: &Request,
            scene: &mut Scene,
            atlas: &mut GlyphAtlas,
        ) -> Result<ModuleState> {
            let palette = Palette::new(self.theme, self.brand).unwrap();
            scene.push_quad(Quad::new(
                Rect::new(0., 0., r.width as f32, r.height as f32),
                palette.color("surface-panel").unwrap(),
            ));
            let fonts = self.fonts.get_or_insert_with(CoreTextFonts::new).clone();
            let mut shaper = CoreTextShaper::new(fonts.clone());
            let mut raster = CoreTextRasterizer::new(fonts);
            let role = proposal::TextRole::UiBody.style();
            let p = shaper.shape(
                &format!("Child value: {}", self.value),
                &TextStyle::system(role.size).with_weight(role.weight),
                &ShapeOptions::default(),
            );
            push_paragraph(
                scene,
                atlas,
                &mut raster,
                &p,
                Point::new(12., 12.),
                r.scale,
                palette.color("primary-text").unwrap(),
            );
            Ok(ModuleState {
                revision: self.revision,
                toolbar: vec![],
            })
        }
    }
    impl ProcessModule for Counter {
        fn view(&mut self) -> Option<&mut dyn PanelModule> {
            Some(self)
        }
        fn view_capabilities(&self) -> ViewCapabilities {
            ViewCapabilities {
                primary_pointer: true,
                independent_toolbar: true,
                ..Default::default()
            }
        }
        fn started(&mut self, e: &ComponentEmitter) -> Result<()> {
            self.toolbar(e, ToolbarScope::View, false)
        }
        fn handle(&mut self, body: RequestBody, e: &ComponentEmitter) -> Result<Value> {
            match body {
                RequestBody::Invoke { command: 7, .. } => self.value += 1,
                RequestBody::Message { topic, payload } => match topic.as_str() {
                    "window" => self.toolbar(
                        e,
                        ToolbarScope::Window {
                            window: e.window_identity().unwrap(),
                        },
                        false,
                    )?,
                    "local" => self.toolbar(e, ToolbarScope::View, false)?,
                    "icon" => self.toolbar(e, ToolbarScope::View, true)?,
                    "theme" => {
                        self.theme = *Theme::ALL
                            .get(payload["theme"].as_u64().unwrap() as usize)
                            .unwrap();
                        self.brand = payload["brand"].as_u64().unwrap() as usize;
                    }
                    _ => anyhow::bail!("unknown service"),
                },
                _ => anyhow::bail!("unsupported request"),
            }
            self.revision += 1;
            e.invalidate_view(self.revision)?;
            Ok(json!({"value":self.value,"revision":self.revision}))
        }
    }
    fn create() -> Box<dyn ProcessModule> {
        Box::new(Counter {
            value: 0,
            revision: 0,
            bar_revision: 0,
            theme: Theme::LIGHT,
            brand: 0,
            fonts: None,
        })
    }
    fn node(host: &mut ContentHost) -> &mut ProcessContainer {
        host.get_mut(ComponentId(1))
            .unwrap()
            .component_mut::<ProcessContainer>()
            .unwrap()
    }
    fn tick(
        host: &mut ContentHost,
        renderer: &mut Renderer,
        text: &mut Labels,
        palette: Palette,
    ) -> Result<Scene> {
        node(host).prepare(renderer)?;
        text.atlas.begin_frame();
        let mut scene = Scene::new();
        let frame = Rect::new(0., 0., 320., 180.);
        host.render(&mut scene, text, frame, frame, palette)
            .map_err(|e| anyhow::anyhow!("{e:?}"))?;
        Ok(scene)
    }
    fn until(
        host: &mut ContentHost,
        renderer: &mut Renderer,
        text: &mut Labels,
        palette: Palette,
        mut test: impl FnMut(&mut ProcessContainer) -> bool,
    ) -> Result<Scene> {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let scene = tick(host, renderer, text, palette)?;
            if test(node(host)) {
                return Ok(scene);
            }
            ensure!(
                Instant::now() < deadline,
                "component deadline: {:?}",
                node(host)
            );
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    fn key(host: &mut ContentHost, key: ControlKey, down: bool) {
        host.event(&ContentEvent::KeyInput(ContentKeyEvent {
            key: Some(key),
            down,
            repeat: false,
            modifiers: Default::default(),
            native_code: 0,
            characters: String::new(),
            characters_ignoring_modifiers: String::new(),
        }));
    }
    fn send(host: &mut ContentHost, topic: &str, payload: Value) -> Result<u64> {
        node(host).request(RequestBody::Message {
            topic: topic.into(),
            payload,
        })
    }
    fn await_reply(host: &mut ContentHost, id: u64) -> Result<Value> {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            for p in node(host).drain_events() {
                if let Output::Response { request, result } = p.output
                    && request == id
                {
                    return result.map_err(anyhow::Error::msg);
                }
            }
            ensure!(Instant::now() < deadline, "reply deadline");
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    fn capture(
        host: &mut ContentHost,
        renderer: &mut Renderer,
        text: &mut Labels,
        palette: Palette,
        path: &std::path::Path,
    ) -> Result<Vec<u8>> {
        let scene = tick(host, renderer, text, palette)?;
        let pixels = renderer.render_to_rgba8(
            &scene,
            &mut text.atlas,
            (320, 180),
            1.,
            palette.color("surface-panel").unwrap(),
        )?;
        let mut encoder = png::Encoder::new(
            std::io::BufWriter::new(std::fs::File::create(path)?),
            320,
            180,
        );
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.write_header()?.write_image_data(&pixels)?;
        Ok(pixels)
    }
    pub fn run() -> Result<()> {
        let args = std::env::args().collect::<Vec<_>>();
        if run_component_worker_from_args(
            &[ComponentDefinition {
                id: "local-toolbar",
                capabilities: Capabilities {
                    view: true,
                    ..Capabilities::HEADLESS
                },
                create,
            }],
            &args,
        )? {
            return Ok(());
        }
        let output = std::path::PathBuf::from(
            args.get(1)
                .ok_or_else(|| anyhow::anyhow!("output directory required"))?,
        );
        std::fs::create_dir_all(&output)?;
        let options = ProcessComponentOptions::new("local-toolbar", 51).with_view(
            11,
            23,
            ViewCapabilities {
                primary_pointer: true,
                independent_toolbar: true,
                ..Default::default()
            },
        );
        let container = ProcessContainer::new_view(options, || {})?;
        let mut host = ContentHost::new(Content::new(ComponentId(1), container)).unwrap();
        host.set_action_events(true);
        let mut renderer = Renderer::new_headless()?;
        let mut text = Labels::new();
        let palette = Theme::LIGHT.into();
        until(&mut host, &mut renderer, &mut text, palette, |c| {
            c.revision() == Some(0) && c.input_ready()
        })?;
        let owner = node(&mut host).session().unwrap().clone();
        let bar = node(&mut host).local_toolbar_frame().unwrap();
        ensure!(bar.size.height > 0. && bar.size.height < 180.);
        let before = capture(
            &mut host,
            &mut renderer,
            &mut text,
            palette,
            &output.join("local-before.png"),
        )?;
        let point = Point::new(bar.min_x() + 24., bar.center().y);
        host.event(&ContentEvent::PointerDown(point));
        host.event(&ContentEvent::PointerUp(point));
        until(&mut host, &mut renderer, &mut text, palette, |c| {
            c.revision() == Some(1) && c.input_ready()
        })?;
        ensure!(
            host.drain_actions().is_empty(),
            "container duplicated its RPC as another business action"
        );
        let after = capture(
            &mut host,
            &mut renderer,
            &mut text,
            palette,
            &output.join("local-after.png"),
        )?;
        ensure!(before != after, "real child text did not change");
        host.focus(Some(ComponentId(1)), true);
        key(&mut host, ControlKey::Space, true);
        key(&mut host, ControlKey::Space, false);
        until(&mut host, &mut renderer, &mut text, palette, |c| {
            c.revision() == Some(2)
        })?;
        // A hidden component must revoke both pointer and keyboard arms, even
        // if it is shown again before a late release reaches the ContentHost.
        for transition in ["hidden", "disabled", "inactive"] {
            for keyboard in [false, true] {
                host.focus(Some(ComponentId(1)), true);
                if keyboard {
                    key(&mut host, ControlKey::Space, true);
                } else {
                    host.event(&ContentEvent::PointerDown(point));
                }
                match transition {
                    "hidden" => {
                        node(&mut host).set_visible(false)?;
                        node(&mut host).set_visible(true)?;
                    }
                    "disabled" => {
                        node(&mut host).set_enabled(false);
                        node(&mut host).set_enabled(true);
                    }
                    _ => {
                        node(&mut host).set_window_active(false);
                        node(&mut host).set_window_active(true);
                    }
                }
                until(&mut host, &mut renderer, &mut text, palette, |c| {
                    c.input_ready()
                })?;
                if keyboard {
                    key(&mut host, ControlKey::Space, false);
                } else {
                    host.event(&ContentEvent::PointerUp(point));
                }
                // Service FIFO is a barrier after any accidental Invoke. It can
                // therefore prove absence of a duplicate without a timing sleep.
                let barrier = send(&mut host, "local", Value::Null)?;
                ensure!(
                    await_reply(&mut host, barrier)?["value"] == 2,
                    "toolbar retained its old activation arm ({transition}, keyboard={keyboard})"
                );
                until(&mut host, &mut renderer, &mut text, palette, |c| {
                    c.input_ready()
                })?;
            }
        }
        let id = send(&mut host, "window", Value::Null)?;
        let result = await_reply(&mut host, id)?;
        let revision = result["revision"].as_u64().unwrap();
        until(&mut host, &mut renderer, &mut text, palette, |c| {
            c.revision() == Some(revision) && c.local_toolbar_frame().is_none() && c.input_ready()
        })?;
        ensure!(node(&mut host).session() == Some(&owner));
        capture(
            &mut host,
            &mut renderer,
            &mut text,
            palette,
            &output.join("window-scope-body.png"),
        )?;
        let id = send(&mut host, "local", Value::Null)?;
        let revision = await_reply(&mut host, id)?["revision"].as_u64().unwrap();
        until(&mut host, &mut renderer, &mut text, palette, |c| {
            c.revision() == Some(revision) && c.local_toolbar_frame().is_some() && c.input_ready()
        })?;
        let mut themes = 0;
        for (theme_index, theme) in Theme::ALL.into_iter().enumerate() {
            for brand in 0..3 {
                let id = send(
                    &mut host,
                    "theme",
                    json!({"theme":theme_index,"brand":brand}),
                )?;
                let result = await_reply(&mut host, id)?;
                ensure!(result["value"] == 2);
                let revision = result["revision"].as_u64().unwrap();
                let palette = Palette::new(theme, brand).unwrap();
                until(&mut host, &mut renderer, &mut text, palette, |c| {
                    c.revision() == Some(revision) && c.input_ready()
                })?;
                capture(
                    &mut host,
                    &mut renderer,
                    &mut text,
                    palette,
                    &output.join(format!("theme-{theme_index}-brand-{brand}.png")),
                )?;
                themes += 1;
            }
        }
        let id = send(&mut host, "icon", Value::Null)?;
        await_reply(&mut host, id)?;
        ensure!(
            node(&mut host).prepare(&mut renderer).is_err(),
            "missing product icon silently substituted text"
        );
        let image = renderer.register_image(&[64, 128, 192, 255], 1, 1)?;
        node(&mut host).set_toolbar_icon_images(vec![("review-icon".into(), image)]);
        until(&mut host, &mut renderer, &mut text, palette, |c| {
            c.local_toolbar_frame().is_some() && c.input_ready()
        })?;
        capture(
            &mut host,
            &mut renderer,
            &mut text,
            palette,
            &output.join("provided-icon.png"),
        )?;
        drop(host);
        renderer.collect_retired_images();
        ensure!(
            renderer.unregister_image(image),
            "borrowed icon was incorrectly unregistered"
        );
        println!(
            "{}",
            json!({"result":"LOCAL_PROCESS_TOOLBAR_OK","real_content_host":true,"real_coretext_and_gpu":true,"one_rpc_per_activation":true,"keyboard_activation":true,"hide_disable_inactive_cancelled_pointer_and_keyboard":true,"scope_change_preserved_owner":true,"theme_brand_cases":themes,"missing_icon_explicit_and_recoverable":true,"borrowed_images_preserved":true,"native_and_gold7":"pending","performance":false})
        );
        Ok(())
    }
}
#[cfg(target_os = "macos")]
fn main() -> anyhow::Result<()> {
    review::run()
}
