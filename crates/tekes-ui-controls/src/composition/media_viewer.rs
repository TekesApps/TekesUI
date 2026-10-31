//! Optional viewport over the original ImageContent resource and load owner.
use super::*;
mod geometry;
use geometry::Geometry;

/// Fit tracks the viewport; Manual preserves its relative scale on resize.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MediaViewerMode {
    Fit,
    Manual,
}

/// Image geometry in viewport-local logical points, independent of backing pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MediaViewerTransform {
    pub relative_scale: f64,
    pub scale: f64,
    pub translation: Point,
    pub mode: MediaViewerMode,
}

/// All buttons and input adapters call these same original-owner operations.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MediaViewerCommand {
    Fit,
    ActualLogicalSize,
    ZoomIn,
    ZoomOut,
    ZoomAt { relative_scale: f64, anchor: Point },
    PanBy { delta: Point },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MediaViewerError {
    InvalidImageSize,
    InvalidTransform,
    InvalidName,
    InvalidImageChild,
    Image(ImageContentError),
}
impl std::fmt::Display for MediaViewerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid media viewer: {self:?}")
    }
}
impl std::error::Error for MediaViewerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Image(error) => Some(error),
            _ => None,
        }
    }
}
impl From<ImageContentError> for MediaViewerError {
    fn from(error: ImageContentError) -> Self {
        Self::Image(error)
    }
}

// Content stays in its independently owned source file, under this module
// so the private geometry owner is shared without exporting another API.
mod content;
pub use content::MediaViewerContent;
