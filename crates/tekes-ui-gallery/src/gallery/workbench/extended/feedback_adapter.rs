//! Gallery adapters for the production feedback and overlay views.
//!
//! The story keeps its deterministic fixture models; these runtimes own their
//! component geometry, clipping, transient pointer state and presentation.
use tekes_ui_controls::{
    ButtonModel, ButtonTextRenderer, ControlKey,
    extended::{Banner, EmptyState, Popover, Progress, StatusBadge, Toast, Tooltip},
    feedback_view::{
        BannerView, EmptyStateView, OverlayEvent, OverlayLayout, ProgressView, StatusBadgeView,
        SurfaceView, ToastView, TooltipEvent, TooltipView, ViewLayout,
    },
};
use tekes_ui_scene::{Point, Rect, Scene, Size};
use tekes_ui_tokens::{ColorRole, Palette, proposal};

use super::super::{Entry, PageCtx, StoryState, action, line, quiet_action};
use super::collections_adapter::InputResult;
use super::{ExtendedStory, controls};

const POPOVER_CONTENT: &str = "Popover content";

#[derive(Debug, Default)]
pub(super) struct FeedbackViews {
    pub tooltip: TooltipView,
    pub popover: tekes_ui_controls::feedback_view::PopoverView,
    pub toast: MountedToast,
    pub banner: MountedBanner,
    pub status_badge: StatusBadgeView,
    pub progress: ProgressView,
    pub empty_state: MountedEmptyState,
    pub surface: SurfaceView,
}


#[derive(Debug)]
pub(super) struct MountedToast{host:Option<tekes_ui_controls::composition::ContentHost>,input:Option<Toast>,active:bool,focus:(bool,bool)}
impl Default for MountedToast{fn default()->Self{Self{host:None,input:None,active:true,focus:(false,false)}}}
impl MountedToast{
 const ROOT:tekes_ui_controls::composition::ComponentId=tekes_ui_controls::composition::ComponentId(79001);
 const ACTION:tekes_ui_controls::composition::ComponentId=tekes_ui_controls::composition::ComponentId(79002);
 fn model(&self)->Option<Toast>{Some(self.host.as_ref()?.get(Self::ROOT)?.component::<tekes_ui_controls::composition::ToastContent>()?.model().clone())}
 #[allow(clippy::too_many_arguments)]
 fn render(&mut self,scene:&mut Scene,text:&mut impl ButtonTextRenderer,model:&Toast,frame:Rect,clip:Option<Rect>,palette:Palette,_role:proposal::TextRole)->ViewLayout{
  use tekes_ui_controls::composition::*;
  let created=self.host.is_none();
  let host=self.host.get_or_insert_with(||{let mut h=ContentHost::new(Content::new(Self::ROOT,ToastContent::new("Toast",model.clone(),Self::ACTION).unwrap())).unwrap();h.set_action_events(true);h.set_window_active(self.active);h});
  if !created&&self.input.as_ref()!=Some(model){
   host.show_toast(Self::ROOT,model.message.clone(),model.tone,model.remaining_ms()).unwrap();host.set_toast_visible(Self::ROOT,model.visible).unwrap();host.set_toast_paused(Self::ROOT,model.paused).unwrap();
  }
  self.input=Some(model.clone());
  host.render(scene,text,frame,clip.unwrap_or(frame),palette).expect("Gallery Toast Content layout and clock");
  host.focus(self.focus.0.then_some(Self::ACTION),self.focus.1);
  self.layout()
 }
 fn layout(&self)->ViewLayout{self.host.as_ref().and_then(|h|h.get(Self::ROOT)).and_then(|n|n.component::<tekes_ui_controls::composition::ToastContent>()).map_or(ViewLayout::default(),|c|c.layout())}
 fn logical_focus_target(&self)->Option<Rect>{Some(self.host.as_ref()?.semantics().into_iter().find(|n|n.id==Self::ACTION)?.frame)}
 fn focus_target(&self)->Option<Rect>{self.host.as_ref()?.semantics().into_iter().find(|n|n.id==Self::ACTION)?.visible}
 fn event(&mut self,event:tekes_ui_controls::composition::ContentEvent)->tekes_ui_controls::composition::ContentResponse{
  let Some(h)=&mut self.host else{return Default::default();};let result=h.event(&event);let activated=h.drain_actions().into_iter().any(|a|a.target==Self::ACTION);tekes_ui_controls::composition::ContentResponse{activated,..result}
 }
 fn pointer_move(&mut self,model:&mut Toast,p:Point){self.event(tekes_ui_controls::composition::ContentEvent::PointerMove(p));if let Some(next)=self.model(){*model=next;self.input=Some(model.clone());}}
 fn pointer_down(&mut self,p:Point)->bool{self.event(tekes_ui_controls::composition::ContentEvent::PointerDown(p)).consumed}
 fn pointer_up(&mut self,model:&mut Toast,p:Point)->bool{let result=self.event(tekes_ui_controls::composition::ContentEvent::PointerUp(p)).activated;if let Some(next)=self.model(){*model=next;self.input=Some(model.clone());}result}
 fn key(&mut self,model:&mut Toast,key:ControlKey,down:bool)->bool{let result=self.event(tekes_ui_controls::composition::ContentEvent::Key{key,down}).activated;if let Some(next)=self.model(){*model=next;self.input=Some(model.clone());}result}
 fn cancel(&mut self){if let Some(h)=&mut self.host{h.cancel();}}
 fn set_window_active(&mut self,active:bool){self.active=active;if let Some(h)=&mut self.host{h.set_window_active(active);}}
 fn focus_dismiss(&mut self,focused:bool,visible:bool){self.focus=(focused,visible);if let Some(h)=&mut self.host{h.focus(focused.then_some(Self::ACTION),visible);}}
 fn advance(&mut self,model:&mut Toast,ms:u32)->bool{model.advance(ms)}
}

#[derive(Debug)]
pub(super) struct MountedBanner{host:Option<tekes_ui_controls::composition::ContentHost>,input:Option<Banner>,active:bool,focus:(bool,bool)}
impl Default for MountedBanner{fn default()->Self{Self{host:None,input:None,active:true,focus:(false,false)}}}
impl MountedBanner{
 const ROOT:tekes_ui_controls::composition::ComponentId=tekes_ui_controls::composition::ComponentId(79003);
 const ACTION:tekes_ui_controls::composition::ComponentId=tekes_ui_controls::composition::ComponentId(79004);
 fn model(&self)->Option<Banner>{Some(self.host.as_ref()?.get(Self::ROOT)?.component::<tekes_ui_controls::composition::BannerContent>()?.model().clone())}
 #[allow(clippy::too_many_arguments)]
 fn render(&mut self,scene:&mut Scene,text:&mut impl ButtonTextRenderer,model:&Banner,frame:Rect,clip:Option<Rect>,palette:Palette,_role:proposal::TextRole)->ViewLayout{
  use tekes_ui_controls::composition::*;
  let created=self.host.is_none();
  let host=self.host.get_or_insert_with(||{let mut h=ContentHost::new(Content::new(Self::ROOT,BannerContent::new("Banner",model.clone(),Self::ACTION).unwrap())).unwrap();h.set_action_events(true);h.set_window_active(self.active);h});
  if !created&&self.input.as_ref()!=Some(model){
   host.replace_banner(Self::ROOT,model.clone()).unwrap();
  }
  self.input=Some(model.clone());
  host.render(scene,text,frame,clip.unwrap_or(frame),palette).expect("Gallery Banner Content layout and clock");
  host.focus(self.focus.0.then_some(Self::ACTION),self.focus.1);
  self.layout()
 }
 fn layout(&self)->ViewLayout{self.host.as_ref().and_then(|h|h.get(Self::ROOT)).and_then(|n|n.component::<tekes_ui_controls::composition::BannerContent>()).map_or(ViewLayout::default(),|c|c.layout())}
 fn logical_focus_target(&self)->Option<Rect>{Some(self.host.as_ref()?.semantics().into_iter().find(|n|n.id==Self::ACTION)?.frame)}
 fn focus_target(&self)->Option<Rect>{self.host.as_ref()?.semantics().into_iter().find(|n|n.id==Self::ACTION)?.visible}
 fn event(&mut self,event:tekes_ui_controls::composition::ContentEvent)->tekes_ui_controls::composition::ContentResponse{
  let Some(h)=&mut self.host else{return Default::default();};let result=h.event(&event);let activated=h.drain_actions().into_iter().any(|a|a.target==Self::ACTION);tekes_ui_controls::composition::ContentResponse{activated,..result}
 }
 fn pointer_move(&mut self,p:Point){self.event(tekes_ui_controls::composition::ContentEvent::PointerMove(p));}
 fn pointer_down(&mut self,p:Point)->bool{self.event(tekes_ui_controls::composition::ContentEvent::PointerDown(p)).consumed}
 fn pointer_up(&mut self,model:&mut Banner,p:Point)->bool{let result=self.event(tekes_ui_controls::composition::ContentEvent::PointerUp(p)).activated;if let Some(next)=self.model(){*model=next;self.input=Some(model.clone());}result}
 fn key(&mut self,model:&mut Banner,key:ControlKey,down:bool)->bool{let result=self.event(tekes_ui_controls::composition::ContentEvent::Key{key,down}).activated;if let Some(next)=self.model(){*model=next;self.input=Some(model.clone());}result}
 fn cancel(&mut self){if let Some(h)=&mut self.host{h.cancel();}}
 fn set_window_active(&mut self,active:bool){self.active=active;if let Some(h)=&mut self.host{h.set_window_active(active);}}
 fn focus_dismiss(&mut self,focused:bool,visible:bool){self.focus=(focused,visible);if let Some(h)=&mut self.host{h.focus(focused.then_some(Self::ACTION),visible);}}
}
/// Gallery host adapter: layout/theme/copy stay here; SDK owns action routing.
#[derive(Debug)]
pub(super) struct MountedEmptyState {
    host: Option<tekes_ui_controls::composition::ContentHost>,
    disabled: bool,
    active: bool,
    focus: (bool, bool),
}
impl Default for MountedEmptyState {
    fn default() -> Self { Self { host: None, disabled: false, active: true, focus: (false, false) } }
}
impl MountedEmptyState {
    const ROOT: tekes_ui_controls::composition::ComponentId = tekes_ui_controls::composition::ComponentId(78001);
    const ACTION: tekes_ui_controls::composition::ComponentId = tekes_ui_controls::composition::ComponentId(78002);
    fn set_disabled(&mut self, disabled: bool) {
        self.disabled = disabled;
        if let Some(host) = &mut self.host { host.set_empty_state_disabled(Self::ROOT, disabled).expect("Gallery valid EmptyState"); }
    }
    fn set_window_active(&mut self, active: bool) {
        self.active = active;
        if let Some(host) = &mut self.host { host.set_window_active(active); }
    }
    fn focus_action(&mut self, focused: bool, visible: bool) {
        self.focus = (focused, visible);
        if let Some(host) = &mut self.host { host.focus(focused.then_some(Self::ACTION), visible); }
    }
    #[allow(clippy::too_many_arguments)]
    fn render(&mut self, scene: &mut Scene, text: &mut impl ButtonTextRenderer, model: &EmptyState,
        frame: Rect, clip: Option<Rect>, palette: Palette, _role: proposal::TextRole) -> ViewLayout {
        use tekes_ui_controls::composition::*;
        let host = self.host.get_or_insert_with(|| {
            let mut host = ContentHost::new(Content::new(Self::ROOT,
                EmptyStateContent::new("Empty state", model.clone(), Self::ACTION).expect("fixture action name"))).unwrap();
            host.set_action_events(true); host.set_window_active(self.active); host
        });
        if *host.get(Self::ROOT).unwrap().component::<EmptyStateContent>().unwrap().model() != *model {
            host.replace_empty_state(Self::ROOT, model.clone()).expect("valid fixture update");
        }
        host.set_empty_state_disabled(Self::ROOT, self.disabled).expect("valid fixture availability");
        // The parent already allocated a measured content slot. ContentHost
        // renders the retained tree and publishes its actual action target.
        host.render(scene, text, frame, clip.unwrap_or(frame), palette).expect("Gallery EmptyState layout");
        host.focus(self.focus.0.then_some(Self::ACTION), self.focus.1);
        self.layout()
    }
    fn layout(&self) -> ViewLayout {
        self.host.as_ref().and_then(|h|h.get(Self::ROOT)).and_then(|n|n.component::<tekes_ui_controls::composition::EmptyStateContent>())
            .map_or(ViewLayout::default(), |c|c.layout())
    }
    fn logical_focus_target(&self) -> Option<Rect> {
        self.host.as_ref()?.semantics().into_iter().find(|n|n.id==Self::ACTION).map(|n|n.frame)
    }
    fn focus_target(&self) -> Option<Rect> {
        self.host.as_ref()?.semantics().into_iter().find(|n|n.id==Self::ACTION)?.visible
    }
    fn event(&mut self, event: tekes_ui_controls::composition::ContentEvent) -> tekes_ui_controls::composition::ContentResponse {
        let Some(host) = &mut self.host else { return Default::default(); };
        let result = host.event(&event);
        let activated = host.drain_actions().into_iter().any(|a|a.target==Self::ACTION);
        tekes_ui_controls::composition::ContentResponse { activated, ..result }
    }
    fn pointer_down(&mut self, point: Point) -> bool { self.event(tekes_ui_controls::composition::ContentEvent::PointerDown(point)).consumed }
    fn pointer_move(&mut self, point: Point) { self.event(tekes_ui_controls::composition::ContentEvent::PointerMove(point)); }
    fn pointer_up(&mut self, point: Point) -> bool { self.event(tekes_ui_controls::composition::ContentEvent::PointerUp(point)).activated }
    fn key(&mut self, key: ControlKey, down: bool) -> bool { self.event(tekes_ui_controls::composition::ContentEvent::Key { key, down }).activated }
}

pub(super) fn supports(renderer: &str) -> bool {
    matches!(
        renderer,
        "tooltip" | "popover" | "toast" | "banner" | "status-badge" | "progress" | "empty-state"
    )
}

/// Feedback controls with an actual transient or focus state supplied by the
/// production view. This is a visual comparison only; Playground owns native
/// pointer and keyboard verification.
pub(super) fn supports_states(renderer: &str) -> bool {
    matches!(
        renderer,
        "tooltip" | "popover" | "toast" | "banner" | "empty-state"
    )
}

/// Contractual values/modes presented as variants, rather than invented
/// hover/pressed states for controls that do not define them.
pub(super) fn supports_variants(renderer: &str) -> bool {
    matches!(
        renderer,
        "toast" | "banner" | "status-badge" | "progress" | "empty-state"
    )
}

pub(super) fn matrix(
    entry: &Entry,
    state: &mut StoryState,
    ctx: &mut PageCtx<'_>,
    scene: &mut Scene,
    viewport: Rect,
) {
    if !supports_states(entry.renderer())
        || viewport.size.width <= 0.0
        || viewport.size.height <= 0.0
    {
        return;
    }
    let gutter = -controls::ControlFrame::new(Rect::new(0., 0., 1., 1.), None)
        .focus_bounds(ctx.text.backing_scale())
        .min_x();
    let bounds = viewport.inset(gutter);
    scene.push_clip(viewport);
    let width = bounds.size.width.max(0.0);
    let mut y = bounds.min_y();
    let palette = Palette::new(ctx.theme, state.brand).expect("registered Gallery brand");
    let description = "Production-view state specimens. Use Playground for real pointer, keyboard, and focus behavior.";
    matrix_caption(ctx, scene, bounds.min_x(), &mut y, width, description);
    y += 12.0;
    match entry.renderer() {
        "tooltip" => matrix_tooltip(ctx, scene, bounds, &mut y, palette),
        "popover" => matrix_popover(ctx, scene, bounds, &mut y, palette),
        "toast" => matrix_toast(ctx, scene, bounds, &mut y, palette),
        "banner" => matrix_banner(ctx, scene, bounds, &mut y, palette),
        "empty-state" => matrix_empty_state(ctx, scene, bounds, &mut y, palette),
        _ => {}
    }
    state.document_bottom = state.document_bottom.max(y + gutter);
    scene.pop_clip();
}

pub(super) fn variants(
    entry: &Entry,
    state: &mut StoryState,
    ctx: &mut PageCtx<'_>,
    scene: &mut Scene,
    viewport: Rect,
) {
    if !supports_variants(entry.renderer())
        || viewport.size.width <= 0.0
        || viewport.size.height <= 0.0
    {
        return;
    }
    let gutter = -controls::ControlFrame::new(Rect::new(0., 0., 1., 1.), None)
        .focus_bounds(ctx.text.backing_scale())
        .min_x();
    let bounds = viewport.inset(gutter);
    scene.push_clip(viewport);
    let width = bounds.size.width.max(0.0);
    let mut y = bounds.min_y();
    let palette = Palette::new(ctx.theme, state.brand).expect("registered Gallery brand");
    let description =
        "Supported model values and modes, rendered by the production presentation views.";
    matrix_caption(ctx, scene, bounds.min_x(), &mut y, width, description);
    y += 12.0;
    match entry.renderer() {
        "toast" => matrix_toast_variants(ctx, scene, bounds, &mut y, palette),
        "banner" => matrix_banner_variants(ctx, scene, bounds, &mut y, palette),
        "status-badge" => matrix_status_badges(ctx, scene, bounds, &mut y, palette),
        "progress" => matrix_progress_variants(ctx, scene, bounds, &mut y, palette),
        "empty-state" => matrix_empty_variants(ctx, scene, bounds, &mut y, palette),
        _ => {}
    }
    state.document_bottom = state.document_bottom.max(y + gutter);
    scene.pop_clip();
}

fn matrix_caption(
    ctx: &mut PageCtx<'_>,
    scene: &mut Scene,
    x: f32,
    y: &mut f32,
    w: f32,
    text: &str,
) {
    let role = proposal::TextRole::UiCaption;
    let size = ctx.text.measure(text, role, w.max(0.0));
    ctx.text.proposal_wrapped_in(
        scene,
        ctx.theme,
        Rect::new(x, *y, w.max(0.0), size.height),
        text,
        role,
        ColorRole::SecondaryText.resolve(ctx.theme),
    );
    *y += size.height.max(18.0) + 8.0;
}

fn matrix_heading(
    ctx: &mut PageCtx<'_>,
    scene: &mut Scene,
    bounds: Rect,
    y: f32,
    title: &str,
) -> f32 {
    let role = proposal::TextRole::UiBody;
    let size = ctx.text.measure(title, role, bounds.size.width.max(0.0));
    ctx.text.proposal_wrapped_in(
        scene,
        ctx.theme,
        Rect::new(
            bounds.min_x(),
            y,
            bounds.size.width.max(0.0),
            size.height.max(20.0),
        ),
        title,
        role,
        ColorRole::PrimaryText.resolve(ctx.theme),
    );
    size.height.max(20.0)
}

fn bounded_fixture_width(width: f32, maximum: f32) -> f32 {
    // Preserve the previous min/max behavior for a malformed NaN viewport:
    // f32::min(NaN, maximum) selects maximum, while clamp(NaN, ..) stays NaN.
    if width.is_nan() {
        maximum
    } else {
        width.clamp(0.0, maximum)
    }
}

fn matrix_clip(ctx: &PageCtx<'_>, bounds: Rect) -> Rect {
    let gutter = -controls::ControlFrame::new(Rect::new(0., 0., 1., 1.), None)
        .focus_bounds(ctx.text.backing_scale())
        .min_x();
    Rect::new(
        bounds.min_x() - gutter,
        bounds.min_y() - gutter,
        bounds.size.width + gutter * 2.0,
        bounds.size.height + gutter * 2.0,
    )
}

fn matrix_button(
    ctx: &mut PageCtx<'_>,
    scene: &mut Scene,
    frame: Rect,
    clip: Rect,
    palette: Palette,
    label: &str,
    focused: bool,
) {
    let mut button = ButtonModel::default();
    button.state.focused = focused;
    button.state.focus_visible = focused;
    button.render(
        scene,
        ctx.text,
        label,
        frame,
        Some(clip),
        palette,
        tekes_ui_controls::paint::ButtonVariant::Quiet,
        tekes_ui_controls::ButtonSize::Body,
        proposal::TextRole::UiBody,
        tekes_ui_controls::ButtonTextAlign::Center,
    );
}

fn matrix_tooltip(
    ctx: &mut PageCtx<'_>,
    scene: &mut Scene,
    bounds: Rect,
    y: &mut f32,
    palette: Palette,
) {
    let clip = matrix_clip(ctx, bounds);
    let width = bounded_fixture_width(bounds.size.width, 420.0);
    for (title, interaction) in [
        ("Rest · hidden", 0),
        ("Hover · shown", 1),
        ("Focus-visible · shown", 2),
        ("Escape · dismissed until leave", 3),
    ] {
        *y += matrix_heading(ctx, scene, bounds, *y, title) + 6.0;
        let button_size = ButtonModel::measure(
            ctx.text,
            "Hover or focus me",
            width.min(190.0),
            tekes_ui_controls::ButtonSize::Body,
            proposal::TextRole::UiBody,
        );
        let anchor = Rect::new(
            bounds.min_x(),
            *y,
            button_size.width.max(160.0).min(width),
            button_size.height.max(36.0),
        );
        matrix_button(
            ctx,
            scene,
            anchor,
            clip,
            palette,
            "Hover or focus me",
            interaction == 2,
        );
        let tip_text = "Tooltip content wraps naturally at the current preview width.";
        let tip_width = width.min(240.0);
        let tip_text_height = ctx
            .text
            .measure(
                tip_text,
                proposal::TextRole::UiBody,
                (tip_width - 24.0).max(0.0),
            )
            .height;
        let tip_size = Size::new(tip_width, tip_text_height + 16.0);
        let model = Tooltip {
            visible: false,
            text: tip_text.into(),
        };
        let mut view = TooltipView::default();
        if interaction == 1 || interaction == 3 {
            let mut probe = Scene::new();
            view.render(
                &mut probe,
                ctx.text,
                &model,
                anchor,
                tip_size,
                clip,
                Some(clip),
                palette,
                proposal::TextRole::UiBody,
            );
            view.pointer_move(Point::new(anchor.min_x() + 8.0, anchor.min_y() + 8.0));
        }
        if interaction == 2 {
            view.set_focus(true, true);
        }
        if interaction == 3 {
            let _ = view.key(ControlKey::Escape);
        }
        view.render(
            scene,
            ctx.text,
            &model,
            anchor,
            tip_size,
            clip,
            Some(clip),
            palette,
            proposal::TextRole::UiBody,
        );
        *y = (*y + button_size.height.max(36.0) + 54.0).max(view.layout().frame.max_y()) + 15.0;
    }
}

fn matrix_popover(
    ctx: &mut PageCtx<'_>,
    scene: &mut Scene,
    bounds: Rect,
    y: &mut f32,
    palette: Palette,
) {
    let clip = matrix_clip(ctx, bounds);
    let width = bounded_fixture_width(bounds.size.width, 420.0);
    for (title, open, focused) in [
        ("Closed", false, false),
        ("Open", true, false),
        ("Open · dismiss action focus-visible", true, true),
    ] {
        *y += matrix_heading(ctx, scene, bounds, *y, title) + 6.0;
        let button_size = ButtonModel::measure(
            ctx.text,
            "More options",
            180.0_f32.min(width),
            tekes_ui_controls::ButtonSize::Body,
            proposal::TextRole::UiBody,
        );
        let anchor = Rect::new(
            bounds.min_x(),
            *y,
            button_size.width.max(140.0).min(width),
            button_size.height.max(36.0),
        );
        matrix_button(ctx, scene, anchor, clip, palette, "More options", false);
        let mut model = Popover::default();
        if open {
            model.show(20);
        }
        let mut view = tekes_ui_controls::feedback_view::PopoverView::default();
        if open {
            let size = tekes_ui_controls::feedback_view::PopoverView::measure(
                ctx.text,
                Size::new((width.min(232.0) - 28.0).max(0.0), 24.0),
                proposal::TextRole::UiBody,
            );
            if focused {
                let mut probe = Scene::new();
                view.render_with_text_content(
                    &mut probe,
                    ctx.text,
                    &model,
                    anchor,
                    size,
                    clip,
                    Some(clip),
                    palette,
                    proposal::TextRole::UiBody,
                    "Gallery popover focus geometry",
                    |scene, text, slot| {
                        text.proposal_label_in(
                            scene,
                            ctx.theme,
                            slot,
                            "Popover content",
                            proposal::TextRole::UiBody,
                            ColorRole::PrimaryText.resolve(ctx.theme),
                        );
                    },
                );
                view.focus_dismiss(true, true);
            }
            view.render_with_text_content(
                scene,
                ctx.text,
                &model,
                anchor,
                size,
                clip,
                Some(clip),
                palette,
                proposal::TextRole::UiBody,
                "Gallery popover state",
                |scene, text, slot| {
                    text.proposal_label_in(
                        scene,
                        ctx.theme,
                        slot,
                        "Popover content",
                        proposal::TextRole::UiBody,
                        ColorRole::PrimaryText.resolve(ctx.theme),
                    );
                },
            );
            *y = view.layout().frame.max_y() + 15.0;
        } else {
            *y += anchor.size.height + 15.0;
        }
    }
}

fn matrix_toast(
    ctx: &mut PageCtx<'_>,
    scene: &mut Scene,
    bounds: Rect,
    y: &mut f32,
    palette: Palette,
) {
    let clip = matrix_clip(ctx, bounds);
    let width = bounded_fixture_width(bounds.size.width, 500.0);
    for (title, visible, focused) in [
        ("Visible · default", true, false),
        ("Visible · dismiss action focus-visible", true, true),
        ("Dismissed", false, false),
    ] {
        *y += matrix_heading(ctx, scene, bounds, *y, title) + 6.0;
        let message = if visible {
            "Changes saved"
        } else {
            "Toast dismissed."
        };
        let mut model = Toast::new(message, controls::NoticeTone::Success, 5000);
        model.paused = true;
        model.visible = visible;
        let size = ToastView::measure(ctx.text, message, width, proposal::TextRole::UiBody);
        let frame = Rect::new(bounds.min_x(), *y, width, size.height);
        let mut view = MountedToast::default();
        if visible && focused {
            let mut probe = Scene::new();
            view.render(
                &mut probe,
                ctx.text,
                &model,
                frame,
                Some(clip),
                palette,
                proposal::TextRole::UiBody,
            );
            view.focus_dismiss(true, true);
        }
        view.render(
            scene,
            ctx.text,
            &model,
            frame,
            Some(clip),
            palette,
            proposal::TextRole::UiBody,
        );
        *y += size.height + 14.0;
    }
}

fn matrix_banner(
    ctx: &mut PageCtx<'_>,
    scene: &mut Scene,
    bounds: Rect,
    y: &mut f32,
    palette: Palette,
) {
    let clip = matrix_clip(ctx, bounds);
    let width = bounded_fixture_width(bounds.size.width, 600.0);
    for (title, visible, dismissible, focused) in [
        ("Visible · dismissible", true, true, false),
        ("Dismiss action focus-visible", true, true, true),
        ("Visible · persistent", true, false, false),
        ("Dismissed", false, true, false),
    ] {
        *y += matrix_heading(ctx, scene, bounds, *y, title) + 6.0;
        let model = Banner {
            message: "Your changes are saved. You can continue working.".into(),
            tone: controls::NoticeTone::Info,
            dismissible,
            visible,
        };
        let size = BannerView::measure(
            ctx.text,
            &model.message,
            width,
            dismissible,
            proposal::TextRole::UiBody,
        );
        let frame = Rect::new(bounds.min_x(), *y, width, size.height);
        let mut view = MountedBanner::default();
        if focused {
            let mut probe = Scene::new();
            view.render(
                &mut probe,
                ctx.text,
                &model,
                frame,
                Some(clip),
                palette,
                proposal::TextRole::UiBody,
            );
            view.focus_dismiss(true, true);
        }
        view.render(
            scene,
            ctx.text,
            &model,
            frame,
            Some(clip),
            palette,
            proposal::TextRole::UiBody,
        );
        *y += size.height.max(30.0) + 14.0;
    }
}

fn matrix_empty_state(
    ctx: &mut PageCtx<'_>,
    scene: &mut Scene,
    bounds: Rect,
    y: &mut f32,
    palette: Palette,
) {
    let clip = matrix_clip(ctx, bounds);
    let width = bounded_fixture_width(bounds.size.width, 520.0);
    for (title, action, disabled, focused) in [
        ("Action enabled", Some("Create document"), false, false),
        ("Action focus-visible", Some("Create document"), false, true),
        ("Action disabled", Some("Create document"), true, false),
        ("No action", None, false, false),
    ] {
        *y += matrix_heading(ctx, scene, bounds, *y, title) + 6.0;
        let model = EmptyState {
            title: "No documents yet".into(),
            detail: "Create a document to begin your workspace.".into(),
            action: action.map(str::to_owned),
        };
        let height = empty_state_matrix_height(ctx, &model, width);
        let frame = Rect::new(bounds.min_x(), *y, width, height);
        let mut surface = SurfaceView::default();
        let mut probe = Scene::new();
        let panel = surface.render_at_scale(
            &mut probe,
            frame,
            Some(clip),
            palette,
            true,
            ctx.text.backing_scale(),
        );
        let mut view = MountedEmptyState::default();
        view.set_disabled(disabled);
        if focused {
            view.render(
                &mut probe,
                ctx.text,
                &model,
                panel.content,
                Some(clip),
                palette,
                proposal::TextRole::UiBody,
            );
            view.focus_action(true, true);
        }
        let panel = surface.render_at_scale(
            scene,
            frame,
            Some(clip),
            palette,
            true,
            ctx.text.backing_scale(),
        );
        view.render(
            scene,
            ctx.text,
            &model,
            panel.content,
            Some(clip),
            palette,
            proposal::TextRole::UiBody,
        );
        *y += height + 14.0;
    }
}

fn matrix_toast_variants(
    ctx: &mut PageCtx<'_>,
    scene: &mut Scene,
    bounds: Rect,
    y: &mut f32,
    palette: Palette,
) {
    let clip = matrix_clip(ctx, bounds);
    let width = bounded_fixture_width(bounds.size.width, 520.0);
    for (label, tone) in [
        ("Info", controls::NoticeTone::Info),
        ("Success", controls::NoticeTone::Success),
        ("Warning", controls::NoticeTone::Warning),
        ("Error", controls::NoticeTone::Error),
    ] {
        *y += matrix_heading(ctx, scene, bounds, *y, label) + 6.0;
        let mut model = Toast::new("Changes saved", tone, 5000);
        model.paused = true;
        let size = ToastView::measure(ctx.text, &model.message, width, proposal::TextRole::UiBody);
        let frame = Rect::new(bounds.min_x(), *y, width, size.height);
        MountedToast::default().render(
            scene,
            ctx.text,
            &model,
            frame,
            Some(clip),
            palette,
            proposal::TextRole::UiBody,
        );
        *y += size.height + 12.0;
    }
}

fn matrix_banner_variants(
    ctx: &mut PageCtx<'_>,
    scene: &mut Scene,
    bounds: Rect,
    y: &mut f32,
    palette: Palette,
) {
    let clip = matrix_clip(ctx, bounds);
    let width = bounded_fixture_width(bounds.size.width, 600.0);
    for (label, tone) in [
        ("Info", controls::NoticeTone::Info),
        ("Success", controls::NoticeTone::Success),
        ("Warning", controls::NoticeTone::Warning),
        ("Error", controls::NoticeTone::Error),
    ] {
        *y += matrix_heading(ctx, scene, bounds, *y, label) + 6.0;
        let model = Banner {
            message: "Your changes are saved. You can continue working.".into(),
            tone,
            dismissible: true,
            visible: true,
        };
        let size = BannerView::measure(
            ctx.text,
            &model.message,
            width,
            model.dismissible,
            proposal::TextRole::UiBody,
        );
        let frame = Rect::new(bounds.min_x(), *y, width, size.height);
        MountedBanner::default().render(
            scene,
            ctx.text,
            &model,
            frame,
            Some(clip),
            palette,
            proposal::TextRole::UiBody,
        );
        *y += size.height + 12.0;
    }
}

fn matrix_status_badges(
    ctx: &mut PageCtx<'_>,
    scene: &mut Scene,
    bounds: Rect,
    y: &mut f32,
    palette: Palette,
) {
    let clip = matrix_clip(ctx, bounds);
    let labels = ["Info", "Running", "Succeeded", "Failed"];
    let statuses = [
        StatusBadge::Info,
        StatusBadge::Running,
        StatusBadge::Succeeded,
        StatusBadge::Failed,
    ];
    let (columns, cell_width, cell_height, _) = status_badge_grid(ctx, bounds.size.width);
    let gap = 12.0;
    *y += matrix_heading(ctx, scene, bounds, *y, "Status variants") + 6.0;
    for (i, (label, status)) in labels.into_iter().zip(statuses).enumerate() {
        let frame = Rect::new(
            bounds.min_x() + (i % columns) as f32 * (cell_width + gap),
            *y + (i / columns) as f32 * (cell_height + 8.0),
            cell_width,
            cell_height,
        );
        StatusBadgeView::default().render_labelled_aligned(scene, ctx.text, status, label,
            frame, frame.inset(5.), Some(clip), palette, tekes_ui_controls::ButtonTextAlign::Leading).expect("finite Gallery badge allocation");
    }
    *y += labels.len().div_ceil(columns) as f32 * (cell_height + 8.0);
}

fn matrix_progress_variants(
    ctx: &mut PageCtx<'_>,
    scene: &mut Scene,
    bounds: Rect,
    y: &mut f32,
    palette: Palette,
) {
    let clip = matrix_clip(ctx, bounds);
    for (label, mut model) in [
        ("0% · determinate", Progress::new(0.0)),
        ("45% · determinate", Progress::new(0.45)),
        ("100% · determinate", Progress::new(1.0)),
        ("Indeterminate", Progress::new(0.45)),
    ] {
        *y += matrix_heading(ctx, scene, bounds, *y, label) + 6.0;
        if label == "Indeterminate" {
            model.indeterminate = true;
        }
        let frame = Rect::new(bounds.min_x(), *y, bounds.size.width.min(500.0), 12.0);
        ProgressView::default().render(scene, model, frame, Some(clip), palette);
        *y += frame.size.height + 18.0;
    }
}

fn matrix_empty_variants(
    ctx: &mut PageCtx<'_>,
    scene: &mut Scene,
    bounds: Rect,
    y: &mut f32,
    palette: Palette,
) {
    let clip = matrix_clip(ctx, bounds);
    for (title, action) in [
        ("With primary action", Some("Create document")),
        ("Without action", None),
    ] {
        *y += matrix_heading(ctx, scene, bounds, *y, title) + 6.0;
        let model = EmptyState {
            title: "No documents yet".into(),
            detail: "Create a document to begin your workspace.".into(),
            action: action.map(str::to_owned),
        };
        let width = bounded_fixture_width(bounds.size.width, 520.0);
        let height = empty_state_matrix_height(ctx, &model, width);
        let frame = Rect::new(bounds.min_x(), *y, width, height);
        let mut surface = SurfaceView::default();
        let panel = surface.render_at_scale(
            scene,
            frame,
            Some(clip),
            palette,
            true,
            ctx.text.backing_scale(),
        );
        MountedEmptyState::default().render(
            scene,
            ctx.text,
            &model,
            panel.content,
            Some(clip),
            palette,
            proposal::TextRole::UiBody,
        );
        *y += height + 14.0;
    }
}

fn empty_state_matrix_height(ctx: &mut PageCtx<'_>, model: &EmptyState, width: f32) -> f32 {
    let inset = proposal::metric("spacing", "space-loose").unwrap_or(16.0);
    EmptyStateView::measure(
        ctx.text,
        model,
        (width - 2.0 * inset).max(0.0),
        proposal::TextRole::UiBody,
    )
    .height
        + 2.0 * inset
}


fn status_badge_grid(ctx: &mut PageCtx<'_>, width: f32) -> (usize, f32, f32, usize) {
    let labels = ["Info", "Running", "Succeeded", "Failed"];
    let columns = if width >= 452.0 {
        4
    } else if width >= 220.0 {
        2
    } else {
        1
    };
    let gap = 12.0;
    let cell_width = ((width - gap * (columns - 1) as f32) / columns as f32).max(0.0);
    let cell_height = labels
        .iter()
        .map(|label| StatusBadgeView::measure_label(ctx.text, label,
            Size::new((cell_width - 10.).max(0.), f32::MAX)).expect("finite Gallery allocation").height + 10.)
        .fold(28.0_f32, f32::max);
    let rows = labels.len().div_ceil(columns);
    (columns, cell_width, cell_height, rows)
}

/// Draws one feedback story from the existing fixtures. Layout and inner
/// button hit targets come from the production presentation views.
#[allow(clippy::too_many_arguments)]
pub(super) fn build(
    ext: &mut ExtendedStory,
    entry: &Entry,
    state: &mut StoryState,
    ctx: &mut PageCtx<'_>,
    scene: &mut Scene,
    body: Rect,
    clip: Rect,
    palette: Palette,
) -> bool {
    let renderer = entry.renderer();
    if !supports(renderer) {
        return false;
    }
    let clip = body
        .intersection(&clip)
        .unwrap_or(Rect::new(body.min_x(), body.min_y(), 0.0, 0.0));
    state.hits.retain(|(id, _)| !component_id(renderer, *id));
    state.button_hits.retain(|id| !component_id(renderer, *id));
    let x = body.min_x();
    let y = body.min_y();
    let width = body.size.width.max(0.0);
    let natural_bottom;
    let focus_visible = state.focus.is_some_and(|id| {
        state
            .buttons
            .get(&id)
            .is_some_and(|button| button.state.focus_visible)
    });
    match renderer {
        "tooltip" => {
            let anchor_size =
                measured_action_size(ctx, "Hover or focus me", width, 164.0, 36.0, state.density);
            let anchor = Rect::new(x, y + 16.0, anchor_size.width, anchor_size.height);
            action(
                state,
                ctx,
                scene,
                20,
                anchor,
                "Hover or focus me",
                (false, false),
            );
            ext.feedback_views
                .tooltip_set_focus(state.focus == Some(20), focus_visible);
            let tooltip_width = width.min(276.0);
            let tooltip_text =
                measured_body_size(ctx, &ext.tooltip.text, (tooltip_width - 24.0).max(0.0));
            let tooltip_size = Size::new(tooltip_width, 48.0_f32.max(tooltip_text.height + 16.0));
            ext.feedback_views.render_tooltip(
                scene,
                ctx.text,
                &ext.tooltip,
                anchor,
                tooltip_size,
                body,
                Some(clip),
                palette,
                proposal::TextRole::UiBody,
            );
            let note = "Tooltip appears on hover or focus-visible. Escape dismisses it.";
            let tooltip_layout = ext.feedback_views.tooltip.layout();
            let note_y = anchor.max_y().max(tooltip_layout.frame.max_y()) + 12.0;
            let note_size = measured_caption_size(ctx, note, width);
            draw_measured_caption(
                ctx,
                scene,
                Rect::new(x, note_y, width, note_size.height),
                note,
                24.0,
            );
            natural_bottom = note_y + note_size.height;
        }
        "popover" => {
            let anchor_size =
                measured_action_size(ctx, "More options", width, 150.0, 36.0, state.density);
            let anchor = Rect::new(x, y + 10.0, anchor_size.width, anchor_size.height);
            action(
                state,
                ctx,
                scene,
                20,
                anchor,
                "More options",
                (ext.popover.open, false),
            );
            ext.feedback_views
                .popover_focus_dismiss(state.focus == Some(21), focus_visible);
            let popover_width = width.min(232.0);
            let content_text = POPOVER_CONTENT;
            let content_size =
                measured_body_size(ctx, content_text, (popover_width - 28.0).max(0.0));
            let popover_size = tekes_ui_controls::feedback_view::PopoverView::measure(
                ctx.text,
                Size::new((popover_width - 28.0).max(0.0), content_size.height),
                proposal::TextRole::UiBody,
            );
            if ext.popover.open || ext.feedback_views.popover.is_presented() {
                let theme = ctx.theme;
                ext.feedback_views.render_popover_with_content(
                    scene,
                    ctx.text,
                    &ext.popover,
                    anchor,
                    popover_size,
                    body,
                    Some(clip),
                    palette,
                    proposal::TextRole::UiBody,
                    |scene, text, bounds| {
                        text.proposal_label_in(
                            scene,
                            theme,
                            bounds,
                            content_text,
                            proposal::TextRole::UiBody,
                            ColorRole::PrimaryText.resolve(theme),
                        );
                    },
                );
                if let Some(hit) = ext.feedback_views.popover_focus_target() {
                    state.hits.push((21, hit));
                    state.button_labels.insert(21, "Dismiss".to_owned());
                }
            }
            let note = "Escape or outside click dismisses and restores the invoker focus.";
            let overlay_bottom = if ext.popover.open || ext.feedback_views.popover.is_presented() {
                ext.feedback_views.popover.layout().frame.max_y()
            } else {
                anchor.max_y()
            };
            let note_y = anchor.max_y().max(overlay_bottom) + 12.0;
            let note_size = measured_caption_size(ctx, note, width);
            draw_measured_caption(
                ctx,
                scene,
                Rect::new(x, note_y, width, note_size.height),
                note,
                24.0,
            );
            natural_bottom = note_y + note_size.height;
        }
        "toast" => {
            if ext.toast.visible {
                let toast_width = width.min(520.0);
                let toast_size = ToastView::measure(
                    ctx.text,
                    &ext.toast.message,
                    toast_width,
                    proposal::TextRole::UiBody,
                );
                let frame = Rect::new(x, y + 12.0, toast_width, toast_size.height);
                ext.feedback_views.render_toast(
                    scene,
                    ctx.text,
                    &mut ext.toast,
                    frame,
                    Some(clip),
                    palette,
                    proposal::TextRole::UiBody,
                );
                if let Some(hit) = ext.feedback_views.toast_focus_target() {
                    state.hits.push((20, hit));
                    state.button_labels.insert(20, "Dismiss".to_owned());
                }
                let timer = format!("{} ms remaining", ext.toast.remaining_ms());
                let toast_bottom = frame.max_y();
                let (action_frame, timer_frame, note_frame) = toast_fixture_layout(
                    ctx,
                    x,
                    y,
                    width,
                    toast_bottom,
                    &timer,
                    "Time follows the Host frame clock; hover pauses the original Toast.",
                    state.density,
                );
                quiet_action(
                    state,
                    ctx,
                    scene,
                    21,
                    action_frame,
                    "Advance fixture timer",
                    (false, false),
                );
                draw_measured_caption(ctx, scene, timer_frame, &timer, 28.0);
                draw_measured_caption(
                    ctx,
                    scene,
                    note_frame,
                    "Time follows the Host frame clock; hover pauses the original Toast.",
                    24.0,
                );
                natural_bottom = note_frame.max_y();
            } else {
                let dismissed_note = "Toast dismissed.";
                let dismissed_size = measured_caption_size(ctx, dismissed_note, width);
                line(
                    ctx,
                    scene,
                    Rect::new(x, y + 20.0, width, dismissed_size.height.max(28.0)),
                    dismissed_note,
                );
                let show_label = "Show again";
                let show_size =
                    measured_action_size(ctx, show_label, width, 112.0, 34.0, state.density);
                let show_y = y + 20.0 + dismissed_size.height.max(28.0) + 8.0;
                quiet_action(
                    state,
                    ctx,
                    scene,
                    21,
                    Rect::new(x, show_y, show_size.width, show_size.height),
                    show_label,
                    (false, false),
                );
                let note = "Time follows the Host frame clock; hover pauses the original Toast.";
                let note_size = measured_caption_size(ctx, note, width);
                let note_y = show_y + show_size.height + 8.0;
                draw_measured_caption(
                    ctx,
                    scene,
                    Rect::new(x, note_y, width, note_size.height),
                    note,
                    24.0,
                );
                natural_bottom = note_y + note_size.height;
            }
        }
        "banner" => {
            if ext.banner.visible {
                let banner_width = width.min(640.0);
                let banner_size = BannerView::measure(
                    ctx.text,
                    &ext.banner.message,
                    banner_width,
                    ext.banner.dismissible,
                    proposal::TextRole::UiBody,
                );
                let frame = Rect::new(x, y + 12.0, banner_width, banner_size.height);
                ext.feedback_views.render_banner(
                    scene,
                    ctx.text,
                    &mut ext.banner,
                    frame,
                    Some(clip),
                    palette,
                    proposal::TextRole::UiBody,
                );
                if let Some(hit) = ext.feedback_views.banner_focus_target() {
                    state.hits.push((20, hit));
                    state.button_labels.insert(20, "Dismiss".to_owned());
                }
                natural_bottom = frame.max_y();
            } else {
                let note = "Banner dismissed. Reset restores the initial fixture.";
                let note_size = measured_caption_size(ctx, note, width);
                line(
                    ctx,
                    scene,
                    Rect::new(x, y + 20.0, width, note_size.height.max(28.0)),
                    note,
                );
                natural_bottom = y + 20.0 + note_size.height.max(28.0);
            }
        }
        "status-badge" => {
            let labels = ["Info", "Running", "Succeeded", "Failed"];
            let gap = 12.0;
            let (columns, cell_width, cell_height, rows) = status_badge_grid(ctx, width);
            for (index, status) in [
                StatusBadge::Info,
                StatusBadge::Running,
                StatusBadge::Succeeded,
                StatusBadge::Failed,
            ]
            .into_iter()
            .enumerate()
            {
                let column = index % columns;
                let row = index / columns;
                let frame = Rect::new(
                    x + column as f32 * (cell_width + gap),
                    y + 20.0 + row as f32 * (cell_height + 8.0),
                    cell_width,
                    cell_height,
                );
                ext.feedback_views.status_badge.render_labelled_aligned(scene, ctx.text, status, labels[index],
                    frame, frame.inset(5.), Some(clip), palette, tekes_ui_controls::ButtonTextAlign::Leading).expect("finite Gallery badge allocation");
            }
            natural_bottom = y + 20.0 + rows as f32 * (cell_height + 8.0) - 8.0;
        }
        "progress" => {
            let frame = Rect::new(x, y + 26.0, width.min(500.0), 12.0);
            ext.feedback_views
                .render_progress(scene, ext.progress, frame, Some(clip), palette);
            let percentage = if ext.progress.indeterminate {
                "Indeterminate".to_owned()
            } else {
                format!("{:.0}%", ext.progress.value() * 100.0)
            };
            let (button_frame, percentage_frame) =
                progress_fixture_layout(ctx, x, y, width, &percentage, state.density);
            action(
                state,
                ctx,
                scene,
                20,
                button_frame,
                "Advance 10%",
                (false, ext.progress.value() >= 1.0),
            );
            let natural = measured_body_size(ctx, &percentage, percentage_frame.size.width);
            if natural.height <= 34.0 {
                ctx.text.proposal_label_in(
                    scene,
                    ctx.theme,
                    Rect::new(
                        percentage_frame.min_x(),
                        percentage_frame.min_y(),
                        percentage_frame.size.width,
                        34.0,
                    ),
                    &percentage,
                    proposal::TextRole::UiBody,
                    ColorRole::PrimaryText.resolve(ctx.theme),
                );
            } else {
                wrapped_body(ctx, scene, percentage_frame, &percentage);
            }
            natural_bottom = percentage_frame.max_y();
        }
        "empty-state" => {
            let model = empty_state_model(ext);
            let panel_width = width.min(520.0);
            let natural_height = empty_state_matrix_height(ctx, &model, panel_width).max(164.0);
            let frame = Rect::new(x, y + 12.0, panel_width, natural_height);
            let backing_scale = ctx.text.scale();
            let panel = ext.feedback_views.surface.render_at_scale(
                scene,
                frame,
                Some(clip),
                palette,
                true,
                backing_scale,
            );
            ext.feedback_views.empty_state_set_disabled(ext.disabled);
            ext.feedback_views
                .empty_state_focus_action(state.focus == Some(20), focus_visible);
            ext.feedback_views.render_empty_state(
                scene,
                ctx.text,
                &model,
                panel.content,
                Some(clip),
                palette,
                proposal::TextRole::UiBody,
            );
            if let Some(hit) = ext.feedback_views.empty_state_focus_target() {
                state.hits.push((20, hit));
                state
                    .button_labels
                    .insert(20, model.action.clone().expect("visible action"));
            }
            natural_bottom = frame.max_y();
        }
        _ => return false,
    }
    state.document_bottom = state.document_bottom.max(natural_bottom);
    true
}

fn measured_action_size(
    ctx: &mut PageCtx<'_>,
    label: &str,
    max_width: f32,
    baseline_width: f32,
    baseline_height: f32,
    density: usize,
) -> Size {
    let measured = ctx.text.measure_button(
        label,
        max_width.max(0.0),
        tekes_ui_controls::ButtonSize::Body,
        proposal::TextRole::UiBody,
    );
    // The shared Gallery action helper applies the density reduction to the
    // supplied frame. Compensate here so the measured text slot stays intact.
    let density_compensation = if density == 1 { 4.0 } else { 0.0 };
    Size::new(
        measured.width.max(baseline_width).min(max_width.max(0.0)),
        measured.height.max(baseline_height) + density_compensation,
    )
}

fn measured_caption_size(ctx: &mut PageCtx<'_>, text: &str, width: f32) -> Size {
    ctx.text
        .measure(text, proposal::TextRole::UiCaption, width.max(0.0))
}

fn measured_body_size(ctx: &mut PageCtx<'_>, text: &str, width: f32) -> Size {
    ctx.text
        .measure(text, proposal::TextRole::UiBody, width.max(0.0))
}

fn wrapped_body(ctx: &mut PageCtx<'_>, scene: &mut Scene, rect: Rect, text: &str) {
    ctx.text.proposal_wrapped_in(
        scene,
        ctx.theme,
        rect,
        text,
        proposal::TextRole::UiBody,
        ColorRole::PrimaryText.resolve(ctx.theme),
    );
}

fn draw_measured_caption(
    ctx: &mut PageCtx<'_>,
    scene: &mut Scene,
    measured: Rect,
    text: &str,
    legacy_height: f32,
) {
    let size = measured_caption_size(ctx, text, measured.size.width);
    if size.height <= legacy_height {
        line(
            ctx,
            scene,
            Rect::new(
                measured.min_x(),
                measured.min_y(),
                measured.size.width,
                legacy_height,
            ),
            text,
        );
    } else {
        ctx.text.proposal_wrapped_in(
            scene,
            ctx.theme,
            Rect::new(
                measured.min_x(),
                measured.min_y(),
                measured.size.width,
                size.height,
            ),
            text,
            proposal::TextRole::UiCaption,
            ColorRole::SecondaryText.resolve(ctx.theme),
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn toast_fixture_layout(
    ctx: &mut PageCtx<'_>,
    x: f32,
    y: f32,
    width: f32,
    toast_bottom: f32,
    timer: &str,
    note: &str,
    density: usize,
) -> (Rect, Rect, Rect) {
    let action_size =
        measured_action_size(ctx, "Advance fixture timer", width, 156.0, 34.0, density);
    let timer_natural = measured_caption_size(ctx, timer, width);
    let controls_y = (y + 84.0).max(toast_bottom + 8.0);
    let action = Rect::new(x, controls_y, action_size.width, action_size.height);
    let inline = width >= action_size.width + 10.0 + timer_natural.width;
    let timer_x = if inline { action.max_x() + 10.0 } else { x };
    let timer_y = if inline {
        controls_y
    } else {
        action.max_y() + 8.0
    };
    let timer_rect_width = (x + width - timer_x).max(0.0);
    let timer_size = measured_caption_size(ctx, timer, timer_rect_width);
    let timer_rect = Rect::new(timer_x, timer_y, timer_rect_width, timer_size.height);
    let row_bottom = action.max_y().max(timer_rect.max_y());
    let note_y = (y + 126.0).max(row_bottom + 8.0);
    let note_size = measured_caption_size(ctx, note, width);
    (
        action,
        timer_rect,
        Rect::new(x, note_y, width, note_size.height),
    )
}

fn progress_fixture_layout(
    ctx: &mut PageCtx<'_>,
    x: f32,
    y: f32,
    width: f32,
    percentage: &str,
    density: usize,
) -> (Rect, Rect) {
    let action_size = measured_action_size(ctx, "Advance 10%", width, 118.0, 34.0, density);
    let natural_percentage = measured_body_size(ctx, percentage, width);
    let action = Rect::new(x, y + 54.0, action_size.width, action_size.height);
    let inline = width >= action_size.width + 14.0 + natural_percentage.width;
    let percentage_x = if inline { action.max_x() + 14.0 } else { x };
    let percentage_y = if inline {
        action.min_y()
    } else {
        action.max_y() + 8.0
    };
    let percentage_width = (x + width - percentage_x).max(0.0);
    let percentage_size = measured_body_size(ctx, percentage, percentage_width);
    let percentage_height = percentage_size.height.max(if inline { 34.0 } else { 0.0 });
    (
        action,
        Rect::new(
            percentage_x,
            percentage_y,
            percentage_width,
            percentage_height,
        ),
    )
}

fn empty_state_model(ext: &ExtendedStory) -> EmptyState {
    if ext.empty_state_created {
        EmptyState {
            title: "Documents".into(),
            detail: "One fixture document was created.".into(),
            action: None,
        }
    } else {
        EmptyState {
            title: "No documents yet".into(),
            detail: "Create a document to begin your workspace.".into(),
            action: Some("Create document".into()),
        }
    }
}

/// Live production surface, not a state-matrix specimen or retained exit paint.
/// Body names share the same models/strings used by the painter.
pub(super) fn semantic_surface(ext: &ExtendedStory, renderer: &str) -> Option<(Rect, String)> {
    let (frame, label) = match renderer {
        "popover" if ext.popover.open => (
            ext.feedback_views.popover.layout().visible?,
            POPOVER_CONTENT.to_owned(),
        ),
        "toast" if ext.toast.visible => (
            ext.feedback_views.toast.layout().visible?,
            ext.toast.message.clone(),
        ),
        "banner" if ext.banner.visible => (
            ext.feedback_views.banner.layout().visible?,
            ext.banner.message.clone(),
        ),
        "empty-state" => {
            let model = empty_state_model(ext);
            (
                ext.feedback_views.empty_state.layout().visible?,
                format!("{}. {}", model.title, model.detail),
            )
        }
        _ => return None,
    };
    Some((frame, label))
}

fn component_id(renderer: &str, id: usize) -> bool {
    match renderer {
        "tooltip" | "popover" | "banner" | "empty-state" => (20..=21).contains(&id),
        "toast" => (20..=21).contains(&id),
        "progress" => id == 20,
        _ => false,
    }
}

/// Returns full, unclipped child-action frames from the last real Playground
/// layout. State/variant specimens deliberately never participate in focus.
pub(super) fn logical_focus_targets(ext: &ExtendedStory, renderer: &str) -> Vec<(usize, Rect)> {
    let target = match renderer {
        "popover" => ext
            .feedback_views
            .popover
            .logical_focus_target()
            .map(|frame| (21, frame)),
        "toast" => ext
            .feedback_views
            .toast
            .logical_focus_target()
            .map(|frame| (20, frame)),
        "banner" => ext
            .feedback_views
            .banner
            .logical_focus_target()
            .map(|frame| (20, frame)),
        "empty-state" => ext
            .feedback_views
            .empty_state
            .logical_focus_target()
            .map(|frame| (20, frame)),
        _ => None,
    };
    target.into_iter().collect()
}

pub(super) fn accepts_key(renderer: &str, focus: Option<usize>, key: ControlKey) -> bool {
    match renderer {
        // Escape is also valid while the tooltip is only hovered; TooltipView
        // decides whether that current transient state can consume it.
        "tooltip" => key == ControlKey::Escape,
        "popover" => focus.is_some_and(|id| (20..=21).contains(&id)) || key == ControlKey::Escape,
        "toast" => focus == Some(20),
        "banner" | "empty-state" => focus == Some(20),
        _ => false,
    }
}

fn overlay_result(
    event: OverlayEvent,
    ext: &mut ExtendedStory,
    log: &mut impl FnMut(String),
) -> InputResult {
    match event {
        OverlayEvent::None => InputResult {
            consumed: false,
            changed: false,
        },
        OverlayEvent::Dismissed { restore_focus } => {
            ext.focused = restore_focus;
            log("popover: dismissed; focus restored".into());
            InputResult {
                consumed: true,
                changed: true,
            }
        }
    }
}

pub(super) fn activate(
    ext: &mut ExtendedStory,
    renderer: &str,
    id: usize,
    log: &mut impl FnMut(String),
) -> Option<InputResult> {
    let changed = match (renderer, id) {
        // Tooltip visibility is owned by TooltipView's hover/focus state.
        // Activating the anchor has no persistent model effect.
        ("tooltip", 20) => false,
        ("popover", 20) => {
            if ext.popover.open {
                let event = ext.feedback_views.popover_close(&mut ext.popover);
                let _ = overlay_result(event, ext, log);
            } else {
                ext.feedback_views.popover_show(&mut ext.popover, 20);
                log("popover: open".into());
            }
            true
        }
        ("toast", 21) => {
            if ext.toast.visible {
                let expired = ext.feedback_views.toast_advance(&mut ext.toast, 1000);
                log(if expired {
                    "toast: fixture timer elapsed".into()
                } else {
                    "toast timer: advanced 1 s".into()
                });
            } else {
                ext.toast
                    .show("Changes saved", controls::NoticeTone::Success, 5000);
                log("toast: shown (5 s fixture)".into());
            }
            true
        }
        ("progress", 20) => {
            let next = (ext.progress.value() + 0.1).min(1.0);
            let changed = ext.progress.set(next);
            if changed {
                log(format!("progress: {:.0}%", next * 100.0));
            }
            changed
        }
        _ => return None,
    };
    Some(InputResult {
        consumed: true,
        changed,
    })
}

pub(super) fn key(
    ext: &mut ExtendedStory,
    renderer: &str,
    focus: Option<usize>,
    key: ControlKey,
    down: bool,
    log: &mut impl FnMut(String),
) -> Option<InputResult> {
    if !supports(renderer) || !accepts_key(renderer, focus, key) {
        return None;
    }
    let result = match renderer {
        "tooltip" => {
            if !down {
                return Some(InputResult {
                    consumed: true,
                    changed: false,
                });
            }
            let changed = ext.feedback_views.tooltip_key(key) == TooltipEvent::Dismissed;
            if changed {
                log("tooltip: dismissed".into());
            }
            InputResult {
                // TooltipView handles Escape without changing the fixture model.
                consumed: changed,
                changed,
            }
        }
        "popover" if ext.popover.open => overlay_result(
            ext.feedback_views.popover_key(&mut ext.popover, key, down),
            ext,
            log,
        ),
        "toast" if focus == Some(20) => {
            let changed = ext.feedback_views.toast_key(&mut ext.toast, key, down);
            if changed {
                log("toast: dismissed".into());
            }
            InputResult {
                consumed: changed,
                changed,
            }
        }
        "banner" => {
            let changed = ext.feedback_views.banner_key(&mut ext.banner, key, down);
            if changed {
                log("banner: dismissed".into());
            }
            InputResult {
                consumed: changed,
                changed,
            }
        }
        "empty-state" => {
            let changed = ext.feedback_views.empty_state_key(key, down);
            if changed {
                ext.empty_state_created = true;
                log("empty-state action: created Documents fixture item".into());
            }
            InputResult {
                consumed: changed,
                changed,
            }
        }
        _ => return None,
    };
    Some(result)
}

pub(super) fn pointer_move(
    ext: &mut ExtendedStory,
    renderer: &str,
    point: Point,
) -> Option<InputResult> {
    match renderer {
        "tooltip" => ext.feedback_views.tooltip_pointer_move(point),
        "popover" => ext.feedback_views.popover_pointer_move(point),
        "toast" => ext.feedback_views.toast_pointer_move(&mut ext.toast, point),
        "banner" => ext.feedback_views.banner_pointer_move(point),
        "empty-state" => ext.feedback_views.empty_state_pointer_move(point),
        _ => return None,
    }
    Some(InputResult {
        consumed: true,
        changed: false,
    })
}

pub(super) fn pointer_down(
    ext: &mut ExtendedStory,
    renderer: &str,
    point: Point,
) -> Option<InputResult> {
    let consumed = match renderer {
        "popover" if ext.popover.open => ext
            .feedback_views
            .popover_pointer_down(&mut ext.popover, point),
        "toast" => ext.feedback_views.toast_pointer_down(point),
        "banner" => ext.feedback_views.banner_pointer_down(point),
        "empty-state" => ext.feedback_views.empty_state_pointer_down(point),
        "tooltip" | "progress" | "status-badge" => return None,
        _ => return None,
    };
    Some(InputResult {
        consumed,
        changed: false,
    })
}

pub(super) fn pointer_up(
    ext: &mut ExtendedStory,
    renderer: &str,
    point: Point,
    log: &mut impl FnMut(String),
) -> Option<InputResult> {
    match renderer {
        "popover" if ext.popover.open => Some(overlay_result(
            ext.feedback_views
                .popover_pointer_up(&mut ext.popover, point),
            ext,
            log,
        )),
        "toast" => {
            let changed = ext.feedback_views.toast_pointer_up(&mut ext.toast, point);
            if changed {
                log("toast: dismissed".into());
            }
            Some(InputResult {
                consumed: changed,
                changed,
            })
        }
        "banner" => {
            let changed = ext.feedback_views.banner_pointer_up(&mut ext.banner, point);
            if changed {
                log("banner: dismissed".into());
            }
            Some(InputResult {
                consumed: changed,
                changed,
            })
        }
        "empty-state" => {
            let changed = ext.feedback_views.empty_state_pointer_up(point);
            if changed {
                ext.empty_state_created = true;
                log("empty-state action: created Documents fixture item".into());
            }
            Some(InputResult {
                consumed: changed,
                changed,
            })
        }
        "tooltip" | "progress" | "status-badge" => None,
        _ => None,
    }
}

pub(super) fn focus_changed(
    ext: &mut ExtendedStory,
    renderer: &str,
    focus: Option<usize>,
    focus_visible: bool,
) {
    match renderer {
        "tooltip" => ext
            .feedback_views
            .tooltip_set_focus(focus == Some(20), focus_visible),
        "popover" => ext
            .feedback_views
            .popover_focus_dismiss(focus == Some(21), focus_visible),
        "toast" => ext
            .feedback_views
            .toast_focus_dismiss(focus == Some(20), focus_visible),
        "banner" => ext
            .feedback_views
            .banner_focus_dismiss(focus == Some(20), focus_visible),
        "empty-state" => ext
            .feedback_views
            .empty_state_focus_action(focus == Some(20), focus_visible),
        _ => {}
    }
}

pub(super) fn blur(ext: &mut ExtendedStory, renderer: &str) {
    match renderer {
        "tooltip" => ext.feedback_views.tooltip_blur(),
        "popover" => {
            let _ = ext.feedback_views.popover_close(&mut ext.popover);
            ext.feedback_views.popover.clear_presentation();
        }
        "toast" => ext.feedback_views.toast_cancel(),
        "banner" => ext.feedback_views.banner_cancel(),
        "empty-state" => ext.feedback_views.empty_state_focus_action(false, false),
        _ => {}
    }
}

pub(super) fn window_active(
    ext: &mut ExtendedStory,
    renderer: &str,
    active: bool,
    log: &mut impl FnMut(String),
) -> Option<InputResult> {
    match renderer {
        "tooltip" => ext.feedback_views.tooltip_set_window_active(active),
        "popover" => {
            let event = ext
                .feedback_views
                .popover_set_window_active(active, &mut ext.popover);
            let result = overlay_result(event, ext, log);
            return Some(result);
        }
        "toast" => ext.feedback_views.toast_set_window_active(active),
        "banner" => ext.feedback_views.banner_set_window_active(active),
        "empty-state" => ext.feedback_views.empty_state_set_window_active(active),
        "status-badge" | "progress" => {}
        _ => return None,
    }
    Some(InputResult {
        consumed: true,
        changed: false,
    })
}

impl FeedbackViews {
    #[allow(clippy::too_many_arguments)]
    pub fn render_tooltip(
        &mut self,
        scene: &mut Scene,
        text: &mut impl tekes_ui_controls::ButtonTextRenderer,
        model: &Tooltip,
        anchor: Rect,
        size: Size,
        viewport: Rect,
        clip: Option<Rect>,
        palette: Palette,
        role: proposal::TextRole,
    ) -> OverlayLayout {
        self.tooltip.render(
            scene, text, model, anchor, size, viewport, clip, palette, role,
        )
    }

    pub fn tooltip_pointer_move(&mut self, point: Point) {
        self.tooltip.pointer_move(point);
    }

    pub fn tooltip_set_focus(&mut self, focused: bool, focus_visible: bool) {
        self.tooltip.set_focus(focused, focus_visible);
    }

    pub fn tooltip_key(&mut self, key: ControlKey) -> TooltipEvent {
        self.tooltip.key(key)
    }

    pub fn tooltip_blur(&mut self) {
        self.tooltip.blur();
    }

    pub fn tooltip_set_window_active(&mut self, active: bool) {
        self.tooltip.set_window_active(active);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render_popover_with_content<T: tekes_ui_controls::ButtonTextRenderer>(
        &mut self,
        scene: &mut Scene,
        text: &mut T,
        model: &Popover,
        anchor: Rect,
        content_size: Size,
        viewport: Rect,
        clip: Option<Rect>,
        palette: Palette,
        role: proposal::TextRole,
        draw_content: impl FnOnce(&mut Scene, &mut T, Rect),
    ) -> OverlayLayout {
        let size = tekes_ui_controls::feedback_view::PopoverView::measure(text, content_size, role);
        self.popover.render_with_text_content(
            scene,
            text,
            model,
            anchor,
            size,
            viewport,
            clip,
            palette,
            role,
            "Popover content",
            draw_content,
        )
    }

    pub fn popover_show(&mut self, model: &mut Popover, invoker: usize) {
        self.popover.show(model, invoker);
    }

    pub fn popover_close(&mut self, model: &mut Popover) -> OverlayEvent {
        self.popover.close(model)
    }

    pub fn popover_set_window_active(&mut self, active: bool, model: &mut Popover) -> OverlayEvent {
        self.popover.set_window_active(active, model)
    }

    pub fn popover_pointer_down(&mut self, model: &mut Popover, point: Point) -> bool {
        self.popover.pointer_down(model, point)
    }

    pub fn popover_pointer_move(&mut self, point: Point) {
        self.popover.pointer_move(point);
    }

    pub fn popover_pointer_up(&mut self, model: &mut Popover, point: Point) -> OverlayEvent {
        self.popover.pointer_up(model, point)
    }

    pub fn popover_key(
        &mut self,
        model: &mut Popover,
        key: ControlKey,
        down: bool,
    ) -> OverlayEvent {
        self.popover.key_event(model, key, down)
    }

    pub fn popover_focus_dismiss(&mut self, focused: bool, focus_visible: bool) {
        self.popover.focus_dismiss(focused, focus_visible);
    }

    pub fn popover_focus_target(&self) -> Option<Rect> {
        self.popover.focus_target()
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render_toast(
        &mut self,
        scene: &mut Scene,
        text: &mut impl tekes_ui_controls::ButtonTextRenderer,
        model: &mut Toast,
        frame: Rect,
        clip: Option<Rect>,
        palette: Palette,
        role: proposal::TextRole,
    ) -> ViewLayout {
        let layout=self.toast.render(scene,text,model,frame,clip,palette,role);
        if let Some(next)=self.toast.model(){*model=next;self.toast.input=Some(model.clone());}
        layout
    }

    pub fn toast_pointer_move(&mut self, model: &mut Toast, point: Point) {
        self.toast.pointer_move(model, point);
    }

    pub fn toast_pointer_down(&mut self, point: Point) -> bool {
        self.toast.pointer_down(point)
    }

    pub fn toast_pointer_up(&mut self, model: &mut Toast, point: Point) -> bool {
        self.toast.pointer_up(model, point)
    }

    pub fn toast_key(&mut self, model: &mut Toast, key: ControlKey, down: bool) -> bool {
        self.toast.key(model, key, down)
    }

    pub fn toast_advance(&mut self, model: &mut Toast, elapsed_ms: u32) -> bool {
        self.toast.advance(model, elapsed_ms)
    }

    pub fn toast_cancel(&mut self) {
        self.toast.cancel();
    }

    pub fn toast_set_window_active(&mut self, active: bool) {
        self.toast.set_window_active(active);
    }

    pub fn toast_focus_dismiss(&mut self, focused: bool, focus_visible: bool) {
        self.toast.focus_dismiss(focused, focus_visible);
    }

    pub fn toast_focus_target(&self) -> Option<Rect> {
        self.toast.focus_target()
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render_banner(
        &mut self,
        scene: &mut Scene,
        text: &mut impl tekes_ui_controls::ButtonTextRenderer,
        model: &mut Banner,
        frame: Rect,
        clip: Option<Rect>,
        palette: Palette,
        role: proposal::TextRole,
    ) -> ViewLayout {
        let layout=self.banner.render(scene,text,model,frame,clip,palette,role);
        if let Some(next)=self.banner.model(){*model=next;self.banner.input=Some(model.clone());}
        layout
    }

    pub fn banner_pointer_move(&mut self, point: Point) {
        self.banner.pointer_move(point);
    }

    pub fn banner_pointer_down(&mut self, point: Point) -> bool {
        self.banner.pointer_down(point)
    }

    pub fn banner_pointer_up(&mut self, model: &mut Banner, point: Point) -> bool {
        self.banner.pointer_up(model, point)
    }

    pub fn banner_key(&mut self, model: &mut Banner, key: ControlKey, down: bool) -> bool {
        self.banner.key(model, key, down)
    }

    pub fn banner_cancel(&mut self) {
        self.banner.cancel();
    }

    pub fn banner_set_window_active(&mut self, active: bool) {
        self.banner.set_window_active(active);
    }

    pub fn banner_focus_dismiss(&mut self, focused: bool, focus_visible: bool) {
        self.banner.focus_dismiss(focused, focus_visible);
    }

    pub fn banner_focus_target(&self) -> Option<Rect> {
        self.banner.focus_target()
    }

    pub fn render_progress(
        &mut self,
        scene: &mut Scene,
        model: Progress,
        frame: Rect,
        clip: Option<Rect>,
        palette: Palette,
    ) -> ViewLayout {
        self.progress.render(scene, model, frame, clip, palette)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render_empty_state(
        &mut self,
        scene: &mut Scene,
        text: &mut impl tekes_ui_controls::ButtonTextRenderer,
        model: &EmptyState,
        frame: Rect,
        clip: Option<Rect>,
        palette: Palette,
        role: proposal::TextRole,
    ) -> ViewLayout {
        self.empty_state
            .render(scene, text, model, frame, clip, palette, role)
    }

    pub fn empty_state_pointer_down(&mut self, point: Point) -> bool {
        self.empty_state.pointer_down(point)
    }

    pub fn empty_state_pointer_move(&mut self, point: Point) {
        self.empty_state.pointer_move(point);
    }

    pub fn empty_state_set_disabled(&mut self, disabled: bool) {
        self.empty_state.set_disabled(disabled);
    }

    pub fn empty_state_set_window_active(&mut self, active: bool) {
        self.empty_state.set_window_active(active);
    }

    pub fn empty_state_focus_action(&mut self, focused: bool, focus_visible: bool) {
        self.empty_state.focus_action(focused, focus_visible);
    }

    pub fn empty_state_focus_target(&self) -> Option<Rect> {
        self.empty_state.focus_target()
    }

    pub fn empty_state_pointer_up(&mut self, point: Point) -> bool {
        self.empty_state.pointer_up(point)
    }

    pub fn empty_state_key(&mut self, key: ControlKey, down: bool) -> bool {
        self.empty_state.key(key, down)
    }
}

#[cfg(test)]
mod layout_tests {
    use super::*;
    use crate::gallery::{static_text::StaticText, text::TextEngine};

    fn context<'a>(text: &'a mut TextEngine, static_text: &'a mut StaticText) -> PageCtx<'a> {
        PageCtx {
            theme: tekes_ui_tokens::Theme::LIGHT,
            text,
            static_text,
        }
    }

    #[test]
    #[ignore = "explicit GPU acceptance with fresh SSD output"]
    fn empty_state_playground_keeps_the_full_measured_action_visible() {
        let entry = crate::gallery::workbench::entries()
            .iter()
            .find(|entry| entry.renderer() == "empty-state")
            .unwrap();
        let output = Some(std::path::PathBuf::from(std::env::var_os("TEKESUI_EMPTY_STATE_OUT")
            .expect("fresh SSD output required for explicit EmptyState GPU capture")));
        let path = output.as_ref().unwrap();
        assert!(path.starts_with("/Volumes/HP P900/DEV/TekesUI/artifacts"), "SSD evidence output required");
        std::fs::create_dir(path).expect("fresh output directory required");
        let mut renderer = tekes_ui_render::Renderer::new_headless().expect("real GPU required");
        let mut records = Vec::new();
        for (theme_index, theme) in tekes_ui_tokens::Theme::ALL.into_iter().enumerate() {
            for width in [220., 420., 640.] {
                for type_scale in [1., 1.5] {
                    let mut ext = ExtendedStory::default();
                    let mut state = StoryState::for_entry(entry);
                    let mut text = TextEngine::new();
                    text.set_scale(2.);
                    text.set_type_scale(type_scale);
                    text.begin_frame();
                    let mut static_text = StaticText::new(None);
                    let mut ctx = context(&mut text, &mut static_text);
                    ctx.theme = theme;
                    let mut scene = Scene::new();
                    let body = Rect::new(24., 20., width, 1200.);
                    assert!(build(
                        &mut ext,
                        entry,
                        &mut state,
                        &mut ctx,
                        &mut scene,
                        body,
                        body,
                        Palette::from(theme)
                    ));
                    let view = &ext.feedback_views.empty_state;
                    let full = view.logical_focus_target().unwrap();
                    let visible = view.focus_target().unwrap();
                    assert_eq!(
                        visible, full,
                        "width={width} type_scale={type_scale}: natural host must not clip its action"
                    );
                    assert!(view.layout().frame.max_y() >= full.max_y());
                    let action_ink: Vec<_> = ink_bounds(&scene, full, 2.)
                        .into_iter()
                        .filter(|r| r.center().y >= full.min_y())
                        .collect();
                    assert!(!action_ink.is_empty());
                    assert!(action_ink.iter().all(|r| r.max_y() <= full.max_y() + 0.5));
                    let size = (
                        ((width + 48.) * 2.).ceil() as u32,
                        ((state.document_bottom + 24.) * 2.).ceil() as u32,
                    );
                    let rgba = renderer
                        .render_to_rgba8(
                            &scene,
                            text.atlas_mut(),
                            size,
                            2.,
                            crate::gallery::Gallery::background(theme),
                        )
                        .unwrap();
                    let name =
                        format!("empty-theme-{theme_index}-width-{width}-text-{type_scale}.png");
                    if let Some(p) = &output {
                        let mut encoder = png::Encoder::new(
                            std::fs::File::create(p.join(&name)).unwrap(),
                            size.0,
                            size.1,
                        );
                        encoder.set_color(png::ColorType::Rgba);
                        encoder.set_depth(png::BitDepth::Eight);
                        encoder
                            .write_header()
                            .unwrap()
                            .write_image_data(&rgba)
                            .unwrap();
                    }
                    records.push(serde_json::json!({"image":name,"width":width,"type_scale":type_scale,
                        "action_y":full.min_y(),"action_height":full.size.height,"fully_visible":visible==full}));
                }
            }
        }
        if let Some(p) = output {
            std::fs::write(p.join("gpu-receipt.json"), serde_json::to_vec_pretty(&serde_json::json!({
                "scope":"real Gallery EmptyState playground integration, CoreText and GPU; not native acceptance",
                "frames":records,"performance":"not measured"})).unwrap()).unwrap();
        }
    }

    fn ink_bounds(scene: &Scene, slot: Rect, scale: f32) -> Vec<Rect> {
        scene
            .primitives()
            .iter()
            .filter_map(|primitive| match primitive {
                tekes_ui_scene::Primitive::Glyphs(run) => Some(run),
                _ => None,
            })
            .flat_map(|run| run.glyphs.iter())
            .filter_map(|glyph| {
                if glyph.glyph.width == 0 || glyph.glyph.height == 0 {
                    return None;
                }
                let rect = Rect::new(
                    glyph.origin.x + glyph.glyph.left as f32 / scale,
                    glyph.origin.y + glyph.glyph.top as f32 / scale,
                    glyph.glyph.width as f32 / scale,
                    glyph.glyph.height as f32 / scale,
                );
                (rect.center().x >= slot.min_x() && rect.center().x <= slot.max_x()).then_some(rect)
            })
            .collect()
    }

    #[test]
    fn number_field_real_glyph_ink_centers_in_value_slot_at_large_and_narrow_sizes() {
        use tekes_ui_controls::{NumberField, feedback_view::NumberFieldView};

        let mut renderer = tekes_ui_render::Renderer::new_headless()
            .expect("headless GPU is required for real NumberField glyph geometry");
        for width in [198.0, 144.0] {
            let mut text = TextEngine::new();
            text.set_scale(2.0);
            text.set_type_scale(1.5);
            text.begin_frame();
            let mut scene = Scene::new();
            let mut view = NumberFieldView::default();
            let model = NumberField::new(8.0, 0.0, 32.0, 1.0).unwrap();
            let frame = Rect::new(16.0, 16.0, width, 40.0);
            let layout = view.render(
                &mut scene,
                &mut text,
                &model,
                frame,
                None,
                tekes_ui_tokens::Palette::from(tekes_ui_tokens::Theme::LIGHT),
                tekes_ui_tokens::proposal::TextRole::UiBody,
            );

            let number = ink_bounds(&scene, layout.value, text.scale());
            let minus = ink_bounds(&scene, layout.decrement.frame, text.scale());
            let plus = ink_bounds(&scene, layout.increment.frame, text.scale());
            assert_eq!(number.len(), 1, "actual CoreText ink for the value 8");
            assert_eq!(minus.len(), 1, "actual CoreText ink for the minus action");
            assert_eq!(plus.len(), 1, "actual CoreText ink for the plus action");

            let number_ink = number[0];
            let stepper_axis_midpoint =
                (layout.decrement.frame.center().x + layout.increment.frame.center().x) * 0.5;
            assert!(
                (number_ink.center().x - layout.value.center().x).abs() <= 2.0,
                "type scale 150%, width={width}: value ink {:?} must center in slot {:?}",
                number_ink,
                layout.value
            );
            assert!(
                (layout.value.center().x - stepper_axis_midpoint).abs() <= 1.0,
                "width={width}: value slot must stay centered between stepper axes"
            );
            assert!(
                (number_ink.center().y - layout.value.center().y).abs() <= 4.0,
                "type scale 150%, width={width}: value ink {:?} must center on stepper row {:?}",
                number_ink,
                frame
            );
            assert!(
                text.measure("8", tekes_ui_tokens::proposal::TextRole::UiHeading, 80.0)
                    .height
                    > 0.0,
                "the measured 150% display value must be nonempty"
            );
            let backing_scale = text.scale();
            let image_width = (frame.max_x() + 16.0).mul_add(backing_scale, 0.0).round() as u32;
            let image_height = (frame.max_y() + 16.0).mul_add(backing_scale, 0.0).round() as u32;
            let pixels = renderer
                .render_to_rgba8(
                    &scene,
                    text.atlas_mut(),
                    (image_width, image_height),
                    backing_scale,
                    crate::gallery::Gallery::background(tekes_ui_tokens::Theme::LIGHT),
                )
                .expect("render actual NumberField CoreText glyphs through Metal");
            if let Some(output) = std::env::var_os("TEKESUI_GOLD7_PAGE_OUT") {
                let output = std::path::PathBuf::from(output);
                std::fs::create_dir_all(&output).unwrap();
                let file = std::fs::File::create(
                    output.join(format!("number-field-{width}-pt-scale2-type150.png")),
                )
                .unwrap();
                let mut encoder = png::Encoder::new(file, image_width, image_height);
                encoder.set_color(png::ColorType::Rgba);
                encoder.set_depth(png::BitDepth::Eight);
                encoder
                    .write_header()
                    .unwrap()
                    .write_image_data(&pixels)
                    .unwrap();
            }
        }
    }

    #[test]
    #[ignore = "explicit actual GPU capture with fresh SSD output"]
    fn production_notices_align_first_line_with_real_shaping_and_gpu() {
        use tekes_ui_controls::{
            extended::{Banner, NoticeTone, Toast},
            feedback_view::{BannerView, ToastView},
        };
        use tekes_ui_scene::Primitive;
        use tekes_ui_tokens::{Palette, Theme, proposal};
        let output = Some(std::path::PathBuf::from(std::env::var_os("TEKESUI_NOTICE_ALIGNMENT_OUT").expect("fresh SSD output required")));
        let path=output.as_ref().unwrap();assert!(path.starts_with("/Volumes/HP P900/DEV/TekesUI/artifacts"));std::fs::create_dir(path).expect("fresh output required");
        let mut renderer = tekes_ui_render::Renderer::new_headless().expect("real GPU required");
        let mut records = Vec::new();
        for (theme_index, theme) in Theme::ALL.into_iter().enumerate() {
            for width in [220., 640.] {
                for scale in [1., 2.] {
                    for type_scale in [1., 1.5] {
                        for (message_index, message) in ["Changes saved", "文档已保存。Your workspace has unsaved changes that wrap across several lines."].into_iter().enumerate() {
                            let palette = Palette::from(theme);
                            let role = proposal::TextRole::UiBody;
                            let mut text = TextEngine::new(); text.set_scale(scale); text.set_type_scale(type_scale); text.begin_frame();
                            let mut scene = Scene::new();
                            let toast_size = ToastView::measure(&mut text, message, width, role);
                            let toast_frame = Rect::new(16., 16., toast_size.width, toast_size.height);
                            let banner_size = BannerView::measure(&mut text, message, width, true, role);
                            let banner_frame = Rect::new(16., toast_frame.max_y() + 24., banner_size.width, banner_size.height);
                            let mut toast_view = ToastView::default();
                            let toast_layout = toast_view.render(&mut scene, &mut text,
                                &Toast::new(message, NoticeTone::Success, 5000), toast_frame, None, palette, role);
                            let mut banner_view = BannerView::default();
                            banner_view.render(&mut scene, &mut text,
                                &Banner { message: message.into(), tone: NoticeTone::Info, dismissible: true, visible: true },
                                banner_frame, None, palette, role);
                            // Inspect actual rasterized glyph ink, not only an enum or layout slot.
                            for (kind, action) in [("toast", toast_view.focus_target().unwrap()), ("banner", banner_view.focus_target().unwrap())] {
                                let glyphs: Vec<_> = ink_bounds(&scene, action, scale).into_iter()
                                    .filter(|r| r.center().y >= action.min_y() && r.center().y <= action.max_y()).collect();
                                assert!(!glyphs.is_empty(), "real Dismiss glyph ink");
                                let left = glyphs.iter().map(|r| r.min_x()).fold(f32::INFINITY, f32::min);
                                let right = glyphs.iter().map(|r| r.max_x()).fold(f32::NEG_INFINITY, f32::max);
                                assert!(((left + right) / 2. - action.center().x).abs() <= 2.5,
                                    "{kind} action text not centered: ink={left}..{right}, action={action:?}, width={width}, text={type_scale}");
                            }
                            let banner_message_width = if width < 280. { width - 46. } else { width - 220. };
                            let toast_line = text.first_line_height(message, role, toast_layout.content.size.width);
                            let banner_line = text.first_line_height(message, role, banner_message_width);
                            let marker = |diameter| scene.primitives().iter().find_map(|p| match p {
                                Primitive::Quad(q) if q.rect.size == Size::new(diameter, diameter) => Some(q.rect), _ => None
                            }).unwrap();
                            let toast_marker = marker(10.); let banner_marker = marker(12.);
                            assert!((toast_marker.center().y - toast_layout.content.min_y() - toast_line / 2.).abs() < 0.01);
                            assert!((banner_marker.center().y - banner_frame.min_y() - 6. - banner_line / 2.).abs() < 0.01);
                            assert!(scene.primitives().iter().any(|p| matches!(p, Primitive::Glyphs(run) if !run.glyphs.is_empty())));
                            let image_width = ((width + 32.) * scale).ceil() as u32;
                            let image_height = ((banner_frame.max_y() + 16.) * scale).ceil() as u32;
                            let rgba = renderer.render_to_rgba8(&scene, text.atlas_mut(), (image_width, image_height), scale,
                                crate::gallery::Gallery::background(theme)).expect("actual notice GPU rendering");
                            let name = format!("notices-theme-{theme_index}-{width}-scale-{scale}-text-{type_scale}-message-{message_index}.png");
                            if let Some(p) = &output {
                                let f = std::fs::File::create(p.join(&name)).unwrap();
                                let mut encoder = png::Encoder::new(f, image_width, image_height);
                                encoder.set_color(png::ColorType::Rgba); encoder.set_depth(png::BitDepth::Eight);
                                encoder.write_header().unwrap().write_image_data(&rgba).unwrap();
                            }
                            records.push(serde_json::json!({"image":name,"theme":theme_index,"width":width,"backing_scale":scale,
                                "type_scale":type_scale,"message":message,"toast_first_line_height":toast_line,"banner_first_line_height":banner_line,
                                "toast_marker_center_y":toast_marker.center().y,"banner_marker_center_y":banner_marker.center().y}));
                        }
                    }
                }
            }
        }
        assert_eq!(records.len(), 64);
        if let Some(p) = output {
            std::fs::write(p.join("gpu-receipt.json"), serde_json::to_vec_pretty(&serde_json::json!({
                "scope":"Production ToastView/BannerView with real Gallery CoreText adapter and GPU; not native or VoiceOver acceptance",
                "frames":records,"performance":"not measured"})).unwrap()).unwrap();
        }
    }

    fn luminance(color: tekes_ui_tokens::Rgba) -> f32 {
        let linear = |channel: f32| {
            if channel <= 0.04045 {
                channel / 12.92
            } else {
                ((channel + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * linear(color.r) + 0.7152 * linear(color.g) + 0.0722 * linear(color.b)
    }

    fn contrast_ratio(a: tekes_ui_tokens::Rgba, b: tekes_ui_tokens::Rgba) -> f32 {
        let (lighter, darker) = if luminance(a) >= luminance(b) {
            (luminance(a), luminance(b))
        } else {
            (luminance(b), luminance(a))
        };
        (lighter + 0.05) / (darker + 0.05)
    }

    #[test]
    fn status_badge_glyph_ink_contrasts_with_all_status_fills_across_themes_and_brands() {
        use tekes_ui_controls::extended::StatusBadge;
        use tekes_ui_scene::Primitive;
        use tekes_ui_tokens::{Palette, Theme};

        let cases = [
            (StatusBadge::Info, "Info", "status-info"),
            (StatusBadge::Running, "Running", "status-running"),
            (StatusBadge::Succeeded, "Succeeded", "status-succeeded"),
            (StatusBadge::Failed, "Failed", "status-failed"),
        ];
        let frame = Rect::new(0., 0., 112., 30.);
        for theme in Theme::ALL {
            for brand in 0..=2 {
                let palette = Palette::new(theme, brand).expect("registered Gallery brand");
                for (status, label, fill_role) in cases {
                    let mut scene = Scene::new();
                    let mut badge = StatusBadgeView::default();
                    let mut text = TextEngine::new();
                    badge.render_labelled_aligned(&mut scene, &mut text, status, label,
                        frame, frame.inset(5.), None, palette, tekes_ui_controls::ButtonTextAlign::Leading).unwrap();

                    let fill = scene
                        .primitives()
                        .iter()
                        .find_map(|primitive| match primitive {
                            Primitive::Quad(quad) if quad.rect == frame => Some(quad.fill),
                            _ => None,
                        })
                        .expect("production status badge fill quad");
                    let glyph_ink = scene
                        .primitives()
                        .iter()
                        .find_map(|primitive| match primitive {
                            Primitive::Glyphs(run) if !run.glyphs.is_empty() => Some(run.color),
                            _ => None,
                        })
                        .expect("actual shaped badge-label glyph run");
                    assert_eq!(glyph_ink, StatusBadgeView::label_color(palette));
                    assert_eq!(fill, palette.color(fill_role).unwrap());
                    assert!(
                        contrast_ratio(glyph_ink, fill) >= 4.5,
                        "{theme:?} brand={brand} {status:?}: {:?} on {:?} = {}",
                        glyph_ink,
                        fill,
                        contrast_ratio(glyph_ink, fill)
                    );
                }
            }
        }
    }

    #[test]
    fn toast_and_progress_fixture_controls_follow_measured_large_text() {
        let mut text = TextEngine::new();
        text.set_type_scale(1.5);
        let mut static_text = StaticText::new(None);
        let mut ctx = context(&mut text, &mut static_text);
        let toast_message = "A toast message wrapping in a narrow preview viewport.";
        let toast_size =
            ToastView::measure(ctx.text, toast_message, 180.0, proposal::TextRole::UiBody);
        assert!(toast_size.height > 62.0);
        let toast_model = Toast::new(toast_message, controls::NoticeTone::Info, 5000);
        let toast_frame = Rect::new(0.0, 0.0, 180.0, toast_size.height);
        let mut toast_view = ToastView::default();
        let mut scene = Scene::new();
        let toast_layout = toast_view.render(
            &mut scene,
            ctx.text,
            &toast_model,
            toast_frame,
            Some(toast_frame),
            Palette::from(ctx.theme),
            proposal::TextRole::UiBody,
        );
        let dismiss_target = toast_view.focus_target().expect("visible dismiss button");
        assert_eq!(
            toast_layout.content.size.height,
            ctx.text
                .measure(
                    toast_message,
                    proposal::TextRole::UiBody,
                    toast_layout.content.size.width,
                )
                .height
        );
        assert!(dismiss_target.max_y() <= toast_frame.max_y());
        assert!(toast_layout.content.max_y() <= toast_frame.max_y());
        let toast_bottom = 12.0 + toast_size.height;
        let (action, timer, note) = toast_fixture_layout(
            &mut ctx,
            0.0,
            0.0,
            180.0,
            toast_bottom,
            "12000 ms remaining",
            "Time follows the Host frame clock; hover pauses the original Toast.",
            0,
        );
        assert!(action.min_y() >= toast_bottom + 8.0);
        assert!(timer.min_y() >= action.max_y() + 8.0);
        assert!(note.min_y() >= timer.max_y() + 8.0);
        for rect in [action, timer, note] {
            assert!(rect.min_x() >= 0.0 && rect.max_x() <= 180.0, "{rect:?}");
            assert!(rect.size.height > 0.0, "{rect:?}");
        }

        let (button, percentage) = progress_fixture_layout(&mut ctx, 0.0, 0.0, 180.0, "70%", 0);
        assert!(percentage.min_y() >= button.max_y() + 8.0);
        assert!(percentage.max_x() <= 180.0);
        assert!(
            percentage.size.height
                >= ctx
                    .text
                    .measure("70%", proposal::TextRole::UiBody, percentage.size.width)
                    .height
        );
    }

    #[test]
    fn narrow_large_text_feedback_stories_report_their_measured_document_bottom() {
        let renderers = [
            "tooltip",
            "popover",
            "banner",
            "status-badge",
            "progress",
            "empty-state",
        ];
        for renderer in renderers {
            let entry = crate::gallery::workbench::entries()
                .iter()
                .find(|entry| entry.renderer() == renderer)
                .expect("registered feedback story");
            let mut ext = ExtendedStory::default();
            ext.tooltip.visible = renderer == "tooltip";
            if renderer == "popover" {
                ext.popover.show(20);
            }
            let mut state = crate::gallery::workbench::StoryState::for_entry(entry);
            let mut text = TextEngine::new();
            text.set_type_scale(1.5);
            let mut static_text = StaticText::new(None);
            let mut ctx = context(&mut text, &mut static_text);
            let mut scene = Scene::new();
            let body = Rect::new(24.0, 100.0, 220.0, 32.0);
            let clip = Rect::new(0.0, 0.0, 720.0, 640.0);
            let palette = Palette::from(ctx.theme);
            assert!(build(
                &mut ext, entry, &mut state, &mut ctx, &mut scene, body, clip, palette,
            ));
            assert!(
                state.document_bottom > body.max_y(),
                "{renderer} must report its actual content below the first clipped viewport: {} <= {}",
                state.document_bottom,
                body.max_y()
            );
            assert!(
                scene.primitives().iter().any(|primitive| matches!(
                    primitive,
                    tekes_ui_scene::Primitive::Glyphs(run) if !run.glyphs.is_empty()
                )),
                "{renderer} should draw actual shaped text"
            );
            if renderer == "tooltip" {
                let anchor = state.buttons[&20].layout().unwrap().frame;
                let measured = ctx.text.measure_button(
                    "Hover or focus me",
                    anchor.size.width,
                    tekes_ui_controls::ButtonSize::Body,
                    proposal::TextRole::UiBody,
                );
                assert!(anchor.size.height >= measured.height);
            }
            if renderer == "popover" {
                let layout = ext.feedback_views.popover.layout();
                assert!(layout.requested_size.height > body.size.height);
                assert!(layout.frame.size.height <= body.size.height);
                assert!(layout.frame.max_y() <= body.max_y());
            }
        }
    }

    #[test]
    fn status_badge_grid_reports_the_actual_wrapped_row_count() {
        let mut text = TextEngine::new();
        text.set_type_scale(1.5);
        let mut static_text = StaticText::new(None);
        let mut ctx = context(&mut text, &mut static_text);
        for (width, expected_columns, expected_rows) in
            [(500.0, 4, 1), (300.0, 2, 2), (180.0, 1, 4)]
        {
            let (columns, _, _, rows) = status_badge_grid(&mut ctx, width);
            assert_eq!(columns, expected_columns);
            assert_eq!(rows, expected_rows);
        }
    }

    #[test]
    fn bounded_fixture_width_keeps_previous_non_finite_viewport_semantics() {
        assert_eq!(bounded_fixture_width(260.0, 420.0), 260.0);
        assert_eq!(bounded_fixture_width(-4.0, 420.0), 0.0);
        assert_eq!(bounded_fixture_width(f32::INFINITY, 420.0), 420.0);
        assert_eq!(bounded_fixture_width(f32::NEG_INFINITY, 420.0), 0.0);
        assert_eq!(bounded_fixture_width(f32::NAN, 420.0), 420.0);
    }

    #[test]
    fn default_size_fixture_rows_keep_their_existing_positions() {
        let mut text = TextEngine::new();
        let mut static_text = StaticText::new(None);
        let mut ctx = context(&mut text, &mut static_text);
        let toast_size =
            ToastView::measure(ctx.text, "Changes saved", 520.0, proposal::TextRole::UiBody);
        assert_eq!(toast_size.height, 62.0);
        let (action, timer, note) = toast_fixture_layout(
            &mut ctx,
            40.0,
            0.0,
            600.0,
            74.0,
            "5000 ms remaining",
            "Time follows the Host frame clock; hover pauses the original Toast.",
            0,
        );
        assert_eq!(action, Rect::new(40.0, 84.0, 156.0, 34.0));
        assert_eq!(timer.min_x(), 206.0);
        assert_eq!(timer.min_y(), 84.0);
        assert_eq!(note.min_y(), 126.0);
        let (button, percentage) = progress_fixture_layout(&mut ctx, 40.0, 0.0, 600.0, "70%", 0);
        assert_eq!(button, Rect::new(40.0, 54.0, 118.0, 34.0));
        assert_eq!(percentage.min_x(), 172.0);
        assert_eq!(percentage.min_y(), 54.0);
        assert_eq!(percentage.size.height, 34.0);
    }

    #[test]
    fn feedback_state_and_variant_specs_use_real_views_without_registering_focus_or_hits() {
        let mut renderer = tekes_ui_render::Renderer::new_headless()
            .expect("headless GPU is required for feedback visual fixtures");
        let cases = [
            ("tooltip", true, false),
            ("popover", true, false),
            ("toast", true, true),
            ("banner", true, true),
            ("status-badge", false, true),
            ("progress", false, true),
            ("empty-state", true, true),
        ];
        for (renderer_name, has_states, has_variants) in cases {
            assert_eq!(
                supports_states(renderer_name),
                has_states,
                "{renderer_name} states"
            );
            assert_eq!(
                supports_variants(renderer_name),
                has_variants,
                "{renderer_name} variants"
            );
            let entry = crate::gallery::workbench::entries()
                .iter()
                .find(|entry| entry.renderer() == renderer_name)
                .expect("registered feedback story");
            if has_states {
                let mut state = StoryState::default();
                state.hits.push((777, Rect::new(1., 2., 3., 4.)));
                let mut text = TextEngine::new();
                text.set_scale(1.0);
                text.set_type_scale(1.5);
                text.begin_frame();
                assert_eq!(text.backing_scale(), 1.0);
                let mut static_text = StaticText::new(None);
                let mut ctx = context(&mut text, &mut static_text);
                let viewport = Rect::new(0.0, 0.0, 360.0, 200.0);
                let mut scene = Scene::new();
                matrix(entry, &mut state, &mut ctx, &mut scene, viewport);
                assert!(
                    state.document_bottom > viewport.max_y(),
                    "{renderer_name} states scroll"
                );
                assert_eq!(state.hits, vec![(777, Rect::new(1., 2., 3., 4.))]);
                assert!(
                    state.buttons.is_empty(),
                    "visual specimens are not tab stops"
                );
                assert!(!scene.is_empty());
                let full_viewport = Rect::new(0.0, 0.0, 360.0, 5_000.0);
                let mut full_scene = Scene::new();
                matrix(entry, &mut state, &mut ctx, &mut full_scene, full_viewport);
                let image_height = state.document_bottom.ceil().max(1.0) as u32;
                let rgba = renderer
                    .render_to_rgba8(
                        &full_scene,
                        ctx.text.atlas_mut(),
                        (360, image_height),
                        1.0,
                        crate::gallery::Gallery::background(ctx.theme),
                    )
                    .expect(
                        "GPU render full feedback state specimens at scale 1 and text scale 1.5",
                    );
                save_feedback_image(&rgba, 360, image_height, renderer_name, "states");
            }
            if has_variants {
                let mut state = StoryState::default();
                state.hits.push((777, Rect::new(1., 2., 3., 4.)));
                let mut text = TextEngine::new();
                text.set_scale(1.0);
                text.set_type_scale(1.5);
                text.begin_frame();
                let mut static_text = StaticText::new(None);
                let mut ctx = context(&mut text, &mut static_text);
                let viewport = Rect::new(0.0, 0.0, 360.0, 200.0);
                let mut scene = Scene::new();
                variants(entry, &mut state, &mut ctx, &mut scene, viewport);
                assert!(
                    state.document_bottom.is_finite() && state.document_bottom > viewport.min_y(),
                    "{renderer_name} variants report their actual document bottom"
                );
                assert_eq!(state.hits, vec![(777, Rect::new(1., 2., 3., 4.))]);
                assert!(
                    state.buttons.is_empty(),
                    "visual specimens are not tab stops"
                );
                assert!(!scene.is_empty());
                let full_viewport = Rect::new(0.0, 0.0, 360.0, 5_000.0);
                let mut full_scene = Scene::new();
                variants(entry, &mut state, &mut ctx, &mut full_scene, full_viewport);
                let image_height = state.document_bottom.ceil().max(1.0) as u32;
                let rgba = renderer
                    .render_to_rgba8(
                        &full_scene,
                        ctx.text.atlas_mut(),
                        (360, image_height),
                        1.0,
                        crate::gallery::Gallery::background(ctx.theme),
                    )
                    .expect(
                        "GPU render full feedback variant specimens at scale 1 and text scale 1.5",
                    );
                save_feedback_image(&rgba, 360, image_height, renderer_name, "variants");
            }
        }
    }

    fn save_feedback_image(rgba: &[u8], width: u32, height: u32, renderer: &str, story: &str) {
        let Some(output) = std::env::var_os("TEKESUI_GOLD7_PAGE_OUT") else {
            return;
        };
        let output = std::path::PathBuf::from(output);
        std::fs::create_dir_all(&output).unwrap();
        let file = std::fs::File::create(
            output.join(format!("feedback-{renderer}-{story}-360pt-text150.png")),
        )
        .unwrap();
        let mut encoder = png::Encoder::new(file, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .unwrap()
            .write_image_data(rgba)
            .unwrap();
    }
    #[test]
    fn empty_state_panel_callers_forward_host_backing_scale() {
        let mut gpu = std::env::var_os("TEKESUI_PANEL_SCALE_OUT")
            .map(|_| tekes_ui_render::Renderer::new_headless().unwrap());
        let mut wrong = 0;
        for (ti, theme) in tekes_ui_tokens::Theme::ALL.into_iter().enumerate() {
            for brand in 0..3 {
                for scale in [1., 2.] {
                    for variants in [false, true] {
                        let mut text = TextEngine::new();
                        text.set_scale(scale);
                        text.begin_frame();
                        let mut static_text = StaticText::new(None);
                        let mut ctx = PageCtx {
                            theme,
                            text: &mut text,
                            static_text: &mut static_text,
                        };
                        let mut scene = Scene::new();
                        let mut y = 8.;
                        let bounds = Rect::new(8., 8., 700., 2000.);
                        let palette = Palette::new(theme, brand).expect("registered Gallery brand");
                        if variants {
                            matrix_empty_variants(&mut ctx, &mut scene, bounds, &mut y, palette);
                        } else {
                            matrix_empty_state(&mut ctx, &mut scene, bounds, &mut y, palette);
                        }
                        let name = format!("empty-v{variants}-t{ti}-b{brand}-s{scale}");
                        wrong += crate::gallery::quality_effects::record_panel_host_scale(
                            &scene,
                            &mut text,
                            palette,
                            &name,
                            gpu.as_mut(),
                        );
                    }
                }
            }
        }
        assert_eq!(
            wrong, 0,
            "all EmptyState panel callers must forward existing backing scale"
        );
    }
    #[test]
    fn real_tooltip_state_matrix_consumes_theme_palette_roles() {
        use tekes_ui_scene::Primitive;
        use tekes_ui_tokens::{Palette, Theme};

        for theme in Theme::ALL {
            let mut text = TextEngine::new();
            text.begin_frame();
            let mut static_text = StaticText::new(None);
            let mut ctx = context(&mut text, &mut static_text);
            ctx.theme = theme;
            let mut scene = Scene::new();
            let mut y = 8.;
            let bounds = Rect::new(8., 8., 360., 1600.);
            let palette = Palette::from(theme);
            matrix_tooltip(&mut ctx, &mut scene, bounds, &mut y, palette);
            let tooltip_frames: Vec<_> = scene
                .primitives()
                .iter()
                .filter_map(|primitive| match primitive {
                    Primitive::Quad(quad)
                        if quad.rect.size.width == bounded_fixture_width(bounds.size.width, 420.).min(240.)
                            && quad.corner_radii.top_left
                                == proposal::metric("radius", "radius-floating").unwrap()
                            && quad.fill
                            == palette
                                .color("surface-3")
                                .expect("generated Tooltip surface") =>
                    {
                        Some(quad.rect)
                    }
                    _ => None,
                })
                .collect();
            assert_eq!(
                tooltip_frames.len(),
                2,
                "hover and focus-visible states in {theme:?}"
            );
            let tooltip_text = tooltip_frames.iter().all(|frame| {
                let content = Rect::new(
                    frame.min_x() + 12.,
                    frame.min_y() + 8.,
                    frame.size.width - 24.,
                    frame.size.height - 16.,
                );
                scene.primitives().iter().any(|primitive| {
                    let Primitive::Glyphs(run) = primitive else {
                        return false;
                    };
                    run.color == palette.color("primary-text").expect("semantic text role")
                        && !run.glyphs.is_empty()
                        && run.glyphs.iter().all(|glyph| content.contains(glyph.origin))
                })
            });
            assert!(
                tooltip_text,
                "real Gallery Tooltip body glyphs consume {theme:?} text palette"
            );
            assert!(
                contrast_ratio(
                    palette.color("primary-text").unwrap(),
                    palette.color("surface-3").unwrap(),
                ) >= if theme.is_increased() { 7. } else { 4.5 },
                "Tooltip text/background contrast in {theme:?}"
            );
            assert!(
                y > bounds.min_y(),
                "the real Tooltip matrix lays out in {theme:?}"
            );
        }
    }
    #[test]
    fn actual_tooltip_forwarder_recovers_each_new_owner_without_reviving_old_requests() {
        use tekes_ui_tokens::{Palette, Theme};
        fn paint(v: &mut FeedbackViews, text: &mut TextEngine, palette: Palette) -> bool {
            let mut scene = Scene::new();
            let layout = v.render_tooltip(
                &mut scene,
                text,
                &Tooltip {
                    visible: false,
                    text: "Help · 操作说明".into(),
                },
                Rect::new(20., 20., 160., 40.),
                Size::new(240., 64.),
                Rect::new(0., 0., 400., 240.),
                None,
                palette,
                proposal::TextRole::UiBody,
            );
            assert_eq!(scene.is_empty(), layout.visible.is_none());
            layout.visible.is_some()
        }
        for theme in Theme::ALL {
            for brand in 0..3 {
                for focus in [false, true] {
                    let palette = Palette::new(theme, brand).expect("registered Gallery brand");
                    let mut views = FeedbackViews::default();
                    let mut text = TextEngine::new();
                    paint(&mut views, &mut text, palette);
                    views.tooltip_pointer_move(Point::new(50., 40.));
                    views.tooltip_set_focus(true, true);
                    assert!(paint(&mut views, &mut text, palette));
                    assert_eq!(
                        views.tooltip_key(ControlKey::Escape),
                        TooltipEvent::Dismissed
                    );
                    views.tooltip_set_focus(true, true);
                    views.tooltip_pointer_move(Point::new(50., 40.));
                    assert!(!paint(&mut views, &mut text, palette));
                    if focus {
                        views.tooltip_set_focus(false, false);
                    } else {
                        views.tooltip_pointer_move(Point::new(-10., -10.));
                    }
                    assert!(!paint(&mut views, &mut text, palette));
                    if focus {
                        views.tooltip_set_focus(true, true);
                    } else {
                        views.tooltip_pointer_move(Point::new(50., 40.));
                    }
                    assert!(paint(&mut views, &mut text, palette));
                    views.tooltip_set_window_active(false);
                    views.tooltip_set_focus(true, true);
                    views.tooltip_pointer_move(Point::new(50., 40.));
                    views.tooltip_set_window_active(true);
                    assert!(!paint(&mut views, &mut text, palette));
                }
            }
        }
    }
    #[test]
    fn actual_tooltip_forwarder_observes_new_controlled_request_with_retained_owners() {
        for theme in tekes_ui_tokens::Theme::ALL {
            for brand in 0..3 {
                let mut views = FeedbackViews::default();
                let mut text = TextEngine::new();
                let mut model = Tooltip {
                    visible: true,
                    text: "Controlled help · 说明".into(),
                };
                let mut paint = |v: &mut FeedbackViews, m: &Tooltip| {
                    v.render_tooltip(
                        &mut Scene::new(),
                        &mut text,
                        m,
                        Rect::new(20., 20., 160., 40.),
                        Size::new(240., 64.),
                        Rect::new(0., 0., 400., 240.),
                        None,
                        tekes_ui_tokens::Palette::new(theme, brand).expect("registered Gallery brand"),
                        proposal::TextRole::UiBody,
                    )
                    .visible
                    .is_some()
                };
                assert!(paint(&mut views, &model));
                views.tooltip_set_focus(true, true);
                views.tooltip_pointer_move(Point::new(50., 40.));
                assert_eq!(
                    views.tooltip_key(ControlKey::Escape),
                    TooltipEvent::Dismissed
                );
                assert!(!paint(&mut views, &model));
                model.visible = false;
                assert!(!paint(&mut views, &model));
                model.visible = true;
                assert!(paint(&mut views, &model));
            }
        }
    }
    #[test]
    fn progress_original_page_and_variants_preserve_selected_brand_packet() {
        use tekes_ui_scene::Primitive;
        use tekes_ui_tokens::surface_palette::resolve_surface;
        let entry = crate::gallery::workbench::entries()
            .iter()
            .find(|e| e.renderer() == "progress")
            .unwrap();
        for theme in tekes_ui_tokens::Theme::ALL {
            for brand in [0, 1, 2] {
                for (value, unknown) in [(0., false), (0.45, false), (1., false), (0.45, true)] {
                    let mut model = Progress::new(value);
                    model.indeterminate = unknown;
                    let mut ext = ExtendedStory {
                        progress: model,
                        ..Default::default()
                    };
                    let mut state = StoryState::for_entry(entry);
                    state.brand = brand;
                    let mut text = TextEngine::new();
                    let mut static_text = StaticText::new(None);
                    let mut ctx = context(&mut text, &mut static_text);
                    ctx.theme = theme;
                    let palette = Palette::new(theme, brand).unwrap();
                    let packet = resolve_surface(palette, "canvas", "progress-track", "rest").unwrap();
                    let mut scene = Scene::new();
                    let body = Rect::new(24., 100., 220., 32.);
                    let clip = Rect::new(0., 0., 720., 640.);
                    assert!(build(
                        &mut ext, entry, &mut state, &mut ctx, &mut scene, body, clip, palette
                    ));
                    let frame = ext.feedback_views.progress.layout().frame;
                    let q = scene
                        .primitives()
                        .iter()
                        .filter_map(|p| {
                            if let Primitive::Quad(q) = p {
                                Some(q)
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>();
                    assert!(
                        q.iter()
                            .any(|q| q.rect == frame && q.fill == packet.background)
                    );
                    assert!(q.iter().any(|q| q.rect == frame
                        && q.border_color == packet.boundary.unwrap()
                        && q.border_width > 0.));
                    assert!(q.iter().any(|q| q.fill == packet.marker.unwrap()
                        && q.rect.min_y() == frame.min_y()
                        && q.rect.size.width == frame.size.width * if unknown { 0.3 } else { value }));
                    assert!(state.buttons.contains_key(&20));
                    let mut scene = Scene::new();
                    let mut y = 10.;
                    matrix_progress_variants(
                        &mut ctx,
                        &mut scene,
                        Rect::new(10., 10., 220., 500.),
                        &mut y,
                        palette,
                    );
                    let fills = scene
                        .primitives()
                        .iter()
                        .filter_map(|p| match p {
                            Primitive::Quad(q) if q.fill == packet.marker.unwrap() => {
                                Some(q.rect.size.width)
                            }
                            _ => None,
                        })
                        .collect::<Vec<_>>();
                    assert_eq!(fills, vec![0., 220. * 0.45, 220., 220. * 0.3]);
                }
            }
        }
    }

    #[test]
    #[ignore = "explicit GPU acceptance under shared lock with fresh SSD output"]
    fn issue76_original_gallery_page_and_variants_actual_gpu_match_badge_view() {
        use tekes_ui_scene::Primitive;
        use tekes_ui_tokens::{Theme, surface_palette::resolve_surface};
        let out=std::env::var_os("TEKESUI_BADGE_GALLERY_OUT").map(std::path::PathBuf::from);
        if let Some(p)=&out { assert!(!p.exists());std::fs::create_dir_all(p).unwrap(); }
        let entry=crate::gallery::workbench::entries().iter().find(|e|e.renderer()=="status-badge").unwrap();
        let mut gpu=tekes_ui_render::Renderer::new_headless().expect("actual GPU required");let mut receipt=vec![];
        for (mi,mode) in Theme::ALL.into_iter().enumerate(){for brand in 0..3{for backing in [1.,2.]{for width in [180.,320.,500.]{for variants_page in [false,true]{
            let palette=Palette::new(mode,brand).unwrap();let mut state=StoryState::for_entry(entry);state.brand=brand;
            let mut ext=ExtendedStory::default();let mut text=TextEngine::new();text.set_scale(backing);text.set_type_scale(1.5);text.begin_frame();let mut static_text=StaticText::new(None);
            let mut ctx=PageCtx{theme:mode,text:&mut text,static_text:&mut static_text};let mut scene=Scene::new();
            let body=Rect::new(12.25,20.5,width,600.);let clip=Rect::new(0.,0.,width+30.,640.);
            if variants_page{variants(entry,&mut state,&mut ctx,&mut scene,body);}else{assert!(build(&mut ext,entry,&mut state,&mut ctx,&mut scene,body,clip,palette));}
            let size=(((width+30.)*backing).ceil()as u32,(640.*backing)as u32);let ground=crate::gallery::Gallery::background(mode);
            let actual=gpu.render_to_rgba8(&scene,ctx.text.atlas_mut(),size,backing,ground).unwrap();
            let mut oracle=Scene::new();let mut badge_frames=vec![];
            for (tone,usage,label)in [(StatusBadge::Info,"status-badge-info","Info"),(StatusBadge::Running,"status-badge-running","Running"),(StatusBadge::Succeeded,"status-badge-succeeded","Succeeded"),(StatusBadge::Failed,"status-badge-failed","Failed")]{
                let packet=resolve_surface(palette,"canvas",usage,"rest").unwrap();let frame=scene.primitives().iter().find_map(|v|match v{Primitive::Quad(q)if q.fill==packet.background&&q.corner_radii==tekes_ui_scene::CornerRadii::uniform(q.rect.size.height*0.5)=>Some(q.rect),_=>None}).expect("actual original Gallery solid badge");
                StatusBadgeView::default().render_labelled_aligned(&mut oracle,ctx.text,tone,label,frame,frame.inset(5.),Some(clip),palette,tekes_ui_controls::ButtonTextAlign::Leading).unwrap();badge_frames.push(frame);
            }
            let expected=gpu.render_to_rgba8(&oracle,ctx.text.atlas_mut(),size,backing,ground).unwrap();let blank=gpu.render_to_rgba8(&Scene::new(),ctx.text.atlas_mut(),size,backing,ground).unwrap();
            let mut checked=0;let mut positive=0;
            for (i,(a,b))in actual.chunks_exact(4).zip(expected.chunks_exact(4)).enumerate(){let x=((i as u32%size.0)as f32+0.5)/backing;let y=((i as u32/size.0)as f32+0.5)/backing;
                if badge_frames.iter().any(|f|x>f.min_x()+1.&&x<f.max_x()-1.&&y>f.min_y()+1.&&y<f.max_y()-1.) {assert_eq!(a,b,"original Gallery/labelled View m{mi} b{brand} width{width} variants{variants_page} x{x} y{y}");checked+=1;if a!=&blank[i*4..i*4+4]{positive+=1;}}
            }
            assert!(checked>0&&positive>0);let key=format!("m{mi}-brand{brand}-w{width}-variants{variants_page}@{backing}x");
            if let Some(p)=&out && width==320. && brand==0 {let file=std::fs::File::create(p.join(format!("{key}.png"))).unwrap();let mut encoder=png::Encoder::new(file,size.0,size.1);encoder.set_color(png::ColorType::Rgba);encoder.set_depth(png::BitDepth::Eight);encoder.write_header().unwrap().write_image_data(&actual).unwrap();}
            receipt.push(serde_json::json!({"case":key,"original_Gallery_build":!variants_page,"original_Gallery_variants":variants_page,"four_original_badge_frames":badge_frames.len(),"checked_pixels":checked,"positive_pixels":positive}));
        }}}}}
        println!("actual original Gallery page/variants GPU cases: {}",receipt.len());
        if let Some(p)=out{std::fs::write(p.join("receipt.json"),serde_json::to_vec_pretty(&serde_json::json!({"cases":receipt,"GUI_opened":false})).unwrap()).unwrap();}
    }
    #[test]
    #[ignore = "explicit actual GPU capture with fresh SSD output"]
    fn issue79_original_gallery_mounted_notices_actual_gpu_and_clock() {
        use std::time::Duration;
        let output=Some(std::path::PathBuf::from(std::env::var_os("TEKESUI_EMPTY_STATE_OUT").expect("fresh SSD output required")));
        let path=output.as_ref().unwrap();assert!(path.starts_with("/Volumes/HP P900/DEV/TekesUI/artifacts"));std::fs::create_dir(path).expect("fresh output required");
        let mut gpu=tekes_ui_render::Renderer::new_headless().expect("actual GPU required");let mut cases=vec![];
        for name in ["toast","banner"]{let entry=crate::gallery::workbench::entries().iter().find(|e|e.renderer()==name).unwrap();
         for (mi,theme) in tekes_ui_tokens::Theme::ALL.into_iter().enumerate(){for brand in 0..3{for backing in [1.,2.]{for width in [180.,320.,500.]{
          let palette=Palette::new(theme,brand).unwrap();let mut ext=ExtendedStory::default();let mut state=StoryState::for_entry(entry);state.brand=brand;
          let mut text=TextEngine::new();text.set_motion_enabled(true);text.set_motion_context(Duration::ZERO,true);text.set_scale(backing);text.set_type_scale(1.5);text.begin_frame();let mut static_text=StaticText::new(None);
          let mut ctx=PageCtx{theme,text:&mut text,static_text:&mut static_text};let mut scene=Scene::new();let body=Rect::new(12.25,20.5,width,600.);let clip=Rect::new(0.,0.,width+30.,640.);
          assert!(build(&mut ext,entry,&mut state,&mut ctx,&mut scene,body,clip,palette));
          let (notice,action)=if name=="toast"{assert!(ext.feedback_views.toast.host.is_some());(ext.feedback_views.toast.layout().frame,ext.feedback_views.toast.focus_target().unwrap())}else{assert!(ext.feedback_views.banner.host.is_some());(ext.feedback_views.banner.layout().frame,ext.feedback_views.banner.focus_target().unwrap())};
          let size=(((width+30.)*backing).ceil() as u32,(640.*backing)as u32);let ground=crate::gallery::Gallery::background(theme);
          let rgba=gpu.render_to_rgba8(&scene,ctx.text.atlas_mut(),size,backing,ground).unwrap();let blank=gpu.render_to_rgba8(&Scene::new(),ctx.text.atlas_mut(),size,backing,ground).unwrap();
          let mut notice_ink=0;let mut action_ink=0;for (i,(a,b))in rgba.chunks_exact(4).zip(blank.chunks_exact(4)).enumerate(){if a!=b{let p=Point::new(((i as u32%size.0)as f32+0.5)/backing,((i as u32/size.0)as f32+0.5)/backing);if notice.contains(p){notice_ink+=1;}
if action.contains(p){action_ink+=1;}}}
          assert!(notice_ink>0&&action_ink>0,"actual Gallery {name} notice/action ink");
          if name=="toast"{let before=ext.toast.remaining_ms();ctx.text.set_motion_context(Duration::from_millis(100),true);ctx.text.begin_frame();assert!(build(&mut ext,entry,&mut state,&mut ctx,&mut Scene::new(),body,clip,palette));assert_eq!(ext.toast.remaining_ms(),before-100);}
          let key=format!("{name}-m{mi}-b{brand}-w{width}@{backing}");
          if let Some(out)=&output && width==320.&&brand==0{let file=std::fs::File::create(out.join(format!("{key}.png"))).unwrap();let mut encoder=png::Encoder::new(file,size.0,size.1);encoder.set_color(png::ColorType::Rgba);encoder.set_depth(png::BitDepth::Eight);encoder.write_header().unwrap().write_image_data(&rgba).unwrap();}
          cases.push(serde_json::json!({"case":key,"notice_ink":notice_ink,"action_ink":action_ink,"actual_mounted_host":true,"automatic_clock":name=="toast"}));
         }}}}
        }
        println!("Issue79 actual original Gallery GPU cases: {}",cases.len());if let Some(out)=output{std::fs::write(out.join("notice-receipt.json"),serde_json::to_vec_pretty(&serde_json::json!({"cases":cases,"GUI":false})).unwrap()).unwrap();}
    }

    #[test]
    #[ignore = "explicit actual GPU capture with fresh SSD output"]
    fn issue79_static_matrix_and_variants_use_actual_content_without_a_clock() {
        let output=std::path::PathBuf::from(std::env::var_os("TEKESUI_NOTICE_STATIC_OUT").expect("fresh SSD output required"));
        assert!(output.starts_with("/Volumes/HP P900/DEV/TekesUI/artifacts"));std::fs::create_dir(&output).expect("fresh output required");
        let mut gpu=tekes_ui_render::Renderer::new_headless().expect("actual GPU required");let mut count=0;
        for name in ["toast","banner"]{let entry=crate::gallery::workbench::entries().iter().find(|e|e.renderer()==name).unwrap();
         for theme in tekes_ui_tokens::Theme::ALL{for backing in [1.,2.]{for width in [180.,320.,500.]{for variant in [false,true]{
          let mut state=StoryState::for_entry(entry);let mut text=TextEngine::new();text.set_motion_enabled(false);text.set_scale(backing);text.set_type_scale(1.5);text.begin_frame();let mut static_text=StaticText::new(None);let mut ctx=PageCtx{theme,text:&mut text,static_text:&mut static_text};let mut scene=Scene::new();let frame=Rect::new(10.25,10.5,width,900.);
          if variant{variants(entry,&mut state,&mut ctx,&mut scene,frame);}else{matrix(entry,&mut state,&mut ctx,&mut scene,frame);}
          let size=(((width+30.)*backing).ceil()as u32,(940.*backing)as u32);let ground=crate::gallery::Gallery::background(theme);let image=gpu.render_to_rgba8(&scene,ctx.text.atlas_mut(),size,backing,ground).unwrap();let blank=gpu.render_to_rgba8(&Scene::new(),ctx.text.atlas_mut(),size,backing,ground).unwrap();assert_ne!(image,blank);count+=1;
         }}}}
        }println!("actual static Gallery matrix/variant GPU cases: {count}");
        std::fs::write(output.join("receipt.json"),format!("{{\"cases\":{count}}}")).unwrap();
    }

}
