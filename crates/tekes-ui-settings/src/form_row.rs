//! Settings-owned layout only; original child controls own all paint and input.
use tekes_ui_controls::{
    ButtonTextRenderer,
    composition::{Component, Content, ContentRole, ContentSemantics},
    layout::MeasuredChild,
};
use tekes_ui_scene::{Rect, Size};
#[derive(Debug)]
pub(super) struct SettingsRow {
    name: String,
    children: Vec<Content>,
    switch: bool,
}
impl SettingsRow {
    pub fn new(name: &str, labels: Content, value: Content, switch: bool) -> Self {
        Self {
            name: name.into(),
            children: vec![labels, value],
            switch,
        }
    }
    fn widths(width: f32) -> (f32, f32) {
        let gap = 12_f32.min(width.max(0.));
        let usable = (width - gap).max(0.);
        let values = (usable * 0.42).max(120.).min(usable * 0.6).min(300.);
        ((usable - values).max(0.), values)
    }
    fn measured(&mut self, text: &mut dyn ButtonTextRenderer, available: Size) -> [Size; 2] {
        let (labels, values) = Self::widths(available.width);
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
        ]
    }
}
impl Component for SettingsRow {
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
        let [labels, value] = self.measured(text, available);
        MeasuredChild::new(Size::new(available.width, labels.height.max(value.height)))
    }
    fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        let [labels, value] = self.measured(text, frame.size);
        let (label_width, value_width) = Self::widths(frame.size.width);
        let control_width = if self.switch {
            value_width.min(48.)
        } else {
            value.width.min(value_width)
        };
        let height = labels.height.max(value.height);
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
            assert_eq!(value.max_x(), frame.max_x());
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
}
