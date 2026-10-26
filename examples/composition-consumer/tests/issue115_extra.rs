use tekes_ui_controls::{BarChart,ChartDatum};
use tekes_ui_scene::Rect;
#[test]
fn raw_scale_sign_extremes_labels_and_zero_boundaries(){
 for values in [vec![1.,2.,3.],vec![-1.,-2.,-3.],vec![-1.,2.,0.,-0.],vec![0.,-0.],vec![f64::MAX,-f64::MAX],vec![f64::MAX,f64::MIN_POSITIVE],vec![]]{
 let m=BarChart::new(values.iter().enumerate().map(|(i,v)|ChartDatum::new(["","  ","same","same"][i%4],*v)).collect());
 for scale in [1.,2.,8.,f32::MAX,0.,-1.,f32::NAN,f32::INFINITY]{for h in [12.,32.,40.,80.,180.,240.]{for w in [12.,16.01,120.,320.]{let b=Rect::new(13.,17.,w,h);let g=m.layout_with_text_scale(b,scale);for bar in &g.bars{assert_eq!(bar.value.to_bits(),values[bar.index].to_bits());assert_eq!(bar.label,m.data()[bar.index].label);assert!(bar.value_label.max_y()+4.<=bar.category_label.min_y()+0.001);for r in [bar.rect,bar.value_label,bar.category_label]{for x in [r.min_x(),r.min_y(),r.size.width,r.size.height]{assert!(x.is_finite());}assert!(r.min_x()>=b.min_x()-0.001);assert!(r.max_x()<=b.max_x()+0.001);assert!(r.min_y()>=b.min_y()-0.001);assert!(r.max_y()<=b.max_y()+0.001);}if bar.value==0.{assert_eq!(bar.rect.size.height,0.);}}}}}
 }
}
