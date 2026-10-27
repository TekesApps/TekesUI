use tekes_ui_tokens::{Rgba,Palette,Theme,surface_palette::{resolve_surface,surface_names}};

fn external_theme_requested() -> bool {
    match std::env::var_os("TEKESUI_SKELETON_EXTERNAL") {
        None => false,
        Some(value) if value.to_str() == Some("0") => false,
        Some(value) if value.to_str() == Some("1") => true,
        Some(value) => panic!("TEKESUI_SKELETON_EXTERNAL must be 0 or 1, got {value:?}"),
    }
}
fn contrast(a:Rgba,b:Rgba)->f32{let l=|c:Rgba|{let x=c.to_linear();0.2126*x[0]+0.7152*x[1]+0.0722*x[2]};let(a,b)=(l(a),l(b));(a.max(b)+0.05)/(a.min(b)+0.05)}
#[test] fn complete_rest_packet_and_both_boundary_contrasts(){
 let mut count=0;for theme in Theme::ALL {let palettes=if external_theme_requested(){["blue","blue-inputs"].into_iter().map(|id|Palette::for_theme_definition(theme,id).unwrap()).collect::<Vec<_>>()}else{[0,1,2].into_iter().map(|brand|Palette::new(theme,brand).unwrap()).collect::<Vec<_>>()};
 for p in palettes {for surface in surface_names(){let c=resolve_surface(p,surface,"skeleton","rest").unwrap();let parent=resolve_surface(p,surface,"content","rest").unwrap().background;assert_eq!(c.background,p.color("surface-2").unwrap());assert_eq!(c.text,p.color("primary-text").unwrap());assert_eq!(c.icon,c.text);assert_eq!(c.boundary,Some(p.color("border").unwrap()));assert_eq!((c.focus,c.marker,c.helper),(None,None,None));assert!(contrast(c.boundary.unwrap(),c.background)>=3.);assert!(contrast(c.boundary.unwrap(),parent)>=3.);
 for state in ["hover","pressed","selected","disabled","focus","loading","error"]{assert!(resolve_surface(p,surface,"skeleton",state).is_err());}count+=1;}}}
 assert_eq!(count,if external_theme_requested(){56}else{84});
}
