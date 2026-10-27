#![cfg(target_os = "macos")]
#[path = "support/coretext.rs"]
mod coretext;
use coretext::Text;
use tekes_ui_controls::{Button, composition::*};
use tekes_ui_scene::{ImageId, Primitive, Rect, Scene};
use tekes_ui_tokens::Theme;
const VIEW: Rect = Rect::new(0., 0., 1200., 300.);
fn regions(left: f32, right: f32, include_panel: bool) -> Vec<ToolbarRegion> {
    let band = Rect::new(0., 0., VIEW.size.width, 60.);
    let mut result = vec![];
    if include_panel {
        result.push(ToolbarRegion::new(ComponentId(10), Rect::new(0., 0., left, 60.)));
        result.push(ToolbarRegion::new(ComponentId(11), Rect::new(left, 0., VIEW.size.width-left-right, 60.)));
        result.push(ToolbarRegion::new(ComponentId(12), Rect::new(VIEW.size.width-right, 0., right, 60.)));
    }
    for (id, edge) in [(20,ToolbarWindowEdge::Leading),(21,ToolbarWindowEdge::Trailing)] {
        let mut region = ToolbarRegion::window(ComponentId(id), band, edge);
        region.native_safe_area.left = 80.;
        result.push(region);
    }
    result
}
fn build() -> (ToolbarHost<&'static str>, Text) {
    let mut host = ToolbarHost::new(ComponentId(1), 7, VIEW).unwrap();
    host.set_regions(7, VIEW, regions(300.,300.,true)).unwrap();
    for (id,region,placement,children) in [
        (100,20,ToolbarPlacement::Navigation,vec![Content::new(ComponentId(101),Button::new(ImageId(1)).with_accessible_name("Toggle")), Content::new(ComponentId(102),TextFieldContent::inline("Search","",12.))]),
        (200,21,ToolbarPlacement::PrimaryAction,vec![Content::new(ComponentId(201),Button::new(ImageId(2)).with_accessible_name("Trailing"))]),
        (300,10,ToolbarPlacement::PrimaryAction,vec![Content::new(ComponentId(301),Button::new(ImageId(3)).with_accessible_name("Panel action"))]),
    ] {
        host.register(ToolbarRegistration { region: ComponentId(region), order: 0,
            group: Content::new(ComponentId(id),ToolbarGroup::new(placement,children)), bindings: vec![] }).unwrap();
    }
    let mut text=Text::new(1.);
    text.services=true;
    (host,text)
}
fn draw(h:&mut ToolbarHost<&'static str>,t:&mut Text)->Scene {
    let mut scene=Scene::new();
    h.render(&mut scene,t,Theme::LIGHT.into()).unwrap();
    scene
}
#[test]
fn window_controls_and_editor_stay_fixed_through_both_container_transitions_and_reversal() {
    for scale in [1.,1.5,2.] {
        let (mut h,mut t)=build(); t.scale=scale;
        draw(&mut h,&mut t);
        let ids=[ComponentId(101),ComponentId(102),ComponentId(201)];
        let frames=ids.map(|id| h.content_host().get(id).unwrap().frame());
        let mounts=ids.map(|id| h.content_host().semantic_mount(id));
        h.event(&ContentEvent::PointerDown(frames[1].center()));
        h.event(&ContentEvent::PointerUp(frames[1].center()));
        h.event_with_text(&ContentEvent::Commit{text:"保留搜索".into(),replacement:None},&mut t).unwrap();
        let mut samples=vec![];
        for step in 0..=60 {
            let width=300.-step as f32*5.;
            samples.push((width,300.));
            samples.push((300.,width));
            samples.push((300.-width,300.-width));
        }
        for (left,right) in samples.into_iter().chain([(0.,300.),(90.,300.),(25.,40.),(300.,300.),(0.,0.)]) {
            h.set_regions(7,VIEW,regions(left,right,true)).unwrap();
            h.update_component::<Button>(ComponentId(100),ComponentId(101),|button|button.button.state.disabled=left==0.).unwrap();
            let scene=draw(&mut h,&mut t);
            for (i,id) in ids.into_iter().enumerate() {
                assert_eq!(h.content_host().get(id).unwrap().frame(),frames[i]);
                assert_eq!(h.content_host().semantic_mount(id),mounts[i]);
            }
            assert_eq!(h.content_host().focused(),Some(ComponentId(102)));
            assert_eq!(h.content_host().get(ComponentId(102)).unwrap().component::<TextFieldContent>().unwrap().surface.input.text,"保留搜索");
            for image in [ImageId(1),ImageId(2)] {
                let images:Vec<_>=scene.primitives().iter().filter_map(|p|match p {Primitive::Image(q) if q.image==image=>Some(q),_=>None}).collect();
                assert_eq!(images.len(),1,"one original button per frame");
                assert!(images[0].rect.min_x()>=80.,"never sweeps behind native window controls");
            }
        }
        h.set_regions(7,VIEW,regions(0.,0.,false)).unwrap();
        draw(&mut h,&mut t);
        assert!(h.content_host().get(ComponentId(301)).is_none());
        for (i,id) in ids.into_iter().enumerate() {
            assert_eq!(h.content_host().semantic_mount(id),mounts[i]);
            assert_eq!(h.content_host().get(id).unwrap().frame(),frames[i]);
        }
    }
}
#[test]
fn fixed_window_edges_share_narrow_width_without_overlapping_each_other_or_containers() {
    for width in [0.,40.,80.,100.,160.,320.] {
        let (mut h,mut t)=build();
        let view=Rect::new(0.,0.,width,300.);
        let mut rs=regions(0.,0.,false);
        for r in &mut rs {r.available.size.width=width;}
        rs.push(ToolbarRegion::new(ComponentId(11),Rect::new(0.,0.,width,60.)));
        h.set_regions(7,view,rs).unwrap();
        draw(&mut h,&mut t);
        let nodes=h.content_host().semantics();
        let visible:Vec<_>=nodes.iter().filter(|n|matches!(n.id,ComponentId(101)|ComponentId(102)|ComponentId(201))).filter_map(|n|n.visible).collect();
        for (i,a) in visible.iter().enumerate() { for b in &visible[i+1..] {
            assert!(a.intersection(b).is_none_or(|r|r.is_empty()));
        }}
    }
}

#[test]
fn composed_fixed_window_toolbar_preserves_real_split_marker_after_overlay_composition() {
    use tekes_ui_controls::{extended::{Axis,SplitView},layout_view::SplitViewView};
    use tekes_ui_scene::Point;
    for scale in [1.,2.] {
        let (mut host,mut text)=build(); text.scale=scale;
        let mut model=SplitView::new(0.5);
        let mut split=SplitViewView::new(Axis::Horizontal);
        let layout=split.arrange(&mut model,VIEW,None);
        let band=Rect::new(0.,0.,VIEW.size.width,84.*scale);
        let mut regions=regions(0.,0.,false);
        for region in &mut regions {region.available=band;}
        for (id,frame) in [(10,layout.leading.frame()),(11,layout.trailing.frame())] {
            regions.push(ToolbarRegion::new(ComponentId(id),Rect::new(frame.min_x(),0.,frame.size.width,band.size.height)));
        }
        host.set_regions(7,VIEW,regions).unwrap();
        let toolbar=draw(&mut host,&mut text);
        split.set_toolbar_region(Some(band),&host.content_host().pointer_regions(),&mut text);
        let mut composed=Scene::new();
        split.paint(&mut composed,&model,Theme::LIGHT.into(),2.);
        let marker=composed.primitives().iter().find_map(|p|match p {
            Primitive::Quad(q) if q.corner_radii.top_left>0.=>Some(*q),_=>None,
        }).expect("real SplitView toolbar marker");
        let row = tekes_ui_controls::ToolbarControl::window().row_height(&mut text);
        assert!((marker.rect.size.height / row - 2. / 3.).abs() < 0.0001);
        for primitive in toolbar.primitives(){composed.push(primitive.clone());}
        // Inspect final paint order, including clips, rather than only checking
        // that a marker primitive existed before the overlay erased it.
        let sample:Point=marker.rect.center();
        let mut clips=vec![VIEW];
        let mut visible=None;
        for primitive in composed.primitives() {
            match primitive {
                Primitive::PushClip(rect)=>clips.push(clips.last().unwrap().intersection(rect).unwrap_or_default()),
                Primitive::PopClip=>{clips.pop();},
                Primitive::Quad(q) if q.rect.contains(sample)&&clips.last().is_some_and(|r|r.contains(sample))=>visible=Some(q.fill),
                _=>{},
            }
        }
        assert_eq!(visible,Some(marker.fill),"toolbar ground must not cover splitter marker at scale {scale}");
    }
}
