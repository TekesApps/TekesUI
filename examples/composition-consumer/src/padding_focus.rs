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
pub const READ_ONLY_TEXT: &str = "A😀B 中文 e\u{301} selectable";

pub fn build() -> Result<ContentHost, CompositionError> {
    let mut field = TextFieldContent::new("Message", "Draft A😀B");
    field.surface.presentation = tekes_ui_controls::TextSurfacePresentation::Content;
    build_with_text(field, false)
}

/// Separate public readonly consumer of the same retained text/action layout.
/// The host provides no hit, caret, focus, capture or IME state machine.
pub fn build_read_only() -> Result<ContentHost, CompositionError> {
    build_with_text(
        TextFieldContent::read_only_content(
            "Selectable read-only text",
            READ_ONLY_TEXT,
            proposal::TextRole::UiBody,
        ),
        true,
    )
}

fn build_with_text(
    field: TextFieldContent,
    allow_read_only: bool,
) -> Result<ContentHost, CompositionError> {
    let band = InputBand {
        shown: true,
        disabled: false,
        view: SurfaceView::default(),
        children: vec![
            Content::new(EDITOR, field),
            Content::new(
                ACTION,
                Button::new(if allow_read_only { "Open" } else { "Send" }),
            ),
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
    if allow_read_only {
        host.set_padding_text_focus(BAND, Some(EDITOR))?;
    } else {
        host.set_padding_focus(BAND, Some(EDITOR))?;
    }
    Ok(host)
}

/// Additional same-turn host policy for recognized decorative padding only.
/// None is not general permission to forward input: retain the host's existing
/// direct-input/overlay policy. This helper stores no focus, capture or IME state.
pub fn padding_ime_admission(host: &ContentHost, event: &ContentEvent) -> Option<bool> {
    let target = host.padding_focus_target(event)?;
    Some(!host.is_composing() || host.focused() == Some(target))
}
