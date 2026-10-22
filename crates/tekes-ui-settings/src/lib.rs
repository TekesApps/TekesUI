//! Experimental shared Settings. No AppHost, platform or Agent business dependency.
mod form;
mod model;
mod sidebar;
mod store;
pub use form::{FormIntent, SettingsForm};
pub use model::{
    Draft, Field, FieldType, Group, Item, OptionValue, Page, Schema, Snapshot, Submission,
};
pub use sidebar::{SelectableListContent, SettingsSidebar};
pub use store::{CommitFailure, FileProvider};
pub use toml::Value;
