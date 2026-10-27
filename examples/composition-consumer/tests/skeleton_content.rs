#[path = "support/coretext.rs"] mod coretext;
use tekes_ui_composition_consumer::{skeleton_content::{self as fixture,BAR,ROW,RESULT},scroll_updates::{COMPOSER,SCROLLER}};
use tekes_ui_controls::{ControlKey,composition::*,feedback_view::SkeletonView,paint::skeleton_on_surface};
use tekes_ui_scene::{Point,Primitive,Quad,Rect,Scene,Size};
use tekes_ui_tokens::{Palette,Theme,surface_palette::{resolve_surface,surface_names}};
fn quads(s:&Scene)->Vec<Quad>{s.primitives().iter().filter_map(|p|if let Primitive::Quad(q)=p{Some(*q)}else{None}).collect()}
fn paint(h:&mut ContentHost,t:&mut coretext::Text,p:Palette)->Scene{t.services=true;t.atlas.begin_frame();let mut s=Scene::new();let f=Rect::new(0.,0.,360.,400.);h.render(&mut s,t,f,f,p).unwrap();s}

#[test] fn finite_parent_input_is_rejected_atomically_without_default_extent(){
 let mut accepted=Size::new(91.25,13.5);let before=accepted;
 for size in [Size::new(f32::NAN,1.),Size::new(1.,f32::INFINITY),Size::new(-1.,2.)]{assert!(!fixture::set_allocation(&mut accepted,size));assert_eq!(accepted,before);}
 let mut c=SkeletonContent::default();let mut t=coretext::Text::new(1.);assert_eq!(c.measure(&mut t,accepted).size,accepted);assert!(fixture::set_allocation(&mut accepted,Size::default()));assert_eq!(c.measure(&mut t,accepted).size,Size::default());
}
#[test] fn surfaces_modes_brands_frames_and_parent_clip_match_original_view(){
 for theme in Theme::ALL { for brand in [0,1,2] { for surface in surface_names(){for (w,h)in [(120.,12.),(0.,0.),(0.25,0.5),(8.,12.)]{
  let p=Palette::new(theme,brand).unwrap();let f=Rect::new(12.25,20.5,w,h);let clip=Rect::new(13.,0.,300.,100.);
  let mut host=ContentHost::new(Content::new(BAR,SkeletonContent::default()).with_surface(surface).unwrap()).unwrap();let mut t=coretext::Text::new(1.);let mut actual=Scene::new();host.render(&mut actual,&mut t,f,clip,p).unwrap();
  let mut expected=Scene::new();let mut v=SkeletonView::default();v.set_surface(surface).unwrap();v.render(&mut expected,f,Some(clip),p);assert_eq!(quads(&actual),quads(&expected));
  if w>1. && h>0. {let packet=resolve_surface(p,surface,"skeleton","rest").unwrap();let q=quads(&actual);assert_eq!(q[0].rect,f);assert_eq!(q[0].corner_radii,tekes_ui_scene::CornerRadii::uniform(4.));assert_eq!(q[0].fill,packet.background);assert_eq!(q[1].rect,f);assert_eq!(q[1].border_color,packet.boundary.unwrap());assert!(q[1].border_width>0.);assert!(!t.requested);}
 }}}}
}
#[test] fn inherited_explicit_clear_unknown_context_preserve_mount(){
 let(mut s,_)=fixture::build().unwrap();let mut t=coretext::Text::new(1.);paint(&mut s.host,&mut t,Theme::LIGHT.into());let mount=s.host.semantic_mount(BAR);
 s.host.set_inherited_surface("sidebar").unwrap();s.host.get_mut(ROW).unwrap().set_surface(Some("panel")).unwrap();s.host.get_mut(BAR).unwrap().set_surface(Some("code")).unwrap();
 for expected in ["code","panel","sidebar"]{let p=Palette::new(Theme::DARK,2).unwrap();let scene=paint(&mut s.host,&mut t,p);let f=s.host.get(BAR).unwrap().frame();let packet=resolve_surface(p,expected,"skeleton","rest").unwrap();assert!(quads(&scene).iter().any(|q|q.rect==f&&q.fill==packet.background));assert_eq!(s.host.semantic_mount(BAR),mount);if expected=="code"{s.host.get_mut(BAR).unwrap().set_surface(None).unwrap();}else{s.host.get_mut(ROW).unwrap().set_surface(None).unwrap();}}
 assert!(s.host.set_inherited_surface("unknown").is_err());assert_eq!(s.host.inherited_surface(),"sidebar");assert!(s.host.get_mut(BAR).unwrap().set_surface(Some("unknown")).is_err());assert_eq!(s.host.get(BAR).unwrap().surface(),None);
 let mut v=SkeletonView::default();v.set_surface("panel").unwrap();assert!(v.set_surface("unknown").is_err());assert_eq!(v.surface(),"panel");let mut scene=Scene::new();assert!(skeleton_on_surface(&mut scene,Rect::new(0.,0.,50.,12.),Theme::LIGHT.into(),"unknown").is_err());assert!(scene.is_empty());
}
#[test] fn decoration_optional_named_group_is_atomic_and_passive(){
 let mut h=ContentHost::new(Content::new(BAR,SkeletonContent::default())).unwrap();let mut t=coretext::Text::new(1.);paint(&mut h,&mut t,Theme::LIGHT.into());assert!(h.semantics().iter().all(|n|n.id!=BAR));let mount=h.semantic_mount(BAR);
 let c=h.get_mut(BAR).unwrap().component_mut::<SkeletonContent>().unwrap();c.set_accessibility(Some("Preview".into()),Some("Pending".into())).unwrap();assert!(c.set_accessibility(Some("  ".into()),Some("Wrong".into())).is_err());assert!(!c.focusable());assert!(!c.interactive());assert!(!c.requires_text_services());assert!(c.text_surface().is_none());
 for e in [ContentEvent::PointerDown(Point::new(1.,1.)),ContentEvent::PointerUp(Point::new(1.,1.)),ContentEvent::Key{key:ControlKey::Enter,down:true},ContentEvent::Scroll{position:Point::new(1.,1.),delta:Point::new(0.,2.),precise:true,begin:true,end:true,momentum:false,time:1.}]{assert_eq!(c.event(&e),ContentResponse::default());}
 paint(&mut h,&mut t,Theme::DARK.into());let n=h.semantics().into_iter().find(|n|n.id==BAR).unwrap();assert_eq!(n.semantics.role,ContentRole::Group);assert_eq!(n.semantics.name,"Preview");assert_eq!(n.semantics.description.as_deref(),Some("Pending"));assert!(n.semantics.value.is_none());assert_eq!(h.value_range(BAR),None);assert!(!h.focus(Some(BAR),true));
 for a in [AccessibilityAction::Press,AccessibilityAction::Focus,AccessibilityAction::SetValue("1".into()),AccessibilityAction::Increment,AccessibilityAction::Decrement]{assert_eq!(h.accessibility_action(BAR,None,a),ContentResponse::default());}
 use tekes_ui_platform_macos::{AccessibilityRole,content_accessibility::{content_key,content_nodes}};
 let key=content_key(&h,BAR,None).unwrap();let n=content_nodes(&h).into_iter().find(|n|n.key==key).unwrap();assert_eq!(n.role,AccessibilityRole::Group);assert!(!n.focused);
 h.get_mut(BAR).unwrap().component_mut::<SkeletonContent>().unwrap().set_accessibility(None,None).unwrap();paint(&mut h,&mut t,Theme::LIGHT.into());assert!(h.semantics().iter().all(|n|n.id!=BAR));assert_eq!(h.semantic_mount(BAR),mount);assert!(h.drain_action_records().is_empty());
}
#[test]
fn all_host_states_preserve_composer_marked_binding_focus_and_original_reading_anchor() {
    let (mut s, mut version) = fixture::build().unwrap();
    let mut t = coretext::Text::new(1.);
    t.services = true;
    t.reduced_motion = true;
    paint(&mut s.host, &mut t, Theme::LIGHT.into());
    s.host
        .get_mut(SCROLLER)
        .unwrap()
        .component_mut::<ScrollContent>()
        .unwrap()
        .follow_latest();
    paint(&mut s.host, &mut t, Theme::LIGHT.into());
    let pos = s.host.get(SCROLLER).unwrap().frame().center();
    assert!(
        s.host
            .event(&ContentEvent::Scroll {
                position: pos,
                delta: Point::new(0., -80.),
                precise: true,
                begin: true,
                end: true,
                momentum: false,
                time: 1.
            })
            .consumed
    );
    paint(&mut s.host, &mut t, Theme::LIGHT.into());
    let keys = s.item_keys();
    let anchor = *keys
        .iter()
        .find(|id| s.host.get(**id).unwrap().visible().is_some())
        .unwrap();
    let y = s.host.get(anchor).unwrap().frame().min_y();
    let anchor_mount = s.host.semantic_mount(anchor);
    assert!(s.host.focus(Some(COMPOSER), true));
    s.host.event(&ContentEvent::Preedit {
        text: "ni".into(),
        selection: 0..2,
        clauses: std::iter::once(0..2).collect(),
        replacement: None,
    });
    paint(&mut s.host, &mut t, Theme::LIGHT.into());
    let before = s.host.text_input_snapshot().unwrap();
    assert!(before.marked.is_some());
    let editor_mount = s.host.semantic_mount(COMPOSER);
    let bar_mount = s.host.semantic_mount(BAR);
    let binding = s.host.focused_text_binding();
    s.host.drain_action_records();
    // rest/loading/error/success/indeterminate remain distinct caller states.
    for (i, (_value, _unknown, result)) in [
        (0., false, "Rest"),
        (0.45, false, "Loading"),
        (0.45, false, "Error"),
        (1., false, "Success"),
        (0.45, true, "Loading, unknown"),
        (1., false, "Awaiting verification"),
    ]
    .into_iter()
    .enumerate()
    {
        let (v, ()) = s
            .host
            .update_scroll_reading_document(
                SCROLLER,
                &version,
                version.revision() + 1,
                keys.clone(),
                |h| {
                    fixture::publish(h, result);
                    h.get_mut(BAR).unwrap().component_mut::<SkeletonContent>().unwrap().set_accessibility(if i % 2 == 0 { Some("Pending preview".into()) } else { None },Some(result.into())).unwrap();
                    h.set_inherited_surface(if i % 2 == 0 { "sidebar" } else { "canvas" })
                        .unwrap();
                },
            )
            .unwrap();
        version = v;
        let actual_scene = paint(
            &mut s.host,
            &mut t,
            Palette::new(
                if i % 2 == 0 {
                    Theme::DARK
                } else {
                    Theme::LIGHT
                },
                2,
            )
            .unwrap(),
        );
        let after = s.host.text_input_snapshot().unwrap();
        assert_eq!(after.binding, before.binding);
        assert_eq!(after.marked, before.marked);
        assert_eq!(after.selection, before.selection);
        assert_eq!(after.text, before.text);
        assert_eq!(after.caret, before.caret);
        assert_eq!(after.carets, before.carets);
        assert_eq!(s.host.focused_text_binding(), binding);
        assert_eq!(s.host.focused(), Some(COMPOSER));
        assert_eq!(s.host.semantic_mount(COMPOSER), editor_mount);
        assert_eq!(s.host.semantic_mount(BAR), bar_mount);
        assert_eq!(s.host.semantic_mount(anchor), anchor_mount);
        let bar_center = s.host.get(BAR).unwrap().frame().center();
        for event in [
            ContentEvent::PointerDown(bar_center),
            ContentEvent::PointerUp(bar_center),
            ContentEvent::Scroll {
                position: bar_center,
                delta: Point::new(0., 2.),
                precise: true,
                begin: true,
                end: true,
                momentum: false,
                time: 10. + i as f64,
            },
        ] {
            assert_eq!(s.host.event(&event), ContentResponse::default());
        }
        assert_eq!(s.host.focused(), Some(COMPOSER));
        assert_eq!(s.host.focused_text_binding(), binding);
        let reading = s
            .host
            .get(SCROLLER)
            .unwrap()
            .component::<ScrollContent>()
            .unwrap()
            .reading_result()
            .unwrap();
        assert_eq!(reading.version, version);
        assert_eq!(
            reading.anchor,
            ContentReadingAnchorUpdate::Preserved(anchor)
        );
        assert!((s.host.get(anchor).unwrap().frame().min_y() - y).abs() < 1.);
        assert_eq!(
            s.host
                .get(RESULT)
                .unwrap()
                .component::<TextContent>()
                .unwrap()
                .text,
            result
        );
        assert!(s.host.drain_action_records().is_empty());
        let mut expected = Scene::new();
        let mut view = SkeletonView::default();
        view.set_surface(s.host.inherited_surface()).unwrap();
        let f = s.host.get(BAR).unwrap().frame();
        view.render(
            &mut expected,
            f,
            Some(f),
            Palette::new(
                if i % 2 == 0 {
                    Theme::DARK
                } else {
                    Theme::LIGHT
                },
                2,
            )
            .unwrap(),
        );
        let q = quads(&expected);
        let actual = quads(&actual_scene);
        for expected_quad in &q {
            assert!(
                actual.contains(expected_quad),
                "host state {result} missed original Progress packet/geometry"
            );
        }
        assert_eq!(q[1].border_color, resolve_surface(Palette::new(if i % 2 == 0 { Theme::DARK } else { Theme::LIGHT }, 2).unwrap(),s.host.inherited_surface(),"skeleton","rest").unwrap().boundary.unwrap());
        // Programmatic preedit must retain the original Composer send gate.
        s.host.event(&ContentEvent::Key { key: ControlKey::Enter, down: true });
        assert!(s.host.drain_action_records().is_empty());
        assert_eq!(s.host.text_input_snapshot().unwrap().marked,before.marked);
    }
}

#[test] fn compatibility_signature_keeps_canonical_canvas_geometry(){let f=Rect::new(2.25,3.5,40.,12.);let p=Theme::DARK.into();let mut a=Scene::new();tekes_ui_controls::paint::skeleton(&mut a,f,p);let mut b=Scene::new();skeleton_on_surface(&mut b,f,p,"canvas").unwrap();assert_eq!(a,b);}
#[test] fn narrow_width_and_parent_clip_preserve_original_composer_state(){mixed_geometry(false);}
#[test] #[ignore="explicit external artifact lane"] fn external_mixed_content_theme_and_geometry(){mixed_geometry(true);}
fn mixed_geometry(external:bool){
 let(mut s,_)=fixture::build().unwrap();let mut t=coretext::Text::new(1.);t.services=true;paint(&mut s.host,&mut t,Theme::LIGHT.into());assert!(s.host.focus(Some(COMPOSER),true));s.host.event(&ContentEvent::Preedit{text:"ni".into(),selection:0..2,clauses:std::iter::once(0..2).collect(),replacement:None});paint(&mut s.host,&mut t,Theme::LIGHT.into());let before=s.host.text_input_snapshot().unwrap();let mount=s.host.semantic_mount(BAR);let editor=s.host.semantic_mount(COMPOSER);let binding=s.host.focused_text_binding();let keys=s.item_keys();let anchors:Vec<_>=keys.iter().map(|id|s.host.semantic_mount(*id)).collect();s.host.drain_action_records();
 for theme in Theme::ALL {for surface in surface_names(){for id in if external {vec!["blue","blue-inputs"]}else{vec!["default"]}{let p=if external{Palette::for_theme_definition(theme,id).unwrap()}else{Palette::new(theme,2).unwrap()};s.host.set_inherited_surface(surface).unwrap();
 for (w,h,clip) in [(360.,400.,Rect::new(0.,0.,360.,400.)),(240.,280.,Rect::new(30.,40.,180.,200.)),(240.,280.,Rect::new(500.,500.,10.,10.))]{fixture::publish(&mut s.host,"Controlled host message");let mut scene=Scene::new();t.atlas.begin_frame();s.host.render(&mut scene,&mut t,Rect::new(0.,0.,w,h),clip,p).unwrap();let after=s.host.text_input_snapshot().unwrap();assert_eq!(after.binding,before.binding);assert_eq!(after.marked,before.marked);assert_eq!(after.selection,before.selection);assert_eq!(after.text,before.text);assert_eq!(s.host.focused(),Some(COMPOSER));assert_eq!(s.host.focused_text_binding(),binding);assert_eq!(s.host.semantic_mount(BAR),mount);assert_eq!(s.host.semantic_mount(COMPOSER),editor);assert_eq!(keys.iter().map(|id|s.host.semantic_mount(*id)).collect::<Vec<_>>(),anchors);s.host.event(&ContentEvent::Key{key:ControlKey::Enter,down:true});assert!(s.host.drain_action_records().is_empty());}
 }}}
}
