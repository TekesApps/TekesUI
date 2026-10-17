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

use super::locale::Locale;
use super::pages::Page;
use super::theme::{AppearanceChoice, ContrastChoice};
use tekes_ui_tokens::palette::gallery_brand_presets;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    Page(Page),
    Appearance(AppearanceChoice),
    Contrast(ContrastChoice),
    Language(Locale),
    PreviewWidth(usize),
    ReducedMotion(bool),
    Brand(usize),
    Density(usize),
    TextScale(usize),
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
        v.extend([
            Command::PreviewWidth(0),
            Command::PreviewWidth(1),
            Command::PreviewWidth(2),
            Command::ReducedMotion(false),
            Command::ReducedMotion(true),
        ]);
        v.extend(
            gallery_brand_presets()
                .iter()
                .map(|preset| Command::Brand(preset.palette_index)),
        );
        v.extend([
            Command::Density(0),
            Command::Density(1),
            Command::TextScale(0),
            Command::TextScale(1),
            Command::TextScale(2),
        ]);
        v.extend(Locale::ALL.into_iter().map(Command::Language));

        v
    }

    pub fn id(self) -> CommandId {
        CommandId(match self {
            Command::Density(v) => 530 + v as u32,
            Command::TextScale(v) => 540 + v as u32,
            Command::Brand(v @ 0..=2) => 520 + v as u32,
            Command::Brand(v) => 560 + (v - 3) as u32,
            Command::PreviewWidth(v) => 500 + v as u32,
            Command::ReducedMotion(v) => 510 + u32::from(v),
            Command::Page(p) => PAGE_BASE + index_of(&Page::ALL, p),
            Command::Appearance(a) => APPEARANCE_BASE + index_of(&AppearanceChoice::ALL, a),
            Command::Contrast(c) => CONTRAST_BASE + index_of(&ContrastChoice::ALL, c),
            Command::Language(l) => 400 + index_of(&Locale::ALL, l),
        })
    }

    pub fn from_id(id: CommandId) -> Option<Command> {
        Page::ALL
            .into_iter()
            .map(Command::Page)
            .chain(Command::all())
            .find(|c| c.id() == id)
    }

    pub fn title(self, locale: Locale) -> String {
        if let Command::Language(l) = self {
            return if l == Locale::En {
                "English"
            } else {
                "简体中文"
            }
            .into();
        }
        let title = match self {
            Command::Density(v) => ["Density: comfortable", "Density: compact"][v].into(),
            Command::TextScale(v) => ["Text: 100%", "Text: 125%", "Text: 150%"][v].into(),
            Command::Brand(v) => format!(
                "Brand: {}",
                tekes_ui_tokens::palette::gallery_brand_preset(v)
                    .map(|preset| preset.name)
                    .unwrap_or("unknown")
            ),
            Command::PreviewWidth(v) => {
                ["Preview: fill", "Preview: 420 pt", "Preview: 720 pt"][v].into()
            }
            Command::ReducedMotion(v) => if v {
                "Reduced motion: On"
            } else {
                "Reduced motion: Off"
            }
            .into(),
            Command::Page(p) => p.name().to_string(),
            Command::Appearance(AppearanceChoice::System) => "Follow System".into(),
            Command::Appearance(AppearanceChoice::Light) => "Light".into(),
            Command::Appearance(AppearanceChoice::Dark) => "Dark".into(),
            Command::Contrast(ContrastChoice::System) => "Increase Contrast: Follow System".into(),
            Command::Contrast(ContrastChoice::On) => "Increase Contrast: On".into(),
            Command::Contrast(ContrastChoice::Off) => "Increase Contrast: Off".into(),
            Command::Language(_) => unreachable!(),
        };
        locale.text(&title).into_owned()
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
            Command::Language(_)
            | Command::PreviewWidth(_)
            | Command::ReducedMotion(_)
            | Command::Brand(_)
            | Command::Density(_)
            | Command::TextScale(_) => None,
            Command::Contrast(c) => char::from_digit(4 + index_of(&ContrastChoice::ALL, c), 10)
                .map(|d| KeyEquivalent::new(d, ctrl_cmd)),
        }
    }
}

fn index_of<T: PartialEq + Copy>(all: &[T], v: T) -> u32 {
    all.iter().position(|x| *x == v).unwrap_or(0) as u32
}

/// 菜单栏描述。`checked` 给出每个命令的初始勾选状态。
pub fn menu_bar(locale: Locale, checked: impl Fn(Command) -> bool) -> MenuBar {
    let item = |c: Command| {
        let mut m = MenuCommand::new(c.title(locale), c.id()).checked(checked(c));
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
            Menu::new(locale.text("Pages"), pages),
            Menu::new(locale.text("Appearance"), appearance),
            Menu::new(
                "Preview",
                preview_commands().into_iter().map(item).collect(),
            ),
            Menu::new(
                locale.text("Language"),
                Locale::ALL
                    .into_iter()
                    .map(|l| item(Command::Language(l)))
                    .collect(),
            ),
        ],
    }
}

fn preview_commands() -> Vec<Command> {
    let mut commands = vec![
        Command::PreviewWidth(0),
        Command::PreviewWidth(1),
        Command::PreviewWidth(2),
        Command::ReducedMotion(false),
        Command::ReducedMotion(true),
    ];
    commands.extend(
        gallery_brand_presets()
            .iter()
            .map(|preset| Command::Brand(preset.palette_index)),
    );
    commands.extend([
        Command::Density(0),
        Command::Density(1),
        Command::TextScale(0),
        Command::TextScale(1),
        Command::TextScale(2),
    ]);
    commands
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
        assert_eq!(Command::from_id(CommandId(9999)), None);
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
        assert_eq!(Command::Brand(0).id(), CommandId(520));
        assert_eq!(Command::Brand(2).id(), CommandId(522));
        assert_eq!(Command::Brand(3).id(), CommandId(560));
    }

    #[test]
    fn menu_bar_is_valid_and_covers_all_commands() {
        let bar = menu_bar(Locale::En, |_| false);
        assert_eq!(bar.validate(), Ok(()));
        let ids: Vec<_> = bar.commands().map(|c| c.command).collect();
        let want: Vec<_> = Command::all().iter().map(|c| c.id()).collect();
        assert_eq!(ids, want);
        assert_eq!(
            tekes_ui_tokens::palette::gallery_brand_presets()
                .iter()
                .map(|preset| Command::Brand(preset.palette_index))
                .collect::<Vec<_>>(),
            Command::all()
                .into_iter()
                .filter(|command| matches!(command, Command::Brand(_)))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn page_shortcuts_are_cmd_digits_in_order() {
        let bar = menu_bar(Locale::En, |_| false);
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
        let target = super::super::navigation::pages()[0];
        let bar = menu_bar(Locale::En, |c| c == Command::Page(target));
        let checked: Vec<_> = bar
            .commands()
            .filter(|c| c.checked)
            .map(|c| c.command)
            .collect();
        assert_eq!(checked, [Command::Page(target).id()]);
    }
}
