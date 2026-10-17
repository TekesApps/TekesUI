//! Gallery 自己的状态与内容。
//!
//! 与平台层的耦合只有 `handle_input` 的 `Event` 参数和 `menu_bar()` 用到的菜单描述类型；
//! 坐标一律是逻辑点，左上角为原点。不依赖窗口和渲染器，所以可以在测试里直接驱动。
//!
//! 模块：
//! - `layout`：固定 JSON 捕获框、目录列几何、命中测试；
//! - `chrome`：目录列等外壳的绘制；
//! - `containers`：`StateMatrix`、`ReferenceSlot` 和它们的标题文字；
//! - `text`：文字引擎（排版器、光栅化器、字体登记表、`GlyphAtlas`）和标签缓存；
//! - `static_text`：静态文字页的状态（夹具、宽度、参照图、差异缓存）；
//! - `pages`：各页（`primitives`、`tokens`、`static_text`）；
//! - `commands`：菜单命令 ↔ 编号 ↔ 菜单栏描述；
//! - `theme`：外观与对比度的选择；
//! - `snapshot`：无头渲染所有页 x 主题 x 缩放，写 PNG。
//!
//! 重绘规则：只有可见状态真的变了（hover 行、选中页、生效主题）才返回 `redraw`；
//! 鼠标在同一行内移动、重复选同一页、重复选同一主题都不重绘。

use std::time::Duration;

pub mod catalog;
pub mod chrome;
pub mod commands;
pub mod containers;
pub mod icon_catalog;
pub mod layout;
pub mod locale;
pub mod motion_preview;
pub mod navigation;
pub mod pages;
#[cfg(test)]
mod quality_effects;
pub mod snapshot;
pub mod static_text;
pub mod text;
pub mod text_demo;
pub mod theme;
pub mod viewport;
pub mod workbench;

use tekes_ui_platform_macos::{
    Event, KeyEventKind, MenuBar, MouseButton, MouseEventKind, Point as PlatformPoint,
};
use tekes_ui_render::Renderer;
use tekes_ui_scene::{GlyphAtlas, Point, Rect, Scene, Size};
use tekes_ui_tokens::{ColorRole, Rgba, Theme};

pub use commands::Command;
pub use layout::{FRAME_H as CONTENT_HEIGHT, FRAME_W as CONTENT_WIDTH};
pub use pages::{Page, PageCtx};
use static_text::StaticText;
use text::TextEngine;
#[cfg(test)]
use theme::{AppearanceChoice, ContrastChoice};
pub use theme::{ThemeState, theme_label};

/// 一次状态更新的结果。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Change {
    /// 画面需要重绘。
    pub redraw: bool,
    /// 窗口标题或菜单勾选需要同步（页面或生效主题变了）。
    pub chrome: bool,
}

impl Change {
    pub const NONE: Change = Change {
        redraw: false,
        chrome: false,
    };
    const REDRAW: Change = Change {
        redraw: true,
        chrome: false,
    };
    const BOTH: Change = Change {
        redraw: true,
        chrome: true,
    };
}

// 键盘虚拟键码。
const KEY_DOWN: u16 = 125;
const KEY_UP: u16 = 126;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ScrollOwner {
    Sidebar,
    Document,
    Text(Page),
}

#[derive(Debug)]
pub struct Gallery {
    page: Page,
    hover: Option<usize>,
    theme: ThemeState,
    /// 文字引擎与窗口用的 atlas（主线程）。
    text: TextEngine,
    static_text: StaticText,
    showcase: catalog::ShowcaseState,
    viewport: viewport::Viewport,
    page_offsets: [Point; Page::ALL.len()],
    editor: text_demo::TextDemo,
    text_view: text_demo::TextDemo,
    scroll_demo: text_demo::TextDemo,
    sidebar: navigation::Navigation,
    motion: motion_preview::MotionPreview,
    scroll_owner: Option<ScrollOwner>,
    icon_search: text_demo::TextDemo,
    icons: icon_catalog::IconCatalog,
    workbench: workbench::Workbench,
    navigation_search: text_demo::TextDemo,
    motion_now: Duration,
}

impl Default for Gallery {
    fn default() -> Self {
        Gallery::new()
    }
}

impl Gallery {
    /// 参照目录取自环境变量 `TEKESUI_REFERENCE_DIR`。
    pub fn new() -> Gallery {
        let mut gallery = Gallery::with_static_text(StaticText::from_env());
        gallery.static_text.select_fixture(7);
        if let Some(locale) = std::env::var("TEKESUI_GALLERY_LOCALE")
            .ok()
            .as_deref()
            .and_then(locale::Locale::from_code)
        {
            gallery.set_locale(locale);
        }
        gallery
    }

    pub fn with_static_text(static_text: StaticText) -> Gallery {
        let mut text = TextEngine::new();
        text.set_motion_enabled(true);
        Gallery {
            page: Page::Workbench(0),
            hover: None,
            theme: ThemeState::default(),
            text,
            static_text,
            showcase: catalog::ShowcaseState::default(),
            viewport: viewport::Viewport::default(),
            page_offsets: [Point::default(); Page::ALL.len()],
            editor: text_demo::TextDemo::editors_only(),
            text_view: text_demo::TextDemo::text_only(),
            scroll_demo: text_demo::TextDemo::scroll_only(),
            sidebar: navigation::Navigation::default(),
            motion: motion_preview::MotionPreview::default(),
            scroll_owner: None,
            icon_search: text_demo::TextDemo::search(),
            icons: icon_catalog::IconCatalog::from_env(),
            workbench: workbench::Workbench::default(),
            navigation_search: text_demo::TextDemo::search(),
            motion_now: Duration::ZERO,
        }
    }

    fn content_page(&self) -> Page {
        if let Page::Workbench(i) = self.page {
            if !self.workbench.show_live(i) {
                return self.page;
            }
            match workbench::entry(i).renderer() {
                "scroll" => Page::ScrollArea,
                "text" => Page::Text,
                "icon" => Page::Icons,
                "motion" => Page::Motion,
                "internal-text" => Page::TextEditing,
                _ => self.page,
            }
        } else {
            self.page
        }
    }
    /// Bounds shared by the live page renderer and its pointer hit routing.
    /// A workbench story hosts its renderer inside the preview below the story
    /// header; standalone pages use the viewport document directly.
    fn live_content_bounds(&self) -> tekes_ui_scene::Rect {
        if let Page::Workbench(i) = self.page {
            self.workbench.preview_rect(i)
        } else {
            self.viewport.content()
        }
    }
    #[cfg(target_os = "macos")]
    pub(crate) fn motion_action_center_for_selftest(
        width: f32,
        height: f32,
        text_scale: usize,
        scroll_to_bottom: bool,
        catalog: motion_preview::PresetCatalog,
        action: pages::motion::Action,
    ) -> Point {
        let mut gallery = Self::new();
        gallery.workbench.text_scale = text_scale;
        assert!(gallery.open_route("foundation-motion/playground"));
        gallery.motion.select_catalog(catalog);
        gallery.set_view_size(width, height);
        gallery.build_scene(&mut Scene::new());
        if scroll_to_bottom {
            let layout = gallery
                .motion
                .button_models
                .get(&pages::motion::Layout::action_id(action))
                .and_then(|button| button.layout())
                .expect("Motion action is laid out before scrolling");
            let delta = layout.frame.center().y - gallery.viewport.bounds().center().y;
            gallery.viewport.move_by(0.0, delta);
            gallery.build_scene(&mut Scene::new());
        }
        let point = gallery
            .motion
            .button_models
            .get(&pages::motion::Layout::action_id(action))
            .and_then(|button| button.layout())
            .map(|layout| layout.frame.center())
            .expect("rendered Motion Button has a retained layout");
        assert!(gallery.viewport.bounds().contains(point));
        point
    }

    /// Resolve a TextSurface pointer target from the exact retained layout used
    /// by the Gallery route under test. `index` addresses the live surfaces in
    /// the internal editor route (0 = single line, 1 = multiline).
    #[cfg(target_os = "macos")]
    pub(crate) fn text_surface_center_for_selftest(
        route: &str,
        width: f32,
        height: f32,
        text_scale: usize,
        index: usize,
    ) -> Point {
        let mut gallery = Self::new();
        assert!(gallery.open_route(route), "self-test route {route}");
        gallery.workbench.text_scale = text_scale;
        gallery.set_view_size(width, height);
        gallery.build_scene(&mut Scene::new());
        let demo = match gallery.content_page() {
            Page::TextEditing => &gallery.editor,
            Page::Text => &gallery.text_view,
            Page::ScrollArea => &gallery.scroll_demo,
            Page::Icons => &gallery.icon_search,
            other => panic!("route {route} does not render a TextSurface: {other:?}"),
        };
        let frame = demo
            .visible_frame_at(index)
            .unwrap_or_else(|| panic!("TextSurface {index} is not visible for {route}"));
        let point = frame.center();
        assert!(gallery.viewport.bounds().contains(point));
        if let Page::Workbench(i) = gallery.page {
            assert!(gallery.workbench.preview_rect(i).contains(point));
        }
        point
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn calibration_selector_center_for_selftest(
        width: f32,
        height: f32,
        fixture: usize,
        selector: usize,
        width_selector: bool,
    ) -> Point {
        let mut gallery = Self::new();
        assert!(gallery.open_route("lab-calibration/playground"));
        gallery.workbench.text_scale = 2;
        gallery.workbench.preview_width = 1;
        gallery.static_text.select_fixture(fixture);
        gallery.set_view_size(width, height);
        gallery.build_scene(&mut Scene::new());
        let button = if width_selector {
            gallery
                .static_text
                .width_buttons()
                .get(selector)
                .expect("calibration width selector exists")
        } else {
            gallery
                .static_text
                .fixture_buttons()
                .get(selector)
                .expect("calibration fixture selector exists")
        };
        let center = button
            .layout()
            .expect("canonical calibration Button was rendered")
            .frame
            .center();
        let point = Point::new(
            center.x - gallery.viewport.offset.x,
            center.y - gallery.viewport.offset.y,
        );
        assert!(gallery.viewport.bounds().contains(point));
        point
    }
    pub fn open_route(&mut self, route: &str) -> bool {
        let Some(page) = workbench::route(route) else {
            return false;
        };
        self.apply(Command::Page(page));
        if let Page::Workbench(i) = page {
            self.workbench.select_route(i, route);
        }
        true
    }
    /// 窗口用的 atlas：渲染器画之前上传它的脏区。
    pub fn atlas_mut(&mut self) -> &mut GlyphAtlas {
        self.text.atlas_mut()
    }

    /// 文字引擎（排版、光栅化、窗口 atlas）。性能 harness 用它往同一个 atlas 里编文字段。
    pub fn text_mut(&mut self) -> &mut TextEngine {
        &mut self.text
    }

    /// 每次重绘之前调用：记下窗口倍率，给静态文字页备好参照图和差异（状态没变时什么都不做）。
    pub fn prepare(&mut self, renderer: &mut Renderer, scale: f32) {
        self.layout_page_extent();
        self.text.set_scale(scale);
        if self.content_page() == Page::Icons {
            let icon_type_scale = if matches!(self.page, Page::Workbench(_)) {
                [1.0, 1.25, 1.5][self.workbench.text_scale]
            } else {
                1.0
            };
            let visible = if icon_type_scale == 1.0 {
                pages::icons::visible_legacy(
                    &self.icons,
                    self.icon_search.query(),
                    self.viewport.content(),
                    self.viewport.bounds(),
                )
            } else {
                pages::icons::visible_legacy_at_scale(
                    &self.icons,
                    self.icon_search.query(),
                    self.viewport.content(),
                    self.viewport.bounds(),
                    icon_type_scale,
                )
            };
            self.icons.prepare(renderer, self.theme(), scale, &visible);
        }
        if self.content_page() == Page::Motion {
            let theme = self.theme();
            self.motion.prepare_symbols(renderer, theme, scale);
        }
        if self.static_text_visible() {
            let theme = self.theme();
            self.static_text
                .prepare(&mut self.text, renderer, theme, scale);
        }
    }

    #[cfg(test)]
    pub fn static_text(&self) -> &StaticText {
        &self.static_text
    }

    pub fn page(&self) -> Page {
        self.page
    }

    pub fn hover(&self) -> Option<usize> {
        self.hover
    }

    /// 当前生效的主题。
    pub fn theme(&self) -> Theme {
        self.theme.effective()
    }

    /// 窗口标题：`TekesUI Gallery — <页面名> — <主题名>`。
    pub fn title(&self) -> String {
        format!(
            "TekesUI Gallery — {} — {}",
            self.locale().text(self.page.name()),
            self.locale().text(theme_label(self.theme()))
        )
    }

    /// 窗口背景色（窗口比内容框大时露出来的部分），传给 `Renderer::render` 的 `clear`。
    pub fn background(theme: Theme) -> Rgba {
        ColorRole::DocumentCanvas.resolve(theme)
    }

    /// 命令当前是否应该勾选。
    pub fn is_checked(&self, command: Command) -> bool {
        match command {
            Command::Page(p) => self.page == p,
            Command::Appearance(a) => self.theme.appearance == a,
            Command::Contrast(c) => self.theme.contrast == c,
            Command::Language(l) => self.locale() == l,
            Command::Density(v) => self.workbench.density == v,
            Command::TextScale(v) => self.workbench.text_scale == v,
            Command::Brand(v) => self.workbench.brand == v,
            Command::PreviewWidth(v) => self.workbench.preview_width == v,
            Command::ReducedMotion(v) => self.workbench.reduced == v,
        }
    }

    /// 菜单栏描述，勾选状态取自当前状态。
    pub fn menu_bar(&self) -> MenuBar {
        commands::menu_bar(self.locale(), |c| self.is_checked(c))
    }

    pub fn locale(&self) -> locale::Locale {
        self.text.locale()
    }

    pub fn set_locale(&mut self, locale: locale::Locale) -> Change {
        if self.locale() == locale {
            return Change::NONE;
        }
        self.end_scroll_gestures();
        self.text.set_locale(locale);
        self.motion.locale = locale;
        if self.motion.is_recipe() && self.motion.recipe.kind.is_typing() {
            self.motion.replay();
        }
        self.editor.set_locale(locale);
        self.text_view.set_locale(locale);
        self.scroll_demo.set_locale(locale);
        if self.static_text.fixture() == 0 && locale == locale::Locale::En {
            self.static_text.select_fixture(7);
        } else if self.static_text.fixture() == 7 && locale == locale::Locale::ZhCn {
            self.static_text.select_fixture(0);
        }
        Change::BOTH
    }

    /// 编译当前内容。布局来自 JSON；live 内容适配实际容器。
    ///
    /// 先 `begin_frame`（一帧一次），再整个重编：scene 里的 `AtlasGlyph` 只在这一帧有效。
    pub fn build_scene(&mut self, scene: &mut Scene) {
        self.layout_page_extent();
        let viewport_bounds = self.viewport.bounds();
        let viewport_geometry = self.viewport.geometry_with_ancestor_clip(viewport_bounds);
        let viewport_clip = viewport_geometry
            .visible_viewport_rect()
            .unwrap_or(Rect::new(
                viewport_bounds.min_x(),
                viewport_bounds.min_y(),
                0.0,
                0.0,
            ));
        let mut viewport_content = self.viewport.content_with_geometry(&viewport_geometry);
        if let Page::Workbench(i) = self.page
            && workbench::entry(i).renderer() == "matrix"
        {
            // The brand comparison is a measured scrolling document. Other
            // component previews retain their available viewport height.
            viewport_content.size.height = viewport_content
                .size
                .height
                .max(self.viewport.document_height());
        }
        let content_page = self.content_page();
        self.text.set_type_scale(1.);
        self.text
            .set_motion_context(self.motion_now, self.workbench.reduced);
        self.text.begin_frame();
        let theme = self.theme.effective();
        let mut layer = Scene::new();
        {
            let mut ctx = PageCtx {
                theme,
                text: &mut self.text,
                static_text: &mut self.static_text,
            };
            chrome::draw_with_navigation(
                scene,
                ctx.theme,
                ctx.text,
                self.page,
                self.hover,
                &self.sidebar,
            );
            self.navigation_search.build_search_clipped(
                &mut ctx,
                navigation::search_rect(),
                Some(self.sidebar.bounds()),
                scene,
            );
            if let Page::Workbench(i) = self.page {
                ctx.text
                    .set_type_scale([1., 1.25, 1.5][self.workbench.text_scale]);
                self.workbench.build_clipped(
                    i,
                    &mut ctx,
                    viewport_content,
                    viewport_clip,
                    &mut layer,
                );
                if !self.workbench.show_live(i) {
                    self.viewport.append_visible_with_geometry(
                        &layer,
                        scene,
                        ctx.theme,
                        &viewport_geometry,
                    );
                    return;
                }
            }
        }

        let live_bounds = if matches!(self.page, Page::Workbench(_)) {
            self.live_content_bounds()
        } else {
            viewport_content
        };
        let mut ctx = PageCtx {
            theme,
            text: &mut self.text,
            static_text: &mut self.static_text,
        };
        if let Page::Workbench(_) = self.page {
            ctx.text
                .set_type_scale([1., 1.25, 1.5][self.workbench.text_scale]);
        }
        let text_ancestor_clip = if matches!(self.page, Page::Workbench(_)) {
            live_bounds.intersection(&viewport_clip)
        } else {
            Some(viewport_clip)
        };
        if content_page == Page::Icons {
            self.icon_search.build_search_clipped(
                &mut ctx,
                pages::icons::search_rect(live_bounds),
                text_ancestor_clip,
                &mut layer,
            );
            pages::icons::build_catalog(
                &mut ctx,
                live_bounds,
                &mut layer,
                self.icon_search.query(),
                &self.icons,
                viewport_clip,
            );
        } else if content_page == Page::Motion {
            pages::motion::build(
                &mut ctx,
                live_bounds,
                viewport_clip,
                &mut layer,
                &mut self.motion,
            );
        } else if content_page == Page::StaticText {
            pages::static_text::build_clipped(&mut ctx, live_bounds, viewport_clip, &mut layer);
        } else if content_page == Page::Text {
            self.text_view.build_clipped(
                &mut ctx,
                if matches!(self.page, Page::Workbench(_)) {
                    live_bounds
                } else {
                    viewport_clip
                },
                text_ancestor_clip,
                &mut layer,
            );
        } else if content_page == Page::ScrollArea {
            self.scroll_demo.build_clipped(
                &mut ctx,
                if matches!(self.page, Page::Workbench(_)) {
                    live_bounds
                } else {
                    viewport_clip
                },
                text_ancestor_clip,
                &mut layer,
            );
        } else if content_page == Page::TextEditing {
            self.editor.build_clipped(
                &mut ctx,
                if matches!(self.page, Page::Workbench(_)) {
                    live_bounds
                } else {
                    viewport_clip
                },
                text_ancestor_clip,
                &mut layer,
            );
        } else if let Some(category) = self.page.category() {
            pages::showcase::build(
                category,
                &mut ctx,
                viewport_content,
                &mut layer,
                &self.showcase,
            );
        } else if !matches!(self.page, Page::Workbench(_)) {
            self.page.build(&mut ctx, viewport_content, &mut layer);
        }
        self.viewport
            .append_visible_with_geometry(&layer, scene, ctx.theme, &viewport_geometry);
    }

    pub fn needs_animation(&self) -> bool {
        self.text.motion_frame_requested()
            || self.viewport.needs_animation()
            || self.navigation_search.needs_animation()
            || (self.content_page() == Page::Icons && self.icon_search.needs_animation())
            || self.sidebar.scroll.needs_animation()
            || matches!(self.page,Page::Workbench(i) if self.workbench.needs_animation(i))
            || match self.content_page() {
                Page::TextEditing => self.editor.needs_animation(),
                Page::Text => self.text_view.needs_animation(),
                Page::ScrollArea => self.scroll_demo.needs_animation(),
                Page::Motion => self.motion.needs_animation(),
                _ => false,
            }
    }
    pub fn animate(&mut self, dt: f32) -> bool {
        if dt.is_finite() && dt > 0.0 {
            // Accumulate the host's frame delta; no wall clock or timer thread is used.
            let step = Duration::from_secs_f64(f64::from(dt.min(60.0)));
            self.motion_now = self.motion_now.saturating_add(step);
            self.text
                .set_motion_context(self.motion_now, self.workbench.reduced);
        }
        let component_frame_pending = self.text.motion_frame_requested();
        let mut dirty = self.viewport.tick(dt)
            | self.sidebar.scroll.tick(dt)
            | self.navigation_search.animate(dt);
        if self.content_page() == Page::Icons {
            dirty |= self.icon_search.animate(dt);
        }
        if let Page::Workbench(i) = self.page {
            dirty |= self.workbench.animate(i, dt);
        }
        dirty |= match self.content_page() {
            Page::TextEditing => self.editor.animate(dt),
            Page::Text => self.text_view.animate(dt),
            Page::ScrollArea => self.scroll_demo.animate(dt),
            Page::Motion => self.motion.animate(dt),
            _ => false,
        };
        dirty || component_frame_pending
    }
    fn layout_page_extent(&mut self) {
        self.viewport.set_fill_window(matches!(
            self.content_page(),
            Page::ScrollArea
                | Page::TextEditing
                | Page::Text
                | Page::Icons
                | Page::Motion
                | Page::Workbench(_)
        ));
        use tekes_ui_controls::ScrollPolicy;
        let icon_page = self.content_page() == Page::Icons;
        self.viewport.set_scroll_policy(
            if icon_page {
                ScrollPolicy::Disabled
            } else {
                ScrollPolicy::Auto
            },
            ScrollPolicy::Auto,
        );
        let calibration_extent = match self.page {
            Page::StaticText => Some(Size::new(
                self.viewport.bounds().size.width,
                pages::static_text::document_extent(
                    self.viewport.bounds().size.width,
                    &mut self.text,
                    &self.static_text,
                ),
            )),
            Page::Workbench(i) if workbench::entry(i).renderer() == "calibration" => {
                self.text
                    .set_type_scale([1., 1.25, 1.5][self.workbench.text_scale]);
                let header = self.workbench.preview_header_layout(
                    i,
                    &mut self.text,
                    self.viewport.bounds().size.width,
                );
                let body_height = pages::static_text::document_extent(
                    header.preview_width,
                    &mut self.text,
                    &self.static_text,
                );
                Some(Size::new(
                    self.viewport.bounds().size.width,
                    header.header_height + body_height,
                ))
            }
            _ => None,
        };
        let motion_extent = if self.content_page() == Page::Motion {
            let mut available = self.viewport.bounds();
            let header_height = if let Page::Workbench(i) = self.page {
                self.text
                    .set_type_scale([1., 1.25, 1.5][self.workbench.text_scale]);
                let header = self.workbench.preview_header_layout(
                    i,
                    &mut self.text,
                    self.viewport.bounds().size.width,
                );
                available.size.width = header.preview_width;
                header.header_height
            } else {
                self.text.set_type_scale(1.0);
                0.0
            };
            let motion_size =
                pages::motion::extent_for_state(available, &self.motion, &mut self.text);
            Some(Size::new(
                self.viewport.bounds().size.width,
                header_height + motion_size.height,
            ))
        } else {
            None
        };
        let matrix_extent = if let Page::Workbench(i) = self.content_page()
            && workbench::entry(i).renderer() == "matrix"
        {
            self.text
                .set_type_scale([1.0, 1.25, 1.5][self.workbench.text_scale]);
            Some(Size::new(
                self.viewport.bounds().size.width,
                self.workbench.matrix_document_extent(
                    i,
                    &mut self.text,
                    self.viewport.bounds().size.width,
                ),
            ))
        } else {
            None
        };
        let size = if let Some(size) = motion_extent {
            size
        } else if let Some(size) = matrix_extent {
            size
        } else if let Some(size) = calibration_extent {
            size
        } else if let Page::Workbench(i) = self.content_page() {
            let mut size = self.viewport.bounds().size;
            if ["tokens", "geometry", "shape", "accessibility"]
                .contains(&workbench::entry(i).renderer())
            {
                // Original measured calibration fixtures retain their full extent.
                size.width = size.width.max(layout::PAGE_CONTENT.size.width);
                size.height = size.height.max(1100.);
            }
            size.height = size.height.max(820.);
            size
        } else if matches!(
            self.content_page(),
            Page::ScrollArea | Page::TextEditing | Page::Text
        ) {
            self.viewport.bounds().size
        } else if self.content_page() == Page::Icons {
            let icon_type_scale = if matches!(self.page, Page::Workbench(_)) {
                [1.0, 1.25, 1.5][self.workbench.text_scale]
            } else {
                1.0
            };
            if icon_type_scale == 1.0 {
                pages::icons::extent(
                    &self.icons,
                    self.icon_search.query(),
                    self.viewport.bounds().size.width,
                )
            } else {
                pages::icons::extent_at_scale(
                    &self.icons,
                    self.icon_search.query(),
                    self.viewport.bounds().size.width,
                    icon_type_scale,
                )
            }
        } else {
            layout::PAGE_CONTENT.size
        };
        self.viewport.set_document_extent(size);
    }
    pub fn set_view_size(&mut self, width: f32, height: f32) {
        self.sidebar.resize(height);
        self.viewport.resize(width, height);
        self.layout_page_extent();
    }

    pub fn set_reduced_motion(&mut self, reduced: bool) -> Change {
        self.workbench.apply_reduced_motion(reduced);
        let mut dirty = self.motion.set_system_reduced_motion(reduced);
        dirty |= self.viewport.set_reduced_motion(reduced);
        dirty |= self.sidebar.scroll.set_reduced_motion(reduced);
        dirty |= self.editor.set_reduced_motion(reduced);
        dirty |= self.text_view.set_reduced_motion(reduced);
        dirty |= self.scroll_demo.set_reduced_motion(reduced);
        dirty |= self.icon_search.set_reduced_motion(reduced);
        dirty |= self.navigation_search.set_reduced_motion(reduced);
        redraw_if(dirty)
    }
    pub fn page_document_height(&self) -> f32 {
        self.viewport.document_height()
    }
    pub fn page_scroll_range(&self) -> f32 {
        self.viewport.limit().y
    }
    pub fn scroll_page_to(&mut self, y: f32) {
        let offset = self.viewport.offset.y;
        self.viewport.move_by(0.0, y - offset);
    }
    pub fn motion_retarget(&mut self, target: f64) {
        self.motion.retarget(target);
    }
    pub fn advance_motion(&mut self, dt: f32) {
        self.motion.animate(dt);
    }
    pub fn editor_scroll_metrics(&self) -> (f32, f32) {
        if self.content_page() == Page::ScrollArea {
            self.scroll_demo.scroll_metrics()
        } else {
            self.editor.scroll_metrics()
        }
    }
    pub fn scroll_editor_to(&mut self, offset: f32) {
        if self.content_page() == Page::ScrollArea {
            self.scroll_demo.scroll_to(offset);
        } else {
            self.editor.scroll_to(offset);
        }
    }
    pub fn text_input_state(&self) -> Option<tekes_ui_platform_macos::TextInputState> {
        if let Some(state) = self.navigation_search.state() {
            return Some(state);
        }
        if let Page::Workbench(i) = self.page
            && let Some(state) = self.workbench.text_state(i)
        {
            return Some(state);
        }

        if self.content_page() == Page::TextEditing {
            self.editor.state()
        } else if self.content_page() == Page::Text {
            self.text_view.state()
        } else if self.content_page() == Page::ScrollArea {
            self.scroll_demo.state()
        } else if self.content_page() == Page::Icons {
            self.icon_search.state()
        } else {
            None
        }
    }
    fn reveal_text_caret(&mut self) -> bool {
        let Some(state) = self.text_input_state() else {
            return false;
        };
        let origin = self.viewport.content().origin;
        self.viewport.reveal(tekes_ui_scene::Rect::new(
            state.caret_origin.x as f32 - origin.x,
            state.caret_origin.y as f32 - origin.y,
            state.caret_size.width as f32,
            state.caret_size.height as f32,
        ))
    }
    pub fn text_command(
        &mut self,
        command: &str,
        clipboard: Option<&str>,
    ) -> (Change, Option<String>) {
        if self.navigation_search.state().is_some() {
            let mut copied = None;
            let changed = match command {
                "copy:" => {
                    copied = self.navigation_search.clipboard_selection();
                    false
                }
                "cut:" => {
                    copied = self.navigation_search.clipboard_selection();
                    self.navigation_search.cut()
                }
                "paste:" => self.navigation_search.paste(clipboard.unwrap_or("")),
                _ => self.navigation_search.command(command).unwrap_or(false),
            };
            self.navigation_search.refresh(&mut self.text);
            self.sidebar.set_query(self.navigation_search.query());
            return (redraw_if(changed), copied);
        }
        if let Page::Workbench(i) = self.page
            && self.workbench.text_state(i).is_some()
        {
            let (changed, copied) =
                self.workbench
                    .text_command(i, command, clipboard, &mut self.text);
            return (redraw_if(changed), copied);
        }
        let editor = match self.content_page() {
            Page::TextEditing => &mut self.editor,
            Page::Text => &mut self.text_view,
            Page::ScrollArea => &mut self.scroll_demo,
            Page::Icons => &mut self.icon_search,
            _ => return (Change::NONE, None),
        };
        let mut copied = None;
        let changed = match command {
            "copy:" => {
                copied = editor.clipboard_selection();
                false
            }
            "cut:" => {
                copied = editor.clipboard_selection();
                editor.cut()
            }
            "paste:" => editor.paste(clipboard.unwrap_or("")),
            _ => editor.command(command).unwrap_or(false),
        };
        editor.refresh(&mut self.text);
        let revealed = changed && self.reveal_text_caret();
        (redraw_if(changed || revealed), copied)
    }

    /// 平台层投来系统外观变化。生效主题变了才需要重绘。
    pub fn set_system_theme(&mut self, system: Theme) -> Change {
        let before = self.theme();
        self.theme.system = system;
        if self.theme() == before {
            Change::NONE
        } else {
            Change::BOTH
        }
    }

    /// 执行菜单命令。状态没变（例如重复选当前页）返回 `Change::NONE`。
    pub fn apply(&mut self, command: Command) -> Change {
        let before = self.theme();
        match command {
            Command::Language(locale) => return self.set_locale(locale),
            Command::Density(v) => {
                self.workbench.density = v;
                return Change::BOTH;
            }
            Command::TextScale(v) => {
                self.workbench.text_scale = v;
                return Change::BOTH;
            }
            Command::Brand(v) => {
                self.workbench.brand = v;
                return Change::BOTH;
            }
            Command::PreviewWidth(v) => {
                self.workbench.preview_width = v;
                return Change::BOTH;
            }
            Command::ReducedMotion(v) => {
                self.set_reduced_motion(v);
                return Change::BOTH;
            }
            Command::Page(p) => {
                if self.page == p {
                    return Change::NONE;
                }
                self.end_scroll_gestures();
                self.workbench.blur();
                self.navigation_search.blur();
                self.editor.blur();
                self.text_view.blur();
                self.scroll_demo.blur();
                self.motion.deactivate();
                self.static_text.blur();
                self.icon_search.blur();
                self.page_offsets[self.page.index()] = self.viewport.offset;
                self.viewport.cancel();
                self.page = p;
                self.sidebar.reveal(p);
                self.layout_page_extent();
                self.viewport.offset = self.page_offsets[p.index()];
                self.viewport.move_by(0.0, 0.0);
                return Change::BOTH;
            }
            Command::Appearance(a) => {
                if self.theme.appearance == a {
                    return Change::NONE;
                }
                self.theme.appearance = a;
            }
            Command::Contrast(c) => {
                if self.theme.contrast == c {
                    return Change::NONE;
                }
                self.theme.contrast = c;
            }
        }
        // 选项变了菜单勾选就要同步；画面只在生效主题变了时才需要重画。
        Change {
            redraw: self.theme() != before,
            chrome: true,
        }
    }

    /// 切到相邻的页（`delta` 为 ±1），到头不循环。
    fn step_page(&mut self, delta: isize) -> Change {
        let pages = navigation::pages();
        let index = pages.iter().position(|p| *p == self.page).unwrap_or(0);
        let next = index as isize + delta;
        match usize::try_from(next).ok().and_then(|i| pages.get(i)) {
            Some(p) => self.apply(Command::Page(*p)),
            None => Change::NONE,
        }
    }

    fn end_scroll_gestures(&mut self) {
        self.sidebar.scroll.end_gesture();
        self.viewport.end_gesture();
        self.editor.end_scroll_gesture();
        self.text_view.end_scroll_gesture();
        self.scroll_demo.end_scroll_gesture();
        self.icon_search.end_scroll_gesture();
        self.scroll_owner = None;
    }
    fn scroll_target_at(&self, p: Point) -> Option<ScrollOwner> {
        if self.sidebar.bounds().contains(p) {
            return Some(ScrollOwner::Sidebar);
        }
        if !self.viewport.bounds().contains(p) {
            return None;
        }
        let editor = match self.content_page() {
            Page::TextEditing => Some(&self.editor),
            Page::Text => Some(&self.text_view),
            Page::ScrollArea => Some(&self.scroll_demo),
            _ => None,
        };
        if editor.is_some_and(|e| e.contains_visible(p)) {
            Some(ScrollOwner::Text(self.content_page()))
        } else if self.content_page() == Page::ScrollArea {
            None
        } else {
            Some(ScrollOwner::Document)
        }
    }
    fn handle_scroll(&mut self, event: &tekes_ui_platform_macos::ScrollEvent) -> Change {
        use tekes_ui_platform_macos::ScrollPhase;
        let point = to_point(event.position);
        let phased = event.precise
            && (event.phase != ScrollPhase::None || event.momentum_phase != ScrollPhase::None);
        let begins = matches!(event.phase, ScrollPhase::Began | ScrollPhase::MayBegin);
        if begins || !phased {
            self.end_scroll_gestures();
            self.scroll_owner = self.scroll_target_at(point);
        }
        let owner = if phased {
            let owner = self.scroll_owner.or_else(|| self.scroll_target_at(point));
            self.scroll_owner = owner;
            owner
        } else {
            self.scroll_target_at(point)
        };
        let Some(owner) = owner else {
            return Change::NONE;
        };
        let mut handled = None;
        if let ScrollOwner::Text(page) = owner {
            let editor = match page {
                Page::Text => &mut self.text_view,
                Page::ScrollArea => &mut self.scroll_demo,
                _ => &mut self.editor,
            };
            handled = editor.handle(&Event::Scroll(*event), Some(point));
        }
        let changed = if let Some(changed) = handled {
            changed
        } else {
            let step = if event.precise { 1.0 } else { layout::ROW_H };
            let x = -(event.delta.x as f32) * step;
            let y = -(event.delta.y as f32) * step;
            let (x, y) = if event.modifiers.shift {
                (x + y, 0.0)
            } else {
                (x, y)
            };
            let end = matches!(event.phase, ScrollPhase::Ended | ScrollPhase::Cancelled)
                || matches!(
                    event.momentum_phase,
                    ScrollPhase::Ended | ScrollPhase::Cancelled
                );
            let area = if owner == ScrollOwner::Sidebar {
                &mut self.sidebar.scroll
            } else {
                &mut *self.viewport
            };
            area.wheel_at(
                Point::new(x, y),
                event.precise,
                event.phase == ScrollPhase::Began,
                end,
                event.momentum_phase != ScrollPhase::None,
                event.timestamp,
            )
        };
        if matches!(
            event.momentum_phase,
            ScrollPhase::Ended | ScrollPhase::Cancelled
        ) || event.phase == ScrollPhase::Cancelled
        {
            self.scroll_owner = None;
        }
        redraw_if(changed)
    }

    /// 更新 hover 行，变了返回 true。
    pub fn set_hover(&mut self, hover: Option<usize>) -> bool {
        if self.hover == hover {
            return false;
        }
        self.hover = hover;
        true
    }

    /// 处理输入事件。
    pub fn handle_input(&mut self, event: &Event) -> Change {
        if let Event::Focus(active) = event {
            self.workbench
                .set_window_active_with_time(*active, self.motion_now);
            self.editor.set_window_active(*active);
            self.text_view.set_window_active(*active);
            self.scroll_demo.set_window_active(*active);
            self.icon_search.set_window_active(*active);
            self.navigation_search.set_window_active(*active);
            if !active {
                self.end_scroll_gestures();
                self.motion.deactivate();
                self.static_text.blur();
                self.sidebar.scroll.cancel();
                self.viewport.cancel();
            }
            return Change::REDRAW;
        }

        let point = match event {
            Event::Mouse(m) => Some(to_point(m.position)),
            _ => None,
        };
        let search_hit = point.is_some_and(|p| {
            self.sidebar.bounds().contains(p) && navigation::search_rect().contains(p)
        });
        if let Event::Mouse(m) = event
            && matches!(m.kind, MouseEventKind::Down { .. })
            && !search_hit
        {
            self.navigation_search.blur();
        }
        if (search_hit || self.navigation_search.state().is_some())
            && let Some(changed) = self.navigation_search.handle(event, point)
        {
            self.navigation_search.refresh(&mut self.text);
            self.sidebar.set_query(self.navigation_search.query());
            return redraw_if(changed);
        }
        if self.static_text_visible() {
            if let Event::Key(key) = event {
                let mapped = match key.key_code {
                    48 => Some(tekes_ui_controls::ControlKey::Tab {
                        reverse: key.modifiers.shift,
                    }),
                    36 => Some(tekes_ui_controls::ControlKey::Enter),
                    49 => Some(tekes_ui_controls::ControlKey::Space),
                    _ => None,
                };
                if mapped.is_some_and(|control| {
                    self.static_text
                        .handle_key(control, key.kind == KeyEventKind::Down)
                }) {
                    return Change::REDRAW;
                }
            }
            if let Some(consumed) = self.handle_static_text_pointer(event) {
                return if consumed {
                    Change::REDRAW
                } else {
                    Change::NONE
                };
            }
        }
        if let Page::Workbench(i) = self.page {
            let in_preview = point.is_none_or(|p| p.x >= layout::CATALOG_W);
            if in_preview {
                if let Some(p) = point
                    && matches!(event,Event::Mouse(m) if matches!(m.kind,MouseEventKind::Down{..}))
                    && let Some(destination) = self.workbench.specimen_destination(i, p)
                {
                    return self.apply(Command::Page(destination));
                }
                if self.workbench.handle(i, event, &mut self.text) {
                    if matches!(event, Event::Key(key) if key.kind == KeyEventKind::Down)
                        && let Some(frame) = self.workbench.focused_control_frame(i)
                    {
                        let origin = self.viewport.geometry().content_origin();
                        self.viewport
                            .reveal_focus(frame.offset(-origin.x, -origin.y), self.text.scale());
                    }
                    return Change::REDRAW;
                }
            }
        }
        if let Event::Scroll(e) = event {
            return self.handle_scroll(e);
        }
        if let Event::Mouse(m) = event {
            let p = to_point(m.position);
            match m.kind {
                MouseEventKind::Down {
                    button: MouseButton::Left,
                    ..
                } if self.sidebar.pointer_down(p) => return Change::REDRAW,
                MouseEventKind::Down {
                    button: MouseButton::Left,
                    ..
                } if self.viewport.pointer_down(p) => {
                    self.editor.blur();
                    self.text_view.blur();
                    self.scroll_demo.blur();
                    self.icon_search.blur();
                    return Change::REDRAW;
                }
                MouseEventKind::Dragged { .. } if self.sidebar.scroll.dragging_axis().is_some() => {
                    return redraw_if(self.sidebar.pointer_drag(p));
                }
                MouseEventKind::Dragged { .. } if self.viewport.dragging_axis().is_some() => {
                    return redraw_if(self.viewport.pointer_drag(p));
                }
                MouseEventKind::Up { .. } if self.viewport.dragging_axis().is_some() => {
                    self.viewport.pointer_up();
                    return Change::NONE;
                }
                MouseEventKind::Up { .. } => self.sidebar.scroll.pointer_up(),
                _ => {}
            }
        }
        let mut search_blurred = false;
        if self.content_page() == Page::Icons {
            let point = match event {
                Event::Mouse(m) => Some(to_point(m.position)),
                _ => None,
            };
            let search_rect = pages::icons::search_rect(if let Page::Workbench(i) = self.page {
                self.workbench.preview_rect(i)
            } else {
                self.viewport.content()
            });
            let search_clip = if let Page::Workbench(i) = self.page {
                self.workbench
                    .preview_rect(i)
                    .intersection(&self.viewport.bounds())
            } else {
                Some(self.viewport.bounds())
            };
            let visible_search = search_clip.and_then(|clip| search_rect.intersection(&clip));
            let search_hit = point.is_some_and(|p| visible_search.is_some_and(|r| r.contains(p)));
            let route = match event {
                Event::Mouse(m) => match m.kind {
                    MouseEventKind::Down { .. } => {
                        if !search_hit {
                            search_blurred = self.icon_search.state().is_some();
                            self.icon_search.blur();
                        }
                        search_hit
                    }
                    MouseEventKind::Dragged { .. } | MouseEventKind::Up { .. } => {
                        self.icon_search.state().is_some()
                    }
                    _ => false,
                },
                Event::Scroll(_) => false,
                _ => true,
            };
            if route && let Some(changed) = self.icon_search.handle(event, point) {
                self.icon_search.refresh(&mut self.text);
                return redraw_if(changed);
            }
        }
        if matches!(
            self.content_page(),
            Page::TextEditing | Page::Text | Page::ScrollArea
        ) {
            let editor = match self.content_page() {
                Page::Text => &mut self.text_view,
                Page::ScrollArea => &mut self.scroll_demo,
                _ => &mut self.editor,
            };
            if let Event::Key(k) = event
                && k.kind == KeyEventKind::Down
                && k.key_code == 48
            {
                let command = if k.modifiers.shift {
                    "insertBacktab:"
                } else {
                    "insertTab:"
                };
                let changed = editor.command(command).unwrap_or(false);
                editor.refresh(&mut self.text);
                let revealed = changed && self.reveal_text_caret();
                return redraw_if(changed || revealed);
            }
            let point = match event {
                Event::Mouse(m) => Some(to_point(m.position)),
                Event::Scroll(e) => Some(to_point(e.position)),
                _ => None,
            };
            let in_catalog = point.is_some_and(|p| p.x < layout::CATALOG_W);
            let handled = if !in_catalog
                || matches!(
                    event,
                    Event::Mouse(tekes_ui_platform_macos::MouseEvent {
                        kind: MouseEventKind::Up { .. },
                        ..
                    })
                ) {
                editor.handle(event, point)
            } else {
                None
            };
            if let Some(changed) = handled {
                editor.refresh(&mut self.text);
                let reveal = !matches!(event, Event::CaretBlink | Event::Mouse(_));
                let revealed = changed && reveal && self.reveal_text_caret();
                return redraw_if(changed || revealed);
            }
        }
        if self.content_page() == Page::Motion {
            let route = match event {
                Event::Mouse(m) => match m.kind {
                    MouseEventKind::Moved | MouseEventKind::Entered | MouseEventKind::Exited => {
                        true
                    }
                    MouseEventKind::Down { .. } => {
                        self.viewport.bounds().contains(to_point(m.position))
                    }
                    MouseEventKind::Dragged { .. } | MouseEventKind::Up { .. } => {
                        self.viewport.bounds().contains(to_point(m.position))
                            || self.motion.dragging.is_some()
                            || self.motion.pressed.is_some()
                    }
                },
                _ => false,
            };
            let live_bounds = self.live_content_bounds();
            if route
                && pages::motion::handle_input_clipped(
                    &mut self.motion,
                    event,
                    live_bounds,
                    self.viewport.bounds(),
                )
            {
                return Change::REDRAW;
            }
        }
        let mut change = match event {
            Event::Mouse(m) => {
                let hit = self.sidebar.hit(to_point(m.position));
                match m.kind {
                    MouseEventKind::Moved | MouseEventKind::Entered => {
                        redraw_if(self.set_hover(hit))
                    }
                    MouseEventKind::Dragged { .. } => {
                        redraw_if(self.viewport.pointer_drag(to_point(m.position)))
                    }
                    MouseEventKind::Up { .. } => {
                        self.viewport.pointer_up();
                        Change::NONE
                    }
                    MouseEventKind::Exited => redraw_if(self.set_hover(None)),
                    MouseEventKind::Down {
                        button: MouseButton::Left,
                        ..
                    } => {
                        if self.viewport.pointer_down(to_point(m.position)) {
                            return Change::REDRAW;
                        }
                        let hover_changed = self.set_hover(hit);
                        let mut change = match hit {
                            Some(i) => self.apply(Command::Page(Page::ALL[i])),
                            None => self.click_in_page(m.position),
                        };
                        change.redraw |= hover_changed;
                        change
                    }
                    _ => Change::NONE,
                }
            }
            Event::Key(k) if k.kind == KeyEventKind::Down => {
                let m = k.modifiers;
                if m.command || m.control || m.alt || m.shift {
                    return Change::NONE;
                }
                {
                    let bounds = self.viewport.bounds();
                    let moved = match k.key_code {
                        121 => Some(self.viewport.move_by(0.0, bounds.size.height * 0.8)),
                        116 => Some(self.viewport.move_by(0.0, -bounds.size.height * 0.8)),
                        119 => {
                            let delta = self.viewport.limit().y - self.viewport.offset.y;
                            Some(self.viewport.move_by(0.0, delta))
                        }
                        115 => {
                            let offset = self.viewport.offset;
                            Some(self.viewport.move_by(-offset.x, -offset.y))
                        }
                        124 => Some(self.viewport.move_by(layout::ROW_H, 0.0)),
                        123 => Some(self.viewport.move_by(-layout::ROW_H, 0.0)),
                        _ => None,
                    };
                    if let Some(changed) = moved {
                        return redraw_if(changed);
                    }
                }
                match k.key_code {
                    KEY_UP => self.step_page(-1),
                    KEY_DOWN => self.step_page(1),
                    _ => Change::NONE,
                }
            }
            _ => Change::NONE,
        };
        change.redraw |= search_blurred;
        change
    }
}

impl Gallery {
    /// 页面区的夹具选择器与组件固定状态选择器。
    fn click_in_page(&mut self, position: PlatformPoint) -> Change {
        if let Some(category) = self.page.category() {
            let Some(point) = self.viewport.point_in_content(to_point(position)) else {
                return Change::NONE;
            };
            return match pages::showcase::hit_state(category, layout::PAGE_CONTENT, point) {
                Some((id, state)) => redraw_if(self.showcase.select(id, state)),
                None => Change::NONE,
            };
        }
        if self.page != Page::StaticText {
            return Change::NONE;
        }
        let _ = position;
        Change::NONE
    }

    fn static_text_visible(&self) -> bool {
        match self.page {
            Page::StaticText => true,
            Page::Workbench(i) => {
                self.workbench.show_live(i) && workbench::entry(i).renderer() == "calibration"
            }
            _ => false,
        }
    }

    /// Route pointer input through the same retained Button placements that
    /// rendered the StaticText selectors. Returns `Some` only when a selector
    /// owns the current pointer sequence.
    fn handle_static_text_pointer(&mut self, event: &Event) -> Option<bool> {
        let point = match event {
            Event::Mouse(mouse) => {
                let raw = to_point(mouse.position);
                let visible = self.viewport.bounds().contains(raw);
                let content_point = self.viewport.point_in_content(raw).unwrap_or_else(|| {
                    Point::new(
                        raw.x + self.viewport.offset.x,
                        raw.y + self.viewport.offset.y,
                    )
                });
                Some(if visible {
                    content_point
                } else {
                    Point::new(f32::NAN, f32::NAN)
                })
            }
            _ => None,
        };
        let has_pressed = self
            .static_text
            .fixture_buttons()
            .iter()
            .chain(self.static_text.width_buttons())
            .any(|button| button.state.pressed);
        match event {
            Event::Mouse(mouse) => {
                let point = point?;
                match mouse.kind {
                    MouseEventKind::Moved | MouseEventKind::Entered => {
                        self.static_text.pointer_move(point).then_some(true)
                    }
                    MouseEventKind::Exited => Some(
                        self.static_text
                            .pointer_move(Point::new(f32::NAN, f32::NAN)),
                    ),
                    MouseEventKind::Down {
                        button: MouseButton::Left,
                        ..
                    } => {
                        if !self.viewport.bounds().contains(to_point(mouse.position)) {
                            return None;
                        }
                        let hit = self
                            .static_text
                            .fixture_buttons()
                            .iter()
                            .chain(self.static_text.width_buttons())
                            .any(|button| {
                                button.layout().is_some_and(|layout| layout.hit_test(point))
                            });
                        hit.then(|| self.static_text.pointer_down(point))
                    }
                    MouseEventKind::Dragged {
                        button: MouseButton::Left,
                    } if has_pressed => {
                        self.static_text.pointer_move(point);
                        Some(true)
                    }
                    MouseEventKind::Up {
                        button: MouseButton::Left,
                        ..
                    } if has_pressed => {
                        let release = if self.viewport.bounds().contains(to_point(mouse.position)) {
                            point
                        } else {
                            Point::new(f32::NAN, f32::NAN)
                        };
                        self.static_text.pointer_up(release);
                        Some(true)
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }
}

fn redraw_if(changed: bool) -> Change {
    if changed {
        Change::REDRAW
    } else {
        Change::NONE
    }
}

fn to_point(p: PlatformPoint) -> Point {
    Point::new(p.x as f32, p.y as f32)
}

/// 编译 `page` 在 `theme` 下的完整内容。快照和窗口共用它。
/// 调用方保证这一帧已经 `begin_frame` 过。
pub fn compose(page: Page, hover: Option<usize>, ctx: &mut PageCtx<'_>, scene: &mut Scene) {
    compose_with_showcase(page, hover, ctx, scene, &catalog::ShowcaseState::default());
}

pub fn compose_with_showcase(
    page: Page,
    hover: Option<usize>,
    ctx: &mut PageCtx<'_>,
    scene: &mut Scene,
    state: &catalog::ShowcaseState,
) {
    chrome::draw(scene, ctx.theme, ctx.text, page, hover);
    if let Some(category) = page.category() {
        pages::showcase::build(category, ctx, layout::PAGE_CONTENT, scene, state);
    } else {
        page.build(ctx, layout::PAGE_CONTENT, scene);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tekes_ui_controls::ButtonTextRenderer;
    use tekes_ui_platform_macos::{CommandId, KeyEvent, Modifiers, MouseEvent};

    #[test]
    fn live_gallery_drives_the_text_renderer_motion_clock() {
        let mut gallery = Gallery::with_static_text(StaticText::new(None));
        assert_eq!(
            gallery.text.motion_context().unwrap().now,
            Duration::ZERO,
            "the live Gallery opts into component motion"
        );
        gallery.text.request_animation_frame();
        assert!(gallery.needs_animation());
        assert!(gallery.animate(1.0 / 120.0));
        let elapsed = gallery.text.motion_context().unwrap().now;
        assert!(
            elapsed.abs_diff(Duration::from_secs_f64(1.0 / 120.0)) <= Duration::from_nanos(2),
            "frame delta advances the supplied monotonic component clock: {elapsed:?}"
        );
        gallery.set_reduced_motion(true);
        let mut scene = Scene::new();
        gallery.build_scene(&mut scene);
        assert!(gallery.text.motion_context().unwrap().reduced_motion);
    }

    fn motion_action_frame(
        gallery: &Gallery,
        action: pages::motion::Action,
    ) -> tekes_ui_scene::Rect {
        let id = pages::motion::Layout::action_id(action);
        gallery
            .motion
            .button_models
            .get(&id)
            .and_then(|button| button.layout())
            .expect("action was rendered through the production Button")
            .frame
    }

    fn mv(x: f64, y: f64) -> Event {
        Event::Mouse(MouseEvent {
            kind: MouseEventKind::Moved,
            position: PlatformPoint { x, y },
            modifiers: Modifiers::NONE,
        })
    }

    fn click(x: f64, y: f64) -> Event {
        Event::Mouse(MouseEvent {
            kind: MouseEventKind::Down {
                button: MouseButton::Left,
                click_count: 1,
            },
            position: PlatformPoint { x, y },
            modifiers: Modifiers::NONE,
        })
    }

    fn release(x: f64, y: f64) -> Event {
        Event::Mouse(MouseEvent {
            kind: MouseEventKind::Up {
                button: MouseButton::Left,
                click_count: 1,
            },
            position: PlatformPoint { x, y },
            modifiers: Modifiers::NONE,
        })
    }

    fn key(code: u16, modifiers: Modifiers) -> Event {
        Event::Key(KeyEvent {
            kind: KeyEventKind::Down,
            key_code: code,
            characters: String::new(),
            characters_ignoring_modifiers: String::new(),
            modifiers,
            is_repeat: false,
        })
    }

    fn row_center(i: usize) -> (f64, f64) {
        let c = layout::catalog_row_rect(i).center();
        (c.x as f64, c.y as f64)
    }

    #[test]
    fn navigation_search_uses_independent_json_focus_ring_and_window_focus_lifecycle() {
        let mut g = Gallery::new();
        let rect = navigation::search_rect();
        let border = |g: &mut Gallery| {
            let mut scene = Scene::new();
            g.build_scene(&mut scene);
            let base = scene
                .primitives()
                .iter()
                .find_map(|p| match p {
                    tekes_ui_scene::Primitive::Quad(q) if q.rect == rect => Some(q.border_color),
                    _ => None,
                })
                .unwrap();
            let ring = scene.primitives().iter().find_map(|p| match p {
                tekes_ui_scene::Primitive::Quad(q)
                    if q.rect == rect.inset(-4.) && q.fill.a == 0. && q.border_width == 2. =>
                {
                    Some(q.border_color)
                }
                _ => None,
            });
            (base, ring)
        };
        let normal = tekes_ui_tokens::proposal::render_color("border", g.theme()).unwrap();
        let focused = tekes_ui_tokens::proposal::render_color("border-focused", g.theme()).unwrap();
        assert_eq!(border(&mut g), (normal, None));
        let point = rect.center();
        g.handle_input(&click(point.x.into(), point.y.into()));
        assert_eq!(border(&mut g), (normal, Some(focused)));
        assert!(g.navigation_search.state().is_some());
        g.handle_input(&Event::Focus(false));
        assert_eq!(border(&mut g), (normal, None));
        assert!(g.navigation_search.state().is_none());
        g.handle_input(&Event::Focus(true));
        assert_eq!(border(&mut g), (normal, Some(focused)));
        g.handle_input(&click(600., 300.));
        assert_eq!(border(&mut g), (normal, None));
    }

    #[test]
    fn combo_box_native_input_route_exposes_composition_state_to_the_gallery_host() {
        let mut g = Gallery::new();
        assert!(g.open_route("component-combo-box"));
        g.set_view_size(CONTENT_WIDTH, CONTENT_HEIGHT);
        g.build_scene(&mut Scene::new());

        let point = workbench::Workbench::fixture_point("component-combo-box", 1000);
        g.handle_input(&click(point.x.into(), point.y.into()));
        g.handle_input(&release(point.x.into(), point.y.into()));
        assert!(
            g.text_input_state().is_some_and(|state| !state.read_only),
            "clicking the real ComboBox input must publish its editable text state"
        );

        g.handle_input(&Event::TextPreedit {
            text: "C".into(),
            clauses: Vec::new(),
            selection: 1..1,
            replacement: None,
        });
        let state = g
            .text_input_state()
            .expect("the focused ComboBox must keep publishing IME state");
        assert_eq!(state.text, "C");
        assert!(state.marked.is_some(), "composition must remain marked");

        g.handle_input(&Event::TextCommit {
            text: "Com".into(),
            replacement: None,
        });
        assert_eq!(g.text_input_state().unwrap().text, "Com");
        // NSTextInputClient delivers these as selector commands instead of
        // ordinary key-down events when its editable input owns focus.
        g.handle_input(&Event::TextCommand("moveDown:".into()));
        g.handle_input(&Event::TextCommand("insertNewline:".into()));
        assert_eq!(
            g.text_input_state().unwrap().text,
            "Comfortable",
            "the active filtered option must commit through the real input"
        );
    }

    #[test]
    fn icons_search_routes_native_text_clipboard_and_page_changes() {
        let mut g = Gallery::new();
        g.apply(Command::Page(Page::Icons));
        g.build_scene(&mut Scene::new());
        let center = pages::icons::search_rect(layout::PAGE_CONTENT).center();
        assert!(
            g.handle_input(&click(center.x.into(), center.y.into()))
                .redraw
        );
        assert!(g.text_input_state().is_some());
        assert!(
            g.handle_input(&Event::TextCommit {
                text: "vRoN".into(),
                replacement: None
            })
            .redraw
        );
        assert_eq!(g.icon_search.query(), "vRoN");
        g.text_command("selectAll:", None);
        assert_eq!(g.text_command("copy:", None).1.as_deref(), Some("vRoN"));
        g.text_command("paste:", Some("missing-icon"));
        assert_eq!(g.icon_search.query(), "missing-icon");
        g.text_command("selectAll:", None);
        g.text_command("cut:", None);
        assert_eq!(g.icon_search.query(), "");
        g.handle_input(&Event::TextPreedit {
            clauses: Vec::new(),
            text: "搜索".into(),
            selection: 2..2,
            replacement: None,
        });
        assert!(g.text_input_state().unwrap().marked.is_some());
        g.handle_input(&Event::TextCommit {
            text: "file".into(),
            replacement: None,
        });
        assert_eq!(g.icon_search.query(), "file");
        g.apply(Command::Page(Page::Tokens));
        assert!(g.text_input_state().is_none());
        g.apply(Command::Page(Page::Icons));
        assert_eq!(g.icon_search.query(), "file");
        assert!(g.text_input_state().is_none());
    }

    #[test]
    fn hover_within_same_row_does_not_redraw() {
        let mut g = Gallery::new();
        let (x, y) = row_center(1);
        // 进入第 1 行：变了，重绘。
        assert_eq!(g.handle_input(&mv(x, y)), Change::REDRAW);
        assert_eq!(g.hover(), Some(navigation::pages()[1].index()));
        // 行内移动：不重绘。
        assert_eq!(g.handle_input(&mv(x + 10.0, y + 3.0)), Change::NONE);
        assert_eq!(g.handle_input(&mv(x - 40.0, y - 5.0)), Change::NONE);
        // 移到页面区：hover 清除，重绘一次；再在页面区移动不重绘。
        assert_eq!(g.handle_input(&mv(600.0, 300.0)), Change::REDRAW);
        assert_eq!(g.hover(), None);
        assert_eq!(g.handle_input(&mv(610.0, 300.0)), Change::NONE);
        // 移出窗口：已经没有 hover，不重绘。
        let exit = Event::Mouse(MouseEvent {
            kind: MouseEventKind::Exited,
            position: PlatformPoint { x: -1.0, y: -1.0 },
            modifiers: Modifiers::NONE,
        });
        assert_eq!(g.handle_input(&exit), Change::NONE);
    }

    #[test]
    fn hover_moving_between_rows_redraws_each_time() {
        let mut g = Gallery::new();
        let (x0, y0) = row_center(0);
        let (x1, y1) = row_center(1);
        assert!(g.handle_input(&mv(x0, y0)).redraw);
        assert!(g.handle_input(&mv(x1, y1)).redraw);
        assert!(g.handle_input(&mv(x0, y0)).redraw);
    }

    #[test]
    fn exit_clears_hover() {
        let mut g = Gallery::new();
        let (x, y) = row_center(0);
        g.handle_input(&mv(x, y));
        let exit = Event::Mouse(MouseEvent {
            kind: MouseEventKind::Exited,
            position: PlatformPoint { x: 0.0, y: 0.0 },
            modifiers: Modifiers::NONE,
        });
        assert_eq!(g.handle_input(&exit), Change::REDRAW);
        assert_eq!(g.hover(), None);
    }

    #[test]
    fn click_selects_page_and_updates_chrome() {
        let mut g = Gallery::new();
        let (x, y) = row_center(1);
        let c = g.handle_input(&click(x, y));
        assert_eq!(c, Change::BOTH);
        assert_eq!(g.page(), navigation::pages()[1]);
        // 再点同一行：页没变，hover 也已经在这一行，什么都不用做。
        assert_eq!(g.handle_input(&click(x, y)), Change::NONE);
        // 点页面区：无事发生。
        assert!(!g.handle_input(&click(700.0, 400.0)).chrome);
        assert_eq!(g.page(), navigation::pages()[1]);
    }

    #[test]
    fn click_on_unhovered_row_redraws_even_if_page_same() {
        // 鼠标没移动事件就直接按下（例如窗口刚激活）：hover 与选中都要更新。
        let mut g = Gallery::new();
        let (x, y) = row_center(0);
        let c = g.handle_input(&click(x, y));
        assert!(c.redraw, "hover 从 None 变成 Some(0)");
        assert!(!c.chrome, "页面没变");
    }

    #[test]
    fn arrow_keys_switch_pages_without_wrapping() {
        let mut g = Gallery::new();
        let pages = navigation::pages();
        g.apply(Command::Page(pages[0]));
        assert_eq!(g.handle_input(&key(KEY_UP, Modifiers::NONE)), Change::NONE);
        for expected in pages.iter().copied().skip(1) {
            assert_eq!(
                g.handle_input(&key(KEY_DOWN, Modifiers::NONE)),
                Change::BOTH
            );
            assert_eq!(g.page(), expected);
        }
        assert_eq!(
            g.handle_input(&key(KEY_DOWN, Modifiers::NONE)),
            Change::NONE
        );
        for expected in pages.iter().copied().rev().skip(1) {
            assert_eq!(g.handle_input(&key(KEY_UP, Modifiers::NONE)), Change::BOTH);
            assert_eq!(g.page(), expected);
        }
        assert_eq!(g.handle_input(&key(KEY_UP, Modifiers::NONE)), Change::NONE);
        // 带修饰键的方向键不处理。
        assert_eq!(
            g.handle_input(&key(KEY_DOWN, Modifiers::COMMAND)),
            Change::NONE
        );
        assert_eq!(g.page(), pages[0]);
    }

    #[test]
    fn page_commands() {
        let mut g = Gallery::new();
        assert_eq!(g.apply(Command::Page(navigation::pages()[0])), Change::NONE);
        assert_eq!(g.apply(Command::Page(Page::Tokens)), Change::BOTH);
        assert!(g.is_checked(Command::Page(Page::Tokens)));
        assert!(!g.is_checked(Command::Page(Page::Primitives)));
        assert_eq!(g.title(), "TekesUI Gallery — Token Swatches — Light");
    }

    #[test]
    fn menu_ids_drive_state() {
        let mut g = Gallery::new();
        let cmd = Command::from_id(CommandId(101)).unwrap();
        g.apply(cmd);
        assert_eq!(g.page(), Page::Tokens);
        let cmd = Command::from_id(CommandId(202)).unwrap();
        let c = g.apply(cmd);
        assert!(c.redraw && c.chrome);
        assert_eq!(g.theme(), Theme::DARK);
        let cmd = Command::from_id(CommandId(301)).unwrap();
        g.apply(cmd);
        assert_eq!(g.theme(), Theme::DARK_INCREASED);
        assert_eq!(
            g.title(),
            "TekesUI Gallery — Token Swatches — Dark (Increase Contrast)"
        );
    }

    #[test]
    fn appearance_radio_state_and_redraw_rules() {
        let mut g = Gallery::new();
        // 系统是浅色，选“浅色”：生效主题没变，不重绘，但勾选要同步。
        let c = g.apply(Command::Appearance(AppearanceChoice::Light));
        assert_eq!(
            c,
            Change {
                redraw: false,
                chrome: true
            }
        );
        assert!(g.is_checked(Command::Appearance(AppearanceChoice::Light)));
        assert!(!g.is_checked(Command::Appearance(AppearanceChoice::System)));
        // 系统变深：固定浅色时不受影响。
        assert_eq!(g.set_system_theme(Theme::DARK), Change::NONE);
        assert_eq!(g.theme(), Theme::LIGHT);
        // 回到跟随系统：生效主题变成深色。
        let c = g.apply(Command::Appearance(AppearanceChoice::System));
        assert!(c.redraw && c.chrome);
        assert_eq!(g.theme(), Theme::DARK);
    }

    #[test]
    fn system_change_while_following_redraws() {
        let mut g = Gallery::new();
        assert_eq!(g.set_system_theme(Theme::LIGHT), Change::NONE);
        assert_eq!(g.set_system_theme(Theme::DARK), Change::BOTH);
        assert_eq!(g.set_system_theme(Theme::DARK_INCREASED), Change::BOTH);
        // 强制关闭对比度后，系统对比度变化不影响。
        g.apply(Command::Contrast(ContrastChoice::Off));
        assert_eq!(g.set_system_theme(Theme::DARK), Change::NONE);
    }

    #[test]
    fn exactly_one_item_checked_per_group() {
        let mut g = Gallery::new();
        g.apply(Command::Appearance(AppearanceChoice::Dark));
        g.apply(Command::Contrast(ContrastChoice::On));
        g.apply(Command::Page(navigation::pages()[1]));
        let bar = g.menu_bar();
        assert_eq!(bar.validate(), Ok(()));
        let checked = |range: std::ops::Range<u32>| {
            bar.commands()
                .filter(|c| range.contains(&c.command.0) && c.checked)
                .count()
        };
        assert_eq!(checked(100..200), 1);
        assert_eq!(checked(200..300), 1);
        assert_eq!(checked(300..400), 1);
    }

    /// 用一个全新的文字引擎和静态文字页状态编一页（像一次新启动）。
    fn fresh_scene(page: Page, hover: Option<usize>, theme: Theme, scale: f32) -> Scene {
        let mut text = TextEngine::new();
        text.set_scale(scale);
        let mut st = StaticText::new(None);
        let mut ctx = PageCtx {
            theme,
            text: &mut text,
            static_text: &mut st,
        };
        let mut s = Scene::new();
        ctx.text.begin_frame();
        compose(page, hover, &mut ctx, &mut s);
        s
    }

    #[test]
    fn scene_is_deterministic() {
        for page in Page::ALL {
            for theme in Theme::ALL {
                for scale in [1.0, 2.0] {
                    let a = fresh_scene(page, None, theme, scale);
                    let b = fresh_scene(page, None, theme, scale);
                    assert!(!a.is_empty());
                    assert_eq!(a, b, "{page:?} {theme:?} {scale}x");
                }
            }
        }
    }

    /// 同一个引擎连续编两次（第二次全部走缓存）与新引擎编的一样：标签只排一次版。
    #[test]
    fn labels_are_laid_out_deterministically_and_shaped_once() {
        for page in Page::ALL {
            let mut text = TextEngine::new();
            let mut st = StaticText::new(None);
            let mut scenes = Vec::new();
            let mut shapes = Vec::new();
            for _ in 0..3 {
                let mut ctx = PageCtx {
                    theme: Theme::LIGHT,
                    text: &mut text,
                    static_text: &mut st,
                };
                let mut s = Scene::new();
                ctx.text.begin_frame();
                compose(page, None, &mut ctx, &mut s);
                scenes.push(s);
                shapes.push(text.shape_count());
            }
            assert_eq!(scenes[0], scenes[1], "{page:?}");
            assert_eq!(scenes[0], scenes[2], "{page:?}");
            assert_eq!(
                scenes[0],
                fresh_scene(page, None, Theme::LIGHT, 2.0),
                "{page:?}"
            );
            // 第一帧之后不再排版。
            assert_eq!(shapes[0], shapes[1], "{page:?}");
            assert_eq!(shapes[0], shapes[2], "{page:?}");
        }
    }

    /// 所有字形原点：x 落在整数 pt 加上字形自己的小数偏移里，整行的原点是整数；
    /// 这里查段落原点的整数性：每个标签的第一个字形行 y 基线与整数原点之差等于 ascent（小数）。
    #[test]
    fn label_origins_are_integer_points() {
        let mut text = TextEngine::new();
        let mut scene = Scene::new();
        let slot = tekes_ui_scene::Rect::new(20.3, 31.6, 180.0, 16.0);
        let p = text.label("Primitives", tekes_ui_tokens::TextRole::Body, None);
        let o = text::label_origin(slot, &p, text::HAlign::Left);
        assert_eq!(
            (o.x, o.y),
            (20.0, (31.6_f32 + (16.0 - p.height as f32) / 2.0).round())
        );
        text.begin_frame();
        text.label_in(
            &mut scene,
            Theme::LIGHT,
            slot,
            "Primitives",
            chrome::CATALOG_STYLE,
        );
        let tekes_ui_scene::Primitive::Glyphs(run) = &scene.primitives()[0] else {
            panic!("应该是字形图元");
        };
        let first = run.glyphs[0].origin;
        // 第一个字形的 x 是整数起点加字形自己的 x（P 的原点 0）。
        assert_eq!(first.x, 20.0);
        let baseline = p.lines[0].baseline_y as f32;
        assert!((first.y - (o.y + baseline)).abs() < 1e-4);
    }

    #[test]
    fn scene_is_independent_of_window_size_and_changes_with_theme_and_hover() {
        let mut g = Gallery::new();
        let mut light = Scene::new();
        g.build_scene(&mut light);
        g.set_system_theme(Theme::DARK);
        let mut dark = Scene::new();
        g.build_scene(&mut dark);
        assert_ne!(light, dark);

        let mut g2 = Gallery::new();
        let (x, y) = row_center(1);
        g2.handle_input(&mv(x, y));
        let mut hovered = Scene::new();
        g2.build_scene(&mut hovered);
        assert_ne!(light, hovered);
    }

    #[test]
    fn pages_differ_and_themes_differ() {
        let build = |p, t| fresh_scene(p, None, t, 2.0);
        assert_ne!(
            build(Page::Primitives, Theme::LIGHT),
            build(Page::Tokens, Theme::LIGHT)
        );
        assert_ne!(
            build(Page::Tokens, Theme::LIGHT),
            build(Page::StaticText, Theme::LIGHT)
        );
        for page in Page::ALL {
            for i in 0..4 {
                for j in i + 1..4 {
                    assert_ne!(
                        build(page, Theme::ALL[i]),
                        build(page, Theme::ALL[j]),
                        "{page:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn icons_disable_horizontal_scroll_even_in_a_narrow_window() {
        use tekes_ui_controls::ScrollPolicy;
        let mut gallery = Gallery::new();
        gallery.apply(Command::Page(Page::Icons));
        gallery.set_view_size(720.0, 480.0);
        assert_eq!(
            gallery.viewport.scroll_policy(),
            (ScrollPolicy::Disabled, ScrollPolicy::Auto)
        );
        assert_eq!(gallery.viewport.limit().x, 0.0);
        assert!(gallery.viewport.limit().y > 0.0);
        gallery
            .viewport
            .wheel(100.0, 10.0, true, true, false, false);
        assert_eq!(gallery.viewport.visual_offset().x, 0.0);
        assert_eq!(gallery.viewport.offset.y, 10.0);
        gallery.apply(Command::Page(Page::Primitives));
        assert_eq!(
            gallery.viewport.scroll_policy(),
            (ScrollPolicy::Auto, ScrollPolicy::Auto)
        );
    }

    #[test]
    fn component_previews_keep_the_available_viewport_at_large_text_scale() {
        let mut gallery = Gallery::new();
        gallery.set_view_size(900.0, 720.0);
        gallery.apply(Command::TextScale(2));
        for (index, entry) in workbench::entries().iter().enumerate() {
            if entry.component().is_empty() {
                continue;
            }
            gallery.apply(Command::Page(Page::Workbench(index)));
            gallery.build_scene(&mut Scene::new());
            let preview = gallery.workbench.preview_rect(index);
            assert!(
                preview.max_y() <= gallery.viewport.bounds().max_y() + 0.01,
                "{} preview extends below the viewport: {preview:?}",
                entry.component()
            );
        }
    }

    #[test]
    fn candidate_dialog_consumes_gallery_scroll_without_moving_the_background_page() {
        let mut gallery = Gallery::new();
        let matrix = workbench::entries()
            .iter()
            .find(|entry| entry.renderer() == "matrix")
            .expect("brand matrix story exists");
        assert!(gallery.open_route(&format!("{}/playground", matrix.slug())));
        gallery.set_view_size(1400.0, 640.0);
        let mut scene = Scene::new();
        gallery.build_scene(&mut scene);

        let click_target = |gallery: &mut Gallery, id| {
            gallery.build_scene(&mut Scene::new());
            let point = gallery
                .workbench
                .brand_preview_target_center_for_test(id)
                .unwrap_or_else(|| panic!("brand preview target {id} is not visible"));
            assert_ne!(
                gallery.handle_input(&click(point.x.into(), point.y.into())),
                Change::NONE
            );
            assert_ne!(
                gallery.handle_input(&release(point.x.into(), point.y.into())),
                Change::NONE
            );
        };
        click_target(&mut gallery, 700);
        assert!(
            gallery
                .workbench
                .brand_preview_candidate_for_test()
                .is_some()
        );

        let max_scroll = gallery.viewport.limit().y;
        assert!(
            max_scroll > 0.0,
            "matrix story should have a scrollable body"
        );
        let mut overlay_visible = false;
        for step in 0..=20 {
            let desired = max_scroll * step as f32 / 20.0;
            gallery
                .viewport
                .move_by(0.0, desired - gallery.viewport.offset.y);
            gallery.build_scene(&mut Scene::new());
            if gallery
                .workbench
                .brand_preview_target_center_for_test(721)
                .is_some()
            {
                overlay_visible = true;
                break;
            }
        }
        assert!(
            overlay_visible,
            "scrolling should expose the dialog trigger"
        );
        click_target(&mut gallery, 721);
        gallery.build_scene(&mut Scene::new());
        assert!(gallery.workbench.brand_preview_dialog_is_visible_for_test());

        let before = gallery.viewport.offset;
        gallery.handle_input(&wheel(
            1100.0,
            500.0,
            tekes_ui_platform_macos::ScrollPhase::Began,
            tekes_ui_platform_macos::ScrollPhase::None,
        ));
        assert_eq!(gallery.viewport.offset, before);
    }

    fn wheel(
        x: f64,
        y: f64,
        phase: tekes_ui_platform_macos::ScrollPhase,
        momentum: tekes_ui_platform_macos::ScrollPhase,
    ) -> Event {
        Event::Scroll(tekes_ui_platform_macos::ScrollEvent {
            timestamp: 0.0,
            delta: tekes_ui_platform_macos::Vec2 { x: 0.0, y: -30.0 },
            precise: true,
            phase,
            momentum_phase: momentum,
            inverted: false,
            position: PlatformPoint { x, y },
            modifiers: Modifiers::NONE,
        })
    }
    #[test]
    fn scroll_gesture_keeps_its_owner_across_regions_and_momentum() {
        use tekes_ui_platform_macos::ScrollPhase as P;
        let mut g = Gallery::new();
        g.set_view_size(1000.0, 480.0);
        g.handle_input(&wheel(100.0, 300.0, P::Began, P::None));
        assert_eq!(g.scroll_owner, Some(ScrollOwner::Sidebar));
        let document = g.viewport.offset;
        g.handle_input(&wheel(800.0, 350.0, P::Changed, P::None));
        g.handle_input(&wheel(800.0, 350.0, P::Ended, P::None));
        assert_eq!(g.viewport.offset, document);
        g.handle_input(&wheel(800.0, 350.0, P::None, P::Began));
        assert_eq!(g.scroll_owner, Some(ScrollOwner::Sidebar));
        g.handle_input(&wheel(800.0, 350.0, P::None, P::Ended));
        assert_eq!(g.scroll_owner, None);
        for _ in 0..120 {
            g.animate(1.0 / 120.0);
        }
        assert!(!g.needs_animation());
        g.handle_input(&wheel(800.0, 350.0, P::Began, P::None));
        assert_eq!(g.scroll_owner, Some(ScrollOwner::Document));
        let sidebar = g.sidebar.scroll.offset;
        g.handle_input(&wheel(100.0, 300.0, P::Changed, P::None));
        g.handle_input(&wheel(100.0, 300.0, P::Ended, P::None));
        assert_eq!(g.sidebar.scroll.offset, sidebar);
        g.handle_input(&Event::Focus(false));
        assert_eq!(g.scroll_owner, None);
        assert!(!g.needs_animation());
    }

    #[test]
    fn open_dialog_blocks_scroll_from_its_story_header_without_moving_document() {
        use tekes_ui_platform_macos::ScrollPhase as P;

        let mut g = Gallery::new();
        g.set_view_size(1440.0, 960.0);
        let index = workbench::entries()
            .iter()
            .position(|entry| entry.renderer() == "dialog")
            .expect("Dialog Story is registered");
        g.apply(Command::Page(Page::Workbench(index)));
        g.build_scene(&mut Scene::new());

        let invoker = workbench::Workbench::fixture_point("component-dialog", 20);
        g.handle_input(&click(invoker.x as f64, invoker.y as f64));
        g.handle_input(&release(invoker.x as f64, invoker.y as f64));
        assert_eq!(
            g.handle_input(&key(53, Modifiers::NONE)),
            Change::REDRAW,
            "Escape is handled only while the actual Dialog Story modal is open"
        );
        g.handle_input(&click(invoker.x as f64, invoker.y as f64));
        g.handle_input(&release(invoker.x as f64, invoker.y as f64));
        g.set_view_size(1000.0, 480.0);
        g.build_scene(&mut Scene::new());

        let preview = g.workbench.preview_rect(index);
        let header_point = PlatformPoint {
            x: preview.min_x() as f64 + 12.0,
            y: preview.min_y() as f64 - 2.0,
        };
        assert!(header_point.x >= layout::CATALOG_W as f64);
        assert!(header_point.y < preview.min_y() as f64);
        assert!(g.viewport.bounds().contains(to_point(header_point)));
        let before = g.viewport.offset;

        let change = g.handle_input(&wheel(header_point.x, header_point.y, P::None, P::None));

        assert_eq!(change, Change::REDRAW, "the modal host consumes the wheel");
        assert_eq!(
            g.viewport.offset, before,
            "the document must not scroll behind Dialog"
        );
    }

    #[test]
    fn text_caret_stays_in_the_remaining_window_viewport() {
        let mut g = Gallery::new();
        g.set_view_size(1000.0, 480.0);
        g.apply(Command::Page(Page::TextEditing));
        g.build_scene(&mut Scene::new());
        g.handle_input(&key(48, Modifiers::NONE));
        g.handle_input(&key(48, Modifiers::NONE));
        g.text_command("moveToEndOfDocument:", None);
        g.build_scene(&mut Scene::new());
        let state = g.text_input_state().unwrap();
        let bounds = g.viewport.bounds();
        assert!(state.caret_origin.y as f32 >= bounds.min_y());
        assert!((state.caret_origin.y + state.caret_size.height) as f32 <= bounds.max_y() + 0.1);
        assert_eq!(g.viewport.offset, Point::default());
    }

    #[test]
    fn internal_multiline_text_editor_fills_remaining_scaled_preview() {
        let mut g = Gallery::new();
        assert!(g.open_route("lab-internal-text/playground"));
        g.workbench.text_scale = 2;
        g.set_view_size(1000.0, 520.0);
        g.build_scene(&mut Scene::new());
        let workbench_index = match g.page {
            Page::Workbench(i) => i,
            _ => unreachable!(),
        };
        let target = g.editor.visible_frame_at(1).unwrap().center();
        g.handle_input(&click(target.x.into(), target.y.into()));
        g.handle_input(&release(target.x.into(), target.y.into()));
        assert!(
            g.text_input_state().is_some(),
            "visible editor field receives focus"
        );
        g.text_command("moveToEndOfDocument:", None);
        g.build_scene(&mut Scene::new());

        let state = g
            .text_input_state()
            .expect("TextEditor has a focused surface");
        let viewport = g.viewport.bounds();
        let preview = g.workbench.preview_rect(workbench_index);
        let visible = preview
            .intersection(&viewport)
            .expect("preview intersects viewport");
        assert!(state.caret_origin.x as f32 >= visible.min_x());
        assert!(state.caret_origin.y as f32 >= visible.min_y());
        assert!(
            state.caret_origin.y as f32 + state.caret_size.height as f32 <= visible.max_y() + 0.1
        );
        let (limit, inner_height) = g.editor_scroll_metrics();
        assert!(inner_height > 0.0);
        assert!(limit >= 0.0);
    }

    #[test]
    fn internal_text_editor_has_nonnegative_fill_at_narrow_150_percent() {
        let mut g = Gallery::new();
        assert!(g.open_route("lab-internal-text/playground"));
        g.workbench.text_scale = 2;
        g.set_view_size(720.0, 480.0);
        g.build_scene(&mut Scene::new());

        let (limit, inner_height) = g.editor_scroll_metrics();
        assert!(inner_height > 0.0);
        assert!(limit >= 0.0);
    }

    #[test]
    fn canonical_text_surface_pointer_targets_use_rendered_visible_frames() {
        for (width, height, scale, index) in [
            (1440.0, 827.0, 0, 0),
            (1440.0, 827.0, 0, 1),
            (1000.0, 520.0, 2, 1),
            (720.0, 480.0, 2, 1),
        ] {
            let mut g = Gallery::new();
            assert!(g.open_route("lab-internal-text/playground"));
            g.workbench.text_scale = scale;
            g.set_view_size(width, height);
            g.build_scene(&mut Scene::new());
            let preview = g.workbench.preview_rect(match g.page {
                Page::Workbench(i) => i,
                _ => unreachable!(),
            });
            let frame = g
                .editor
                .visible_frame_at(index)
                .unwrap_or_else(|| panic!("surface {index} should be visible at {width}x{height}"));
            let target = frame.center();
            assert!(g.viewport.bounds().contains(target));
            assert!(preview.contains(target));
            g.handle_input(&click(target.x.into(), target.y.into()));
            g.handle_input(&release(target.x.into(), target.y.into()));
            let state = g
                .text_input_state()
                .expect("rendered TextSurface pointer target receives focus");
            assert!(!state.read_only);
            assert_eq!(
                state.text,
                if index == 0 {
                    "Example project name"
                } else {
                    include_str!("../../../../fixtures/text/gallery-reading-en.txt").trim_end()
                }
            );
        }
    }

    #[test]
    fn canonical_scroll_area_reaches_document_end_from_a_visible_pointer_target() {
        let mut g = Gallery::new();
        assert!(g.open_route("component-scroll-area/playground"));
        g.workbench.text_scale = 2;
        g.set_view_size(720.0, 480.0);
        g.build_scene(&mut Scene::new());
        let frame = g.scroll_demo.visible_frame_at(0).unwrap();
        let target = frame.center();
        assert!(g.viewport.bounds().contains(target));
        g.handle_input(&click(target.x.into(), target.y.into()));
        g.handle_input(&release(target.x.into(), target.y.into()));
        assert!(
            g.text_input_state().is_some(),
            "visible reading surface receives focus"
        );
        g.text_command("moveToEndOfDocument:", None);
        g.build_scene(&mut Scene::new());
        let state = g.text_input_state().expect("focused read-only TextSurface");
        assert_eq!(state.caret_index, state.text.len());
        let frame = g.scroll_demo.visible_frame_at(0).unwrap();
        assert!(frame.contains(Point::new(
            state.caret_origin.x as f32,
            state.caret_origin.y as f32
        )));
    }
    #[test]
    fn scroll_area_demo_has_only_one_document_scroll_owner_at_every_window_size() {
        let mut g = Gallery::new();
        g.apply(Command::Page(Page::ScrollArea));
        for (w, h) in [
            (1440.0, 960.0),
            (1440.0, 827.0),
            (720.0, 480.0),
            (1920.0, 1200.0),
        ] {
            g.set_view_size(w, h);
            g.build_scene(&mut Scene::new());
            assert_eq!(g.viewport.limit(), Point::default());
            assert_eq!(g.viewport.offset, Point::default());
            let p = g.viewport.bounds().center();
            assert_eq!(
                g.scroll_target_at(p),
                Some(ScrollOwner::Text(Page::ScrollArea))
            );
            if h <= 960.0 {
                assert!(g.scroll_demo.scroll_metrics().0 > 0.0);
            }
            assert_eq!(
                g.viewport.bounds().max_y(),
                h - tekes_ui_tokens::Spacing::Section.value()
            );
        }
    }
    #[test]
    fn page_change_ends_the_previous_scroll_gesture() {
        use tekes_ui_platform_macos::ScrollPhase as P;
        let mut g = Gallery::new();
        g.handle_input(&wheel(800.0, 350.0, P::Began, P::None));
        g.apply(Command::Page(Page::TextEditing));
        assert_eq!(g.scroll_owner, None);
        assert!(!g.needs_animation());
    }
    #[test]
    fn short_window_scroll_and_translated_state_hits_remain_reachable() {
        let mut gallery = Gallery::new();
        gallery.set_view_size(1000.0, 720.0);
        gallery.apply(Command::Page(Page::Foundations));
        assert!(gallery.handle_input(&key(119, Modifiers::NONE)).redraw);
        assert!(gallery.viewport.offset.y > 0.0);
        // Horizontal scrolling exposes the right-hand column in a narrow window.
        for _ in 0..30 {
            gallery.handle_input(&key(124, Modifiers::NONE));
        }
        let point = pages::showcase::state_center(
            "foundations",
            layout::PAGE_CONTENT,
            "tui.disclosure",
            "rest",
        )
        .unwrap();
        let visible = Point::new(
            point.x - gallery.viewport.offset.x,
            point.y - gallery.viewport.offset.y,
        );
        assert!(gallery.viewport.bounds().contains(visible));
        assert!(
            gallery
                .handle_input(&click(f64::from(visible.x), f64::from(visible.y)))
                .redraw
        );
        assert_eq!(
            gallery
                .showcase
                .selected(catalog::component("tui.disclosure")),
            "rest"
        );
        gallery.apply(Command::Page(Page::Primitives));
        assert_eq!(gallery.viewport.offset, Point::default());
        assert!(gallery.handle_input(&key(121, Modifiers::NONE)).redraw);
        assert!(gallery.viewport.offset.y > 0.0);
    }

    #[test]
    fn every_catalog_state_chip_routes_through_gallery_input() {
        let mut gallery = Gallery::new();
        let mut components = 0;
        let mut states = 0;
        for page in Page::ALL {
            let Some(category) = page.category() else {
                continue;
            };
            gallery.apply(Command::Page(page));
            for id in &catalog::group(category).component_ids {
                let component = catalog::component(id);
                components += 1;
                for state in &component.states {
                    states += 1;
                    let point =
                        pages::showcase::state_center(category, layout::PAGE_CONTENT, id, state)
                            .unwrap();
                    let expected_change = gallery.showcase.selected(component) != state;
                    let event = click(f64::from(point.x), f64::from(point.y));
                    let change = gallery.handle_input(&event);
                    assert_eq!(change.redraw, expected_change, "{id}/{state}");
                    assert!(!change.chrome);
                    assert_eq!(gallery.showcase.selected(component), state);
                    assert_eq!(
                        gallery.handle_input(&event),
                        Change::NONE,
                        "repeated {id}/{state}"
                    );
                }
            }
        }
        assert_eq!(components, 40);
        assert!(states > 200);
    }

    #[test]
    fn default_language_and_live_switch_preserve_page_theme_and_layout() {
        use locale::Locale;
        let mut g = Gallery::with_static_text(StaticText::new(None));
        assert_eq!(g.locale(), Locale::En);
        g.apply(Command::Page(Page::TextEditing));
        g.set_view_size(1000.0, 520.0);
        g.build_scene(&mut Scene::new());
        let bounds = g.viewport.bounds();
        let theme = g.theme.effective();
        assert_eq!(g.apply(Command::Language(Locale::ZhCn)), Change::BOTH);
        assert_eq!(g.page, Page::TextEditing);
        assert_eq!(g.theme.effective(), theme);
        g.build_scene(&mut Scene::new());
        assert_eq!(g.viewport.bounds(), bounds);
        assert!(g.is_checked(Command::Language(Locale::ZhCn)));
        assert!(!g.is_checked(Command::Language(Locale::En)));
        assert_eq!(g.apply(Command::Language(Locale::ZhCn)), Change::NONE);
        assert_eq!(g.apply(Command::Language(Locale::En)), Change::BOTH);
        assert_eq!(g.viewport.bounds(), bounds);
    }

    #[test]
    fn motion_story_draw_and_pointer_share_preview_bounds_after_scroll_and_resize() {
        use motion_preview::PresetCatalog;
        use pages::motion::{Action, Layout};

        let mut gallery = Gallery::new();
        assert!(gallery.open_route("foundation-motion/playground"));
        gallery.workbench.text_scale = 2;
        gallery.set_view_size(1000.0, 520.0);
        let mut scene = Scene::new();
        gallery.build_scene(&mut scene);

        let page_bounds = gallery.viewport.content();
        let preview_bounds = gallery.live_content_bounds();
        assert_ne!(page_bounds.origin, preview_bounds.origin);
        let stale_position = Layout::new(page_bounds)
            .button(Action::Catalog(PresetCatalog::Recipes))
            .center();
        let target = Action::Catalog(PresetCatalog::Recipes);
        let target_rect = motion_action_frame(&gallery, target);
        assert!(
            scene.primitives().iter().any(|primitive| matches!(
                primitive,
                tekes_ui_scene::Primitive::Quad(quad) if quad.rect == target_rect
            )),
            "the live scene must draw the exact rectangle used for hit testing"
        );
        let target_center = target_rect.center();
        assert!(
            gallery.viewport.bounds().contains(target_center),
            "target {target_center:?} must be visible in {:?} (preview {preview_bounds:?}, offset {:?}, limit {:?})",
            gallery.viewport.bounds(),
            gallery.viewport.visual_offset(),
            gallery.viewport.limit()
        );

        // The same live scene is built from the preview rectangle; pointer
        // movement at the old page origin must not light a control there.
        assert_eq!(
            gallery.handle_input(&mv(stale_position.x.into(), stale_position.y.into())),
            Change::NONE
        );
        assert_eq!(gallery.motion.hovered, None);
        assert_eq!(
            gallery.handle_input(&mv(target_center.x.into(), target_center.y.into())),
            Change::REDRAW
        );
        let target_id = Layout::action_id(target);
        assert!(gallery.motion.button_models[&target_id].state.hovered);
        gallery.handle_input(&click(target_center.x.into(), target_center.y.into()));
        gallery.handle_input(&release(target_center.x.into(), target_center.y.into()));
        assert_eq!(gallery.motion.catalog, PresetCatalog::Recipes);

        // Repeat with the document offset changed. The preview origin is
        // recomputed by the actual Gallery build path before routing input.
        assert!(gallery.viewport.move_by(0.0, 180.0));
        gallery.set_view_size(920.0, 500.0);
        gallery.viewport.move_by(0.0, -gallery.viewport.offset.y);
        gallery.build_scene(&mut scene);
        let target = Action::Target(2);
        let initial_target = motion_action_frame(&gallery, target).center();
        let visible = gallery.viewport.bounds();
        gallery
            .viewport
            .move_by(0.0, initial_target.y - visible.center().y);
        scene.clear();
        gallery.build_scene(&mut scene);
        let preview_bounds = gallery.live_content_bounds();
        let target_rect = motion_action_frame(&gallery, target);
        assert!(
            scene.primitives().iter().any(|primitive| matches!(
                primitive,
                tekes_ui_scene::Primitive::Quad(quad) if quad.rect == target_rect
            )),
            "after scrolling/resizing, drawing and hit testing must share the rectangle: target={target_rect:?}, viewport={:?}, preview={preview_bounds:?}, nearby={:?}",
            gallery.viewport.bounds(),
            scene
                .primitives()
                .iter()
                .filter_map(|primitive| match primitive {
                    tekes_ui_scene::Primitive::Quad(quad)
                        if (quad.rect.center().y - target_rect.center().y).abs() < 200.0 =>
                        Some(quad.rect),
                    _ => None,
                })
                .collect::<Vec<_>>()
        );
        let target_center = target_rect.center();
        assert!(
            gallery.viewport.bounds().contains(target_center),
            "scrolled target {target_center:?} must be visible in {:?} (preview {preview_bounds:?}, offset {:?}, limit {:?})",
            gallery.viewport.bounds(),
            gallery.viewport.visual_offset(),
            gallery.viewport.limit()
        );
        gallery.motion.retarget(0.15);
        assert!(!gallery.motion.recipe.target());
        gallery.handle_input(&mv(target_center.x.into(), target_center.y.into()));
        assert!(
            gallery.motion.button_models[&Layout::action_id(target)]
                .state
                .hovered
        );
        gallery.handle_input(&click(target_center.x.into(), target_center.y.into()));
        gallery.handle_input(&release(target_center.x.into(), target_center.y.into()));
        assert!(gallery.motion.recipe.target());

        let target = Action::Target(0);
        let reverse_center = motion_action_frame(&gallery, target).center();
        gallery.handle_input(&mv(reverse_center.x.into(), reverse_center.y.into()));
        assert!(
            gallery.motion.button_models[&Layout::action_id(target)]
                .state
                .hovered
        );
        gallery.handle_input(&click(reverse_center.x.into(), reverse_center.y.into()));
        gallery.handle_input(&release(reverse_center.x.into(), reverse_center.y.into()));
        assert!(!gallery.motion.recipe.target());
    }

    #[test]
    fn motion_catalog_transitions_draw_and_hit_the_same_action_rectangles() {
        use motion_preview::PresetCatalog;
        use pages::motion::{Action, Layout};

        let mut gallery = Gallery::new();
        assert!(gallery.open_route("foundation-motion/playground"));
        gallery.motion.select_catalog(PresetCatalog::Recipes);
        gallery.set_view_size(1440.0, 900.0);
        let mut scene = Scene::new();
        gallery.build_scene(&mut scene);
        for catalog in [
            PresetCatalog::Apple,
            PresetCatalog::DesignSystem,
            PresetCatalog::Recipes,
            PresetCatalog::Symbols,
            PresetCatalog::DesignSystem,
            PresetCatalog::Recipes,
        ] {
            let catalog_action = Action::Catalog(catalog);
            let catalog_rect = motion_action_frame(&gallery, catalog_action);
            gallery.handle_input(&click(
                f64::from(catalog_rect.center().x),
                f64::from(catalog_rect.center().y),
            ));
            gallery.handle_input(&release(
                f64::from(catalog_rect.center().x),
                f64::from(catalog_rect.center().y),
            ));
            assert_eq!(gallery.motion.catalog, catalog);

            scene.clear();
            gallery.build_scene(&mut scene);
            let bounds = gallery.live_content_bounds();
            let layout = Layout::for_state(bounds, &gallery.motion, &mut gallery.text);
            let actions = layout.state_actions(&gallery.motion);
            assert!(
                actions.iter().all(|(action, rect)| {
                    let rendered_rect = motion_action_frame(&gallery, *action);
                    let visible = gallery.motion.button_models[&Layout::action_id(*action)]
                        .layout()
                        .is_some_and(|layout| layout.visible.is_some());
                    !visible || scene.primitives().iter().any(|primitive| {
                        matches!(
                            primitive,
                            tekes_ui_scene::Primitive::Quad(quad) if quad.rect == rendered_rect && rendered_rect == *rect
                        )
                    })
                }),
                "every live action in {catalog:?} must be drawn at its hit rectangle"
            );

            // Symbols intentionally moves target/replay controls directly below
            // its icon grid. Verify both their moved pixels and hit routing.
            let action = if catalog == PresetCatalog::Symbols {
                Action::Target(0)
            } else {
                Action::Preset(0)
            };
            let rect = actions
                .iter()
                .find_map(|(candidate, rect)| (*candidate == action).then_some(*rect))
                .unwrap();
            if catalog == PresetCatalog::Symbols {
                assert_eq!(rect.min_y(), bounds.min_y() + 238.0);
                assert!(rect.min_y() >= bounds.min_y() + 236.0);
            }
            let center = motion_action_frame(&gallery, action).center();
            gallery.handle_input(&mv(f64::from(center.x), f64::from(center.y)));
            scene.clear();
            gallery.build_scene(&mut scene);
            assert!(
                gallery.motion.button_models[&Layout::action_id(action)]
                    .state
                    .hovered
            );
            let rect = motion_action_frame(&gallery, action);
            assert!(scene.primitives().iter().any(|primitive| matches!(
                primitive,
                tekes_ui_scene::Primitive::Quad(quad) if quad.rect == rect
            )));
        }

        // A real Symbols animation preset remains actionable after catalog
        // changes, including the final Linear rotation entry.
        let symbols =
            motion_action_frame(&gallery, Action::Catalog(PresetCatalog::Symbols)).center();
        gallery.handle_input(&click(f64::from(symbols.x), f64::from(symbols.y)));
        gallery.handle_input(&release(f64::from(symbols.x), f64::from(symbols.y)));
        gallery.build_scene(&mut scene);
        let linear = Action::Preset(tekes_ui_motion::symbols::SymbolEffect::ALL.len());
        let linear_rect = motion_action_frame(&gallery, linear);
        let center = linear_rect.center();
        gallery.handle_input(&click(f64::from(center.x), f64::from(center.y)));
        gallery.handle_input(&release(f64::from(center.x), f64::from(center.y)));
        assert_eq!(
            gallery.motion.selected,
            tekes_ui_motion::symbols::SymbolEffect::ALL.len()
        );
        assert_eq!(
            gallery.motion.recipe.kind,
            tekes_ui_motion::recipes::RecipeKind::LinearRotation
        );

        // Hover clears outside the clipped preview; captured button release
        // outside cannot activate the control.
        let bounds = gallery.live_content_bounds();
        let visible = gallery.viewport.bounds();
        gallery.handle_input(&mv(
            f64::from(visible.min_x() - 4.0),
            f64::from(visible.min_y() - 4.0),
        ));
        assert_eq!(gallery.motion.hovered, None);
        let action = Action::Catalog(PresetCatalog::Apple);
        let point = Layout::new(bounds).button(action).center();
        gallery.handle_input(&click(f64::from(point.x), f64::from(point.y)));
        gallery.handle_input(&release(
            f64::from(visible.min_x() - 4.0),
            f64::from(visible.min_y() - 4.0),
        ));
        assert_eq!(gallery.motion.catalog, PresetCatalog::Symbols);
    }

    #[test]
    fn standalone_motion_keeps_viewport_content_as_its_draw_and_hit_origin() {
        use pages::motion::{Action, Layout};
        let mut gallery = Gallery::new();
        gallery.apply(Command::Page(Page::Motion));
        gallery.set_view_size(1000.0, 520.0);
        gallery.build_scene(&mut Scene::new());
        let bounds = gallery.live_content_bounds();
        assert_eq!(bounds, gallery.viewport.content());
        let action = Action::Catalog(motion_preview::PresetCatalog::Recipes);
        let point = motion_action_frame(&gallery, action).center();
        gallery.handle_input(&mv(point.x.into(), point.y.into()));
        assert!(
            gallery.motion.button_models[&Layout::action_id(action)]
                .state
                .hovered
        );
    }

    #[test]
    fn english_pages_have_no_untranslated_interface_labels() {
        let mut g = Gallery::new();
        g.set_locale(locale::Locale::En);
        let mut missing = std::collections::BTreeSet::new();
        for page in Page::ALL {
            g.apply(Command::Page(page));
            g.build_scene(&mut Scene::new());
            missing.extend(g.text.untranslated_labels());
        }
        g.apply(Command::Page(Page::Motion));
        for catalog in [
            motion_preview::PresetCatalog::Recipes,
            motion_preview::PresetCatalog::Symbols,
            motion_preview::PresetCatalog::Apple,
            motion_preview::PresetCatalog::DesignSystem,
        ] {
            g.motion.select_catalog(catalog);
            for index in 0..g.motion.presets.len() {
                g.motion.select(index);
                g.build_scene(&mut Scene::new());
                missing.extend(g.text.untranslated_labels());
            }
        }
        assert!(
            missing.is_empty(),
            "Untranslated interface copy: {missing:#?}"
        );
    }

    #[test]
    fn static_text_pickers_respond_to_clicks() {
        let mut g = Gallery::with_static_text(StaticText::new(None));
        g.apply(Command::Page(Page::StaticText));
        let center = |g: &Gallery, frame: tekes_ui_scene::Rect| {
            let c = frame.center();
            (
                f64::from(c.x - g.viewport.offset.x),
                f64::from(c.y - g.viewport.offset.y),
            )
        };
        g.build_scene(&mut Scene::new());
        // 选 mixed-cjk-latin：重绘，页和菜单不变。
        let frame = g.static_text.fixture_buttons()[1]
            .layout()
            .expect("fixture selector was rendered")
            .frame;
        let (x, y) = center(&g, frame);
        assert_eq!(g.handle_input(&click(x, y)), Change::REDRAW);
        assert_eq!(g.handle_input(&release(x, y)), Change::REDRAW);
        g.build_scene(&mut Scene::new());
        assert_eq!(g.static_text().fixture(), 1);
        // 重复选择不改变夹具，但 press/release 仍会清除交互反馈。
        let hover_before = g.hover();
        assert_eq!(g.handle_input(&click(x, y)), Change::REDRAW);
        assert_eq!(g.handle_input(&release(x, y)), Change::REDRAW);
        assert_eq!(g.hover(), hover_before);
        // 选宽度 500、320。
        g.build_scene(&mut Scene::new());
        let frame = g.static_text.width_buttons()[1]
            .layout()
            .expect("width selector was rendered")
            .frame;
        let (x, y) = center(&g, frame);
        assert_eq!(g.handle_input(&click(x, y)), Change::REDRAW);
        assert_eq!(g.handle_input(&release(x, y)), Change::REDRAW);
        g.build_scene(&mut Scene::new());
        assert_eq!(g.static_text().width(), 500);
        // sentences-a 只有 700：第 1 个宽度按钮不存在，点同样位置没有效果。
        let frame = g.static_text.fixture_buttons()[0]
            .layout()
            .expect("fixture selector was rendered")
            .frame;
        let (fx, fy) = center(&g, frame);
        g.handle_input(&click(fx, fy));
        g.handle_input(&release(fx, fy));
        g.build_scene(&mut Scene::new());
        assert_eq!(g.static_text().width(), 700);
        assert_eq!(g.handle_input(&click(x, y)), Change::NONE);
        // 其它页上同一位置的点击没有效果。
        g.apply(Command::Page(Page::Tokens));
        assert_eq!(g.handle_input(&click(x, y)), Change::NONE);
        assert_eq!(g.static_text().fixture(), 0);
    }

    #[test]
    fn canonical_calibration_buttons_share_clipped_placements_and_measured_extent() {
        let mut g = Gallery::new();
        assert!(g.open_route("lab-calibration/playground"));
        let index = match g.page() {
            Page::Workbench(i) => i,
            _ => panic!("route opens the calibration story"),
        };
        g.workbench.text_scale = 2;
        g.workbench.preview_width = 1;
        g.set_view_size(1000.0, 520.0);
        g.build_scene(&mut Scene::new());

        let header =
            g.workbench
                .preview_header_layout(index, &mut g.text, g.viewport.bounds().size.width);
        let body =
            pages::static_text::document_extent(header.preview_width, &mut g.text, &g.static_text);
        assert!(g.page_document_height() + 0.01 >= header.header_height + body);

        let window_center = |g: &Gallery, frame: tekes_ui_scene::Rect| {
            let center = frame.center();
            Point::new(
                center.x - g.viewport.offset.x,
                center.y - g.viewport.offset.y,
            )
        };
        let fixture_frame = g.static_text.fixture_buttons()[1]
            .layout()
            .expect("canonical fixture selector has retained placement")
            .frame;
        let fixture_point = window_center(&g, fixture_frame);
        assert!(g.viewport.bounds().contains(fixture_point));
        assert_eq!(
            g.handle_input(&click(
                f64::from(fixture_point.x),
                f64::from(fixture_point.y)
            )),
            Change::REDRAW
        );
        assert_eq!(
            g.handle_input(&release(
                f64::from(fixture_point.x),
                f64::from(fixture_point.y),
            )),
            Change::REDRAW
        );
        g.build_scene(&mut Scene::new());
        assert_eq!(g.static_text.fixture(), 1);

        let width_frame = g.static_text.width_buttons()[1]
            .layout()
            .expect("canonical width selector has retained placement")
            .frame;
        let width_point = window_center(&g, width_frame);
        assert!(g.viewport.bounds().contains(width_point));
        assert_eq!(
            g.handle_input(&click(f64::from(width_point.x), f64::from(width_point.y))),
            Change::REDRAW
        );
        assert_eq!(
            g.handle_input(&release(f64::from(width_point.x), f64::from(width_point.y),)),
            Change::REDRAW
        );
        g.build_scene(&mut Scene::new());
        assert_eq!(g.static_text.width(), 500);
    }

    #[test]
    fn static_text_page_changes_with_fixture_width_and_scale() {
        let mut text = TextEngine::new();
        let mut st = StaticText::new(None);
        let build = |text: &mut TextEngine, st: &mut StaticText, scale: f32| {
            text.set_scale(scale);
            let mut ctx = PageCtx {
                theme: Theme::LIGHT,
                text,
                static_text: st,
            };
            let mut s = Scene::new();
            ctx.text.begin_frame();
            compose(Page::StaticText, None, &mut ctx, &mut s);
            s
        };
        let a = build(&mut text, &mut st, 2.0);
        st.select_fixture(1);
        let b = build(&mut text, &mut st, 2.0);
        assert_ne!(a, b);
        st.select_width(320);
        let c = build(&mut text, &mut st, 2.0);
        assert_ne!(b, c);
        // 同一状态再编一次：相同；倍率变了，图元里的位图键变，所以不同。
        assert_eq!(c, build(&mut text, &mut st, 2.0));
        assert_ne!(c, build(&mut text, &mut st, 1.0));
    }
}
