//! Native modules are compiled into the consumer executable. Factories run only
//! in their designated child processes; ordinary Rust objects stay local.
use anyhow::Result;
use tekes_ui_app_protocol::{Request, ToolbarItem};
use tekes_ui_scene::{GlyphAtlas, Scene};
mod panel_image;
mod worker_entry;
pub use panel_image::panel_image_rect;
pub use worker_entry::run_worker_from_args;
#[cfg(target_os = "macos")]
mod parent;
#[cfg(target_os = "macos")]
mod process;
#[cfg(target_os = "macos")]
pub mod transport;
#[cfg(target_os = "macos")]
pub use parent::{ModuleProcess, ReceivedFrame, import_frame};
#[cfg(target_os = "macos")]
pub mod surface;

pub struct ModuleState {
    pub revision: u64,
    pub toolbar: Vec<ToolbarItem>,
}

pub trait PanelModule {
    fn update(
        &mut self,
        input: &Request,
        scene: &mut Scene,
        atlas: &mut GlyphAtlas,
    ) -> Result<ModuleState>;
}

#[derive(Clone, Copy)]
pub struct ModuleDefinition {
    pub id: &'static str,
    pub create: fn() -> Box<dyn PanelModule>,
}

#[cfg(target_os = "macos")]
pub use process::run;
#[cfg(not(target_os = "macos"))]
pub fn run(_: Box<dyn PanelModule>, _: &str) -> Result<()> {
    anyhow::bail!("native module GPU transport currently requires macOS")
}

mod control;
pub use control::ProcessControl;
mod pipes;
mod services;
pub use services::{
    ComponentDefinition, ComponentEmitter, NativeViewOptions, ProcessComponent,
    ProcessComponentOptions, ProcessModule, ProcessStatus, run_component_worker_from_args,
};
pub use tekes_ui_app_protocol::component;

#[cfg(target_os = "macos")]
mod view;
#[cfg(target_os = "macos")]
pub(crate) use view::ViewConnection;
#[cfg(target_os = "macos")]
pub use view::{ViewFrame, ViewImage};

#[cfg(target_os = "macos")]
mod container;
#[cfg(target_os = "macos")]
pub use container::ProcessContainer;

#[cfg(target_os = "macos")]
mod process_view;
