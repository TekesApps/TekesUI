//! Target-only probe: drives the real Gallery input path, never product code.
use super::super::{Gallery, Page};
use tekes_ui_platform_macos::{Event,KeyEvent,KeyEventKind,Modifiers,MouseButton,MouseEvent,MouseEventKind,Point as NativePoint};
use tekes_ui_scene::Point;
fn index(g:&Gallery)->usize { if let Page::Workbench(i)=g.page(){i}else{panic!("workbench expected")} }
pub fn center(g:&mut Gallery,id:usize)->Point { let i=index(g);g.workbench.state(i).hits.iter().rev().find(|(n,_)|*n==id).unwrap_or_else(||panic!("missing hit {id}")).1.center() }
pub fn mouse(g:&mut Gallery,p:Point,kind:MouseEventKind){g.handle_input(&Event::Mouse(MouseEvent{kind,position:NativePoint{x:p.x as f64,y:p.y as f64},modifiers:Modifiers::NONE}));}
pub fn moved(g:&mut Gallery,p:Point){mouse(g,p,MouseEventKind::Moved);}
pub fn click(g:&mut Gallery,id:usize){let p=center(g,id);moved(g,p);for kind in [MouseEventKind::Down{button:MouseButton::Left,click_count:1},MouseEventKind::Up{button:MouseButton::Left,click_count:1}]{mouse(g,p,kind);}}
pub fn press(g:&mut Gallery,code:u16){for kind in [KeyEventKind::Down,KeyEventKind::Up]{g.handle_input(&Event::Key(KeyEvent{kind,key_code:code,characters:String::new(),characters_ignoring_modifiers:String::new(),modifiers:Modifiers::NONE,is_repeat:false}));}}
pub fn drag(g:&mut Gallery,id:usize){let p=center(g,id);mouse(g,p,MouseEventKind::Down{button:MouseButton::Left,click_count:1});let q=Point::new(p.x+36.,p.y);moved(g,q);mouse(g,q,MouseEventKind::Up{button:MouseButton::Left,click_count:1});}
pub fn state(g:&mut Gallery,kind:&str)->serde_json::Value{let i=index(g);let s=g.workbench.state(i);let x=&s.extended;let value=match kind {
 "disclosure"=>serde_json::json!(x.disclosure.open),"split-view"=>serde_json::json!(x.split.ratio()),"segmented-control"=>serde_json::json!(x.segmented.selected),"breadcrumb"=>serde_json::json!(x.selected),
 "combo-box"=>serde_json::json!({"open":x.combo.open,"query":s.input.query(),"selected":x.combo.selected}),"command-palette"=>serde_json::json!({"open":x.command.combo.open,"selected":x.command.combo.selected}),
 "select"=>serde_json::json!({"open":s.select.menu.open,"selected":s.select.selected}),"number-field"=>serde_json::json!(s.number.value()),"list"=>serde_json::json!(x.list.selected),"tree"=>serde_json::json!(x.tree.selected),"data-table"=>serde_json::json!({"sort":format!("{:?}",x.table.sort),"selected":x.table.selected_row}),"chart"=>serde_json::json!(x.chart.hovered()),
 "tooltip"=>serde_json::json!(format!("{:?}",x.feedback_views.tooltip.layout().visible)),"popover"=>serde_json::json!(x.popover.open),"toast"=>serde_json::json!(x.toast.visible),"banner"=>serde_json::json!(x.banner.visible),"progress"=>serde_json::json!(x.progress.value()),"empty-state"=>serde_json::json!(x.empty_state_created),_=>serde_json::json!(s.log)};serde_json::json!({"value":value,"focus":s.focus,"events":s.log})}
