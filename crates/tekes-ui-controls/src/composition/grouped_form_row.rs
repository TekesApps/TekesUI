//! Shared grouped form row card anatomy and column layout.
use super::*;
use crate::ButtonTextRenderer;
use tekes_ui_scene::{CornerRadii, Quad, Rect, Scene, Size};
use tekes_ui_tokens::{Spacing, proposal};

/// Paired two-column form row sharing a single passive group card background,
/// corner rounding, inset separator and unified intrinsic-width column allocation.
#[derive(Debug)]
pub struct GroupedFormRowContent {
    name: String,
    children: Vec<Content>,
    group_edges: (bool, bool),
    child_ids: [ComponentId; 2],
    arranged_mounts: Option<[u64; 2]>,
}

impl GroupedFormRowContent {
    pub fn new(name: impl Into<String>, label: Content, value: Content) -> Self {
        Self {
            name: name.into(),
            child_ids: [label.id(), value.id()],
            children: vec![label, value],
            group_edges: (true, true),
            arranged_mounts: None,
        }
    }

    pub fn in_group(mut self, first: bool, last: bool) -> Self {
        self.group_edges = (first, last);
        self
    }

    pub fn label(&self) -> &Content {
        &self.children[0]
    }

    pub fn value(&self) -> &Content {
        &self.children[1]
    }

    pub fn padding() -> Size {
        Size::new(
            proposal::metric("spacing", "space-grouped-form-row-inline")
                .expect("grouped form row inline padding"),
            proposal::metric("spacing", "space-grouped-form-row-block")
                .expect("grouped form row block padding"),
        )
    }

    fn widths(width: f32) -> (f32, f32) {
        let gap = Spacing::Relaxed.value().min(width.max(0.));
        let usable = (width - gap).max(0.);
        let values = (usable * 0.42).max(120.).min(usable * 0.6).min(300.);
        ((usable - values).max(0.), values)
    }

    fn widths_for(&mut self, text: &mut dyn ButtonTextRenderer, available: Size) -> (f32, f32) {
        let (mut labels, mut values) = Self::widths(available.width);
        if self.children.len() != 2 {
            return (labels, values);
        }
        let usable = labels + values;
        let probe_w = 1e6_f32;
        let value_intrinsic = self.children[1]
            .measure_intrinsic(text, Size::new(probe_w, available.height))
            .size
            .width;
        if value_intrinsic.is_finite()
            && value_intrinsic < probe_w * 0.5
            && value_intrinsic > values
        {
            let label_natural = self.children[0]
                .measure_intrinsic(text, Size::new(usable, available.height))
                .size
                .width;
            let label_min = if label_natural.is_finite() {
                label_natural.max(0.).min(labels)
            } else {
                labels
            };
            values = values
                .max(value_intrinsic)
                .min((usable - label_min).max(0.));
            labels = (usable - values).max(0.);
        }
        (labels, values)
    }

    fn measured(
        &mut self,
        text: &mut dyn ButtonTextRenderer,
        available: Size,
    ) -> ([Size; 2], (f32, f32)) {
        let (labels, values) = self.widths_for(text, available);
        (
            [
                self.children[0]
                    .measure(text, Size::new(labels, available.height))
                    .size,
                self.children[1]
                    .measure(text, Size::new(values, available.height))
                    .size,
            ],
            (labels, values),
        )
    }
}

impl Component for GroupedFormRowContent {
    fn has_stable_hooks(&self) -> bool {
        true
    }

    fn supports_prepared_lifecycle(&self) -> bool {
        true
    }

    fn curve_subtree_capability(&self) -> CurveSubtreeCapability {
        CurveSubtreeCapability::Recursive
    }

    fn fixed_slots(&self) -> bool {
        true
    }

    fn ignores_pointer_actions(&self) -> bool {
        true
    }

    fn default_surface(&self) -> Option<&'static str> {
        Some("panel")
    }

    fn validate(&self) -> Result<(), CompositionError> {
        if self.children.len() != 2 {
            return Err(CompositionError::InvalidSlotCount);
        }
        Ok(())
    }

    fn lifecycle(&mut self, _: Lifecycle) {
        self.arranged_mounts = None;
    }

    fn host_reveal_passthrough(&self) -> bool {
        let Some(mounts) = self.arranged_mounts else {
            return false;
        };
        self.children.len() == 2
            && self.children.iter().enumerate().all(|(i, child)| {
                child.id() == self.child_ids[i] && child.mount_identity() == mounts[i]
            })
    }

    fn children(&self) -> &[Content] {
        &self.children
    }

    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.children)
    }

    fn semantics(&self) -> ContentSemantics {
        ContentSemantics::new(ContentRole::Group, &self.name)
    }

    fn measure(&mut self, text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        self.arranged_mounts = None;
        if self.children.len() != 2 {
            return MeasuredChild::new(Size::default());
        }
        let padding = Self::padding();
        let inner = Size::new(
            (available.width - padding.width * 2.).max(0.),
            (available.height - padding.height * 2.).max(0.),
        );
        let ([labels, value], _) = self.measured(text, inner);
        MeasuredChild::new(Size::new(
            available.width,
            (labels.height.max(value.height) + padding.height * 2.)
                .max(proposal::metric("size", "c-row-min-height").expect("shared row minimum")),
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
        let radius = proposal::metric("radius", "radius-panel").expect("Settings group radius");
        let corners = match self.group_edges {
            (true, true) => CornerRadii::uniform(radius),
            (true, false) => CornerRadii::top(radius),
            (false, true) => CornerRadii::bottom(radius),
            (false, false) => CornerRadii::ZERO,
        };
        scene.push_clip(clip);
        scene.push_quad(
            Quad::new(
                frame,
                theme
                    .palette
                    .color("c-settings-group-ground")
                    .expect("Settings group ground"),
            )
            .with_corner_radii(corners),
        );
        if !self.group_edges.0 {
            let (width, ink) = crate::feedback_view::SurfaceView::decorative_border(
                theme.palette,
                text.backing_scale(),
            );
            let inset = Self::padding().width.min(frame.size.width * 0.5);
            scene.push_quad(Quad::new(
                Rect::new(
                    frame.min_x() + inset,
                    frame.min_y(),
                    frame.size.width - 2. * inset,
                    width,
                ),
                ink,
            ));
        }
        scene.pop_clip();
    }

    fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        self.arranged_mounts = None;
        if self.children.len() != 2 {
            return;
        }
        let padding = Self::padding();
        let x = padding.width.min(frame.size.width * 0.5);
        let y = padding.height.min(frame.size.height * 0.5);
        let frame = Rect::new(
            frame.min_x() + x,
            frame.min_y() + y,
            frame.size.width - 2. * x,
            frame.size.height - 2. * y,
        );
        let ([labels, value], (label_width, value_width)) = self.measured(text, frame.size);
        let control_width = value.width.min(value_width);
        let height = frame.size.height.max(labels.height.max(value.height));
        self.children[1].arrange(
            text,
            Rect::new(
                frame.max_x() - control_width,
                frame.min_y() + (height - value.height) * 0.5,
                control_width,
                value.height,
            ),
            clip,
        );
        let value_center = self.children[1]
            .component::<TextFieldContent>()
            .map(|field| field.surface.outer())
            .filter(|rect| !rect.is_empty())
            .map(|rect| rect.center().y)
            .or_else(|| {
                self.children[1]
                    .component::<RadioGroupContent>()
                    .and_then(|radio| radio.view().layout().rows.first())
                    .map(|row| row.frame().center().y)
            })
            .unwrap_or_else(|| self.children[1].frame().center().y);
        let label_y = (value_center - labels.height * 0.5).clamp(
            frame.min_y(),
            frame.min_y() + (height - labels.height).max(0.),
        );
        self.children[0].arrange(
            text,
            Rect::new(frame.min_x(), label_y, label_width, labels.height),
            clip,
        );
        if self.children.iter().map(Content::id).eq(self.child_ids) {
            self.arranged_mounts = Some([
                self.children[0].mount_identity(),
                self.children[1].mount_identity(),
            ]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ButtonTextAlign, TextSurfaceAlignment, TextSurfacePresentation, extended::Axis,
        selection::SwitchView,
    };
    use tekes_ui_scene::{Primitive, Scene};
    use tekes_ui_tokens::{Palette, Rgba, Theme, proposal};

    #[derive(Debug)]
    struct BoxContent {
        text_width: f32,
        wraps: bool,
    }
    impl Component for BoxContent {
        fn measure(&mut self, _: &mut dyn ButtonTextRenderer, a: Size) -> MeasuredChild {
            MeasuredChild::new(Size::new(
                a.width,
                if self.wraps {
                    (self.text_width / a.width.max(1.)).ceil() * 20.
                } else {
                    32.
                },
            ))
        }
    }

    struct Text;
    impl ButtonTextRenderer for Text {
        fn text_surface_host(&mut self) -> Option<&mut dyn crate::TextSurfaceHost> {
            Some(self)
        }
        fn measure(&mut self, value: &str, _: proposal::TextRole, width: f32) -> Size {
            Size::new((value.chars().count() as f32 * 8.).min(width), 20.)
        }
        fn draw(
            &mut self,
            _: &mut Scene,
            _: &str,
            _: proposal::TextRole,
            _: Rect,
            _: ButtonTextAlign,
            _: Rgba,
        ) {
        }
    }
    impl crate::TextSurfaceTextRenderer for Text {
        fn push_visible(
            &mut self,
            _: &mut Scene,
            _: &tekes_ui_text::Paragraph,
            _: Point,
            _: Rgba,
            _: Rect,
        ) {
        }
    }
    impl crate::TextSurfaceHost for Text {
        fn shape_surface(
            &mut self,
            text: &str,
            role: proposal::TextRole,
            _: Option<f32>,
        ) -> std::rc::Rc<tekes_ui_text::Paragraph> {
            let len = text.encode_utf16().count();
            let h = f64::from(role.style().line_height);
            std::rc::Rc::new(tekes_ui_text::Paragraph {
                lines: vec![tekes_ui_text::Line {
                    index: 0,
                    utf16: 0..len,
                    bytes: 0..text.len(),
                    ascent: h * 0.8,
                    descent: h * 0.2,
                    leading: 0.,
                    height: h,
                    top: 0.,
                    baseline_y: h * 0.8,
                    width: len as f64 * 8.,
                    width_trimmed: len as f64 * 8.,
                    runs: vec![],
                    carets: (0..=len).map(|i| (i, i as f64 * 8.)).collect(),
                    graphemes: vec![],
                    has_newline: false,
                }],
                height: h,
                width: len as f64 * 8.,
                default_line_height: h,
                fonts: vec![],
                utf16_len: len,
            })
        }
    }

    #[test]
    fn shared_columns_do_not_overlap_and_multiline_labels_keep_controls_centered() {
        for width in [120., 240., 620.] {
            let mut row = GroupedFormRowContent::new(
                "description",
                Content::new(
                    ComponentId(1),
                    BoxContent {
                        text_width: 700.,
                        wraps: true,
                    },
                ),
                Content::new(
                    ComponentId(2),
                    BoxContent {
                        text_width: 80.,
                        wraps: false,
                    },
                ),
            );
            let size = row.measure(&mut Text, Size::new(width, 1000.)).size;
            let frame = Rect::new(10., 20., width, size.height);
            row.arrange(&mut Text, frame, frame);
            let label = row.label().frame();
            let value = row.value().frame();
            assert!(label.max_x() <= value.min_x());
            assert_eq!(
                value.max_x(),
                frame.max_x() - GroupedFormRowContent::padding().width
            );
            assert_eq!(
                label.min_x(),
                frame.min_x() + GroupedFormRowContent::padding().width
            );
            assert_eq!(label.center().y, value.center().y);
            assert!(label.size.height <= size.height);
            let value_start = value.min_x();
            row.children[0] = Content::new(
                ComponentId(1),
                BoxContent {
                    text_width: 30.,
                    wraps: true,
                },
            );
            let size = row.measure(&mut Text, Size::new(width, 1000.)).size;
            row.arrange(&mut Text, Rect::new(10., 20., width, size.height), frame);
            assert_eq!(
                row.value().frame().min_x(),
                value_start,
                "column alignment is independent of label length"
            );
        }
    }

    #[test]
    fn row_ground_links_columns_propagates_panel_surface_and_respects_clip() {
        let row = GroupedFormRowContent::new(
            "Row",
            Content::new(ComponentId(1), TextContent::new("Row")),
            Content::new(ComponentId(2), SwitchContent::new("", false)),
        )
        .in_group(false, true);
        let mut host = ContentHost::new(Content::new(ComponentId(9), row)).unwrap();
        let size = {
            let r = host
                .get_mut(ComponentId(9))
                .unwrap()
                .component_mut::<GroupedFormRowContent>()
                .unwrap();
            r.measure(&mut Text, Size::new(620., 1000.)).size
        };
        let frame = Rect::new(10., 20., size.width, size.height);
        let clip = Rect::new(10., 20., 620., size.height);
        for theme in Theme::ALL {
            let mut scene = Scene::new();
            let palette = Palette::from(theme);
            host.render(&mut scene, &mut Text, frame, clip, palette)
                .unwrap();
            let label = host.get(ComponentId(1)).unwrap().frame();
            let switch = host.get(ComponentId(2)).unwrap().frame();
            assert_eq!(
                switch.min_y() - frame.min_y(),
                GroupedFormRowContent::padding().height
            );
            assert_eq!(
                frame.max_y() - switch.max_y(),
                GroupedFormRowContent::padding().height
            );
            assert_eq!(label.center().y, switch.center().y);
            assert_eq!(switch.size.width, SwitchView::TRACK_WIDTH);
            assert_eq!(
                switch.max_x(),
                frame.max_x() - GroupedFormRowContent::padding().width
            );
            let panel_surface_bg = palette.color("c-settings-group-ground").unwrap();
            assert!(scene.primitives().iter().any(|p| matches!(
                p,
                Primitive::Quad(q) if q.rect == frame && q.border_width == 0. && q.fill == panel_surface_bg
            )));
            assert!(scene.primitives().iter().any(|p| matches!(
                p,
                Primitive::Quad(q)
                    if q.fill == palette.color("separator").unwrap()
                        && q.rect.min_y() == frame.min_y()
                        && q.rect.min_x() == frame.min_x() + GroupedFormRowContent::padding().width
            )));
            assert_eq!(
                host.root
                    .theme_for(ComponentId(1), palette.into())
                    .map(|t| t.surface()),
                Some("panel"),
                "children on GroupedFormRowContent must inherit panel Surface"
            );
            assert_eq!(
                host.root
                    .theme_for(ComponentId(2), palette.into())
                    .map(|t| t.surface()),
                Some("panel"),
                "value controls on GroupedFormRowContent must inherit panel Surface"
            );
        }
    }

    #[test]
    fn arranged_row_translation_keeps_original_children_and_invalidates_on_reflow_or_lifecycle() {
        let label = || {
            Content::new(
                ComponentId(1),
                BoxContent {
                    text_width: 180.,
                    wraps: true,
                },
            )
        };
        let value = || {
            Content::new(
                ComponentId(2),
                BoxContent {
                    text_width: 80.,
                    wraps: false,
                },
            )
        };
        let mut row = GroupedFormRowContent::new("row", label(), value());
        let clip = Rect::new(0., 0., 1000., 1000.);
        let first = Rect::new(10., 20., 620., 100.);
        assert!(!row.host_reveal_passthrough());
        row.measure(&mut Text, first.size);
        assert!(!row.host_reveal_passthrough());
        row.arrange(&mut Text, first, clip);
        assert!(row.host_reveal_passthrough());
        let original: Vec<_> = row.children().iter().map(|c| c.frame()).collect();
        let _ = row.children_mut();
        assert!(row.host_reveal_passthrough());
        assert!(row.host_reveal_local_clip().is_none());
        row.measure(&mut Text, first.size);
        assert!(!row.host_reveal_passthrough());
        row.arrange(&mut Text, Rect::new(50., 80., 620., 100.), clip);
        assert!(row.host_reveal_passthrough());
        for (before, child) in original.iter().zip(row.children()) {
            assert_eq!(
                child.frame(),
                Rect::new(
                    before.min_x() + 40.,
                    before.min_y() + 60.,
                    before.size.width,
                    before.size.height
                )
            );
        }
        assert!(row.host_reveal_local_clip().is_none());
        row.children.swap(0, 1);
        assert!(!row.host_reveal_passthrough());
        row.children.swap(0, 1);
        row.children[1] = value();
        assert!(!row.host_reveal_passthrough());
    }

    #[test]
    fn unified_intrinsic_allocation_gives_horizontal_radio_unused_label_slack_and_aligns_centers() {
        let mut radio = RadioGroupContent::new(
            "Appearance",
            vec![
                "System Default".into(),
                "Light Mode".into(),
                "Dark Mode".into(),
            ],
        );
        radio.set_axis(Axis::Horizontal);
        let mut row = GroupedFormRowContent::new(
            "Theme",
            Content::new(ComponentId(1), TextContent::new("Theme")),
            Content::new(ComponentId(2), radio),
        );
        let size = row.measure(&mut Text, Size::new(560., 400.)).size;
        let frame = Rect::new(0., 0., 560., size.height);
        row.arrange(&mut Text, frame, frame);
        let label = row.label().frame();
        let value = row.value().frame();
        assert!(label.max_x() < value.min_x());
        assert_eq!(
            value.min_y(),
            row.value()
                .component::<RadioGroupContent>()
                .unwrap()
                .view()
                .layout()
                .rows[0]
                .frame()
                .min_y()
        );
        assert!(
            (label.center().y
                - row
                    .value()
                    .component::<RadioGroupContent>()
                    .unwrap()
                    .view()
                    .layout()
                    .rows[0]
                    .frame()
                    .center()
                    .y)
                .abs()
                <= 0.01
        );

        let field = TextFieldContent::new("", "Mono")
            .with_surface_presentation(TextSurfacePresentation::Content)
            .with_alignment(TextSurfaceAlignment::Trailing);
        let mut text_row = GroupedFormRowContent::new(
            "Font",
            Content::new(ComponentId(3), TextContent::new("Font")),
            Content::new(ComponentId(4), field),
        );
        let size = text_row.measure(&mut Text, Size::new(560., 400.)).size;
        let frame = Rect::new(0., 0., 560., size.height);
        text_row.arrange(&mut Text, frame, frame);
        let label = text_row.label().frame();
        let surface_outer = text_row
            .value()
            .component::<TextFieldContent>()
            .unwrap()
            .surface()
            .outer();
        assert!((label.center().y - surface_outer.center().y).abs() <= 0.01);
    }
}
