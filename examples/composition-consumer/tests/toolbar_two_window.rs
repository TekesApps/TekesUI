#![cfg(target_os = "macos")]
#[path = "support/coretext.rs"] mod coretext;
use coretext::Text;
use tekes_ui_controls::{composition::*, ControlKey, ToolbarControl};
use tekes_ui_scene::{Rect, Scene};
use tekes_ui_tokens::Theme;
const VIEW: Rect = Rect::new(0.,0.,1000.,300.);
const ROOT: ComponentId = ComponentId(1);
const REGION: ComponentId = ComponentId(2);
const OTHER: ComponentId = ComponentId(3);
const GROUP: ComponentId = ComponentId(10);
const COMMAND: ComponentId = ComponentId(11);
const EDIT_GROUP: ComponentId = ComponentId(20);
const EDIT: ComponentId = ComponentId(21);
#[derive(Clone,Debug,PartialEq,Eq)] struct Source { window:u64, generation:u64 }
fn declaration(window:u64,generation:u64)->ToolbarRegistration<Source> {
 ToolbarRegistration { region:REGION,order:0,
 group:Content::new(GROUP,ToolbarGroup::new(ToolbarPlacement::Navigation,vec![toolbar_commands(COMMAND,vec![ToolbarButtonDeclaration {id:7,icon:None,text:"Execute".into(),hint:None,enabled:true}],ToolbarGroupOverflow::Together)])),
 bindings:vec![ToolbarCommandBinding{target:COMMAND,action:Some(7),command:7,source:Source{window,generation}}] }
}
fn build(window:u64)->(ToolbarHost<Source>,Text) {
 let mut h=ToolbarHost::new(ROOT,window,VIEW).unwrap();
 h.set_regions(window,VIEW,vec![ToolbarRegion::new(REGION,Rect::new(0.,0.,500.,80.)),ToolbarRegion::new(OTHER,Rect::new(500.,0.,500.,80.))]).unwrap();
 h.register(declaration(window,1)).unwrap();
 h.register(ToolbarRegistration{region:OTHER,order:0,group:Content::new(EDIT_GROUP,ToolbarGroup::new(ToolbarPlacement::Navigation,vec![Content::new(EDIT,TextFieldContent::inline("Search","",12.))])),bindings:vec![]}).unwrap();
 let mut text=Text::new(1.);text.services=true;draw(&mut h,&mut text);(h,text)
}
fn draw(h:&mut ToolbarHost<Source>,t:&mut Text) { h.render(&mut Scene::new(),t,Theme::LIGHT.into()).unwrap(); }
fn target(h:&ToolbarHost<Source>)->(ToolbarAccessibilityTarget,Rect) {
 h.semantics().into_iter().find(|n|n.node.id==COMMAND).unwrap().parts.into_iter().find(|(_,p)|p.semantics.name=="Execute").map(|(t,p)|(t,p.frame)).unwrap()
}
fn click(h:&mut ToolbarHost<Source>,r:Rect){h.event(&ContentEvent::PointerDown(r.center()));h.event(&ContentEvent::PointerUp(r.center()));}
fn assert_command(h:&mut ToolbarHost<Source>,window:u64,generation:u64) {
 let commands=h.drain_commands();assert_eq!(commands.len(),1);assert_eq!(commands[0].window,window);assert_eq!(commands[0].source,Source{window,generation});assert_eq!(commands[0].group,GROUP);assert_eq!(commands[0].command,7);
}
#[test]
fn cross_window_opaque_ax_targets_must_not_alias() {
 let(mut a,_ta)=build(101);let(mut b,_tb)=build(202);
 let(at,_)=target(&a);let(bt,_)=target(&b);
 assert_ne!(at,bt,"same component IDs must retain distinct opaque mount identities");
 let aedit=a.semantics().into_iter().find(|n|n.node.id==EDIT).unwrap().target;
 let bedit=b.semantics().into_iter().find(|n|n.node.id==EDIT).unwrap().target;
 assert!(!b.accessibility_action(aedit,AccessibilityAction::Focus).consumed);
 assert!(!a.accessibility_action(bedit,AccessibilityAction::Focus).consumed);
 assert_eq!(a.content_host().focused(),None);assert_eq!(b.content_host().focused(),None);
 assert!(a.accessibility_action(at,AccessibilityAction::Press).activated);assert_command(&mut a,101,1);
 assert!(b.accessibility_action(bt,AccessibilityAction::Press).activated);assert_command(&mut b,202,1);
 // Opaque target captured from A, handed unchanged to B through public APIs.
 let response=b.accessibility_action(at,AccessibilityAction::Press);
 let commands=b.drain_commands();
 assert!(!response.activated && commands.is_empty(),"FOREIGN AX ACCEPTED: A={at:?}; B={bt:?}; response={response:?}; commands={commands:?}");
 assert!(!a.accessibility_action(bt,AccessibilityAction::Press).activated);
 assert!(a.drain_commands().is_empty());
}
#[test]
fn local_lifecycle_invalidation_does_not_touch_other_live_host() {
 for case in ["update","replace","hide","move","remove","deactivate","close_regions","rebind_window"] {
  eprintln!("lifecycle case: {case}");
  let(mut a,mut ta)=build(101);let(mut b,mut tb)=build(202);
  let(at,ar)=target(&a);let(bt,br)=target(&b);
  let am=a.content_host().semantic_mount(COMMAND);let bm=b.content_host().semantic_mount(COMMAND);
  // Both queues remain pending until the local A transition.
  click(&mut a,ar);click(&mut b,br);
  a.event(&ContentEvent::PointerDown(ar.center()));
  b.event(&ContentEvent::PointerDown(br.center()));
  match case {
   "update"=>a.update_component::<ToolbarControl>(GROUP,COMMAND,|c|{c.set_action_help(7,Some("Updated".into()));}).unwrap(),
   "replace"=>a.register(declaration(101,2)).unwrap(),
   "hide"=>a.set_visible(GROUP,false).unwrap(),
   "move"=>a.move_group(GROUP,OTHER,ToolbarPlacement::PrimaryAction,1).unwrap(),
   "remove"=>{a.remove(GROUP).unwrap();},
   "deactivate"=>a.set_window_active(false),
   "close_regions"=>a.set_regions(101,VIEW,vec![]).unwrap(),
   "rebind_window"=>a.set_regions(303,VIEW,vec![ToolbarRegion::new(REGION,Rect::new(0.,0.,500.,80.)),ToolbarRegion::new(OTHER,Rect::new(500.,0.,500.,80.))]).unwrap(),
   _=>unreachable!()
  }
  assert!(a.drain_commands().is_empty(),"A stale queue: {case}");
  assert_command(&mut b,202,1);
  assert!(!a.accessibility_action(at,AccessibilityAction::Press).activated,"before render: {case}");
  draw(&mut a,&mut ta);draw(&mut b,&mut tb);
  a.event(&ContentEvent::PointerUp(ar.center()));
  assert!(a.drain_commands().is_empty(),"late release: {case}");
  assert!(!a.accessibility_action(at,AccessibilityAction::Press).activated,"after render: {case}");
  assert_eq!(b.content_host().semantic_mount(COMMAND),bm);
  assert_eq!(target(&b).0,bt,"B revision changed: {case}");
  b.event(&ContentEvent::PointerUp(br.center()));assert_command(&mut b,202,1);
  if case=="replace" {assert_ne!(a.content_host().semantic_mount(COMMAND),am);assert!(a.accessibility_action(target(&a).0,AccessibilityAction::Press).activated);assert_command(&mut a,101,2);}
  if case=="hide" {assert!(a.content_host().get(COMMAND).is_some());assert_eq!(a.content_host().semantic_mount(COMMAND),None);a.set_visible(GROUP,true).unwrap();draw(&mut a,&mut ta);assert_eq!(a.content_host().semantic_mount(COMMAND),am);assert!(!a.accessibility_action(at,AccessibilityAction::Press).activated);}
  if case=="deactivate" {a.set_window_active(true);draw(&mut a,&mut ta);assert!(!a.accessibility_action(at,AccessibilityAction::Press).activated);}
 }
}
#[test]
fn focus_text_capture_and_drop_stay_in_originating_host() {
 let(mut a,mut ta)=build(101);let(mut b,mut tb)=build(202);
 let ae=a.content_host().get(EDIT).unwrap().frame();let be=b.content_host().get(EDIT).unwrap().frame();
 click(&mut a,ae);a.event_with_text(&ContentEvent::Commit{text:"A中文".into(),replacement:None},&mut ta).unwrap();
 assert_eq!(a.content_host().focused(),Some(EDIT));assert_eq!(b.content_host().focused(),None);
 click(&mut b,be);b.event_with_text(&ContentEvent::Commit{text:"B中文".into(),replacement:None},&mut tb).unwrap();
 assert_eq!(a.content_host().get(EDIT).unwrap().component::<TextFieldContent>().unwrap().surface.input.text,"A中文");
 assert_eq!(b.content_host().get(EDIT).unwrap().component::<TextFieldContent>().unwrap().surface.input.text,"B中文");
 a.set_window_active(false);assert_eq!(b.content_host().focused(),Some(EDIT));
 a.set_window_active(true);draw(&mut a,&mut ta);assert_eq!(a.content_host().focused(),None);assert_eq!(b.content_host().focused(),Some(EDIT));
 let(at,ar)=target(&a);let(bt,br)=target(&b);a.event(&ContentEvent::PointerDown(ar.center()));
 b.event(&ContentEvent::PointerUp(br.center()));assert!(b.drain_commands().is_empty());
 a.event(&ContentEvent::PointerUp(ar.center()));assert_command(&mut a,101,1);
 b.event(&ContentEvent::Key{key:ControlKey::Escape,down:true});
 drop(a); // Object lifetime ends; this is not an NSWindow close receipt.
 assert!(!b.accessibility_action(at,AccessibilityAction::Press).activated);assert!(b.drain_commands().is_empty());
 assert!(b.accessibility_action(bt,AccessibilityAction::Press).activated);assert_command(&mut b,202,1);
}


#[test]
fn original_owner_restore_authenticates_window_mount_and_visibility_without_part_promotion() {
    let (mut a, mut ta) = build(101);
    let (mut b, _) = build(202);
    let owner = a.semantics().into_iter().find(|n| n.node.id == COMMAND).unwrap().target;
    let part = target(&a).0;
    assert!(!a.restore_focus(part, false), "a semantic part cannot silently become its owner");
    assert!(!b.restore_focus(owner, false), "foreign window owner must remain rejected");
    assert_eq!(a.content_host().focused(), None);
    assert_eq!(b.content_host().focused(), None);
    assert!(a.restore_focus(owner, false));
    assert_eq!(a.content_host().focused(), Some(COMMAND));
    let mut pointer_focus = Scene::new();
    a.render(&mut pointer_focus, &mut ta, Theme::LIGHT.into()).unwrap();
    assert!(a.restore_focus(owner, true));
    let mut keyboard_focus = Scene::new();
    a.render(&mut keyboard_focus, &mut ta, Theme::LIGHT.into()).unwrap();
    assert_ne!(pointer_focus, keyboard_focus, "restoration preserves original focus-visible presentation");
    assert!(a.drain_commands().is_empty());
    a.set_window_active(false);
    assert!(!a.restore_focus(owner, false));
    a.set_window_active(true);
    draw(&mut a, &mut ta);
    assert!(!a.restore_focus(owner, false), "retained revision cannot recover after activity change");
    let current = a.semantics().into_iter().find(|n| n.node.id == COMMAND).unwrap().target;
    assert!(a.restore_focus(current, false));
    a.remove(GROUP).unwrap();
    draw(&mut a, &mut ta);
    assert!(!a.restore_focus(current, false), "removed original mount cannot be restored");
    assert!(a.drain_commands().is_empty());
}
