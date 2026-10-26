use tekes_ui_controls::{BarChart,ChartDatum,ButtonTextRenderer,ButtonTextAlign,ChartGeometry,ControlKey,paint_bar_chart,paint_bar_chart_with_text_scale,collections_view::{ChartView,CollectionEvent}};
use tekes_ui_scene::{Rect,Scene,Size,Point};
use tekes_ui_tokens::{Theme,Rgba,proposal};
fn fixed()->bool{true}
fn model(values:&[f64])->BarChart{BarChart::new(values.iter().enumerate().map(|(i,v)|ChartDatum::new(["","same","same","  "][i%4],*v)).collect())}
fn check(g:&ChartGeometry){
 for b in &g.bars {for r in [b.rect,b.value_label,b.category_label]{for x in [r.min_x(),r.min_y(),r.size.width,r.size.height]{assert!(x.is_finite());}assert!(r.size.width>=0.&&r.size.height>=0.);}
 if fixed(){assert!(b.value_label.max_y()+4.<=b.category_label.min_y()+0.001,"{g:?}");assert!(b.value_label.min_y()>=g.bounds.min_y()-0.001);assert!(b.category_label.max_y()<=g.bounds.max_y()+0.001);}}
}
#[test] fn raw_witness_and_paint_share_original_geometry(){
 let m=model(&[-4.,8.]);let b=Rect::new(0.,0.,240.,180.);let g=m.layout(b);
 assert_eq!(g.plot,Rect::new(8.,8.,224.,144.));assert_eq!(g.bars[0].value_label.min_y(),152.);
 assert_eq!(g.bars[0].category_label.min_y(),if fixed(){164.}else{156.});
 assert_eq!(paint_bar_chart(&mut Scene::new(),b,Theme::LIGHT,&m),g);
 for scale in [1.,2.,8.,f32::MAX,0.,-1.,f32::NAN,f32::INFINITY]{let gg=m.layout_with_text_scale(b,scale);check(&gg);assert_eq!(gg,paint_bar_chart_with_text_scale(&mut Scene::new(),b,Theme::LIGHT,&m,scale));}
}
#[test] fn measured_rows_cover_real_sides_and_rounding(){
 for values in [vec![2.,8.,4.],vec![-1.,-2.,-3.],vec![-4.,8.,0.,-0.],vec![0.,-0.],vec![f64::MAX,-f64::MAX],vec![f64::MAX,f64::MIN_POSITIVE],vec![f64::MAX,f64::MIN_POSITIVE,0.],vec![]]{
 let m=model(&values);
 for (v,c) in [(16.,16.),(26.,16.),(32.,48.),(0.,f32::NAN),(f32::INFINITY,-1.)]{
 for b in [Rect::new(13.,17.,420.,260.),Rect::new(0.,0.,150.,180.),Rect::new(0.,0.,120.,32.),Rect::new(0.,0.,16.01,100.),Rect::new(0.,0.,12.,120.),Rect::new(0.,0.,0.,120.),Rect::new(0.,0.,f32::NAN,120.)]{
 let g=m.layout_with_label_metrics(b,v,c);check(&g);if !g.bars.is_empty(){assert_eq!(g.bars.len(),values.len());for (i,bar) in g.bars.iter().enumerate(){assert_eq!(bar.index,i);assert_eq!(bar.value.to_bits(),values[i].to_bits());assert_eq!(bar.label,m.data()[i].label);}}
 }}}
}
#[test] fn zeros_and_budget_boundaries_do_not_fabricate_marks(){
 let m=model(&[0.,-0.]);for b in [Rect::new(0.,0.,240.,180.),Rect::new(0.,0.,240.,40.)]{let raw=m.layout(b);assert_eq!(raw.baseline,raw.plot.center().y);check(&raw);let g=m.layout_with_label_metrics(b,26.,16.);check(&g);if !g.bars.is_empty(){assert_eq!(g.baseline,g.plot.min_y());assert!(g.bars.iter().all(|b|b.rect.size.height==0.));}}
 if fixed(){let m=model(&[-1.,1.]);assert!(m.layout_with_label_metrics(Rect::new(0.,0.,120.,92.),26.,16.).bars.is_empty());assert!(!m.layout_with_label_metrics(Rect::new(0.,0.,120.,93.),26.,16.).bars.is_empty());}
 let invalid=model(&[f64::NAN,3.,f64::INFINITY,-2.]);let g=invalid.layout_with_label_metrics(Rect::new(0.,0.,240.,180.),26.,16.);assert_eq!(g.invalid_count,2);assert_eq!(g.bars.iter().map(|b|b.index).collect::<Vec<_>>(),vec![1,3]);check(&g);
}
#[derive(Default)] struct Text{calls:Vec<(String,f32)>,draws:Vec<Rect>,height:f32}
impl ButtonTextRenderer for Text{
 fn measure(&mut self,s:&str,_:proposal::TextRole,w:f32)->Size{self.calls.push((s.to_owned(),w));Size::new(w.min(s.len() as f32*7.),self.height.max(16.))}
 fn draw(&mut self,_:&mut Scene,_:&str,_:proposal::TextRole,r:Rect,_:ButtonTextAlign,_:Rgba){self.draws.push(r);}
}
#[test] fn actual_measurement_survives_empty_vertical_probe(){
 let mut m=model(&[-1.,2.]);let frame=Rect::new(0.,0.,240.,32.);assert!(m.layout_with_text_scale(frame,1.).bars.is_empty());
 let mut view=ChartView::new();let mut text=Text{height:64.,..Default::default()};let l=view.render(&mut Scene::new(),&mut text,&mut m,frame,None,Theme::LIGHT.into(),1.);
 assert!(l.geometry.bars.is_empty());assert_eq!(text.calls.len(),if fixed(){4}else{0});
 if fixed(){assert_eq!(text.calls.iter().map(|(_,w)|*w).collect::<Vec<_>>(),vec![80.64,112.,80.64,112.]);}
}
#[test] fn public_view_uses_current_geometry_for_hit_and_original_reducer(){
 for clip in [None,Some(Rect::new(0.,0.,60.,180.)),Some(Rect::new(500.,0.,10.,10.))]{
 let mut m=model(&[-4.,8.,0.]);let mut view=ChartView::new();let mut text=Text{height:26.,..Default::default()};let g=view.render(&mut Scene::new(),&mut text,&mut m,Rect::new(0.,0.,240.,180.),clip,Theme::LIGHT.into(),1.);check(&g.geometry);
 for bar in &g.geometry.bars {for point in [bar.rect.center(),bar.category_label.center(),Point::new(-1.,-1.)]{let expected=g.geometry.bars.iter().find(|b|{let r=b.rect.inset(-6.);g.visible.and_then(|v|r.intersection(&v)).is_some_and(|v|v.contains(point))}).map(|b|b.index);let mut mm=m.clone();let mut vv=view.clone();let _=vv.pointer_move(point,&mut mm);let event=vv.pointer_down(point,&mut mm);assert_eq!(mm.hovered(),expected);assert_eq!(mm.selected(),expected.or(m.selected()));assert_eq!(matches!(event,CollectionEvent::Ignored),expected.is_none());}}
 view.focus(true,&mut m);assert_eq!(view.key(ControlKey::End,true,&mut m),CollectionEvent::Changed);assert_eq!(m.selected(),Some(2));assert_eq!(view.key(ControlKey::Home,true,&mut m),CollectionEvent::Changed);assert_eq!(m.selected(),Some(0));assert_eq!(view.key(ControlKey::Right,false,&mut m),CollectionEvent::Ignored);
 }
}
