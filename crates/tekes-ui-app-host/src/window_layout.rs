use super::*;

pub(super) struct WindowLayoutState {
    pub(super) leading_visible: bool,
    pub(super) trailing_visible: bool,
    pub(super) sidebar_motion: crate::sidebar_motion::SidebarMotion,
    pub(super) motion_epoch: std::time::Instant,
    pub(super) reduced_motion: bool,
    pub(super) outer: SplitView,
    pub(super) inner: SplitView,
    pub(super) outer_view: SplitViewView,
    pub(super) inner_view: SplitViewView,
    pub(super) drag_split: Option<usize>,
    // AppKit starts an asynchronous WindowServer drag and may omit mouse-up.
    // This owns only unclaimed left-drag events, never content captures.
    pub(super) native_titlebar_drag: bool,
    pub(super) pressed: Option<usize>,
    pub(super) focused: usize,
    pub(super) order: Vec<usize>,
    pub(super) presentation: Option<crate::PresentationState>,
    pub(super) presentation_layout: crate::PresentationLayout,
    pub(super) presentation_tabs:
        Option<tekes_ui_controls::composition::ToolbarHost<window_switcher::SwitcherAction>>,
    pub(super) presentation_switcher: window_switcher::SwitcherState,
    pub(super) presentation_scroll_pending: bool,
    pub(super) presentation_icons: tekes_ui_controls::composition::SvgIconCache,
    pub(super) presentation_chrome_edges: [Rect; 2],
    pub(super) native_toolbar_geometry: Option<(f32, f32, f32, f32)>,
    pub(super) container_window_boundary: Option<f32>,
}

impl WindowLayoutState {
    pub(super) fn new(n: usize) -> Self {
        Self {
            leading_visible: n != 3,
            trailing_visible: true,
            sidebar_motion: crate::sidebar_motion::SidebarMotion::new(n != 3, true),
            motion_epoch: std::time::Instant::now(),
            reduced_motion: false,
            outer: SplitView::new(1. / 3.),
            inner: SplitView::new(2. / 3.),
            outer_view: SplitViewView::new(Axis::Horizontal),
            inner_view: SplitViewView::new(Axis::Horizontal),
            drag_split: None,
            native_titlebar_drag: false,
            pressed: None,
            focused: if n >= 2 { 1 } else { 0 },
            order: (0..n).collect(),
            presentation: None,
            presentation_layout: crate::PresentationLayout::default(),
            presentation_tabs: None,
            presentation_switcher: window_switcher::SwitcherState::default(),
            presentation_scroll_pending: false,
            presentation_icons: Default::default(),
            presentation_chrome_edges: [Rect::default(); 2],
            native_toolbar_geometry: None,
            container_window_boundary: None,
        }
    }
}
