//! Ignored frozen-source validation helper. Reads real Gallery hit geometry and models only.
use super::super::{Gallery, Page};
use tekes_ui_platform_macos::{Event, KeyEvent, KeyEventKind, Modifiers, MouseButton, MouseEvent, MouseEventKind, Point as NativePoint};
use tekes_ui_scene::{Point,Rect};
fn index(g: &Gallery) -> usize {match g.page(){Page::Workbench(i)=>i,_=>panic!("expected workbench")}}
fn frame(g:&mut Gallery,id:usize)->Rect {let i=index(g);g.workbench.state(i).hits.iter().find(|(n,_)|*n==id).expect("actual rendered hit").1}
fn mouse(g:&mut Gallery,p:Point,kind:MouseEventKind){g.handle_input(&Event::Mouse(MouseEvent{kind,position:NativePoint{x:f64::from(p.x),y:f64::from(p.y)},modifiers:Modifiers::NONE}));}
pub fn interact(g:&mut Gallery,kind:&str){
 let id=if kind=="radio" {32}else{20};let r=frame(g,id);let start=r.center();
 mouse(g,start,MouseEventKind::Moved);
 mouse(g,start,MouseEventKind::Down{button:MouseButton::Left,click_count:1});
 let end=if kind=="slider" {Point::new(r.min_x()+r.size.height*0.5+(r.size.width-r.size.height)*0.75,r.center().y)}else{start};
 mouse(g,end,MouseEventKind::Moved);
 mouse(g,end,MouseEventKind::Up{button:MouseButton::Left,click_count:1});
 mouse(g,Point::new(1400.0,940.0),MouseEventKind::Moved);
 mouse(g,Point::new(1400.0,940.0),MouseEventKind::Moved);
 if std::env::var_os("SELECTION_PROBE_KEYBOARD").is_some(){
  for reverse in [true,false]{for kind in [KeyEventKind::Down,KeyEventKind::Up]{
   let mut modifiers=Modifiers::NONE;modifiers.shift=reverse;
   g.handle_input(&Event::Key(KeyEvent{kind,key_code:48,characters:String::new(),characters_ignoring_modifiers:String::new(),modifiers,is_repeat:false}));
  }}
 }

}
pub fn state(g:&mut Gallery,kind:&str)->serde_json::Value{let i=index(g);let s=&g.workbench.state(i).extended;match kind{
 "slider"=>serde_json::json!({"value":s.slider.value()}),
 "radio"=>serde_json::json!({"selected":s.radio.selected}),
 "switch"=>serde_json::json!({"checked":s.switch.checked}),_=>unreachable!()}}
