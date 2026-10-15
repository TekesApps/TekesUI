//! 把 `MenuBar` 描述翻译成 NSMenu，并附带标准的应用菜单与窗口菜单。
//!
//! 标准项直接使用 AppKit 的标准 selector，target 为 nil，沿响应链找到处理者
//! （`terminate:` 到 NSApplication，`performClose:` 到窗口……），不需要引擎参与。
//! 引擎的命令项统一指向委托的 `menuItemSelected:`，用 `tag` 携带 `CommandId`。

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, Sel};
use objc2::{MainThreadOnly, sel};
use objc2_app_kit::{
    NSApplication, NSControlStateValueOff, NSControlStateValueOn, NSEventModifierFlags, NSMenu,
    NSMenuItem,
};
use objc2_foundation::{MainThreadMarker, NSString};

use crate::event::Modifiers;
use crate::menu::{CommandId, MenuBar, MenuCommand, MenuItem};

/// 引擎命令项的 action。
pub(crate) fn command_action() -> Sel {
    sel!(menuItemSelected:)
}

fn ns_modifiers(m: Modifiers) -> NSEventModifierFlags {
    NSEventModifierFlags::from_bits_retain(m.to_ns_flags())
}

fn new_menu(mtm: MainThreadMarker, title: &str) -> Retained<NSMenu> {
    NSMenu::initWithTitle(NSMenu::alloc(mtm), &NSString::from_str(title))
}

fn new_item(
    mtm: MainThreadMarker,
    title: &str,
    action: Option<Sel>,
    key: &str,
    modifiers: Modifiers,
) -> Retained<NSMenuItem> {
    // SAFETY: action 要么为 None，要么是下面各调用点传入的合法 selector，
    // 参数（单个 sender）与 AppKit 约定一致。
    let item = unsafe {
        NSMenuItem::initWithTitle_action_keyEquivalent(
            NSMenuItem::alloc(mtm),
            &NSString::from_str(title),
            action,
            &NSString::from_str(key),
        )
    };
    if !key.is_empty() {
        item.setKeyEquivalentModifierMask(ns_modifiers(modifiers));
    }
    item
}

/// 顶层菜单项：挂着一个子菜单。
fn top_level(mtm: MainThreadMarker, submenu: &NSMenu) -> Retained<NSMenuItem> {
    let item = new_item(mtm, "", None, "", Modifiers::NONE);
    item.setSubmenu(Some(submenu));
    item
}

fn command_item(
    mtm: MainThreadMarker,
    target: &AnyObject,
    c: &MenuCommand,
) -> Retained<NSMenuItem> {
    let (key, mods) = match c.key_equivalent {
        Some(k) => (k.ns_key(), k.modifiers),
        None => (String::new(), Modifiers::NONE),
    };
    let item = new_item(mtm, &c.title, Some(command_action()), &key, mods);
    // SAFETY: target 是委托对象，它在应用生命周期内存活，menu item 对 target 是弱引用。
    unsafe { item.setTarget(Some(target)) };
    item.setTag(c.command.0 as isize);
    item.setState(if c.checked {
        NSControlStateValueOn
    } else {
        NSControlStateValueOff
    });
    item
}

fn app_menu(mtm: MainThreadMarker, app_name: &str) -> Retained<NSMenu> {
    let menu = new_menu(mtm, app_name);
    let cmd = Modifiers::COMMAND;
    let cmd_opt = Modifiers {
        alt: true,
        ..Modifiers::COMMAND
    };
    menu.addItem(&new_item(
        mtm,
        &format!("About {app_name}"),
        Some(sel!(orderFrontStandardAboutPanel:)),
        "",
        Modifiers::NONE,
    ));
    menu.addItem(&NSMenuItem::separatorItem(mtm));
    menu.addItem(&new_item(
        mtm,
        &format!("Hide {app_name}"),
        Some(sel!(hide:)),
        "h",
        cmd,
    ));
    menu.addItem(&new_item(
        mtm,
        "Hide Others",
        Some(sel!(hideOtherApplications:)),
        "h",
        cmd_opt,
    ));
    menu.addItem(&new_item(
        mtm,
        "Show All",
        Some(sel!(unhideAllApplications:)),
        "",
        Modifiers::NONE,
    ));
    menu.addItem(&NSMenuItem::separatorItem(mtm));
    menu.addItem(&new_item(
        mtm,
        &format!("Quit {app_name}"),
        Some(sel!(terminate:)),
        "q",
        cmd,
    ));
    menu
}

fn window_menu(mtm: MainThreadMarker) -> Retained<NSMenu> {
    let menu = new_menu(mtm, "Window");
    let cmd = Modifiers::COMMAND;
    menu.addItem(&new_item(
        mtm,
        "Minimize",
        Some(sel!(performMiniaturize:)),
        "m",
        cmd,
    ));
    menu.addItem(&new_item(
        mtm,
        "Zoom",
        Some(sel!(performZoom:)),
        "",
        Modifiers::NONE,
    ));
    menu.addItem(&NSMenuItem::separatorItem(mtm));
    menu.addItem(&new_item(mtm, "Close", Some(sel!(performClose:)), "w", cmd));
    menu.addItem(&NSMenuItem::separatorItem(mtm));
    menu.addItem(&new_item(
        mtm,
        "Enter Full Screen",
        Some(sel!(toggleFullScreen:)),
        "f",
        Modifiers {
            control: true,
            ..Modifiers::COMMAND
        },
    ));
    menu
}

/// 构建整条菜单栏并安装到 NSApplication。
pub(crate) fn install(mtm: MainThreadMarker, app_name: &str, bar: &MenuBar, target: &AnyObject) {
    let app = NSApplication::sharedApplication(mtm);
    let main = new_menu(mtm, "MainMenu");

    main.addItem(&top_level(mtm, &app_menu(mtm, app_name)));

    for m in &bar.menus {
        let submenu = new_menu(mtm, &m.title);
        for item in &m.items {
            match item {
                MenuItem::Command(c) => submenu.addItem(&command_item(mtm, target, c)),
                MenuItem::Separator => submenu.addItem(&NSMenuItem::separatorItem(mtm)),
            }
        }
        let top = top_level(mtm, &submenu);
        top.setTitle(&NSString::from_str(&m.title));
        main.addItem(&top);
    }

    let window = window_menu(mtm);
    let window_item = top_level(mtm, &window);
    window_item.setTitle(&NSString::from_str("Window"));
    main.addItem(&window_item);

    app.setMainMenu(Some(&main));
    // 让 AppKit 自动往窗口菜单末尾追加打开的窗口列表。
    app.setWindowsMenu(Some(&window));
}

fn find_command(menu: &NSMenu, id: isize) -> Option<Retained<NSMenuItem>> {
    for item in menu.itemArray().iter() {
        if let Some(sub) = item.submenu()
            && let Some(found) = find_command(&sub, id)
        {
            return Some(found);
        }
        if item.action() == Some(command_action()) && item.tag() == id {
            return Some(item);
        }
    }
    None
}

/// 修改命令项的勾选状态，不存在时返回 false。
pub(crate) fn set_command_checked(
    mtm: MainThreadMarker,
    command: CommandId,
    checked: bool,
) -> bool {
    let Some(main) = NSApplication::sharedApplication(mtm).mainMenu() else {
        return false;
    };
    match find_command(&main, command.0 as isize) {
        Some(item) => {
            item.setState(if checked {
                NSControlStateValueOn
            } else {
                NSControlStateValueOff
            });
            true
        }
        None => false,
    }
}

/// 读取命令项的勾选状态，不存在时返回 None（测试用）。
pub(crate) fn is_command_checked(mtm: MainThreadMarker, command: CommandId) -> Option<bool> {
    let main = NSApplication::sharedApplication(mtm).mainMenu()?;
    find_command(&main, command.0 as isize).map(|i| i.state() == NSControlStateValueOn)
}
