#![cfg(target_os="macos")]
#[path="support/coretext.rs"]mod coretext;
use tekes_ui_controls::{composition::*,feedback_view::SkeletonView};
use tekes_ui_scene::{Rect,Scene,Quad};
use tekes_ui_tokens::{Palette,Theme,surface_palette::{resolve_surface,surface_names}};

fn external_theme_requested() -> bool {
    match std::env::var_os("TEKESUI_SKELETON_EXTERNAL") {
        None => false,
        Some(value) if value.to_str() == Some("0") => false,
        Some(value) if value.to_str() == Some("1") => true,
        Some(value) => panic!("TEKESUI_SKELETON_EXTERNAL must be 0 or 1, got {value:?}"),
    }
}
fn palettes()->Vec<(String,Palette)>{
 let external=external_theme_requested();let mut out=vec![];
 for(mode,theme)in Theme::ALL.into_iter().enumerate(){if external{for id in ["blue","blue-inputs"]{out.push((format!("{id}-mode{mode}"),Palette::for_theme_definition(theme,id).unwrap()));}}else{for brand in [0,1,2]{out.push((format!("brand{brand}-mode{mode}"),Palette::new(theme,brand).unwrap()));}}}out
}
#[test]
#[ignore = "explicit GPU acceptance requires a fresh SSD output directory"]
fn actual_gpu_packet_visibility_original_view_clip_and_fractional_allocations(){
 let out=std::path::PathBuf::from(std::env::var_os("TEKESUI_SKELETON_GPU_OUT").expect("fresh evidence output"));std::fs::create_dir(&out).unwrap();
 let mut gpu=tekes_ui_render::Renderer::new_headless().unwrap();let mut cases=vec![];let mut legacy_invisible=0;let mut additional_frames=0;
 for(label,palette)in palettes(){for surface in surface_names(){for backing in [1.,2.]{
  let packet=resolve_surface(palette,surface,"skeleton","rest").unwrap();let ground=resolve_surface(palette,surface,"content","rest").unwrap().background;
  for(name,f,clip,positive)in [
   ("normal",Rect::new(12.,14.,60.,16.),Rect::new(0.,0.,96.,64.),true),
   ("fractional",Rect::new(12.25,14.5,60.5,16.25),Rect::new(0.,0.,96.,64.),true),
   ("narrow",Rect::new(12.25,14.5,2.5,16.25),Rect::new(0.,0.,96.,64.),true),
   ("subpixel",Rect::new(12.25,14.5,0.75,16.25),Rect::new(0.,0.,96.,64.),true),
   ("partial",Rect::new(12.25,14.5,60.5,16.25),Rect::new(24.,14.,40.,10.),true),
   ("interiorclip",Rect::new(12.25,14.5,60.5,16.25),Rect::new(24.,18.,40.,10.),packet.background!=ground),
   ("parentclipempty",Rect::new(12.25,14.5,60.5,16.25),Rect::new(12.25,15.,0.5,10.),false),
   ("fullclip",Rect::new(12.25,14.5,60.5,16.25),Rect::new(80.,40.,10.,10.),false),
   ("zero",Rect::new(12.25,14.5,0.,0.),Rect::new(0.,0.,96.,64.),false),
  ]{
   let mut text=coretext::Text::new(1.);text.backing=backing;
   let mut host=ContentHost::new(Content::new(ComponentId(1),SkeletonContent::default()).with_surface(surface).unwrap()).unwrap();let mut actual=Scene::new();host.render(&mut actual,&mut text,f,clip,palette).unwrap();assert!(!text.requested);
   let mut v=SkeletonView::default();v.set_surface(surface).unwrap();let mut expected=Scene::new();let layout=v.render(&mut expected,f,Some(clip),palette);
   let size=((96.*backing)as u32,(64.*backing)as u32);
   let empty=gpu.render_to_rgba8(&Scene::new(),&mut text.atlas,size,backing,ground).unwrap();
   let rgba=gpu.render_to_rgba8(&actual,&mut text.atlas,size,backing,ground).unwrap();let oracle=gpu.render_to_rgba8(&expected,&mut text.atlas,size,backing,ground).unwrap();assert_eq!(rgba,oracle,"{label}/{surface}/{name}@{backing}");
   let mut ink=0;let mut outside=0;let mut ancestor_outside=0;let visible=layout.visible;
   for(i,(a,b))in rgba.chunks_exact(4).zip(empty.chunks_exact(4)).enumerate(){if a!=b{ink+=1;let x=(i as u32%size.0)as f32/backing;let y=(i as u32/size.0)as f32/backing;
     let px=i as u32%size.0;let py=i as u32/size.0;
     let left=(clip.min_x()*backing).ceil().max(0.) as u32;
     let top=(clip.min_y()*backing).ceil().max(0.) as u32;
     let right=(clip.max_x()*backing).floor().max(0.) as u32;
     let bottom=(clip.max_y()*backing).floor().max(0.) as u32;
     if px<left||px>=right||py<top||py>=bottom{ancestor_outside+=1;}
     // Renderer scissor rounds inward to integer pixel bounds. Check the entire
     // pixel footprint against the logical visible region, with at most the
     // original renderer's subpixel edge coverage (no arbitrary halo).
     if !visible.is_some_and(|r|x+1./backing>r.min_x()&&x<r.max_x()&&y+1./backing>r.min_y()&&y<r.max_y()){outside+=1;}
   }}
   let evidence_key=format!("{label}-{surface}-{name}@{backing}x");
   std::fs::write(out.join(format!("{evidence_key}.actual.rgba8")),&rgba).unwrap();
   std::fs::write(out.join(format!("{evidence_key}.view.rgba8")),&oracle).unwrap();
   std::fs::write(out.join(format!("{evidence_key}.empty.rgba8")),&empty).unwrap();
   std::fs::write(out.join("latest-case.json"),serde_json::to_vec_pretty(&serde_json::json!({"case":evidence_key,"size":[size.0,size.1],"ink":ink,"outside":outside})).unwrap()).unwrap();
   assert_eq!(ancestor_outside,0,"strict ancestor clip ink {label}/{surface}/{name}@{backing}");
   if positive{assert!(ink>0,"zero positive ink {label}/{surface}/{name}@{backing}");}else{assert_eq!(ink,0);}assert_eq!(outside,0,"out of clip ink {label}/{surface}/{name}@{backing}");
   // Retained original painter geometry/fill: independent historical oracle,
   // intentionally no new boundary. Never attribute this to a new API run.
   let mut old=Scene::new();if let Some(visible)=visible{old.push_clip(visible);old.push_quad(Quad::new(f,palette.color("surface-2").unwrap()).with_corner_radius(4.));old.pop_clip();}
   let legacy=gpu.render_to_rgba8(&old,&mut text.atlas,size,backing,ground).unwrap();let old_ink=legacy.chunks_exact(4).zip(empty.chunks_exact(4)).filter(|(a,b)|a!=b).count();if positive&&old_ink==0{legacy_invisible+=1;}
   if name=="normal" { additional_frames+=2;
    // Read actual fully-covered fill, boundary and parent pixels against
    // separately rendered solid swatches, independent of Skeleton routing.
    let mut swatches=Scene::new();swatches.push_quad(Quad::new(Rect::new(0.,0.,16.,16.),packet.background));swatches.push_quad(Quad::new(Rect::new(20.,0.,16.,16.),packet.boundary.unwrap()));
    let sw=gpu.render_to_rgba8(&swatches,&mut text.atlas,size,backing,ground).unwrap();
    let sample=|buf:&[u8],x:f32,y:f32|{let off=((((y*backing)as u32)*size.0+(x*backing)as u32)*4)as usize;buf[off..off+4].to_vec()};
    assert_eq!(sample(&rgba,42.,22.),sample(&sw,8.,8.),"fill pixel");
    assert_eq!(sample(&rgba,42.,14.),sample(&sw,28.,8.),"boundary pixel");
    assert_eq!(sample(&rgba,0.,40.),sample(&empty,0.,40.),"parent pixel");
    let mut unclipped=Scene::new();let mut bare=SkeletonView::default();bare.set_surface(surface).unwrap();bare.render(&mut unclipped,f,None,palette);let bare_rgba=gpu.render_to_rgba8(&unclipped,&mut text.atlas,size,backing,ground).unwrap();assert_eq!(bare_rgba,rgba,"public bare View None clip");
   }
   let key=format!("{label}-{surface}-{name}@{backing}x");std::fs::write(out.join(format!("{key}.rgba8")),&rgba).unwrap();
   cases.push(serde_json::json!({"case":key,"size":[size.0,size.1],"positive_expected":positive,"ink_pixels":ink,"shape_footprint_outside_pixels":outside,"strict_ancestor_outside_pixels":ancestor_outside,"original_view_RGBA_equal":true,"historical_fill_only_ink_pixels":old_ink,"fill":format!("{:?}",packet.background),"boundary":format!("{:?}",packet.boundary)}));
  }
 }}}
 assert!(legacy_invisible>0);std::fs::write(out.join("receipt.json"),serde_json::to_vec_pretty(&serde_json::json!({"actual_GPU_frames":cases.len()*4+additional_frames,"cases":cases,"legacy_fill_only_invisible_cases":legacy_invisible,"physical_IME":false,"VoiceOver":false})).unwrap()).unwrap();
}
