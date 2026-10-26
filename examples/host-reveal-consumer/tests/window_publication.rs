//! Independent package exercises the public immutable window publication API.
use tekes_ui_controls::{composition::*,ButtonTextAlign,ButtonTextRenderer,Insets};
use tekes_ui_scene::{Rect,Scene,Size};
use tekes_ui_tokens::{Theme,Rgba,proposal};
use tekes_ui_platform_macos::{AccessibilityAction as Action,Point,content_accessibility::*};
struct Text;
impl ButtonTextRenderer for Text {
 fn measure(&mut self,s:&str,_:proposal::TextRole,w:f32)->Size {Size::new((s.len() as f32*7.).min(w),20.)}
 fn draw(&mut self,_:&mut Scene,_:&str,_:proposal::TextRole,_:Rect,_:ButtonTextAlign,_:Rgba){}
}
fn render(h:&mut ContentHost,x:f32) {h.render(&mut Scene::new(),&mut Text,Rect::new(x,0.,100.,100.),Rect::new(0.,0.,300.,100.),Theme::LIGHT.into()).unwrap();}
#[test]
fn independent_window_publication_preserves_original_leases_aliases_and_exact_owner() {
 let mut p=PanelContent::new(Content::new(ComponentId(2),Button::new("curve")).preserving_focus()).unwrap();
 p.set_padding(Some(Insets::new(12.,12.,12.,12.))).unwrap();
 let mut curve=ContentHost::new(Content::new(ComponentId(1),p)).unwrap();
 let mut plain=ContentHost::new(Content::new(ComponentId(3),Button::new("plain")).preserving_focus()).unwrap();
 render(&mut curve,0.);render(&mut plain,120.);
 let a=ContentAccessibilitySnapshot::from_host(&curve).unwrap();
 let b=ContentAccessibilitySnapshot::from_host(&plain).unwrap();
 let key=content_key(&plain,ComponentId(3),None).unwrap();
 let alias=tekes_ui_platform_macos::AccessibilityKey{component:u64::MAX,part:None};
 let b=b.with_native_keys(&[(key,alias)].into_iter().collect()).unwrap();
 let pubn=WindowAccessibilityPublication::prepare(vec![(ContentAccessibilityOwner(1),a),(ContentAccessibilityOwner(2),b)]).unwrap();
 assert_eq!(pubn.hit_test(Point{x:170.,y:50.}),Some(alias));
 assert!(pubn.hit_test(Point{x:0.5,y:0.5}).is_none());
 assert!(!pubn.dispatch(ContentAccessibilityOwner(1),&mut plain,alias,Action::Press).consumed);
 assert!(pubn.dispatch(ContentAccessibilityOwner(2),&mut plain,alias,Action::Press).consumed);
 let old=pubn.clone();let mut store=Some(pubn);
 assert!(!WindowAccessibilityPublication::commit(&mut store,None));
 assert!(!old.is_current());assert!(old.owner(alias).is_none());
 render(&mut curve,0.);render(&mut plain,120.);
 let next=WindowAccessibilityPublication::prepare(vec![(ContentAccessibilityOwner(1),ContentAccessibilitySnapshot::from_host(&curve).unwrap()),(ContentAccessibilityOwner(2),ContentAccessibilitySnapshot::from_host(&plain).unwrap())]).unwrap();
 assert!(WindowAccessibilityPublication::commit(&mut store,Some(next)));
 assert!(!old.is_current());
}

#[test]
fn checked_split_mutations_revoke_the_original_window_publication_only_on_commit() {
 use tekes_ui_controls::extended::Pane;
 let budget=64*1024*1024;
 let root=Content::new(ComponentId(100),SplitViewContent::try_new(SplitViewConstruction{
  config:SplitViewConfig::default(),slots:SplitViewSlotIds{leading:ComponentId(101),trailing:ComponentId(102),divider:ComponentId(103)},
  leading:Content::new(ComponentId(110),Button::new("leading")),trailing:Content::new(ComponentId(120),Button::new("trailing")),budget,
 }).unwrap());
 let mut host=ContentHost::try_new_prepared(root,budget).unwrap();
 let paint=|h:&mut ContentHost|{let rect=Rect::new(0.,0.,640.,240.);h.render_checked(&mut Scene::new(),&mut Text,rect,rect,Theme::LIGHT.into()).unwrap();};
 paint(&mut host);
 let publish=|h:&ContentHost|WindowAccessibilityPublication::prepare(vec![(ContentAccessibilityOwner(1),ContentAccessibilitySnapshot::from_host(h).unwrap())]).unwrap();
 let old=publish(&host);let expected=host.split_view_expected(ComponentId(100)).unwrap();
 let config=host.get(ComponentId(100)).unwrap().component::<SplitViewContent>().unwrap().config().unwrap();
 assert!(!host.update_split_view(ComponentId(100),expected,config,budget).unwrap());
 assert!(old.is_current());
 let mut bad=config;bad.ratio=f32::NAN;
 assert!(host.update_split_view(ComponentId(100),expected,bad,budget).is_err());
 assert!(old.is_current());
 let mut next=config;next.ratio=0.7;
 assert!(host.update_split_view(ComponentId(100),expected,next,budget).unwrap());
 assert!(!old.is_current());assert!(old.nodes().is_empty());
 assert!(ContentAccessibilitySnapshot::from_host(&host).is_none(),"changed layout must not be re-signed before arrange");
 paint(&mut host);let before_replace=publish(&host);
 let expected=host.split_view_expected(ComponentId(100)).unwrap();
 host.replace_split_view_pane(SplitViewReplacement{owner:ComponentId(100),expected,pane:Pane::Trailing,
  replacement:Content::new(ComponentId(130),Button::new("replacement")),budget}).unwrap();
 assert!(!before_replace.is_current());assert!(before_replace.owner(content_key(&host,ComponentId(110),None).unwrap()).is_none());
 assert!(ContentAccessibilitySnapshot::from_host(&host).is_none());
 paint(&mut host);let recovery=publish(&host);
 assert!(recovery.is_current());assert!(!old.is_current());assert!(!before_replace.is_current());
}
