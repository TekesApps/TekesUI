//! Gallery 自己的状态与内容。
//!
//! 与平台层的耦合只有 `handle_input` 的 `Event` 参数和 `menu_bar()` 用到的菜单描述类型；
//! 坐标一律是逻辑点，左上角为原点。不依赖窗口和渲染器，所以可以在测试里直接驱动。
//!
//! 模块：
//! - `layout`：固定 1200x800 框、目录列几何、命中测试；
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

pub mod chrome;
pub mod commands;
pub mod containers;
pub mod layout;
pub mod pages;
pub mod snapshot;
pub mod static_text;
pub mod text;
pub mod theme;

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

#[derive(Debug)]
pub struct Gallery {
    page: Page,
    hover: Option<usize>,
    theme: ThemeState,
    /// 文字引擎与窗口用的 atlas（主线程）。
    text: TextEngine,
    static_text: StaticText,
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
        self.text.set_scale(scale);
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

    /// 编译当前内容。内容永远排在固定的 1200x800 框里，与窗口尺寸无关。
    ///
    /// 先 `begin_frame`（一帧一次），再整个重编：scene 里的 `AtlasGlyph` 只在这一帧有效。
    pub fn build_scene(&mut self, scene: &mut Scene) {
        self.text.begin_frame();
        let mut ctx = PageCtx {
            theme: self.theme.effective(),
            text: &mut self.text,
            static_text: &mut self.static_text,
        };
        compose(self.page, self.hover, &mut ctx, scene);
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
                self.page = p;
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
        let next = self.page.index() as isize + delta;
        match usize::try_from(next).ok().and_then(|i| Page::ALL.get(i)) {
            Some(p) => self.apply(Command::Page(*p)),
            None => Change::NONE,
        }
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
        match event {
            Event::Mouse(m) => {
                let hit = || layout::catalog_hit_test(to_point(m.position), Page::ALL.len());
                match m.kind {
                    MouseEventKind::Moved
                    | MouseEventKind::Entered
                    | MouseEventKind::Dragged { .. } => redraw_if(self.set_hover(hit())),
                    MouseEventKind::Exited => redraw_if(self.set_hover(None)),
                    MouseEventKind::Down {
                        button: MouseButton::Left,
                        ..
                    } => {
                        let hover_changed = self.set_hover(hit());
                        let mut change = match hit() {
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
                match k.key_code {
                    KEY_UP => self.step_page(-1),
                    KEY_DOWN => self.step_page(1),
                    _ => Change::NONE,
                }
            }
            _ => Change::NONE,
        }
    }
}

impl Gallery {
    /// 页面区里的点击：目前只有静态文字页的夹具与宽度选择器。
    fn click_in_page(&mut self, position: PlatformPoint) -> Change {
        if self.page != Page::StaticText {
            return Change::NONE;
        }
        match pages::static_text::hit_test(
            layout::PAGE_CONTENT,
            &self.static_text,
            to_point(position),
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
    chrome::draw(scene, ctx.theme, ctx.text, page, hover);
    page.build(ctx, layout::PAGE_CONTENT, scene);
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
        assert_eq!(g.handle_input(&key(KEY_UP, Modifiers::NONE)), Change::NONE);
        for expected in [Page::Tokens, Page::StaticText] {
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
        assert_eq!(g.handle_input(&key(KEY_UP, Modifiers::NONE)), Change::BOTH);
        assert_eq!(g.page(), Page::Tokens);
        assert_eq!(g.handle_input(&key(KEY_UP, Modifiers::NONE)), Change::BOTH);
        assert_eq!(g.page(), Page::Primitives);
        // 带修饰键的方向键不处理。
        assert_eq!(
            g.handle_input(&key(KEY_DOWN, Modifiers::COMMAND)),
            Change::NONE
        );
        assert_eq!(g.page(), Page::Primitives);
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
