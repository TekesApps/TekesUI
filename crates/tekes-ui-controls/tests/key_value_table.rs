use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer,
    composition::{
        Component, ComponentId, CompositionError, Content, KeyValueTableContent, TextContent,
    },
};
use tekes_ui_scene::{Rect, Scene, Size};
use tekes_ui_tokens::{Rgba, proposal::TextRole};

struct WrappedText;
impl ButtonTextRenderer for WrappedText {
    fn measure(&mut self, text: &str, _: TextRole, width: f32) -> Size {
        let natural = text.len() as f32 * 8.;
        Size::new(
            natural.min(width),
            (natural / width.max(1.)).ceil().max(1.) * 20.,
        )
    }
    fn draw(&mut self, _: &mut Scene, _: &str, _: TextRole, _: Rect, _: ButtonTextAlign, _: Rgba) {}
}
fn table() -> KeyValueTableContent {
    KeyValueTableContent::new([
        (
            Content::new(ComponentId(2), TextContent::new("Short")),
            Content::new(ComponentId(3), TextContent::new("long value ".repeat(60))),
        ),
        (
            Content::new(ComponentId(4), TextContent::new("Second key")),
            Content::new(ComponentId(5), TextContent::new("value")),
        ),
    ])
}

#[test]
fn wrapped_values_keep_rows_separate_and_columns_aligned_after_resize() {
    let mut content = Content::new(ComponentId(1), table());
    let mut narrow_height = 0.;
    for width in [220., 640.] {
        let measured = content.measure(&mut WrappedText, Size::new(width, 10000.));
        let frame = Rect::new(10., 20., width, measured.size.height);
        content.arrange(&mut WrappedText, frame, frame);
        let cells = content.component::<KeyValueTableContent>().unwrap().cells();
        assert_eq!(cells[0].frame().min_x(), cells[2].frame().min_x());
        assert_eq!(cells[1].frame().min_x(), cells[3].frame().min_x());
        assert_eq!(cells[0].frame().min_y(), cells[1].frame().min_y());
        assert_eq!(cells[0].frame().size.height, cells[1].frame().size.height);
        assert!(cells[1].frame().size.width > cells[0].frame().size.width);
        assert!(cells[2].frame().min_y() > cells[1].frame().max_y());
        assert!(cells[3].frame().max_x() <= frame.max_x());
        assert_eq!(cells[1].id(), ComponentId(3));
        if width == 220. {
            narrow_height = measured.size.height;
        } else {
            assert!(measured.size.height < narrow_height);
        }
    }
}

#[test]
fn direct_child_mutation_cannot_silently_drop_an_unpaired_cell() {
    let mut table = table();
    table.children_mut().unwrap().pop();
    assert_eq!(table.validate(), Err(CompositionError::InvalidSlotCount));
}
