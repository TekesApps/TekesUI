//! Structured errors for `tekes-ui-settings`.

use std::fmt;

/// Typed errors returned by settings schema parsing, draft state transitions,
/// persistence, and optional UI composition.
#[derive(Debug)]
pub enum SettingsError {
    /// Schema syntax or structural invariant violation.
    InvalidSchema(String),
    /// Field value fails type, enum, or range validation.
    InvalidFieldValue(String),
    /// Invalid draft edit, submit, or receipt reconciliation state.
    DraftState(String),
    /// Expected snapshot revision or commit identity conflict.
    RevisionConflict(String),
    /// Settings file lock is already held or provider mutex is poisoned.
    ProviderLocked(String),
    /// Underlying filesystem I/O failure.
    Storage(std::io::Error),
    /// Serialization/deserialization failure.
    Format(String),
    /// Failure constructing or driving the optional settings UI widgets.
    #[cfg(feature = "ui")]
    Ui(String),
}

/// Result alias using [`SettingsError`].
pub type Result<T> = std::result::Result<T, SettingsError>;

impl fmt::Display for SettingsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSchema(msg)
            | Self::InvalidFieldValue(msg)
            | Self::DraftState(msg)
            | Self::RevisionConflict(msg)
            | Self::ProviderLocked(msg)
            | Self::Format(msg) => f.write_str(msg),
            Self::Storage(err) => fmt::Display::fmt(err, f),
            #[cfg(feature = "ui")]
            Self::Ui(msg) => f.write_str(msg),
        }
    }
}

impl std::error::Error for SettingsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Storage(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for SettingsError {
    fn from(err: std::io::Error) -> Self {
        Self::Storage(err)
    }
}

impl From<toml::de::Error> for SettingsError {
    fn from(err: toml::de::Error) -> Self {
        Self::Format(err.to_string())
    }
}

impl From<toml::ser::Error> for SettingsError {
    fn from(err: toml::ser::Error) -> Self {
        Self::Format(err.to_string())
    }
}
