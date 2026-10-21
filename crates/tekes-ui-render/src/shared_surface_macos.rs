//! Opt-in macOS native shared-surface allocation/import. No Host dependencies.
use crate::gpu::GpuContext;
use anyhow::{Result, anyhow, ensure};
use objc2::{rc::Retained, runtime::ProtocolObject};
use objc2_core_foundation::{CFDictionary, CFNumber, CFRetained};
use objc2_io_surface::{
    IOSurfaceRef, kIOSurfaceAllocSize, kIOSurfaceBytesPerElement, kIOSurfaceBytesPerRow,
    kIOSurfaceHeight, kIOSurfacePixelFormat, kIOSurfaceWidth,
};
use objc2_metal::{
    MTLDevice, MTLPixelFormat, MTLStorageMode, MTLTexture, MTLTextureDescriptor, MTLTextureType,
    MTLTextureUsage,
};

fn metal(context: &GpuContext) -> Result<Retained<ProtocolObject<dyn MTLDevice>>> {
    // SAFETY: the context was created by the Metal-only renderer; retained device
    // stays alive after the HAL guard drops. No HAL mutation is performed.
    let hal = unsafe { context.device().as_hal::<wgpu_hal::api::Metal>() }
        .ok_or_else(|| anyhow!("not a Metal device"))?;
    Ok(hal.raw_device().clone())
}
pub fn registry(context: &GpuContext) -> Result<u64> {
    Ok(metal(context)?.registryID())
}
fn wrap(
    context: &GpuContext,
    raw: Retained<ProtocolObject<dyn MTLTexture>>,
    size: (u32, u32),
    initialized: bool,
) -> wgpu::Texture {
    let extent = wgpu::Extent3d {
        width: size.0,
        height: size.1,
        depth_or_array_layers: 1,
    };
    let desc = wgpu::TextureDescriptor {
        label: Some("apphost imported shared Metal texture"),
        size: extent,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Bgra8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    };
    // SAFETY: raw was created/imported on this exact local Metal device, retained
    // by the HAL texture, with the descriptor below. BGRA, D2, one mip/layer.
    unsafe {
        let hal = wgpu_hal::metal::Device::texture_from_raw(
            raw,
            wgpu::TextureFormat::Bgra8Unorm,
            MTLTextureType::Type2D,
            1,
            1,
            extent.into(),
            None,
        );
        context
            .device()
            .create_texture_from_hal::<wgpu_hal::api::Metal>(
                hal,
                &desc,
                if initialized {
                    wgpu::TextureUses::RESOURCE
                } else {
                    wgpu::TextureUses::UNINITIALIZED
                },
            )
    }
}
fn descriptor(size: (u32, u32)) -> MTLDescriptor {
    let d = MTLTextureDescriptor::new();
    d.setTextureType(MTLTextureType::Type2D);
    d.setPixelFormat(MTLPixelFormat::BGRA8Unorm);
    // SAFETY: dimensions are validated by worker protocol before allocation.
    unsafe {
        d.setWidth(size.0 as usize);
        d.setHeight(size.1 as usize);
    }
    d.setStorageMode(MTLStorageMode::Shared);
    d.setUsage(MTLTextureUsage::RenderTarget | MTLTextureUsage::ShaderRead);
    d
}
type MTLDescriptor = Retained<MTLTextureDescriptor>;
pub fn create(
    context: &GpuContext,
    size: (u32, u32),
) -> Result<(wgpu::Texture, CFRetained<IOSurfaceRef>)> {
    let row = (size.0 as usize * 4).div_ceil(256) * 256;
    let values = [
        CFNumber::new_i64(size.0 as i64),
        CFNumber::new_i64(size.1 as i64),
        CFNumber::new_i64(4),
        CFNumber::new_i64(row as i64),
        CFNumber::new_i64((row * size.1 as usize) as i64),
        CFNumber::new_i64(u32::from_be_bytes(*b"BGRA") as i64),
    ];
    // SAFETY: all keys are IOSurface numeric properties and values are CFNumbers.
    let keys = unsafe {
        [
            kIOSurfaceWidth,
            kIOSurfaceHeight,
            kIOSurfaceBytesPerElement,
            kIOSurfaceBytesPerRow,
            kIOSurfaceAllocSize,
            kIOSurfacePixelFormat,
        ]
    };
    let refs: Vec<_> = values.iter().map(|v| &**v).collect();
    let properties = CFDictionary::from_slices(&keys, &refs);
    let surface = unsafe { IOSurfaceRef::new(properties.as_opaque()) }
        .ok_or_else(|| anyhow!("IOSurface allocation failed"))?;
    let raw = metal(context)?
        .newTextureWithDescriptor_iosurface_plane(&descriptor(size), &surface, 0)
        .ok_or_else(|| anyhow!("IOSurface Metal texture failed"))?;
    Ok((wrap(context, raw, size, false), surface))
}
pub fn import(
    context: &GpuContext,
    size: (u32, u32),
    surface: &IOSurfaceRef,
    expected: u64,
) -> Result<wgpu::Texture> {
    ensure!(
        registry(context)? == expected,
        "GPU registry identity mismatch"
    );
    let raw = metal(context)?
        .newTextureWithDescriptor_iosurface_plane(&descriptor(size), surface, 0)
        .ok_or_else(|| anyhow!("import surface failed"))?;
    ensure!(
        raw.width() == size.0 as usize && raw.height() == size.1 as usize,
        "invalid shared size"
    );
    Ok(wrap(context, raw, size, true))
}
