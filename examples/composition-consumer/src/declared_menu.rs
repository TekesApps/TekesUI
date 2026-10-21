//! Independent original Menu consumer: business values stay here, not in SDK.
use tekes_ui_controls::{
    ButtonFramePolicy, ButtonTextAlign, ButtonTextRenderer, composition::*, layout::MeasuredChild,
};
use tekes_ui_scene::{Rect, Scene, Size};
use tekes_ui_tokens::{Spacing, proposal};
pub const ROOT: ComponentId = ComponentId(18200);
pub const OPTIONS: ComponentId = ComponentId(18201);
pub const GLYPH: ComponentId = ComponentId(18202);
pub const MESSAGE: ComponentId = ComponentId(18203);
pub const HEADING: ComponentId = ComponentId(18204);
#[derive(Debug)]
struct OptionsGlyph;
impl Component for OptionsGlyph {
    fn measure(&mut self, t: &mut dyn ButtonTextRenderer, a: Size) -> MeasuredChild {
        let s = t.measure("⋮", proposal::TextRole::UiBody, a.width);
        MeasuredChild::new(Size::new(s.width.max(28.), s.height.max(28.)))
    }
    fn paint(
        &mut self,
        s: &mut Scene,
        t: &mut dyn ButtonTextRenderer,
        f: Rect,
        _: Rect,
        theme: ContentTheme,
    ) {
        t.draw(
            s,
            "⋮",
            proposal::TextRole::UiBody,
            f,
            ButtonTextAlign::Center,
            theme
                .foreground
                .unwrap_or_else(|| theme.palette.color("primary-text").unwrap()),
        );
    }
}
#[derive(Debug)]
struct Sidebar {
    children: Vec<Content>,
    measured: Vec<Size>,
}
impl Component for Sidebar {
    fn children(&self) -> &[Content] {
        &self.children
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.children)
    }
    fn measure(&mut self, t: &mut dyn ButtonTextRenderer, a: Size) -> MeasuredChild {
        self.measured = self
            .children
            .iter_mut()
            .map(|c| c.measure(t, a).size)
            .collect();
        MeasuredChild::new(Size::new(
            a.width,
            self.measured.iter().map(|s| s.height).sum::<f32>() + Spacing::Regular.value() * 2.,
        ))
    }
    fn arrange(&mut self, t: &mut dyn ButtonTextRenderer, f: Rect, c: Rect) {
        let mut y = f.min_y();
        for (i, child) in self.children.iter_mut().enumerate() {
            let measured = self.measured[i];
            let width = if i == 1 {
                measured.width.min(f.size.width)
            } else {
                f.size.width
            };
            child.arrange(t, Rect::new(f.min_x(), y, width, measured.height), c);
            y += measured.height + Spacing::Regular.value();
        }
    }
}
pub fn options() -> Vec<MenuEntry> {
    vec![
        MenuEntry::submenu(
            "sort",
            "Sort by",
            vec![
                MenuEntry::Item {
                    value: "recentActivity".into(),
                    label: "Recent activity".into(),
                    checked: Some(true),
                    disabled: false,
                },
                MenuEntry::Item {
                    value: "titleAscending".into(),
                    label: "Title A–Z".into(),
                    checked: Some(false),
                    disabled: false,
                },
                MenuEntry::Item {
                    value: "titleDescending".into(),
                    label: "Title Z–A".into(),
                    checked: Some(false),
                    disabled: false,
                },
            ],
        ),
        MenuEntry::submenu(
            "group",
            "Group by",
            vec![
                MenuEntry::Item {
                    value: "none".into(),
                    label: "None".into(),
                    checked: Some(true),
                    disabled: false,
                },
                MenuEntry::Item {
                    value: "folder".into(),
                    label: "Primary Folder".into(),
                    checked: Some(false),
                    disabled: false,
                },
            ],
        ),
        MenuEntry::Separator,
        MenuEntry::Item {
            value: "movePinUp".into(),
            label: "Move Pin Up".into(),
            checked: None,
            disabled: true,
        },
        MenuEntry::item("movePinDown", "Move Pin Down"),
    ]
}
pub fn build() -> Result<ContentHost, CompositionError> {
    let mut menu = MenuContent::new("Session list options", Vec::new())
        .with_trigger_content("Session list options", Content::new(GLYPH, OptionsGlyph))?;
    menu.trigger.button.frame_policy = ButtonFramePolicy::Exact;
    menu.set_entries(options())
        .expect("fixed valid example declarations");
    let root = Content::new(
        ROOT,
        Sidebar {
            children: vec![
                Content::new(HEADING, TextContent::new("Sidebar options")),
                Content::new(OPTIONS, menu),
                Content::new(MESSAGE, TextFieldContent::new("Message", "Keep A😀B")),
            ],
            measured: Vec::new(),
        },
    );
    let mut h = ContentHost::new(root)?;
    h.set_action_events(true);
    Ok(h)
}
