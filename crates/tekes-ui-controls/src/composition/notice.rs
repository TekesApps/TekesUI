//! Original notice owners and fixed action proxies; no independent countdown or input engine.
use super::action_proxy::Exposure;
use super::*;
use crate::{
    extended::{Banner, NoticeTone, Toast},
    feedback_view::{BannerView, ToastView, ViewLayout},
};
use std::{
    cell::{Ref, RefCell},
    rc::Rc,
    time::Duration,
};

#[derive(Debug)]
struct ToastOwner {
    model: Toast,
    view: ToastView,
    window_active: bool,
    runnable: bool,
    dismissed_action: bool,
    exposure: Exposure,
    last: Option<Duration>,
    fraction: Duration,
    #[cfg(test)]
    hover_calls: usize,
}
impl ToastOwner {
    fn available(&self) -> bool {
        self.window_active
            && self.runnable
            && self.model.visible
            && !self.exposure.pending()
            && self.view.focus_target().is_some()
    }
    fn revoke(&mut self) {
        self.dismissed_action = false;
        self.exposure.revoke();
        self.view.cancel();
        self.view.invalidate_geometry();
        self.clear_sample();
    }
    fn clear_sample(&mut self) {
        self.last = None;
        self.fraction = Duration::ZERO;
    }
}
/// Passive message Group containing the original View and one fixed Dismiss proxy.
#[derive(Debug)]
pub struct ToastContent {
    name: String,
    expected_action_id: ComponentId,
    owner: Rc<RefCell<ToastOwner>>,
    children: Vec<Content>,
}
impl ToastContent {
    pub fn new(
        name: impl Into<String>,
        model: Toast,
        dismiss_id: ComponentId,
    ) -> Result<Self, CompositionError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(CompositionError::EmptyAccessibleName);
        }
        let owner = Rc::new(RefCell::new(ToastOwner {
            model,
            view: ToastView::default(),
            window_active: true,
            runnable: false,
            dismissed_action: false,
            exposure: Exposure::default(),
            last: None,
            fraction: Duration::ZERO,
            #[cfg(test)]
            hover_calls: 0,
        }));
        Ok(Self {
            name,
            expected_action_id: dismiss_id,
            children: vec![Content::new(
                dismiss_id,
                ToastProxy {
                    owner: owner.clone(),
                },
            )],
            owner,
        })
    }
    pub fn model(&self) -> Ref<'_, Toast> {
        Ref::map(self.owner.borrow(), |o| &o.model)
    }
    pub fn layout(&self) -> ViewLayout {
        self.owner.borrow().view.layout()
    }
    pub fn set_metadata(&mut self, name: impl Into<String>) -> Result<(), CompositionError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(CompositionError::EmptyAccessibleName);
        }
        self.validate()?;
        self.name = name;
        Ok(())
    }
    fn intact(&self) -> bool {
        self.children.len() == 1
            && self.children.first().is_some_and(|c| {
                c.id == self.expected_action_id
                    && c.component::<ToastProxy>()
                        .is_some_and(|p| Rc::ptr_eq(&p.owner, &self.owner))
            })
    }
}
impl Component for ToastContent {
    fn interactive(&self) -> bool {
        true
    }
    fn validate(&self) -> Result<(), CompositionError> {
        if !self.intact() {
            return Err(CompositionError::InvalidSlotCount);
        }
        if self.name.trim().is_empty() {
            return Err(CompositionError::EmptyAccessibleName);
        }
        Ok(())
    }
    fn active(&self) -> bool {
        self.intact() && self.owner.borrow().model.visible
    }
    fn fixed_slots(&self) -> bool {
        true
    }
    fn paints_children(&self) -> bool {
        true
    }
    fn children(&self) -> &[Content] {
        &self.children
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.children)
    }
    fn measure(&mut self, text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        if !self.intact() {
            return MeasuredChild::new(Size::default());
        }
        let o = self.owner.borrow();
        MeasuredChild::new(ToastView::measure(
            &mut FrameTextAdapter(text, None, true),
            &o.model.message,
            available.width,
            proposal::TextRole::UiBody,
        ))
    }
    fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        if !self.intact() {
            return;
        }
        let action = {
            let mut o = self.owner.borrow_mut();
            let ToastOwner { model, view, .. } = &mut *o;
            view.prepare(
                &mut FrameTextAdapter(text, None, true),
                model,
                frame,
                Some(clip),
                proposal::TextRole::UiBody,
            );
            view.logical_focus_target().unwrap_or_default()
        };
        self.children[0].arrange(text, action, clip);
    }
    fn paint(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        _frame: Rect,
        _clip: Rect,
        theme: ContentTheme,
    ) {
        if !self.intact() {
            return;
        }
        let mut o = self.owner.borrow_mut();
        let frames_allowed = o.window_active && o.runnable && !o.model.paused;
        let ToastOwner { model, view, .. } = &mut *o;
        view.set_surface(theme.surface())
            .expect("canonical inherited Surface");
        view.paint_prepared(
            scene,
            &mut FrameTextAdapter(text, None, frames_allowed),
            model,
            theme.palette,
        );
    }
    fn semantics(&self) -> ContentSemantics {
        let mut s = ContentSemantics::new(ContentRole::Group, &self.name);
        s.description = Some(self.owner.borrow().model.message.clone());
        s
    }
    fn window_active_changed(&mut self, active: bool) {
        let mut o = self.owner.borrow_mut();
        if !active {
            o.revoke();
        }
        o.window_active = active;
        o.view.set_window_active(active);
    }
    fn presentation_runnable_changed(&mut self, runnable: bool) {
        let mut o = self.owner.borrow_mut();
        if !runnable {
            if o.runnable && o.model.visible {
                o.revoke();
            }
            o.clear_sample();
        }
        o.runnable = runnable;
    }
    fn lifecycle(&mut self, event: Lifecycle) {
        if event == Lifecycle::Unmounted {
            let mut o = self.owner.borrow_mut();
            o.runnable = false;
            o.revoke();
        }
    }
    fn event(&mut self, event: &ContentEvent) -> ContentResponse {
        if let ContentEvent::PointerMove(p) = event {
            if !self.intact() {
                return ContentResponse::default();
            }
            let mut o = self.owner.borrow_mut();
            let before = o.model.paused;
            #[cfg(test)]
            {
                o.hover_calls += 1;
            }
            let ToastOwner { model, view, .. } = &mut *o;
            view.pointer_move(model, *p);
            if o.model.paused || before != o.model.paused {
                o.clear_sample();
            }
            return ContentResponse {
                consumed: true,
                changed: true,
                ..Default::default()
            };
        }
        ContentResponse::default()
    }
}
#[derive(Debug)]
struct ToastProxy {
    owner: Rc<RefCell<ToastOwner>>,
}
impl Component for ToastProxy {
    fn active(&self) -> bool {
        self.owner.borrow().available()
    }
    fn focusable(&self) -> bool {
        self.active()
    }
    fn measure(&mut self, _text: &mut dyn ButtonTextRenderer, _available: Size) -> MeasuredChild {
        MeasuredChild::new(Size::default())
    }
    fn set_focus(&mut self, focused: bool, visible: bool) {
        let mut o = self.owner.borrow_mut();
        let focused = focused && o.available();
        o.view.focus_dismiss(focused, visible);
    }
    fn semantic_frame(&self, _frame: Rect) -> Rect {
        self.owner
            .borrow()
            .view
            .logical_focus_target()
            .unwrap_or_default()
    }
    fn semantics(&self) -> ContentSemantics {
        let mut s = ContentSemantics::new(ContentRole::Button, "Dismiss");
        s.disabled = !self.active();
        s
    }
    fn bubble_handled_event(&self, event: &ContentEvent) -> bool {
        matches!(event.routing_event().as_ref(), ContentEvent::PointerMove(_))
    }
    fn event(&mut self, event: &ContentEvent) -> ContentResponse {
        let mut o = self.owner.borrow_mut();
        if matches!(event, ContentEvent::Cancel) {
            o.view.cancel();
            return Default::default();
        }
        if !o.available() {
            return Default::default();
        }
        let mut r = ContentResponse::default();
        match event {
            ContentEvent::PointerMove(p) => {
                o.view.pointer_move_button(*p);
                r.changed = true;
            }
            ContentEvent::PointerDown(p) => {
                r.consumed = o.view.pointer_down(*p);
                r.capture_pointer = r.consumed;
                r.changed = r.consumed;
            }
            ContentEvent::PointerUp(p) => {
                let ToastOwner { model, view, .. } = &mut *o;
                r.activated = view.pointer_up(model, *p);
                r.consumed = true;
                r.changed = true;
            }
            ContentEvent::Key { key, down } => {
                let ToastOwner { model, view, .. } = &mut *o;
                r.activated = view.key(model, *key, *down);
                r.consumed = matches!(key, ControlKey::Space | ControlKey::Enter);
                r.changed = r.consumed;
            }
            _ => {}
        }
        if r.activated {
            o.revoke();
            o.dismissed_action = true;
        }
        r
    }
    fn accessibility_action(
        &mut self,
        part: Option<usize>,
        action: &AccessibilityAction,
    ) -> ContentResponse {
        let mut o = self.owner.borrow_mut();
        if part.is_some() || !matches!(action, AccessibilityAction::Press) || !o.available() {
            return Default::default();
        }
        let ToastOwner { model, view, .. } = &mut *o;
        let activated = view.activate_dismiss(model);
        if activated {
            o.revoke();
            o.dismissed_action = true;
        }
        ContentResponse {
            consumed: activated,
            changed: activated,
            activated,
            ..Default::default()
        }
    }
}

#[derive(Debug)]
struct BannerOwner {
    model: Banner,
    view: BannerView,
    window_active: bool,
    runnable: bool,
    dismissed_action: bool,
    exposure: Exposure,
}
impl BannerOwner {
    fn available(&self) -> bool {
        self.window_active
            && self.runnable
            && self.model.visible
            && self.model.dismissible
            && !self.exposure.pending()
            && self.view.focus_target().is_some()
    }
    fn revoke(&mut self) {
        self.dismissed_action = false;
        self.exposure.revoke();
        self.view.cancel();
        self.view.invalidate_geometry();
    }
}
/// Passive message Group containing the original View and one fixed Dismiss proxy.
#[derive(Debug)]
pub struct BannerContent {
    name: String,
    expected_action_id: ComponentId,
    owner: Rc<RefCell<BannerOwner>>,
    children: Vec<Content>,
}
impl BannerContent {
    pub fn new(
        name: impl Into<String>,
        model: Banner,
        dismiss_id: ComponentId,
    ) -> Result<Self, CompositionError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(CompositionError::EmptyAccessibleName);
        }
        let owner = Rc::new(RefCell::new(BannerOwner {
            model,
            view: BannerView::default(),
            window_active: true,
            runnable: false,
            dismissed_action: false,
            exposure: Exposure::default(),
        }));
        Ok(Self {
            name,
            expected_action_id: dismiss_id,
            children: vec![Content::new(
                dismiss_id,
                BannerProxy {
                    owner: owner.clone(),
                },
            )],
            owner,
        })
    }
    pub fn model(&self) -> Ref<'_, Banner> {
        Ref::map(self.owner.borrow(), |o| &o.model)
    }
    pub fn layout(&self) -> ViewLayout {
        self.owner.borrow().view.layout()
    }
    pub fn set_metadata(&mut self, name: impl Into<String>) -> Result<(), CompositionError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(CompositionError::EmptyAccessibleName);
        }
        self.validate()?;
        self.name = name;
        Ok(())
    }
    fn intact(&self) -> bool {
        self.children.len() == 1
            && self.children.first().is_some_and(|c| {
                c.id == self.expected_action_id
                    && c.component::<BannerProxy>()
                        .is_some_and(|p| Rc::ptr_eq(&p.owner, &self.owner))
            })
    }
}
impl Component for BannerContent {
    fn validate(&self) -> Result<(), CompositionError> {
        if !self.intact() {
            return Err(CompositionError::InvalidSlotCount);
        }
        if self.name.trim().is_empty() {
            return Err(CompositionError::EmptyAccessibleName);
        }
        Ok(())
    }
    fn active(&self) -> bool {
        self.intact() && self.owner.borrow().model.visible
    }
    fn fixed_slots(&self) -> bool {
        true
    }
    fn paints_children(&self) -> bool {
        true
    }
    fn children(&self) -> &[Content] {
        &self.children
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.children)
    }
    fn measure(&mut self, text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        if !self.intact() {
            return MeasuredChild::new(Size::default());
        }
        let o = self.owner.borrow();
        MeasuredChild::new(BannerView::measure(
            &mut FrameTextAdapter(text, None, true),
            &o.model.message,
            available.width,
            o.model.dismissible,
            proposal::TextRole::UiBody,
        ))
    }
    fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        if !self.intact() {
            return;
        }
        let action = {
            let mut o = self.owner.borrow_mut();
            let BannerOwner { model, view, .. } = &mut *o;
            view.prepare(
                &mut FrameTextAdapter(text, None, true),
                model,
                frame,
                Some(clip),
                proposal::TextRole::UiBody,
            );
            view.logical_focus_target().unwrap_or_default()
        };
        self.children[0].arrange(text, action, clip);
    }
    fn paint(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        _frame: Rect,
        _clip: Rect,
        theme: ContentTheme,
    ) {
        if !self.intact() {
            return;
        }
        let mut o = self.owner.borrow_mut();
        let BannerOwner { model, view, .. } = &mut *o;
        view.set_surface(theme.surface())
            .expect("canonical inherited Surface");
        view.paint_prepared(
            scene,
            &mut FrameTextAdapter(text, None, true),
            model,
            theme.palette,
        );
    }
    fn semantics(&self) -> ContentSemantics {
        let mut s = ContentSemantics::new(ContentRole::Group, &self.name);
        s.description = Some(self.owner.borrow().model.message.clone());
        s
    }
    fn window_active_changed(&mut self, active: bool) {
        let mut o = self.owner.borrow_mut();
        if !active {
            o.revoke();
        }
        o.window_active = active;
        o.view.set_window_active(active);
    }
    fn presentation_runnable_changed(&mut self, runnable: bool) {
        let mut o = self.owner.borrow_mut();
        if !runnable && o.runnable && o.model.visible {
            o.revoke();
        }
        o.runnable = runnable;
    }
    fn lifecycle(&mut self, event: Lifecycle) {
        if event == Lifecycle::Unmounted {
            let mut o = self.owner.borrow_mut();
            o.runnable = false;
            o.revoke();
        }
    }
}
#[derive(Debug)]
struct BannerProxy {
    owner: Rc<RefCell<BannerOwner>>,
}
impl Component for BannerProxy {
    fn active(&self) -> bool {
        self.owner.borrow().available()
    }
    fn focusable(&self) -> bool {
        self.active()
    }
    fn measure(&mut self, _text: &mut dyn ButtonTextRenderer, _available: Size) -> MeasuredChild {
        MeasuredChild::new(Size::default())
    }
    fn set_focus(&mut self, focused: bool, visible: bool) {
        let mut o = self.owner.borrow_mut();
        let focused = focused && o.available();
        o.view.focus_dismiss(focused, visible);
    }
    fn semantic_frame(&self, _frame: Rect) -> Rect {
        self.owner
            .borrow()
            .view
            .logical_focus_target()
            .unwrap_or_default()
    }
    fn semantics(&self) -> ContentSemantics {
        let mut s = ContentSemantics::new(ContentRole::Button, "Dismiss");
        s.disabled = !self.active();
        s
    }
    fn bubble_handled_event(&self, event: &ContentEvent) -> bool {
        matches!(event.routing_event().as_ref(), ContentEvent::PointerMove(_))
    }
    fn event(&mut self, event: &ContentEvent) -> ContentResponse {
        let mut o = self.owner.borrow_mut();
        if matches!(event, ContentEvent::Cancel) {
            o.view.cancel();
            return Default::default();
        }
        if !o.available() {
            return Default::default();
        }
        let mut r = ContentResponse::default();
        match event {
            ContentEvent::PointerMove(p) => {
                o.view.pointer_move_button(*p);
                r.changed = true;
            }
            ContentEvent::PointerDown(p) => {
                r.consumed = o.view.pointer_down(*p);
                r.capture_pointer = r.consumed;
                r.changed = r.consumed;
            }
            ContentEvent::PointerUp(p) => {
                let BannerOwner { model, view, .. } = &mut *o;
                r.activated = view.pointer_up(model, *p);
                r.consumed = true;
                r.changed = true;
            }
            ContentEvent::Key { key, down } => {
                let BannerOwner { model, view, .. } = &mut *o;
                r.activated = view.key(model, *key, *down);
                r.consumed = matches!(key, ControlKey::Space | ControlKey::Enter);
                r.changed = r.consumed;
            }
            _ => {}
        }
        if r.activated {
            o.revoke();
            o.dismissed_action = true;
        }
        r
    }
    fn accessibility_action(
        &mut self,
        part: Option<usize>,
        action: &AccessibilityAction,
    ) -> ContentResponse {
        let mut o = self.owner.borrow_mut();
        if part.is_some() || !matches!(action, AccessibilityAction::Press) || !o.available() {
            return Default::default();
        }
        let BannerOwner { model, view, .. } = &mut *o;
        let activated = view.activate_dismiss(model);
        if activated {
            o.revoke();
            o.dismissed_action = true;
        }
        ContentResponse {
            consumed: activated,
            changed: activated,
            activated,
            ..Default::default()
        }
    }
}

fn visit(node: &Content, f: &mut impl FnMut(&Content)) {
    f(node);
    for c in node.component.children() {
        visit(c, f);
    }
}
fn visit_mut(node: &mut Content, f: &mut impl FnMut(&mut Content)) {
    f(node);
    if let Some(children) = node.component.children_mut() {
        for c in children {
            visit_mut(c, f);
        }
    }
}
pub(super) fn clear_samples(node: &mut Content) {
    visit_mut(node, &mut |n| {
        if let Some(t) = n.component_mut::<ToastContent>() {
            t.owner.borrow_mut().clear_sample();
        }
    });
}
pub(super) fn authorized_action(node: &Content, target: ComponentId) -> bool {
    fn check(n: &Content, target: ComponentId, ancestors: bool) -> Option<bool> {
        let active = ancestors && n.component.active();
        if let Some(t) = n.component::<ToastContent>()
            && t.expected_action_id == target
        {
            let o = t.owner.borrow();
            return Some(
                ancestors
                    && t.intact()
                    && o.window_active
                    && (o.dismissed_action
                        || active && o.runnable && !o.exposure.pending() && o.model.visible),
            );
        }
        if let Some(t) = n.component::<BannerContent>()
            && t.expected_action_id == target
        {
            let o = t.owner.borrow();
            return Some(
                ancestors
                    && t.intact()
                    && o.window_active
                    && (o.dismissed_action
                        || active
                            && o.runnable
                            && !o.exposure.pending()
                            && o.model.visible
                            && o.model.dismissible),
            );
        }
        n.component
            .children()
            .iter()
            .find_map(|c| check(c, target, active))
    }
    check(node, target, true).unwrap_or(true)
}
impl ContentHost {
    pub(super) fn preflight_notice_structures(&self) -> Result<(), CompositionError> {
        let mut result = Ok(());
        visit(&self.root, &mut |n| {
            if let Some(t) = n.component::<ToastContent>() {
                result = result.and(t.validate());
            }
            if let Some(t) = n.component::<BannerContent>() {
                result = result.and(t.validate());
            }
        });
        result
    }
    pub(super) fn publish_notice_exposures(&mut self) -> Result<(), CompositionError> {
        let mut mounts = Vec::new();
        let mut result = Ok(());
        visit(&self.root, &mut |n| {
            if let Some(t) = n.component::<ToastContent>() {
                let o = t.owner.borrow();
                if o.runnable
                    && o.window_active
                    && o.model.visible
                    && o.view.focus_target().is_some()
                    && o.exposure.pending()
                {
                    match o.exposure.reserve() {
                        Ok(m) => mounts.push((n.id, m)),
                        Err(e) => result = Err(e),
                    }
                }
            }
            if let Some(t) = n.component::<BannerContent>() {
                let o = t.owner.borrow();
                if o.runnable
                    && o.window_active
                    && o.model.visible
                    && o.model.dismissible
                    && o.view.focus_target().is_some()
                    && o.exposure.pending()
                {
                    match o.exposure.reserve() {
                        Ok(m) => mounts.push((n.id, m)),
                        Err(e) => result = Err(e),
                    }
                }
            }
        });
        result?;
        for (id, mount) in mounts {
            let n = self.root.find_mut(id).unwrap();
            if let Some(t) = n.component_mut::<ToastContent>() {
                if let Some(m) = mount {
                    t.children[0].mount = m;
                }
                t.owner.borrow_mut().exposure.published();
            }
            if let Some(t) = n.component_mut::<BannerContent>() {
                if let Some(m) = mount {
                    t.children[0].mount = m;
                }
                t.owner.borrow_mut().exposure.published();
            }
        }
        Ok(())
    }
    // One frame-local sample, all owners validated read-only before any advance.
    pub(super) fn advance_notices(
        &mut self,
        text: &mut dyn ButtonTextRenderer,
    ) -> Result<(), CompositionError> {
        let now = text.motion_context().map(|m| m.now);
        let mut error = None;
        visit(&self.root, &mut |n| {
            if let Some(t) = n.component::<ToastContent>() {
                let o = t.owner.borrow();
                if o.runnable && o.window_active && o.model.visible && !o.model.paused {
                    if now.is_none() {
                        error = Some(CompositionError::MissingNoticeClock(n.id));
                    } else if o.last.is_some_and(|last| now.unwrap() < last) {
                        error = Some(CompositionError::NoticeClockWentBackwards(n.id));
                    }
                }
            }
        });
        if let Some(e) = error {
            clear_samples(&mut self.root);
            self.dirty = false;
            return Err(e);
        }
        let mut request = false;
        visit_mut(&mut self.root, &mut |n| {
            if let Some(t) = n.component_mut::<ToastContent>() {
                let mut o = t.owner.borrow_mut();
                if !o.runnable || !o.window_active || !o.model.visible || o.model.paused {
                    o.clear_sample();
                    return;
                }
                let now = now.expect("all runnable notice clocks validated");
                let elapsed = o
                    .last
                    .map_or(Duration::ZERO, |last| now - last)
                    .saturating_add(o.fraction);
                let ms = elapsed.as_millis().min(u32::MAX as u128) as u32;
                o.last = Some(now);
                o.fraction = if elapsed.as_millis() > u32::MAX as u128 {
                    Duration::ZERO
                } else {
                    elapsed - Duration::from_millis(ms as u64)
                };
                let ToastOwner { model, view, .. } = &mut *o;
                view.advance(model, ms);
                if !o.model.visible {
                    o.revoke();
                } else if o.model.remaining_ms() > 0 {
                    request = true;
                }
            }
        });
        if request {
            text.request_animation_frame();
        }
        Ok(())
    }
    fn update_toast(
        &mut self,
        id: ComponentId,
        model: Toast,
        force: bool,
    ) -> Result<(), CompositionError> {
        self.root.ids(&mut BTreeSet::new())?;
        let t = self
            .root
            .find(id)
            .ok_or(CompositionError::MissingId(id))?
            .component::<ToastContent>()
            .ok_or(CompositionError::InvalidNotice(id))?;
        if !force && *t.model() == model {
            return Ok(());
        }
        let mount = actions::try_next_mount()?;
        let action_id = t.expected_action_id;
        self.cancel_subtree(action_id);
        let t = self
            .root
            .find_mut(id)
            .unwrap()
            .component_mut::<ToastContent>()
            .unwrap();
        {
            let mut o = t.owner.borrow_mut();
            o.model = model;
            o.revoke();
            o.exposure.replaced();
        }
        t.children[0].mount = mount;
        self.dirty = true;
        Ok(())
    }
    fn update_banner(
        &mut self,
        id: ComponentId,
        model: Banner,
        force: bool,
    ) -> Result<(), CompositionError> {
        self.root.ids(&mut BTreeSet::new())?;
        let t = self
            .root
            .find(id)
            .ok_or(CompositionError::MissingId(id))?
            .component::<BannerContent>()
            .ok_or(CompositionError::InvalidNotice(id))?;
        if !force && *t.model() == model {
            return Ok(());
        }
        let mount = actions::try_next_mount()?;
        let action_id = t.expected_action_id;
        self.cancel_subtree(action_id);
        let t = self
            .root
            .find_mut(id)
            .unwrap()
            .component_mut::<BannerContent>()
            .unwrap();
        {
            let mut o = t.owner.borrow_mut();
            o.model = model;
            o.revoke();
            o.exposure.replaced();
        }
        t.children[0].mount = mount;
        self.dirty = true;
        Ok(())
    }
    pub fn show_toast(
        &mut self,
        id: ComponentId,
        message: impl Into<String>,
        tone: NoticeTone,
        duration_ms: u32,
    ) -> Result<(), CompositionError> {
        self.update_toast(id, Toast::new(message, tone, duration_ms), true)
    }
    pub fn set_toast_visible(
        &mut self,
        id: ComponentId,
        visible: bool,
    ) -> Result<(), CompositionError> {
        let mut model = self
            .root
            .find(id)
            .ok_or(CompositionError::MissingId(id))?
            .component::<ToastContent>()
            .ok_or(CompositionError::InvalidNotice(id))?
            .model()
            .clone();
        model.visible = visible;
        self.update_toast(id, model, false)
    }
    pub fn set_toast_paused(
        &mut self,
        id: ComponentId,
        paused: bool,
    ) -> Result<(), CompositionError> {
        self.root.ids(&mut BTreeSet::new())?;
        let t = self
            .root
            .find_mut(id)
            .ok_or(CompositionError::MissingId(id))?
            .component_mut::<ToastContent>()
            .ok_or(CompositionError::InvalidNotice(id))?;
        let mut o = t.owner.borrow_mut();
        if o.model.paused == paused {
            return Ok(());
        }
        o.model.paused = paused;
        o.clear_sample();
        self.dirty = true;
        Ok(())
    }
    pub fn dismiss_toast(&mut self, id: ComponentId) -> Result<(), CompositionError> {
        let mut model = self
            .root
            .find(id)
            .ok_or(CompositionError::MissingId(id))?
            .component::<ToastContent>()
            .ok_or(CompositionError::InvalidNotice(id))?
            .model()
            .clone();
        model.dismiss();
        self.update_toast(id, model, false)
    }
    pub fn replace_banner(
        &mut self,
        id: ComponentId,
        model: Banner,
    ) -> Result<(), CompositionError> {
        self.update_banner(id, model, true)
    }
}

pub(super) fn hide_subtree(node: &mut Content) {
    visit_mut(node, &mut |n| {
        if let Some(t) = n.component_mut::<ToastContent>() {
            t.owner.borrow_mut().revoke();
        }
        if let Some(t) = n.component_mut::<BannerContent>() {
            t.owner.borrow_mut().revoke();
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Default)]
    struct Text;
    impl ButtonTextRenderer for Text {
        fn motion_context(&self) -> Option<ControlMotionContext> {
            Some(ControlMotionContext {
                now: Duration::ZERO,
                reduced_motion: true,
            })
        }
        fn measure(&mut self, _: &str, _: proposal::TextRole, w: f32) -> Size {
            Size::new(w.min(80.), 18.)
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
    const T: ComponentId = ComponentId(790);
    const A: ComponentId = ComponentId(791);
    fn host() -> ContentHost {
        let mut h = ContentHost::new(Content::new(
            T,
            ToastContent::new("Toast", Toast::new("Saved", NoticeTone::Info, 1000), A).unwrap(),
        ))
        .unwrap();
        paint(&mut h).unwrap();
        h
    }
    fn paint(h: &mut ContentHost) -> Result<(), CompositionError> {
        let f = Rect::new(0., 0., 360., 200.);
        h.render(
            &mut Scene::new(),
            &mut Text,
            f,
            f,
            tekes_ui_tokens::Theme::LIGHT.into(),
        )
    }
    fn calls(h: &ContentHost) -> usize {
        h.get(T)
            .unwrap()
            .component::<ToastContent>()
            .unwrap()
            .owner
            .borrow()
            .hover_calls
    }
    #[test]
    fn original_hover_once_ordinary_exit_and_captured_route_from() {
        let mut h = host();
        let p = h.get(A).unwrap().frame().center();
        let outside = Point::new(800., 800.);
        h.event(&ContentEvent::PointerMove(p));
        assert_eq!(calls(&h), 1);
        h.event(&ContentEvent::PointerMove(outside));
        assert_eq!(calls(&h), 2);
        h.event(&ContentEvent::PointerDown(p));
        assert_eq!(calls(&h), 2);
        assert_eq!(h.pointer_capture().map(|v| v.0), Some(A));
        h.event(&ContentEvent::PointerMove(p));
        assert_eq!(calls(&h), 3);
        h.event(&ContentEvent::PointerMove(outside));
        assert_eq!(calls(&h), 4);
        let pointer = PointerEvent {
            position: p,
            phase: PointerPhase::Move,
            button: None,
            modifiers: Default::default(),
            click_count: 0,
        };
        h.event(&ContentEvent::Pointer(pointer));
        assert_eq!(calls(&h), 5);
    }
    #[test]
    fn allocation_failure_before_cancel_and_recovery_pending() {
        let mut h = host();
        let p = h.get(A).unwrap().frame().center();
        let old = h.semantic_mount(A);
        h.event(&ContentEvent::PointerDown(p));
        let capture = h.pointer_capture();
        actions::FAIL_MOUNT.with(|f| f.set(true));
        let result = h.show_toast(T, "New", NoticeTone::Warning, 5000);
        actions::FAIL_MOUNT.with(|f| f.set(false));
        assert_eq!(result, Err(CompositionError::MountExhausted));
        assert_eq!(capture, h.pointer_capture());
        assert_eq!(old, h.semantic_mount(A));
        assert_eq!(
            h.get(T)
                .unwrap()
                .component::<ToastContent>()
                .unwrap()
                .model()
                .message,
            "Saved"
        );
        h.set_window_active(false);
        h.set_window_active(true);
        actions::FAIL_MOUNT.with(|f| f.set(true));
        let result = paint(&mut h);
        actions::FAIL_MOUNT.with(|f| f.set(false));
        assert_eq!(result, Err(CompositionError::MountExhausted));
        assert!(h.semantic_mount(A).is_none());
        assert!(!h.focus(Some(A), true));
        paint(&mut h).unwrap();
        assert_ne!(h.semantic_mount(A), old);
    }
    #[derive(Debug)]
    struct RetainedExit {
        active: bool,
        children: Vec<Content>,
        paints: usize,
    }
    impl Component for RetainedExit {
        fn active(&self) -> bool {
            self.active
        }
        fn paints_children(&self) -> bool {
            true
        }
        fn overlay_presented(&self) -> bool {
            true
        }
        fn children(&self) -> &[Content] {
            &self.children
        }
        fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
            Some(&mut self.children)
        }
        fn measure(&mut self, _: &mut dyn ButtonTextRenderer, size: Size) -> MeasuredChild {
            MeasuredChild::new(size)
        }
        fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
            self.children[0].measure(text, frame.size);
            self.children[0].arrange(text, frame, clip);
        }
        fn paint_overlay(
            &mut self,
            scene: &mut Scene,
            text: &mut dyn ButtonTextRenderer,
            _: Rect,
            _: Rect,
            theme: ContentTheme,
        ) {
            self.paints += 1;
            self.children[0].paint(scene, text, theme);
        }
    }
    #[test]
    fn retained_exit_paint_does_not_extend_clock_or_actions() {
        let mut h = ContentHost::new(Content::new(
            ComponentId(799),
            RetainedExit {
                active: true,
                paints: 0,
                children: vec![Content::new(
                    T,
                    ToastContent::new("Toast", Toast::new("Saved", NoticeTone::Info, 1000), A)
                        .unwrap(),
                )],
            },
        ))
        .unwrap();
        paint(&mut h).unwrap();
        h.root.component_mut::<RetainedExit>().unwrap().active = false;
        paint(&mut h).unwrap();
        assert_eq!(h.root.component::<RetainedExit>().unwrap().paints, 2);
        assert!(!h.animation_frame_requested());
        assert!(h.semantic_mount(A).is_none());
        let t = h.get(T).unwrap().component::<ToastContent>().unwrap();
        assert!(t.owner.borrow().last.is_none());
        assert_eq!(t.model().remaining_ms(), 1000);
    }
}
