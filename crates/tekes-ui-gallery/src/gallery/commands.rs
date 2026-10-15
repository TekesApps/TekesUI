//! 菜单：命令编号 ↔ 命令的映射，以及菜单栏描述。
//!
//! | 菜单 | 项 | 快捷键 |
//! |---|---|---|
//! | Pages | 每页一项，勾选当前页 | ⌘1、⌘2、…（按页序） |
//! | Appearance | Follow System / Light / Dark（单选） | ⌃⌘1 / ⌃⌘2 / ⌃⌘3 |
//! | Appearance | Increase Contrast: Follow System / On / Off（单选） | ⌃⌘4 / ⌃⌘5 / ⌃⌘6 |
//!
//! 命令编号：页面 100 + 页序，外观 200 + 选项序，对比度 300 + 选项序。

use tekes_ui_platform_macos::{
    CommandId, KeyEquivalent, Menu, MenuBar, MenuCommand, MenuItem, Modifiers,
};

use super::pages::Page;
use super::theme::{AppearanceChoice, ContrastChoice};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    Page(Page),
    Appearance(AppearanceChoice),
    Contrast(ContrastChoice),
}

const PAGE_BASE: u32 = 100;
const APPEARANCE_BASE: u32 = 200;
const CONTRAST_BASE: u32 = 300;

impl Command {
    /// 全部命令，按菜单里出现的顺序。
    pub fn all() -> Vec<Command> {
        let mut v: Vec<Command> = super::navigation::pages()
            .into_iter()
            .map(Command::Page)
            .collect();
        v.extend(
            AppearanceChoice::ALL
                .iter()
                .map(|a| Command::Appearance(*a)),
        );
        v.extend(ContrastChoice::ALL.iter().map(|c| Command::Contrast(*c)));
        v
    }

    pub fn id(self) -> CommandId {
        CommandId(match self {
            Command::Page(p) => PAGE_BASE + index_of(&Page::ALL, p),
            Command::Appearance(a) => APPEARANCE_BASE + index_of(&AppearanceChoice::ALL, a),
            Command::Contrast(c) => CONTRAST_BASE + index_of(&ContrastChoice::ALL, c),
        })
    }

    pub fn from_id(id: CommandId) -> Option<Command> {
        Command::all().into_iter().find(|c| c.id() == id)
    }

    fn title(self) -> String {
        match self {
            Command::Page(p) => p.name().to_string(),
            Command::Appearance(AppearanceChoice::System) => "Follow System".into(),
            Command::Appearance(AppearanceChoice::Light) => "Light".into(),
            Command::Appearance(AppearanceChoice::Dark) => "Dark".into(),
            Command::Contrast(ContrastChoice::System) => "Increase Contrast: Follow System".into(),
            Command::Contrast(ContrastChoice::On) => "Increase Contrast: On".into(),
            Command::Contrast(ContrastChoice::Off) => "Increase Contrast: Off".into(),
        }
    }

    fn key(self) -> Option<KeyEquivalent> {
        let ctrl_cmd = Modifiers {
            control: true,
            command: true,
            ..Modifiers::NONE
        };
        match self {
            Command::Page(p) => {
                let n = index_of(&super::navigation::pages(), p) + 1;
                char::from_digit(n, 10).map(KeyEquivalent::command)
            }
            Command::Appearance(a) => char::from_digit(1 + index_of(&AppearanceChoice::ALL, a), 10)
                .map(|d| KeyEquivalent::new(d, ctrl_cmd)),
            Command::Contrast(c) => char::from_digit(4 + index_of(&ContrastChoice::ALL, c), 10)
                .map(|d| KeyEquivalent::new(d, ctrl_cmd)),
        }
    }
}

fn index_of<T: PartialEq + Copy>(all: &[T], v: T) -> u32 {
    all.iter().position(|x| *x == v).unwrap_or(0) as u32
}

/// 菜单栏描述。`checked` 给出每个命令的初始勾选状态。
pub fn menu_bar(checked: impl Fn(Command) -> bool) -> MenuBar {
    let item = |c: Command| {
        let mut m = MenuCommand::new(c.title(), c.id()).checked(checked(c));
        if let Some(k) = c.key() {
            m = m.key(k);
        }
        MenuItem::Command(m)
    };
    let mut pages = Vec::new();
    for (index, section) in super::navigation::sections().iter().enumerate() {
        if index > 0 {
            pages.push(MenuItem::Separator);
        }
        pages.extend(section.items.iter().map(|(p, _)| item(Command::Page(*p))));
    }
    let mut appearance: Vec<MenuItem> = AppearanceChoice::ALL
        .iter()
        .map(|a| item(Command::Appearance(*a)))
        .collect();
    appearance.push(MenuItem::Separator);
    appearance.extend(
        ContrastChoice::ALL
            .iter()
            .map(|c| item(Command::Contrast(*c))),
    );
    MenuBar {
        menus: vec![
            Menu::new("Pages", pages),
            Menu::new("Appearance", appearance),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_roundtrip_and_are_unique() {
        let all = Command::all();
        for c in &all {
            assert_eq!(Command::from_id(c.id()), Some(*c));
        }
        let mut ids: Vec<_> = all.iter().map(|c| c.id()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), all.len());
        assert_eq!(Command::from_id(CommandId(0)), None);
        assert_eq!(Command::from_id(CommandId(150)), None);
    }

    #[test]
    fn known_ids() {
        assert_eq!(Command::Page(Page::Primitives).id(), CommandId(100));
        assert_eq!(Command::Page(Page::Tokens).id(), CommandId(101));
        assert_eq!(
            Command::Appearance(AppearanceChoice::Dark).id(),
            CommandId(202)
        );
        assert_eq!(Command::Contrast(ContrastChoice::Off).id(), CommandId(302));
    }

    #[test]
    fn menu_bar_is_valid_and_covers_all_commands() {
        let bar = menu_bar(|_| false);
        assert_eq!(bar.validate(), Ok(()));
        let ids: Vec<_> = bar.commands().map(|c| c.command).collect();
        let want: Vec<_> = Command::all().iter().map(|c| c.id()).collect();
        assert_eq!(ids, want);
    }

    #[test]
    fn page_shortcuts_are_cmd_digits_in_order() {
        let bar = menu_bar(|_| false);
        let pages: Vec<_> = bar.menus[0]
            .items
            .iter()
            .filter_map(|i| match i {
                MenuItem::Command(c) => c.key_equivalent,
                _ => None,
            })
            .collect();
        assert_eq!(pages.len(), Page::ALL.len().min(9));
        for (i, k) in pages.iter().enumerate() {
            assert_eq!(k.key, char::from_digit(i as u32 + 1, 10).unwrap());
            assert_eq!(k.modifiers, Modifiers::COMMAND);
        }
    }

    #[test]
    fn shortcuts_do_not_collide() {
        let keys: Vec<_> = Command::all().iter().filter_map(|c| c.key()).collect();
        for i in 0..keys.len() {
            for j in i + 1..keys.len() {
                assert_ne!(keys[i], keys[j]);
            }
        }
    }

    /// 已知被 macOS 或标准 AppKit 菜单占用的组合（键，⌃⌥⇧⌘ 四个修饰）。
    /// 来源：Apple 支持文档“Mac 键盘快捷键”、系统设置 → 键盘 → 键盘快捷键里的系统默认值
    /// （Mission Control、Dock、Spotlight、截屏、辅助功能缩放/反转/对比度），
    /// 以及 AppKit 标准 App/窗口/编辑菜单。不求完整，只拦已知的坑；
    /// 本机实际启用的热键见 docs/gallery.md。
    const RESERVED: &[(char, [bool; 4])] = &[
        // (键, [control, alt, shift, command])
        ('d', [false, true, false, true]),  // ⌥⌘D 显示/隐藏 Dock
        ('d', [true, false, false, true]),  // ⌃⌘D 查词
        ('h', [false, false, false, true]), // ⌘H 隐藏
        ('h', [false, true, false, true]),  // ⌥⌘H 隐藏其他
        ('m', [false, false, false, true]), // ⌘M 最小化
        ('m', [false, true, false, true]),  // ⌥⌘M 全部最小化
        ('w', [false, false, false, true]), // ⌘W 关闭
        ('w', [false, true, false, true]),  // ⌥⌘W 关闭全部
        ('q', [false, false, false, true]), // ⌘Q 退出
        ('q', [true, false, false, true]),  // ⌃⌘Q 锁定屏幕
        ('f', [true, false, false, true]),  // ⌃⌘F 全屏
        ('`', [false, false, false, true]), // ⌘` 切换窗口
        (',', [false, false, false, true]), // ⌘, 设置
        ('?', [false, false, false, true]), // ⌘? 帮助
        ('/', [false, true, false, true]),  // ⌥⌘/ 帮助搜索
        (' ', [false, false, false, true]), // ⌘Space Spotlight
        (' ', [false, true, false, true]),  // ⌥⌘Space Finder 搜索
        (' ', [true, false, false, true]),  // ⌃⌘Space 表情与符号
        ('8', [false, true, false, true]),  // ⌥⌘8 缩放开关
        ('8', [true, true, false, true]),   // ⌃⌥⌘8 反转颜色
        ('=', [false, true, false, true]),  // ⌥⌘= 放大
        ('-', [false, true, false, true]),  // ⌥⌘- 缩小
        (',', [true, true, false, true]),   // ⌃⌥⌘, 降低对比度
        ('.', [true, true, false, true]),   // ⌃⌥⌘. 增加对比度
        ('3', [false, false, true, true]),  // ⇧⌘3 截屏
        ('4', [false, false, true, true]),  // ⇧⌘4 截取选区
        ('5', [false, false, true, true]),  // ⇧⌘5 截屏工具
        ('6', [false, false, true, true]),  // ⇧⌘6 Touch Bar 截屏
    ];

    #[test]
    fn shortcuts_avoid_reserved_system_combinations() {
        for c in Command::all() {
            let Some(k) = c.key() else { continue };
            let m = k.modifiers;
            let key = k.key.to_ascii_lowercase();
            for (rk, rm) in RESERVED {
                assert!(
                    !(*rk == key && *rm == [m.control, m.alt, m.shift, m.command]),
                    "{c:?} 用了被占用的快捷键 {k:?}"
                );
            }
        }
    }

    #[test]
    fn appearance_and_contrast_are_ctrl_cmd_digits() {
        for c in Command::all() {
            if matches!(c, Command::Appearance(_) | Command::Contrast(_)) {
                let k = c.key().unwrap();
                assert!(k.key.is_ascii_digit());
                assert_eq!(
                    k.modifiers,
                    Modifiers {
                        control: true,
                        command: true,
                        ..Modifiers::NONE
                    }
                );
            }
        }
    }

    #[test]
    fn checked_state_comes_from_callback() {
        let bar = menu_bar(|c| c == Command::Page(Page::Tokens));
        let checked: Vec<_> = bar
            .commands()
            .filter(|c| c.checked)
            .map(|c| c.command)
            .collect();
        assert_eq!(checked, [Command::Page(Page::Tokens).id()]);
    }
}
