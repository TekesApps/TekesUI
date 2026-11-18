//! Original icon catalogue mounted in AppHost's single input/text/resource host.
use super::{
    super::{
        icon_catalog::IconCatalog,
        pages::icons::{self, IconText},
    },
    Entry,
};
use anyhow::Result;
use std::{cell::RefCell, rc::Rc};
use tekes_ui_app_host::ContainerRenderContext;
use tekes_ui_controls::{ButtonTextRenderer, composition::*, layout::MeasuredChild};
use tekes_ui_scene::{Point, Rect, Scene, Size};
use tekes_ui_text::Paragraph;
use tekes_ui_tokens::{Rgba, Theme, proposal};

const ROOT: ComponentId = ComponentId(98500);
const ROWS: ComponentId = ComponentId(98501);
const BODY: ComponentId = ComponentId(98505);
const COMMAND: u64 = 98600;
pub(crate) const SEARCH: ComponentId = ComponentId(98502);
pub(crate) const SPECIMEN: ComponentId = ComponentId(98520);

struct HostText<'a> {
    text: &'a mut dyn ButtonTextRenderer,
    clip: Rect,
    type_scale: f32,
}
impl IconText for HostText<'_> {
    fn type_scale(&self) -> f32 {
        self.type_scale
    }
    fn scale(&self) -> f32 {
        self.text.backing_scale()
    }
    fn proposal_label(
        &mut self,
        value: &str,
        role: proposal::TextRole,
        width: Option<f32>,
    ) -> Rc<Paragraph> {
        let value = super::super::locale::Locale::default()
            .text(value)
            .into_owned();
        self.text
            .text_surface_host()
            .expect("preflighted icon text service")
            .shape_surface(&value, role, width)
    }
    fn proposal_label_in(
        &mut self,
        scene: &mut Scene,
        _: Theme,
        slot: Rect,
        value: &str,
        role: proposal::TextRole,
        ink: Rgba,
    ) -> f32 {
        let p = self.proposal_label(value, role, None);
        let origin = Point::new(
            slot.min_x().round(),
            (slot.min_y() + (slot.size.height - p.height as f32) * 0.5).round(),
        );
        scene.push_clip(slot);
        self.text
            .text_surface_host()
            .expect("preflighted icon text service")
            .push_visible(scene, &p, origin, ink, self.clip);
        scene.pop_clip();
        p.width as f32
    }
    fn proposal_wrapped_in(
        &mut self,
        scene: &mut Scene,
        _: Theme,
        slot: Rect,
        value: &str,
        role: proposal::TextRole,
        ink: Rgba,
    ) -> usize {
        let p = self.proposal_label(value, role, Some(slot.size.width));
        scene.push_clip(slot);
        self.text
            .text_surface_host()
            .expect("preflighted icon text service")
            .push_visible(
                scene,
                &p,
                Point::new(slot.min_x().round(), slot.min_y().round()),
                ink,
                self.clip,
            );
        scene.pop_clip();
        p.lines
            .iter()
            .filter(|line| line.top < f64::from(slot.size.height))
            .count()
    }
}
#[derive(Debug)]
struct State {
    catalog: IconCatalog,
    query: String,
    error: Option<IconRenderError>,
}
#[derive(Debug)]
struct Catalogue(Rc<RefCell<State>>);
impl Component for Catalogue {
    fn requires_icon_services(&self) -> bool {
        true
    }
    fn requires_text_services(&self) -> bool {
        true
    }
    fn icon_error(&self) -> Option<IconRenderError> {
        self.0.borrow().error
    }
    fn measure(&mut self, text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        let scale = text
            .text_surface_host()
            .expect("preflighted icon text service")
            .type_scale();
        let state = self.0.borrow();
        MeasuredChild::new(icons::extent_at_scale(
            &state.catalog,
            &state.query,
            available.width,
            scale,
        ))
    }
    fn paint(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        frame: Rect,
        clip: Rect,
        theme: ContentTheme,
    ) {
        let type_scale = text
            .text_surface_host()
            .expect("preflighted icon text service")
            .type_scale();
        let scale = text.backing_scale();
        let mut state = self.0.borrow_mut();
        let visible =
            icons::visible_legacy_at_scale(&state.catalog, &state.query, frame, clip, type_scale);
        // ScrollContent has already arranged this exact frame and clip. The
        // short service borrow registers every visible image before scene IDs
        // are emitted, including the first frame and newly scrolled-in cards.
        let prepared = text
            .icon_renderer()
            .ok_or(IconRenderError::UnsupportedRenderer)
            .and_then(|renderer| {
                state
                    .catalog
                    .prepare_host(renderer, theme.palette.theme, scale, &visible)
            });
        state.error = prepared.err();
        if state.error.is_some() {
            return;
        }
        scene.push_clip(clip);
        icons::build_catalog_with_text(
            &mut HostText {
                text,
                clip,
                type_scale,
            },
            theme.palette.theme,
            frame,
            scene,
            &state.query,
            &state.catalog,
            clip,
        );
        scene.pop_clip();
    }
}

pub(crate) struct AppHostIcons {
    pub(crate) host: ContentHost,
    state: Rc<RefCell<State>>,
    entry: &'static Entry,
    section: usize,
    retained: Option<Content>,
}
impl AppHostIcons {
    pub(crate) fn new(entry: &'static Entry) -> Result<Self> {
        Self::with_catalog(entry, IconCatalog::from_env())
    }
    fn with_catalog(entry: &'static Entry, catalog: IconCatalog) -> Result<Self> {
        let state = Rc::new(RefCell::new(State {
            catalog,
            query: String::new(),
            error: None,
        }));
        let mut host = ContentHost::new(Content::new(
            ROOT,
            ListContent::scrollable(
                "Icon catalogue",
                ROWS,
                vec![
                    super::story_heading(98503, entry.title()),
                    Content::new(
                        ComponentId(98504),
                        TextContent::new(super::story_docs(entry)),
                    ),
                    super::apphost_basic::section_bar(entry, COMMAND),
                    Self::playground(state.clone()),
                ],
            )
            .content_padding(16.),
        ))
        .map_err(|e| anyhow::anyhow!("Icons host: {e:?}"))?;
        host.set_action_events(true);
        Ok(Self {
            host,
            state,
            entry,
            section: 1,
            retained: None,
        })
    }
    fn playground(state: Rc<RefCell<State>>) -> Content {
        let mut search = SearchFieldContent::new("Search icon catalogue", &state.borrow().query);
        search.set_placeholder(Some("Search icon names".into()));
        Content::new(
            BODY,
            ListContent::new(
                "Playground",
                vec![
                    Content::new(ComponentId(98506), TextContent::new("Playground")),
                    Content::new(SEARCH, search),
                    Content::new(SPECIMEN, Catalogue(state)),
                ],
            ),
        )
    }
    fn section_body(&mut self) -> Content {
        if self.section == 1 {
            return self
                .retained
                .take()
                .unwrap_or_else(|| Self::playground(self.state.clone()));
        }
        let copy = if self.section == 0 {
            format!(
                "Docs\nComponent: {} | maturity: experimental\nUsage: {}\nStory routes: {}\nState is local to this route. Reset restores deterministic fixture data.",
                self.entry.component(),
                super::api_reference(self.entry),
                ["docs", "playground", "behaviors"]
                    .map(|section| format!("{}/{section}", self.entry.slug()))
                    .join(" · ")
            )
        } else {
            format!(
                "Behaviors\n{}\nState matrices are deterministic visual comparisons. Use Playground for real input. Native VoiceOver and physical IME checks remain manual.",
                super::behavior_copy(self.entry.renderer())
            )
        };
        Content::new(BODY, TextContent::new(copy))
    }
    pub(crate) fn select_section(&mut self, section: usize) {
        if self.section == section {
            return;
        }
        let old = self.host.remove(BODY).expect("original Icons body");
        if self.section == 1 {
            self.retained = Some(old);
        }
        self.section = section;
        let body = self.section_body();
        self.host.insert(ROWS, body).expect("unique Icons body");
    }
    pub(crate) fn after_host_visit(&mut self) {
        if let Some(search) = self
            .host
            .get(SEARCH)
            .and_then(|node| node.component::<SearchFieldContent>())
        {
            let query = search.query();
            if self.state.borrow().query != query {
                self.state.borrow_mut().query = query.to_owned();
                let _ = self.host.get_mut(SPECIMEN);
            }
        }
        for record in self.host.drain_action_records() {
            if record.action.kind != ContentActionKind::Activate {
                continue;
            }
            let (section, reset) = match record.action.target.0.checked_sub(COMMAND) {
                Some(0) => (self.section, true),
                Some(1) => (0, false),
                Some(2) => (1, false),
                Some(4) => (3, false),
                _ => continue,
            };
            if reset {
                self.host.remove(BODY).expect("original Icons body");
                self.retained = None;
                {
                    let mut state = self.state.borrow_mut();
                    state.query.clear();
                    state.error = None;
                }
                let body = self.section_body();
                self.host.insert(ROWS, body).expect("unique Icons body");
            } else {
                self.select_section(section);
            }
        }
    }

    pub(crate) fn prepare_resources(&mut self, _: &mut ContainerRenderContext<'_>) -> Result<()> {
        Ok(())
    }
    pub(crate) fn finish_resources(&mut self, _: &mut ContainerRenderContext<'_>) -> Result<()> {
        Ok(())
    }
    #[cfg(feature = "selftest")]
    pub(crate) fn scroll_metrics(&self) -> (f32, f32) {
        self.host
            .get(ROOT)
            .and_then(|node| node.component::<ScrollContent>())
            .map_or((0., 0.), |scroll| {
                (
                    scroll.scroll.limit().y,
                    scroll.scroll.viewport_size().height,
                )
            })
    }
    #[cfg(feature = "selftest")]
    pub(crate) fn scroll_to(&mut self, y: f32) {
        if let Some(scroll) = self
            .host
            .get_mut(ROOT)
            .and_then(|node| node.component_mut::<ScrollContent>())
        {
            let current = scroll.scroll.offset.y;
            scroll.scroll.scroll_by(0., y - current);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gallery::text::TextEngine;
    use tekes_ui_controls::layout_view::IconRenderer;
    use tekes_ui_scene::{ImageId, Primitive};
    use tekes_ui_tokens::Palette;

    #[derive(Default)]
    struct Images {
        identity: u64,
        uploads: Vec<(ImageId, u32, std::rc::Weak<()>)>,
        fail: bool,
    }
    impl IconRenderer for Images {
        fn resource_identity(&self) -> Option<u64> {
            Some(self.identity)
        }
        fn register_icon_bitmap(
            &mut self,
            bytes: &[u8],
            width: u32,
            height: u32,
        ) -> Result<IconImage, IconRenderError> {
            if self.fail {
                return Err(IconRenderError::Upload);
            }
            assert_eq!(bytes.len(), (width * height * 4) as usize);
            assert!(bytes.chunks_exact(4).any(|p| p[3] > 0));
            let id = ImageId(self.identity as u32 * 10000 + self.uploads.len() as u32 + 1);
            let lease = Rc::new(());
            self.uploads.push((id, width, Rc::downgrade(&lease)));
            Ok(IconImage::new(id, lease))
        }
        fn draw_icon(&mut self, _: &mut Scene, _: &str, _: Rect, _: Rgba) {}
    }
    fn entry() -> &'static Entry {
        super::super::entries()
            .iter()
            .find(|entry| entry.slug() == "component-icon")
            .unwrap()
    }
    fn page() -> AppHostIcons {
        let names: Vec<_> = (0..24).map(|i| format!("catalog-{i:02}")).collect();
        let mut catalog =
            IconCatalog::fixture(&names.iter().map(String::as_str).collect::<Vec<_>>());
        for entry in &mut catalog.entries {
            entry.kind = "refine".into();
        }
        catalog.entries[0].before = Some(Box::new(
            IconCatalog::fixture(&["historical"]).entries.remove(0),
        ));
        AppHostIcons::with_catalog(entry(), catalog).unwrap()
    }
    fn draw(
        page: &mut AppHostIcons,
        text: &mut TextEngine,
        images: &mut Images,
        width: f32,
    ) -> Result<Scene, CompositionError> {
        text.begin_frame();
        let mut scene = Scene::new();
        let frame = Rect::new(0., 0., width, 1100.);
        page.host.render(
            &mut scene,
            &mut WithIconRenderer::new(text, images),
            frame,
            frame,
            Palette::from(Theme::LIGHT),
        )?;
        page.after_host_visit();
        Ok(scene)
    }
    #[test]
    fn apphost_icons_first_frame_scroll_reuse_and_renderer_replacement() {
        let mut page = page();
        let mut text = TextEngine::new();
        let mut images = Images {
            identity: 1,
            ..Default::default()
        };
        let scene = draw(&mut page, &mut text, &mut images, 800.).unwrap();
        assert!(
            scene
                .primitives()
                .iter()
                .any(|p| matches!(p, Primitive::Image(_)))
        );
        let first = images.uploads.len();
        assert!(first >= 6);
        {
            let state = page.state.borrow();
            let entry = &state.catalog.entries[0];
            for id in entry
                .images
                .iter()
                .chain(entry.before.as_ref().unwrap().images.iter())
            {
                let id = id.expect("both current and historical registered in first frame");
                assert!(
                    scene
                        .primitives()
                        .iter()
                        .any(|p| matches!(p, Primitive::Image(image) if image.image == id))
                );
            }
        }
        assert_eq!(
            &images.uploads[..3]
                .iter()
                .map(|(_, size, _)| *size)
                .collect::<Vec<_>>(),
            // TextEngine defaults to 2x backing: 12/16/24 logical points
            // register 24/32/48-pixel bitmaps without enlarging scene geometry.
            &[24, 32, 48]
        );
        draw(&mut page, &mut text, &mut images, 800.).unwrap();
        assert_eq!(images.uploads.len(), first, "unchanged frame reuses leases");
        page.host
            .get_mut(ROOT)
            .unwrap()
            .component_mut::<ScrollContent>()
            .unwrap()
            .scroll
            .scroll_by(0., 1600.);
        let scrolled = draw(&mut page, &mut text, &mut images, 800.).unwrap();
        assert!(
            images.uploads.len() > first,
            "new viewport prepares new cards in that same frame"
        );
        assert!(
            scrolled
                .primitives()
                .iter()
                .any(|p| matches!(p, Primitive::Image(_)))
        );
        let old_leases: Vec<_> = images
            .uploads
            .iter()
            .map(|(_, _, weak)| weak.clone())
            .collect();
        let mut replacement = Images {
            identity: 2,
            ..Default::default()
        };
        let scene = draw(&mut page, &mut text, &mut replacement, 800.).unwrap();
        assert!(!replacement.uploads.is_empty());
        assert!(old_leases.iter().all(|weak| weak.upgrade().is_none()));
        for primitive in scene.primitives() {
            if let Primitive::Image(image) = primitive {
                assert!(image.image.0 >= 20000);
            }
        }
    }
    #[test]
    fn apphost_icons_search_clear_retains_original_host_at_narrow_scaled_width() {
        let mut page = page();
        let mut text = TextEngine::new();
        text.set_type_scale(1.5);
        let mut images = Images::default();
        draw(&mut page, &mut text, &mut images, 320.).unwrap();
        assert!(page.host.focus(Some(SEARCH), false));
        page.host.event(&ContentEvent::Commit {
            text: "CATALOG-03".into(),
            replacement: None,
        });
        page.after_host_visit();
        let scene = draw(&mut page, &mut text, &mut images, 320.).unwrap();
        assert_eq!(page.state.borrow().query, "CATALOG-03");
        assert_eq!(
            scene
                .primitives()
                .iter()
                .filter(|p| matches!(p, Primitive::Image(_)))
                .count(),
            3
        );
        page.host.set_window_active(false);
        page.host.set_window_active(true);
        assert_eq!(
            page.host
                .get(SEARCH)
                .unwrap()
                .component::<SearchFieldContent>()
                .unwrap()
                .query(),
            "CATALOG-03"
        );
        page.host
            .accessibility_action(SEARCH, Some(0), AccessibilityAction::Press);
        page.after_host_visit();
        draw(&mut page, &mut text, &mut images, 320.).unwrap();
        assert!(page.state.borrow().query.is_empty());
        assert!(page.host.get(SEARCH).is_some());
    }
    #[test]
    fn apphost_icons_sections_retain_editor_and_reset_original_query() {
        let mut page = page();
        let mut text = TextEngine::new();
        let mut images = Images::default();
        draw(&mut page, &mut text, &mut images, 800.).unwrap();
        page.host.focus(Some(SEARCH), false);
        page.host.event(&ContentEvent::Commit {
            text: "catalog-03".into(),
            replacement: None,
        });
        page.after_host_visit();
        draw(&mut page, &mut text, &mut images, 800.).unwrap();
        let baseline = page
            .host
            .get(SEARCH)
            .unwrap()
            .component::<SearchFieldContent>()
            .unwrap()
            .input()
            .external_baseline();
        for command in [1, 4] {
            page.host.accessibility_action(
                ComponentId(COMMAND + command),
                None,
                AccessibilityAction::Press,
            );
            page.after_host_visit();
            draw(&mut page, &mut text, &mut images, 800.).unwrap();
            assert!(page.host.get(SEARCH).is_none());
            let copy = &page
                .host
                .get(BODY)
                .unwrap()
                .component::<TextContent>()
                .unwrap()
                .text;
            assert!(copy.contains(if command == 1 {
                super::super::api_reference(entry())
            } else {
                super::super::behavior_copy("icon")
            }));
        }
        page.host
            .accessibility_action(ComponentId(COMMAND + 2), None, AccessibilityAction::Press);
        page.after_host_visit();
        draw(&mut page, &mut text, &mut images, 800.).unwrap();
        let search = page
            .host
            .get(SEARCH)
            .unwrap()
            .component::<SearchFieldContent>()
            .unwrap();
        assert_eq!(search.query(), "catalog-03");
        assert_eq!(search.input().external_baseline(), baseline);
        page.host
            .accessibility_action(ComponentId(COMMAND), None, AccessibilityAction::Press);
        page.after_host_visit();
        draw(&mut page, &mut text, &mut images, 800.).unwrap();
        assert!(page.state.borrow().query.is_empty());
        assert!(
            page.host
                .get(SEARCH)
                .unwrap()
                .component::<SearchFieldContent>()
                .unwrap()
                .query()
                .is_empty()
        );
    }
    #[test]
    fn apphost_icons_missing_service_and_upload_failure_are_host_errors() {
        let mut page = page();
        let mut text = TextEngine::new();
        let frame = Rect::new(0., 0., 800., 700.);
        let mut scene = Scene::new();
        assert!(matches!(
            page.host.render(
                &mut scene,
                &mut text,
                frame,
                frame,
                Palette::from(Theme::LIGHT)
            ),
            Err(CompositionError::MissingIconServices(SPECIMEN))
        ));
        struct Unsupported;
        impl IconRenderer for Unsupported {
            fn draw_icon(&mut self, _: &mut Scene, _: &str, _: Rect, _: Rgba) {}
        }
        assert!(
            page.host
                .render(
                    &mut Scene::new(),
                    &mut WithIconRenderer::new(&mut text, &mut Unsupported),
                    frame,
                    frame,
                    Palette::from(Theme::LIGHT)
                )
                .is_err()
        );
        assert_eq!(
            page.state.borrow().error,
            Some(IconRenderError::UnsupportedRenderer)
        );
        let mut images = Images {
            fail: true,
            ..Default::default()
        };
        assert!(draw(&mut page, &mut text, &mut images, 800.).is_err());
        assert_eq!(page.state.borrow().error, Some(IconRenderError::Upload));
        images.fail = false;
        draw(&mut page, &mut text, &mut images, 800.).unwrap();
        assert_eq!(page.state.borrow().error, None);
    }
    #[test]
    fn apphost_icons_gpu_first_frame_has_original_registered_pixels() {
        let mut page = page();
        let mut text = TextEngine::new();
        let mut renderer =
            tekes_ui_render::Renderer::new_headless().expect("controlled Metal icon check");
        let mut cache = SvgIconCache::default();
        let frame = Rect::new(0., 0., 800., 1100.);
        text.begin_frame();
        let mut scene = Scene::new();
        {
            let mut service = cache.renderer(&mut renderer, 1.);
            page.host
                .render(
                    &mut scene,
                    &mut WithIconRenderer::new(&mut text, &mut service),
                    frame,
                    frame,
                    Palette::from(Theme::LIGHT),
                )
                .unwrap();
        }
        let pixels = renderer
            .render_to_rgba8(
                &scene,
                text.atlas_mut(),
                (800, 1100),
                1.,
                crate::gallery::Gallery::background(Theme::LIGHT),
            )
            .unwrap();
        let ink = tekes_ui_tokens::ColorRole::PrimaryText.resolve(Theme::LIGHT);
        let expected = [ink.r, ink.g, ink.b].map(|v| (v * 255.).round() as i16);
        let mut sampled = 0;
        for p in scene.primitives() {
            if let Primitive::Image(image) = p {
                let center = image.rect.center();
                if !(0. ..800.).contains(&center.x) || !(0. ..1100.).contains(&center.y) {
                    continue;
                }
                let index = (center.y as usize * 800 + center.x as usize) * 4;
                for channel in 0..3 {
                    assert!(
                        (i16::from(pixels[index + channel]) - expected[channel]).abs() <= 2,
                        "original solid SVG center must be painted in the first frame"
                    );
                }
                sampled += 1;
            }
        }
        assert!(sampled >= 3);
    }
}
