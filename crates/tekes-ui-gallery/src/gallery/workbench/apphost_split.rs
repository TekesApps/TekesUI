//! The original split model in formal SplitViewContent, with finite pane allocation.
use super::{StoryState, apphost_basic::SPECIMEN};
use std::{cell::Cell, rc::Rc};
use tekes_ui_controls::{
    ButtonTextRenderer,
    composition::*,
    extended::{Axis, Pane, SplitView, Stack},
    layout::MeasuredChild,
    paint,
};
use tekes_ui_scene::{Rect, Scene, Size};
use tekes_ui_tokens::proposal::TextRole;
pub(super) const DIVIDER: ComponentId = ComponentId(123003);
pub(super) const VARIANTS: [f32; 3] = [0.3, 0.5, 0.7];
#[derive(Debug)]
struct PaneBody(&'static str);
impl Component for PaneBody {
    fn table_transaction_compatibility(&self) -> TableTransactionCompatibility {
        TableTransactionCompatibility::StableHooks
    }
    // Immutable leaf: children access and lifecycle/presentation hooks have no
    // external effects, allocations, or callbacks to prepare.
    fn prepared_effects_work(&self, calls: usize) -> Result<usize, PreparedEffectsError> {
        PreparedEffects::local_work(calls)
    }
    fn prepare_effects(
        &self,
        hooks: &[PreparedHook],
    ) -> Result<PreparedEffects, PreparedEffectsError> {
        Ok(PreparedEffects::local(self, hooks))
    }
    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, a: Size) -> MeasuredChild {
        MeasuredChild::new(a)
    }
    fn paint(
        &mut self,
        s: &mut Scene,
        t: &mut dyn ButtonTextRenderer,
        f: Rect,
        c: Rect,
        theme: ContentTheme,
    ) {
        s.push_clip(c);
        paint::surface_at_scale(s, f, theme.palette, true, t.backing_scale());
        t.draw(
            s,
            self.0,
            TextRole::UiBody,
            f.inset(12.),
            tekes_ui_controls::ButtonTextAlign::Leading,
            theme.palette.color("primary-text").unwrap(),
        );
        s.pop_clip();
    }
}
#[derive(Debug)]
struct Choices {
    visible: Rc<Cell<bool>>,
    stack: StackContent,
    arranged: bool,
}
impl Component for Choices {
    fn active(&self) -> bool {
        self.visible.get()
    }
    fn children(&self) -> &[Content] {
        &self.stack.children
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.stack.children)
    }
    fn host_reveal_passthrough(&self) -> bool {
        self.arranged
    }
    fn action_transaction_compatibility(&self) -> ActionTransactionCompatibility {
        ActionTransactionCompatibility::StableHooks
    }
    fn prepared_effects_work(&self, n: usize) -> Result<usize, PreparedEffectsError> {
        PreparedEffects::local_work(n)
    }
    fn prepare_effects(&self, h: &[PreparedHook]) -> Result<PreparedEffects, PreparedEffectsError> {
        Ok(PreparedEffects::local(self, h))
    }
    fn measure(&mut self, t: &mut dyn ButtonTextRenderer, a: Size) -> MeasuredChild {
        self.arranged = false;
        self.stack.measure(t, a)
    }
    fn arrange(&mut self, t: &mut dyn ButtonTextRenderer, f: Rect, c: Rect) {
        self.stack.arrange(t, f, c);
        self.arranged = true;
    }
    fn lifecycle(&mut self, _: Lifecycle) {
        self.arranged = false;
    }
}
#[derive(Debug)]
struct Viewport {
    children: Vec<Content>,
    constrained: Rc<Cell<bool>>,
    choice_height: f32,
    arranged: bool,
}
impl Component for Viewport {
    fn table_transaction_compatibility(&self) -> TableTransactionCompatibility {
        TableTransactionCompatibility::StableHooks
    }
    fn children(&self) -> &[Content] {
        &self.children
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.children)
    }
    fn fixed_slots(&self) -> bool {
        true
    }
    fn host_reveal_passthrough(&self) -> bool {
        self.arranged
    }
    fn action_transaction_compatibility(&self) -> ActionTransactionCompatibility {
        ActionTransactionCompatibility::StableHooks
    }
    fn prepared_effects_work(&self, n: usize) -> Result<usize, PreparedEffectsError> {
        PreparedEffects::local_work(n)
    }
    fn prepare_effects(&self, h: &[PreparedHook]) -> Result<PreparedEffects, PreparedEffectsError> {
        Ok(PreparedEffects::local(self, h))
    }
    fn lifecycle(&mut self, _: Lifecycle) {
        self.arranged = false;
    }
    fn measure(&mut self, t: &mut dyn ButtonTextRenderer, a: Size) -> MeasuredChild {
        self.arranged = false;
        let width = a.width.min(760.);
        let config = self.children[1]
            .component::<SplitViewContent>()
            .unwrap()
            .config()
            .unwrap();
        let mut model = SplitView::new(config.ratio);
        model.divider = config.divider;
        model.min_pane = config.min_pane;
        self.constrained
            .set(model.is_constrained(Rect::new(0., 0., width, 360.), Axis::Horizontal));
        self.choice_height = if self.constrained.get() {
            self.children[0]
                .measure(t, Size::new(width, 100.))
                .size
                .height
                + 8.
        } else {
            0.
        };
        self.children[1].measure(t, Size::new(width, 360.));
        MeasuredChild::new(Size::new(width, 360. + self.choice_height))
    }
    fn arrange(&mut self, t: &mut dyn ButtonTextRenderer, f: Rect, c: Rect) {
        self.children[0].arrange(
            t,
            Rect::new(
                f.min_x(),
                f.min_y(),
                f.size.width,
                (self.choice_height - 8.).max(0.),
            ),
            c,
        );
        self.children[1].arrange(
            t,
            Rect::new(
                f.min_x(),
                f.min_y() + self.choice_height,
                f.size.width,
                (f.size.height - self.choice_height).max(0.),
            ),
            c,
        );
        self.arranged = true;
    }
}
pub(super) fn playground(state: &StoryState) -> Vec<Content> {
    let model = state.extended.split;
    let split = SplitViewContent::try_new(SplitViewConstruction {
        config: SplitViewConfig {
            axis: Axis::Horizontal,
            ratio: model.ratio(),
            divider: model.divider,
            min_pane: model.min_pane,
            collapsed: model.collapsed,
        },
        slots: SplitViewSlotIds {
            leading: ComponentId(123001),
            trailing: ComponentId(123002),
            divider: DIVIDER,
        },
        leading: Content::new(ComponentId(123011), PaneBody("Master pane")),
        trailing: Content::new(ComponentId(123012), PaneBody("Detail pane")),
        // Two immutable leaves plus the three owned slots. The public budget
        // measures preparation work, not the number of nodes.
        budget: 5 * 5 * 512 + 2 * PreparedEffects::local_work(16).unwrap(),
    })
    .expect("original finite split panes");
    let constrained = Rc::new(Cell::new(false));
    vec![
        Content::new(
            ComponentId(123000),
            Viewport {
                children: vec![
                    Content::new(
                        ComponentId(123010),
                        Choices {
                            visible: constrained.clone(),
                            arranged: false,
                            stack: StackContent {
                                model: Stack {
                                    axis: Axis::Horizontal,
                                    gap: 8.,
                                },
                                children: vec![
                                    Content::new(ComponentId(123091), Button::new("Show master")),
                                    Content::new(ComponentId(123092), Button::new("Show detail")),
                                ],
                                flexible_child: None,
                            },
                        },
                    ),
                    Content::new(SPECIMEN, split),
                ],
                constrained,
                choice_height: 0.,
                arranged: false,
            },
        ),
        Content::new(ComponentId(123021), TextContent::new(help(model.ratio()))),
        Content::new(
            ComponentId(92003),
            TextContent::new(state.log.iter().cloned().collect::<Vec<_>>().join("\n")),
        ),
    ]
}
fn help(ratio: f32) -> String {
    format!(
        "Divider position: {:.0}% · drag, arrow keys, Home / End; double-click or Enter / Space resets; Escape restores the drag start.",
        ratio * 100.
    )
}
pub(super) fn sync(state: &mut StoryState, host: &mut ContentHost) {
    let Some(config) = host
        .get(SPECIMEN)
        .and_then(|n| n.component::<SplitViewContent>())
        .and_then(SplitViewContent::config)
    else {
        return;
    };
    let changed = state.extended.split.ratio() != config.ratio;
    state.extended.split.set_ratio(config.ratio);
    state.extended.split.collapsed = config.collapsed;
    if changed {
        state.record(format!("split: {:.0}%", config.ratio * 100.));
        host.get_mut(ComponentId(123021))
            .unwrap()
            .component_mut::<TextContent>()
            .unwrap()
            .text = help(config.ratio);
        host.get_mut(ComponentId(92003))
            .unwrap()
            .component_mut::<TextContent>()
            .unwrap()
            .text = state.log.iter().cloned().collect::<Vec<_>>().join("\n");
    }
}
pub(super) fn choose(host: &mut ContentHost, id: ComponentId) -> bool {
    let pane = match id.0 {
        123091 => Pane::Trailing,
        123092 => Pane::Leading,
        _ => return false,
    };
    let expected = host
        .split_view_expected(SPECIMEN)
        .expect("live original split");
    let mut config = expected.config;
    config.collapsed = Some(pane);
    // Configuration checks the whole mounted story, including its section bar.
    // This fixed story has fewer than 64 nodes; translate that node bound into
    // the configuration API's quadratic work units (not a 64-unit work budget).
    const STORY_NODE_BOUND: usize = 64;
    host.update_split_view(
        SPECIMEN,
        expected,
        config,
        STORY_NODE_BOUND * STORY_NODE_BOUND * 256,
    )
    .expect("original pane selection");
    true
}
#[derive(Debug)]
struct Variant(SplitView);
impl Component for Variant {
    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, a: Size) -> MeasuredChild {
        MeasuredChild::new(Size::new(a.width.min(620.), 78.))
    }
    fn paint(
        &mut self,
        s: &mut Scene,
        t: &mut dyn ButtonTextRenderer,
        f: Rect,
        c: Rect,
        theme: ContentTheme,
    ) {
        let (leading, divider, trailing) = self.0.layout(f, Axis::Horizontal);
        s.push_clip(c);
        paint::surface_at_scale(s, leading, theme.palette, true, t.backing_scale());
        tekes_ui_controls::extended::separator(s, divider, theme.palette, Axis::Vertical);
        paint::surface_at_scale(s, trailing, theme.palette, true, t.backing_scale());
        s.pop_clip();
    }
}
pub(super) fn variants() -> Vec<Content> {
    VARIANTS
        .into_iter()
        .enumerate()
        .flat_map(|(i, ratio)| {
            [
                Content::new(
                    ComponentId(123100 + i as u64 * 2),
                    TextContent::new(format!("Leading {:.0}%", ratio * 100.)),
                ),
                Content::new(
                    ComponentId(123101 + i as u64 * 2),
                    Variant(SplitView::new(ratio)),
                ),
            ]
        })
        .collect()
}
