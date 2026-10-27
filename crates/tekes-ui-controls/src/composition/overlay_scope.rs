use super::*;

/// An interactive overlay uses its original mounted Content subtree for pointer,
/// Tab, text queries and accessibility. Passive overlays do not declare a scope.
/// Closing excludes the subtree immediately, independently of exit painting.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OverlayScope {
    /// An eligible original descendant, otherwise the first eligible focus owner.
    /// This is an opening intent; subsequent renders preserve selection and IME.
    pub initial_focus: Option<ComponentId>,
}

#[derive(Debug)]
pub(super) struct ScopedOverlay {
    pub(super) id: ComponentId,
    mount: u64,
    pub(super) focus_pending: bool,
    pub(super) restore: Option<(ComponentId, u64)>,
}

impl Content {
    fn overlay_scope_path(&self) -> Vec<(ComponentId, u64, OverlayScope)> {
        if !self.active() || self.component.semantics().disabled {
            return Vec::new();
        }
        // Same ordering as overlay paint: the last sibling is uppermost.
        let mut path = self
            .component
            .children()
            .iter()
            .rev()
            .map(Content::overlay_scope_path)
            .find(|path| !path.is_empty())
            .unwrap_or_default();
        if self.component.overlay_open()
            && let Some(scope) = self.component.overlay_scope()
        {
            path.insert(0, (self.id, self.mount, scope));
        }
        path
    }

    fn clear_hover_outside(&mut self, scope: ComponentId, captured: Option<ComponentId>) {
        if self.id == scope || captured == Some(self.id) {
            return;
        }
        // Exit notification only: never a new hit, action, focus or capture.
        self.component
            .event(&ContentEvent::PointerMove(Point::new(f32::MIN, f32::MIN)));
        if let Some(children) = self.component.children_mut() {
            for child in children {
                child.clear_hover_outside(scope, captured);
            }
        }
    }
}

impl ContentHost {
    fn current_overlay_path(&self) -> Vec<(ComponentId, u64, OverlayScope)> {
        self.base_scope().overlay_scope_path()
    }
    pub(super) fn overlay_scope_id(&self) -> Option<ComponentId> {
        self.current_overlay_path().last().map(|(id, _, _)| *id)
    }

    pub(super) fn reconcile_overlay_scopes(&mut self) {
        let path = self.current_overlay_path();
        let common = path
            .iter()
            .zip(&self.overlay_scopes)
            .take_while(|((id, mount, _), previous)| *id == previous.id && *mount == previous.mount)
            .count();
        let mut restore = None;
        let mut closed = None;
        while self.overlay_scopes.len() > common {
            let scope = self.overlay_scopes.pop().unwrap();
            restore = Some(scope.restore);
            closed = Some((scope.id, scope.mount));
        }
        // Only an actual close finishes an old intent. Switching directly to a
        // different overlay must not briefly focus/blur the background editor.
        let return_enters_current_scope = restore.flatten().is_some_and(|(id, mount)| {
            self.scope().find_active(id).is_some_and(|node| {
                node.mount == mount
                    && node.component.focusable()
                    && !node.component.semantics().disabled
            })
        });
        if (path.len() == common || return_enters_current_scope)
            && let Some(restore) = restore
        {
            let newer_outside_focus = self.focused.is_some_and(|focused| {
                let eligible = self.scope().find_active(focused).is_some_and(|node| {
                    node.component.focusable() && !node.component.semantics().disabled
                });
                let belonged_to_closed = closed
                    .and_then(|(id, mount)| self.root.find(id).filter(|node| node.mount == mount))
                    .is_some_and(|node| node.find(focused).is_some());
                eligible && !belonged_to_closed
            });
            if !newer_outside_focus {
                self.restore_focus(restore);
            }
        }
        for (id, mount, _) in path.iter().skip(common) {
            let restore = self.focused.and_then(|focused| {
                self.base_scope()
                    .find_active(focused)
                    .filter(|node| {
                        (node.component.focusable() || focused == *id)
                            && !node.component.semantics().disabled
                    })
                    .map(|node| (focused, node.mount))
            });
            self.overlay_scopes.push(ScopedOverlay {
                id: *id,
                mount: *mount,
                focus_pending: true,
                restore,
            });
        }
        let Some(&(id, _, declaration)) = path.last() else {
            return;
        };
        if path.len() > common {
            self.root.clear_hover_outside(id, self.captured);
        }
        // Changing focus blurs controls and can cancel an armed pointer. Finish
        // that original gesture before applying the opening focus intent.
        if self.captured.is_some() || !self.overlay_scopes.last().is_some_and(|s| s.focus_pending) {
            return;
        }
        let scope = self.scope();
        let mut eligible = Vec::new();
        scope.focus_ids(&mut eligible);
        if eligible.is_empty() {
            return; // Layout may not yet have arranged the new original subtree.
        }
        if !self.focused.is_some_and(|id| eligible.contains(&id)) {
            let target = declaration
                .initial_focus
                .filter(|id| eligible.contains(id))
                .or_else(|| eligible.first().copied());
            self.focus(target, true);
            if self.focused != target {
                return;
            }
        }
        self.overlay_scopes.last_mut().unwrap().focus_pending = false;
    }
}

impl ContentHost {
    // Pure original-owner staging for a request that leaves the active scope path
    // intact. Branches requiring close/open/restore preparation refuse explicitly.
    pub(super) fn replacement_keeps_overlay_scope(
        &self,
        retired: &Content,
        replacement: Option<&Content>,
    ) -> bool {
        fn opens(c: &Content) -> bool {
            (c.component.overlay_scope().is_some() && c.component.overlay_open())
                || c.component.children().iter().any(opens)
        }
        if replacement.is_some_and(opens) {
            return false;
        }
        let path = self.current_overlay_path();
        if path.len() != self.overlay_scopes.len()
            || path
                .iter()
                .zip(&self.overlay_scopes)
                .any(|((id, mount, _), s)| {
                    *id != s.id || *mount != s.mount || retired.find(*id).is_some()
                })
        {
            return false;
        }
        if self.captured.is_none() && self.overlay_scopes.last().is_some_and(|s| s.focus_pending) {
            let mut eligible = Vec::new();
            self.scope().focus_ids(&mut eligible);
            if !eligible.is_empty() {
                return false;
            }
        }
        true
    }
}
