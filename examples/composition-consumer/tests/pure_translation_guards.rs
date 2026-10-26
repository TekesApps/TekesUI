//! Public opt-in, actual original Scroll reveal, and no-geometry logical focus.
use std::{cell::Cell, rc::Rc};
use tekes_ui_controls::{ButtonTextAlign, ButtonTextRenderer, ScrollArea, ScrollRevealOptions, composition::*};
use tekes_ui_scene::{Point, Rect, Scene, Size};
use tekes_ui_tokens::{Rgba, Theme, proposal};
struct Text;
impl ButtonTextRenderer for Text {
    fn measure(&mut self, _: &str, _: proposal::TextRole, _: f32) -> Size { Size::new(60., 24.) }
    fn draw(&mut self, _: &mut Scene, _: &str, _: proposal::TextRole, _: Rect, _: ButtonTextAlign, _: Rgba) {}
}
#[derive(Debug)]
struct TranslationBox {
    child: Vec<Content>,
    allowed: Rc<Cell<bool>>,
    local: Rc<Cell<Option<Rect>>>,
    callbacks: Rc<Cell<usize>>,
    arranged: bool,
}
impl Component for TranslationBox {
    fn host_reveal_passthrough(&self) -> bool { self.arranged && self.allowed.get() && self.child.len() == 1 }
    fn host_reveal_local_clip(&self) -> Option<Rect> { self.local.get() }
    fn children(&self) -> &[Content] { &self.child }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> { Some(&mut self.child) }
    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, _: Size) -> tekes_ui_controls::layout::MeasuredChild {
        tekes_ui_controls::layout::MeasuredChild::new(Size::new(200., 400.))
    }
    fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        // Original Scroll reserves focus bleed around its document child. This
        // full local domain includes that bleed and is NOT its clipped viewport.
        let domain=Rect::new(frame.origin.x-16.,frame.origin.y-16.,frame.size.width+32.,frame.size.height+32.);
        self.local.set(Some(domain));
        let visible=clip.intersection(&domain).unwrap_or_default();
        self.child[0].arrange(text, Rect::new(frame.origin.x+10.,frame.origin.y+300.,80.,30.),visible);
        self.arranged=true;
    }
    fn event(&mut self, _: &ContentEvent) -> ContentResponse {
        self.callbacks.set(self.callbacks.get()+1);ContentResponse::default()
    }
}
type Fixture = (ContentHost, Rc<Cell<bool>>, Rc<Cell<Option<Rect>>>, Rc<Cell<usize>>);
fn fixture(scroll:bool)->Fixture {
    let allowed=Rc::new(Cell::new(true));let local=Rc::new(Cell::new(None));let callbacks=Rc::new(Cell::new(0));
    let wrapper=Content::new(ComponentId(2),TranslationBox{child:vec![Content::new(ComponentId(3),Button::new("target"))],allowed:allowed.clone(),local:local.clone(),callbacks:callbacks.clone(),arranged:false});
    let root=if scroll{Content::new(ComponentId(1),ScrollContent::new(ScrollArea::default(),wrapper))}else{wrapper};
    let mut host=ContentHost::new(root).unwrap();render(&mut host,scroll);callbacks.set(0);(host,allowed,local,callbacks)
}
fn render(host:&mut ContentHost,scroll:bool){let r=Rect::new(0.,0.,200.,if scroll{100.}else{400.});host.render(&mut Scene::new(),&mut Text,r,r,Theme::LIGHT.into()).unwrap();}
fn plan(host:&ContentHost)->Result<HostRevealPlan,HostRevealError>{host.plan_host_reveal(ComponentId(3),ScrollRevealOptions{animated:false,..Default::default()},&[],HostRevealBudget::default())}
fn scroll_state(host:&ContentHost)->(Point,String){let s=host.get(ComponentId(1)).unwrap().component::<ScrollContent>().unwrap();(s.scroll.offset,format!("{:?}",s.scroll))}
#[test]
fn positive_qualification_expires_without_geometry_change_and_logical_focus_needs_no_opt_in(){
    let (mut host,allowed,_,callbacks)=fixture(true);let prepared=plan(&host).unwrap();
    let before_geometry:Vec<_>=(1..=3).map(|n|{let c=host.get(ComponentId(n)).unwrap();(c.mount_identity(),c.frame(),c.visible())}).collect();
    allowed.set(false);let before=format!("{host:?}");let scroll=scroll_state(&host);
    assert_eq!(host.commit_host_reveal(prepared),Err(HostRevealError::StalePlan));
    assert_eq!(format!("{host:?}"),before);assert_eq!(scroll_state(&host),scroll);assert_eq!(callbacks.get(),0);
    assert_eq!(before_geometry,(1..=3).map(|n|{let c=host.get(ComponentId(n)).unwrap();(c.mount_identity(),c.frame(),c.visible())}).collect::<Vec<_>>());
    assert_eq!(plan(&host).unwrap_err(),HostRevealError::UnsupportedLayout);
    let (mut logical,allowed,_,_)=fixture(false);allowed.set(false);
    assert!(logical.focus(Some(ComponentId(3)),true));assert_eq!(logical.focused(),Some(ComponentId(3)));
    let split=SplitViewContent::try_new(SplitViewConstruction{config:SplitViewConfig::default(),slots:SplitViewSlotIds{leading:ComponentId(21),trailing:ComponentId(22),divider:ComponentId(23)},leading:Content::new(ComponentId(11),Button::new("left")),trailing:Content::new(ComponentId(12),Button::new("right")),budget:64*1024*1024}).unwrap();
    assert!(!split.host_reveal_passthrough());
    let mut sdk=ContentHost::try_new_prepared(Content::new(ComponentId(10),split),64*1024*1024).unwrap();
    let bounds=Rect::new(0.,0.,600.,200.);sdk.render(&mut Scene::new(),&mut Text,bounds,bounds,Theme::LIGHT.into()).unwrap();
    assert!(sdk.get(ComponentId(10)).unwrap().component::<SplitViewContent>().unwrap().host_reveal_passthrough());
    let position=sdk.get(ComponentId(10)).unwrap().component::<SplitViewContent>().unwrap().layout().unwrap().divider.frame().center();
    sdk.event(&ContentEvent::PointerDown(position));sdk.event(&ContentEvent::PointerMove(Point::new(position.x+30.,position.y)));
    assert!(!sdk.get(ComponentId(10)).unwrap().component::<SplitViewContent>().unwrap().host_reveal_passthrough());
    sdk.render(&mut Scene::new(),&mut Text,bounds,bounds,Theme::LIGHT.into()).unwrap();
    assert!(sdk.get(ComponentId(10)).unwrap().component::<SplitViewContent>().unwrap().host_reveal_passthrough());
}
#[test]
fn invalid_local_clip_refuses_before_effects_and_some_zero_is_an_empty_domain(){
    let (mut host,_,local,callbacks)=fixture(true);let valid=local.get();
    for bad in [Rect::new(f32::NAN,0.,200.,400.),Rect::new(0.,f32::INFINITY,200.,400.),Rect::new(0.,0.,-1.,400.),Rect::new(0.,0.,200.,-1.),Rect::new(0.,0.,f32::INFINITY,400.),Rect::new(f32::MAX,0.,f32::MAX,400.),Rect::new(0.,f32::MAX,200.,f32::MAX)] {
        local.set(Some(bad));let before=format!("{host:?}");let scroll=scroll_state(&host);
        assert_eq!(plan(&host).unwrap_err(),HostRevealError::Geometry(tekes_ui_scene::ClipError::InvalidInput));
        assert_eq!(scroll_state(&host),scroll);assert_eq!(format!("{host:?}"),before);assert_eq!(callbacks.get(),0);
    }
    local.set(valid);let prepared=plan(&host).unwrap();local.set(Some(Rect::new(0.,0.,f32::NAN,400.)));let before=format!("{host:?}");
    assert_eq!(host.commit_host_reveal(prepared),Err(HostRevealError::StalePlan));assert_eq!(format!("{host:?}"),before);
    local.set(None);assert!(plan(&host).is_ok());
    for empty in [Rect::new(0.,0.,0.,400.),Rect::new(0.,0.,200.,0.)]{local.set(Some(empty));let before=format!("{host:?}");assert_eq!(plan(&host).unwrap_err(),HostRevealError::NoFeasibleRegion);assert_eq!(format!("{host:?}"),before);}
}
#[test]
fn original_scroll_translation_exposes_new_region_of_full_local_clip(){
    let (mut host,_,local,callbacks)=fixture(true);let initial=local.get().unwrap();let target=host.get(ComponentId(3)).unwrap();
    assert!(target.visible().is_none());assert!(initial.contains(target.frame().center()));assert!(initial.size.height>100.);
    let prepared=plan(&host).unwrap();assert!(prepared.planned_offsets().any(|(id,p)|id==ComponentId(1)&&p.y>0.));
    let result=host.commit_host_reveal(prepared).unwrap();assert!(result.revealed&&result.changed);assert!(scroll_state(&host).0.y>0.);assert_eq!(callbacks.get(),0);
    render(&mut host,true);let target=host.get(ComponentId(3)).unwrap();assert!(target.visible().is_some());
    let translated=local.get().unwrap();assert!(translated.origin.y<initial.origin.y);assert_eq!(translated.size,initial.size);assert!(translated.contains(target.frame().center()));
}
