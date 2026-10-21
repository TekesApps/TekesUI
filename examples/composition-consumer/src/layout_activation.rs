//! Minimal application-owned layout producer for #27. The original field,
//! editor and reading controller stay owned by their production components.
use std::{cell::Cell, rc::Rc};
use tekes_ui_controls::{
    ButtonTextRenderer, ReadingController, composition::*, layout::MeasuredChild,
};
use tekes_ui_scene::{Rect, Scene, Size};
use tekes_ui_tokens::proposal;
pub const PANE: ComponentId = ComponentId(9910);
pub const FIELD: ComponentId = ComponentId(9911);
#[derive(Debug)]
pub struct Pane {
    active: bool,
    activate_in_measure: bool,
    children: Vec<Content>,
    paints: Rc<Cell<usize>>,
    arranges: Rc<Cell<usize>>,
}
impl Pane {
    pub fn layout_active(&self) -> bool {
        self.active
    }
}
impl Component for Pane {
    fn active(&self) -> bool {
        self.active
    }
    fn children(&self) -> &[Content] {
        &self.children
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.children)
    }
    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, a: Size) -> MeasuredChild {
        if self.activate_in_measure {
            self.active = a.width > 0. && a.height > 0.;
        }
        MeasuredChild::new(a)
    }
    fn arrange(&mut self, t: &mut dyn ButtonTextRenderer, f: Rect, c: Rect) {
        self.arranges.set(self.arranges.get() + 1);
        self.active = f.size.width > 0. && f.size.height > 0.;
        if self.active {
            self.children[0].arrange(t, f, c);
        }
    }
    fn paint(
        &mut self,
        _: &mut Scene,
        _: &mut dyn ButtonTextRenderer,
        _: Rect,
        _: Rect,
        _: ContentTheme,
    ) {
        self.paints.set(self.paints.get() + 1);
    }
}
pub fn fixture(
    in_measure: bool,
    readonly: bool,
) -> (ContentHost, Rc<Cell<usize>>, Rc<Cell<usize>>) {
    let paints = Rc::new(Cell::new(0));
    let arranges = Rc::new(Cell::new(0));
    let mut field = if readonly {
        TextFieldContent::read_only_content(
            "history",
            "line A\nline B 😀",
            proposal::TextRole::UiBody,
        )
    } else {
        TextFieldContent::new("field", "keep😀value")
    };
    if readonly {
        let mut reading = ReadingController::default();
        reading
            .set_document(&mut field.surface, "document-A", 1, "line A\nline B 😀")
            .unwrap();
        field.reading = Some(reading);
    }
    let pane = Pane {
        active: false,
        activate_in_measure: in_measure,
        children: vec![Content::new(FIELD, field)],
        paints: paints.clone(),
        arranges: arranges.clone(),
    };
    (
        ContentHost::new(Content::new(PANE, pane)).unwrap(),
        paints,
        arranges,
    )
}
