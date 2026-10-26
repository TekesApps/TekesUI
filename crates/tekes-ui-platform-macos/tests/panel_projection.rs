#![cfg(all(target_os = "macos", feature = "content-accessibility"))]
//! Public original Host + real original text editor; no native window or GPU.
use std::rc::Rc;
use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, Insets, TextSurfaceHost, TextSurfaceTextRenderer,
    composition::*,
};
use tekes_ui_scene::{Point, Rect, Scene, Size};
use tekes_ui_text::{
    Paragraph, ShapeOptions, Shaper, TextStyle,
    coretext::{CoreTextFonts, CoreTextShaper},
};
use tekes_ui_tokens::{Rgba, Theme, proposal};
struct Text {
    enabled: bool,
    now: std::time::Duration,
    reduced: bool,
    clocked: bool,
    frames: usize,
    shaper: CoreTextShaper,
    widths: Vec<Option<f32>>,
}
impl Text {
    fn new(enabled: bool) -> Self {
        Self {
            enabled,
            now: std::time::Duration::ZERO,
            reduced: false,
            clocked: true,
            frames: 0,
            shaper: CoreTextShaper::new(CoreTextFonts::new()),
            widths: Vec::new(),
        }
    }
}
impl ButtonTextRenderer for Text {
    fn text_surface_host(&mut self) -> Option<&mut dyn TextSurfaceHost> {
        if self.enabled { Some(self) } else { None }
    }
    fn motion_context(&self) -> Option<tekes_ui_controls::ControlMotionContext> {
        self.clocked
            .then_some(tekes_ui_controls::ControlMotionContext {
                now: self.now,
                reduced_motion: self.reduced,
            })
    }
    fn request_animation_frame(&mut self) {
        self.frames += 1;
    }
    fn measure(&mut self, s: &str, r: proposal::TextRole, w: f32) -> Size {
        assert!(w > 0., "inactive pane must not measure zero-width text");
        let paragraph = self.shape_surface(s, r, Some(w));
        Size::new(paragraph.width as f32, paragraph.height as f32)
    }
    fn draw(
        &mut self,
        _: &mut Scene,
        _: &str,
        _: proposal::TextRole,
        _: Rect,
        _: ButtonTextAlign,
        _: Rgba,
    ) {
    }
}
impl TextSurfaceHost for Text {
    fn shape_surface(&mut self, s: &str, r: proposal::TextRole, w: Option<f32>) -> Rc<Paragraph> {
        assert!(w.is_none_or(|w| w > 0.), "zero-width editor shaping");
        self.widths.push(w);
        Rc::new(self.shaper.shape(
            s,
            &TextStyle::system(r.style().size).with_weight(r.style().weight),
            &ShapeOptions {
                width: w.map(f64::from),
                line_spacing: 0.,
            },
        ))
    }
}
impl TextSurfaceTextRenderer for Text {
    fn push_visible(&mut self, _: &mut Scene, _: &Paragraph, _: Point, _: Rgba, _: Rect) {}
}

const ROOT: ComponentId = ComponentId(8100);
const CHILD: ComponentId = ComponentId(8101);
const FIELD: ComponentId = ComponentId(8102);
fn render(h: &mut ContentHost, t: &mut Text, clip: Rect) -> Scene {
    let mut s = Scene::new();
    h.render(
        &mut s,
        t,
        Rect::new(0., 0., 100., 100.),
        clip,
        Theme::LIGHT.into(),
    )
    .unwrap();
    s
}
fn panel(child: Content, zero: bool) -> PanelContent {
    let mut p = PanelContent::new(child).unwrap();
    if zero {
        p.set_padding(Some(Insets::default())).unwrap();
    }
    p
}

#[test]
fn actual_native_projection_uses_exact_points_same_mount_generation_and_stales() {
    use tekes_ui_platform_macos::content_accessibility::ContentAccessibilitySnapshot;
    use tekes_ui_platform_macos::event::Point as P;
    let mut h = ContentHost::new(Content::new(
        ROOT,
        panel(
            Content::new(CHILD, Button::new("button")).preserving_focus(),
            true,
        ),
    ))
    .unwrap();
    let mut text = Text::new(true);
    render(&mut h, &mut text, Rect::new(0., 0., 100., 100.));
    assert!(tekes_ui_platform_macos::content_accessibility::content_nodes(&h).is_empty());
    let s = ContentAccessibilitySnapshot::from_host(&h).unwrap();
    assert!(s.is_valid());
    assert!(s.hit_test(P { x: 0.5, y: 0.5 }).is_none());
    let key = s.hit_test(P { x: 50., y: 50. }).unwrap();
    assert_eq!(key.component, h.semantic_mount(CHILD).unwrap());
    let generation = s.generation();
    let _ = h.get_mut(CHILD);
    assert!(!s.is_current());
    assert!(s.hit_test(P { x: 50., y: 50. }).is_none());
    assert!(ContentAccessibilitySnapshot::from_host(&h).is_none());
    render(&mut h, &mut text, Rect::new(0., 0., 100., 100.));
    let next = ContentAccessibilitySnapshot::from_host(&h).unwrap();
    assert!(next.generation() > generation);
    assert_eq!(next.hit_test(P { x: 50., y: 50. }), Some(key));
}
#[test]
fn actual_fully_clipped_editor_keeps_native_key_value_and_selection_interfaces() {
    use tekes_ui_platform_macos::content_accessibility::ContentAccessibilitySnapshot;
    let mut h = ContentHost::new(Content::new(
        ROOT,
        panel(
            Content::new(FIELD, TextFieldContent::new("Composer", "kept draft")),
            false,
        ),
    ))
    .unwrap();
    let mut text = Text::new(true);
    render(&mut h, &mut text, Rect::new(0., 0., 100., 100.));
    assert!(h.focus(Some(FIELD), true));
    let mount = h.semantic_mount(FIELD).unwrap();
    render(&mut h, &mut text, Rect::new(300., 300., 100., 100.));
    assert!(tekes_ui_platform_macos::content_accessibility::content_nodes(&h).is_empty());
    let s = ContentAccessibilitySnapshot::from_host(&h).unwrap();
    assert!(s.is_valid());
    let n = s.nodes().iter().find(|n| n.key.component == mount).unwrap();
    assert_eq!(n.frame.size.width, 0.);
    assert_eq!(n.frame.size.height, 0.);
    assert!(n.focused);
    assert!(n.text_selection.is_some());
    assert!(s.text_layouts().contains_key(&n.key));
}


#[test]
fn window_batch_keeps_three_original_owners_text_geometry_and_direct_dispatch() {
    use tekes_ui_platform_macos::{AccessibilityAction as A,content_accessibility::*};
    use tekes_ui_platform_macos::Point as P;
    let mut toolbar=ContentHost::new(Content::new(ComponentId(91),Button::new("Toolbar"))).unwrap();
    let mut plain=ContentHost::new(Content::new(ComponentId(92),Button::new("Other"))).unwrap();
    let mut curve=ContentHost::new(Content::new(ROOT,panel(Content::new(FIELD,TextFieldContent::new("Composer","draft")),false))).unwrap();
    let mut text=Text::new(true);
    for (h,x,y) in [(&mut plain,120.,40.),(&mut curve,0.,40.),(&mut toolbar,0.,0.)] {
        h.render(&mut Scene::new(),&mut text,Rect::new(x,y,100.,if y==0.{30.}else{100.}),Rect::new(0.,0.,300.,200.),Theme::LIGHT.into()).unwrap();
    }
    let key=tekes_ui_platform_macos::content_accessibility::content_key(&curve,FIELD,None).unwrap();
    let toolbar_key=tekes_ui_platform_macos::content_accessibility::content_key(&toolbar,ComponentId(91),None).unwrap();
    let publication=WindowAccessibilityPublication::prepare(vec![
        (ContentAccessibilityOwner(1),ContentAccessibilitySnapshot::from_host(&plain).unwrap()),
        (ContentAccessibilityOwner(2),ContentAccessibilitySnapshot::from_host(&curve).unwrap()),
        (ContentAccessibilityOwner(0),ContentAccessibilitySnapshot::from_host(&toolbar).unwrap()),
    ]).unwrap();
    let mut slot=None; assert!(WindowAccessibilityPublication::commit(&mut slot,Some(publication)));
    let publication=slot.as_ref().unwrap().clone();
    assert_eq!(publication.owner(key),Some(ContentAccessibilityOwner(2)));
    assert_eq!(publication.owner(toolbar_key),Some(ContentAccessibilityOwner(0)));
    assert!(publication.text_layouts().unwrap().contains_key(&key));
    assert!(publication.frame_for_range(key,0..1).is_some());
    assert!(publication.hit_test(P{x:0.5,y:40.5}).is_none());
    let frame=publication.nodes().iter().find(|n|n.key==key).unwrap().frame;
    assert_eq!(publication.hit_test(P{x:frame.origin.x+frame.size.width/2.,y:frame.origin.y+frame.size.height/2.}),Some(key));
    assert!(!publication.dispatch(ContentAccessibilityOwner(2),&mut plain,key,A::Focus).consumed);
    assert!(!publication.dispatch(ContentAccessibilityOwner(1),&mut curve,key,A::Focus).consumed);
    assert!(publication.dispatch(ContentAccessibilityOwner(2),&mut curve,key,A::Focus).consumed);
    // Mutating any one original Host closes the entire batch, including toolbar.
    assert!(!publication.is_current());
    assert!(publication.owner(toolbar_key).is_none());
    assert!(publication.text_layouts().is_none());
    assert!(publication.frame_for_range(key,0..1).is_none());
    assert!(!publication.dispatch(ContentAccessibilityOwner(0),&mut toolbar,toolbar_key,A::Press).consumed);
    assert!(!WindowAccessibilityPublication::commit(&mut slot,Some(publication.clone())));
    assert!(!slot.as_ref().unwrap().is_current());
    render(&mut curve,&mut text,Rect::new(0.,0.,100.,100.));
    let next=WindowAccessibilityPublication::prepare(vec![(ContentAccessibilityOwner(2),ContentAccessibilitySnapshot::from_host(&curve).unwrap())]).unwrap();
    assert!(WindowAccessibilityPublication::commit(&mut slot,Some(next)));
    assert_eq!(slot.as_ref().unwrap().owner(key),Some(ContentAccessibilityOwner(2)));
    assert!(!publication.is_current());
}

#[test]
fn duplicate_or_missing_native_parent_rejects_whole_batch_and_revokes_saved_authority() {
    use tekes_ui_platform_macos::content_accessibility::*;
    let mut host=ContentHost::new(Content::new(ROOT,panel(Content::new(CHILD,Button::new("button")),true))).unwrap();
    render(&mut host,&mut Text::new(true),Rect::new(0.,0.,100.,100.));
    let snapshot=ContentAccessibilitySnapshot::from_host(&host).unwrap();
    let publication=WindowAccessibilityPublication::prepare(vec![(ContentAccessibilityOwner(1),snapshot.clone())]).unwrap();
    let saved=publication.clone();let mut slot=Some(publication);
    let duplicate=WindowAccessibilityPublication::prepare(vec![(ContentAccessibilityOwner(1),snapshot.clone()),(ContentAccessibilityOwner(2),snapshot.clone())]);
    assert!(duplicate.is_none());
    assert!(!WindowAccessibilityPublication::commit(&mut slot,duplicate));
    assert!(!saved.is_current());assert!(saved.nodes().is_empty());
    let child=tekes_ui_platform_macos::content_accessibility::content_key(&host,CHILD,None).unwrap();
    let mut mapping=std::collections::HashMap::new();mapping.insert(child,child);
    let illegal_parent=snapshot.clone().with_native_keys(&mapping);
    assert!(illegal_parent.is_none());
    let next=WindowAccessibilityPublication::prepare(vec![(ContentAccessibilityOwner(1),snapshot.clone())]).unwrap();
    let next_saved=next.clone();
    assert!(WindowAccessibilityPublication::commit(&mut slot,Some(next)));
    let rejected=illegal_parent.and_then(|s|WindowAccessibilityPublication::prepare(vec![(ContentAccessibilityOwner(1),s)]));
    assert!(!WindowAccessibilityPublication::commit(&mut slot,rejected));
    assert!(!next_saved.is_current());
    let recovery=WindowAccessibilityPublication::prepare(vec![(ContentAccessibilityOwner(1),snapshot)]).unwrap();
    assert!(WindowAccessibilityPublication::commit(&mut slot,Some(recovery)));
    assert!(!saved.is_current());
}

#[test]
fn inactive_original_window_cannot_republish_geometry_or_text_authority() {
    use tekes_ui_platform_macos::content_accessibility::ContentAccessibilitySnapshot;
    let mut host = ContentHost::new(Content::new(
        ROOT,
        panel(Content::new(FIELD, TextFieldContent::new("Composer", "draft")), false),
    )).unwrap();
    render(&mut host, &mut Text::new(true), Rect::new(0., 0., 100., 100.));
    let saved = ContentAccessibilitySnapshot::from_host(&host).unwrap();
    host.set_window_active(false);
    assert!(!saved.is_current());
    assert!(ContentAccessibilitySnapshot::from_host(&host).is_none());
    assert!(tekes_ui_platform_macos::content_accessibility::content_nodes(&host).is_empty());
    assert!(tekes_ui_platform_macos::content_accessibility::content_text_layouts(&host).is_empty());
}
