//! Workbench-only facade: Search, Field/Area and Combo use ContentHost; other editors stay on TextDemo.
use super::super::text_demo::TextDemo;
use super::search_host::StoryTextHost;
use tekes_ui_platform_macos::TextInputState;

#[derive(Debug)]
pub(in crate::gallery::workbench) enum StoryInput {
    Legacy(Box<TextDemo>),
    Search(Box<StoryTextHost>),
    Field(Box<StoryTextHost>),
    Combo(Box<StoryTextHost>),
}

impl From<TextDemo> for StoryInput {
    fn from(value: TextDemo) -> Self {
        Self::Legacy(Box::new(value))
    }
}

impl StoryInput {
    pub(super) fn search_with_root(
        root: fn(
            tekes_ui_controls::composition::Content,
        ) -> tekes_ui_controls::composition::Content,
    ) -> Self {
        Self::Search(Box::new(StoryTextHost::with_root(root)))
    }

    pub(super) fn combo_box(
        root: fn(
            tekes_ui_controls::composition::Content,
        ) -> tekes_ui_controls::composition::Content,
    ) -> Self {
        Self::Combo(Box::new(StoryTextHost::combo_box(root)))
    }
    pub(super) fn combo(&self) -> &tekes_ui_controls::composition::ComboBoxContent {
        self.hosted().expect("Combo host").combo()
    }
    pub(super) fn field(
        value: &str,
        multiline: bool,
        root: fn(
            tekes_ui_controls::composition::Content,
        ) -> tekes_ui_controls::composition::Content,
    ) -> Self {
        Self::Field(Box::new(StoryTextHost::text_field(value, multiline, root)))
    }
    pub(super) fn hosted(&self) -> Option<&StoryTextHost> {
        match self {
            Self::Search(host) | Self::Field(host) | Self::Combo(host) => Some(host),
            Self::Legacy(_) => None,
        }
    }
    pub(super) fn hosted_mut(&mut self) -> Option<&mut StoryTextHost> {
        match self {
            Self::Search(host) | Self::Field(host) | Self::Combo(host) => Some(host),
            Self::Legacy(_) => None,
        }
    }

    pub(super) fn query(&self) -> &str {
        match self {
            Self::Legacy(demo) => demo.query(),
            Self::Search(host) | Self::Field(host) | Self::Combo(host) => host.query(),
        }
    }

    pub(super) fn blur(&mut self) {
        match self {
            Self::Legacy(demo) => demo.blur(),
            Self::Search(host) | Self::Field(host) | Self::Combo(host) => host.blur(),
        }
    }

    pub(super) fn state(&self) -> Option<TextInputState> {
        match self {
            Self::Legacy(demo) => demo.state(),
            Self::Search(host) | Self::Field(host) | Self::Combo(host) => host.state(),
        }
    }

    pub(super) fn set_window_active(&mut self, active: bool) {
        match self {
            Self::Legacy(demo) => demo.set_window_active(active),
            Self::Search(host) | Self::Field(host) | Self::Combo(host) => {
                host.set_window_active(active)
            }
        }
    }

    pub(super) fn set_disabled(&mut self, disabled: bool) {
        match self {
            Self::Legacy(demo) => demo.set_disabled(disabled),
            Self::Search(host) | Self::Field(host) | Self::Combo(host) => {
                host.set_disabled(disabled)
            }
        }
    }

    pub(super) fn set_error(&mut self, error: bool) {
        match self {
            Self::Legacy(demo) => demo.set_error(error),
            Self::Search(host) | Self::Field(host) | Self::Combo(host) => host.set_error(error),
        }
    }

    pub(super) fn set_value(&mut self, value: &str) {
        match self {
            Self::Legacy(demo) => demo.set_value(value),
            Self::Search(host) | Self::Field(host) | Self::Combo(host) => host.set_value(value),
        }
    }
}
