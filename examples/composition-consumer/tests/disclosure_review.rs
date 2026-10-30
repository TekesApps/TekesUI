//! Drives the exact native phase controller through public ContentHost APIs.
//! This is headless fixture validation, not AppKit, real SVG pixels, IME or VO.
#![cfg(target_os = "macos")]
#[path = "support/coretext.rs"]
mod coretext;
#[path = "../examples/support/disclosure_review.rs"]
mod review;
use std::{collections::VecDeque, rc::Rc, time::Duration};
use tekes_ui_composition_consumer::disclosure_review as f;
use tekes_ui_controls::{ControlKey, composition::*, layout_view::IconRenderer};
use tekes_ui_platform_macos::selftest::Action;
use tekes_ui_scene::{ImageId, Rect, Scene};
use tekes_ui_tokens::{Rgba, Theme};
struct Icons;
impl IconRenderer for Icons {
    fn prepare_icon(
        &mut self,
        _: &str,
        _: f32,
        _: f32,
        _: Rgba,
    ) -> Result<IconImage, IconRenderError> {
        Ok(IconImage::new(ImageId(99), Rc::new(())))
    }
    fn draw_icon(&mut self, _: &mut Scene, _: &str, _: Rect, _: Rgba) {
        unreachable!()
    }
}
#[test]
fn original_native_phase_script_reaches_all_checkpoints_with_normal_and_reduced_motion() {
    for reduced in [false, true] {
        let directory = std::env::temp_dir().join(format!(
            "tekesui-disclosure38-headless-{}-{reduced}",
            std::process::id()
        ));
        std::fs::create_dir(&directory).unwrap();
        let mut controller = review::Review::new(directory.clone(), false);
        let mut tree = f::build().unwrap();
        let mut text = coretext::Text::new(1.);
        text.services = true;
        text.reduced_motion = reduced;
        let mut width = 720.;
        let mut theme = Theme::LIGHT;
        let mut records = Vec::new();
        let mut phases = Vec::new();
        let mut captures = Vec::new();
        let mut queue = VecDeque::from([(300, Action::MenuCommand(38_001))]);
        while let Some((delay, action)) = queue.pop_front() {
            text.now += Duration::from_millis(delay);
            let frame = Rect::new(8., 8., width as f32 - 16., 624.);
            tree.render(
                &mut Scene::new(),
                &mut WithIconRenderer::new(&mut text, &mut Icons),
                frame,
                frame,
                theme.into(),
            )
            .unwrap();
            match action {
                Action::MenuCommand(phase) => {
                    phases.push(phase);
                    let script = controller.advance(phase, &mut tree, &records, true, width);
                    // Native run_script timestamps are relative to this phase.
                    let mut previous = 0;
                    for (at, action) in script {
                        queue.push_back((at - previous, action));
                        previous = at;
                    }
                }
                Action::Click { x, y } => {
                    let p = tekes_ui_scene::Point::new(x as f32, y as f32);
                    tree.event(&ContentEvent::PointerDown(p));
                    tree.event(&ContentEvent::PointerUp(p));
                }
                Action::KeyCharacters { key_code, .. } => {
                    let key = match key_code {
                        36 => ControlKey::Enter,
                        49 => ControlKey::Space,
                        48 => ControlKey::Tab { reverse: false },
                        _ => panic!("unhandled native fixture key"),
                    };
                    tree.event(&ContentEvent::Key { key, down: true });
                    tree.event(&ContentEvent::Key { key, down: false });
                }
                // Every queued AXPress is a positive native action. The marked
                // negative public SDK action is asserted by phase 38009 itself.
                Action::AXPress { component } => {
                    assert!(
                        tree.accessibility_action(
                            ComponentId(component),
                            None,
                            AccessibilityAction::Press
                        )
                        .activated
                    );
                }
                Action::AXSelect { component, range } => {
                    assert!(
                        tree.accessibility_action(
                            ComponentId(component),
                            None,
                            AccessibilityAction::SetSelection(range)
                        )
                        .consumed
                    );
                }
                Action::PreeditRange {
                    text,
                    selection,
                    replacement,
                } => {
                    tree.event(&ContentEvent::Preedit {
                        clauses: vec![],
                        text,
                        selection,
                        replacement,
                    });
                }
                Action::Unmark => {
                    tree.event(&ContentEvent::Unmark);
                }
                Action::Resize { w, .. } => width = w,
                Action::WindowDark(dark) => theme = if dark { Theme::DARK } else { Theme::LIGHT },
                Action::Capture { path } => captures.push(path),
                Action::Close => break,
                _ => panic!("unhandled action in fixed Disclosure script"),
            }
            records.extend(tree.drain_action_records());
        }
        assert_eq!(phases, (38_001..=38_015).collect::<Vec<_>>());
        assert_eq!(records.len(), 7);
        assert_eq!(captures.len(), 6);
        assert!(!f::disclosure(&tree).expanded());
        std::fs::remove_file(directory.join("disclosure38-journal.jsonl")).unwrap();
        std::fs::remove_dir(directory).unwrap();
    }
}
