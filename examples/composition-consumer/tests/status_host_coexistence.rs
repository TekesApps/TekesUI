#![cfg(target_os = "macos")]
#[path = "support/coretext.rs"]
mod coretext;

use std::{cell::Cell, rc::Rc};
use tekes_ui_composition_consumer::{
    choice_status, combo_status,
    content_status::{self, Kind, Status},
    slider_content,
};
use tekes_ui_controls::{
    composition::*,
    extended::{Axis, Stack},
    layout_view::IconRenderer,
};
use tekes_ui_scene::{ImageId, Rect, Scene};
use tekes_ui_tokens::{Palette, Rgba, Spacing, Theme};

struct TestIcons;
impl IconRenderer for TestIcons {
    fn resource_identity(&self) -> Option<u64> {
        Some(1)
    }
    fn register_icon_bitmap(
        &mut self,
        _: &[u8],
        _: u32,
        _: u32,
    ) -> Result<IconImage, IconRenderError> {
        Ok(IconImage::new(ImageId(1), Rc::new(())))
    }
    fn prepare_icon(
        &mut self,
        _: &str,
        _: f32,
        _: f32,
        _: Rgba,
    ) -> Result<IconImage, IconRenderError> {
        Ok(IconImage::new(ImageId(2), Rc::new(())))
    }
    fn draw_icon(&mut self, _: &mut Scene, _: &str, _: Rect, _: Rgba) {}
}

fn render_host(host: &mut ContentHost, text: &mut coretext::Text) {
    let frame = Rect::new(0., 0., 740., 4000.);
    let mut icons = TestIcons;
    host.render(
        &mut Scene::new(),
        &mut WithIconRenderer::new(text, &mut icons),
        frame,
        frame,
        Palette::from(Theme::LIGHT),
    )
    .unwrap();
}

#[test]
fn input_status_consumers_keep_independent_drafts_in_one_host() {
    let mut text = coretext::Text::new(1.);
    text.services = true;
    for kind in ["combo-box", "select"] {
        let cases: &[(Status, bool)] = if kind == "combo-box" {
            &[
                (Status::Loading, false),
                (Status::Error, false),
                (Status::Ready, true),
            ]
        } else {
            &[(Status::Loading, false), (Status::Error, false)]
        };
        let children: Vec<Content> = cases
            .iter()
            .enumerate()
            .map(|(i, (status, validation))| {
                let base = 91400 + i as u64 * 10;
                let ids = [
                    ComponentId(base + 1),
                    ComponentId(base + 2),
                    ComponentId(base + 3),
                ];
                let labels = ["Automatic", "Compact", "Comfortable"]
                    .into_iter()
                    .map(str::to_owned)
                    .collect();
                let cell = Rc::new(Cell::new(*status));
                if kind == "combo-box" {
                    combo_status::fixture_content(
                        ids,
                        labels,
                        cell,
                        validation.then(|| "Choose a permitted layout.".into()),
                        Some("Com"),
                    )
                } else {
                    choice_status::fixture_content(ids, labels, cell)
                }
            })
            .collect();
        let mut host = ContentHost::new(Content::new(
            ComponentId(91000),
            StackContent {
                model: Stack {
                    axis: Axis::Vertical,
                    gap: Spacing::Regular.value(),
                },
                children,
                flexible_child: None,
            },
        ))
        .unwrap();
        host.set_window_active(true);
        render_host(&mut host, &mut text);
        let semantics = host.semantics();
        for i in 0..cases.len() as u64 {
            let id = ComponentId(91402 + i * 10);
            assert!(host.get(id).is_some());
            let node = semantics.iter().find(|n| n.id == id).unwrap();
            assert!(!node.semantics.disabled);
            if kind == "combo-box" {
                assert_eq!(host.text_surface(id).unwrap().text(), "Compact");
                assert_eq!(node.semantics.invalid, i == 2);
            }
        }
        if kind == "combo-box" {
            assert!(host.focus(Some(ComponentId(91402)), true));
            host.event(&ContentEvent::Commit {
                text: "edited".into(),
                replacement: None,
            });
            assert_ne!(
                host.text_surface(ComponentId(91402)).unwrap().text(),
                "Compact"
            );
            assert_eq!(
                host.text_surface(ComponentId(91412)).unwrap().text(),
                "Compact"
            );
        }
    }
}

#[test]
fn tabs_disclosure_and_slider_consumers_preserve_independent_state_in_shared_host() {
    let mut text = coretext::Text::new(1.);
    text.services = true;
    for kind in [Some(Kind::Tabs), Some(Kind::Disclosure), None] {
        let children = match kind {
            Some(nav_kind) => [
                Status::Loading,
                Status::Error,
                Status::Empty,
                Status::Loading,
                Status::Error,
            ]
            .into_iter()
            .enumerate()
            .map(|(i, status)| {
                let base = 102000 + i as u64 * 10;
                content_status::fixture_content(
                    nav_kind,
                    std::array::from_fn(|n| ComponentId(base + 1 + n as u64)),
                    Rc::new(Cell::new(status)),
                    Rc::new(Cell::new(true)),
                    Rc::new(Cell::new(0)),
                    nav_kind == Kind::Disclosure,
                    i >= 3,
                )
            })
            .collect(),
            None => vec![slider_content::fixture_content(60.)],
        };
        let mut host = ContentHost::new(Content::new(
            ComponentId(100000),
            StackContent {
                model: Stack {
                    axis: Axis::Vertical,
                    gap: Spacing::Regular.value(),
                },
                children,
                flexible_child: None,
            },
        ))
        .unwrap();
        host.set_window_active(true);
        render_host(&mut host, &mut text);
        if let Some(nav_kind) = kind {
            for i in 0..5 {
                let base = 102000 + i * 10;
                assert!(host.get(ComponentId(base + 4)).is_some());
                assert!(host.get(ComponentId(base + 6)).is_some());
                assert!(
                    host.accessibility_action(
                        ComponentId(base + 2),
                        (nav_kind == Kind::Tabs).then_some(1),
                        AccessibilityAction::Press,
                    )
                    .consumed
                );
                render_host(&mut host, &mut text);
                let header = host
                    .semantics()
                    .into_iter()
                    .find(|n| n.id == ComponentId(base + 2))
                    .unwrap();
                if nav_kind == Kind::Tabs {
                    assert_eq!(header.semantics.value.as_deref(), Some("1"));
                } else {
                    assert_eq!(header.semantics.expanded, Some(false));
                }
            }
        } else {
            assert_eq!(
                host.get(slider_content::RANGE)
                    .unwrap()
                    .component::<SliderContent>()
                    .unwrap()
                    .model
                    .value(),
                60.
            );
            assert!(host.focus(Some(slider_content::RANGE), true));
            render_host(&mut host, &mut text);
            assert!(
                host.accessibility_action(
                    slider_content::RANGE,
                    None,
                    AccessibilityAction::Increment,
                )
                .changed
            );
            render_host(&mut host, &mut text);
            assert_eq!(
                host.get(slider_content::RANGE)
                    .unwrap()
                    .component::<SliderContent>()
                    .unwrap()
                    .model
                    .value(),
                65.
            );
            assert_eq!(
                host.text_surface(slider_content::EDITOR).unwrap().text(),
                "Keep this draft"
            );
        }
    }
}
