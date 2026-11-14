//! Shared Settings row surface and layout; original child controls own their input and state.
use tekes_ui_controls::{
    ButtonTextRenderer,
    composition::{
        Component, ComponentId, Content, ContentRole, ContentSemantics, ContentTheme, Lifecycle,
    },
    layout::MeasuredChild,
};
use tekes_ui_scene::{CornerRadii, Quad, Rect, Scene, Size};
use tekes_ui_tokens::{Spacing, proposal};
#[derive(Debug)]
pub(super) struct SettingsRow {
    name: String,
    children: Vec<Content>,
    switch: bool,
    group_edges: (bool, bool),
    child_ids: [ComponentId; 2],
    arranged_mounts: Option<[u64; 2]>,
}
impl SettingsRow {
    pub fn new(name: &str, labels: Content, value: Content, switch: bool) -> Self {
        Self {
            name: name.into(),
            child_ids: [labels.id(), value.id()],
            children: vec![labels, value],
            arranged_mounts: None,
            switch,
            group_edges: (true, true),
        }
    }
    pub fn in_group(mut self, first: bool, last: bool) -> Self {
        self.group_edges = (first, last);
        self
    }
    fn padding() -> Size {
        Size::new(
            proposal::metric("spacing", "space-content-row-inline").expect("row inline padding"),
            proposal::metric("spacing", "space-content-row-block").expect("row block padding"),
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
        let horizontal_radio = !self.switch
            && self.children[1]
                .component::<tekes_ui_controls::composition::RadioGroupContent>()
                .is_some_and(|radio| {
                    radio.view().axis() == tekes_ui_controls::extended::Axis::Horizontal
                });
        if horizontal_radio {
            let usable = labels + values;
            let title_natural = text
                .measure(
                    &self.name,
                    tekes_ui_tokens::proposal::TextRole::UiBody,
                    f32::INFINITY,
                )
                .width;
            let label_min = title_natural.max(0.).min(labels);
            let radio_natural = self.children[1]
                .measure(text, Size::new(usable, available.height))
                .size
                .width;
            values = values.max(radio_natural).min((usable - label_min).max(0.));
            labels = usable - values;
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
                    .measure(
                        text,
                        Size::new(
                            if self.switch { values.min(48.) } else { values },
                            available.height,
                        ),
                    )
                    .size,
            ],
            (labels, values),
        )
    }
}
impl Component for SettingsRow {
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
        // Adjacent rows share one group surface; only the group's outer corners round.
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
            let (width, ink) =
                tekes_ui_controls::paint::decorative_border(theme.palette, text.backing_scale());
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
        let control_width = if self.switch {
            value_width.min(48.)
        } else {
            value.width.min(value_width)
        };
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
        // TextField retains its default metadata layout. Align the external
        // name with the real input viewport, rather than its empty label slot.
        let value_center = self.children[1]
            .component::<tekes_ui_controls::composition::TextFieldContent>()
            .map(|field| field.surface.outer.center().y)
            .or_else(|| {
                self.children[1]
                    .component::<tekes_ui_controls::composition::RadioGroupContent>()
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
        // A scroll translation preserves this arranged row's widths, child
        // order, relative frames and absence of a local clip; reflow re-arranges.
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
    use tekes_ui_controls::{ButtonTextAlign, composition::ComponentId};
    use tekes_ui_scene::Scene;
    use tekes_ui_tokens::{Rgba, proposal};
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
        fn measure(&mut self, _: &str, _: proposal::TextRole, width: f32) -> Size {
            Size::new(width, 20.)
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
    #[test]
    fn shared_columns_do_not_overlap_and_multiline_labels_keep_controls_centered() {
        for width in [120., 240., 620.] {
            let mut row = SettingsRow::new(
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
                false,
            );
            let size = row.measure(&mut Text, Size::new(width, 1000.)).size;
            let frame = Rect::new(10., 20., width, size.height);
            row.arrange(&mut Text, frame, frame);
            let label = row.children[0].frame();
            let value = row.children[1].frame();
            assert!(label.max_x() <= value.min_x());
            assert_eq!(value.max_x(), frame.max_x() - SettingsRow::padding().width);
            assert_eq!(label.min_x(), frame.min_x() + SettingsRow::padding().width);
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
                row.children[1].frame().min_x(),
                value_start,
                "column alignment is independent of label length"
            );
        }
    }
    #[test]
    fn row_ground_links_columns_without_a_border_and_respects_parent_clip() {
        let mut row = SettingsRow::new(
            "Row",
            Content::new(
                ComponentId(1),
                BoxContent {
                    text_width: 100.,
                    wraps: false,
                },
            ),
            Content::new(
                ComponentId(2),
                BoxContent {
                    text_width: 80.,
                    wraps: false,
                },
            ),
            false,
        );
        let measured = row.measure(&mut Text, Size::new(620., 1000.)).size;
        let frame = Rect::new(10., 20., measured.width, measured.height);
        let clip = Rect::new(10., 30., 620., 20.);
        row.arrange(&mut Text, frame, clip);
        assert_eq!(
            row.children[0].frame().min_y() - frame.min_y(),
            SettingsRow::padding().height
        );
        assert_eq!(
            frame.max_y() - row.children[0].frame().max_y(),
            SettingsRow::padding().height
        );
        for theme in tekes_ui_tokens::Theme::ALL {
            let mut scene = Scene::new();
            row.paint(
                &mut scene,
                &mut Text,
                frame,
                clip,
                tekes_ui_tokens::Palette::from(theme).into(),
            );
            assert!(
                matches!(scene.primitives()[0], tekes_ui_scene::Primitive::PushClip(r) if r == clip)
            );
            let tekes_ui_scene::Primitive::Quad(q) = &scene.primitives()[1] else {
                panic!("row ground")
            };
            assert_eq!(q.rect, frame);
            assert_eq!(q.border_width, 0.);
            assert_eq!(
                q.fill,
                tekes_ui_tokens::Palette::from(theme)
                    .color("c-settings-group-ground")
                    .unwrap()
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
        let mut row = SettingsRow::new("row", label(), value(), false);
        let clip = Rect::new(0., 0., 1000., 1000.);
        let first = Rect::new(10., 20., 620., 100.);
        assert!(!row.host_reveal_passthrough());
        row.measure(&mut Text, first.size);
        assert!(!row.host_reveal_passthrough());
        row.arrange(&mut Text, first, clip);
        assert!(row.host_reveal_passthrough());
        let original: Vec<_> = row.children().iter().map(|c| c.frame()).collect();
        let _ = row.children_mut();
        assert!(
            row.host_reveal_passthrough(),
            "child accessor must not revoke its own planner admission"
        );
        assert!(row.host_reveal_local_clip().is_none());
        row.measure(&mut Text, first.size);
        assert!(
            !row.host_reveal_passthrough(),
            "measuring/reflow needs a fresh arrange"
        );
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
        assert!(
            !row.host_reveal_passthrough(),
            "original child order is required"
        );
        row.children.swap(0, 1);
        row.children[1] = value();
        assert!(
            !row.host_reveal_passthrough(),
            "same numeric ID cannot renew an original mount"
        );
        row.arrange(&mut Text, first, clip);
        assert!(row.host_reveal_passthrough());
        row.lifecycle(Lifecycle::Unmounted);
        assert!(!row.host_reveal_passthrough());
    }
}
