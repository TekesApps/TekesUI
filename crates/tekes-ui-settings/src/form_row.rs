//! Settings-owned layout only; original child controls own all paint and input.
use tekes_ui_controls::{
    ButtonTextRenderer,
    composition::{Component, ComponentId, Content, ContentRole, ContentSemantics, Lifecycle},
    layout::MeasuredChild,
};
use tekes_ui_scene::{Rect, Size};
#[derive(Debug)]
pub(super) struct SettingsRow {
    name: String,
    children: Vec<Content>,
    switch: bool,
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
    fn lifecycle(&mut self, _: Lifecycle) { self.arranged_mounts = None; }
    fn host_reveal_passthrough(&self) -> bool {
        let Some(mounts) = self.arranged_mounts else { return false; };
        self.children.len() == 2 && self.children.iter().enumerate().all(|(i, child)|
            child.id() == self.child_ids[i] && child.mount_identity() == mounts[i])
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
        let [labels, value] = self.measured(text, available);
        MeasuredChild::new(Size::new(available.width, labels.height.max(value.height)))
    }
    fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        self.arranged_mounts = None;
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
        // A scroll translation preserves this arranged row's widths, child
        // order, relative frames and absence of a local clip; reflow re-arranges.
        if self.children.iter().map(Content::id).eq(self.child_ids) {
            self.arranged_mounts = Some([self.children[0].mount_identity(), self.children[1].mount_identity()]);
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
    #[test]
    fn arranged_row_translation_keeps_original_children_and_invalidates_on_reflow_or_lifecycle() {
        let label = || Content::new(ComponentId(1), BoxContent{text_width:180.,wraps:true});
        let value = || Content::new(ComponentId(2), BoxContent{text_width:80.,wraps:false});
        let mut row=SettingsRow::new("row",label(),value(),false);
        let clip=Rect::new(0.,0.,1000.,1000.);
        let first=Rect::new(10.,20.,620.,100.);
        assert!(!row.host_reveal_passthrough());
        row.measure(&mut Text,first.size);
        assert!(!row.host_reveal_passthrough());
        row.arrange(&mut Text,first,clip);
        assert!(row.host_reveal_passthrough());
        let original:Vec<_>=row.children().iter().map(|c|c.frame()).collect();
        let _=row.children_mut();
        assert!(row.host_reveal_passthrough(),"child accessor must not revoke its own planner admission");
        assert!(row.host_reveal_local_clip().is_none());
        row.measure(&mut Text,first.size);
        assert!(!row.host_reveal_passthrough(),"measuring/reflow needs a fresh arrange");
        row.arrange(&mut Text,Rect::new(50.,80.,620.,100.),clip);
        assert!(row.host_reveal_passthrough());
        for (before,child) in original.iter().zip(row.children()) {
            assert_eq!(child.frame(),Rect::new(before.min_x()+40.,before.min_y()+60.,before.size.width,before.size.height));
        }
        assert!(row.host_reveal_local_clip().is_none());
        row.children.swap(0,1);
        assert!(!row.host_reveal_passthrough(),"original child order is required");
        row.children.swap(0,1);
        row.children[1]=value();
        assert!(!row.host_reveal_passthrough(),"same numeric ID cannot renew an original mount");
        row.arrange(&mut Text,first,clip);
        assert!(row.host_reveal_passthrough());
        row.lifecycle(Lifecycle::Unmounted);
        assert!(!row.host_reveal_passthrough());
    }

}
