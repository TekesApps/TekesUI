//! Passive EmptyState message with one fixed proxy for the original View action.
use super::action_proxy::Exposure;
use super::*;
use crate::{
    extended::EmptyState,
    feedback_view::{EmptyStateView, ViewLayout},
};
use std::{
    cell::{Ref, RefCell},
    rc::Rc,
};
#[derive(Debug)]
struct Owner {
    model: EmptyState,
    view: EmptyStateView,
    disabled: bool,
    window_active: bool,
    exposure: Exposure,
}
impl Owner {
    fn available(&self) -> bool {
        self.window_active
            && !self.disabled
            && !self.exposure.pending()
            && self.model.action.is_some()
            && self.view.focus_target().is_some()
    }
    fn revoke(&mut self) {
        self.exposure.revoke();
        self.view.cancel_action();
        self.view.invalidate_geometry();
    }
}
/// One original EmptyState/EmptyStateView/ButtonModel. Mutating action semantics
/// is deliberately restricted to the finite typed ContentHost entry points.
#[derive(Debug)]
pub struct EmptyStateContent {
    name: String,
    description: Option<String>,
    expected_action_id: ComponentId,
    owner: Rc<RefCell<Owner>>,
    children: Vec<Content>,
}
impl EmptyStateContent {
    pub fn new(
        name: impl Into<String>,
        model: EmptyState,
        action_id: ComponentId,
    ) -> Result<Self, CompositionError> {
        let name = name.into();
        validate_metadata(&name, None)?;
        validate_model(&model)?;
        let owner = Rc::new(RefCell::new(Owner {
            model,
            view: EmptyStateView::default(),
            disabled: false,
            window_active: true,
            exposure: Exposure::default(),
        }));
        Ok(Self {
            name,
            description: None,
            expected_action_id: action_id,
            children: vec![Content::new(
                action_id,
                ActionProxy {
                    owner: owner.clone(),
                },
            )],
            owner,
        })
    }
    /// Read-only guard, never held across Host delivery, layout or callbacks.
    pub fn model(&self) -> Ref<'_, EmptyState> {
        Ref::map(self.owner.borrow(), |o| &o.model)
    }
    pub fn layout(&self) -> ViewLayout {
        self.owner.borrow().view.layout()
    }
    pub fn disabled(&self) -> bool {
        self.owner.borrow().disabled
    }
    pub fn set_metadata(
        &mut self,
        name: impl Into<String>,
        description: Option<String>,
    ) -> Result<(), CompositionError> {
        let name = name.into();
        validate_metadata(&name, description.as_deref())?;
        self.validate()?;
        self.name = name;
        self.description = description;
        Ok(())
    }
    fn intact(&self) -> bool {
        self.children.len() == 1
            && self.children.first().is_some_and(|c| {
                c.id == self.expected_action_id
                    && c.component::<ActionProxy>()
                        .is_some_and(|proxy| Rc::ptr_eq(&proxy.owner, &self.owner))
            })
    }
}
fn validate_metadata(name: &str, description: Option<&str>) -> Result<(), CompositionError> {
    if name.trim().is_empty() || description.is_some_and(|v| v.trim().is_empty()) {
        return Err(CompositionError::EmptyAccessibleName);
    }
    Ok(())
}
fn validate_model(model: &EmptyState) -> Result<(), CompositionError> {
    if model
        .action
        .as_ref()
        .is_some_and(|label| label.trim().is_empty())
    {
        return Err(CompositionError::EmptyAccessibleName);
    }
    Ok(())
}
impl Component for EmptyStateContent {
    fn validate(&self) -> Result<(), CompositionError> {
        if !self.intact() {
            return Err(CompositionError::InvalidSlotCount);
        }
        validate_metadata(&self.name, self.description.as_deref())?;
        validate_model(&self.owner.borrow().model)
    }
    fn active(&self) -> bool {
        self.intact()
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
        MeasuredChild::new(EmptyStateView::measure(
            &mut TextAdapter(text),
            &self.owner.borrow().model,
            available.width,
            proposal::TextRole::UiBody,
        ))
    }
    fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        if !self.intact() {
            return;
        }
        let action_frame = {
            let mut o = self.owner.borrow_mut();
            let previous = o.view.focus_target().is_some();
            let Owner { model, view, .. } = &mut *o;
            view.prepare(
                &mut TextAdapter(text),
                model,
                frame,
                Some(clip),
                proposal::TextRole::UiBody,
            );
            let next = view.logical_focus_target().unwrap_or_default();
            if previous && view.focus_target().is_none() {
                o.exposure.revoke();
                o.view.cancel_action();
            }
            next
        }; // Never recurse into Content while holding the owner borrow.
        self.children[0].arrange(text, action_frame, clip);
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
        let Owner { model, view, .. } = &mut *o;
        view.set_surface(theme.surface())
            .expect("inherited canonical Surface");
        view.paint_prepared(
            scene,
            &mut TextAdapter(text),
            model,
            theme.palette,
        );
    }
    fn semantics(&self) -> ContentSemantics {
        let mut s = ContentSemantics::new(ContentRole::Group, &self.name);
        s.description = self.description.clone();
        s
    }
    fn window_active_changed(&mut self, active: bool) {
        let mut o = self.owner.borrow_mut();
        if o.window_active && !active {
            o.revoke();
        }
        o.window_active = active;
        o.view.set_window_active(active);
    }
    fn lifecycle(&mut self, event: Lifecycle) {
        if event == Lifecycle::Unmounted {
            self.owner.borrow_mut().revoke();
        }
    }
}
#[derive(Debug)]
struct ActionProxy {
    owner: Rc<RefCell<Owner>>,
}
impl Component for ActionProxy {
    fn measure(&mut self, _text: &mut dyn ButtonTextRenderer, _available: Size) -> MeasuredChild {
        MeasuredChild::new(Size::default())
    }
    fn active(&self) -> bool {
        self.owner.borrow().available()
    }
    fn focusable(&self) -> bool {
        self.active()
    }
    fn set_focus(&mut self, focused: bool, visible: bool) {
        let mut o = self.owner.borrow_mut();
        let focused = focused && o.available();
        o.view.focus_action(focused, visible);
    }
    fn semantic_frame(&self, _frame: Rect) -> Rect {
        self.owner
            .borrow()
            .view
            .logical_focus_target()
            .unwrap_or_default()
    }
    fn semantics(&self) -> ContentSemantics {
        let o = self.owner.borrow();
        let mut s =
            ContentSemantics::new(ContentRole::Button, o.model.action.as_deref().unwrap_or(""));
        s.disabled = !o.available();
        s
    }
    fn event(&mut self, event: &ContentEvent) -> ContentResponse {
        let mut o = self.owner.borrow_mut();
        if matches!(event, ContentEvent::Cancel) {
            o.view.cancel_action();
            return ContentResponse::default();
        }
        if !o.available() {
            return ContentResponse::default();
        }
        let mut r = ContentResponse::default();
        match event {
            ContentEvent::PointerMove(p) => {
                o.view.pointer_move(*p);
                r.changed = true;
            }
            ContentEvent::PointerDown(p) => {
                r.consumed = o.view.pointer_down(*p);
                r.capture_pointer = r.consumed;
                r.changed = r.consumed;
            }
            ContentEvent::PointerUp(p) => {
                r.activated = o.view.pointer_up(*p);
                r.consumed = true;
                r.changed = true;
            }
            ContentEvent::Key { key, down } => {
                r.activated = o.view.key(*key, *down);
                r.consumed = matches!(key, ControlKey::Space | ControlKey::Enter);
                r.changed = r.consumed;
            }
            _ => {}
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
            return ContentResponse::default();
        }
        let activated = o.view.activate_action();
        ContentResponse {
            consumed: activated,
            changed: activated,
            activated,
            ..Default::default()
        }
    }
}
/// Called only on the original transition into an inactive retained subtree.
pub(super) fn hide_subtree(node: &mut Content) {
    if let Some(empty) = node.component_mut::<EmptyStateContent>() {
        empty.owner.borrow_mut().revoke();
    }
    if let Some(children) = node.component.children_mut() {
        for child in children {
            hide_subtree(child);
        }
    }
}
// Specific proxy authorization for the original pending queue. Ordinary action
// owners retain their existing contract; body-only layout invalidation keeps
// the same authorization even before the next frame.
pub(super) fn authorized_action(node: &Content, target: ComponentId) -> bool {
    fn check(node: &Content, target: ComponentId, ancestors: bool) -> Option<bool> {
        let active = ancestors && node.component.active();
        if let Some(empty) = node.component::<EmptyStateContent>()
            && empty.expected_action_id == target
        {
            let o = empty.owner.borrow();
            return Some(
                active
                    && empty.intact()
                    && o.window_active
                    && !o.disabled
                    && !o.exposure.pending()
                    && o.model.action.is_some(),
            );
        }
        node.component
            .children()
            .iter()
            .find_map(|c| check(c, target, active))
    }
    check(node, target, true).unwrap_or(true)
}
impl ContentHost {
    // Adapter structure errors are preflight errors. Reject before the legacy
    // whole-tree error cancellation, preserving unrelated original editors.
    pub(super) fn preflight_empty_state_structures(&self) -> Result<(), CompositionError> {
        fn check(node: &Content) -> Result<(), CompositionError> {
            if let Some(empty) = node.component::<EmptyStateContent>() {
                empty.validate()?;
            }
            for child in node.component.children() {
                check(child)?;
            }
            Ok(())
        }
        check(&self.root)
    }
    pub fn replace_empty_state(
        &mut self,
        id: ComponentId,
        model: EmptyState,
    ) -> Result<(), CompositionError> {
        validate_model(&model)?;
        self.update_empty_state(id, Some(model), None)
    }
    pub fn set_empty_state_disabled(
        &mut self,
        id: ComponentId,
        disabled: bool,
    ) -> Result<(), CompositionError> {
        self.update_empty_state(id, None, Some(disabled))
    }
    fn update_empty_state(
        &mut self,
        id: ComponentId,
        model: Option<EmptyState>,
        disabled: Option<bool>,
    ) -> Result<(), CompositionError> {
        self.root.ids(&mut BTreeSet::new())?;
        let empty = self
            .root
            .find(id)
            .ok_or(CompositionError::MissingId(id))?
            .component::<EmptyStateContent>()
            .ok_or(CompositionError::InvalidEmptyState(id))?;
        let o = empty.owner.borrow();
        if model.as_ref().is_none_or(|m| *m == o.model) && disabled.is_none_or(|v| v == o.disabled)
        {
            return Ok(());
        }
        let renew = model.as_ref().is_some_and(|m| m.action != o.model.action)
            || disabled.is_some_and(|v| v != o.disabled);
        drop(o);
        // Preflight is complete. Allocation failure cannot cancel a live arm,
        // replace the model or mutate the existing mount/queue/focus.
        let mount = renew.then(actions::try_next_mount).transpose()?;
        let action_id = empty.expected_action_id;
        if renew {
            self.cancel_subtree(action_id);
        }
        let empty = self
            .root
            .find_mut(id)
            .unwrap()
            .component_mut::<EmptyStateContent>()
            .unwrap();
        {
            let mut o = empty.owner.borrow_mut();
            if let Some(model) = model {
                o.model = model;
            }
            if let Some(disabled) = disabled {
                o.disabled = disabled;
                o.view.set_disabled(disabled);
            }
            if renew {
                o.view.invalidate_geometry();
                o.exposure.replaced();
            } else {
                o.view.invalidate_body_geometry();
            }
        }
        if let Some(mount) = mount {
            empty.children[0].mount = mount;
        }
        self.dirty = true;
        Ok(())
    }
    // Layout is fallible already. Reserve every needed original mount before
    // committing any exposure; no inactive notification calls the allocator.
    pub(super) fn publish_empty_state_exposures(&mut self) -> Result<(), CompositionError> {
        fn reserve(
            node: &Content,
            out: &mut Vec<(ComponentId, Option<u64>)>,
        ) -> Result<(), CompositionError> {
            if !node.component.active() {
                return Ok(());
            }
            if let Some(empty) = node.component::<EmptyStateContent>() {
                let o = empty.owner.borrow();
                if o.window_active
                    && !o.disabled
                    && o.model.action.is_some()
                    && o.view.focus_target().is_some()
                    && o.exposure.pending()
                {
                    out.push((node.id, o.exposure.reserve()?));
                }
            }
            for child in node.component.children() {
                reserve(child, out)?;
            }
            Ok(())
        }
        let mut mounts = Vec::new();
        reserve(&self.root, &mut mounts)?;
        for (id, mount) in mounts {
            let empty = self
                .root
                .find_mut(id)
                .unwrap()
                .component_mut::<EmptyStateContent>()
                .unwrap();
            if let Some(mount) = mount {
                empty.children[0].mount = mount;
            }
            empty.owner.borrow_mut().exposure.published();
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const ROOT: ComponentId = ComponentId(780);
    const ACTION: ComponentId = ComponentId(781);
    struct Text;
    impl ButtonTextRenderer for Text {
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
    fn model(label: &str) -> EmptyState {
        EmptyState {
            title: "Empty".into(),
            detail: "Details".into(),
            action: Some(label.into()),
        }
    }
    fn host() -> ContentHost {
        let mut h = ContentHost::new(Content::new(
            ROOT,
            EmptyStateContent::new("Empty", model("Create"), ACTION).unwrap(),
        ))
        .unwrap();
        h.set_action_events(true);
        paint(&mut h).unwrap();
        h
    }
    fn paint(h: &mut ContentHost) -> Result<(), CompositionError> {
        let f = Rect::new(0., 0., 320., 220.);
        h.render(
            &mut Scene::new(),
            &mut Text,
            f,
            f,
            tekes_ui_tokens::Theme::LIGHT.into(),
        )
    }
    #[test]
    fn allocation_exhaustion_precedes_cancel_model_mount_and_queue_commit() {
        let mut h = host();
        let old = h.semantic_mount(ACTION);
        let frame = h.get(ACTION).unwrap().frame();
        assert!(
            h.event(&ContentEvent::PointerDown(frame.center()))
                .capture_pointer
        );
        let capture = h.captured;
        actions::FAIL_MOUNT.with(|f| f.set(true));
        let result = h.replace_empty_state(ROOT, model("Open"));
        actions::FAIL_MOUNT.with(|f| f.set(false));
        assert_eq!(result, Err(CompositionError::MountExhausted));
        assert_eq!(h.semantic_mount(ACTION), old);
        assert_eq!(h.captured, capture);
        assert_eq!(
            *h.get(ROOT)
                .unwrap()
                .component::<EmptyStateContent>()
                .unwrap()
                .model(),
            model("Create")
        );
        assert!(h.event(&ContentEvent::PointerUp(frame.center())).activated);
        assert_eq!(h.drain_actions().len(), 1);
    }
    #[test]
    fn inactive_notification_never_allocates_and_failed_recovery_remains_closed() {
        let mut h = host();
        let mount = h.get(ACTION).unwrap().mount;
        actions::FAIL_MOUNT.with(|f| f.set(true));
        h.set_window_active(false);
        h.set_window_active(false);
        h.set_window_active(true);
        assert_eq!(h.get(ACTION).unwrap().mount, mount);
        assert!(h.semantic_mount(ACTION).is_none());
        assert_eq!(paint(&mut h), Err(CompositionError::MountExhausted));
        assert!(h.semantic_mount(ACTION).is_none());
        assert!(!h.focus(Some(ACTION), true));
        assert!(
            !h.accessibility_action(ACTION, None, AccessibilityAction::Press)
                .activated
        );
        actions::FAIL_MOUNT.with(|f| f.set(false));
        paint(&mut h).unwrap();
        assert_ne!(h.semantic_mount(ACTION), Some(mount));
    }
    #[test]
    fn hidden_retained_proxy_renews_without_replacing_owner_or_root() {
        const DIALOG: ComponentId = ComponentId(782);
        let mut h = ContentHost::new(Content::new(DIALOG, DialogContent::new(
            "Empty state dialog",
            Content::new(ComponentId(783), TextContent::new("Header")),
            Content::new(ROOT, EmptyStateContent::new("Empty", model("Create"), ACTION).unwrap()),
            Content::new(ComponentId(784), TextContent::new("Footer")),
        ))).unwrap();
        h.set_action_events(true);
        h.show_dialog(DIALOG).unwrap();
        paint(&mut h).unwrap();
        let root = h.get(ROOT).unwrap().mount;
        let action = h.get(ACTION).unwrap().mount;
        let owner = h
            .get(ROOT).unwrap()
            .component::<EmptyStateContent>()
            .unwrap()
            .owner
            .clone();
        assert!(h.accessibility_action(ACTION, None, AccessibilityAction::Press).activated);
        assert!(h.close_dialog());
        assert!(h.semantic_mount(ACTION).is_none());
        h.show_dialog(DIALOG).unwrap();
        assert!(h.drain_actions().is_empty());
        assert!(h.semantic_mount(ACTION).is_none());
        paint(&mut h).unwrap();
        assert_eq!(h.get(ROOT).unwrap().mount, root);
        assert_ne!(h.semantic_mount(ACTION), Some(action));
        assert!(Rc::ptr_eq(
            &owner,
            &h.get(ROOT).unwrap().component::<EmptyStateContent>().unwrap().owner
        ));
    }
    #[test]
    fn pure_body_and_surface_updates_preserve_authorized_queue_and_mount() {
        let mut h = host();
        let mount = h.semantic_mount(ACTION);
        assert!(
            h.accessibility_action(ACTION, None, AccessibilityAction::Press)
                .activated
        );
        let mut next = model("Create");
        next.title = "Different body".into();
        h.replace_empty_state(ROOT, next).unwrap();
        h.set_inherited_surface("panel").unwrap();
        paint(&mut h).unwrap();
        assert_eq!(h.semantic_mount(ACTION), mount);
        assert_eq!(h.drain_actions().len(), 1);
    }
    #[derive(Debug)]
    struct RetainedParent { shown: bool, children: Vec<Content> }
    impl Component for RetainedParent {
        fn active(&self) -> bool { self.shown }
        fn children(&self) -> &[Content] { &self.children }
        fn children_mut(&mut self) -> Option<&mut Vec<Content>> { Some(&mut self.children) }
        fn measure(&mut self, _: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild { MeasuredChild::new(available) }
        fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
            for child in &mut self.children { child.measure(text, frame.size); child.arrange(text, frame, clip); }
        }
    }
    #[test]
    fn inactive_ancestor_and_window_revoke_retained_descendants_without_resurrection() {
        const PARENT: ComponentId = ComponentId(785);
        let mut h = ContentHost::new(Content::new(PARENT, RetainedParent { shown: true,
            children: vec![Content::new(ROOT, EmptyStateContent::new("Empty", model("Create"), ACTION).unwrap())],
        })).unwrap();
        h.set_action_events(true);
        paint(&mut h).unwrap();
        let root = h.get(ROOT).unwrap().mount;
        let old = h.semantic_mount(ACTION).unwrap();
        assert!(h.accessibility_action(ACTION, None, AccessibilityAction::Press).activated);
        h.root.component_mut::<RetainedParent>().unwrap().shown = false;
        paint(&mut h).unwrap();
        assert!(h.semantic_mount(ACTION).is_none());
        assert!(!h.get(ROOT).unwrap().component::<EmptyStateContent>().unwrap().owner.borrow().window_active);
        h.set_window_active(false);
        h.set_window_active(true);
        assert!(!h.get(ROOT).unwrap().component::<EmptyStateContent>().unwrap().owner.borrow().window_active);
        h.root.component_mut::<RetainedParent>().unwrap().shown = true;
        assert!(h.semantic_mount(ACTION).is_none());
        assert!(h.drain_actions().is_empty());
        paint(&mut h).unwrap();
        assert_eq!(h.get(ROOT).unwrap().mount, root);
        assert_ne!(h.semantic_mount(ACTION), Some(old));
        assert!(h.accessibility_action(ACTION, None, AccessibilityAction::Press).activated);
        assert_eq!(h.drain_actions().len(), 1);
    }

}
