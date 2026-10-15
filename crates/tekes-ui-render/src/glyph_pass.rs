//! 字形的 GPU 管线、实例数据，以及 atlas 在 GPU 上的镜像（一张 2D 数组纹理）。
//!
//! # 纹理布局：一张可增长的 `texture_2d_array`
//!
//! 每个 atlas 页是数组的一层。选它而不是"每页一张纹理"的原因：
//!
//! - 实例里带页号，一个 draw 能画任意页的字形；每页一张纹理的话，换页就得换 bind group
//!   并切断 draw，而字形的叠放顺序（painter's order）要求不能按页重排，
//!   最坏情况是每个字形一次 draw；
//! - 只要一个 bind group，管线布局固定；
//! - 与参照实现（Metal 版）的布局一致，便于以后对比 atlas 内存。
//!
//! 代价是层数要事先定。这里按 2 的幂增长（1、2、4 ... 直到 `max_pages`）：
//! 需要更多层时新建一张纹理，把 CPU 页缓冲整页重新上传（CPU 页是真值，不必从旧纹理拷贝），
//! 增长事件至多 log2(max_pages) 次。

use crate::common::instanced_pipeline;
use bytemuck::{Pod, Zeroable};
use std::mem::size_of;
use tekes_ui_scene::{GlyphAtlas, RasterFormat};

/// 一个字形的实例数据，48 字节，与 `glyph.wgsl` 的顶点属性一一对应。
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub(crate) struct GlyphInstance {
    /// 目标：x, y, width, height，设备像素，整数值。
    pub dest: [f32; 4],
    /// atlas：x, y, 页号, 模式（0 mask，1 彩色）。
    pub src: [u32; 4],
    /// 非预乘 sRGB RGBA。
    pub color: [f32; 4],
}

impl GlyphInstance {
    /// 由字形原点（pt）和 scale 算出实例。位图原点取 `round(origin * scale) + (left, top)`，
    /// `round` 是四舍五入、0.5 向远离 0 的方向（Rust 的 `f32::round`）。
    /// 原点不是有限数时返回 `None`。
    pub fn new(
        origin: tekes_ui_scene::Point,
        glyph: &tekes_ui_scene::AtlasGlyph,
        color: tekes_ui_tokens::Rgba,
        scale: f32,
    ) -> Option<GlyphInstance> {
        let ox = (origin.x * scale).round();
        let oy = (origin.y * scale).round();
        if !(ox.is_finite() && oy.is_finite()) {
            return None;
        }
        let mode = match glyph.format {
            RasterFormat::Mask => 0,
            RasterFormat::Color => 1,
        };
        Some(GlyphInstance {
            dest: [
                ox + glyph.left as f32,
                oy + glyph.top as f32,
                glyph.width as f32,
                glyph.height as f32,
            ],
            src: [glyph.x, glyph.y, glyph.page, mode],
            color: [color.r, color.g, color.b, color.a],
        })
    }
}

/// atlas 在 GPU 上的镜像。
pub(crate) struct AtlasTexture {
    texture: wgpu::Texture,
    pub bind_group: wgpu::BindGroup,
    layers: u32,
    page_size: u32,
    atlas_id: u64,
}

impl AtlasTexture {
    pub fn layers(&self) -> u32 {
        self.layers
    }
}

pub(crate) struct GlyphPass {
    pub pipeline: wgpu::RenderPipeline,
    atlas_layout: wgpu::BindGroupLayout,
    pub atlas: Option<AtlasTexture>,
    /// 渲染器编号，atlas 据此区分是谁取走了脏区。
    consumer: u64,
    /// 数组纹理重建过多少次，测试用。
    texture_creations: usize,
    pub staging: Vec<GlyphInstance>,
    pub instances: crate::common::GrowBuffer<GlyphInstance>,
}

const MIN_GLYPH_CAPACITY: usize = 1024;

impl GlyphPass {
    pub fn new(
        device: &wgpu::Device,
        globals_layout: &wgpu::BindGroupLayout,
        consumer: u64,
    ) -> GlyphPass {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("tekes glyph shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("glyph.wgsl").into()),
        });
        let atlas_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("tekes atlas layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    // 只用 textureLoad，不需要可过滤。
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    view_dimension: wgpu::TextureViewDimension::D2Array,
                    multisampled: false,
                },
                count: None,
            }],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("tekes glyph pipeline layout"),
            bind_group_layouts: &[Some(globals_layout), Some(&atlas_layout)],
            immediate_size: 0,
        });
        let attributes = wgpu::vertex_attr_array![
            0 => Float32x4,
            1 => Uint32x4,
            2 => Float32x4,
        ];
        GlyphPass {
            pipeline: instanced_pipeline(
                device,
                "tekes glyph pipeline",
                &shader,
                &layout,
                size_of::<GlyphInstance>() as u64,
                &attributes,
            ),
            atlas_layout,
            atlas: None,
            consumer,
            texture_creations: 0,
            staging: Vec::new(),
            instances: crate::common::GrowBuffer::new(
                device,
                "tekes glyph instances",
                MIN_GLYPH_CAPACITY,
            ),
        }
    }

    pub fn texture_creations(&self) -> usize {
        self.texture_creations
    }

    fn create_texture(&mut self, device: &wgpu::Device, atlas: &GlyphAtlas) {
        let pages = atlas.page_count().max(1);
        let layers = pages
            .next_power_of_two()
            .min(atlas.config().max_pages)
            .max(pages);
        let size = atlas.page_size();
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("tekes glyph atlas"),
            size: wgpu::Extent3d {
                width: size,
                height: size,
                depth_or_array_layers: layers,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: crate::common::COLOR_FORMAT,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("tekes atlas"),
            layout: &self.atlas_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            }],
        });
        self.atlas = Some(AtlasTexture {
            texture,
            bind_group,
            layers,
            page_size: size,
            atlas_id: atlas.id(),
        });
        self.texture_creations += 1;
    }

    /// 让 GPU 纹理与 atlas 的 CPU 页一致：必要时（首次、换了 atlas、页数超过层数）
    /// 重建纹理，然后上传脏区。上传用 `Queue::write_texture`，在下一次 `submit`
    /// 时排在该次提交所有命令缓冲的最前面，见 `tekes_ui_scene` 的 atlas 文档。
    pub fn sync(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, atlas: &mut GlyphAtlas) {
        let pages = atlas.page_count();
        if pages == 0 {
            return;
        }
        let stale = match &self.atlas {
            None => true,
            Some(t) => {
                t.atlas_id != atlas.id() || t.page_size != atlas.page_size() || t.layers < pages
            }
        };
        if stale {
            self.create_texture(device, atlas);
        }
        let Some(gpu) = &self.atlas else {
            return;
        };
        let size = atlas.page_size() as usize;
        for r in atlas.take_dirty(self.consumer, stale) {
            let Some(pixels) = atlas.page_pixels(r.page) else {
                continue;
            };
            let offset = (r.y as usize * size + r.x as usize) * 4;
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &gpu.texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: r.x,
                        y: r.y,
                        z: r.page,
                    },
                    aspect: wgpu::TextureAspect::All,
                },
                &pixels[offset..],
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(size as u32 * 4),
                    rows_per_image: Some(r.height),
                },
                wgpu::Extent3d {
                    width: r.width,
                    height: r.height,
                    depth_or_array_layers: 1,
                },
            );
        }
    }
}
