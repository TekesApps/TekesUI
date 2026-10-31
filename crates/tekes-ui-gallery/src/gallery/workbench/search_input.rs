//! Workbench-only facade: Search uses ContentHost while all other editors stay on TextDemo.
use super::super::{PageCtx, text::TextEngine, text_demo::TextDemo};
use super::search_host::SearchHost;
use tekes_ui_platform_macos::{Event, TextInputState};
use tekes_ui_scene::{Point, Rect, Scene};

#[derive(Debug)]
pub(super) enum StoryInput {
    Legacy(Box<TextDemo>),
    Search(Box<SearchHost>),
}

impl From<TextDemo> for StoryInput {
    fn from(value: TextDemo) -> Self {
        Self::Legacy(Box::new(value))
    }
}

impl StoryInput {
    pub(super) fn search() -> Self {
        Self::Search(Box::new(SearchHost::new()))
    }

    pub(super) fn legacy_mut(&mut self) -> Option<&mut TextDemo> {
        match self {
            Self::Legacy(demo) => Some(demo.as_mut()),
            Self::Search(_) => None,
        }
    }

    pub(super) fn search_host(&self) -> Option<&SearchHost> {
        match self {
            Self::Search(host) => Some(host),
            Self::Legacy(_) => None,
        }
    }

    pub(super) fn search_host_mut(&mut self) -> Option<&mut SearchHost> {
        match self {
            Self::Search(host) => Some(host),
            Self::Legacy(_) => None,
        }
    }

    pub(super) fn owns_native_key(&self, key: tekes_ui_platform_macos::AccessibilityKey) -> bool {
        match self {
            Self::Legacy(_) => false,
            Self::Search(host) => host.owns_native_key(key),
        }
    }

    pub(super) fn native_action(
        &mut self,
        key: tekes_ui_platform_macos::AccessibilityKey,
        action: tekes_ui_platform_macos::AccessibilityAction,
    ) -> tekes_ui_controls::composition::ContentResponse {
        match self {
            Self::Legacy(_) => Default::default(),
            Self::Search(host) => host.native_action(key, action),
        }
    }

    pub(super) fn query(&self) -> &str {
        match self {
            Self::Legacy(demo) => demo.query(),
            Self::Search(host) => host.query(),
        }
    }

    pub(super) fn focus_field(&mut self) {
        match self {
            Self::Legacy(demo) => demo.focus_field(),
            Self::Search(host) => host.focus_field(),
        }
    }

    pub(super) fn blur(&mut self) {
        match self {
            Self::Legacy(demo) => demo.blur(),
            Self::Search(host) => host.blur(),
        }
    }

    pub(super) fn frame_at(&self, index: usize) -> Option<Rect> {
        match self {
            Self::Legacy(demo) => demo.frame_at(index),
            Self::Search(host) => (index == 0).then(|| host.frame_at(0)).flatten(),
        }
    }

    pub(super) fn visible_frame(&self) -> Option<Rect> {
        match self {
            Self::Legacy(demo) => demo.visible_frame(),
            Self::Search(host) => host.visible_frame(),
        }
    }

    pub(super) fn state(&self) -> Option<TextInputState> {
        match self {
            Self::Legacy(demo) => demo.state(),
            Self::Search(host) => host.state(),
        }
    }

    pub(super) fn set_window_active(&mut self, active: bool) {
        match self {
            Self::Legacy(demo) => demo.set_window_active(active),
            Self::Search(host) => host.set_window_active(active),
        }
    }

    pub(super) fn set_disabled(&mut self, disabled: bool) {
        match self {
            Self::Legacy(demo) => demo.set_disabled(disabled),
            Self::Search(host) => host.set_disabled(disabled),
        }
    }

    pub(super) fn set_error(&mut self, error: bool) {
        match self {
            Self::Legacy(demo) => demo.set_error(error),
            Self::Search(host) => host.set_error(error),
        }
    }

    pub(super) fn set_value(&mut self, value: &str) {
        match self {
            Self::Legacy(demo) => demo.set_value(value),
            Self::Search(host) => host.set_value(value),
        }
    }

    pub(super) fn use_fixture_cache(&mut self) {
        match self {
            Self::Legacy(demo) => demo.use_fixture_cache(),
            Self::Search(host) => host.use_fixture_cache(),
        }
    }

    pub(super) fn set_reduced_motion(&mut self, reduced: bool) -> bool {
        match self {
            Self::Legacy(demo) => demo.set_reduced_motion(reduced),
            Self::Search(host) => host.set_reduced_motion(reduced),
        }
    }

    pub(super) fn needs_animation(&self) -> bool {
        match self {
            Self::Legacy(demo) => demo.needs_animation(),
            Self::Search(host) => host.needs_animation(),
        }
    }

    pub(super) fn animate(&mut self, dt: f32) -> bool {
        match self {
            Self::Legacy(demo) => demo.animate(dt),
            Self::Search(host) => host.animate(dt),
        }
    }

    pub(super) fn refresh(&mut self, text: &mut TextEngine) {
        match self {
            Self::Legacy(demo) => demo.refresh(text),
            Self::Search(host) => host.refresh(text),
        }
    }

    pub(super) fn measure_search_height(&mut self, text: &mut TextEngine) -> f32 {
        match self {
            Self::Legacy(demo) => demo.measure_search_height(text),
            Self::Search(host) => host.measure_search_height(text),
        }
    }

    pub(super) fn build_search_clipped(
        &mut self,
        ctx: &mut PageCtx<'_>,
        rect: Rect,
        clip: Option<Rect>,
        scene: &mut Scene,
    ) {
        self.build_search_clipped_with_brand(ctx, rect, clip, scene, 0);
    }

    pub(super) fn build_search_clipped_with_brand(
        &mut self,
        ctx: &mut PageCtx<'_>,
        rect: Rect,
        clip: Option<Rect>,
        scene: &mut Scene,
        brand: usize,
    ) {
        match self {
            Self::Legacy(demo) => demo.build_search_clipped(ctx, rect, clip, scene),
            Self::Search(host) => host.build_search_clipped(ctx, rect, clip, scene, brand),
        }
    }

    #[cfg(test)]
    pub(super) fn handle(&mut self, event: &Event, point: Option<Point>) -> Option<bool> {
        match self {
            Self::Legacy(demo) => demo.handle(event, point),
            Self::Search(_) => None,
        }
    }

    pub(super) fn handle_with_text(
        &mut self,
        event: &Event,
        point: Option<Point>,
        text: &mut TextEngine,
    ) -> Option<bool> {
        match self {
            Self::Legacy(demo) => demo.handle(event, point),
            Self::Search(host) => host.handle(event, point, text),
        }
    }

    pub(super) fn accessibility_nodes(
        &self,
        owner: u64,
        label: &str,
    ) -> Vec<tekes_ui_platform_macos::AccessibilityNode> {
        match self {
            Self::Legacy(demo) => demo.accessibility_nodes(owner, label),
            Self::Search(host) => host.native_nodes(),
        }
    }

    pub(super) fn accessibility_action(
        &mut self,
        id: usize,
        action: &tekes_ui_platform_macos::AccessibilityAction,
    ) -> bool {
        match self {
            Self::Legacy(demo) => demo.accessibility_action(id, action),
            Self::Search(_) => false,
        }
    }

    #[cfg(test)]
    pub(super) fn scroll_metrics(&self) -> (f32, f32) {
        match self {
            Self::Legacy(demo) => demo.scroll_metrics(),
            Self::Search(_) => (0., 0.),
        }
    }

    #[cfg(test)]
    pub(super) fn scroll_to(&mut self, offset: f32) {
        if let Self::Legacy(demo) = self {
            demo.scroll_to(offset);
        }
    }

    pub(super) fn drain_search_logs(&mut self) -> Vec<String> {
        match self {
            Self::Legacy(_) => Vec::new(),
            Self::Search(host) => host.drain_logs(),
        }
    }
}
