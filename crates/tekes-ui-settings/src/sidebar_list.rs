//! Compatibility Content adapter for the pinned release's original List/ListView.
//! Selection, input and default paint remain in those shared production controls.
use tekes_ui_controls::{
    ButtonTextRenderer,
    composition::{
        AccessibilityAction, Component, ContentEvent, ContentResponse, ContentRole,
        ContentSemantics, ContentTheme, SemanticPart, CurveSubtreeCapability, PointerPhase,
    },
    layout::MeasuredChild,
};
use tekes_ui_scene::{Point, Rect, Scene, Size};
#[derive(Debug)]
pub struct SelectableListContent {
    pub model: tekes_ui_controls::extended::List,
    pub view: tekes_ui_controls::collections_view::ListView,
    pub labels: Vec<String>,
    pub name: String,
}
impl SelectableListContent {
    pub fn new(name: impl Into<String>, labels: Vec<String>) -> Self {
        Self {
            model: tekes_ui_controls::extended::List::new(labels.len()),
            view: tekes_ui_controls::collections_view::ListView::new(),
            labels,
            name: name.into(),
        }
    }
}
impl Component for SelectableListContent {
    fn curve_subtree_capability(&self) -> CurveSubtreeCapability { CurveSubtreeCapability::Local }
    fn pointer_event_in_region(&mut self, event: &ContentEvent, inside: bool) -> ContentResponse {
        let phase = match event {
            ContentEvent::Pointer(pointer) if pointer.is_primary() => Some(pointer.phase),
            ContentEvent::PointerDown(_) => Some(PointerPhase::Down),
            ContentEvent::PointerMove(_) => Some(PointerPhase::Move),
            ContentEvent::PointerUp(_) => Some(PointerPhase::Up),
            _ => None,
        };
        if !inside {
            match phase {
                Some(PointerPhase::Down) => return ContentResponse::default(),
                Some(PointerPhase::Up) => {
                    self.view.cancel();
                    self.view.pointer_move(Point::new(f32::MIN, f32::MIN));
                    return ContentResponse::consumed(true);
                }
                Some(PointerPhase::Move | PointerPhase::Drag | PointerPhase::Enter | PointerPhase::Exit) => {
                    // Same finite MISS used by original Host subtree cancellation.
                    return ContentResponse { changed: self.view.pointer_move(Point::new(f32::MIN, f32::MIN)), ..Default::default() };
                }
                None => {}
            }
        }
        match event {
            ContentEvent::Pointer(pointer) => self.pointer_event(pointer),
            _ => self.event(event),
        }
    }
    fn measure(&mut self, mut text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        // This release exposes layout through render, not a separate measure
        // method. An empty clip computes the original row metrics without paint.
        let layout = tekes_ui_controls::collections_view::ListView::new().render(
            &mut Scene::new(),
            &mut text,
            &self.model,
            &self.labels,
            Rect::new(0., 0., available.width, available.height),
            Some(Rect::default()),
            tekes_ui_tokens::Palette::from(tekes_ui_tokens::Theme::LIGHT),
        );
        MeasuredChild::new(Size::new(available.width, layout.content_height))
    }
    fn paint(
        &mut self,
        scene: &mut Scene,
        mut text: &mut dyn ButtonTextRenderer,
        frame: Rect,
        clip: Rect,
        theme: ContentTheme,
    ) {
        self.view.render(
            scene,
            &mut text,
            &self.model,
            &self.labels,
            frame,
            Some(clip),
            theme.palette,
        );
    }
    fn focusable(&self) -> bool {
        self.model.count > 0
    }
    fn set_focus(&mut self, focused: bool, visible: bool) {
        if focused {
            self.view.focus(self.model.selected.unwrap_or(0), visible);
        } else {
            self.view.blur();
        }
    }
    fn semantics(&self) -> ContentSemantics {
        ContentSemantics::new(ContentRole::List, &self.name)
    }
    fn semantic_parts(&self) -> Vec<SemanticPart> {
        self.view
            .layout()
            .items
            .iter()
            .filter_map(|row| {
                let label = self.labels.get(row.id)?;
                let mut semantics = ContentSemantics::new(ContentRole::ListItem, label);
                semantics.selected = self.model.selected == Some(row.id);
                Some(SemanticPart {
                    index: row.id,
                    frame: row.frame,
                    focused: false,
                    semantics,
                })
            })
            .collect()
    }
    fn accessibility_action(
        &mut self,
        part: Option<usize>,
        action: &AccessibilityAction,
    ) -> ContentResponse {
        if matches!(action, AccessibilityAction::Press)
            && let Some(index) = part
            && index < self.model.count
        {
            let changed = self.model.select(index);
            self.view.focus(index, false);
            return ContentResponse {
                consumed: true,
                changed,
                activated: true,
                ..Default::default()
            };
        }
        ContentResponse::default()
    }
    fn event(&mut self, event: &ContentEvent) -> ContentResponse {
        use tekes_ui_controls::collections_view::CollectionEvent;
        let mut response = ContentResponse::default();
        let result = match event {
            ContentEvent::PointerMove(point) => {
                response.changed = self.view.pointer_move(*point);
                return response;
            }
            ContentEvent::PointerDown(point) => {
                let result = self.view.pointer_down(*point, &mut self.model);
                response.capture_pointer = result != CollectionEvent::Ignored;
                result
            }
            ContentEvent::PointerUp(point) => self.view.pointer_up(*point, &mut self.model),
            ContentEvent::Key { key, down } => self.view.key(*key, *down, &mut self.model),
            ContentEvent::Cancel => {
                self.view.cancel();
                return ContentResponse::consumed(true);
            }
            _ => return response,
        };
        response.consumed = result != CollectionEvent::Ignored;
        response.changed = matches!(
            result,
            CollectionEvent::Changed | CollectionEvent::Activated(_)
        );
        response.activated = response.changed;
        response
    }
}

#[cfg(test)]
mod local_input_tests {
    use super::*;
    use tekes_ui_controls::{ButtonTextAlign,ControlKey,ScrollArea,composition::{ComponentId,Content,ContentHost,ScrollContent}};
    use tekes_ui_controls::collections_view::CollectionEvent;
    use tekes_ui_tokens::{Rgba,Theme,proposal};
    struct Text;
    impl ButtonTextRenderer for Text {
        fn measure(&mut self,text:&str,role:proposal::TextRole,width:f32)->Size { Size::new((text.len() as f32*8.).min(width),role.style().line_height) }
        fn draw(&mut self,_:&mut Scene,_:&str,_:proposal::TextRole,_:Rect,_:ButtonTextAlign,_:Rgba) {}
    }
    fn list()->SelectableListContent { SelectableListContent::new("Route",vec!["Original row".into()]) }
    #[test]
    fn actual_scrolling_list_accepts_pointer_while_unknown_owner_stays_refused() {
        let frame=Rect::new(0.,0.,240.,120.);
        let mut known=ContentHost::new(Content::new(ComponentId(1),ScrollContent::new(ScrollArea::default(),Content::new(ComponentId(2),list())))).unwrap();
        known.set_action_events(true);
        known.render(&mut Scene::new(),&mut Text,frame,frame,Theme::LIGHT.into()).unwrap();
        let point=known.get(ComponentId(2)).unwrap().component::<SelectableListContent>().unwrap().view.layout().items[0].visible.unwrap().center();
        assert!(known.event(&ContentEvent::PointerDown(point)).consumed);
        assert_eq!(known.focused(),Some(ComponentId(2)));
        assert_eq!(known.get(ComponentId(2)).unwrap().component::<SelectableListContent>().unwrap().model.selected,Some(0));
        assert_eq!(known.drain_actions().len(),1);
        #[derive(Debug)]
        struct Unknown(SelectableListContent);
        impl Component for Unknown {
            fn measure(&mut self,text:&mut dyn ButtonTextRenderer,available:Size)->MeasuredChild {self.0.measure(text,available)}
            fn paint(&mut self,scene:&mut Scene,text:&mut dyn ButtonTextRenderer,frame:Rect,clip:Rect,theme:ContentTheme){self.0.paint(scene,text,frame,clip,theme);}
            fn focusable(&self)->bool{self.0.focusable()}
            fn set_focus(&mut self,focused:bool,visible:bool){self.0.set_focus(focused,visible);}
            fn semantics(&self)->ContentSemantics{self.0.semantics()}
            fn event(&mut self,event:&ContentEvent)->ContentResponse{self.0.event(event)}
        }
        let mut unknown=ContentHost::new(Content::new(ComponentId(1),ScrollContent::new(ScrollArea::default(),Content::new(ComponentId(2),Unknown(list()))))).unwrap();
        unknown.set_action_events(true);
        unknown.render(&mut Scene::new(),&mut Text,frame,frame,Theme::LIGHT.into()).unwrap();
        assert!(!unknown.event(&ContentEvent::PointerDown(point)).consumed);
        assert_eq!(unknown.get(ComponentId(2)).unwrap().component::<Unknown>().unwrap().0.model.selected,None);
        assert!(unknown.drain_actions().is_empty());
    }
    #[test]
    fn captured_move_and_up_inside_rect_outside_curve_clear_hover_without_activation_or_blur() {
        use tekes_ui_scene::{ClipBudget,ClipRegion,ClipShape,CornerRadii};
        let mut content=SelectableListContent::new("Route",vec!["First".into(),"Second".into()]);
        let frame=Rect::new(0.,0.,240.,100.);
        content.paint(&mut Scene::new(),&mut Text,frame,frame,ContentTheme::from(tekes_ui_tokens::Palette::from(Theme::LIGHT)));
        let row=content.view.layout().items[0].frame;
        let region=ClipRegion::new(&[ClipShape::normalize(row,CornerRadii::uniform(row.size.height*0.5),Point::default()).unwrap()],ClipBudget{max_shapes:4,max_work:100_000}).unwrap();
        let center=row.center();let corner=Point::new(row.min_x()+0.5,row.min_y()+0.5);
        assert!(row.contains(corner));
        assert!(region.contains([center.x as f64,center.y as f64]).unwrap());
        assert!(!region.contains([corner.x as f64,corner.y as f64]).unwrap());
        let pointer=|phase,position| ContentEvent::Pointer(tekes_ui_controls::composition::PointerEvent {position,phase,button:Some(tekes_ui_controls::composition::PointerButton::Primary),modifiers:Default::default(),click_count:1});
        let down=content.pointer_event_in_region(&pointer(PointerPhase::Down,center),true);
        assert!(down.capture_pointer && down.activated);
        content.set_focus(true,true);
        assert!(content.event(&ContentEvent::Key{key:ControlKey::Down,down:true}).activated);
        assert_eq!(content.model.selected,Some(1),"keyboard changes selection while original pointer capture remains");
        content.pointer_event_in_region(&pointer(PointerPhase::Drag,center),true);
        assert!(content.pointer_event_in_region(&pointer(PointerPhase::Drag,corner),false).changed);
        assert!(!content.view.pointer_move(Point::new(f32::MIN,f32::MIN)),"original hover was already cleared");
        assert!(!content.pointer_event_in_region(&pointer(PointerPhase::Up,corner),false).activated);
        assert_eq!(content.view.pointer_up(center,&mut content.model),CollectionEvent::Ignored,"old capture was cancelled");
        assert_eq!(content.model.selected,Some(1));
        assert!(content.event(&ContentEvent::Key{key:ControlKey::Enter,down:true}).activated,"gesture cancellation keeps original keyboard focus");
    }
}
