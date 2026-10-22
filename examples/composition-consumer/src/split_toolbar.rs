//! Two real container contributions, using only public shared components.
//! Review assembly, not an AppHost implementation or copied interaction engine.
use tekes_ui_controls::{
    ButtonTextRenderer, ToolbarControl, ToolbarIconAction,
    composition::*,
    extended::{Axis, SplitView},
    layout::MeasuredChild,
    layout_view::SplitViewView,
};
use tekes_ui_scene::{ImageId, Point, Rect, Scene, Size};
use tekes_ui_tokens::Spacing;
pub const ROOT: ComponentId = ComponentId(20200);
pub const LEFT_BAR: ComponentId = ComponentId(20201);
pub const RIGHT_BAR: ComponentId = ComponentId(20202);
pub const NAV: ComponentId = ComponentId(20203);
pub const LEFT_ACTION: ComponentId = ComponentId(20204);

#[derive(Debug)]
pub struct SplitToolbar {
    pub model: SplitView,
    pub view: SplitViewView,
    pub row: Rect,
    pub held: bool,
    pub native_leading: f32,
    pub native_center: Option<f32>,
    children: Vec<Content>,
}
impl Default for SplitToolbar {
    fn default() -> Self {
        let left = ToolbarContent::new(vec![
            Content::new(
                ComponentId(20205),
                ToolbarGroup::new(
                    ToolbarPlacement::Navigation,
                    vec![Content::new(
                        ComponentId(20206),
                        TextContent::new("Left container"),
                    )],
                ),
            ),
            Content::new(
                ComponentId(20207),
                ToolbarGroup::new(
                    ToolbarPlacement::PrimaryAction,
                    vec![Content::new(LEFT_ACTION, ButtonContent::new("Action"))],
                ),
            ),
        ]);
        let mut nav = ToolbarControl::window();
        nav.set_icon_actions(vec![ToolbarIconAction {
            id: 1,
            image: ImageId(1),
            label: "Right container navigation".into(),
            enabled: true,
        }]);
        nav.set_action_help(
            1,
            Some("Navigation registered by the right container".into()),
        );
        let right = ToolbarContent::new(vec![
            Content::new(
                ComponentId(20208),
                ToolbarGroup::new(ToolbarPlacement::Navigation, vec![Content::new(NAV, nav)]),
            ),
            Content::new(
                ComponentId(20209),
                ToolbarGroup::new(
                    ToolbarPlacement::Principal,
                    vec![Content::new(
                        ComponentId(20210),
                        TextContent::new("Right container"),
                    )],
                ),
            ),
        ]);
        Self {
            model: SplitView::new(0.5),
            view: SplitViewView::new(Axis::Horizontal),
            row: Rect::default(),
            held: false,
            native_leading: 0.,
            native_center: None,
            children: vec![
                Content::new(LEFT_BAR, left),
                Content::new(RIGHT_BAR, right),
                Content::new(
                    ComponentId(20211),
                    TextContent::new("Drag the divider · Command + / − changes text size"),
                ),
                Content::new(
                    ComponentId(20212),
                    TextContent::new("The Navigation icon follows this container"),
                ),
            ],
        }
    }
}
// Only traverse the original toolbar types mounted by this fixture; all hit
// rectangles are provided by those controls, never estimated from labels.
fn toolbar_regions(node: &Content, regions: &mut Vec<Rect>) {
    let clip = node.visible().unwrap_or_default();
    if let Some(toolbar) = node.component::<ToolbarControl>() {
        regions.extend(toolbar.pointer_regions(node.frame(), clip));
    } else if let Some(button) = node.component::<ButtonContent>() {
        regions.extend(button.pointer_regions(node.frame(), clip));
    } else if let Some(toolbar) = node.component::<ToolbarContent>() {
        for child in &toolbar.children {
            toolbar_regions(child, regions);
        }
    } else if let Some(group) = node.component::<ToolbarGroup>() {
        for child in &group.children {
            toolbar_regions(child, regions);
        }
    }
}
impl Component for SplitToolbar {
    fn children(&self) -> &[Content] {
        &self.children
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.children)
    }
    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        MeasuredChild::new(available)
    }
    fn semantics(&self) -> ContentSemantics {
        ContentSemantics::new(ContentRole::Group, "Split containers")
    }
    fn focusable(&self) -> bool {
        true
    }
    fn pointer_regions(&self, _: Rect, _: Rect) -> Vec<Rect> {
        self.view
            .resize_regions(&self.model)
            .into_iter()
            .map(|(r, _)| r)
            .collect()
    }
    fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        let height = self.children[0].measure(text, frame.size).size.height;
        let y = self
            .native_center
            .map_or(frame.min_y(), |center| center - height * 0.5);
        self.row = Rect::new(frame.min_x(), y, frame.size.width, height);
        let split = self.view.arrange(&mut self.model, frame, Some(clip));
        let panes = [split.leading.frame(), split.trailing.frame()];
        for (i, pane) in panes.into_iter().enumerate() {
            let start = pane.min_x().max(frame.min_x() + self.native_leading);
            let bar = Rect::new(start, y, (pane.max_x() - start).max(0.), height);
            self.children[i].arrange(text, bar, bar.intersection(&clip).unwrap_or_default());
            let pad = Spacing::Regular.value();
            let body = Rect::new(
                pane.min_x() + pad,
                self.row.max_y() + pad,
                (pane.size.width - pad * 2.).max(0.),
                (pane.max_y() - self.row.max_y() - pad).max(0.),
            );
            self.children[i + 2].arrange(text, body, body.intersection(&clip).unwrap_or_default());
        }
        let mut controls = Vec::new();
        // The platform's genuine leading-control exclusion is another occupied
        // top-row region; do not draw a seam through native traffic lights.
        if self.native_leading > 0. {
            controls.push(Rect::new(frame.min_x(), y, self.native_leading, height));
        }

        for child in &self.children[..2] {
            toolbar_regions(child, &mut controls);
        }
        self.view
            .set_toolbar_region(Some(self.row), &controls, text);
    }
    fn paint(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        _: Rect,
        _: Rect,
        theme: ContentTheme,
    ) {
        // Existing shared painter owns both the content line and top short mark.
        // Exclude any native-only top padding when the text row is very small.
        let divider = self.view.layout().divider.frame();
        let start = divider.min_y().max(self.row.min_y());
        self.view.paint_presented_divider(
            scene,
            &self.model,
            Rect::new(
                divider.min_x(),
                start,
                divider.size.width,
                (divider.max_y() - start).max(0.),
            ),
            theme.palette,
            text.backing_scale(),
        );
    }
    fn event(&mut self, event: &ContentEvent) -> ContentResponse {
        let changed = match event {
            ContentEvent::PointerDown(p) => {
                self.held = self.view.pointer_down(*p, &mut self.model);
                return ContentResponse {
                    consumed: self.held,
                    changed: self.held,
                    capture_pointer: self.held,
                    ..ContentResponse::default()
                };
            }
            ContentEvent::PointerMove(p) => {
                let changed = self.view.pointer_move(*p, &mut self.model);
                // Keep the acquired original gesture even at a clamped edge;
                // unchanged geometry must never bubble into native window drag.
                return ContentResponse {
                    consumed: self.held,
                    changed,
                    ..Default::default()
                };
            }
            ContentEvent::PointerUp(_) => {
                self.held = false;
                self.view.pointer_up(&mut self.model)
            }
            ContentEvent::Key { key, down: true } => self.view.key(*key, &mut self.model),
            ContentEvent::Cancel => {
                self.held = false;
                self.view.cancel(&mut self.model)
            }
            _ => false,
        };
        if changed {
            ContentResponse::consumed(true)
        } else {
            ContentResponse::default()
        }
    }
}
pub fn build() -> Result<ContentHost, CompositionError> {
    let mut host = ContentHost::new(Content::new(ROOT, SplitToolbar::default()))?;
    host.set_action_events(true);
    Ok(host)
}
// Public query used by native receipts and independent geometry regressions.
pub fn divider_center(host: &ContentHost) -> Point {
    host.get(ROOT)
        .unwrap()
        .component::<SplitToolbar>()
        .unwrap()
        .view
        .layout()
        .divider
        .frame()
        .center()
}
