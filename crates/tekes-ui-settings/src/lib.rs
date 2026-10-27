//! Shared Settings schema, drafts and single-writer local persistence.
//! Original shared forms compose the accepted public controls; AppHost integration is separate.
mod form;
mod model;
mod sidebar;
mod store;
pub use form::{FormIntent, SettingsForm};
pub use sidebar::{SelectableListContent, SettingsSidebar};
pub use model::{Draft, Field, FieldType, Group, Item, OptionValue, Page, Schema, Snapshot, Submission};
pub use store::{CommitFailure, FileProvider};
pub use toml::Value;
