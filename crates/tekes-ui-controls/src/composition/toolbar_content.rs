//! Original Content groups arranged in a toolbar's own leading/center/trailing
//! regions. No native window control, action, focus or popup state is copied.
use super::*;

/// Semantic placement within the receiving toolbar, not screen coordinates.
/// These defaults follow AppKit's regions/current HIG; PrimaryAction is not a
/// promise to reproduce SwiftUI's differently documented macOS placement.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ToolbarPlacement {
    #[default]
    Automatic,
    Navigation,
    Principal,
    Status,
    PrimaryAction,
    SecondaryAction,
}
impl ToolbarPlacement {
    fn section(self) -> usize {
        match self {
            Self::Navigation => 0,
            Self::PrimaryAction | Self::SecondaryAction => 2,
            _ => 1,
        }
    }
}

fn spacing(text: &mut dyn ButtonTextRenderer, token: &str) -> f32 {
    let scale = text.resolved_font_size(proposal::TextRole::UiBody)
        / proposal::TOOLBAR_REFERENCE_TITLE_SIZE;
    let scale = if scale.is_finite() && scale > 0. {
        scale
    } else {
        1.
    };
    proposal::metric("spacing", token).expect("existing toolbar spacing token") * scale
}

/// One logical group. Its outer Content ID owns mount/reorder identity; each
/// child retains its original control, semantic role and event ownership.
#[derive(Debug)]
pub struct ToolbarGroup {
    pub placement: ToolbarPlacement,
    pub visible: bool,
    pub children: Vec<Content>,
}
impl ToolbarGroup {
    pub fn new(placement: ToolbarPlacement, children: Vec<Content>) -> Self {
        Self {
            placement,
            visible: true,
            children,
        }
    }
}
impl Component for ToolbarGroup {
    fn curve_subtree_capability(&self) -> CurveSubtreeCapability { CurveSubtreeCapability::Recursive }

    fn action_transaction_compatibility(&self) -> ActionTransactionCompatibility {
        ActionTransactionCompatibility::StableHooks
    }
    fn prepare_action(&self, request: ComponentActionRequest<'_>) -> ActionPreparation {
        action_preparation::ignored_pointer(request)
    }
    fn active(&self) -> bool { self.visible }
    fn children(&self) -> &[Content] {
        &self.children
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.children)
    }
    fn measure(&mut self, text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        let gap = spacing(text, "space-tight");
        let mut size = Size::default();
        for child in self.children.iter_mut().filter(|c| c.active()) {
            let m = child.measure(text, available).size;
            size.width += m.width;
            size.height = size.height.max(m.height);
        }
        size.width += gap * self.children.len().saturating_sub(1) as f32;
        MeasuredChild::new(size)
    }
    fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        let gap = spacing(text, "space-tight");
        let measured: Vec<_> = self
            .children
            .iter_mut()
            .map(|c| c.measure(text, frame.size))
            .collect();
        let placements = layout_measured_children(
            frame,
            Axis::Horizontal,
            &measured,
            gap,
            proposal::layout::ROW,
            AlignmentOverride {
                cross_axis: Some(proposal::layout::CrossAxisAlignment::Center),
                ..Default::default()
            },
        );
        let natural_width = measured.iter().map(|m| m.size.width).sum::<f32>()
            + gap * self.children.len().saturating_sub(1) as f32;
        // Preserve the true ancestor viewport for ordinary focus decoration.
        // Only actual constrained overflow introduces a group viewport.
        let clip = if natural_width > frame.size.width {
            frame.intersection(&clip).unwrap_or_default()
        } else {
            clip
        };
        for (child, placed) in self.children.iter_mut().zip(placements) {
            child.arrange(text, placed.frame, clip);
        }
    }
}

/// Composable toolbar using the existing Content tree. Direct children without
/// ToolbarGroup metadata use Automatic placement. Parent allocation already
/// excludes native window controls; this component never queries/moves them.
///
/// Text and button labels stay on one line at every width and typography size.
/// Existing label-capable hosts use ellipsis; legacy string renderers clip the
/// unwrapped line. Full semantic names and original sources are preserved.
///
/// Group overflow is not synthesized from arbitrary Content. A ToolbarControl
/// child keeps its existing command overflow. ToolbarHost mounts this original
/// adapter for each registered window region; custom content remains in-row.
#[derive(Debug)]
pub struct ToolbarContent {
    pub children: Vec<Content>,
    paint_ground: bool,
}
impl ToolbarContent {
    pub fn new(children: Vec<Content>) -> Self {
        Self { children, paint_ground: true }
    }
    pub(super) fn hosted() -> Self {
        Self { children: vec![], paint_ground: false }
    }
    fn section(child: &Content) -> usize {
        child
            .component::<ToolbarGroup>()
            .map_or(1, |group| group.placement.section())
    }
}
impl Component for ToolbarContent {
    fn curve_subtree_capability(&self) -> CurveSubtreeCapability { CurveSubtreeCapability::Recursive }

    fn action_transaction_compatibility(&self) -> ActionTransactionCompatibility {
        ActionTransactionCompatibility::StableHooks
    }
    fn prepare_action(&self, request: ComponentActionRequest<'_>) -> ActionPreparation {
        action_preparation::ignored_pointer(request)
    }
    fn child_theme(&self, theme: ContentTheme) -> ContentTheme {
        theme.with_surface("toolbar").expect("generated Toolbar Surface")
    }
    fn children(&self) -> &[Content] {
        &self.children
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.children)
    }
    fn semantics(&self) -> ContentSemantics {
        ContentSemantics::new(ContentRole::Group, "Toolbar")
    }
    fn measure(&mut self, text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        for child in &mut self.children {
            child.toolbar_label_layouts();
        }
        let row = crate::toolbar::metrics(text, proposal::TextRole::UiBody).row;
        let gap = spacing(text, "space-regular");
        let mut widths = 0.;
        let mut counts = [0usize; 3];
        for child in self.children.iter_mut().filter(|c| c.active()) {
            counts[Self::section(child)] += 1;
            widths += child.measure(text, Size::new(available.width, row)).size.width;
        }
        let gaps = [spacing(text, "space-tight"), gap, gap];
        let sections = counts.iter().filter(|&&n| n > 0).count();
        let width = if sections == 0 { 0. } else {
            widths + gap * (2 + sections.saturating_sub(1)) as f32
                + (0..3).map(|i| gaps[i] * counts[i].saturating_sub(1) as f32).sum::<f32>()
        };
        MeasuredChild::new(Size::new(width.min(available.width), row))
    }
    fn paint(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        frame: Rect,
        clip: Rect,
        theme: ContentTheme,
    ) {
        if self.paint_ground {
            let row = crate::toolbar::metrics(text, proposal::TextRole::UiBody).row;
            crate::paint::toolbar_background(scene, frame, Some(clip), row, theme.palette);
        }
    }
    fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        for child in &mut self.children {
            child.toolbar_label_layouts();
        }
        let row = crate::toolbar::metrics(text, proposal::TextRole::UiBody).row;
        let gap = spacing(text, "space-regular");
        let pad = gap.min(frame.size.width * 0.5);
        let inner = Rect::new(
            frame.min_x() + pad,
            frame.min_y(),
            (frame.size.width - pad * 2.).max(0.),
            row,
        );
        fn minimum(node: &Content, text: &mut dyn ButtonTextRenderer) -> f32 {
            if !node.active() { return 0.; }
            if let Some(control) = node.component::<crate::ToolbarControl>() {
                return control.minimum_width(text);
            }
            let children = node.component.children();
            let gap = spacing(text, "space-tight");
            children.iter().map(|c|minimum(c,text)).sum::<f32>()
                + gap * children.len().saturating_sub(1) as f32
        }
        let mut entries = Vec::new();
        for (index, child) in self.children.iter_mut().enumerate().filter(|(_,c)|c.active()) {
            let natural = child.measure(text, inner.size).size.width;
            let floor = minimum(child, text).min(natural);
            entries.push((index, natural, floor));
        }
        let section_gaps = [spacing(text, "space-tight"), gap, gap];
        let mut counts = [0usize; 3];
        for (index, _, _) in &entries { counts[Self::section(&self.children[*index])] += 1; }
        let reserved = (0..3).map(|i| section_gaps[i] * counts[i].saturating_sub(1) as f32).sum::<f32>()
            + gap * counts.iter().filter(|&&n| n > 0).count().saturating_sub(1) as f32;
        // Reserve each command group's original More trigger before spending
        // width on titles/custom content. Placement remains local to this row.
        let budget = (inner.size.width - reserved).max(0.);
        let floors = entries.iter().map(|(_,_,m)|m).sum::<f32>();
        let extra = entries.iter().map(|(_,n,m)|n-m).sum::<f32>();
        let mut members: [Vec<(usize, f32)>; 3] = Default::default();
        for (index, natural, floor) in entries {
            let width = if budget < floors {
                if floors > 0. { floor * budget / floors } else { 0. }
            } else {
                floor + (natural-floor) * if extra > 0. { ((budget-floors)/extra).min(1.) } else { 0. }
            };
            members[Self::section(&self.children[index])].push((index, width));
        }
        let widths: [f32; 3] = std::array::from_fn(|i| {
            members[i].iter().map(|(_, w)| w).sum::<f32>()
                + section_gaps[i] * members[i].len().saturating_sub(1) as f32
        });
        let side_gaps = if members[1].is_empty() {
            usize::from(widths[0] > 0. && widths[2] > 0.)
        } else {
            usize::from(widths[0] > 0.) + usize::from(widths[2] > 0.)
        };
        let side_budget = (inner.size.width - gap * side_gaps as f32).max(0.);
        let side_total = widths[0] + widths[2];
        let factor = if side_total > side_budget {
            side_budget / side_total
        } else {
            1.
        };
        let left = widths[0] * factor;
        let right = widths[2] * factor;
        let center_start = inner.min_x() + left + if left > 0. { gap } else { 0. };
        let center_end =
            (inner.max_x() - right - if right > 0. { gap } else { 0. }).max(center_start);
        let center_width = widths[1].min((center_end - center_start).max(0.));
        let center_x = (inner.center().x - center_width * 0.5)
            .clamp(center_start, (center_end - center_width).max(center_start));
        let regions = [
            Rect::new(inner.min_x(), inner.min_y(), left, row),
            Rect::new(center_x, inner.min_y(), center_width, row),
            Rect::new(inner.max_x() - right, inner.min_y(), right, row),
        ];
        for (section, (items, region)) in members.iter().zip(regions).enumerate() {
            let gap = section_gaps[section];
            let spaces = (gap * items.len().saturating_sub(1) as f32).min(region.size.width);
            let natural = items.iter().map(|(_, w)| w).sum::<f32>();
            let factor = if natural > 0. {
                ((region.size.width - spaces) / natural).min(1.)
            } else {
                0.
            };
            let group_clip = if factor < 1. {
                region.intersection(&clip).unwrap_or_default()
            } else {
                clip
            };
            let mut x = region.min_x();
            for &(index, width) in items {
                let width = width * factor;
                self.children[index].arrange(
                    text,
                    Rect::new(x, region.min_y(), width, row),
                    group_clip,
                );
                x += width + gap;
            }
        }
    }
}
