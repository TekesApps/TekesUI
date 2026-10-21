use tekes_ui_scene::Rect;

/// Mounted, un-clipped logical focus geometry for the current story render.
/// `frame` stays in document/window coordinates even when ancestor clipping
/// removes the target from the current hit-test list.
#[derive(Clone, Copy, Debug)]
pub(super) struct FocusTarget {
    pub id: usize,
    pub frame: Rect,
    pub eligible: bool,
    pub mounted: bool,
    pub roving_group: Option<u8>,
    pub roving_selected: bool,
}
