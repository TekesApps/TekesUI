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

fn snap_to_scale(value: f32, backing_scale: f32) -> f32 {
    let scale = if backing_scale.is_finite() && backing_scale > 0. {
        backing_scale
    } else {
        1.
    };
    (value * scale).round() / scale
}

fn ceil_to_scale(value: f32, backing_scale: f32) -> f32 {
    let scale = if backing_scale.is_finite() && backing_scale > 0. {
        backing_scale
    } else {
        1.
    };
    (value * scale).ceil() / scale
}

/// Visual chrome presentation for a [`ToolbarGroup`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ToolbarGroupChrome {
    /// Original unframed flat toolbar spacing (`space-tight` gap).
    #[default]
    Plain,
    /// Shared rounded capsule container with zero internal gap and optional
    /// 1px hairline dividers between adjacent visible segments.
    Capsule { divided: bool },
}

/// One logical group. Its outer Content ID owns mount/reorder identity; each
/// child retains its original control, semantic role and event ownership.
#[derive(Debug)]
pub struct ToolbarGroup {
    pub placement: ToolbarPlacement,
    pub visible: bool,
    pub children: Vec<Content>,
    axis: Axis,
    radio: bool,
    reveal_passthrough: bool,
    chrome: ToolbarGroupChrome,
    label: Option<String>,
    help: Option<String>,
    capsule_frame: Option<Rect>,
}
impl ToolbarGroup {
    pub fn new(placement: ToolbarPlacement, children: Vec<Content>) -> Self {
        Self {
            placement,
            visible: true,
            children,
            axis: Axis::Horizontal,
            radio: false,
            reveal_passthrough: false,
            chrome: ToolbarGroupChrome::Plain,
            label: None,
            help: None,
            capsule_frame: None,
        }
    }
    pub fn with_chrome(mut self, chrome: ToolbarGroupChrome) -> Self {
        self.set_chrome(chrome);
        self
    }
    pub fn set_chrome(&mut self, chrome: ToolbarGroupChrome) {
        if self.chrome != chrome {
            self.chrome = chrome;
            if chrome == ToolbarGroupChrome::Plain {
                self.capsule_frame = None;
                Self::clear_segment_specs(&mut self.children);
            }
        }
    }
    pub fn chrome(&self) -> ToolbarGroupChrome {
        self.chrome
    }
    pub fn with_capsule(mut self, capsule: bool) -> Self {
        let chrome = if capsule {
            match self.chrome {
                ToolbarGroupChrome::Capsule { divided } => ToolbarGroupChrome::Capsule { divided },
                ToolbarGroupChrome::Plain => ToolbarGroupChrome::Capsule { divided: true },
            }
        } else {
            ToolbarGroupChrome::Plain
        };
        self.set_chrome(chrome);
        self
    }
    pub fn with_divided(mut self, divided: bool) -> Self {
        self.set_chrome(ToolbarGroupChrome::Capsule { divided });
        self
    }
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.set_label(Some(label.into()));
        self
    }
    pub fn set_label(&mut self, label: Option<String>) {
        self.label = label.filter(|text| !text.trim().is_empty());
    }
    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }
    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.set_help(Some(help.into()));
        self
    }
    pub fn set_help(&mut self, help: Option<String>) {
        self.help = help.filter(|text| !text.trim().is_empty());
    }
    pub fn help_text(&self) -> Option<&str> {
        self.help.as_deref()
    }
    pub(crate) fn is_capsule(&self) -> bool {
        matches!(self.chrome, ToolbarGroupChrome::Capsule { .. })
    }
    #[cfg(test)]
    pub(crate) fn capsule_frame(&self) -> Option<Rect> {
        self.capsule_frame
    }
    fn clear_segment_specs(children: &mut [Content]) {
        for child in children {
            if let Some(button) = child.component_mut::<Button>() {
                button.set_capsule_segment(None);
            } else if let Some(menu) = child.component_mut::<MenuContent>() {
                menu.trigger_mut().set_capsule_segment(None);
            }
        }
    }
    fn sync_segment_specs(&mut self, text: &mut dyn ButtonTextRenderer, radius: f32) {
        if !self.is_capsule() {
            Self::clear_segment_specs(&mut self.children);
            return;
        }
        let inner_inset = spacing(text, "space-regular");
        let outer_extra = spacing(text, "space-micro");
        let active_indices: Vec<usize> = self
            .children
            .iter()
            .enumerate()
            .filter(|(_, child)| child.active())
            .map(|(idx, _)| idx)
            .collect();
        let active_len = active_indices.len();
        for (idx, child) in self.children.iter_mut().enumerate() {
            let Some(pos) = active_indices
                .iter()
                .position(|&active_idx| active_idx == idx)
            else {
                if let Some(button) = child.component_mut::<Button>() {
                    button.set_capsule_segment(None);
                } else if let Some(menu) = child.component_mut::<MenuContent>() {
                    menu.trigger_mut().set_capsule_segment(None);
                }
                continue;
            };
            let first = pos == 0;
            let last = pos + 1 == active_len;
            let radii = if active_len == 1 {
                tekes_ui_scene::CornerRadii::uniform(radius)
            } else if self.axis == Axis::Horizontal {
                tekes_ui_scene::CornerRadii {
                    top_left: if first { radius } else { 0. },
                    top_right: if last { radius } else { 0. },
                    bottom_right: if last { radius } else { 0. },
                    bottom_left: if first { radius } else { 0. },
                }
            } else {
                tekes_ui_scene::CornerRadii {
                    top_left: if first { radius } else { 0. },
                    top_right: if first { radius } else { 0. },
                    bottom_right: if last { radius } else { 0. },
                    bottom_left: if last { radius } else { 0. },
                }
            };
            let (leading_inset, trailing_inset) = if self.axis == Axis::Horizontal {
                (
                    inner_inset + if first { outer_extra } else { 0. },
                    inner_inset + if last { outer_extra } else { 0. },
                )
            } else {
                (inner_inset, inner_inset)
            };
            let spec = Some(super::button::CapsuleSegmentSpec {
                radii,
                leading_inset,
                trailing_inset,
            });
            if let Some(button) = child.component_mut::<Button>() {
                button.set_capsule_segment(spec);
            } else if let Some(menu) = child.component_mut::<MenuContent>() {
                menu.trigger_mut().set_capsule_segment(spec);
            }
        }
    }
    pub(super) fn paint_boundary(
        &self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        clip: Rect,
        theme: ContentTheme,
    ) {
        let ToolbarGroupChrome::Capsule { divided } = self.chrome else {
            return;
        };
        let Some(rect) = self.capsule_frame else {
            return;
        };
        let pill = proposal::metric("radius", "radius-pill").unwrap_or(999.0);
        let radius = (rect.size.width.min(rect.size.height) * 0.5).min(pill);
        let mut dividers = Vec::new();
        if divided {
            let segment_active = |child: &Content| {
                child
                    .component::<Button>()
                    .is_some_and(Button::is_visually_active_in_capsule)
                    || child.component::<MenuContent>().is_some_and(|menu| {
                        menu.trigger().is_visually_active_in_capsule() || menu.is_open()
                    })
            };
            let active: Vec<_> = self.children.iter().filter(|c| c.active()).collect();
            for pair in active.windows(2) {
                let [left, right] = pair else { continue };
                if !segment_active(left) && !segment_active(right) {
                    dividers.push(if self.axis == Axis::Horizontal {
                        right.frame().min_x()
                    } else {
                        right.frame().min_y()
                    });
                }
            }
        }
        crate::paint::toolbar_capsule_boundary(
            scene,
            rect,
            Some(clip),
            theme.palette,
            text.backing_scale(),
            radius,
            self.axis == Axis::Horizontal,
            &dividers,
        );
    }
    /// Declare one externally controlled choice group. The group never owns a
    /// second selected ID; consumers project their committed source selection.
    pub fn set_radio(&mut self, radio: bool) {
        self.radio = radio;
    }
    pub fn is_radio(&self) -> bool {
        self.radio
    }
    /// Project an original Button/Menu selection. Radio groups clear siblings;
    /// non-radio groups preserve other independent checked states.
    pub fn set_selected_id(&mut self, selected: Option<ComponentId>) -> bool {
        let mut changed = false;
        for child in &mut self.children {
            let id = child.id;
            if self.radio || selected == Some(id) {
                let next = selected == Some(id);
                if let Some(button) = child.component_mut::<Button>() {
                    changed |= button.button.state.selected != next;
                    button.button.state.selected = next;
                } else if let Some(menu) = child.component_mut::<MenuContent>() {
                    changed |= menu.trigger.button.state.selected != next;
                    menu.trigger.button.state.selected = next;
                }
            }
        }
        changed
    }
    pub fn set_axis(&mut self, axis: Axis) {
        self.axis = axis;
        set_content_axis(&mut self.children, axis);
    }
}

fn set_content_axis(children: &mut [Content], axis: Axis) {
    for child in children {
        if let Some(control) = child.component_mut::<crate::ToolbarControl>() {
            control.set_axis(axis);
        }
        if let Some(group) = child.component_mut::<ToolbarGroup>() {
            group.axis = axis;
        }
        if let Some(descendants) = child.component.children_mut() {
            set_content_axis(descendants, axis);
        }
    }
}
fn measure_plain_toolbar_child(
    child: &mut Content,
    text: &mut dyn ButtonTextRenderer,
    available: Size,
    axis: Axis,
    box_size: f32,
) -> MeasuredChild {
    let mut measured = child.measure(text, available);
    if child.component::<Button>().is_some() {
        match axis {
            Axis::Horizontal => {
                measured.size.height = measured.size.height.max(box_size).min(available.height);
            }
            Axis::Vertical => {
                measured.size.width = measured.size.width.max(box_size).min(available.width);
            }
        }
    }
    measured
}

impl Component for ToolbarGroup {
    fn host_reveal_passthrough(&self) -> bool {
        self.reveal_passthrough
    }
    fn curve_subtree_capability(&self) -> CurveSubtreeCapability {
        CurveSubtreeCapability::Recursive
    }
    fn ignores_pointer_actions(&self) -> bool {
        true
    }
    fn semantics(&self) -> ContentSemantics {
        let name = self
            .label
            .as_deref()
            .unwrap_or(if self.radio { "Toolbar choices" } else { "" });
        ContentSemantics::new(
            if self.radio {
                ContentRole::RadioGroup
            } else {
                ContentRole::Group
            },
            name,
        )
    }
    fn active(&self) -> bool {
        self.visible
    }
    fn children(&self) -> &[Content] {
        &self.children
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.children)
    }
    fn paint(
        &mut self,
        scene: &mut Scene,
        _: &mut dyn ButtonTextRenderer,
        _: Rect,
        clip: Rect,
        theme: ContentTheme,
    ) {
        if self.is_capsule()
            && let Some(rect) = self.capsule_frame
        {
            let pill = proposal::metric("radius", "radius-pill").unwrap_or(999.0);
            let radius = (rect.size.width.min(rect.size.height) * 0.5).min(pill);
            crate::paint::toolbar_capsule_background(
                scene,
                rect,
                Some(clip),
                theme.palette,
                radius,
            );
        }
    }
    fn measure(&mut self, text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        if !self.is_capsule() {
            Self::clear_segment_specs(&mut self.children);
            self.capsule_frame = None;
            let gap = spacing(text, "space-tight");
            let box_size = crate::toolbar::metrics(text, proposal::TextRole::UiBody).box_size;
            let mut size = Size::default();
            let mut count: usize = 0;
            for child in self.children.iter_mut().filter(|c| c.active()) {
                count += 1;
                let m =
                    measure_plain_toolbar_child(child, text, available, self.axis, box_size).size;
                if self.axis == Axis::Horizontal {
                    size.width += m.width;
                    size.height = size.height.max(m.height);
                } else {
                    size.width = size.width.max(m.width);
                    size.height += m.height;
                }
            }
            if self.axis == Axis::Horizontal {
                size.width += gap * count.saturating_sub(1) as f32;
            } else {
                size.height += gap * count.saturating_sub(1) as f32;
            }
            return MeasuredChild::new(size);
        }
        let box_size = crate::toolbar::metrics(text, proposal::TextRole::UiBody).box_size;
        let pill = proposal::metric("radius", "radius-pill").unwrap_or(999.0);
        self.sync_segment_specs(text, (box_size * 0.5).min(pill));
        let mut size = Size::default();
        let mut count: usize = 0;
        let scale = text.backing_scale();
        let child_available = if self.axis == Axis::Horizontal {
            Size::new(available.width, available.height.max(box_size))
        } else {
            Size::new(available.width.max(box_size), available.height)
        };
        for child in self.children.iter_mut().filter(|c| c.active()) {
            count += 1;
            let m = child.measure(text, child_available).size;
            if self.axis == Axis::Horizontal {
                size.width += ceil_to_scale(m.width, scale);
                size.height = size.height.max(m.height);
            } else {
                size.width = size.width.max(m.width);
                size.height += ceil_to_scale(m.height, scale);
            }
        }
        if count > 0 {
            if self.axis == Axis::Horizontal {
                size.height = size.height.max(box_size);
            } else {
                size.width = size.width.max(box_size);
            }
        }
        MeasuredChild::new(size)
    }
    fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        if !self.is_capsule() {
            Self::clear_segment_specs(&mut self.children);
            self.capsule_frame = None;
            let gap = spacing(text, "space-tight");
            let box_size = crate::toolbar::metrics(text, proposal::TextRole::UiBody).box_size;
            let mut children: Vec<_> = self.children.iter_mut().filter(|c| c.active()).collect();
            let measured: Vec<_> = children
                .iter_mut()
                .map(|c| measure_plain_toolbar_child(c, text, frame.size, self.axis, box_size))
                .collect();
            let placements = layout_measured_children(
                frame,
                self.axis,
                &measured,
                gap,
                if self.axis == Axis::Horizontal {
                    proposal::layout::ROW
                } else {
                    proposal::layout::COLUMN
                },
                AlignmentOverride {
                    cross_axis: Some(proposal::layout::CrossAxisAlignment::Center),
                    ..Default::default()
                },
            );
            let natural_main = if self.axis == Axis::Horizontal {
                measured.iter().map(|m| m.size.width).sum::<f32>()
                    + gap * children.len().saturating_sub(1) as f32
            } else {
                measured.iter().map(|m| m.size.height).sum::<f32>()
                    + gap * children.len().saturating_sub(1) as f32
            };
            // Preserve the true ancestor viewport for ordinary focus decoration.
            // Only actual constrained overflow introduces a group viewport.
            let available_main = if self.axis == Axis::Horizontal {
                frame.size.width
            } else {
                frame.size.height
            };
            // Only an unclipped group is a translation-only ancestor. Overflow
            // remains unavailable to the strict reveal planner.
            self.reveal_passthrough = natural_main <= available_main;
            let clip = if natural_main > available_main {
                frame.intersection(&clip).unwrap_or_default()
            } else {
                clip
            };
            for (child, placed) in children.into_iter().zip(placements) {
                child.arrange(text, placed.frame, clip);
            }
            return;
        }

        let box_size = crate::toolbar::metrics(text, proposal::TextRole::UiBody).box_size;
        let pill = proposal::metric("radius", "radius-pill").unwrap_or(999.0);
        self.sync_segment_specs(text, (box_size * 0.5).min(pill));

        let active_indices: Vec<usize> = self
            .children
            .iter()
            .enumerate()
            .filter(|(_, c)| c.active())
            .map(|(i, _)| i)
            .collect();
        if active_indices.is_empty() {
            self.capsule_frame = None;
            self.reveal_passthrough = true;
            return;
        }
        let child_available = if self.axis == Axis::Horizontal {
            Size::new(frame.size.width, frame.size.height.max(box_size))
        } else {
            Size::new(frame.size.width.max(box_size), frame.size.height)
        };
        let measured: Vec<Size> = active_indices
            .iter()
            .map(|&idx| self.children[idx].measure(text, child_available).size)
            .collect();
        let scale = text.backing_scale();
        let seg_mains: Vec<f32> = measured
            .iter()
            .map(|m| {
                if self.axis == Axis::Horizontal {
                    ceil_to_scale(m.width, scale)
                } else {
                    ceil_to_scale(m.height, scale)
                }
            })
            .collect();
        let (natural_main, max_child_cross, available_main, available_cross) =
            if self.axis == Axis::Horizontal {
                (
                    seg_mains.iter().sum::<f32>(),
                    measured.iter().map(|m| m.height).fold(0.0f32, f32::max),
                    frame.size.width.max(0.),
                    frame.size.height.max(0.),
                )
            } else {
                (
                    seg_mains.iter().sum::<f32>(),
                    measured.iter().map(|m| m.width).fold(0.0f32, f32::max),
                    frame.size.height.max(0.),
                    frame.size.width.max(0.),
                )
            };
        let capsule_cross = box_size.max(max_child_cross).min(available_cross);
        let compressed = natural_main > available_main && natural_main > 0.;
        let factor = if compressed {
            available_main / natural_main
        } else {
            1.
        };
        let mut cursor = 0.0f32;
        let mut segment_frames = Vec::with_capacity(measured.len());
        let capsule_rect = if self.axis == Axis::Horizontal {
            let cap_x = snap_to_scale(frame.min_x(), scale);
            let cap_y = snap_to_scale(frame.center().y - capsule_cross * 0.5, scale);
            let cap_bottom = snap_to_scale(frame.center().y + capsule_cross * 0.5, scale);
            let cap_h = (cap_bottom - cap_y).max(0.);
            let mut prev_x = cap_x;
            for &seg_w in &seg_mains {
                if compressed {
                    cursor += seg_w * factor;
                    let end = cap_x + snap_to_scale(cursor, scale);
                    segment_frames.push(Rect::new(prev_x, cap_y, (end - prev_x).max(0.), cap_h));
                    prev_x = end;
                } else {
                    cursor += seg_w;
                    let end = snap_to_scale(prev_x + seg_w, scale);
                    segment_frames.push(Rect::new(prev_x, cap_y, (end - prev_x).max(0.), cap_h));
                    prev_x = end;
                }
            }
            let total_w = (prev_x - cap_x).max(0.);
            Rect::new(cap_x, cap_y, total_w, cap_h)
        } else {
            let cap_x = snap_to_scale(frame.center().x - capsule_cross * 0.5, scale);
            let cap_right = snap_to_scale(frame.center().x + capsule_cross * 0.5, scale);
            let cap_w = (cap_right - cap_x).max(0.);
            let cap_y = snap_to_scale(frame.min_y(), scale);
            let mut prev_y = cap_y;
            for &seg_h in &seg_mains {
                if compressed {
                    cursor += seg_h * factor;
                    let end = cap_y + snap_to_scale(cursor, scale);
                    segment_frames.push(Rect::new(cap_x, prev_y, cap_w, (end - prev_y).max(0.)));
                    prev_y = end;
                } else {
                    cursor += seg_h;
                    let end = snap_to_scale(prev_y + seg_h, scale);
                    segment_frames.push(Rect::new(cap_x, prev_y, cap_w, (end - prev_y).max(0.)));
                    prev_y = end;
                }
            }
            let total_h = (prev_y - cap_y).max(0.);
            Rect::new(cap_x, cap_y, cap_w, total_h)
        };
        let radius = (capsule_rect.size.width.min(capsule_rect.size.height) * 0.5).min(pill);
        self.sync_segment_specs(text, radius);
        self.capsule_frame =
            (capsule_rect.size.width > 0. && capsule_rect.size.height > 0.).then_some(capsule_rect);
        self.reveal_passthrough = natural_main <= available_main;
        let child_clip = if natural_main > available_main {
            frame.intersection(&clip).unwrap_or_default()
        } else {
            clip
        };
        for (idx, seg_frame) in active_indices.into_iter().zip(segment_frames) {
            self.children[idx].arrange(text, seg_frame, child_clip);
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
    pub(super) horizontal_inset: Option<f32>,
    pub children: Vec<Content>,
    paint_ground: bool,
    reveal_passthrough: bool,
    axis: Axis,
}
impl ToolbarContent {
    pub fn new(children: Vec<Content>) -> Self {
        Self {
            children,
            horizontal_inset: None,
            paint_ground: true,
            reveal_passthrough: false,
            axis: Axis::Horizontal,
        }
    }
    pub(super) fn hosted() -> Self {
        Self {
            children: vec![],
            horizontal_inset: None,
            paint_ground: false,
            reveal_passthrough: false,
            axis: Axis::Horizontal,
        }
    }
    pub(super) fn set_axis(&mut self, axis: Axis) {
        if self.axis != axis {
            self.axis = axis;
            set_content_axis(&mut self.children, axis);
        }
    }
    fn section(child: &Content) -> usize {
        child
            .component::<ToolbarGroup>()
            .map_or(1, |group| group.placement.section())
    }
}
impl Component for ToolbarContent {
    fn host_reveal_passthrough(&self) -> bool {
        self.reveal_passthrough
    }
    fn curve_subtree_capability(&self) -> CurveSubtreeCapability {
        CurveSubtreeCapability::Recursive
    }
    fn ignores_pointer_actions(&self) -> bool {
        true
    }
    fn child_theme(&self, theme: ContentTheme) -> ContentTheme {
        theme
            .with_surface("toolbar")
            .expect("generated Toolbar Surface")
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
        if self.axis == Axis::Vertical {
            let gap = spacing(text, "space-regular");
            let mut counts = [0usize; 3];
            let mut heights = 0.0f32;
            let mut width = 0.0f32;
            for child in self.children.iter_mut().filter(|child| child.active()) {
                counts[Self::section(child)] += 1;
                let size = child.measure(text, available).size;
                heights += size.height;
                width = width.max(size.width);
            }
            let gaps = [spacing(text, "space-tight"), gap, gap];
            let sections = counts.iter().filter(|&&count| count > 0).count();
            let default_inset = crate::toolbar::metrics(text, proposal::TextRole::UiBody).inset;
            if sections > 0 {
                heights += self.horizontal_inset.unwrap_or(default_inset) * 2.;
            }
            heights += counts
                .iter()
                .enumerate()
                .map(|(i, count)| gaps[i] * count.saturating_sub(1) as f32)
                .sum::<f32>();
            if sections > 1 {
                heights += gap * (sections - 1) as f32;
            }
            return MeasuredChild::new(Size::new(
                width.min(available.width),
                heights.min(available.height),
            ));
        }
        let metrics = crate::toolbar::metrics(text, proposal::TextRole::UiBody);
        let row = metrics.row;
        let default_inset = metrics.inset;
        let gap = spacing(text, "space-regular");
        let mut widths = 0.;
        let mut counts = [0usize; 3];
        for child in self.children.iter_mut().filter(|c| c.active()) {
            counts[Self::section(child)] += 1;
            widths += child
                .measure(text, Size::new(available.width, row))
                .size
                .width;
        }
        let gaps = [spacing(text, "space-tight"), gap, gap];
        let sections = counts.iter().filter(|&&n| n > 0).count();
        let width = if sections == 0 {
            0.
        } else {
            widths
                + self.horizontal_inset.unwrap_or(default_inset) * 2.
                + gap * sections.saturating_sub(1) as f32
                + (0..3)
                    .map(|i| gaps[i] * counts[i].saturating_sub(1) as f32)
                    .sum::<f32>()
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
            crate::paint::toolbar_background(
                scene,
                frame,
                Some(clip),
                if self.axis == Axis::Horizontal {
                    row
                } else {
                    frame.size.height
                },
                theme.palette,
            );
        }
    }
    fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        for child in &mut self.children {
            child.toolbar_label_layouts();
        }
        self.reveal_passthrough = false;
        if self.axis == Axis::Vertical {
            self.arrange_vertical(text, frame, clip);
            return;
        }
        let metrics = crate::toolbar::metrics(text, proposal::TextRole::UiBody);
        let row = metrics.row;
        let gap = spacing(text, "space-regular");
        let pad = self
            .horizontal_inset
            .unwrap_or(metrics.inset)
            .min(frame.size.width * 0.5);
        let inner = Rect::new(
            frame.min_x() + pad,
            frame.min_y(),
            (frame.size.width - pad * 2.).max(0.),
            row,
        );
        fn minimum(node: &Content, text: &mut dyn ButtonTextRenderer) -> f32 {
            if !node.active() {
                return 0.;
            }
            if let Some(control) = node.component::<crate::ToolbarControl>() {
                return control.minimum_width(text);
            }
            let gap = if node
                .component::<ToolbarGroup>()
                .is_some_and(ToolbarGroup::is_capsule)
            {
                0.
            } else {
                spacing(text, "space-tight")
            };
            let mut count: usize = 0;
            let width: f32 = node
                .component
                .children()
                .iter()
                .filter(|c| c.active())
                .map(|child| {
                    count += 1;
                    minimum(child, text)
                })
                .sum();
            width + gap * count.saturating_sub(1) as f32
        }
        let mut entries = Vec::new();
        for (index, child) in self
            .children
            .iter_mut()
            .enumerate()
            .filter(|(_, c)| c.active())
        {
            let natural = child.measure(text, inner.size).size.width;
            let floor = minimum(child, text).min(natural);
            entries.push((index, natural, floor));
        }
        let section_gaps = [spacing(text, "space-tight"), gap, gap];
        let mut counts = [0usize; 3];
        for (index, _, _) in &entries {
            counts[Self::section(&self.children[*index])] += 1;
        }
        let reserved = (0..3)
            .map(|i| section_gaps[i] * counts[i].saturating_sub(1) as f32)
            .sum::<f32>()
            + gap * counts.iter().filter(|&&n| n > 0).count().saturating_sub(1) as f32;
        // Reserve each command group's original More trigger before spending
        // width on titles/custom content. Placement remains local to this row.
        let budget = (inner.size.width - reserved).max(0.);
        let floors = entries.iter().map(|(_, _, m)| m).sum::<f32>();
        let extra = entries.iter().map(|(_, n, m)| n - m).sum::<f32>();
        let mut members: [Vec<(usize, f32)>; 3] = Default::default();
        for (index, natural, floor) in entries {
            let width = if budget < floors {
                if floors > 0. {
                    floor * budget / floors
                } else {
                    0.
                }
            } else {
                floor
                    + (natural - floor)
                        * if extra > 0. {
                            ((budget - floors) / extra).min(1.)
                        } else {
                            0.
                        }
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
        self.reveal_passthrough = true;
        for (section, (items, region)) in members.iter().zip(regions).enumerate() {
            let gap = section_gaps[section];
            let spaces = (gap * items.len().saturating_sub(1) as f32).min(region.size.width);
            let natural = items.iter().map(|(_, w)| w).sum::<f32>();
            let factor = if natural > 0. {
                ((region.size.width - spaces) / natural).min(1.)
            } else {
                0.
            };
            self.reveal_passthrough &= factor >= 1. || items.is_empty();
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

impl ToolbarContent {
    fn arrange_vertical(&mut self, text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        let gap = spacing(text, "space-regular");
        let default_inset = crate::toolbar::metrics(text, proposal::TextRole::UiBody).inset;
        let pad = self
            .horizontal_inset
            .unwrap_or(default_inset)
            .min(frame.size.height * 0.5);
        let inner = Rect::new(
            frame.min_x(),
            frame.min_y() + pad,
            frame.size.width,
            (frame.size.height - pad * 2.).max(0.),
        );
        let section_gaps = [spacing(text, "space-tight"), gap, gap];
        fn minimum(node: &Content, text: &mut dyn ButtonTextRenderer) -> f32 {
            if !node.active() {
                return 0.;
            }
            if let Some(control) = node.component::<crate::ToolbarControl>() {
                return control.minimum_main(text);
            }
            let children = node.component.children();
            let child_gap = if node
                .component::<ToolbarGroup>()
                .is_some_and(ToolbarGroup::is_capsule)
            {
                0.
            } else {
                spacing(text, "space-tight")
            };
            let mut active_count: usize = 0;
            let total: f32 = children
                .iter()
                .filter(|c| c.active())
                .map(|child| {
                    active_count += 1;
                    minimum(child, text)
                })
                .sum();
            total + child_gap * active_count.saturating_sub(1) as f32
        }
        let mut entries = Vec::new();
        for (index, child) in self
            .children
            .iter_mut()
            .enumerate()
            .filter(|(_, c)| c.active())
        {
            let natural = child.measure(text, inner.size).size.height;
            entries.push((index, natural, minimum(child, text).min(natural)));
        }
        let mut counts = [0usize; 3];
        for (index, _, _) in &entries {
            counts[Self::section(&self.children[*index])] += 1;
        }
        let reserved = counts
            .iter()
            .enumerate()
            .map(|(i, count)| section_gaps[i] * count.saturating_sub(1) as f32)
            .sum::<f32>()
            + gap
                * counts
                    .iter()
                    .filter(|&&count| count > 0)
                    .count()
                    .saturating_sub(1) as f32;
        let budget = (inner.size.height - reserved).max(0.);
        let floors = entries.iter().map(|(_, _, floor)| floor).sum::<f32>();
        let extra = entries
            .iter()
            .map(|(_, natural, floor)| natural - floor)
            .sum::<f32>();
        let mut members: [Vec<(usize, f32)>; 3] = Default::default();
        for (index, natural, floor) in entries {
            let height = if budget < floors {
                if floors > 0. {
                    floor * budget / floors
                } else {
                    0.
                }
            } else {
                floor
                    + (natural - floor)
                        * if extra > 0. {
                            ((budget - floors) / extra).min(1.)
                        } else {
                            0.
                        }
            };
            members[Self::section(&self.children[index])].push((index, height));
        }
        let heights: [f32; 3] = std::array::from_fn(|i| {
            members[i].iter().map(|(_, h)| h).sum::<f32>()
                + section_gaps[i] * members[i].len().saturating_sub(1) as f32
        });
        let side_gaps = if members[1].is_empty() {
            usize::from(heights[0] > 0. && heights[2] > 0.)
        } else {
            usize::from(heights[0] > 0.) + usize::from(heights[2] > 0.)
        };
        let side_budget = (inner.size.height - gap * side_gaps as f32).max(0.);
        let side_total = heights[0] + heights[2];
        let factor = if side_total > side_budget && side_total > 0. {
            side_budget / side_total
        } else {
            1.
        };
        let top = heights[0] * factor;
        let bottom = heights[2] * factor;
        let center_start = inner.min_y() + top + if top > 0. { gap } else { 0. };
        let center_end =
            (inner.max_y() - bottom - if bottom > 0. { gap } else { 0. }).max(center_start);
        let center_height = heights[1].min((center_end - center_start).max(0.));
        let center_y = (inner.center().y - center_height * 0.5)
            .clamp(center_start, (center_end - center_height).max(center_start));
        let regions = [
            Rect::new(inner.min_x(), inner.min_y(), inner.size.width, top),
            Rect::new(inner.min_x(), center_y, inner.size.width, center_height),
            Rect::new(
                inner.min_x(),
                inner.max_y() - bottom,
                inner.size.width,
                bottom,
            ),
        ];
        for (section, (items, region)) in members.iter().zip(regions).enumerate() {
            let section_gap = section_gaps[section];
            let spaces =
                (section_gap * items.len().saturating_sub(1) as f32).min(region.size.height);
            let natural = items.iter().map(|(_, h)| h).sum::<f32>();
            let factor = if natural > 0. {
                ((region.size.height - spaces) / natural).min(1.)
            } else {
                0.
            };
            let group_clip = if factor < 1. {
                region.intersection(&clip).unwrap_or_default()
            } else {
                clip
            };
            let mut y = region.min_y();
            for &(index, height) in items {
                let height = height * factor;
                self.children[index].arrange(
                    text,
                    Rect::new(region.min_x(), y, region.size.width, height),
                    group_clip,
                );
                y += height + section_gap;
            }
        }
    }
}

#[cfg(test)]
mod radio_projection_tests {
    use super::*;
    #[test]
    fn toolbar_group_hidden_children_preserve_visible_layout_and_mounts() {
        struct Text;
        impl ButtonTextRenderer for Text {
            fn measure(&mut self, text: &str, _: proposal::TextRole, width: f32) -> Size {
                Size::new((text.len() as f32 * 7.).min(width), 20.)
            }
            fn draw(
                &mut self,
                _: &mut Scene,
                _: &str,
                _: proposal::TextRole,
                _: Rect,
                _: crate::ButtonTextAlign,
                _: Rgba,
            ) {
            }
        }
        let group = |axis, hidden_position: Option<usize>, enclosed| {
            let mut children = vec![
                Content::new(ComponentId(2), Button::new("First")),
                Content::new(ComponentId(3), Button::new("Last")),
            ];
            if let Some(position) = hidden_position {
                let mut hidden = ToolbarGroup::new(
                    ToolbarPlacement::Automatic,
                    vec![Content::new(
                        ComponentId(5),
                        Button::new("Hidden wide action"),
                    )],
                );
                hidden.visible = false;
                children.insert(position, Content::new(ComponentId(4), hidden));
            }
            let mut group = ToolbarGroup::new(ToolbarPlacement::Automatic, children);
            group.set_axis(axis);
            let content = Content::new(ComponentId(1), group);
            let root = if enclosed {
                let mut toolbar = ToolbarContent::new(vec![
                    content,
                    Content::new(ComponentId(6), Button::new("Competing action")),
                ]);
                toolbar.set_axis(axis);
                Content::new(ComponentId(100), toolbar)
            } else {
                content
            };
            ContentHost::new(root).unwrap()
        };
        for axis in [Axis::Horizontal, Axis::Vertical] {
            for enclosed in [false, true] {
                for width in [120., 800.] {
                    let frame = Rect::new(10., 20., width, 200.);
                    let paint = |host: &mut ContentHost| {
                        host.render(
                            &mut Scene::new(),
                            &mut Text,
                            frame,
                            frame,
                            tekes_ui_tokens::Theme::LIGHT.into(),
                        )
                        .unwrap();
                    };
                    let mut reference = group(axis, None, enclosed);
                    let expected_size = reference
                        .get_mut(ComponentId(1))
                        .unwrap()
                        .measure(&mut Text, frame.size)
                        .size;
                    paint(&mut reference);
                    for position in 0..3 {
                        let mut host = group(axis, Some(position), enclosed);
                        assert_eq!(
                            host.get_mut(ComponentId(1))
                                .unwrap()
                                .measure(&mut Text, frame.size)
                                .size,
                            expected_size
                        );
                        paint(&mut host);
                        let mounts = [2, 3, 4, 5]
                            .map(|id| host.get(ComponentId(id)).unwrap().mount_identity());
                        for visible in [false, true, false] {
                            host.get_mut(ComponentId(4))
                                .unwrap()
                                .component_mut::<ToolbarGroup>()
                                .unwrap()
                                .visible = visible;
                            paint(&mut host);
                            assert_eq!(
                                [2, 3, 4, 5]
                                    .map(|id| host.get(ComponentId(id)).unwrap().mount_identity()),
                                mounts
                            );
                            if !visible {
                                for id in [ComponentId(2), ComponentId(3)] {
                                    assert_eq!(
                                        host.get(id).unwrap().frame(),
                                        reference.get(id).unwrap().frame()
                                    );
                                    assert_eq!(
                                        host.get(id).unwrap().visible(),
                                        reference.get(id).unwrap().visible()
                                    );
                                }
                                if enclosed {
                                    assert_eq!(
                                        host.get(ComponentId(6)).unwrap().frame(),
                                        reference.get(ComponentId(6)).unwrap().frame()
                                    );
                                }
                                assert!(
                                    !host
                                        .focus_targets()
                                        .iter()
                                        .any(|(id, _)| *id == ComponentId(5))
                                );
                            }
                        }
                    }
                    let mut empty = ToolbarGroup::new(
                        ToolbarPlacement::Automatic,
                        (4..6)
                            .map(|id| {
                                let mut hidden =
                                    ToolbarGroup::new(ToolbarPlacement::Automatic, Vec::new());
                                hidden.visible = false;
                                Content::new(ComponentId(id), hidden)
                            })
                            .collect(),
                    );
                    empty.set_axis(axis);
                    assert_eq!(empty.measure(&mut Text, frame.size).size, Size::default());
                }
            }
        }
    }
    #[test]
    fn radio_projection_has_no_second_selection_and_false_preserves_checked_siblings() {
        let mut first = Button::new("First");
        first.button.state.selected = true;
        let mut menu = MenuContent::new("Family", vec!["Instance".into()]);
        menu.trigger.button.state.selected = true;
        let mut group = ToolbarGroup::new(
            ToolbarPlacement::Navigation,
            vec![
                Content::new(ComponentId(1), first),
                Content::new(ComponentId(2), menu),
            ],
        );
        assert!(!group.is_radio());
        group.set_selected_id(Some(ComponentId(2)));
        assert!(
            group.children[0]
                .component::<Button>()
                .unwrap()
                .button
                .state
                .selected
        );
        assert!(
            group.children[1]
                .component::<MenuContent>()
                .unwrap()
                .trigger
                .button
                .state
                .selected
        );
        group.set_radio(true);
        assert!(group.set_selected_id(Some(ComponentId(1))));
        assert!(
            group.children[0]
                .component::<Button>()
                .unwrap()
                .button
                .state
                .selected
        );
        assert!(
            !group.children[1]
                .component::<MenuContent>()
                .unwrap()
                .trigger
                .button
                .state
                .selected
        );
        assert_eq!(group.semantics().role, ContentRole::RadioGroup);
        group.set_selected_id(Some(ComponentId(2)));
        assert!(
            !group.children[0]
                .component::<Button>()
                .unwrap()
                .button
                .state
                .selected
        );
        assert!(
            group.children[1]
                .component::<MenuContent>()
                .unwrap()
                .trigger
                .button
                .state
                .selected
        );
        group.set_radio(false);
        group.set_selected_id(Some(ComponentId(1)));
        assert!(
            group
                .children
                .iter()
                .all(|child| child.component.semantics().selected)
        );
        group.set_selected_id(None);
        assert!(
            group
                .children
                .iter()
                .all(|child| child.component.semantics().selected)
        );
    }
    #[test]
    fn icon_and_dropdown_use_original_toolbar_box_and_centered_decorative_row() {
        struct Text;
        impl ButtonTextRenderer for Text {
            fn measure(&mut self, text: &str, _: proposal::TextRole, _: f32) -> Size {
                Size::new(text.len() as f32 * 8., 16.)
            }
            fn draw(
                &mut self,
                _: &mut Scene,
                _: &str,
                _: proposal::TextRole,
                _: Rect,
                _: crate::ButtonTextAlign,
                _: Rgba,
            ) {
            }
        }
        let mut text = Text;
        let icon = |id, name| Content::new(ComponentId(id), IconContent::new(name).unwrap());
        let mut single =
            Button::new(icon(11, "tekes-file-generic")).with_accessible_name("Document");
        single.variant = crate::paint::ButtonVariant::Toolbar;
        single.set_toolbar_icon_slot(true);
        let regular = crate::toolbar::metrics(&mut text, proposal::TextRole::UiBody).box_size;
        let size = single.measure(&mut text, Size::new(400., 48.)).size;
        assert_eq!(size, Size::new(regular, regular));
        let content = Content::new(
            ComponentId(20),
            ToolbarGroup::new(
                ToolbarPlacement::Navigation,
                vec![icon(21, "tekes-file-generic"), icon(22, "tekes-expand")],
            ),
        );
        let mut menu = MenuContent::new("Documents", Vec::new())
            .with_trigger_content("Documents", content)
            .unwrap();
        menu.trigger.set_toolbar_icon_slot(true);
        let natural = menu.measure(&mut text, Size::new(400., 48.)).size;
        let icon_side = proposal::metric("size", "size-icon-standard").unwrap();
        let gap = proposal::metric("spacing", "space-tight").unwrap();
        assert_eq!(natural, Size::new(regular + icon_side + gap, regular));
        let frame = Rect::new(30., 10., natural.width, natural.height);
        menu.arrange(&mut text, frame, frame);
        let row = menu.children()[0].component::<ToolbarGroup>().unwrap();
        assert_eq!(row.children[0].frame().center().y, frame.center().y);
        assert_eq!(row.children[1].frame().center().y, frame.center().y);
        assert_eq!(row.children[0].frame().size, row.children[1].frame().size);
    }

    #[test]
    fn vertical_toolbar_uses_symmetric_end_padding() {
        struct Text;
        impl ButtonTextRenderer for Text {
            fn measure(&mut self, value: &str, role: proposal::TextRole, width: f32) -> Size {
                Size::new(
                    (value.chars().count() as f32 * 8.).min(width),
                    role.style().line_height,
                )
            }
            fn draw(
                &mut self,
                _: &mut Scene,
                _: &str,
                _: proposal::TextRole,
                _: Rect,
                _: crate::ButtonTextAlign,
                _: Rgba,
            ) {
            }
        }
        let mut text = Text;
        let mut toolbar = ToolbarContent::new(vec![
            Content::new(ComponentId(31), Button::new("First")),
            Content::new(ComponentId(32), Button::new("Second")),
        ]);
        toolbar.set_axis(Axis::Vertical);
        let measured = toolbar.measure(&mut text, Size::new(200., 400.)).size;
        let frame = Rect::new(0., 0., 200., measured.height);
        toolbar.arrange(&mut text, frame, frame);
        let top_pad = toolbar.children[0].frame().min_y() - frame.min_y();
        let bottom_pad = frame.max_y() - toolbar.children[1].frame().max_y();
        let expected_pad = crate::toolbar::metrics(&mut text, proposal::TextRole::UiBody).inset;
        assert!((top_pad - expected_pad).abs() <= 0.01, "top_pad={top_pad}");
        assert!(
            (bottom_pad - expected_pad).abs() <= 0.01,
            "bottom_pad={bottom_pad}"
        );
    }
}

#[cfg(test)]
mod capsule_group_tests {
    use super::super::toolbar_host::{
        ToolbarCommandBinding, ToolbarHost, ToolbarRegion, ToolbarRegistration,
    };
    use super::*;
    use tekes_ui_scene::{CornerRadii, Primitive, Quad};

    struct RecordingIcon(Vec<(String, Rect, Rgba)>);
    impl crate::layout_view::IconRenderer for RecordingIcon {
        fn prepare_icon(
            &mut self,
            _: &str,
            _: f32,
            _: f32,
            _: Rgba,
        ) -> Result<crate::composition::IconImage, crate::composition::IconRenderError> {
            Ok(crate::composition::IconImage::new(
                tekes_ui_scene::ImageId(7),
                std::rc::Rc::new(()),
            ))
        }
        fn draw_icon(&mut self, _: &mut Scene, canonical_name: &str, frame: Rect, tint: Rgba) {
            self.0.push((canonical_name.to_owned(), frame, tint));
        }
    }

    struct ScaledText {
        scale: f32,
        draws: Vec<(String, Rgba)>,
        icons: RecordingIcon,
    }
    impl ScaledText {
        fn new(scale: f32) -> Self {
            Self {
                scale,
                draws: Vec::new(),
                icons: RecordingIcon(Vec::new()),
            }
        }
    }
    impl ButtonTextRenderer for ScaledText {
        fn icon_renderer(&mut self) -> Option<&mut dyn crate::layout_view::IconRenderer> {
            Some(&mut self.icons)
        }
        fn measure(&mut self, value: &str, role: proposal::TextRole, width: f32) -> Size {
            Size::new(
                (value.chars().count() as f32 * 8.).min(width),
                role.style().line_height,
            )
        }
        fn backing_scale(&self) -> f32 {
            self.scale
        }
        fn draw(
            &mut self,
            _: &mut Scene,
            value: &str,
            _: proposal::TextRole,
            _: Rect,
            _: crate::ButtonTextAlign,
            color: Rgba,
        ) {
            self.draws.push((value.to_owned(), color));
        }
    }

    fn mixed_capsule_children() -> Vec<Content> {
        let mut icon_menu = MenuContent::new("", Vec::new());
        icon_menu
            .set_entries(vec![MenuEntry::Item {
                value: "workspace".into(),
                label: "Workspace".into(),
                checked: Some(true),
                disabled: false,
            }])
            .unwrap();
        icon_menu.set_trigger_variant(crate::paint::ButtonVariant::Toolbar);
        icon_menu.set_trigger_toolbar_icon_slot(true);
        icon_menu
            .set_trigger_icon_label("tekes-folder-edit", "")
            .unwrap();
        icon_menu.set_trigger_accessible_name(Some("Access".into()));
        icon_menu.set_trigger_help(Some("Access scope".into()));

        let mut model_menu = MenuContent::new("claude-sonnet-4", Vec::new())
            .with_trigger_tone(crate::paint::ButtonTone::Secondary);
        model_menu
            .set_entries(vec![
                MenuEntry::Item {
                    value: "claude-sonnet-4".into(),
                    label: "claude-sonnet-4".into(),
                    checked: Some(true),
                    disabled: false,
                },
                MenuEntry::Item {
                    value: "claude-opus-4".into(),
                    label: "claude-opus-4".into(),
                    checked: None,
                    disabled: false,
                },
            ])
            .unwrap();
        model_menu.set_trigger_variant(crate::paint::ButtonVariant::Toolbar);

        let high = Button::new("High")
            .with_variant(crate::paint::ButtonVariant::Toolbar)
            .with_tone(crate::paint::ButtonTone::Warning);

        vec![
            Content::new(ComponentId(101), icon_menu),
            Content::new(ComponentId(102), model_menu),
            Content::new(ComponentId(103), high),
        ]
    }

    fn mount_single_group(chrome: ToolbarGroupChrome, label: Option<&str>) -> ToolbarHost<u64> {
        let viewport = Rect::new(0., 0., 500., 60.);
        let region = ToolbarRegion::new(ComponentId(10), Rect::new(0., 0., 500., 44.));
        let mut host = ToolbarHost::new(ComponentId(1), 1, viewport).unwrap();
        host.set_regions(1, viewport, vec![region.clone()]).unwrap();
        let mut group =
            ToolbarGroup::new(ToolbarPlacement::PrimaryAction, mixed_capsule_children())
                .with_chrome(chrome);
        if let Some(label) = label {
            group.set_label(Some(label.into()));
        }
        host.register(ToolbarRegistration {
            region: region.id,
            order: 0,
            group: Content::new(ComponentId(20), group),
            bindings: vec![
                ToolbarCommandBinding {
                    target: ComponentId(101),
                    action: None,
                    command: 101,
                    source: 101,
                },
                ToolbarCommandBinding {
                    target: ComponentId(102),
                    action: None,
                    command: 102,
                    source: 102,
                },
                ToolbarCommandBinding {
                    target: ComponentId(103),
                    action: None,
                    command: 103,
                    source: 103,
                },
            ],
        })
        .unwrap();
        host
    }

    fn quads(scene: &Scene) -> Vec<Quad> {
        scene
            .primitives()
            .iter()
            .filter_map(|p| match p {
                Primitive::Quad(q) | Primitive::KeyedQuad { quad: q, .. } => Some(*q),
                _ => None,
            })
            .collect()
    }

    fn divider_quads(scene: &Scene, capsule: Rect, stroke: f32, ink: Rgba) -> Vec<Quad> {
        quads(scene)
            .into_iter()
            .filter(|q| {
                q.border_width == 0.
                    && q.fill == ink
                    && (q.rect.size.width - stroke).abs() < 0.01
                    && q.rect.size.height > 0.
                    && q.rect.size.height < capsule.size.height
                    && q.rect.min_x() > capsule.min_x()
                    && q.rect.max_x() < capsule.max_x()
            })
            .collect()
    }

    #[test]
    fn plain_toolbar_group_preserves_default_layout_and_has_no_capsule_chrome() {
        let mut text = ScaledText::new(2.0);
        let mut host = mount_single_group(ToolbarGroupChrome::Plain, None);
        let mut scene = Scene::new();
        let theme = tekes_ui_tokens::Theme::LIGHT;
        host.render(&mut scene, &mut text, theme.into()).unwrap();

        let ch = host.content_host();
        let group = ch
            .get(ComponentId(20))
            .unwrap()
            .component::<ToolbarGroup>()
            .unwrap();
        assert_eq!(group.chrome(), ToolbarGroupChrome::Plain);
        assert!(group.capsule_frame().is_none());

        let tight = proposal::metric("spacing", "space-tight").unwrap();
        let s0 = ch.get(ComponentId(101)).unwrap().frame();
        let s1 = ch.get(ComponentId(102)).unwrap().frame();
        let s2 = ch.get(ComponentId(103)).unwrap().frame();
        assert!((s1.min_x() - s0.max_x() - tight).abs() < 0.01);
        assert!((s2.min_x() - s1.max_x() - tight).abs() < 0.01);
        assert!(
            !ch.get(ComponentId(103))
                .unwrap()
                .component::<Button>()
                .unwrap()
                .uses_capsule_segment()
        );

        let palette: Palette = theme.into();
        let (stroke, ink) = crate::paint::decorative_border(palette, 2.0);
        assert!(
            !quads(&scene)
                .iter()
                .any(|q| (q.border_width - stroke).abs() < 0.01 && q.border_color == ink),
            "plain group must not draw a capsule perimeter"
        );
    }

    #[test]
    fn capsule_toolbar_group_chrome_paints_unified_surface_hairline_and_dividers_across_themes_and_scales()
     {
        let pill = proposal::metric("radius", "radius-pill").unwrap_or(999.0);
        for theme in tekes_ui_tokens::Theme::ALL {
            for scale in [1.0f32, 2.0f32] {
                let mut text = ScaledText::new(scale);
                let box_size =
                    crate::toolbar::metrics(&mut text, proposal::TextRole::UiBody).box_size;
                let palette: Palette = theme.into();
                let (stroke, ink) = crate::paint::decorative_border(palette, scale);
                let surface2 = tekes_ui_tokens::surface_palette::resolve_surface(
                    palette,
                    "panel",
                    "surface-frame",
                    "rest",
                )
                .unwrap()
                .background;

                for divided in [true, false] {
                    let mut host = mount_single_group(
                        ToolbarGroupChrome::Capsule { divided },
                        Some("Controls"),
                    );
                    let mut scene = Scene::new();
                    host.render(&mut scene, &mut text, theme.into()).unwrap();

                    let ch = host.content_host();
                    let group = ch
                        .get(ComponentId(20))
                        .unwrap()
                        .component::<ToolbarGroup>()
                        .unwrap();
                    let cap = group.capsule_frame().expect("capsule frame recorded");
                    let s0 = ch.get(ComponentId(101)).unwrap().frame();
                    let s1 = ch.get(ComponentId(102)).unwrap().frame();
                    let s2 = ch.get(ComponentId(103)).unwrap().frame();

                    // Zero gap, uniform box_size cross height, full span covered.
                    assert!((s0.min_x() - cap.min_x()).abs() < 0.001);
                    assert!((s0.max_x() - s1.min_x()).abs() < 0.001);
                    assert!((s1.max_x() - s2.min_x()).abs() < 0.001);
                    assert!((s2.max_x() - cap.max_x()).abs() < 0.001);
                    assert!((cap.size.height - box_size).abs() <= 1.0 / scale + 0.001);
                    assert_eq!(s0.size.height, cap.size.height);
                    assert_eq!(s1.size.height, cap.size.height);
                    assert_eq!(s2.size.height, cap.size.height);

                    // Per-segment corner radii: pill on outer ends, square on middle.
                    let expected_r = (cap.size.width.min(cap.size.height) * 0.5).min(pill);
                    let r0 = ch
                        .get(ComponentId(101))
                        .unwrap()
                        .component::<MenuContent>()
                        .unwrap()
                        .trigger()
                        .button
                        .corner_radii;
                    let r1 = ch
                        .get(ComponentId(102))
                        .unwrap()
                        .component::<MenuContent>()
                        .unwrap()
                        .trigger()
                        .button
                        .corner_radii;
                    let r2 = ch
                        .get(ComponentId(103))
                        .unwrap()
                        .component::<Button>()
                        .unwrap()
                        .button
                        .corner_radii;
                    assert_eq!(
                        r0,
                        Some(CornerRadii {
                            top_left: expected_r,
                            top_right: 0.,
                            bottom_right: 0.,
                            bottom_left: expected_r,
                        })
                    );
                    assert_eq!(r1, Some(CornerRadii::uniform(0.)));
                    assert_eq!(
                        r2,
                        Some(CornerRadii {
                            top_left: 0.,
                            top_right: expected_r,
                            bottom_right: expected_r,
                            bottom_left: 0.,
                        })
                    );

                    let scene_quads = quads(&scene);
                    assert!(
                        scene_quads.iter().any(|q| {
                            q.rect == cap
                                && q.border_width == 0.
                                && q.fill == surface2
                                && q.corner_radii == CornerRadii::uniform(expected_r)
                        }),
                        "missing surface-2 capsule background for {theme:?} scale={scale}"
                    );
                    assert!(
                        scene_quads.iter().any(|q| {
                            q.rect == cap
                                && q.fill.a == 0.
                                && (q.border_width - stroke).abs() < 0.001
                                && q.border_color == ink
                                && q.corner_radii == CornerRadii::uniform(expected_r)
                        }),
                        "missing hairline capsule perimeter for {theme:?} scale={scale}"
                    );

                    let divs = divider_quads(&scene, cap, stroke, ink);
                    if divided {
                        assert_eq!(divs.len(), 2, "expected 2 dividers for 3 segments");
                        for (div, seam) in divs.iter().zip([s0.max_x(), s1.max_x()]) {
                            let snapped_min = ((seam - stroke * 0.5) * scale).round() / scale;
                            assert!((div.rect.min_x() - snapped_min).abs() < 0.001);
                        }
                    } else {
                        assert!(divs.is_empty(), "undivided capsule must draw 0 dividers");
                    }
                }
            }
        }
    }

    #[test]
    fn capsule_dividers_hide_adjacent_to_active_hovered_focused_selected_or_open_segments() {
        let scale = 2.0f32;
        let theme = tekes_ui_tokens::Theme::LIGHT;
        let palette: Palette = theme.into();
        let (stroke, ink) = crate::paint::decorative_border(palette, scale);
        let mut text = ScaledText::new(scale);

        let mut host = mount_single_group(ToolbarGroupChrome::Capsule { divided: true }, None);
        let mut scene = Scene::new();
        host.render(&mut scene, &mut text, theme.into()).unwrap();
        let cap = host
            .content_host()
            .get(ComponentId(20))
            .unwrap()
            .component::<ToolbarGroup>()
            .unwrap()
            .capsule_frame()
            .unwrap();
        let s0 = host.content_host().get(ComponentId(101)).unwrap().frame();
        let s1 = host.content_host().get(ComponentId(102)).unwrap().frame();
        let s2 = host.content_host().get(ComponentId(103)).unwrap().frame();
        assert_eq!(divider_quads(&scene, cap, stroke, ink).len(), 2);

        // Hover middle segment -> hides both adjacent dividers.
        host.event(&ContentEvent::PointerMove(s1.center()));
        scene = Scene::new();
        host.render(&mut scene, &mut text, theme.into()).unwrap();
        assert_eq!(divider_quads(&scene, cap, stroke, ink).len(), 0);

        // Hover leading segment -> hides divider 0, keeps divider 1.
        host.event(&ContentEvent::PointerMove(s0.center()));
        scene = Scene::new();
        host.render(&mut scene, &mut text, theme.into()).unwrap();
        let divs = divider_quads(&scene, cap, stroke, ink);
        assert_eq!(divs.len(), 1);
        assert!(divs[0].rect.min_x() > s0.max_x());

        // Move pointer off capsule -> restores both dividers.
        host.event(&ContentEvent::PointerMove(Point::new(450., 50.)));
        scene = Scene::new();
        host.render(&mut scene, &mut text, theme.into()).unwrap();
        assert_eq!(divider_quads(&scene, cap, stroke, ink).len(), 2);

        // Keyboard focus on middle segment -> hides both dividers & emits middle-radius focus ring.
        let middle_target = host
            .semantics()
            .into_iter()
            .find(|n| n.node.id == ComponentId(102))
            .unwrap()
            .target;
        assert!(host.restore_focus(middle_target, true));
        scene = Scene::new();
        host.render(&mut scene, &mut text, theme.into()).unwrap();
        assert_eq!(divider_quads(&scene, cap, stroke, ink).len(), 0);
        assert!(
            quads(&scene).iter().any(|q| {
                q.fill.a == 0.
                    && q.border_width > stroke
                    && q.border_color.a > 0.
                    && q.rect.min_x() < s1.min_x()
                    && q.rect.max_x() > s1.max_x()
                    && q.corner_radii == CornerRadii::uniform(0.)
            }),
            "focused middle segment must paint square-corner focus ring"
        );

        // Blur, then open leading MenuContent -> hides seam 0|1, keeps seam 1|2, and popup anchors to s0.
        host.blur();
        assert!(host.event(&ContentEvent::PointerDown(s0.center())).consumed);
        assert!(host.event(&ContentEvent::PointerUp(s0.center())).consumed);
        scene = Scene::new();
        host.render(&mut scene, &mut text, theme.into()).unwrap();
        let icon_menu = host
            .content_host()
            .get(ComponentId(101))
            .unwrap()
            .component::<MenuContent>()
            .unwrap();
        assert!(icon_menu.is_open());
        assert_eq!(icon_menu.popup_layout().unwrap().anchor, s0);
        let divs = divider_quads(&scene, cap, stroke, ink);
        assert_eq!(divs.len(), 1);
        assert!(divs[0].rect.min_x() > s0.max_x());

        // Dismiss popup by clicking outside capsule, move pointer clear, then select trailing button -> hides seam 1|2, keeps seam 0|1.
        host.event(&ContentEvent::PointerDown(Point::new(450., 50.)));
        host.event(&ContentEvent::PointerUp(Point::new(450., 50.)));
        host.event(&ContentEvent::PointerMove(Point::new(450., 50.)));
        host.update_component::<Button>(ComponentId(20), ComponentId(103), |b| {
            b.set_selected(true)
        })
        .unwrap();
        scene = Scene::new();
        host.render(&mut scene, &mut text, theme.into()).unwrap();
        let divs = divider_quads(&scene, cap, stroke, ink);
        assert_eq!(divs.len(), 1);
        assert!(divs[0].rect.max_x() < s2.min_x());
    }

    #[test]
    fn capsule_segment_button_tones_and_ax_semantics() {
        for theme in [tekes_ui_tokens::Theme::LIGHT, tekes_ui_tokens::Theme::DARK] {
            let palette: Palette = theme.into();
            let mut text = ScaledText::new(2.0);
            let viewport = Rect::new(0., 0., 500., 60.);
            let region = ToolbarRegion::new(ComponentId(10), Rect::new(0., 0., 500., 44.));
            let mut host: ToolbarHost<u64> = ToolbarHost::new(ComponentId(1), 1, viewport).unwrap();
            host.set_regions(1, viewport, vec![region.clone()]).unwrap();
            let children = vec![
                Content::new(
                    ComponentId(101),
                    Button::new("PlainLabel")
                        .with_variant(crate::paint::ButtonVariant::Toolbar)
                        .with_tone(crate::paint::ButtonTone::Default),
                ),
                Content::new(
                    ComponentId(102),
                    Button::new("SecondaryLabel")
                        .with_variant(crate::paint::ButtonVariant::Toolbar)
                        .with_tone(crate::paint::ButtonTone::Secondary),
                ),
                Content::new(
                    ComponentId(103),
                    Button::new("WarningLabel")
                        .with_variant(crate::paint::ButtonVariant::Toolbar)
                        .with_tone(crate::paint::ButtonTone::Warning),
                ),
                Content::new(
                    ComponentId(104),
                    Button::new("ErrorLabel")
                        .with_variant(crate::paint::ButtonVariant::Toolbar)
                        .with_tone(crate::paint::ButtonTone::Error),
                ),
            ];
            let group = ToolbarGroup::new(ToolbarPlacement::PrimaryAction, children)
                .with_capsule(true)
                .with_label("Tone controls");
            host.register(ToolbarRegistration {
                region: region.id,
                order: 0,
                group: Content::new(ComponentId(20), group),
                bindings: vec![],
            })
            .unwrap();
            host.render(&mut Scene::new(), &mut text, theme.into())
                .unwrap();

            let color_for = |label: &str| {
                text.draws
                    .iter()
                    .find(|(text_val, _)| text_val == label)
                    .map(|(_, c)| *c)
                    .expect(label)
            };
            let sec = tekes_ui_tokens::surface_palette::resolve_surface(
                palette,
                "panel",
                "data-category-label",
                "rest",
            )
            .unwrap()
            .text;
            let warn = tekes_ui_tokens::surface_palette::resolve_surface(
                palette,
                "panel",
                "content-status-running",
                "rest",
            )
            .unwrap()
            .text;
            let err = tekes_ui_tokens::surface_palette::resolve_surface(
                palette,
                "panel",
                "content-status-failed",
                "rest",
            )
            .unwrap()
            .text;
            assert_eq!(color_for("SecondaryLabel"), sec);
            assert_eq!(color_for("WarningLabel"), warn);
            assert_eq!(color_for("ErrorLabel"), err);
            assert_ne!(color_for("PlainLabel"), err);

            let sems = host.semantics();
            let group_sem = sems.iter().find(|n| n.node.id == ComponentId(20)).unwrap();
            assert_eq!(group_sem.node.semantics.role, ContentRole::Group);
            assert_eq!(group_sem.node.semantics.name, "Tone controls");
            for child_id in [101, 102, 103, 104] {
                let child = sems
                    .iter()
                    .find(|n| n.node.id == ComponentId(child_id))
                    .unwrap();
                assert_eq!(child.node.parent, Some(ComponentId(20)));
                assert_eq!(child.node.semantics.role, ContentRole::Button);
            }
        }
    }

    #[test]
    fn capsule_segments_never_lose_subpixel_width_to_seam_rounding() {
        struct FractionalText {
            scale: f32,
        }
        impl ButtonTextRenderer for FractionalText {
            fn measure(&mut self, value: &str, role: proposal::TextRole, width: f32) -> Size {
                let w = match value {
                    "seg-a" => 40.32,
                    "seg-b" => 62.36,
                    "High" => 29.36,
                    _ => value.chars().count() as f32 * 7.18,
                };
                Size::new(w.min(width), role.style().line_height)
            }
            fn backing_scale(&self) -> f32 {
                self.scale
            }
            fn draw(
                &mut self,
                _: &mut Scene,
                _: &str,
                _: proposal::TextRole,
                _: Rect,
                _: crate::ButtonTextAlign,
                _: Rgba,
            ) {
            }
        }

        for scale in [1.0f32, 2.0f32] {
            let mut text = FractionalText { scale };
            let viewport = Rect::new(0., 0., 600., 60.);
            let region = ToolbarRegion::new(ComponentId(10), Rect::new(0., 0., 600., 44.));
            let mut host: ToolbarHost<u64> = ToolbarHost::new(ComponentId(1), 1, viewport).unwrap();
            host.set_regions(1, viewport, vec![region.clone()]).unwrap();
            let children = vec![
                Content::new(
                    ComponentId(101),
                    Button::new("seg-a").with_variant(crate::paint::ButtonVariant::Toolbar),
                ),
                Content::new(
                    ComponentId(102),
                    Button::new("seg-b").with_variant(crate::paint::ButtonVariant::Toolbar),
                ),
                Content::new(
                    ComponentId(103),
                    Button::new("High").with_variant(crate::paint::ButtonVariant::Toolbar),
                ),
            ];
            let mut group =
                ToolbarGroup::new(ToolbarPlacement::PrimaryAction, children).with_capsule(true);
            let measured = group.measure(&mut text, Size::new(600., 44.)).size;
            host.register(ToolbarRegistration {
                region: region.id,
                order: 0,
                group: Content::new(ComponentId(20), group),
                bindings: vec![],
            })
            .unwrap();
            host.render(
                &mut Scene::new(),
                &mut text,
                tekes_ui_tokens::Theme::LIGHT.into(),
            )
            .unwrap();

            let ch = host.content_host();
            let grp = ch
                .get(ComponentId(20))
                .unwrap()
                .component::<ToolbarGroup>()
                .unwrap();
            let cap = grp.capsule_frame().unwrap();
            assert!((cap.size.width - measured.width).abs() < 0.001);
            let s0 = ch.get(ComponentId(101)).unwrap().frame();
            let s1 = ch.get(ComponentId(102)).unwrap().frame();
            let s2 = ch.get(ComponentId(103)).unwrap().frame();
            assert!((s0.max_x() - s1.min_x()).abs() < 0.001);
            assert!((s1.max_x() - s2.min_x()).abs() < 0.001);

            // Compare against unrounded natural width: each segment frame >= measured child width.
            let m0 = 40.32
                + spacing(&mut text, "space-regular")
                + spacing(&mut text, "space-micro")
                + spacing(&mut text, "space-regular");
            let m1 = 62.36 + spacing(&mut text, "space-regular") * 2.;
            let m2 = 29.36
                + spacing(&mut text, "space-regular")
                + spacing(&mut text, "space-regular")
                + spacing(&mut text, "space-micro");
            assert!(s0.size.width >= m0, "{} < {m0}", s0.size.width);
            assert!(s1.size.width >= m1, "{} < {m1}", s1.size.width);
            assert!(s2.size.width >= m2, "{} < {m2}", s2.size.width);
            for seam in [s0.min_x(), s1.min_x(), s2.min_x(), s2.max_x()] {
                assert!(((seam * scale) - (seam * scale).round()).abs() < 0.001);
            }
        }
    }

    #[test]
    fn plain_toolbar_group_aligns_buttons_to_box_size_and_toolbar_uses_metrics_inset() {
        struct Text;
        impl ButtonTextRenderer for Text {
            fn measure(&mut self, value: &str, role: proposal::TextRole, width: f32) -> Size {
                Size::new(
                    (value.chars().count() as f32 * 8.).min(width),
                    role.style().line_height,
                )
            }
            fn draw(
                &mut self,
                _: &mut Scene,
                _: &str,
                _: proposal::TextRole,
                _: Rect,
                _: crate::ButtonTextAlign,
                _: Rgba,
            ) {
            }
        }
        let mut text = Text;
        let metrics = crate::toolbar::metrics(&mut text, proposal::TextRole::UiBody);
        let mut group = ToolbarGroup::new(
            ToolbarPlacement::Navigation,
            vec![Content::new(ComponentId(401), Button::new("Save"))],
        );
        let measured_group = group.measure(&mut text, Size::new(300., metrics.row)).size;
        assert_eq!(measured_group.height, metrics.box_size);

        let mut toolbar = ToolbarContent::new(vec![Content::new(ComponentId(402), group)]);
        let frame = Rect::new(0., 0., 400., metrics.row);
        toolbar.arrange(&mut text, frame, frame);
        let arranged_btn = toolbar.children[0].component.children()[0].frame();
        assert_eq!(arranged_btn.min_x(), metrics.inset);
        assert_eq!(arranged_btn.size.height, metrics.box_size);
    }
}
