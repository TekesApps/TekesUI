//! Public ToolbarHost coverage for the original ToolbarControl/MenuView bridge.
use std::time::Duration;
use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, ControlKey, ControlMotionContext, composition::*,
};
use tekes_ui_scene::{Point, Rect, Scene, Size};
use tekes_ui_tokens::{Rgba, Theme, proposal};

const ROOT: ComponentId = ComponentId(27200);
const REGION: ComponentId = ComponentId(27201);
const GROUP: ComponentId = ComponentId(27202);
const CONTROL: ComponentId = ComponentId(27203);

#[derive(Default)]
struct Text {
    millis: u64,
    reduced: bool,
    requested: bool,
}
impl ButtonTextRenderer for Text {
    fn motion_context(&self) -> Option<ControlMotionContext> {
        Some(ControlMotionContext {
            now: Duration::from_millis(self.millis),
            reduced_motion: self.reduced,
        })
    }
    fn request_animation_frame(&mut self) {
        self.requested = true;
    }
    fn measure(&mut self, s: &str, role: proposal::TextRole, width: f32) -> Size {
        Size::new((s.len() as f32 * 8.).min(width), role.style().line_height)
    }
    fn draw(
        &mut self,
        _: &mut Scene,
        _: &str,
        _: proposal::TextRole,
        _: Rect,
        _: ButtonTextAlign,
        _: Rgba,
    ) {
    }
}

fn host(count: u64) -> ToolbarHost<&'static str> {
    let viewport = Rect::new(0., 0., 400., 220.);
    let mut host = ToolbarHost::new(ROOT, 272, viewport).unwrap();
    host.set_regions(
        272,
        viewport,
        vec![ToolbarRegion::new(REGION, Rect::new(0., 0., 400., 44.))],
    )
    .unwrap();
    host.register(ToolbarRegistration {
        region: REGION,
        order: 0,
        group: Content::new(
            GROUP,
            ToolbarGroup::new(
                ToolbarPlacement::SecondaryAction,
                vec![toolbar_commands(
                    CONTROL,
                    (0..count)
                        .map(|id| ToolbarButtonDeclaration {
                            id,
                            icon: None,
                            text: format!("Action {id}"),
                            hint: None,
                            enabled: id != 1,
                        })
                        .collect(),
                    ToolbarGroupOverflow::Menu,
                )],
            ),
        ),
        bindings: (0..count)
            .map(|id| ToolbarCommandBinding {
                target: CONTROL,
                action: Some(id),
                command: id,
                source: "original-module",
            })
            .collect(),
    })
    .unwrap();
    host
}
fn paint(host: &mut ToolbarHost<&str>, text: &mut Text) -> Scene {
    text.requested = false;
    let mut overlay = Scene::new();
    host.render_layers(&mut Scene::new(), &mut overlay, text, Theme::LIGHT.into())
        .unwrap();
    overlay
}
fn rows(host: &ToolbarHost<&str>) -> Vec<(ToolbarAccessibilityTarget, SemanticPart)> {
    host.semantics()
        .into_iter()
        .flat_map(|node| node.parts)
        .filter(|(_, part)| part.semantics.role == ContentRole::MenuItem)
        .collect()
}
fn open(host: &mut ToolbarHost<&str>, text: &mut Text) {
    paint(host, text);
    let trigger = host
        .semantics()
        .into_iter()
        .flat_map(|node| node.parts)
        .find(|(_, part)| part.semantics.name == "More options")
        .unwrap()
        .0;
    host.accessibility_action(trigger, AccessibilityAction::Press);
    paint(host, text);
    text.millis += 1000;
    assert!(!paint(host, text).primitives().is_empty());
    assert!(!rows(host).is_empty());
}
fn wheel(host: &mut ToolbarHost<&str>, position: Point, begin: bool, end: bool, time: f64) {
    host.event(&ContentEvent::Scroll {
        position,
        delta: Point::new(0., 35.),
        precise: true,
        begin,
        end,
        momentum: false,
        time,
    });
}
fn geometry(host: &ToolbarHost<&str>) -> Vec<(String, Rect)> {
    rows(host)
        .into_iter()
        .map(|(_, p)| (p.semantics.name, p.frame))
        .collect()
}

#[test]
fn long_menu_wheel_uses_original_rows_and_precise_gesture_owner() {
    let mut host = host(30);
    let mut text = Text::default();
    open(&mut host, &mut text);
    let before = geometry(&host);
    let outside = Point::new(5., 215.);
    wheel(&mut host, outside, true, true, 1.);
    paint(&mut host, &mut text);
    assert_eq!(
        geometry(&host),
        before,
        "outside wheel must not move the menu"
    );
    let inside = rows(&host)[2].1.frame.center();
    wheel(&mut host, inside, true, false, 2.);
    paint(&mut host, &mut text);
    let scrolled = geometry(&host);
    assert_ne!(
        scrolled, before,
        "popup wheel must change original row geometry"
    );
    wheel(&mut host, outside, false, false, 2.02);
    paint(&mut host, &mut text);
    assert_ne!(
        geometry(&host),
        scrolled,
        "precise continuation retains original menu owner"
    );
    wheel(&mut host, outside, false, true, 2.04);
    paint(&mut host, &mut text);
    let ended = geometry(&host);
    wheel(&mut host, outside, false, true, 2.06);
    paint(&mut host, &mut text);
    assert_eq!(geometry(&host), ended, "end releases the captured gesture");
    host.event(&ContentEvent::Key {
        key: ControlKey::End,
        down: true,
    });
    paint(&mut host, &mut text);
    assert!(
        rows(&host)
            .iter()
            .any(|(_, p)| p.semantics.name == "Action 29"),
        "keyboard reveal must use the same scrolled menu"
    );
    assert!(host.drain_commands().is_empty());
}

#[test]
fn close_revokes_input_and_semantics_before_clocked_exit_finishes() {
    for close in ["activate", "escape", "outside"] {
        let mut host = host(3);
        let mut text = Text::default();
        open(&mut host, &mut text);
        let original_rows = rows(&host);
        let stale = original_rows[0].0;
        let disabled = original_rows[1].0;
        host.accessibility_action(disabled, AccessibilityAction::Press);
        assert!(host.drain_commands().is_empty());
        assert_eq!(rows(&host).len(), 3);
        match close {
            "activate" => {
                host.accessibility_action(stale, AccessibilityAction::Press);
            }
            "escape" => {
                host.event(&ContentEvent::Key {
                    key: ControlKey::Escape,
                    down: true,
                });
            }
            _ => {
                host.event(&ContentEvent::PointerDown(Point::new(5., 215.)));
            }
        }
        assert!(
            rows(&host).is_empty(),
            "closed menu semantics revoke immediately"
        );
        let commands = host.drain_commands();
        assert_eq!(commands.len(), usize::from(close == "activate"));
        if let Some(command) = commands.first() {
            assert_eq!(
                (
                    command.window,
                    command.group,
                    command.command,
                    command.source
                ),
                (272, GROUP, 0, "original-module")
            );
        }
        assert!(
            !host
                .accessibility_action(stale, AccessibilityAction::Press)
                .activated
        );
        assert!(host.drain_commands().is_empty());
        let exiting = paint(&mut host, &mut text);
        assert!(
            !exiting.primitives().is_empty(),
            "{close}: closed menu retains paint-only exit"
        );
        assert!(text.requested, "{close}: exit requests another host frame");
        text.millis += 1000;
        assert!(
            paint(&mut host, &mut text).primitives().is_empty(),
            "exit releases fragment"
        );
        assert!(!text.requested);
    }
}

#[test]
fn reduced_motion_and_unmount_leave_no_exit_or_stale_action() {
    for remove in [false, true] {
        let mut host = host(3);
        let mut text = Text {
            reduced: !remove,
            ..Default::default()
        };
        open(&mut host, &mut text);
        let stale = rows(&host)[0].0;
        if remove {
            host.remove(GROUP).unwrap();
        } else {
            host.event(&ContentEvent::Key {
                key: ControlKey::Escape,
                down: true,
            });
        }
        assert!(paint(&mut host, &mut text).primitives().is_empty());
        assert!(!text.requested);
        assert!(rows(&host).is_empty());
        assert!(
            !host
                .accessibility_action(stale, AccessibilityAction::Press)
                .activated
        );
        assert!(host.drain_commands().is_empty());
    }
}
