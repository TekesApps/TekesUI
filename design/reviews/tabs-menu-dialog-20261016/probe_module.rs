//! Target-only helpers for driving the real Gallery workbench with native-shaped events.
//! This file is copied into a frozen source clone; it is never compiled into the product.

use super::super::Gallery;
use tekes_ui_platform_macos::{
    Event, KeyEvent, KeyEventKind, Modifiers, MouseButton, MouseEvent, MouseEventKind,
    Point as NativePoint,
};
use tekes_ui_scene::Point;

fn workbench_index(gallery: &Gallery) -> usize {
    match gallery.page() {
        super::super::Page::Workbench(index) => index,
        other => panic!("expected a workbench route, got {other:?}"),
    }
}

fn click(gallery: &mut Gallery, point: Point) {
    for kind in [
        MouseEventKind::Down {
            button: MouseButton::Left,
            click_count: 1,
        },
        MouseEventKind::Up {
            button: MouseButton::Left,
            click_count: 1,
        },
    ] {
        gallery.handle_input(&Event::Mouse(MouseEvent {
            kind,
            position: NativePoint {
                x: f64::from(point.x),
                y: f64::from(point.y),
            },
            modifiers: Modifiers::NONE,
        }));
    }
}

pub fn press(gallery: &mut Gallery, key_code: u16) {
    gallery.handle_input(&Event::Key(KeyEvent {
        kind: KeyEventKind::Down,
        key_code,
        characters: String::new(),
        characters_ignoring_modifiers: String::new(),
        modifiers: Modifiers::NONE,
        is_repeat: false,
    }));
}

#[cfg(feature = "baseline")]
fn button_center(gallery: &mut Gallery, id: usize) -> Point {
    let index = workbench_index(gallery);
    gallery
        .workbench
        .state(index)
        .buttons
        .get(&id)
        .and_then(|button| button.layout())
        .expect("the actual Gallery build must lay out the requested Button")
        .frame
        .center()
}

#[cfg(feature = "current")]
fn button_center(gallery: &mut Gallery, id: usize) -> Point {
    let index = workbench_index(gallery);
    gallery
        .workbench
        .state(index)
        .buttons
        .get(&id)
        .and_then(|button| button.layout())
        .expect("the actual Gallery build must lay out the requested Button")
        .frame
        .center()
}

#[cfg(feature = "baseline")]
fn tab_center(gallery: &mut Gallery, tab: usize) -> Point {
    button_center(gallery, 30 + tab)
}

#[cfg(feature = "current")]
fn tab_center(gallery: &mut Gallery, tab: usize) -> Point {
    let index = workbench_index(gallery);
    gallery
        .workbench
        .state(index)
        .tabs_layout
        .as_ref()
        .and_then(|layout| layout.items.get(tab))
        .expect("the actual TabsView render must expose this tab frame")
        .frame
        .center()
}

pub fn activate_tab(gallery: &mut Gallery, tab: usize) {
    let center = tab_center(gallery, tab);
    click(gallery, center);
}

pub fn activate_button(gallery: &mut Gallery, id: usize) {
    let center = button_center(gallery, id);
    click(gallery, center);
}

#[cfg(feature = "baseline")]
pub fn state(gallery: &mut Gallery, kind: &str) -> serde_json::Value {
    let index = workbench_index(gallery);
    let state = gallery.workbench.state(index);
    match kind {
        "tabs" => serde_json::json!({ "selected": state.tabs.selected }),
        "menu" => serde_json::json!({
            "open": state.menu.open,
            "selected": state.menu.items.selected,
        }),
        "select" => serde_json::json!({
            "open": state.select.menu.open,
            "selected": state.select.menu.items.selected,
        }),
        "dialog" | "danger" => serde_json::json!({
            "open": state.dialog.open,
            "focus": state.dialog.focus,
        }),
        _ => panic!("unknown probe kind: {kind}"),
    }
}

#[cfg(feature = "current")]
pub fn state(gallery: &mut Gallery, kind: &str) -> serde_json::Value {
    let index = workbench_index(gallery);
    let state = gallery.workbench.state(index);
    match kind {
        "tabs" => serde_json::json!({ "selected": state.tabs_view.selected() }),
        "menu" => serde_json::json!({
            "open": state.menu_view.is_open(),
            "selected": state.menu_view.menu().items.selected,
            "active_item": state.menu_layout.as_ref().map(|layout| layout.active_item),
        }),
        "select" => serde_json::json!({
            "open": state.select.menu.open,
            "selected": state.select.menu.items.selected,
            "active_item": state.select_menu_layout.as_ref().map(|layout| layout.active_item),
        }),
        "dialog" | "danger" => serde_json::json!({
            "open": state.dialog_view.dialog().open,
            "focus": state.dialog_view.dialog().focus,
            "modal": state.dialog_layout.as_ref().map(|layout| {
                [layout.modal.origin.x, layout.modal.origin.y, layout.modal.size.width, layout.modal.size.height]
            }),
        }),
        _ => panic!("unknown probe kind: {kind}"),
    }
}

