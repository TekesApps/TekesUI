//! Host-side acquisition of an out-of-band surface right and local GPU import.
use anyhow::{Result, anyhow};
use objc2_io_surface::IOSurfaceRef;
use tekes_ui_render::{gpu::GpuContext, shared_surface_macos};
pub fn import(
    context: &GpuContext,
    size: (u32, u32),
    port: u32,
    registry: u64,
) -> Result<wgpu::Texture> {
    let right = unsafe { crate::transport::OwnedSendRight::from_raw(port) };
    import_owned(context, size, right, registry)
}
pub(crate) fn import_owned(
    context: &GpuContext,
    size: (u32, u32),
    right: crate::transport::OwnedSendRight,
    registry: u64,
) -> Result<wgpu::Texture> {
    let surface = IOSurfaceRef::lookup_from_mach_port(right.name());
    drop(right);
    let surface = surface.ok_or_else(|| anyhow!("lookup received IOSurface right failed"))?;
    shared_surface_macos::import(context, size, &surface, registry)
}
