//! Read-only evidence hook for #207; never mutates fixture or focus.
use super::*;
fn rect(r: Rect) -> serde_json::Value {
    serde_json::json!([r.min_x(), r.min_y(), r.max_x(), r.max_y()])
}
pub(crate) fn observe(gallery: &super::super::Gallery, event: &Event) {
    if std::env::var("TEKES_SELFTEST").as_deref() != Ok("settings-native207") {
        return;
    }
    let Page::Workbench(i) = gallery.page else {
        return;
    };
    if entry(i).renderer() != "settings" {
        return;
    }
    let Some(state) = gallery.workbench.states.get(&i) else {
        return;
    };
    let frame = gallery.workbench.focused_control_frame(i);
    let viewport = gallery.viewport.geometry().visible_viewport_rect();
    eprintln!(
        "SETTINGS207 {}",
        serde_json::json!({
            "event": format!("{event:?}"), "phase": format!("{:?}", state.phase),
            "text": state.input.query(), "focus": state.focus,
            "tab_stops": state.logical_tab_ids(),
            "focus_frame": frame.map(rect), "viewport": viewport.map(rect),
            "log": state.log, "attempts": state.attempts,
            "source": "original Gallery Host after event; local AppKit selectors/NSEvents; no physical input claim"
        })
    );
}
