//! Gallery 自己的状态与内容。
//!
//! 与平台层的耦合只有 `handle_input` 的 `Event` 参数和 `menu_bar()` 用到的菜单描述类型；
//! 坐标一律是逻辑点，左上角为原点。不依赖窗口和渲染器，所以可以在测试里直接驱动。
//!
//! 模块：
//! - `layout`：固定 1200x800 框、目录列几何、命中测试；
//! - `chrome`：目录列等外壳的绘制；
//! - `containers`：`StateMatrix`、`ReferenceSlot`、占位标签条；
//! - `pages`：各页（`primitives`、`tokens`）；
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
pub mod theme;

use tekes_ui_platform_macos::{
    Event, KeyEventKind, MenuBar, MouseButton, MouseEventKind, Point as PlatformPoint,
};
use tekes_ui_scene::{Point, Scene};
use tekes_ui_tokens::{ColorRole, Rgba, Theme};

pub use commands::Command;
pub use layout::{FRAME_H as CONTENT_HEIGHT, FRAME_W as CONTENT_WIDTH};
pub use pages::Page;
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
}

impl Default for Gallery {
    fn default() -> Self {
        Gallery::new()
    }
}

impl Gallery {
    pub fn new() -> Gallery {
        Gallery {
            page: Page::Primitives,
            hover: None,
            theme: ThemeState::default(),
        }
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
    pub fn build_scene(&self, scene: &mut Scene) {
        compose(self.page, self.hover, self.theme(), scene);
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
                            None => Change::NONE,
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
pub fn compose(page: Page, hover: Option<usize>, theme: Theme, scene: &mut Scene) {
    chrome::draw(scene, theme, page, hover);
    page.build(theme, layout::PAGE_CONTENT, scene);
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
        assert_eq!(
            g.handle_input(&key(KEY_DOWN, Modifiers::NONE)),
            Change::BOTH
        );
        assert_eq!(g.page(), Page::Tokens);
        assert_eq!(
            g.handle_input(&key(KEY_DOWN, Modifiers::NONE)),
            Change::NONE
        );
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

    #[test]
    fn scene_is_deterministic() {
        for page in Page::ALL {
            for theme in Theme::ALL {
                let mut a = Scene::new();
                let mut b = Scene::new();
                compose(page, None, theme, &mut a);
                compose(page, None, theme, &mut b);
                assert!(!a.is_empty());
                assert_eq!(a, b, "{page:?} {theme:?}");
            }
        }
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
        let build = |p, t| {
            let mut s = Scene::new();
            compose(p, None, t, &mut s);
            s
        };
        assert_ne!(
            build(Page::Primitives, Theme::LIGHT),
            build(Page::Tokens, Theme::LIGHT)
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
}
