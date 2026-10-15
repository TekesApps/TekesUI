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

pub mod catalog;
pub mod chrome;
pub mod commands;
pub mod containers;
pub mod icon_catalog;
pub mod layout;
pub mod motion_preview;
pub mod navigation;
pub mod pages;
pub mod snapshot;
pub mod static_text;
pub mod text;
pub mod text_demo;
pub mod theme;
pub mod viewport;

use tekes_ui_platform_macos::{
    Event, KeyEventKind, MenuBar, MouseButton, MouseEventKind, Point as PlatformPoint,
};
use tekes_ui_render::Renderer;
use tekes_ui_scene::{GlyphAtlas, Point, Scene};
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
}

impl Default for Gallery {
    fn default() -> Self {
        Gallery::new()
    }
}

impl Gallery {
    /// 参照目录取自环境变量 `TEKESUI_REFERENCE_DIR`。
    pub fn new() -> Gallery {
        Gallery::with_static_text(StaticText::from_env())
    }

    pub fn with_static_text(static_text: StaticText) -> Gallery {
        Gallery {
            page: Page::Primitives,
            hover: None,
            theme: ThemeState::default(),
            text: TextEngine::new(),
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
        }
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
        if self.page == Page::Icons {
            let visible = pages::icons::visible_legacy(
                &self.icons,
                self.icon_search.query(),
                self.viewport.content(),
                self.viewport.bounds(),
            );
            self.icons.prepare(renderer, self.theme(), scale, &visible);
        }
        if self.page == Page::StaticText {
            let theme = self.theme();
            self.static_text
                .prepare(&mut self.text, renderer, theme, scale);
        }
    }

    #[cfg(test)]
    pub fn static_text(&self) -> &StaticText {
        &self.static_text
    }

    #[cfg(test)]
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
            self.page.name(),
            theme_label(self.theme())
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
        }
    }

    /// 菜单栏描述，勾选状态取自当前状态。
    pub fn menu_bar(&self) -> MenuBar {
        commands::menu_bar(|c| self.is_checked(c))
    }

    /// 编译当前内容。内容永远排在固定的 JSON 捕获框里，与窗口尺寸无关。
    ///
    /// 先 `begin_frame`（一帧一次），再整个重编：scene 里的 `AtlasGlyph` 只在这一帧有效。
    pub fn build_scene(&mut self, scene: &mut Scene) {
        self.layout_page_extent();
        self.text.begin_frame();
        let mut ctx = PageCtx {
            theme: self.theme.effective(),
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
        let mut layer = Scene::new();
        if self.page == Page::Icons {
            self.icon_search.build_search(
                &mut ctx,
                pages::icons::search_rect(self.viewport.content()),
                &mut layer,
            );
            pages::icons::build_catalog(
                &mut ctx,
                self.viewport.content(),
                &mut layer,
                self.icon_search.query(),
                &self.icons,
                self.viewport.bounds(),
            );
        } else if self.page == Page::Motion {
            pages::motion::build(&mut ctx, self.viewport.content(), &mut layer, &self.motion);
        } else if self.page == Page::Text {
            self.text_view
                .build(&mut ctx, self.viewport.content(), &mut layer);
        } else if self.page == Page::ScrollArea {
            self.scroll_demo
                .build(&mut ctx, self.viewport.content(), &mut layer);
        } else if self.page == Page::TextEditing {
            self.editor
                .build(&mut ctx, self.viewport.content(), &mut layer);
        } else if let Some(category) = self.page.category() {
            pages::showcase::build(
                category,
                &mut ctx,
                self.viewport.content(),
                &mut layer,
                &self.showcase,
            );
        } else {
            self.page
                .build(&mut ctx, self.viewport.content(), &mut layer);
        }
        self.viewport
            .append_visible(&layer, scene, ctx.text.scale(), ctx.theme);
    }

    pub fn needs_animation(&self) -> bool {
        self.viewport.needs_animation()
            || self.sidebar.scroll.needs_animation()
            || match self.page {
                Page::TextEditing => self.editor.needs_animation(),
                Page::Text => self.text_view.needs_animation(),
                Page::ScrollArea => self.scroll_demo.needs_animation(),
                Page::Motion => self.motion.needs_animation(),
                _ => false,
            }
    }
    pub fn animate(&mut self, dt: f32) -> bool {
        let mut dirty = self.viewport.tick(dt) | self.sidebar.scroll.tick(dt);
        dirty |= match self.page {
            Page::TextEditing => self.editor.animate(dt),
            Page::Text => self.text_view.animate(dt),
            Page::ScrollArea => self.scroll_demo.animate(dt),
            Page::Motion => self.motion.animate(dt),
            _ => false,
        };
        dirty
    }
    fn layout_page_extent(&mut self) {
        let size = if self.page == Page::Icons {
            pages::icons::extent(&self.icons, self.icon_search.query())
        } else if self.page == Page::Motion {
            pages::motion::extent(layout::PAGE_CONTENT)
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
        let mut dirty = self.motion.set_system_reduced_motion(reduced);
        dirty |= self.viewport.set_reduced_motion(reduced);
        dirty |= self.sidebar.scroll.set_reduced_motion(reduced);
        dirty |= self.editor.set_reduced_motion(reduced);
        dirty |= self.text_view.set_reduced_motion(reduced);
        dirty |= self.scroll_demo.set_reduced_motion(reduced);
        dirty |= self.icon_search.set_reduced_motion(reduced);
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
        self.editor.scroll_metrics()
    }
    pub fn scroll_editor_to(&mut self, offset: f32) {
        self.editor.scroll_to(offset);
    }
    pub fn text_input_state(&self) -> Option<tekes_ui_platform_macos::TextInputState> {
        if self.page == Page::TextEditing {
            self.editor.state()
        } else if self.page == Page::Text {
            self.text_view.state()
        } else if self.page == Page::ScrollArea {
            self.scroll_demo.state()
        } else if self.page == Page::Icons {
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
        let editor = match self.page {
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
            Command::Page(p) => {
                if self.page == p {
                    return Change::NONE;
                }
                self.end_scroll_gestures();
                self.editor.blur();
                self.text_view.blur();
                self.scroll_demo.blur();
                self.motion.deactivate();
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
        let index = pages.iter().position(|p| *p == self.page).unwrap();
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
        let editor = match self.page {
            Page::TextEditing => Some(&self.editor),
            Page::Text => Some(&self.text_view),
            Page::ScrollArea => Some(&self.scroll_demo),
            _ => None,
        };
        if editor.is_some_and(|e| e.contains_surface(p)) {
            Some(ScrollOwner::Text(self.page))
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
            area.wheel(
                x,
                y,
                event.precise,
                event.phase == ScrollPhase::Began,
                end,
                event.momentum_phase != ScrollPhase::None,
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
            self.editor.set_window_active(*active);
            self.text_view.set_window_active(*active);
            self.scroll_demo.set_window_active(*active);
            self.icon_search.set_window_active(*active);
            if !active {
                self.end_scroll_gestures();
                self.motion.deactivate();
                self.sidebar.scroll.cancel();
                self.viewport.cancel();
            }
            return Change::REDRAW;
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
        if self.page == Page::Icons {
            let point = match event {
                Event::Mouse(m) => Some(to_point(m.position)),
                _ => None,
            };
            let search_hit = point.is_some_and(|p| {
                self.viewport.bounds().contains(p)
                    && pages::icons::search_rect(self.viewport.content()).contains(p)
            });
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
        if matches!(self.page, Page::TextEditing | Page::Text | Page::ScrollArea) {
            let editor = match self.page {
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
        if self.page == Page::Motion {
            let route = match event {
                Event::Mouse(m) => {
                    self.viewport.bounds().contains(to_point(m.position))
                        || self.motion.dragging.is_some()
                        || self.motion.pressed.is_some()
                }
                _ => false,
            };
            if route
                && pages::motion::handle_input(&mut self.motion, event, self.viewport.content())
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
        match pages::static_text::hit_test(
            layout::PAGE_CONTENT,
            &self.static_text,
            match self.viewport.point_in_content(to_point(position)) {
                Some(p) => p,
                None => return Change::NONE,
            },
        ) {
            Some(pick) => redraw_if(self.static_text.apply(pick)),
            None => Change::NONE,
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
    use tekes_ui_platform_macos::{CommandId, KeyEvent, Modifiers, MouseEvent};

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
        assert_eq!(g.hover(), Some(1));
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
        assert_eq!(g.page(), Page::Tokens);
        // 再点同一行：页没变，hover 也已经在这一行，什么都不用做。
        assert_eq!(g.handle_input(&click(x, y)), Change::NONE);
        // 点页面区：无事发生。
        assert!(!g.handle_input(&click(700.0, 400.0)).chrome);
        assert_eq!(g.page(), Page::Tokens);
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
        assert_eq!(g.apply(Command::Page(Page::Primitives)), Change::NONE);
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
        g.apply(Command::Page(Page::Tokens));
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

    fn wheel(
        x: f64,
        y: f64,
        phase: tekes_ui_platform_macos::ScrollPhase,
        momentum: tekes_ui_platform_macos::ScrollPhase,
    ) -> Event {
        Event::Scroll(tekes_ui_platform_macos::ScrollEvent {
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
    fn text_caret_is_revealed_through_both_nested_scroll_areas() {
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
        assert!(g.viewport.offset.y > 0.0);
    }
    #[test]
    fn editor_outer_scrollbar_captures_before_text_surface_input() {
        let mut g = Gallery::new();
        g.set_view_size(1000.0, 480.0);
        g.apply(Command::Page(Page::TextEditing));
        g.build_scene(&mut Scene::new());
        let track = g.viewport.track(true);
        let p = g
            .viewport
            .thumb(
                true,
                track.inset(-tekes_ui_tokens::Spacing::Micro.value()),
                tekes_ui_tokens::ControlSize::RowHeight.value(),
            )
            .center();
        g.handle_input(&click(f64::from(p.x), f64::from(p.y)));
        assert_eq!(g.viewport.dragging_axis(), Some(true));
        g.handle_input(&Event::Mouse(MouseEvent {
            kind: MouseEventKind::Dragged {
                button: MouseButton::Left,
            },
            position: PlatformPoint {
                x: f64::from(p.x),
                y: f64::from(track.max_y()),
            },
            modifiers: Modifiers::NONE,
        }));
        assert!(g.viewport.offset.y > 0.0);
        g.handle_input(&Event::Mouse(MouseEvent {
            kind: MouseEventKind::Up {
                button: MouseButton::Left,
                click_count: 1,
            },
            position: PlatformPoint { x: 0.0, y: 0.0 },
            modifiers: Modifiers::NONE,
        }));
        assert_eq!(g.viewport.dragging_axis(), None);
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
    fn static_text_pickers_respond_to_clicks() {
        use pages::static_text::{fixture_button, width_button};
        let mut g = Gallery::with_static_text(StaticText::new(None));
        g.apply(Command::Page(Page::StaticText));
        let content = layout::PAGE_CONTENT;
        let center = |r: tekes_ui_scene::Rect| {
            let c = r.center();
            (c.x as f64, c.y as f64)
        };
        // 选 mixed-cjk-latin：重绘，页和菜单不变。
        let (x, y) = center(fixture_button(content, 1));
        let c = g.handle_input(&click(x, y));
        assert_eq!(c, Change::REDRAW);
        assert_eq!(g.static_text().fixture(), 1);
        // 再点同一个按钮：什么都不变，不重绘。
        let hover_before = g.hover();
        let c = g.handle_input(&click(x, y));
        assert!(!c.redraw && !c.chrome);
        assert_eq!(g.hover(), hover_before);
        // 选宽度 500、320。
        let (x, y) = center(width_button(content, 1));
        assert_eq!(g.handle_input(&click(x, y)), Change::REDRAW);
        assert_eq!(g.static_text().width(), 500);
        // sentences-a 只有 700：第 1 个宽度按钮不存在，点同样位置没有效果。
        let (fx, fy) = center(fixture_button(content, 0));
        g.handle_input(&click(fx, fy));
        assert_eq!(g.static_text().width(), 700);
        assert_eq!(g.handle_input(&click(x, y)), Change::NONE);
        // 其它页上同一位置的点击没有效果。
        g.apply(Command::Page(Page::Tokens));
        let (x, y) = center(fixture_button(content, 2));
        assert_eq!(g.handle_input(&click(x, y)), Change::NONE);
        assert_eq!(g.static_text().fixture(), 0);
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
