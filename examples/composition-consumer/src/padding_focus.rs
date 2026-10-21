//! Passive input decoration around one retained editor and a real sibling action.
//! This consumer supplies only layout and IDs; ContentHost owns input routing.
use tekes_ui_controls::{
    ButtonTextRenderer,
    composition::*,
    extended::{Axis, Stack},
    feedback_view::SurfaceView,
    layout::MeasuredChild,
};
use tekes_ui_scene::{Rect, Scene, Size};
use tekes_ui_tokens::{Spacing, proposal};
pub const ROOT: ComponentId = ComponentId(14000);
pub const BAND: ComponentId = ComponentId(14001);
pub const EDITOR: ComponentId = ComponentId(14002);
pub const ACTION: ComponentId = ComponentId(14003);
pub const OTHER: ComponentId = ComponentId(14004);
#[derive(Debug)]
pub struct InputBand {
    pub shown: bool,
    pub disabled: bool,
    pub children: Vec<Content>,
    view: SurfaceView,
}
impl Component for InputBand {
    fn active(&self) -> bool {
        self.shown
    }
    fn semantics(&self) -> ContentSemantics {
        let mut s = ContentSemantics::new(ContentRole::Group, "Input surface");
        s.disabled = self.disabled;
        s
    }
    fn children(&self) -> &[Content] {
        &self.children
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.children)
    }
    fn measure(&mut self, t: &mut dyn ButtonTextRenderer, a: Size) -> MeasuredChild {
        for c in &mut self.children {
            c.measure(t, a);
        }
        MeasuredChild::new(Size::new(a.width, 120.))
    }
    fn arrange(&mut self, t: &mut dyn ButtonTextRenderer, f: Rect, clip: Rect) {
        let p = proposal::metric("spacing", "space-loose").unwrap();
        let body = f.inset(p);
        for child in &mut self.children {
            let frame = if child.id() == ACTION {
                Rect::new(body.max_x() - 100., body.min_y(), 100., body.size.height)
            } else {
                Rect::new(
                    body.min_x(),
                    body.min_y(),
                    (body.size.width - 112.).max(0.),
                    body.size.height,
                )
            };
            child.arrange(t, frame, clip.intersection(&f).unwrap_or_default());
        }
    }
    fn paint(
        &mut self,
        s: &mut Scene,
        t: &mut dyn ButtonTextRenderer,
        f: Rect,
        clip: Rect,
        theme: ContentTheme,
    ) {
        self.view
            .render_at_scale(s, f, Some(clip), theme.palette, true, t.backing_scale());
    }
}
pub fn build() -> Result<ContentHost, CompositionError> {
    let mut field = TextFieldContent::new("Message", "Draft A😀B");
    field.surface.presentation = tekes_ui_controls::TextSurfacePresentation::Content;
    let band = InputBand {
        shown: true,
        disabled: false,
        view: SurfaceView::default(),
        children: vec![
            Content::new(EDITOR, field),
            Content::new(ACTION, ButtonContent::new("Send")),
        ],
    };
    let mut host = ContentHost::new(Content::new(
        ROOT,
        StackContent {
            model: Stack {
                axis: Axis::Vertical,
                gap: Spacing::Regular.value(),
            },
            flexible_child: None,
            children: vec![
                Content::new(BAND, band),
                Content::new(OTHER, TextFieldContent::new("Other", "Other draft")),
            ],
        },
    ))?;
    host.set_action_events(true);
    host.set_padding_focus(BAND, Some(EDITOR))?;
    Ok(host)
}
