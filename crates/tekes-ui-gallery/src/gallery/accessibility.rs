//! Default Gallery host projection. State and activation remain in the real views.
use super::*;
use tekes_ui_platform_macos::{
    AccessibilityAction as A, AccessibilityKey as K, AccessibilityNode as N, AccessibilityRole as R,
};
use tekes_ui_scene::Rect;
pub(super) const NAV: u64 = 0x47414c0001;
pub(super) const SEARCH: u64 = 0x47414c0002;
pub(super) const STORY: u64 = 0x47414c1000;
pub(super) const INPUT: u64 = 0x47414c2000;
pub(super) const DETAIL: u64 = 0x47414c3000;
pub(super) fn node(key: K, role: R, label: &str, frame: Rect) -> N {
    N {
        key,
        parent: None,
        role,
        frame: tekes_ui_platform_macos::Rect {
            origin: tekes_ui_platform_macos::Point {
                x: frame.min_x() as f64,
                y: frame.min_y() as f64,
            },
            size: tekes_ui_platform_macos::Size {
                width: frame.size.width as f64,
                height: frame.size.height as f64,
            },
        },
        label: label.into(),
        value: None,
        description: None,
        enabled: true,
        focused: false,
        selected: false,
        expanded: None,
        invalid: false,
        required: false,
        text_selection: None,
        read_only: false,
        can_show_menu: false,
    }
}
impl Gallery {
    pub fn accessibility_nodes(&self) -> Vec<N> {
        let mut nodes = self
            .navigation_search
            .accessibility_nodes(SEARCH, "Search components");
        let geometry = self.sidebar.geometry();
        let origin = geometry.content_origin();
        if let Some(clip) = geometry.visible_viewport_rect() {
            nodes.push(node(
                K {
                    component: NAV,
                    part: None,
                },
                R::List,
                "Components",
                clip,
            ));
            for section in self.sidebar.sections() {
                for (page, frame) in &section.items {
                    if let Some(frame) = frame.offset(origin.x, origin.y).intersection(&clip) {
                        let mut n = node(
                            K {
                                component: NAV,
                                part: Some(page.index()),
                            },
                            R::Button,
                            page.name(),
                            frame,
                        );
                        n.parent = Some(K {
                            component: NAV,
                            part: None,
                        });
                        n.selected = *page == self.page;
                        n.focused = self.navigation_focus == Some(*page);
                        nodes.push(n);
                    }
                }
            }
        }
        if let Page::Workbench(i) = self.page {
            nodes.extend(self.workbench.accessibility_nodes(i));
        } else {
            let demo = match self.content_page() {
                Page::TextEditing => Some(&self.editor),
                Page::Text => Some(&self.text_view),
                Page::ScrollArea => Some(&self.scroll_demo),
                Page::Icons => Some(&self.icon_search),
                _ => None,
            };
            if let Some(demo) = demo {
                nodes.extend(demo.accessibility_nodes(
                    INPUT + 0x100 + self.page.index() as u64,
                    self.page.name(),
                ));
            }
        }
        if let Page::Workbench(i) = self.page {
            let page = K {
                component: STORY + i as u64,
                part: None,
            };
            let frame = self.viewport.bounds();
            if !nodes.iter().any(|n| n.key == page)
                && frame.size.width > 0.
                && frame.size.height > 0.
            {
                for n in &mut nodes {
                    if n.parent.is_none()
                        && (n.key.component == page.component
                            || n.key.component == INPUT + i as u64
                            || n.key.component == DETAIL + i as u64)
                    {
                        n.parent = Some(page);
                    }
                }
                nodes.push(node(
                    page,
                    R::Group,
                    workbench::entry(i).title(),
                    self.viewport.bounds(),
                ));
            }
        }
        // The Gallery sidebar is a different host scope from a preview modal.
        // Match the existing native key/text owner precedence instead of
        // inventing a window-wide modal or reporting two focused owners.
        if self.navigation_search.state().is_some() {
            for n in &mut nodes {
                if n.key.component != SEARCH {
                    n.focused = false;
                }
            }
        } else if self.navigation_focus.is_some() {
            for n in &mut nodes {
                if n.key.component != NAV {
                    n.focused = false;
                }
            }
        }
        let root = K {
            component: 0x47414c0000,
            part: None,
        };
        for n in &mut nodes {
            if n.parent.is_none() {
                n.parent = Some(root);
            }
        }
        let content = self.viewport.bounds();
        let sidebar = self.sidebar.bounds();
        nodes.insert(
            0,
            node(
                root,
                R::Group,
                &self.title(),
                Rect::new(
                    0.,
                    0.,
                    content.max_x().max(sidebar.max_x()),
                    content.max_y().max(sidebar.max_y()),
                ),
            ),
        );
        nodes
    }
    pub fn accessibility_action(&mut self, target: K, action: &A) -> Change {
        // All Gallery producers share the host window scope, including controls
        // first mounted after the initial inactive notification.
        if !self.workbench.is_window_active() {
            return Change::NONE;
        }
        if !self
            .accessibility_nodes()
            .iter()
            .any(|n| n.key == target && n.enabled)
            || self.text_input_state().is_some_and(|s| s.marked.is_some())
        {
            return Change::NONE;
        }
        if target.component == NAV {
            let Some(page) = target.part.and_then(|i| Page::ALL.get(i)).copied() else {
                return Change::NONE;
            };
            return match action {
                A::Press => self.apply(Command::Page(page)),
                A::Focus => {
                    self.navigation_search.blur();
                    if let Page::Workbench(i) = self.page {
                        self.workbench.clear_focus(i);
                    }
                    self.navigation_focus = Some(page);
                    Change::REDRAW
                }
                _ => Change::NONE,
            };
        }
        let Some(index) = target.part else {
            return Change::NONE;
        };
        if target.component == SEARCH {
            let changed = self.navigation_search.accessibility_action(index, action);
            if changed && matches!(action, A::Focus) {
                if let Page::Workbench(i) = self.page {
                    self.workbench.clear_focus(i);
                }
                self.navigation_focus = None;
            }
            self.navigation_search.refresh(&mut self.text);
            self.sidebar.set_query(self.navigation_search.query());
            return if changed {
                Change::REDRAW
            } else {
                Change::NONE
            };
        }
        if let Page::Workbench(i) = self.page {
            let changed = self
                .workbench
                .accessibility_action(i, target, action, &mut self.text);
            if changed {
                self.navigation_search.blur();
                self.navigation_focus = None;
            }
            return if changed {
                Change::REDRAW
            } else {
                Change::NONE
            };
        }
        let owner = INPUT + 0x100 + self.page.index() as u64;
        if target.component != owner {
            return Change::NONE;
        }
        let demo = match self.content_page() {
            Page::TextEditing => &mut self.editor,
            Page::Text => &mut self.text_view,
            Page::ScrollArea => &mut self.scroll_demo,
            Page::Icons => &mut self.icon_search,
            _ => return Change::NONE,
        };
        let changed = demo.accessibility_action(index, action);
        demo.refresh(&mut self.text);
        if changed {
            self.navigation_search.blur();
            self.navigation_focus = None;
            Change::REDRAW
        } else {
            Change::NONE
        }
    }
}

#[cfg(test)]
mod window_lifecycle_tests {
    use super::*;
    use crate::gallery::static_text::StaticText;

    #[test]
    fn inactive_gallery_rejects_modal_and_navigation_ax_until_restored() {
        let mut g = Gallery::with_static_text(StaticText::new(None));
        assert!(g.open_route("component-dialog"));
        // Native startup delivers Focus(false) before the first geometry/paint.
        g.handle_input(&Event::Focus(false));
        g.set_view_size(1200., 850.);
        g.build_scene(&mut tekes_ui_scene::Scene::new());
        let Page::Workbench(i) = g.page else {
            panic!("dialog page")
        };
        let open = K {
            component: STORY + i as u64,
            part: Some(20),
        };
        let nav = g
            .accessibility_nodes()
            .into_iter()
            .find(|n| n.key.component == NAV && !n.selected)
            .expect("visible other page")
            .key;
        assert!(
            !g.accessibility_action(open, &A::Press).redraw,
            "inactive AX must not open a modal"
        );
        assert!(!g.workbench.accessibility_modal(i));
        assert!(!g.accessibility_action(nav, &A::Press).redraw);
        assert_eq!(g.page, Page::Workbench(i));
        assert!(g.accessibility_nodes().iter().all(|n| !n.focused));
        g.handle_input(&Event::Focus(true));
        g.build_scene(&mut tekes_ui_scene::Scene::new());
        assert!(g.accessibility_action(open, &A::Press).redraw);
        assert!(g.workbench.accessibility_modal(i));
    }
}
