//! #123: original MenuContent/MenuView with real CoreText, retained across Theme changes.
#![cfg(target_os = "macos")]
#[path = "support/coretext.rs"]
mod coretext;
use tekes_ui_controls::{ButtonTextAlign, ButtonTextRenderer, ControlKey, composition::*};
use tekes_ui_scene::{Primitive, Rect, Scene, Size};
use tekes_ui_tokens::{Palette, Rgba, Theme, proposal};
const MENU: ComponentId = ComponentId(123_003);
struct Text {
    inner: coretext::Text,
    draws: Vec<(String, Rect, Rgba)>,
}
impl ButtonTextRenderer for Text {
    fn measure(&mut self, s: &str, r: proposal::TextRole, w: f32) -> Size {
        self.inner.measure(s, r, w)
    }
    fn motion_context(&self) -> Option<tekes_ui_controls::ControlMotionContext> {
        self.inner.motion_context()
    }
    fn backing_scale(&self) -> f32 {
        self.inner.backing_scale()
    }
    fn text_surface_host(&mut self) -> Option<&mut dyn tekes_ui_controls::TextSurfaceHost> {
        Some(&mut self.inner)
    }
    fn draw(
        &mut self,
        scene: &mut Scene,
        s: &str,
        role: proposal::TextRole,
        frame: Rect,
        align: ButtonTextAlign,
        ink: Rgba,
    ) {
        self.draws.push((s.into(), frame, ink));
        self.inner.draw(scene, s, role, frame, align, ink);
    }
}
fn menu(h: &mut ContentHost) -> &mut MenuContent {
    h.get_mut(MENU)
        .unwrap()
        .component_mut::<MenuContent>()
        .unwrap()
}
fn render(h: &mut ContentHost, text: &mut Text, palette: Palette) -> Scene {
    text.draws.clear();
    let mut scene = Scene::new();
    h.render(
        &mut scene,
        text,
        Rect::new(20., 20., 240., 36.),
        Rect::new(0., 0., 640., 520.),
        palette,
    )
    .unwrap();
    scene
}
fn contrast(a: Rgba, b: Rgba) -> f32 {
    let luminance = |c: Rgba| {
        let v = c.to_linear();
        0.2126 * v[0] + 0.7152 * v[1] + 0.0722 * v[2]
    };
    let (a, b) = (luminance(a), luminance(b));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}
#[test]
fn menu_retained_theme_changes_keep_real_title_items_checks_and_navigation_readable() {
    let mut content = MenuContent::new(
        "Options",
        ["Open", "Checked", "Disabled"].map(String::from).to_vec(),
    );
    content.set_item_checked(1, true);
    content.set_item_disabled(2, true);
    let mut host = ContentHost::new(Content::new(MENU, content)).unwrap();
    host.set_action_events(true);
    let mut text = Text {
        inner: coretext::Text::new(1.),
        draws: Vec::new(),
    };
    text.inner.services = true;
    text.inner.reduced_motion = true;
    render(&mut host, &mut text, Theme::LIGHT.into());
    assert!(
        host.accessibility_action(MENU, None, AccessibilityAction::Press)
            .consumed
    );
    let mut surfaces = Vec::new();
    let mut expected_active = 0;
    // Shared Button/Palette brand acceptance is reused; this menu stays mounted and open.
    for theme in Theme::ALL {
        let palette = Palette::new(theme, 0).unwrap();
        let initial = render(&mut host, &mut text, palette);
        let popup = menu(&mut host).popup.clone().unwrap();
        assert_eq!(
            popup.active_item, expected_active,
            "Theme change retains active item"
        );
        let surface = initial
            .primitives()
            .iter()
            .find_map(|p| match p {
                Primitive::Quad(q) if q.rect == popup.popup => Some(q.fill),
                _ => None,
            })
            .unwrap();
        assert_eq!(surface, palette.color("surface-popover").unwrap());
        surfaces.push(surface);
        // Clear stale hover using the original MenuView input domain, inside its popup gutter.
        menu(&mut host).menu.pointer_move_at(popup.popup.origin);
        assert!(
            host.event(&ContentEvent::Key {
                key: ControlKey::Home,
                down: true
            })
            .consumed
        );
        for state in ["keyboard-focus", "hover", "pressed"] {
            if state == "hover" {
                host.event(&ContentEvent::PointerMove(popup.items[1].frame.center()));
            } else if state == "pressed" {
                let response =
                    host.event(&ContentEvent::PointerDown(popup.items[1].frame.center()));
                assert!(response.consumed && !response.activated);
            }
            let scene = render(&mut host, &mut text, palette);
            let layout = menu(&mut host).popup.clone().unwrap();
            assert_eq!(
                layout.active_item,
                if state == "keyboard-focus" { 0 } else { 1 }
            );
            for (index, label) in ["Open", "Checked", "Disabled"].iter().enumerate() {
                let row = layout.items[index].frame;
                let fill = scene
                    .primitives()
                    .iter()
                    .find_map(|p| match p {
                        Primitive::Quad(q) if q.rect == row => Some(q.fill),
                        _ => None,
                    })
                    .unwrap();
                let role = if index == layout.active_item {
                    if state == "keyboard-focus" {
                        "row-selection"
                    } else {
                        "row-selection-hover"
                    }
                } else {
                    "surface-2"
                };
                assert_eq!(
                    fill,
                    palette.color(role).unwrap(),
                    "{theme:?} {state} {label}"
                );
                let (_, frame, ink) = text
                    .draws
                    .iter()
                    .find(|(s, _, _)| s.as_str() == *label)
                    .expect("real menu label draw");
                assert_eq!(
                    *ink,
                    palette
                        .color(if index == 2 {
                            "tertiary-text"
                        } else {
                            "primary-text"
                        })
                        .unwrap()
                );
                assert!(scene.primitives().iter().any(|p| matches!(p, Primitive::Glyphs(run) if run.color == *ink && run.glyphs.iter().any(|g| frame.contains(g.origin)))), "actual CoreText item glyphs");
                assert!(
                    contrast(*ink, fill) >= if theme.is_increased() { 7. } else { 4.5 },
                    "{theme:?} {state} {label}: {}",
                    contrast(*ink, fill)
                );
            }
            let title = text.draws.iter().find(|(s, _, _)| s == "Options").unwrap();
            let trigger = menu(&mut host).trigger.button.layout().unwrap().frame;
            let title_fill = scene
                .primitives()
                .iter()
                .find_map(|p| match p {
                    Primitive::Quad(q) if q.rect == trigger => Some(q.fill),
                    _ => None,
                })
                .unwrap();
            assert_eq!(title.2, palette.color("primary-text").unwrap());
            assert!(scene.primitives().iter().any(|p| matches!(p, Primitive::Glyphs(run) if run.color == title.2 && run.glyphs.iter().any(|g| title.1.contains(g.origin)))), "actual CoreText title glyphs");
            assert!(contrast(title.2, title_fill) >= if theme.is_increased() { 7. } else { 4.5 });
            let focus = scene.primitives().iter().any(|p| matches!(p, Primitive::Quad(q) if q.border_color == palette.color("border-focused").unwrap() && q.rect.center() == layout.items[layout.active_item].frame.center()));
            assert_eq!(focus, state == "keyboard-focus");
            let semantics = host.semantics();
            assert!(
                semantics[0]
                    .parts
                    .iter()
                    .find(|p| p.index == 1)
                    .unwrap()
                    .semantics
                    .selected
            );
            assert!(
                semantics[0]
                    .parts
                    .iter()
                    .find(|p| p.index == 2)
                    .unwrap()
                    .semantics
                    .disabled
            );
            assert!(menu(&mut host).menu.is_open());
        }
        expected_active = 1;
        // Release the held pointer in the popup gutter: cancel the arm without closing/remounting.
        host.event(&ContentEvent::PointerMove(popup.popup.origin));
        assert!(
            !host
                .event(&ContentEvent::PointerUp(popup.popup.origin))
                .activated
        );
        assert!(menu(&mut host).menu.is_open());
        assert!(host.drain_action_records().is_empty());
        assert!(
            !host
                .accessibility_action(MENU, Some(2), AccessibilityAction::Press)
                .activated
        );
        assert!(menu(&mut host).menu.is_open());
    }
    assert!(surfaces.windows(2).any(|w| w[0] != w[1]));
    assert!(
        host.accessibility_action(MENU, Some(1), AccessibilityAction::Press)
            .activated
    );
    assert_eq!(host.drain_action_records().len(), 1);
    assert_eq!(menu(&mut host).selected, Some(1));
}
