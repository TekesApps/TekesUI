//! Module-declared single-row toolbar. Measurement, overflow and menu input
//! belong to this control; window hosts only provide its allocated frame.
use crate::composite::MenuRowStyle;
use crate::{
    Button, ButtonSize, ButtonTextAlign, ButtonTextRenderer, ControlKey, MenuEvent, MenuView,
    paint::ButtonVariant,
};
use tekes_ui_scene::{ImageId, ImageQuad, Point, Rect, Scene, Size};
use tekes_ui_tokens::{Palette, proposal};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolbarAction {
    pub id: u64,
    pub title: String,
    pub enabled: bool,
}
/// Resolved toolkit icon artwork; labels also name overflow actions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolbarIconAction {
    pub id: u64,
    pub image: ImageId,
    pub label: String,
    pub enabled: bool,
}
#[derive(Debug)]
pub struct ToolbarControl {
    actions: Vec<ToolbarAction>,
    icons: std::collections::HashMap<u64, ImageId>,
    selected: std::collections::HashSet<u64>,
    buttons: Vec<crate::composition::ButtonContent>,
    visible: Vec<usize>,
    hidden: Vec<usize>,
    trigger: crate::composition::ButtonContent,
    viewport: Rect,
    held: Option<usize>,
    focused: Option<usize>,
    has_focus: bool,
    focus_visible: bool,
    last_action: Option<u64>,
    trigger_rect: Rect,
    menu: MenuView,
    frame: Rect,
}
impl Default for ToolbarControl {
    fn default() -> Self {
        Self {
            actions: Vec::new(),
            icons: Default::default(),
            selected: Default::default(),
            buttons: Vec::new(),
            visible: Vec::new(),
            hidden: Vec::new(),
            trigger: crate::composition::ButtonContent::new("…"),
            viewport: Rect::default(),
            held: None,
            focused: None,
            has_focus: false,
            focus_visible: false,
            last_action: None,
            trigger_rect: Rect::default(),
            menu: MenuView::new(0),
            frame: Rect::default(),
        }
    }
}
impl ToolbarControl {
    fn padding() -> f32 {
        proposal::metric("spacing", "space-regular").expect("toolbar spacing")
    }
    pub fn set_selected_ids(&mut self, ids: &[u64]) {
        self.selected = ids.iter().copied().collect();
    }
    pub fn set_icon_actions(&mut self, actions: Vec<ToolbarIconAction>) {
        self.icons = actions.iter().map(|a| (a.id, a.image)).collect();
        self.update_actions(
            actions
                .into_iter()
                .map(|a| ToolbarAction {
                    id: a.id,
                    title: a.label,
                    enabled: a.enabled,
                })
                .collect(),
        );
    }
    pub fn set_actions(&mut self, actions: Vec<ToolbarAction>) {
        self.icons.clear();
        self.update_actions(actions);
    }
    fn update_actions(&mut self, mut actions: Vec<ToolbarAction>) {
        // A label never introduces a second toolbar row. Popup labels reuse
        // these same normalized action names.
        for action in &mut actions {
            action.title = action
                .title
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
        }
        if self.actions != actions {
            let focused_id = self.focused.and_then(|i| self.actions.get(i)).map(|a| a.id);
            let overflow_focused = self.focused == Some(self.actions.len());
            self.cancel();
            self.buttons
                .resize_with(actions.len(), || crate::composition::ButtonContent::new(""));
            self.actions = actions;
            self.focused = focused_id
                .and_then(|id| self.actions.iter().position(|a| a.id == id))
                .or_else(|| overflow_focused.then_some(self.actions.len()));
            crate::composition::Component::set_focus(self, self.has_focus, self.focus_visible);
        }
    }
    pub fn measure(&self, text: &mut impl ButtonTextRenderer) -> Size {
        if self.actions.is_empty() {
            return Size::default();
        }
        let pad = Self::padding();
        let mut size = Button::measure(
            text,
            "…",
            f32::MAX,
            ButtonSize::Body,
            proposal::TextRole::UiBody,
        );
        size.width = 0.;
        for action in &self.actions {
            let item = if self.icons.contains_key(&action.id) {
                crate::layout_view::IconButtonView::measure(false)
            } else {
                Button::measure(
                    text,
                    &action.title,
                    f32::MAX,
                    ButtonSize::Body,
                    proposal::TextRole::UiBody,
                )
            };
            size.width += item.width;
            size.height = size.height.max(item.height);
        }
        Size::new(
            size.width + pad * (self.actions.len() + 1) as f32,
            size.height + pad * 2.,
        )
    }
    pub fn render(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        frame: Rect,
        palette: Palette,
    ) {
        let pad = Self::padding();
        let inner = Rect::new(
            frame.min_x() + pad,
            frame.min_y(),
            (frame.size.width - pad * 2.).max(0.),
            frame.size.height,
        );
        let sizes: Vec<_> = self
            .actions
            .iter()
            .map(|a| {
                if self.icons.contains_key(&a.id) {
                    crate::layout_view::IconButtonView::measure(false)
                } else {
                    Button::measure(
                        text,
                        &a.title,
                        f32::MAX,
                        ButtonSize::Body,
                        proposal::TextRole::UiBody,
                    )
                }
            })
            .collect();
        let total = sizes.iter().map(|s| s.width).sum::<f32>()
            + pad * self.actions.len().saturating_sub(1) as f32;
        let overflow = Button::measure(
            text,
            "…",
            f32::MAX,
            ButtonSize::Body,
            proposal::TextRole::UiBody,
        );
        let mut visible = Vec::new();
        let mut used = 0.;
        for (i, s) in sizes.iter().enumerate() {
            let next = used + if visible.is_empty() { 0. } else { pad } + s.width;
            if total > inner.size.width && next + pad + overflow.width > inner.size.width {
                break;
            }
            visible.push(i);
            used = next;
        }
        let hidden: Vec<_> = (visible.len()..self.actions.len()).collect();
        if self.frame != frame || self.hidden != hidden {
            self.cancel();
        }
        if self.hidden != hidden {
            self.menu.replace_items(hidden.len());
        }
        self.frame = frame;
        self.visible = visible;
        self.hidden = hidden;
        let styles = self
            .hidden
            .iter()
            .map(|&i| MenuRowStyle {
                disabled: !self.actions[i].enabled,
                ..Default::default()
            })
            .collect();
        self.menu.set_row_styles(styles);
        let mut x = inner.min_x();
        for &i in &self.visible {
            let size = sizes[i];
            let rect = Rect::new(
                x,
                frame.min_y() + (frame.size.height - size.height) * 0.5,
                size.width,
                size.height,
            );
            self.buttons[i].button.state.disabled = !self.actions[i].enabled;
            self.buttons[i].button.state.selected = self.selected.contains(&self.actions[i].id);
            self.buttons[i].button.render(
                scene,
                text,
                if self.icons.contains_key(&self.actions[i].id) {
                    ""
                } else {
                    &self.actions[i].title
                },
                rect,
                Some(frame),
                palette,
                ButtonVariant::Quiet,
                ButtonSize::Body,
                proposal::TextRole::UiBody,
                ButtonTextAlign::Center,
            );
            if let Some(&image) = self.icons.get(&self.actions[i].id) {
                let side = proposal::metric("size", "size-icon-standard")
                    .expect("toolbar icon token")
                    .min(rect.size.width)
                    .min(rect.size.height);
                scene.push_clip(frame);
                scene.push_image(ImageQuad {
                    rect: Rect::new(
                        rect.center().x - side / 2.,
                        rect.center().y - side / 2.,
                        side,
                        side,
                    ),
                    image,
                    opacity: if self.actions[i].enabled { 1. } else { 0.35 },
                });
                scene.pop_clip();
            }
            x += size.width + pad;
        }
        self.trigger_rect = Rect::default();
        if !self.hidden.is_empty() && inner.size.width > 0. {
            self.trigger_rect = Rect::new(
                (inner.max_x() - overflow.width).max(inner.min_x()),
                frame.min_y() + (frame.size.height - overflow.height) * 0.5,
                overflow.width.min(inner.size.width),
                overflow.height,
            );
            self.trigger.button.render(
                scene,
                text,
                "…",
                self.trigger_rect,
                Some(frame),
                palette,
                ButtonVariant::Quiet,
                ButtonSize::Body,
                proposal::TextRole::UiBody,
                ButtonTextAlign::Center,
            );
        }
    }
    /// Called after all ordinary content so the popup can overlay sibling panes.
    pub fn render_overlay(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        viewport: Rect,
        palette: Palette,
    ) {
        let labels: Vec<_> = self
            .hidden
            .iter()
            .map(|&i| self.actions[i].title.as_str())
            .collect();
        self.menu.render(
            scene,
            text,
            &labels,
            self.trigger_rect,
            viewport,
            None,
            palette,
            proposal::TextRole::UiBody,
            false,
        );
    }
    pub fn set_viewport(&mut self, viewport: Rect) {
        self.viewport = viewport;
    }
    pub fn take_action(&mut self) -> Option<u64> {
        self.last_action.take()
    }
    pub fn is_open(&self) -> bool {
        self.menu.is_open()
    }
    pub fn pointer_down(&mut self, p: Point) -> bool {
        if self.menu.is_open() {
            self.menu.pointer_down_at(p);
            return true;
        }
        if !self.hidden.is_empty() && self.trigger.button.pointer_down_at(p) {
            return true;
        }
        self.visible
            .iter()
            .any(|&i| self.buttons[i].button.pointer_down_at(p))
    }
    pub fn pointer_move(&mut self, p: Point) {
        if self.menu.is_open() {
            self.menu.pointer_move_at(p);
        }
        self.trigger.button.pointer_move_at(p);
        for b in &mut self.buttons {
            b.button.pointer_move_at(p);
        }
    }
    pub fn pointer_up(&mut self, p: Point) -> Option<u64> {
        if self.menu.is_open() {
            let event = self.menu.pointer_up_at(p);
            return self.action_for(event);
        }
        if !self.hidden.is_empty() && self.trigger.button.pointer_up_at(p) {
            self.menu.show(0);
            return None;
        }
        let mut action = None;
        for &i in &self.visible {
            if self.buttons[i].button.pointer_up_at(p) && self.actions[i].enabled {
                action = Some(self.actions[i].id);
            }
        }
        action
    }
    fn action_for(&self, event: MenuEvent) -> Option<u64> {
        if let MenuEvent::Activated { index, .. } = event {
            let action = self.actions.get(*self.hidden.get(index)?)?;
            return action.enabled.then_some(action.id);
        }
        None
    }
    pub fn key(&mut self, key: ControlKey, down: bool) -> Option<u64> {
        let event = self.menu.key(key, down);
        self.action_for(event)
    }
    pub fn cancel(&mut self) {
        for b in &mut self.buttons {
            b.button.cancel();
        }
        self.trigger.button.cancel();
        self.held = None;
        self.last_action = None;
        self.menu.key(ControlKey::Escape, true);
        self.menu.clear_presentation();
    }
    pub fn overflow_ids(&self) -> Vec<u64> {
        self.hidden.iter().map(|&i| self.actions[i].id).collect()
    }
}

// The composition wrapper keeps the real ButtonContent input contract,
// including explicit drag propagation, modifiers and primary-button ownership.
impl crate::composition::Component for ToolbarControl {
    fn measure(
        &mut self,
        text: &mut dyn ButtonTextRenderer,
        _: Size,
    ) -> crate::layout::MeasuredChild {
        crate::layout::MeasuredChild::new(ToolbarControl::measure(self, &mut ToolbarText(text)))
    }
    fn paint(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        frame: Rect,
        _: Rect,
        theme: crate::composition::ContentTheme,
    ) {
        self.render(scene, &mut ToolbarText(text), frame, theme.palette);
    }
    fn paint_overlay(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        _: Rect,
        _: Rect,
        theme: crate::composition::ContentTheme,
    ) {
        self.render_overlay(scene, &mut ToolbarText(text), self.viewport, theme.palette);
    }
    fn overlay_open(&self) -> bool {
        self.is_open()
    }
    fn overlay_event(
        &mut self,
        event: &crate::composition::ContentEvent,
    ) -> crate::composition::ContentResponse {
        self.event(event)
    }
    fn focusable(&self) -> bool {
        self.actions.iter().any(|a| a.enabled)
    }
    fn set_focus(&mut self, focused: bool, visible: bool) {
        self.has_focus = focused;
        self.focus_visible = visible;
        for (i, button) in self.buttons.iter_mut().enumerate() {
            button.set_focus(focused && self.focused == Some(i), visible);
        }
        self.trigger
            .set_focus(focused && self.focused == Some(self.actions.len()), visible);
    }
    fn key_event(
        &mut self,
        key: &crate::composition::ContentKeyEvent,
    ) -> crate::composition::ContentResponse {
        use crate::composition::ContentResponse;
        self.last_action = None;
        if self.is_open() {
            if let Some(k) = key.key {
                self.last_action = self.key(k, key.down);
            }
            return ContentResponse {
                consumed: true,
                activated: self.last_action.is_some(),
                changed: true,
                ..Default::default()
            };
        }
        let Some(i) = self.focused else {
            return ContentResponse::default();
        };
        let result = if i == self.actions.len() {
            self.trigger.key_event(key)
        } else if let Some(button) = self.buttons.get_mut(i) {
            button.key_event(key)
        } else {
            return ContentResponse::default();
        };
        if result.activated {
            if i == self.actions.len() {
                self.menu.show(0);
            } else if self.actions[i].enabled {
                self.last_action = Some(self.actions[i].id);
            }
        }
        ContentResponse {
            activated: self.last_action.is_some(),
            ..result
        }
    }
    fn overlay_key_event(
        &mut self,
        key: &crate::composition::ContentKeyEvent,
    ) -> crate::composition::ContentResponse {
        self.key_event(key)
    }
    fn interactive(&self) -> bool {
        true
    }
    fn semantics(&self) -> crate::composition::ContentSemantics {
        crate::composition::ContentSemantics::new(crate::composition::ContentRole::Group, "Toolbar")
    }
    fn event(
        &mut self,
        event: &crate::composition::ContentEvent,
    ) -> crate::composition::ContentResponse {
        use crate::composition::{ContentEvent, ContentResponse};
        match event {
            ContentEvent::Pointer(pointer) => self.pointer_event(pointer),
            ContentEvent::PointerDown(p)
            | ContentEvent::PointerMove(p)
            | ContentEvent::PointerUp(p) => {
                use crate::composition::{PointerButton, PointerEvent, PointerPhase};
                let phase = match event {
                    ContentEvent::PointerDown(_) => PointerPhase::Down,
                    ContentEvent::PointerUp(_) => PointerPhase::Up,
                    _ => PointerPhase::Move,
                };
                self.pointer_event(&PointerEvent {
                    position: *p,
                    phase,
                    button: Some(PointerButton::Primary),
                    modifiers: Default::default(),
                    click_count: 1,
                })
            }
            ContentEvent::Key { key, down } => {
                self.last_action = self.key(*key, *down);
                ContentResponse {
                    consumed: true,
                    activated: self.last_action.is_some(),
                    ..Default::default()
                }
            }
            ContentEvent::Cancel => {
                self.cancel();
                ContentResponse::consumed(true)
            }
            _ => ContentResponse::default(),
        }
    }
    fn pointer_event(
        &mut self,
        pointer: &crate::composition::PointerEvent,
    ) -> crate::composition::ContentResponse {
        use crate::composition::{ContentResponse, PointerPhase};
        if !pointer.is_primary() {
            return ContentResponse::default();
        }
        self.last_action = None;
        if self.is_open() {
            let p = pointer.position;
            match pointer.phase {
                PointerPhase::Down => {
                    self.menu.pointer_down_at(p);
                }
                PointerPhase::Up => {
                    let event = self.menu.pointer_up_at(p);
                    self.last_action = self.action_for(event);
                }
                _ => self.menu.pointer_move_at(p),
            }
            return ContentResponse {
                consumed: true,
                capture_pointer: pointer.phase == PointerPhase::Down,
                activated: self.last_action.is_some(),
                changed: true,
                ..Default::default()
            };
        }
        if pointer.phase == PointerPhase::Down {
            self.held = if !self.hidden.is_empty() && self.trigger_rect.contains(pointer.position) {
                Some(self.actions.len())
            } else {
                self.visible.iter().copied().find(|&i| {
                    self.buttons[i]
                        .button
                        .layout()
                        .is_some_and(|l| l.hit_test(pointer.position))
                })
            };
        }
        if pointer.phase == PointerPhase::Down {
            self.focused = self.held;
            self.set_focus(self.has_focus, self.focus_visible);
        }
        let Some(i) = self.held else {
            self.pointer_move(pointer.position);
            return ContentResponse::default();
        };
        let result = if i == self.actions.len() {
            self.trigger.pointer_event(pointer)
        } else {
            self.buttons[i].pointer_event(pointer)
        };
        if result.activated {
            if i == self.actions.len() {
                self.menu.show(0);
            } else if self.actions[i].enabled {
                self.last_action = Some(self.actions[i].id);
            }
        }
        if pointer.phase == PointerPhase::Up {
            self.held = None;
        }
        ContentResponse {
            activated: self.last_action.is_some(),
            ..result
        }
    }
}
struct ToolbarText<'a>(&'a mut dyn ButtonTextRenderer);
impl ButtonTextRenderer for ToolbarText<'_> {
    fn measure(&mut self, s: &str, role: proposal::TextRole, width: f32) -> Size {
        self.0.measure(s, role, width)
    }
    fn draw(
        &mut self,
        scene: &mut Scene,
        s: &str,
        role: proposal::TextRole,
        rect: Rect,
        align: ButtonTextAlign,
        ink: tekes_ui_tokens::Rgba,
    ) {
        self.0.draw(scene, s, role, rect, align, ink);
    }
    fn backing_scale(&self) -> f32 {
        self.0.backing_scale()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::composition::{
        Component, PointerButton, PointerEvent, PointerModifiers, PointerPhase,
    };
    struct Text {
        height: f32,
    }
    impl ButtonTextRenderer for Text {
        fn measure(&mut self, text: &str, _: proposal::TextRole, _: f32) -> Size {
            assert!(!text.contains('\n'));
            Size::new(text.chars().count() as f32 * 8., self.height)
        }
        fn draw(
            &mut self,
            _: &mut Scene,
            text: &str,
            _: proposal::TextRole,
            _: Rect,
            _: ButtonTextAlign,
            _: tekes_ui_tokens::Rgba,
        ) {
            assert!(!text.contains('\n'));
        }
    }
    fn toolbar() -> ToolbarControl {
        let mut toolbar = ToolbarControl::default();
        toolbar.set_actions(vec![
            ToolbarAction {
                id: 11,
                title: "First\nAction".into(),
                enabled: true,
            },
            ToolbarAction {
                id: 22,
                title: "Second action".into(),
                enabled: false,
            },
            ToolbarAction {
                id: 33,
                title: "Third action".into(),
                enabled: true,
            },
        ]);
        toolbar.set_viewport(Rect::new(0., 0., 600., 400.));
        toolbar
    }
    fn pointer(p: Point, phase: PointerPhase) -> PointerEvent {
        PointerEvent {
            position: p,
            phase,
            button: Some(PointerButton::Primary),
            modifiers: PointerModifiers::default(),
            click_count: 1,
        }
    }
    #[test]
    fn pure_icon_toolbar_uses_square_hits_original_ids_and_named_overflow() {
        let mut toolbar = ToolbarControl::default();
        toolbar.set_icon_actions(vec![
            ToolbarIconAction {
                id: 101,
                image: ImageId(1),
                label: "Back".into(),
                enabled: true,
            },
            ToolbarIconAction {
                id: 102,
                image: ImageId(2),
                label: "Forward".into(),
                enabled: false,
            },
        ]);
        let mut text = Text { height: 16. };
        let height = toolbar.measure(&mut text).height;
        let mut scene = Scene::new();
        toolbar.render(
            &mut scene,
            &mut text,
            Rect::new(0., 0., 240., height),
            Palette::from(tekes_ui_tokens::Theme::LIGHT),
        );
        let first = toolbar.buttons[0].button.layout().unwrap().frame;
        assert_eq!(
            first.size.width,
            crate::layout_view::IconButtonView::measure(false).width
        );
        assert!(toolbar.pointer_down(first.center()));
        assert_eq!(toolbar.pointer_up(first.center()), Some(101));
        let second = toolbar.buttons[1].button.layout().unwrap().frame;
        toolbar.pointer_down(second.center());
        assert_eq!(toolbar.pointer_up(second.center()), None);
        toolbar.render(
            &mut scene,
            &mut text,
            Rect::new(0., 0., 40., height),
            Palette::from(tekes_ui_tokens::Theme::LIGHT),
        );
        assert_eq!(toolbar.measure(&mut text).height, height);
        assert_eq!(toolbar.overflow_ids(), vec![101, 102]);
        assert_eq!(toolbar.actions[0].title, "Back");
    }

    #[test]
    fn narrow_toolbar_overflows_without_growing_and_measures_font_changes() {
        let mut toolbar = toolbar();
        let mut text = Text { height: 16. };
        let height = toolbar.measure(&mut text).height;
        let mut scene = Scene::new();
        toolbar.render(
            &mut scene,
            &mut text,
            Rect::new(0., 0., 600., height),
            Palette::from(tekes_ui_tokens::Theme::LIGHT),
        );
        assert!(toolbar.overflow_ids().is_empty());
        toolbar.render(
            &mut scene,
            &mut text,
            Rect::new(0., 0., 80., height),
            Palette::from(tekes_ui_tokens::Theme::LIGHT),
        );
        assert_eq!(toolbar.overflow_ids(), vec![11, 22, 33]);
        assert_eq!(toolbar.measure(&mut text).height, height);
        text.height = 60.;
        assert!(toolbar.measure(&mut text).height > height);
    }
    #[test]
    fn overflow_activates_registered_id_and_disables_unavailable_actions() {
        let mut toolbar = toolbar();
        let mut text = Text { height: 16. };
        let height = toolbar.measure(&mut text).height;
        toolbar.render(
            &mut Scene::new(),
            &mut text,
            Rect::new(0., 0., 80., height),
            Palette::from(tekes_ui_tokens::Theme::LIGHT),
        );
        let p = Point::new(
            toolbar.trigger_rect.min_x() + 4.,
            toolbar.trigger_rect.min_y() + 4.,
        );
        assert!(
            toolbar
                .pointer_event(&pointer(p, PointerPhase::Down))
                .capture_pointer
        );
        toolbar.pointer_event(&pointer(p, PointerPhase::Up));
        assert!(toolbar.is_open());
        toolbar.render_overlay(
            &mut Scene::new(),
            &mut text,
            Rect::new(0., 0., 600., 400.),
            Palette::from(tekes_ui_tokens::Theme::LIGHT),
        );
        // Disabled rows cannot activate even through a direct menu action.
        assert_eq!(toolbar.menu.activate_item(1), MenuEvent::None);
        let event = toolbar.menu.activate_item(2);
        assert_eq!(toolbar.action_for(event), Some(33));
        // A geometry change revokes a held trigger so a late up cannot reopen it.
        toolbar.pointer_event(&pointer(p, PointerPhase::Down));
        toolbar.render(
            &mut Scene::new(),
            &mut text,
            Rect::new(0., 0., 600., height),
            Palette::from(tekes_ui_tokens::Theme::LIGHT),
        );
        toolbar.pointer_event(&pointer(p, PointerPhase::Up));
        assert!(!toolbar.is_open());
    }
    #[test]
    fn real_button_drag_can_bubble_without_a_phantom_activation() {
        let mut toolbar = toolbar();
        let mut text = Text { height: 16. };
        let height = toolbar.measure(&mut text).height;
        toolbar.render(
            &mut Scene::new(),
            &mut text,
            Rect::new(0., 0., 600., height),
            Palette::from(tekes_ui_tokens::Theme::LIGHT),
        );
        let p = Point::new(20., height * 0.5);
        assert!(
            toolbar
                .pointer_event(&pointer(p, PointerPhase::Down))
                .capture_pointer
        );
        assert!(
            !toolbar
                .pointer_event(&pointer(Point::new(220., height * 0.5), PointerPhase::Drag))
                .consumed
        );
        toolbar.cancel();
        assert!(
            !toolbar
                .pointer_event(&pointer(p, PointerPhase::Up))
                .activated
        );
        assert_eq!(toolbar.take_action(), None);
    }
}

#[cfg(test)]
mod composition_tests {
    use super::*;
    use crate::composition::*;
    struct Text;
    impl ButtonTextRenderer for Text {
        fn measure(&mut self, text: &str, _: proposal::TextRole, _: f32) -> Size {
            Size::new(text.len() as f32 * 8., 16.)
        }
        fn draw(
            &mut self,
            _: &mut Scene,
            _: &str,
            _: proposal::TextRole,
            _: Rect,
            _: ButtonTextAlign,
            _: tekes_ui_tokens::Rgba,
        ) {
        }
    }
    #[test]
    fn component_host_retains_command_identity_and_focus_across_registration_updates() {
        let mut toolbar = ToolbarControl::default();
        toolbar.set_actions(vec![ToolbarAction {
            id: 7,
            title: "Increment 0".into(),
            enabled: true,
        }]);
        toolbar.set_viewport(Rect::new(0., 0., 600., 400.));
        let mut host = ContentHost::new(Content::new(ComponentId(1), toolbar)).unwrap();
        host.set_action_events(true);
        let frame = Rect::new(0., 0., 600., 48.);
        host.render(
            &mut Scene::new(),
            &mut Text,
            frame,
            frame,
            Palette::from(tekes_ui_tokens::Theme::LIGHT),
        )
        .unwrap();
        let p = Point::new(24., 24.);
        host.event(&ContentEvent::PointerDown(p));
        host.event(&ContentEvent::PointerUp(p));
        assert_eq!(host.drain_actions().len(), 1);
        let toolbar = host
            .get_mut(ComponentId(1))
            .unwrap()
            .component_mut::<ToolbarControl>()
            .unwrap();
        assert_eq!(toolbar.take_action(), Some(7));
        toolbar.set_actions(vec![ToolbarAction {
            id: 7,
            title: "Increment 1".into(),
            enabled: true,
        }]);
        let key = |down| {
            ContentEvent::KeyInput(ContentKeyEvent {
                key: Some(ControlKey::Space),
                down,
                repeat: false,
                modifiers: Default::default(),
                native_code: 49,
                characters: " ".into(),
                characters_ignoring_modifiers: " ".into(),
            })
        };
        host.event(&key(true));
        host.event(&key(false));
        assert_eq!(host.drain_actions().len(), 1);
        assert_eq!(
            host.get_mut(ComponentId(1))
                .unwrap()
                .component_mut::<ToolbarControl>()
                .unwrap()
                .take_action(),
            Some(7)
        );
    }
}
