//! Shared Settings schema, drafts and optional local persistence / UI forms.
//! Original shared forms compose the accepted public controls; AppHost integration is separate.
mod error;
#[cfg(feature = "ui")]
mod form;
mod model;
#[cfg(feature = "ui")]
mod sidebar;
#[cfg(feature = "fs-store")]
mod store;

pub use error::{Result, SettingsError};
#[cfg(feature = "ui")]
pub use form::{FormIntent, SettingsForm};
pub use model::{
    Draft, Field, FieldPresentation, FieldType, Group, Item, OptionValue, Page, Schema, Snapshot,
    Submission, Table, Value,
};
#[cfg(feature = "ui")]
pub use sidebar::SettingsSidebar;
#[cfg(feature = "fs-store")]
pub use store::{CommitFailure, FileProvider};
