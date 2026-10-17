//! Rust-owned scrolling and text state; platform events and drawing are adapters.
pub mod collections_view;
mod composite;
mod control_motion;
pub mod feedback_view;
pub mod layout;
pub mod layout_view;
mod scroll_view;
mod text_input;
pub use layout::{ContainerLayout, Insets, ScrollGeometry};
pub use scroll_view::{ScrollPolicy, ScrollView as ScrollArea};
mod selection;
mod text_surface;
pub use composite::{
    DialogEvent, DialogLayout, DialogView, MenuEvent, MenuLayout, MenuView, MenuWheelEvent,
    MenuWheelInput, TabsLayout, TabsView,
};
pub use selection::{
    RadioGroupLayout, RadioGroupView, SelectionEvent, SliderLayout, SliderView, SwitchLayout,
    SwitchView,
};
pub use text_input::TextInput;
pub use text_surface::{ReflowAnchor, TextSurface, TextSurfaceTextRenderer};
mod widgets;
pub use widgets::{
    Button, ButtonLayout, ButtonSize, ButtonTextAlign, ButtonTextRenderer, CheckState, Checkbox,
    CheckboxLayout, ControlKey, ControlMotionContext, ControlState, Dialog, Field, Menu,
    NumberField, Select, Tabs,
};
pub mod paint;
pub use widgets::Toolbar;
mod chart;
pub mod extended;
pub use chart::{
    BarChart, BarGeometry, ChartDatum, ChartGeometry, paint_bar_chart,
    paint_bar_chart_with_text_scale,
};
