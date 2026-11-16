//! Original emphasized Field metadata and passive input state fixtures.
use super::{StoryState, apphost_basic::FixtureText, apphost_input_sections::TextFixture};
use tekes_ui_controls::{
    ButtonTextRenderer, Field,
    composition::*,
    feedback_view::{FieldPresentation, FieldView},
    layout::MeasuredChild,
};
use tekes_ui_scene::{Rect, Scene, Size};
use tekes_ui_tokens::proposal::TextRole;
#[derive(Debug)]
struct Fixture {
    field: Field,
    view: FieldView,
    input: TextFixture,
}
impl Component for Fixture {
    fn measure(&mut self, text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        let layout = self.view.render_with_presentation(
            &mut Scene::new(),
            &mut FixtureText(text),
            &self.field,
            Rect::new(0., 0., available.width, 4096.),
            None,
            tekes_ui_tokens::Theme::LIGHT.into(),
            TextRole::UiBody,
            FieldPresentation::emphasized_form(),
        );
        MeasuredChild::new(Size::new(
            available.width,
            layout.message.max_y().max(layout.slot.max_y()),
        ))
    }
    fn paint(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        frame: Rect,
        clip: Rect,
        theme: ContentTheme,
    ) {
        let layout = self.view.render_with_presentation(
            scene,
            &mut FixtureText(text),
            &self.field,
            frame,
            Some(clip),
            theme.palette,
            TextRole::UiBody,
            FieldPresentation::emphasized_form(),
        );
        self.input.arrange(text, layout.slot, clip);
        self.input.paint(scene, text, layout.slot, clip, theme);
    }
}
pub(super) fn matrix(state: &StoryState) -> Content {
    let mut children = Vec::new();
    for (i, (name, mut control)) in super::apphost_basic::standard_states()
        .into_iter()
        .filter(|(name, _)| !matches!(*name, "hover" | "pressed" | "loading"))
        .enumerate()
    {
        control.error = name == "error + focus";
        let mut field = state.field.clone();
        field.label = "Workspace name".into();
        field.help = "Use a clear name for your workspace.".into();
        field.error = control.error.then(|| "A name is required.".into());
        children.push(Content::new(
            ComponentId(120100 + i as u64 * 2),
            TextContent::new(name),
        ));
        children.push(Content::new(
            ComponentId(120101 + i as u64 * 2),
            Fixture {
                field,
                view: FieldView::default(),
                input: TextFixture::new(false, "default", control),
            },
        ));
    }
    Content::new(
        ComponentId(120099),
        StackContent {
            model: tekes_ui_controls::extended::Stack {
                axis: tekes_ui_controls::extended::Axis::Vertical,
                gap: 24.,
            },
            children,
            flexible_child: None,
        },
    )
}
