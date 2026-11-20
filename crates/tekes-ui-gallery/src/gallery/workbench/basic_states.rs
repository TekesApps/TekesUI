//! State fixtures for controls whose public behavior is intentionally small.
//!
//! Every sample below gets a fresh production view and model. The Gallery's
//! Playground state is therefore independent from these visual comparisons.

use tekes_ui_controls::{
    ButtonTextRenderer, NumberField,
    feedback_view::{NumberFieldView, ToolbarView},
};
use tekes_ui_scene::{Point, Rect, Scene};
use tekes_ui_tokens::{Palette, proposal};

#[derive(Clone, Copy)]
enum ToolbarState {
    Default,
    Hover,
    Pressed,
    FocusVisible,
    Disabled,
}

#[derive(Clone, Copy)]
enum NumberState {
    Default,
    Minimum,
    Maximum,
    Hover,
    Pressed,
    FocusVisible,
    Disabled,
}

#[derive(Clone, Copy)]
struct Fixture<'a, S> {
    title: &'a str,
    state: S,
}

const TOOLBAR_FIXTURES: [Fixture<'static, ToolbarState>; 5] = [
    Fixture {
        title: "Default",
        state: ToolbarState::Default,
    },
    Fixture {
        title: "Hover",
        state: ToolbarState::Hover,
    },
    Fixture {
        title: "Pressed",
        state: ToolbarState::Pressed,
    },
    Fixture {
        title: "Focus visible",
        state: ToolbarState::FocusVisible,
    },
    Fixture {
        title: "Disabled",
        state: ToolbarState::Disabled,
    },
];

const NUMBER_FIXTURES: [Fixture<'static, NumberState>; 7] = [
    Fixture {
        title: "Default · 8",
        state: NumberState::Default,
    },
    Fixture {
        title: "Minimum · decrement unavailable",
        state: NumberState::Minimum,
    },
    Fixture {
        title: "Maximum · increment unavailable",
        state: NumberState::Maximum,
    },
    Fixture {
        title: "Hover · increment",
        state: NumberState::Hover,
    },
    Fixture {
        title: "Pressed · increment",
        state: NumberState::Pressed,
    },
    Fixture {
        title: "Focus visible · increment",
        state: NumberState::FocusVisible,
    },
    Fixture {
        title: "Disabled · whole control",
        state: NumberState::Disabled,
    },
];

pub(super) fn supports(renderer: &str) -> bool {
    matches!(renderer, "toolbar" | "number-field")
}

fn render_toolbar(
    text: &mut impl ButtonTextRenderer,
    scene: &mut Scene,
    frame: Rect,
    clip: Rect,
    palette: Palette,
    state: ToolbarState,
) {
    let labels = ["Add", "Rename", "More"];
    let mut view = ToolbarView::new(labels.len(), 8.);
    if matches!(state, ToolbarState::Disabled) {
        view.set_disabled(true);
    }
    let layout = {
        let mut probe = Scene::new();
        view.render(
            &mut probe,
            text,
            &labels,
            frame,
            Some(clip),
            palette,
            proposal::TextRole::UiBody,
        )
    };
    let interaction_target = match state {
        ToolbarState::Hover | ToolbarState::Pressed => layout.get(1),
        _ => layout.first(),
    };
    if let Some(target) = interaction_target {
        let center = center(target.frame);
        match state {
            ToolbarState::Hover => view.pointer_move(center),
            ToolbarState::Pressed => {
                view.pointer_move(center);
                let _ = view.pointer_down(center);
            }
            ToolbarState::FocusVisible => view.focus(Some(0), true),
            ToolbarState::Default | ToolbarState::Disabled => {}
        }
    }
    view.render(
        scene,
        text,
        &labels,
        frame,
        Some(clip),
        palette,
        proposal::TextRole::UiBody,
    );
}

fn render_number_field(
    text: &mut impl ButtonTextRenderer,
    scene: &mut Scene,
    frame: Rect,
    clip: Rect,
    palette: Palette,
    state: NumberState,
) {
    let (value, min, max) = match state {
        NumberState::Minimum => (0., 0., 10.),
        NumberState::Maximum => (10., 0., 10.),
        _ => (8., 0., 10.),
    };
    let model = NumberField::new(value, min, max, 1.).expect("bounded fixture is valid");
    let mut view = NumberFieldView::default();
    if matches!(state, NumberState::Disabled) {
        view.set_disabled(true);
    }
    let layout = {
        let mut probe = Scene::new();
        view.render(
            &mut probe,
            text,
            &model,
            frame,
            Some(clip),
            palette,
            proposal::TextRole::UiBody,
        )
    };
    let increment_center = center(layout.increment.frame);
    match state {
        NumberState::Hover => view.pointer_move(increment_center),
        NumberState::Pressed => {
            view.pointer_move(increment_center);
            let _ = view.pointer_down(increment_center);
        }
        NumberState::FocusVisible => view.focus(true, true),
        NumberState::Default
        | NumberState::Minimum
        | NumberState::Maximum
        | NumberState::Disabled => {}
    }
    view.render(
        scene,
        text,
        &model,
        frame,
        Some(clip),
        palette,
        proposal::TextRole::UiBody,
    );
}

fn center(rect: Rect) -> Point {
    Point::new(
        rect.min_x() + rect.size.width * 0.5,
        rect.min_y() + rect.size.height * 0.5,
    )
}

pub(super) fn apphost_matrix(renderer: &str) -> Vec<tekes_ui_controls::composition::Content> {
    use tekes_ui_controls::composition::*;
    let fixtures: Vec<_> = if renderer == "toolbar" {
        TOOLBAR_FIXTURES
            .iter()
            .map(|f| (f.title, MatrixCase::Toolbar(f.state)))
            .collect()
    } else {
        NUMBER_FIXTURES
            .iter()
            .map(|f| (f.title, MatrixCase::Number(f.state)))
            .collect()
    };
    fixtures
        .into_iter()
        .enumerate()
        .map(|(i, (title, fixture))| {
            StackContent::labeled_specimen(
                ComponentId(91650 + i as u64),
                ComponentId(91600 + i as u64 * 2),
                title,
                Content::new(ComponentId(91601 + i as u64 * 2), fixture),
            )
        })
        .collect()
}
#[derive(Clone, Copy)]
enum MatrixCase {
    Toolbar(ToolbarState),
    Number(NumberState),
}
impl std::fmt::Debug for MatrixCase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Original basic state fixture")
    }
}
impl tekes_ui_controls::composition::Component for MatrixCase {
    fn measure(
        &mut self,
        text: &mut dyn ButtonTextRenderer,
        available: tekes_ui_scene::Size,
    ) -> tekes_ui_controls::layout::MeasuredChild {
        let labels: &[&str] = match self {
            Self::Toolbar(_) => &["Add", "Rename", "More"],
            Self::Number(_) => &["−", "8", "+"],
        };
        let height = labels
            .iter()
            .map(|label| {
                tekes_ui_controls::ButtonModel::measure(
                    &mut &mut *text,
                    label,
                    available.width / 3.,
                    tekes_ui_controls::ButtonSize::Body,
                    proposal::TextRole::UiBody,
                )
                .height
            })
            .fold(52., f32::max);
        tekes_ui_controls::layout::MeasuredChild::new(tekes_ui_scene::Size::new(
            available.width.min(600.),
            height,
        ))
    }
    fn paint(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        frame: Rect,
        clip: Rect,
        theme: tekes_ui_controls::composition::ContentTheme,
    ) {
        let mut text = super::apphost_basic::FixtureText(text);
        match *self {
            Self::Toolbar(state) => {
                render_toolbar(&mut text, scene, frame, clip, theme.palette, state)
            }
            Self::Number(state) => {
                render_number_field(&mut text, scene, frame, clip, theme.palette, state)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::{apphost_basic::BasicStory, entries};
    use crate::gallery::text::TextEngine;
    use tekes_ui_controls::composition::{
        AccessibilityAction, ComponentId, StackContent, TextContent,
    };
    use tekes_ui_scene::{Rect, Scene};
    use tekes_ui_tokens::{Theme, proposal::TextRole};

    #[test]
    fn apphost_toolbar_and_number_field_state_matrices_render_on_gpu() {
        let mut renderer = tekes_ui_render::Renderer::new_headless()
            .expect("headless GPU is required for real state-matrix rendering");
        for renderer_name in ["toolbar", "number-field"] {
            let entry = entries()
                .iter()
                .find(|entry| entry.renderer() == renderer_name)
                .expect("registered component");
            let mut story = BasicStory::new(entry);
            let mut text = TextEngine::new();
            text.set_scale(1.);
            text.set_type_scale(1.5);
            let frame = Rect::new(0., 0., 360., 1200.);
            story
                .host
                .render(
                    &mut Scene::new(),
                    &mut text,
                    frame,
                    frame,
                    Theme::LIGHT.into(),
                )
                .unwrap();
            assert!(
                story
                    .host
                    .accessibility_action(ComponentId(92103), None, AccessibilityAction::Press)
                    .consumed
            );
            story.after_host_visit();
            let mut scene = Scene::new();
            story
                .host
                .render(&mut scene, &mut text, frame, frame, Theme::LIGHT.into())
                .unwrap();
            assert!(!scene.is_empty());

            let specimen_count = if renderer_name == "toolbar" { 5 } else { 7 };
            for i in 0..specimen_count {
                let group = story.host.get(ComponentId(91650 + i as u64)).unwrap();
                let stack = group.component::<StackContent>().unwrap();
                assert_eq!(stack.model.gap, StackContent::SPECIMEN_LABEL_GAP);
                let label = story.host.get(ComponentId(91600 + i as u64 * 2)).unwrap();
                assert_eq!(
                    label.component::<TextContent>().unwrap().role,
                    TextRole::UiCaption
                );
                let control = story.host.get(ComponentId(91601 + i as u64 * 2)).unwrap();
                assert!(control.frame().size.height > 0.);
            }

            let rgba = renderer
                .render_to_rgba8(
                    &scene,
                    text.atlas_mut(),
                    (360, 1200),
                    1.,
                    crate::gallery::Gallery::background(Theme::LIGHT),
                )
                .expect("production state matrix renders on the headless GPU");
            let background = &rgba[0..4];
            assert!(
                rgba.chunks_exact(4).any(|pixel| pixel != background),
                "{renderer_name} GPU frame contains rendered matrix pixels"
            );
        }
    }
}
