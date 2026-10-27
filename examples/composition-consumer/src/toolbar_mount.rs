//! Independent window consumer: add Undo/Redo declarations, no per-group wiring.
use tekes_ui_controls::{
    ButtonTextRenderer, composition::*, extended::Progress, feedback_view::ProgressView,
    layout::MeasuredChild,
};
use tekes_ui_scene::{Rect, Scene, Size};
pub const ROOT: ComponentId = ComponentId(21000);
pub const MAIN: ComponentId = ComponentId(21001);
pub const SETTINGS: ComponentId = ComponentId(21002);
pub const NAV: ComponentId = ComponentId(21010);
pub const EDIT: ComponentId = ComponentId(21020);
pub const EDIT_CONTROL: ComponentId = ComponentId(21021);
pub const PROGRESS: ComponentId = ComponentId(21030);
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Source {
    pub module: &'static str,
    pub generation: u64,
}
pub fn regions(width: f32) -> Vec<ToolbarRegion> {
    let mut main = ToolbarRegion::new(MAIN, Rect::new(0., 0., width * 0.55, 84.));
    main.native_safe_area.left = 70.;
    vec![
        main,
        ToolbarRegion::new(SETTINGS, Rect::new(width * 0.55, 0., width * 0.45, 84.)),
    ]
}
pub fn build(width: f32) -> Result<ToolbarHost<Source>, CompositionError> {
    let mut host = ToolbarHost::new(ROOT, 7, Rect::new(0., 0., width, 360.))?;
    host.set_regions(7, Rect::new(0., 0., width, 360.), regions(width))?;
    host.register(ToolbarRegistration {
        region: MAIN,
        order: 0,
        group: Content::new(
            NAV,
            ToolbarGroup::new(
                ToolbarPlacement::Navigation,
                vec![toolbar_commands(
                    ComponentId(21011),
                    vec![ToolbarButtonDeclaration {
                        id: 1,
                        icon: None,
                        text: "Home".into(),
                        hint: Some("Navigate home".into()),
                        enabled: true,
                    }],
                    ToolbarGroupOverflow::Together,
                )],
            ),
        ),
        bindings: vec![ToolbarCommandBinding {
            target: ComponentId(21011),
            action: Some(1),
            command: 1,
            source: Source {
                module: "navigation",
                generation: 3,
            },
        }],
    })?;
    host.register(ToolbarRegistration {
        region: MAIN,
        order: 1,
        group: Content::new(
            ComponentId(21015),
            ToolbarGroup::new(
                ToolbarPlacement::Principal,
                vec![Content::new(
                    ComponentId(21016),
                    TextContent::new("Main / Settings · Original Text"),
                )],
            ),
        ),
        bindings: vec![],
    })?;
    host.register(undo_redo())?;
    host.register(ToolbarRegistration {
        region: SETTINGS,
        order: 1,
        group: Content::new(
            PROGRESS,
            ToolbarGroup::new(
                ToolbarPlacement::Status,
                vec![
                    Content::new(ComponentId(21031), JobProgress::default())
                        .with_help("Background import progress"),
                ],
            ),
        ),
        bindings: vec![],
    })?;
    Ok(host)
}
pub fn undo_redo() -> ToolbarRegistration<Source> {
    ToolbarRegistration {
        region: SETTINGS,
        order: 0,
        group: Content::new(
            EDIT,
            ToolbarGroup::new(
                ToolbarPlacement::PrimaryAction,
                vec![toolbar_commands(
                    EDIT_CONTROL,
                    [(100, "Undo"), (101, "Redo")]
                        .into_iter()
                        .map(|(id, label)| ToolbarButtonDeclaration {
                            id,
                            icon: None,
                            text: label.into(),
                            hint: Some(format!("{label} settings edit")),
                            enabled: true,
                        })
                        .collect(),
                    ToolbarGroupOverflow::Together,
                )],
            ),
        ),
        bindings: [100, 101]
            .into_iter()
            .map(|id| ToolbarCommandBinding {
                target: EDIT_CONTROL,
                action: Some(id),
                command: id,
                source: Source {
                    module: "settings",
                    generation: 9,
                },
            })
            .collect(),
    }
}
/// Test consumer's ordinary Component adapter over the existing ProgressView.
/// It owns only its Progress value; Toolbar adds no progress-specific logic.
#[derive(Debug)]
pub struct JobProgress {
    pub value: Progress,
    view: ProgressView,
}
impl Default for JobProgress {
    fn default() -> Self {
        Self {
            value: Progress::new(0.42),
            view: ProgressView::default(),
        }
    }
}
impl Component for JobProgress {
    fn measure(&mut self, text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        let body = text.resolved_font_size(tekes_ui_tokens::proposal::TextRole::UiBody);
        MeasuredChild::new(Size::new((body * 8.).min(available.width), body * 0.5))
    }
    fn paint(
        &mut self,
        scene: &mut Scene,
        _: &mut dyn ButtonTextRenderer,
        frame: Rect,
        clip: Rect,
        theme: ContentTheme,
    ) {
        self.view
            .render(scene, self.value, frame, Some(clip), theme.palette);
    }
    fn semantics(&self) -> ContentSemantics {
        let mut s = ContentSemantics::new(ContentRole::Group, "Import progress");
        s.value = Some(format!("{:.0}%", self.value.value() * 100.));
        s
    }
}
