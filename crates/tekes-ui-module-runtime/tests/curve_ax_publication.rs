//! Issue #427: Verify parent-host macOS AX publication (`from_host` + `prepare`)
//! over a remote `ProcessContainer` exposing both text editors and sliders.
#[cfg(not(target_os = "macos"))]
fn main() {}

#[cfg(target_os = "macos")]
fn main() -> anyhow::Result<()> {
    use anyhow::{Context, Result, bail, ensure};
    use std::rc::Rc;
    use std::time::{Duration, Instant};
    use tekes_ui_app_protocol::Request;
    use tekes_ui_controls::{
        ButtonTextAlign, ButtonTextRenderer, TextSurfaceHost, TextSurfaceTextRenderer,
        composition::*, layout::MeasuredChild,
    };
    use tekes_ui_macos_bridge::content_accessibility::{
        ContentAccessibilityOwner, ContentAccessibilitySnapshot, WindowAccessibilityPublication,
    };
    use tekes_ui_module_runtime::{component::*, *};
    use tekes_ui_platform_macos::{AccessibilityRange, AccessibilityRole};
    use tekes_ui_scene::{GlyphAtlas, Point, Rect, Scene, Size};
    use tekes_ui_text::{
        coretext::{CoreTextFonts, CoreTextShaper},
        model::{Paragraph, ShapeOptions},
        style::TextStyle,
        traits::Shaper,
    };
    use tekes_ui_tokens::{Rgba, Theme, proposal};

    const CAPS: ViewCapabilities = ViewCapabilities {
        accessibility: true,
        primary_pointer: false,
        content_pointer: false,
        keyboard: false,
        scroll: false,
        text_ime: false,
        independent_toolbar: false,
    };

    struct Text(CoreTextShaper);
    impl Text {
        fn new() -> Self {
            Self(CoreTextShaper::new(CoreTextFonts::new()))
        }
    }
    impl ButtonTextRenderer for Text {
        fn text_surface_host(&mut self) -> Option<&mut dyn TextSurfaceHost> {
            Some(self)
        }
        fn measure(&mut self, s: &str, role: proposal::TextRole, w: f32) -> Size {
            Size::new(
                (s.chars().count() as f32 * role.style().size * 0.5).min(w),
                role.style().line_height,
            )
        }
        fn draw(
            &mut self,
            _: &mut Scene,
            _: &str,
            _: proposal::TextRole,
            _: Rect,
            _: ButtonTextAlign,
            _: Rgba,
        ) {
        }
    }
    impl TextSurfaceTextRenderer for Text {
        fn push_visible(&mut self, _: &mut Scene, _: &Paragraph, _: Point, _: Rgba, _: Rect) {}
    }
    impl TextSurfaceHost for Text {
        fn shape_surface(
            &mut self,
            text: &str,
            role: proposal::TextRole,
            width: Option<f32>,
        ) -> Rc<Paragraph> {
            let style = TextStyle::system(role.style().size).with_weight(role.style().weight);
            Rc::new(self.0.shape(
                text,
                &style,
                &ShapeOptions {
                    width: width.map(f64::from),
                    line_spacing: 0.,
                },
            ))
        }
    }

    #[derive(Debug)]
    struct MultiSlider {
        count: usize,
        frame: std::cell::Cell<Rect>,
    }
    impl Component for MultiSlider {
        fn measure(&mut self, _: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
            MeasuredChild::new(Size::new(
                available.width,
                (self.count as f32 * 24.).max(24.),
            ))
        }
        fn arrange(&mut self, _: &mut dyn ButtonTextRenderer, frame: Rect, _: Rect) {
            self.frame.set(frame);
        }
        fn semantics(&self) -> ContentSemantics {
            ContentSemantics::new(ContentRole::Group, "RemoteSliders")
        }
        fn semantic_parts(&self) -> Vec<SemanticPart> {
            let f = self.frame.get();
            (0..self.count)
                .map(|i| {
                    let mut s =
                        ContentSemantics::new(ContentRole::Slider, format!("RemoteSlider-{i}"));
                    s.value = Some("40".into());
                    SemanticPart {
                        index: i + 1,
                        frame: Rect::new(f.min_x() + 4., f.min_y() + i as f32 * 22., 180., 18.),
                        focused: false,
                        semantics: s,
                    }
                })
                .collect()
        }
        fn semantic_part_parent(&self, _: usize) -> Option<usize> {
            None
        }
        fn semantic_part_value_range(&self, part: usize) -> Option<std::ops::RangeInclusive<f64>> {
            (1..=self.count).contains(&part).then_some(0.0..=100.0)
        }
    }

    struct RemoteChild {
        host: ContentHost,
        text: Text,
        revision: u64,
    }
    impl PanelModule for RemoteChild {
        fn update(
            &mut self,
            r: &Request,
            s: &mut Scene,
            _: &mut GlyphAtlas,
        ) -> Result<ModuleState> {
            let f = Rect::new(0., 0., r.width as f32, r.height as f32);
            self.host
                .render(s, &mut self.text, f, f, Theme::LIGHT.into())?;
            Ok(ModuleState {
                revision: self.revision,
                toolbar: vec![],
            })
        }
    }
    impl ProcessModule for RemoteChild {
        fn view(&mut self) -> Option<&mut dyn PanelModule> {
            Some(self)
        }
        fn content(&mut self) -> Option<(&mut ContentHost, &mut dyn ButtonTextRenderer)> {
            Some((&mut self.host, &mut self.text))
        }
        fn view_capabilities(&self) -> ViewCapabilities {
            CAPS
        }
        fn handle(&mut self, _: RequestBody, _: &ComponentEmitter) -> Result<serde_json::Value> {
            Ok(serde_json::Value::Null)
        }
    }

    fn create_worker() -> Box<dyn ProcessModule> {
        let mut children = Vec::new();
        for i in 0..8usize {
            let mut f = TextFieldContent::new(format!("RemoteField-{i}"), &format!("hello-{i}"));
            f.show_metadata = false;
            children.push(Content::new(ComponentId(100 + i as u64), f));
        }
        children.push(Content::new(
            ComponentId(200),
            MultiSlider {
                count: 16,
                frame: std::cell::Cell::new(Rect::default()),
            },
        ));
        let root = Content::new(ComponentId(10), StackContent::vertical(4., children));
        let host = ContentHost::new(root).expect("worker host");
        Box::new(RemoteChild {
            host,
            text: Text::new(),
            revision: 1,
        })
    }

    let args = std::env::args().collect::<Vec<_>>();
    let caps = Capabilities {
        view: true,
        ..Capabilities::HEADLESS
    };
    let defs = [ComponentDefinition {
        id: "curve-ax-pub",
        capabilities: caps,
        create: create_worker,
    }];
    if run_component_worker_from_args(&defs, &args)? {
        return Ok(());
    }

    let mut gpu = tekes_ui_render::Renderer::new_headless()?;
    let mut container_init = ProcessContainer::new_view(
        ProcessComponentOptions::new("curve-ax-pub", 88).with_view(88, 1, CAPS),
        || {},
    )?;
    container_init.begin_curve_preflight()?;
    let deadline = Instant::now() + Duration::from_secs(15);
    while !container_init.curve_preflight_ready()? {
        ensure!(Instant::now() < deadline, "curve preflight deadline");
        std::thread::sleep(Duration::from_millis(5));
    }
    let panel = PanelContent::new(Content::new(ComponentId(50), container_init))?;
    let mut host = ContentHost::new(Content::new(ComponentId(1), panel))?;
    let mut text = Text::new();
    let frame = Rect::new(0., 0., 360., 700.);
    let mut need_layout = true;
    loop {
        let container = host
            .get(ComponentId(50))
            .unwrap()
            .component::<ProcessContainer>()
            .unwrap();
        let events = container.drain_curve_host_events();
        let host_update = !events.is_empty() || container.curve_requires_host_update();
        if host_update {
            host.get_mut(ComponentId(50))
                .unwrap()
                .component_mut::<ProcessContainer>()
                .unwrap()
                .prepare_curve_host_update()?;
        }
        if need_layout || host_update || host.needs_render() {
            host.render(
                &mut Scene::new(),
                &mut text,
                frame,
                frame,
                Theme::LIGHT.into(),
            )?;
            need_layout = false;
        }
        let prepared = host
            .get(ComponentId(50))
            .unwrap()
            .component::<ProcessContainer>()
            .unwrap()
            .frame_resource_handle()?
            .prepare_current_frame(&mut gpu)?;
        if prepared {
            host.repaint_current(
                &mut Scene::new(),
                &mut text,
                frame,
                frame,
                Theme::LIGHT.into(),
            )?;
        }
        let container = host
            .get(ComponentId(50))
            .unwrap()
            .component::<ProcessContainer>()
            .unwrap();
        if let Some(err) = container.error() {
            bail!("container error: {err}");
        }
        if container.input_ready() && !container.semantic_parts().is_empty() {
            break;
        }
        ensure!(Instant::now() < deadline, "curve ready deadline");
        std::thread::sleep(Duration::from_millis(5));
    }

    // Exercise full parent-host macOS AX publication (`from_host` + `prepare`; stops at `prepare`).
    let snapshot = ContentAccessibilitySnapshot::from_host(&host)
        .context("parent ContentAccessibilitySnapshot::from_host failed")?;
    let publication =
        WindowAccessibilityPublication::prepare(vec![(ContentAccessibilityOwner(1), snapshot)])
            .context("WindowAccessibilityPublication::prepare failed")?;

    let container_node = publication
        .nodes()
        .iter()
        .find(|n| {
            n.key.part.is_none() && !publication.nodes().iter().all(|c| c.parent != Some(n.key))
        })
        .context("missing container parent AX node")?;
    let expected_range = AccessibilityRange::new(0.0, 100.0).unwrap();
    let slider_nodes: Vec<_> = publication
        .nodes()
        .iter()
        .filter(|n| matches!(n.role, AccessibilityRole::Slider(r) if r == expected_range))
        .collect();
    let editor_nodes: Vec<_> = publication
        .nodes()
        .iter()
        .filter(|n| {
            matches!(n.role, AccessibilityRole::TextField) && n.label.starts_with("RemoteField-")
        })
        .collect();
    ensure!(
        slider_nodes.len() == 16,
        "expected 16 remote sliders in prepared AX publication, got {}",
        slider_nodes.len()
    );
    ensure!(
        editor_nodes.len() == 8,
        "expected 8 remote editors in prepared AX publication, got {}",
        editor_nodes.len()
    );
    let layout_count = publication
        .text_layouts()
        .context("missing publication text_layouts")?
        .len();
    ensure!(
        layout_count >= 8,
        "expected remote editor text layouts in prepared AX publication, got {}",
        layout_count
    );
    let _ = container_node;

    println!(
        "CURVE_AX_PUBLICATION sliders={} editors={} text_layouts={} stage=prepare_only",
        slider_nodes.len(),
        editor_nodes.len(),
        layout_count
    );
    Ok(())
}
