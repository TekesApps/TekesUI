use tekes_ui_text::{Shaper,ShapeOptions,TextStyle,coretext::{CoreTextFonts,CoreTextShaper}};
fn main(){let mut s=CoreTextShaper::new(CoreTextFonts::new()); for text in ["Increment","增加","设置", "A😀B"] {let p=s.shape(text,&TextStyle::system(20.).with_weight(400),&ShapeOptions::default());println!("{text}: height={} proposed_chrome={}",p.height,20.*(p.height/24.).max(1.));}}
