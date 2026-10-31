//! Workbench transport for the original SelectContent/ContentHost owners.
use super::*;
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    sync::atomic::{AtomicU64, Ordering},
};
use tekes_ui_controls::layout_view::SelectLayout;
use tekes_ui_controls::{Select, composition::*};
use tekes_ui_platform_macos::{
    AccessibilityAction as A, AccessibilityKey as K, AccessibilityNode as N,
};
const SELECT: ComponentId = ComponentId(8901);
// Disjoint from Gallery fixed keys and Search's existing high-bit projection.
static NEXT_NATIVE: AtomicU64 = AtomicU64::new(1_u64 << 62);
#[derive(Debug)]
pub(super) struct SelectStoryHost {
    pub(super) host: ContentHost,
    projection: RefCell<HashMap<K, K>>,
}
impl Default for SelectStoryHost {
    fn default() -> Self {
        Self::new()
    }
}
impl SelectStoryHost {
    pub(super) fn new() -> Self {
        let mut host = ContentHost::new(Content::new(
            SELECT,
            SelectContent::new(
                "Selection",
                vec!["Automatic".into(), "Compact".into(), "Comfortable".into()],
            ),
        ))
        .expect("fixed Select story");
        host.set_action_events(true);
        host.set_focus_wrap(false);
        Self {
            host,
            projection: RefCell::new(HashMap::new()),
        }
    }
    fn owner(&self) -> &SelectContent {
        self.host
            .get(SELECT)
            .unwrap()
            .component::<SelectContent>()
            .unwrap()
    }
    fn owner_mut(&mut self) -> &mut SelectContent {
        self.host
            .get_mut(SELECT)
            .unwrap()
            .component_mut::<SelectContent>()
            .unwrap()
    }
    pub(super) fn model(&self) -> &Select {
        self.owner().model()
    }
    pub(super) fn layout(&self) -> &SelectLayout {
        self.owner().layout()
    }
    pub(super) fn set_labels(&mut self, labels: Vec<String>) -> Result<bool, SelectUpdateError> {
        self.owner_mut().set_labels(labels)
    }
    #[cfg(test)]
    pub(super) fn set_selected(&mut self, index: usize) -> bool {
        self.owner_mut().set_selected(index)
    }
    pub(super) fn set_disabled(&mut self, disabled: bool) {
        self.owner_mut().set_disabled(disabled)
    }
    pub(super) fn set_error(&mut self, error: bool) {
        self.owner_mut().set_error(error)
    }
    pub(super) fn focus(&mut self, visible: bool) {
        self.host.focus(Some(SELECT), visible);
    }
    pub(super) fn blur(&mut self) {
        self.host.focus(None, false);
    }
    pub(super) fn cancel(&mut self) {
        self.host.cancel();
    }
    pub(super) fn set_window_active(&mut self, active: bool) {
        self.host.set_window_active(active);
        if !active {
            self.projection.borrow_mut().clear();
        }
    }
    // Original TextEngine supplies #79's clock/motion context; no second clock.
    pub(super) fn set_reduced_motion(&mut self, _: bool) -> bool {
        false
    }
    pub(super) fn needs_animation(&self) -> bool {
        self.host.animation_frame_requested()
    }
    pub(super) fn tick(&mut self, _: f32) -> bool {
        self.needs_animation()
    }
    pub(super) fn paint(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        frame: Rect,
        clip: Rect,
        palette: tekes_ui_tokens::Palette,
    ) -> SelectLayout {
        self.host
            .render(scene, text, frame, clip, palette)
            .expect("fixed Select story layout");
        self.layout().clone()
    }
    pub(super) fn pointer_move(&mut self, p: Point) -> bool {
        self.host.event(&ContentEvent::PointerMove(p)).consumed
    }
    pub(super) fn pointer_down(&mut self, p: Point) -> bool {
        self.host.event(&ContentEvent::PointerDown(p)).consumed
    }
    pub(super) fn pointer_up(&mut self, p: Point) -> bool {
        self.host.event(&ContentEvent::PointerUp(p)).consumed
    }
    pub(super) fn key(&mut self, key: ControlKey, down: bool) -> MenuEvent {
        let _ = self.host.event(&ContentEvent::Key { key, down });
        MenuEvent::None
    }
    pub(super) fn wheel_event(&mut self, input: MenuWheelInput) -> MenuWheelEvent {
        let response = self.host.event(&ContentEvent::Scroll {
            position: input.point,
            delta: input.delta,
            precise: input.precise,
            begin: input.begin,
            end: input.end,
            momentum: input.momentum,
            time: input.time,
        });
        if response.consumed {
            MenuWheelEvent::Consumed {
                scrolled: response.changed,
            }
        } else {
            MenuWheelEvent::Ignored
        }
    }
    pub(super) fn open(&mut self) {
        self.host
            .accessibility_action(SELECT, None, AccessibilityAction::Press);
    }

    pub(super) fn handle(&mut self, event: &Event) -> Option<ContentResponse> {
        let focused = self.host.focused() == Some(SELECT);
        let open = self.owner().overlay_open();
        let input = match event {
            Event::Mouse(m) => {
                let p = Point::new(m.position.x as f32, m.position.y as f32);
                let inside = self.layout().trigger.visible.is_some_and(|r| r.contains(p));
                if !inside
                    && !open
                    && self.host.pointer_capture().is_none()
                    && !matches!(m.kind, MouseEventKind::Moved | MouseEventKind::Exited)
                {
                    return None;
                }
                crate::reading_pointer::translate(*m)
            }
            Event::Key(k) if focused || open => {
                ContentEvent::KeyInput(crate::reading_key::key(k.clone()))
            }
            Event::Scroll(e) if open => ContentEvent::Scroll {
                position: Point::new(e.position.x as f32, e.position.y as f32),
                delta: Point::new(e.delta.x as f32, e.delta.y as f32),
                precise: e.precise,
                begin: matches!(e.phase, tekes_ui_platform_macos::ScrollPhase::Began),
                end: matches!(
                    e.phase,
                    tekes_ui_platform_macos::ScrollPhase::Ended
                        | tekes_ui_platform_macos::ScrollPhase::Cancelled
                ),
                momentum: e.momentum_phase != tekes_ui_platform_macos::ScrollPhase::None,
                time: e.timestamp,
            },
            _ => return None,
        };
        Some(self.host.event(&input))
    }
    pub(super) fn drain_logs(&mut self) -> Vec<String> {
        self.host
            .drain_action_records()
            .into_iter()
            .filter_map(|r| match r.payload {
                Some(ContentActionPayload::MenuSelection(s)) => {
                    Some(format!("select: {}", s.index()))
                }
                _ => None,
            })
            .collect()
    }
    pub(super) fn native_nodes(&self) -> Vec<N> {
        let mut nodes = tekes_ui_platform_macos::content_accessibility::content_nodes(&self.host);
        let current: HashSet<_> = nodes.iter().map(|n| n.key).collect();
        let mut keys = self.projection.borrow_mut();
        keys.retain(|key, _| current.contains(key));
        for n in &nodes {
            if !keys.contains_key(&n.key) {
                let Ok(component) =
                    NEXT_NATIVE.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| {
                        n.checked_add(1).filter(|n| *n < (1_u64 << 63))
                    })
                else {
                    keys.clear();
                    return vec![];
                };
                keys.insert(
                    n.key,
                    K {
                        component,
                        part: n.key.part,
                    },
                );
            }
        }
        for n in &mut nodes {
            n.parent = n.parent.and_then(|key| keys.get(&key).copied());
            n.key = keys[&n.key];
        }
        nodes
    }
    pub(super) fn owns_native_key(&self, key: K) -> bool {
        self.native_nodes().iter().any(|n| n.key == key)
    }
    pub(super) fn native_action(&mut self, key: K, action: A) -> ContentResponse {
        self.native_nodes();
        let original = self
            .projection
            .borrow()
            .iter()
            .find_map(|(a, b)| (*b == key).then_some(*a));
        original
            .map(|key| {
                tekes_ui_platform_macos::content_accessibility::content_action(
                    &mut self.host,
                    key,
                    action,
                )
            })
            .unwrap_or_default()
    }
}
