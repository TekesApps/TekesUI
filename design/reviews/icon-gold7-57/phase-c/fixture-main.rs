mod coretext;
use tekes_ui_controls::{ButtonTextRenderer,ButtonTextAlign,composition::*,layout_view::IconView,extended::{Axis,Stack,StatusBadge}};
use tekes_ui_scene::{Rect,Scene};
use tekes_ui_tokens::{Palette,Theme,proposal,surface_palette::resolve_surface};
const NAMES:[&str;12]=["tekes-back","tekes-forward","tekes-settings","tekes-search","tekes-folder","tekes-file-generic","tekes-remove-one","tekes-new","tekes-info","tekes-goal-completed","tekes-resume","tekes-error"];
fn node(id:u64,c:impl Component+'static)->Content {Content::new(ComponentId(id),c)}
fn icon(id:u64,name:&str)->Content {node(id,IconContent::new(name).unwrap())}
fn action(id:u64,name:&str)->Content {node(id,Button::new(icon(id+1,name)).with_accessible_name(name))}
fn row(id:u64,name:&str,label:&str)->Content {node(id,StackContent{model:Stack{axis:Axis::Horizontal,gap:10.},flexible_child:None,children:vec![icon(id+1,name),node(id+2,TextContent::new(label))]})}
fn page()->ContentHost {
 let mut failed=IconContent::new("tekes-error").unwrap();failed.set_status_tone(Some(StatusBadge::Failed));
 let mut ok=IconContent::new("tekes-goal-completed").unwrap();ok.set_status_tone(Some(StatusBadge::Succeeded));
 let toolbar=node(2,ToolbarContent::new(vec![
  node(3,ToolbarGroup::new(ToolbarPlacement::Navigation,vec![action(4,"tekes-back"),action(6,"tekes-forward")])),
  node(8,ToolbarGroup::new(ToolbarPlacement::Principal,vec![node(9,TextContent::new("Icon direction · Review"))])),
  node(10,ToolbarGroup::new(ToolbarPlacement::SecondaryAction,vec![action(11,"tekes-settings")])),
 ]));
 ContentHost::new(node(1,StackContent{model:Stack{axis:Axis::Vertical,gap:18.},flexible_child:None,children:vec![
  toolbar,
  node(20,TextContent::new("Workspace")),
  node(21,StackContent{model:Stack{axis:Axis::Horizontal,gap:10.},flexible_child:Some(ComponentId(23)),children:vec![icon(22,"tekes-search"),node(23,TextFieldContent::inline("Search files","Design review",16.)),action(24,"tekes-remove-one")]}),
  node(30,ListContent::new("Files",vec![row(31,"tekes-folder","Design sources"),row(34,"tekes-file-generic","Icon guidelines"),row(37,"tekes-info","Canonical identity preserved")])),
  node(40,TextContent::new("Status attachments")),
  node(41,StackContent{model:Stack{axis:Axis::Horizontal,gap:10.},flexible_child:None,children:vec![node(42,ok),node(43,TextContent::new("Completed")),icon(44,"tekes-resume"),node(45,TextContent::new("Run")),node(46,failed)]}),
  node(50,TextContent::new("Original Button · passive canonical IconContent")),
  node(51,StackContent{model:Stack{axis:Axis::Horizontal,gap:12.},flexible_child:None,children:vec![action(52,"tekes-new"),action(54,"tekes-settings"),action(56,"tekes-remove-one")]}),
 ]})).unwrap()
}
fn main(){
 let out=std::path::PathBuf::from(std::env::args().nth(1).expect("output directory"));std::fs::create_dir_all(&out).unwrap();
 let mut gpu=tekes_ui_render::Renderer::new_headless().expect("actual Metal GPU");let mut cache=SvgIconCache::default();let mut receipts=vec![];
 for (ti,theme) in Theme::ALL.into_iter().enumerate(){for backing in [1.,2.]{
  let p=Palette::from(theme);let colors=resolve_surface(p,"canvas","content","rest").unwrap();let mut text=coretext::Text::new(1.);text.backing=backing;
  let mut scene=Scene::new();text.draw(&mut scene,"Original size: 12 / 16 / 24 pt",proposal::TextRole::UiBody,Rect::new(20.,8.,600.,32.),ButtonTextAlign::Leading,colors.text);
  {let mut icons=cache.renderer(&mut gpu,backing);for(i,name)in NAMES.into_iter().enumerate(){let y=50.+i as f32*43.;text.draw(&mut scene,name,proposal::TextRole::UiBody,Rect::new(20.,y,280.,32.),ButtonTextAlign::Leading,colors.text);for(j,size)in [12.,16.,24.].into_iter().enumerate(){IconView::default().render(&mut scene,&mut icons,name,Rect::new(320.+j as f32*90.,y,40.,32.),size,None,p);}}}
  let size=((640.*backing)as u32,(590.*backing)as u32);let rgba=gpu.render_to_rgba8(&scene,&mut text.atlas,size,backing,colors.background).unwrap();save(&out.join(format!("sheet-t{ti}-b{backing}.png")),rgba,size);
  for brand in [0,1]{for narrow in [false,true]{for scale in [1.,1.5]{
   let p=Palette::new(theme,brand).unwrap();let bg=resolve_surface(p,"canvas","content","rest").unwrap().background;let width=if narrow{520.}else{900.};let height=800.;
   let mut host=page();let mut text=coretext::Text::new(scale);text.backing=backing;text.services=true;let mut scene=Scene::new();let f=Rect::new(20.,20.,width-40.,height-40.);
   {let mut icons=cache.renderer(&mut gpu,backing);host.render(&mut scene,&mut WithIconRenderer::new(&mut text,&mut icons),f,f,p).expect("original shared components");}
   assert!(!host.focus_targets().is_empty());
   let size=((width*backing)as u32,(height*backing)as u32);let rgba=gpu.render_to_rgba8(&scene,&mut text.atlas,size,backing,bg).unwrap();
   let file=format!("page-t{ti}-b{backing}-brand{brand}-n{narrow}-s{scale}.png");save(&out.join(&file),rgba,size);
   receipts.push(serde_json::json!({"file":file,"theme":ti,"brand":brand,"backing":backing,"type_scale":scale,"width":width,"renderer":"shared SvgIconCache + IconView/IconContent + Metal/CoreText","native_window":false,"physical_input":false,"voiceover":false,"full_gold7_pass":false}));
  }}}
 }}
 std::fs::write(out.join("receipt.json"),serde_json::to_vec_pretty(&receipts).unwrap()).unwrap();println!("8 actual-size sheets + {} shared-component GPU pages",receipts.len());
}
fn save(path:&std::path::Path,rgba:Vec<u8>,size:(u32,u32)){resvg::tiny_skia::Pixmap::from_vec(rgba,resvg::tiny_skia::IntSize::from_wh(size.0,size.1).unwrap()).unwrap().save_png(path).unwrap();}
