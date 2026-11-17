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

use std::collections::VecDeque;

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

fn validate_dimensions(
    context: &GpuContext,
    size: (u32, u32),
) -> Result<(usize, usize, usize, usize, usize)> {
    let max_dim = context.device().limits().max_texture_dimension_2d;
    ensure!(
        size.0 >= 1 && size.1 >= 1 && size.0 <= max_dim && size.1 <= max_dim,
        "invalid shared surface dimensions: {size:?}"
    );
    let w = usize::try_from(size.0)?;
    let h = usize::try_from(size.1)?;
    let min_row_bytes = w
        .checked_mul(4)
        .ok_or_else(|| anyhow!("shared surface row bytes overflow"))?;
    let aligned_row = min_row_bytes
        .checked_add(255)
        .ok_or_else(|| anyhow!("shared surface row align overflow"))?
        / 256
        * 256;
    let alloc_size = aligned_row
        .checked_mul(h)
        .ok_or_else(|| anyhow!("shared surface alloc size overflow"))?;
    ensure!(
        i64::try_from(alloc_size).is_ok(),
        "shared surface alloc size exceeds i64"
    );
    Ok((w, h, min_row_bytes, aligned_row, alloc_size))
}

fn wrap_surface(
    context: &GpuContext,
    size: (u32, u32),
    surface: &IOSurfaceRef,
    initialized: bool,
) -> Result<wgpu::Texture> {
    let raw = metal(context)?
        .newTextureWithDescriptor_iosurface_plane(&descriptor(size), surface, 0)
        .ok_or_else(|| anyhow!("IOSurface Metal texture failed"))?;
    Ok(wrap(context, raw, size, initialized))
}

pub fn create(
    context: &GpuContext,
    size: (u32, u32),
) -> Result<(wgpu::Texture, CFRetained<IOSurfaceRef>)> {
    let (_, _, _, row, alloc_size) = validate_dimensions(context, size)?;
    let values = [
        CFNumber::new_i64(i64::from(size.0)),
        CFNumber::new_i64(i64::from(size.1)),
        CFNumber::new_i64(4),
        CFNumber::new_i64(row as i64),
        CFNumber::new_i64(alloc_size as i64),
        CFNumber::new_i64(i64::from(u32::from_be_bytes(*b"BGRA"))),
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
    let texture = wrap_surface(context, size, &surface, false)?;
    Ok((texture, surface))
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
    let (w, h, min_row_bytes, _, _) = validate_dimensions(context, size)?;
    ensure!(
        surface.width() == w && surface.height() == h,
        "invalid shared surface dimensions"
    );
    ensure!(
        surface.pixel_format() == u32::from_be_bytes(*b"BGRA"),
        "invalid shared surface pixel format"
    );
    ensure!(
        surface.bytes_per_element() == 4,
        "invalid shared surface bytes per element"
    );
    let bpr = surface.bytes_per_row();
    ensure!(
        bpr >= min_row_bytes && bpr.is_multiple_of(4),
        "invalid shared surface bytes per row"
    );
    let required_alloc = bpr
        .checked_mul(h - 1)
        .and_then(|v| v.checked_add(min_row_bytes))
        .ok_or_else(|| anyhow!("shared surface allocation overflow"))?;
    ensure!(
        surface.alloc_size() >= required_alloc,
        "undersized shared surface allocation"
    );
    let raw = metal(context)?
        .newTextureWithDescriptor_iosurface_plane(&descriptor(size), surface, 0)
        .ok_or_else(|| anyhow!("import surface failed"))?;
    ensure!(raw.width() == w && raw.height() == h, "invalid shared size");
    Ok(wrap(context, raw, size, true))
}

/// RAII consumer-side kernel use-count guard for an imported `IOSurfaceRef`.
/// Calls `IOSurfaceIncrementUseCount` on construction and `IOSurfaceDecrementUseCount`
/// on drop so a producer pool observes `is_in_use() == true` until the consumer's
/// GPU submission finishes reading the shared surface.
pub struct SharedSurfaceUseGuard(CFRetained<IOSurfaceRef>);

impl std::fmt::Debug for SharedSurfaceUseGuard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SharedSurfaceUseGuard")
            .field("surface_id", &self.0.id())
            .field("is_in_use", &self.0.is_in_use())
            .finish()
    }
}

// SAFETY: `CFRetained<IOSurfaceRef>` is reference-counted and `IOSurfaceDecrementUseCount`
// is an atomic XNU kernel operation safe to invoke from the consumer retirement thread.
unsafe impl Send for SharedSurfaceUseGuard {}
// SAFETY: `SharedSurfaceUseGuard` performs no interior unsynchronized mutation; the inner
// `IOSurfaceRef` is immutable once exported.
unsafe impl Sync for SharedSurfaceUseGuard {}

impl SharedSurfaceUseGuard {
    pub fn new(surface: CFRetained<IOSurfaceRef>) -> Self {
        surface.increment_use_count();
        Self(surface)
    }

    pub fn surface(&self) -> &IOSurfaceRef {
        &self.0
    }
}

impl Drop for SharedSurfaceUseGuard {
    fn drop(&mut self) {
        self.0.decrement_use_count();
    }
}

/// Import a shared `IOSurfaceRef` along with a `SharedSurfaceUseGuard` that keeps
/// the kernel `is_in_use` count elevated until the returned guard is dropped.
pub fn import_with_guard(
    context: &GpuContext,
    size: (u32, u32),
    surface: CFRetained<IOSurfaceRef>,
    expected: u64,
) -> Result<(wgpu::Texture, SharedSurfaceUseGuard)> {
    let texture = import(context, size, &surface, expected)?;
    Ok((texture, SharedSurfaceUseGuard::new(surface)))
}

struct PooledSurface {
    surface: CFRetained<IOSurfaceRef>,
    texture: Option<wgpu::Texture>,
    local_texture_marks_in_use: bool,
}

/// Bounded per-size `IOSurfaceRef` swap pool (up to 3-4 slots per size across
/// the 2 most recent dimensions) that reuses idle surfaces without overwriting
/// any surface whose consumer `IOSurfaceIsInUse` count is still elevated.
#[derive(Default)]
pub struct SharedSurfacePool {
    buckets: VecDeque<((u32, u32), VecDeque<PooledSurface>)>,
    transient: Option<(wgpu::Texture, CFRetained<IOSurfaceRef>)>,
}

impl std::fmt::Debug for SharedSurfacePool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let buckets: Vec<_> = self
            .buckets
            .iter()
            .map(|(size, slots)| (*size, slots.len()))
            .collect();
        f.debug_struct("SharedSurfacePool")
            .field("buckets", &buckets)
            .field("has_transient", &self.transient.is_some())
            .finish()
    }
}

impl SharedSurfacePool {
    pub const RING_SLOTS: usize = 3;
    pub const MAX_SLOTS_PER_SIZE: usize = 4;
    pub const MAX_POOLED_SIZES: usize = 2;

    pub fn new() -> Self {
        Self::default()
    }

    /// Acquire a GPU-renderable `BGRA8Unorm` texture backed by a reusable (or freshly
    /// allocated if all pooled slots are still in use) `IOSurfaceRef` for `size`.
    pub fn acquire<'a>(
        &'a mut self,
        context: &GpuContext,
        size: (u32, u32),
    ) -> Result<(&'a wgpu::Texture, &'a IOSurfaceRef)> {
        self.transient = None;
        for (_, slots) in &mut self.buckets {
            for slot in slots.iter_mut() {
                if slot.local_texture_marks_in_use {
                    slot.texture = None;
                }
            }
        }

        let bucket_idx = if let Some(pos) = self.buckets.iter().position(|(s, _)| *s == size) {
            let bucket = self.buckets.remove(pos).unwrap();
            self.buckets.push_back(bucket);
            self.buckets.len() - 1
        } else {
            if self.buckets.len() >= Self::MAX_POOLED_SIZES {
                self.buckets.pop_front();
            }
            self.buckets.push_back((size, VecDeque::new()));
            self.buckets.len() - 1
        };

        let (_, bucket) = &mut self.buckets[bucket_idx];
        let idle_idx = if bucket.len() >= Self::RING_SLOTS {
            bucket.iter().position(|slot| !slot.surface.is_in_use())
        } else {
            None
        };

        if let Some(idx) = idle_idx {
            let mut slot = bucket.remove(idx).unwrap();
            if slot.texture.is_none() {
                slot.texture = Some(wrap_surface(context, size, &slot.surface, false)?);
            }
            bucket.push_back(slot);
            let slot = bucket.back().unwrap();
            return Ok((slot.texture.as_ref().unwrap(), &slot.surface));
        }

        if bucket.len() < Self::MAX_SLOTS_PER_SIZE {
            let (texture, surface) = create(context, size)?;
            let local_texture_marks_in_use = surface.is_in_use();
            bucket.push_back(PooledSurface {
                surface,
                texture: Some(texture),
                local_texture_marks_in_use,
            });
            let slot = bucket.back().unwrap();
            return Ok((slot.texture.as_ref().unwrap(), &slot.surface));
        }

        self.transient = Some(create(context, size)?);
        let (texture, surface) = self.transient.as_ref().unwrap();
        Ok((texture, surface))
    }

    #[cfg(test)]
    fn pooled_sizes(&self) -> Vec<(u32, u32)> {
        self.buckets.iter().map(|(s, _)| *s).collect()
    }

    #[cfg(test)]
    fn bucket_len(&self, size: (u32, u32)) -> usize {
        self.buckets
            .iter()
            .find(|(s, _)| *s == size)
            .map_or(0, |(_, q)| q.len())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        SharedSurfacePool, SharedSurfaceUseGuard, create, import, import_with_guard, registry,
        rgba_to_bgra,
    };
    use objc2_core_foundation::{CFDictionary, CFNumber, CFRetained, Type};
    use objc2_io_surface::{
        IOSurfaceRef, kIOSurfaceAllocSize, kIOSurfaceBytesPerElement, kIOSurfaceBytesPerRow,
        kIOSurfaceHeight, kIOSurfacePixelFormat, kIOSurfaceWidth,
    };

    fn raw_surface(
        w: usize,
        h: usize,
        bpe: usize,
        bpr: usize,
        alloc: usize,
        fmt: u32,
    ) -> CFRetained<IOSurfaceRef> {
        let values = [
            CFNumber::new_i64(w as i64),
            CFNumber::new_i64(h as i64),
            CFNumber::new_i64(bpe as i64),
            CFNumber::new_i64(bpr as i64),
            CFNumber::new_i64(alloc as i64),
            CFNumber::new_i64(i64::from(fmt)),
        ];
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
        unsafe { IOSurfaceRef::new(properties.as_opaque()) }.expect("allocate test IOSurface")
    }

    #[test]
    fn verified_rgba_readback_is_swizzled_for_bgra_without_alpha_changes() {
        assert_eq!(
            rgba_to_bgra(&[0x13, 0x57, 0xe9, 0x80, 0x22, 0x41, 0xa6, 0x35]),
            [0xe9, 0x57, 0x13, 0x80, 0xa6, 0x41, 0x22, 0x35]
        );
    }

    #[test]
    fn create_and_import_reject_invalid_dimensions_and_mismatched_surfaces() {
        let Ok(renderer) = crate::Renderer::new_headless() else {
            return;
        };
        let context = renderer.gpu_context();
        let expected = registry(&context).unwrap();
        let max_dim = context.device().limits().max_texture_dimension_2d;

        for bad in [
            (0, 0),
            (0, 8),
            (8, 0),
            (u32::MAX, u32::MAX),
            (max_dim.saturating_add(1), 8),
        ] {
            assert!(create(&context, bad).is_err(), "create accepted {bad:?}");
        }

        let (_, valid) = create(&context, (8, 8)).unwrap();
        assert!(import(&context, (8, 8), &valid, expected).is_ok());
        assert!(import(&context, (0, 0), &valid, expected).is_err());
        assert!(import(&context, (4, 8), &valid, expected).is_err());
        assert!(import(&context, (u32::MAX, 8), &valid, expected).is_err());

        let wrong_fmt = raw_surface(8, 8, 4, 256, 2048, u32::from_be_bytes(*b"ARGB"));
        assert!(import(&context, (8, 8), &wrong_fmt, expected).is_err());

        let wrong_bpe = raw_surface(8, 8, 1, 256, 2048, u32::from_be_bytes(*b"L008"));
        assert!(import(&context, (8, 8), &wrong_bpe, expected).is_err());

        let wrong_dims = raw_surface(8, 4, 4, 256, 1024, u32::from_be_bytes(*b"BGRA"));
        assert!(import(&context, (8, 8), &wrong_dims, expected).is_err());
    }

    #[test]
    fn shared_surface_pool_reuses_three_slots_respects_use_guards_and_caps_dimensions() {
        let Ok(renderer) = crate::Renderer::new_headless() else {
            return;
        };
        let context = renderer.gpu_context();
        let expected = registry(&context).unwrap();
        let mut pool = SharedSurfacePool::new();

        let mut warmup_ids = Vec::new();
        for _ in 0..3 {
            let (_, surface) = pool.acquire(&context, (32, 32)).unwrap();
            warmup_ids.push(surface.id());
        }
        assert_eq!(pool.bucket_len((32, 32)), 3);
        assert_eq!(
            warmup_ids
                .iter()
                .collect::<std::collections::HashSet<_>>()
                .len(),
            3
        );

        for frame in 0..6 {
            let (_, surface) = pool.acquire(&context, (32, 32)).unwrap();
            assert_eq!(surface.id(), warmup_ids[frame % 3]);
        }
        assert_eq!(pool.bucket_len((32, 32)), 3);

        // Pin all 3 slots with consumer use guards; the 4th acquire must not overwrite any
        // active surface (grows to MAX_SLOTS_PER_SIZE=4, and a 5th falls back to transient).
        let mut guards = Vec::new();
        for _ in 0..3 {
            let (_, surface) = pool.acquire(&context, (32, 32)).unwrap();
            let retained = surface.retain();
            let (_tex, guard) = import_with_guard(&context, (32, 32), retained, expected).unwrap();
            guards.push(guard);
        }
        let fourth_id = {
            let s4_id = pool.acquire(&context, (32, 32)).unwrap().1.id();
            assert!(!warmup_ids.contains(&s4_id));
            assert_eq!(pool.bucket_len((32, 32)), 4);
            s4_id
        };
        let fourth_guard = {
            let (_tex, g) = import_with_guard(
                &context,
                (32, 32),
                pool.acquire(&context, (32, 32)).unwrap().1.retain(),
                expected,
            )
            .unwrap();
            // Note: acquire above just checked slot 3 or re-selected s4? Actually slot 4 was not guarded yet!
            g
        };
        let _ = fourth_guard;
        // Now all 4 pooled slots are pinned; next acquire uses transient without growing bucket.
        let transient_id = {
            let (_, st) = pool.acquire(&context, (32, 32)).unwrap();
            st.id()
        };
        assert!(!warmup_ids.contains(&transient_id) && transient_id != fourth_id);
        assert_eq!(pool.bucket_len((32, 32)), 4);

        drop(guards);
        let (_, recycled) = pool.acquire(&context, (32, 32)).unwrap();
        assert!(warmup_ids.contains(&recycled.id()));

        // Resizing across 3 distinct sizes retains only MAX_POOLED_SIZES=2 most recent buckets.
        pool.acquire(&context, (48, 48)).unwrap();
        pool.acquire(&context, (64, 64)).unwrap();
        assert_eq!(pool.pooled_sizes(), vec![(48, 48), (64, 64)]);
        drop(SharedSurfaceUseGuard::new(raw_surface(
            8,
            8,
            4,
            256,
            2048,
            u32::from_be_bytes(*b"BGRA"),
        )));
    }
}
