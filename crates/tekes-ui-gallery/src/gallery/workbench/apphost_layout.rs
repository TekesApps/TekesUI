//! Passive layout stories retain their original Stack/Grid/Separator models.
//! The Content node owns measurement and placement, not a captured legacy Scene.
use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer,
    composition::*,
    extended::{Axis, Grid, Stack},
    layout::MeasuredChild,
    paint,
};
use tekes_ui_scene::{Rect, Scene, Size};
use tekes_ui_tokens::proposal::TextRole;

#[derive(Debug)]
enum LayoutModel {
    Stack(Stack),
    Grid(Grid),
    Separator(Axis),
}
#[derive(Debug)]
struct LayoutFixture {
    model: LayoutModel,
    count: usize,
    height: f32,
    labels: Vec<String>,
    cells: Vec<Rect>,
}
impl Component for LayoutFixture {
    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        let limit = match self.model {
            LayoutModel::Stack(_) if !self.labels.is_empty() => 650.,
            LayoutModel::Grid(_) if !self.labels.is_empty() => 720.,
            LayoutModel::Separator(_) => 600.,
            _ => 620.,
        };
        MeasuredChild::new(Size::new(available.width.min(limit), self.height))
    }
    fn arrange(&mut self, _: &mut dyn ButtonTextRenderer, frame: Rect, _: Rect) {
        self.cells = match &self.model {
            LayoutModel::Stack(model) => model.layout(frame, self.count),
            LayoutModel::Grid(model) => model.layout(
                frame,
                self.count,
                if self.labels.is_empty() { 26. } else { 58. },
            ),
            LayoutModel::Separator(Axis::Horizontal) => vec![frame],
            LayoutModel::Separator(Axis::Vertical) => vec![Rect::new(
                frame.min_x() + (frame.size.width - 12.).clamp(0., 300.),
                frame.min_y(),
                12_f32.min(frame.size.width),
                frame.size.height,
            )],
        };
    }
    fn paint(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        _: Rect,
        clip: Rect,
        theme: ContentTheme,
    ) {
        scene.push_clip(clip);
        for (i, frame) in self.cells.iter().copied().enumerate() {
            if let LayoutModel::Separator(axis) = self.model {
                tekes_ui_controls::extended::separator_at_scale(
                    scene,
                    frame,
                    theme.palette,
                    axis,
                    text.backing_scale(),
                );
            } else {
                paint::surface_at_scale(
                    scene,
                    frame.inset(0.5),
                    theme.palette,
                    true,
                    text.backing_scale(),
                );
                if let Some(label) = self.labels.get(i) {
                    text.draw(
                        scene,
                        label,
                        TextRole::UiBody,
                        frame.inset(8.),
                        ButtonTextAlign::default(),
                        theme
                            .palette
                            .color("primary-text")
                            .expect("standard text token"),
                    );
                }
            }
        }
        scene.pop_clip();
    }
    fn inspection_properties(&self) -> Option<Vec<InspectionProperty>> {
        Some(vec![
            InspectionProperty::new("model", format!("{:?}", self.model)),
            InspectionProperty::new("count", self.count.to_string()),
        ])
    }
}
fn fixture(id: u64, model: LayoutModel, count: usize, height: f32, labels: Vec<String>) -> Content {
    Content::new(
        ComponentId(id),
        LayoutFixture {
            model,
            count,
            height,
            labels,
            cells: vec![],
        },
    )
}
fn text(id: u64, value: impl Into<String>) -> Content {
    Content::new(ComponentId(id), TextContent::new(value))
}
pub(super) fn supports(renderer: &str) -> bool {
    matches!(renderer, "stack" | "grid" | "separator")
}
pub(super) fn children(renderer: &str, variants: bool) -> Vec<Content> {
    let mut children = vec![];
    match renderer {
        "stack" => {
            for (i, gap) in if variants {
                vec![0., 8., 16.]
            } else {
                vec![8.]
            }
            .into_iter()
            .enumerate()
            {
                if variants {
                    children.push(text(92700 + i as u64, format!("Gap: {gap:.0} pt")));
                }
                children.push(fixture(
                    92600 + (gap / 8.) as u64,
                    LayoutModel::Stack(Stack {
                        axis: Axis::Horizontal,
                        gap,
                    }),
                    3,
                    if variants { 36. } else { 64. },
                    if variants {
                        vec![]
                    } else {
                        vec!["Leading".into(), "Center".into(), "Trailing".into()]
                    },
                ));
            }
            if !variants {
                children.push(text(
                    92710,
                    "Stack divides the available main axis evenly; gap is 8 pt.",
                ));
            }
        }
        "grid" => {
            for (i, columns) in if variants { vec![2, 3, 4] } else { vec![3] }
                .into_iter()
                .enumerate()
            {
                if variants {
                    children.push(text(92700 + i as u64, format!("{columns} columns")));
                }
                let mut model = Grid::new(columns);
                model.gap_x = 8.;
                model.gap_y = 8.;
                children.push(fixture(
                    92600 + columns as u64 - 2,
                    LayoutModel::Grid(model),
                    if variants { columns * 2 } else { 9 },
                    if variants { 60. } else { 190. },
                    if variants {
                        vec![]
                    } else {
                        (1..=9).map(|i| format!("Item {i:02}")).collect()
                    },
                ));
            }
        }
        "separator" => {
            children.push(text(
                92700,
                if variants {
                    "Horizontal"
                } else {
                    "Account settings"
                },
            ));
            children.push(fixture(
                92600,
                LayoutModel::Separator(Axis::Horizontal),
                1,
                12.,
                vec![],
            ));
            children.push(text(
                92701,
                if variants {
                    "Vertical"
                } else {
                    "Profile and preferences"
                },
            ));
            children.push(fixture(
                92601,
                LayoutModel::Separator(Axis::Vertical),
                1,
                160.,
                vec![],
            ));
        }
        _ => unreachable!("registered layout story"),
    }
    children
}

/// A Gallery page and its original Dialog share the receiving viewport. Modal
/// content is a sibling of the scrolled document, never a row measured with the
/// document's unbounded height. Both stories use this one composition boundary.
pub(super) fn dialog_viewport(id: ComponentId, children: Vec<Content>) -> Content {
    Content::new(
        id,
        DialogViewport {
            children,
            arranged: false,
        },
    )
}
#[derive(Debug)]
struct DialogViewport {
    children: Vec<Content>,
    arranged: bool,
}
impl Component for DialogViewport {
    fn children(&self) -> &[Content] {
        &self.children
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.children)
    }
    fn host_reveal_passthrough(&self) -> bool {
        self.arranged
    }
    fn curve_subtree_capability(&self) -> CurveSubtreeCapability {
        CurveSubtreeCapability::Recursive
    }
    fn table_transaction_compatibility(&self) -> TableTransactionCompatibility {
        TableTransactionCompatibility::StableHooks
    }
    fn action_transaction_compatibility(&self) -> ActionTransactionCompatibility {
        ActionTransactionCompatibility::StableHooks
    }
    fn prepared_effects_work(&self, n: usize) -> Result<usize, PreparedEffectsError> {
        PreparedEffects::local_work(n)
    }
    fn prepare_effects(
        &self,
        hooks: &[PreparedHook],
    ) -> Result<PreparedEffects, PreparedEffectsError> {
        Ok(PreparedEffects::local(self, hooks))
    }
    fn lifecycle(&mut self, _: Lifecycle) {
        self.arranged = false;
    }
    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, a: Size) -> MeasuredChild {
        self.arranged = false;
        MeasuredChild::new(a)
    }
    fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        for child in &mut self.children {
            child.arrange(text, frame, clip);
        }
        self.arranged = true;
    }
}
