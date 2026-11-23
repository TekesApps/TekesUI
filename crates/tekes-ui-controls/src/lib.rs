//! Rust-owned scrolling and text state; platform events and drawing are adapters.
pub mod collections_view;
mod composite;
pub mod composition;
pub use composition::{Button, ButtonActivationPhase, ButtonContents, ButtonImagePosition};
mod control_motion;
mod draft;
pub mod feedback_view;
pub mod layout;
pub mod layout_view;
mod reading;
pub use draft::{DraftJournal, DraftRevision, DraftSubmission};
mod scroll_view;
mod text_input;
pub use layout::{ContainerLayout, Insets, ScrollGeometry};
pub use reading::{
    ReadingAnchorUpdate, ReadingController, ReadingDocumentVersion, ReadingSelectionUpdate,
    ReadingUpdate, ReadingUpdateError, TextRangeMapping,
};
pub use scroll_view::{
    ScrollPolicy, ScrollRevealError, ScrollRevealOptions, ScrollRevealPlacement,
    ScrollRevealResult, ScrollView as ScrollArea,
};
mod selection;
mod text_surface;
mod toolbar;
pub use text_input::{
    ExternalSelection, ExternalUndo, ExternalValueConflict, ExternalValueResult, TextEditBaseline,
    TextInput,
};
pub use text_surface::{
    ReflowAnchor, TextEditCommand, TextSurface, TextSurfaceAlignment, TextSurfaceHost,
    TextSurfacePresentation, TextSurfaceTextRenderer,
};
pub use toolbar::{ToolbarAction, ToolbarControl, ToolbarIconAction};
mod widgets;
pub use widgets::{
    ButtonFramePolicy, ButtonLayout, ButtonModel, ButtonSize, ButtonTextAlign, ButtonTextRenderer,
    CheckState, Checkbox, CheckboxLayout, ControlKey, ControlMotionContext, ControlState, Dialog,
    Field, Menu, NumberField, Select, Tabs,
};
mod paint;
pub use paint::{ButtonTone, ButtonVariant, SurfaceKind};
pub use widgets::Toolbar;
pub(crate) use widgets::button_typography_scale;
mod chart;
pub mod extended;
pub use chart::{
    BarChart, BarGeometry, ChartDataUpdateError, ChartDatum, ChartGeometry, paint_bar_chart,
    paint_bar_chart_with_text_scale,
};

mod ticking_number;
pub use ticking_number::{TickingNumber, TickingNumberError};
