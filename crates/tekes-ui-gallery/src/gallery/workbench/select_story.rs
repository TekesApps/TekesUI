//! Workbench transport for the original SelectContent/ContentHost owners.
use tekes_ui_controls::{Select, composition::*};
const SELECT: ComponentId = ComponentId(8901);
#[derive(Debug)]
pub(super) struct SelectStoryHost {
    pub(super) host: ContentHost,
}
impl Default for SelectStoryHost {
    fn default() -> Self {
        Self::new()
    }
}
impl SelectStoryHost {
    pub(super) fn new() -> Self {
        Self::with_root(std::convert::identity)
    }
    pub(super) fn with_root(root: fn(Content) -> Content) -> Self {
        let mut host = ContentHost::new(root(Content::new(
            SELECT,
            SelectContent::new(
                "Selection",
                vec!["Automatic".into(), "Compact".into(), "Comfortable".into()],
            ),
        )))
        .expect("fixed Select story");
        host.set_action_events(true);
        host.set_focus_wrap(false);
        Self { host }
    }
    fn owner(&self) -> &SelectContent {
        self.host
            .get(SELECT)
            .unwrap()
            .component::<SelectContent>()
            .unwrap()
    }
    fn owner_mut(&mut self) -> &mut SelectContent {
        self.host
            .get_mut(SELECT)
            .unwrap()
            .component_mut::<SelectContent>()
            .unwrap()
    }
    pub(super) fn model(&self) -> &Select {
        self.owner().model()
    }
    pub(super) fn set_labels(&mut self, labels: Vec<String>) -> Result<bool, SelectUpdateError> {
        self.owner_mut().set_labels(labels)
    }
    pub(super) fn set_disabled(&mut self, disabled: bool) {
        self.owner_mut().set_disabled(disabled)
    }
    pub(super) fn set_error(&mut self, error: bool) {
        self.owner_mut().set_error(error)
    }
    pub(super) fn focus(&mut self, visible: bool) {
        self.host.focus(Some(SELECT), visible);
    }
    pub(super) fn set_window_active(&mut self, active: bool) {
        self.host.set_window_active(active);
    }
    pub(super) fn open(&mut self) {
        self.host
            .accessibility_action(SELECT, None, AccessibilityAction::Press);
    }
}
