//! 菜单：命令编号 ↔ 命令的映射，以及菜单栏描述。
//!
//! | 菜单 | 项 | 快捷键 |
//! |---|---|---|
//! | Pages | 每页一项，勾选当前页 | ⌘1、⌘2、…（按页序） |
//! | Appearance | Follow System / Light / Dark（单选） | ⌥⌘S / ⌥⌘L / ⌥⌘D |
//! | Appearance | Increase Contrast: Follow System / On / Off（单选） | ⌃⌥⌘C / ⌥⌘C / ⇧⌥⌘C |
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
        let mut v: Vec<Command> = Page::ALL.iter().map(|p| Command::Page(*p)).collect();
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
        let opt_cmd = Modifiers {
            alt: true,
            command: true,
            ..Modifiers::NONE
        };
        match self {
            Command::Page(p) => {
                let n = index_of(&Page::ALL, p) + 1;
                char::from_digit(n, 10).map(KeyEquivalent::command)
            }
            Command::Appearance(AppearanceChoice::System) => Some(KeyEquivalent::new('s', opt_cmd)),
            Command::Appearance(AppearanceChoice::Light) => Some(KeyEquivalent::new('l', opt_cmd)),
            Command::Appearance(AppearanceChoice::Dark) => Some(KeyEquivalent::new('d', opt_cmd)),
            Command::Contrast(ContrastChoice::System) => Some(KeyEquivalent::new(
                'c',
                Modifiers {
                    control: true,
                    ..opt_cmd
                },
            )),
            Command::Contrast(ContrastChoice::On) => Some(KeyEquivalent::new('c', opt_cmd)),
            Command::Contrast(ContrastChoice::Off) => Some(KeyEquivalent::new(
                'c',
                Modifiers {
                    shift: true,
                    ..opt_cmd
                },
            )),
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
    let pages = Page::ALL.iter().map(|p| item(Command::Page(*p))).collect();
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
        assert_eq!(pages.len(), Page::ALL.len());
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
