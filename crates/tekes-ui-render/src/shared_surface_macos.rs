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
    MTLDevice, MTLOrigin, MTLPixelFormat, MTLRegion, MTLSize, MTLStorageMode, MTLTexture,
    MTLTextureDescriptor, MTLTextureType, MTLTextureUsage,
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

/// Transport the exact independent-layer pixels, retaining their extent.
/// No Scene is redrawn and no parent-boundary proof is asserted here.
pub fn create_from_layer_frame(
    context: &GpuContext,
    output: &crate::LayerFrameOutput,
) -> Result<(wgpu::Texture, CFRetained<IOSurfaceRef>)> {
    create_from_rgba8(context, output.dimensions_px(), output.rgba8())
}

/// Create an immutable BGRA8 surface from an exact RGBA8 pixel buffer.
/// This validates the buffer extent and swizzles R/B without changing alpha;
/// accepting a byte slice does not establish parent-boundary proof authority.
pub fn create_from_rgba8(
    context: &GpuContext,
    size: (u32, u32),
    pixels: &[u8],
) -> Result<(wgpu::Texture, CFRetained<IOSurfaceRef>)> {
    ensure!(
        (1..=4096).contains(&size.0) && (1..=4096).contains(&size.1),
        "invalid shared pixel dimensions"
    );
    let row_bytes = usize::try_from(size.0)?
        .checked_mul(4)
        .ok_or_else(|| anyhow!("shared pixel row overflow"))?;
    let byte_len = row_bytes
        .checked_mul(usize::try_from(size.1)?)
        .ok_or_else(|| anyhow!("shared pixel size overflow"))?;
    ensure!(pixels.len() == byte_len, "invalid RGBA8 pixel length");
    let bgra = rgba_to_bgra(pixels);

    let row = row_bytes.div_ceil(256) * 256;
    let values = [
        CFNumber::new_i64(i64::from(size.0)),
        CFNumber::new_i64(i64::from(size.1)),
        CFNumber::new_i64(4),
        CFNumber::new_i64(row as i64),
        CFNumber::new_i64((row * usize::try_from(size.1)?) as i64),
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
    let region = MTLRegion {
        origin: MTLOrigin { x: 0, y: 0, z: 0 },
        size: MTLSize {
            width: usize::try_from(size.0)?,
            height: usize::try_from(size.1)?,
            depth: 1,
        },
    };
    // SAFETY: `bgra` remains alive and contains exactly width*height BGRA
    // texels for the duration of this synchronous CPU upload.
    unsafe {
        raw.replaceRegion_mipmapLevel_withBytes_bytesPerRow(
            region,
            0,
            std::ptr::NonNull::new(bgra.as_ptr() as *mut std::ffi::c_void)
                .ok_or_else(|| anyhow!("empty BGRA8 pixels"))?,
            row_bytes,
        );
    }
    Ok((wrap(context, raw, size, true), surface))
}

fn rgba_to_bgra(rgba: &[u8]) -> Vec<u8> {
    let mut bgra = rgba.to_vec();
    for pixel in bgra.chunks_exact_mut(4) {
        pixel.swap(0, 2);
    }
    bgra
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

#[cfg(test)]
mod tests {
    use super::rgba_to_bgra;

    #[test]
    fn verified_rgba_readback_is_swizzled_for_bgra_without_alpha_changes() {
        assert_eq!(
            rgba_to_bgra(&[0x13, 0x57, 0xe9, 0x80, 0x22, 0x41, 0xa6, 0x35]),
            [0xe9, 0x57, 0x13, 0x80, 0xa6, 0x41, 0x22, 0x35]
        );
    }
}
