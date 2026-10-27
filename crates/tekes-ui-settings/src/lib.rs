//! Shared Settings schema, drafts and single-writer local persistence.
//! Existing forms and AppHost integration are received separately.
mod model;
mod store;
pub use model::{Draft, Field, FieldType, Group, Item, OptionValue, Page, Schema, Snapshot, Submission};
pub use store::{CommitFailure, FileProvider};
pub use toml::Value;
