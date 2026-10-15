//! 菜单的纯数据描述。引擎提供 `MenuBar`，平台层把它翻译成 NSMenu。
//!
//! 标准的应用菜单（关于、隐藏、退出）和窗口菜单（最小化、缩放、关闭、全屏）由平台层
//! 自动生成，使用 AppKit 标准 selector，不需要引擎参与；这里只描述引擎自己的菜单，
//! 它们会被放在应用菜单与窗口菜单之间。

use std::collections::HashSet;
use std::fmt;

use crate::event::Modifiers;

/// 引擎定义的命令编号。菜单项被选中时以 `Event::Command` 投递。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CommandId(pub u32);

/// 菜单项的快捷键。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyEquivalent {
    /// 不带 Shift 的那个键，例如 `'1'`、`'k'`。字母不分大小写，平台层统一转小写。
    pub key: char,
    pub modifiers: Modifiers,
}

impl KeyEquivalent {
    pub fn command(key: char) -> Self {
        KeyEquivalent {
            key,
            modifiers: Modifiers::COMMAND,
        }
    }

    pub fn new(key: char, modifiers: Modifiers) -> Self {
        KeyEquivalent { key, modifiers }
    }

    /// 交给 `NSMenuItem.keyEquivalent` 的字符串。
    pub fn ns_key(self) -> String {
        self.key.to_lowercase().collect()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MenuCommand {
    pub title: String,
    pub command: CommandId,
    pub key_equivalent: Option<KeyEquivalent>,
    /// 初始勾选状态。运行中用 `Window::set_command_checked` 修改。
    pub checked: bool,
}

impl MenuCommand {
    pub fn new(title: impl Into<String>, command: CommandId) -> Self {
        MenuCommand {
            title: title.into(),
            command,
            key_equivalent: None,
            checked: false,
        }
    }

    pub fn key(mut self, key_equivalent: KeyEquivalent) -> Self {
        self.key_equivalent = Some(key_equivalent);
        self
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MenuItem {
    Command(MenuCommand),
    Separator,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Menu {
    pub title: String,
    pub items: Vec<MenuItem>,
}

impl Menu {
    pub fn new(title: impl Into<String>, items: Vec<MenuItem>) -> Self {
        Menu {
            title: title.into(),
            items,
        }
    }
}

/// 引擎提供的菜单栏描述。可以为空，空时只有标准的应用菜单和窗口菜单。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MenuBar {
    pub menus: Vec<Menu>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MenuError {
    /// 同一个 `CommandId` 出现了不止一次，`set_command_checked` 会有歧义。
    DuplicateCommand(CommandId),
    EmptyTitle,
}

impl fmt::Display for MenuError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MenuError::DuplicateCommand(id) => write!(f, "菜单里命令 {} 出现了多次", id.0),
            MenuError::EmptyTitle => write!(f, "菜单或菜单项的标题为空"),
        }
    }
}

impl std::error::Error for MenuError {}

impl MenuBar {
    /// 依次遍历所有命令项。
    pub fn commands(&self) -> impl Iterator<Item = &MenuCommand> {
        self.menus
            .iter()
            .flat_map(|m| m.items.iter())
            .filter_map(|i| match i {
                MenuItem::Command(c) => Some(c),
                MenuItem::Separator => None,
            })
    }

    /// 检查描述是否自洽：标题非空、命令编号唯一。
    pub fn validate(&self) -> Result<(), MenuError> {
        let mut seen = HashSet::new();
        for menu in &self.menus {
            if menu.title.is_empty() {
                return Err(MenuError::EmptyTitle);
            }
        }
        for c in self.commands() {
            if c.title.is_empty() {
                return Err(MenuError::EmptyTitle);
            }
            if !seen.insert(c.command) {
                return Err(MenuError::DuplicateCommand(c.command));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> MenuBar {
        MenuBar {
            menus: vec![Menu::new(
                "View",
                vec![
                    MenuItem::Command(
                        MenuCommand::new("Light", CommandId(1))
                            .key(KeyEquivalent::command('1'))
                            .checked(true),
                    ),
                    MenuItem::Command(MenuCommand::new("Dark", CommandId(2))),
                    MenuItem::Separator,
                    MenuItem::Command(MenuCommand::new("Increase Contrast", CommandId(3))),
                ],
            )],
        }
    }

    #[test]
    fn validate_ok() {
        assert_eq!(sample().validate(), Ok(()));
        assert_eq!(MenuBar::default().validate(), Ok(()));
    }

    #[test]
    fn commands_skip_separators() {
        let ids: Vec<_> = sample().commands().map(|c| c.command.0).collect();
        assert_eq!(ids, [1, 2, 3]);
    }

    #[test]
    fn duplicate_command_rejected() {
        let mut bar = sample();
        bar.menus[0]
            .items
            .push(MenuItem::Command(MenuCommand::new("Again", CommandId(2))));
        assert_eq!(
            bar.validate(),
            Err(MenuError::DuplicateCommand(CommandId(2)))
        );
    }

    #[test]
    fn empty_titles_rejected() {
        let mut bar = sample();
        bar.menus[0].title.clear();
        assert_eq!(bar.validate(), Err(MenuError::EmptyTitle));
        let bar = MenuBar {
            menus: vec![Menu::new(
                "X",
                vec![MenuItem::Command(MenuCommand::new("", CommandId(1)))],
            )],
        };
        assert_eq!(bar.validate(), Err(MenuError::EmptyTitle));
    }

    #[test]
    fn key_equivalent_is_lowercase() {
        assert_eq!(KeyEquivalent::command('K').ns_key(), "k");
        assert_eq!(KeyEquivalent::command('1').ns_key(), "1");
    }
}
