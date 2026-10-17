#[path = "../gallery/mod.rs"] pub mod gallery;
use gallery::{Gallery,Command};
use gallery::workbench::probe as p;
use tekes_ui_render::Renderer;
use tekes_ui_scene::Scene;
use tekes_ui_tokens::Theme;
use std::{fs,path::PathBuf};
fn layout(g:&mut Gallery,r:&mut Renderer){g.prepare(r,1.);g.build_scene(&mut Scene::new());}
fn main(){let out=PathBuf::from(std::env::args_os().nth(1).expect("output dir"));fs::create_dir_all(&out).unwrap();let mut r=Renderer::new_headless().unwrap();let cases=[("disclosure",20),("split-view",90),("icon-button",20),("link",20),("segmented-control",31),("breadcrumb",31),("combo-box",1000),("command-palette",20),("select",20),("list",31),("tree",31),("data-table",21),("chart",31),("tooltip",20),("popover",20),("toast",20),("banner",20),("progress",20),("empty-state",20),("toolbar",20),("number-field",24)];let mut receipts=vec![];
for (theme,width,height,ts,label) in [(Theme::LIGHT,1440,960,0,"light"),(Theme::DARK,1440,960,0,"dark"),(Theme::LIGHT,1000,800,2,"large-narrow")] {for (kind,id) in cases {let mut g=Gallery::new();assert!(g.open_route(&format!("component-{kind}/playground")),"route {kind}");g.set_view_size(width as f32,height as f32);g.apply(Command::TextScale(ts));g.set_system_theme(theme);layout(&mut g,&mut r);let before=p::state(&mut g,kind);
match kind {"chart"|"tooltip"=>{let c=p::center(&mut g,id);p::moved(&mut g,c);},"split-view"=>p::drag(&mut g,id),_=>p::click(&mut g,id)};layout(&mut g,&mut r);
if matches!(kind,"combo-box"|"command-palette"|"select"){p::press(&mut g,125);layout(&mut g,&mut r);p::press(&mut g,36);layout(&mut g,&mut r);}
let after=p::state(&mut g,kind);let passed = before["value"] != after["value"];
let mut scene=Scene::new();g.build_scene(&mut scene);let rgba=r.render_to_rgba8(&scene,g.atlas_mut(),(width,height),1.,Gallery::background(theme)).unwrap();let stem=format!("{kind}-{label}");let file=fs::File::create(out.join(format!("{stem}.png"))).unwrap();let mut e=png::Encoder::new(file,width,height);e.set_color(png::ColorType::Rgba);e.set_depth(png::BitDepth::Eight);e.write_header().unwrap().write_image_data(&rgba).unwrap();receipts.push(serde_json::json!({"id":stem,"passed":passed,"before":before,"after":after,"viewport":[width,height],"text_scale":([100,125,150][ts])}));fs::write(out.join("manifest.json"),serde_json::to_vec_pretty(&receipts).unwrap()).unwrap();eprintln!("{passed} {stem}");}}
}
