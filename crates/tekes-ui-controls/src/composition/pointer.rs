//! Pointer metadata at the existing component protocol boundary, independent of AppKit.
use super::{Component, ContentEvent, ContentResponse};
use std::borrow::Cow;
use tekes_ui_scene::Point;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointerButton {
    Primary,
    Secondary,
    Middle,
    Other(u16),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointerPhase {
    Down,
    Up,
    Move,
    Drag,
    Enter,
    Exit,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct InputModifiers {
    pub shift: bool,
    pub control: bool,
    pub alt: bool,
    pub command: bool,
    pub caps_lock: bool,
    pub function: bool,
}
/// Compatibility name for the shared keyboard/pointer modifier data.
pub type PointerModifiers = InputModifiers;
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointerEvent {
    pub position: Point,
    pub phase: PointerPhase,
    /// None for hover/enter/exit, Some for a button gesture.
    pub button: Option<PointerButton>,
    pub modifiers: PointerModifiers,
    /// Native down/up count. Zero when the platform event has no click count.
    pub click_count: u32,
}
impl PointerEvent {
    pub fn is_primary(&self) -> bool {
        match self.phase {
            PointerPhase::Move | PointerPhase::Enter | PointerPhase::Exit => {
                self.button.is_none() || self.button == Some(PointerButton::Primary)
            }
            _ => self.button == Some(PointerButton::Primary),
        }
    }
    /// Compatibility input for existing components. Host routing still retains
    /// the original metadata; consumers needing it override pointer_event.
    pub(crate) fn routing_event(&self) -> ContentEvent {
        match self.phase {
            PointerPhase::Down => ContentEvent::PointerDown(self.position),
            PointerPhase::Up => ContentEvent::PointerUp(self.position),
            PointerPhase::Exit => ContentEvent::PointerMove(Point::new(-1., -1.)),
            _ => ContentEvent::PointerMove(self.position),
        }
    }
}
impl ContentEvent {
    pub(super) fn routing_event(&self) -> Cow<'_, ContentEvent> {
        match self {
            Self::Pointer(p) => Cow::Owned(p.routing_event()),
            Self::KeyInput(k) if k.key.is_some() => Cow::Owned(Self::Key {
                key: k.key.unwrap(),
                down: k.down,
            }),
            _ => Cow::Borrowed(self),
        }
    }
    pub(super) fn is_primary_pointer(&self) -> bool {
        match self {
            Self::Pointer(p) => p.is_primary(),
            _ => true,
        }
    }
    pub(super) fn deliver(&self, component: &mut dyn Component) -> ContentResponse {
        match self {
            Self::Pointer(p) => component.pointer_event(p),
            Self::KeyInput(k) => component.key_event(k),
            _ => component.event(self),
        }
    }
    pub(super) fn deliver_overlay(&self, component: &mut dyn Component) -> ContentResponse {
        match self {
            Self::Pointer(p) => component.overlay_pointer_event(p),
            Self::KeyInput(k) => component.overlay_key_event(k),
            _ => component.overlay_event(self),
        }
    }
}
