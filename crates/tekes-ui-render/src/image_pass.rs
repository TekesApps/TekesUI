//! 图像的 GPU 管线与登记表。
//!
//! 每张图是一张 `Rgba8Unorm` 纹理（输入是预乘 alpha 的 RGBA，数值原样使用，不做颜色转换），
//! 线性采样、边缘钳制。矩形边缘不做抗锯齿：图像用于 Gallery 的参照图，边缘与像素对齐。

use crate::common::{GrowBuffer, instanced_pipeline};
use crate::error::RenderError;
use bytemuck::{Pod, Zeroable};
use std::collections::HashMap;
use std::mem::size_of;
use tekes_ui_scene::{ImageId, ImageQuad};
use wgpu::util::DeviceExt as _;

/// 一个图像绘制的实例数据，32 字节。
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub(crate) struct ImageInstance {
    /// x, y, width, height，单位 pt。
    pub rect: [f32; 4],
    /// x：不透明度（钳到 0..=1），其余保留。
    pub params: [f32; 4],
}

impl ImageInstance {
    pub fn from_quad(q: &ImageQuad) -> ImageInstance {
        let opacity = if q.opacity.is_finite() {
            q.opacity.clamp(0.0, 1.0)
        } else {
            0.0
        };
        ImageInstance {
            rect: [
                q.rect.origin.x,
                q.rect.origin.y,
                q.rect.size.width.max(0.0),
                q.rect.size.height.max(0.0),
            ],
            params: [opacity, 0.0, 0.0, 0.0],
        }
    }
}

struct GpuImage {
    _texture: wgpu::Texture,
    bind_group: wgpu::BindGroup,
}

pub(crate) struct ImagePass {
    pub pipeline: wgpu::RenderPipeline,
    image_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    images: HashMap<u32, GpuImage>,
    next_id: u32,
    pub staging: Vec<ImageInstance>,
    pub instances: GrowBuffer<ImageInstance>,
}

impl ImagePass {
    pub fn new(device: &wgpu::Device, globals_layout: &wgpu::BindGroupLayout) -> ImagePass {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("tekes image shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("image.wgsl").into()),
        });
        let image_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("tekes image layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("tekes image pipeline layout"),
            bind_group_layouts: &[Some(globals_layout), Some(&image_layout)],
            immediate_size: 0,
        });
        let attributes = wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4];
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("tekes image sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });
        ImagePass {
            pipeline: instanced_pipeline(
                device,
                "tekes image pipeline",
                &shader,
                &layout,
                size_of::<ImageInstance>() as u64,
                &attributes,
            ),
            image_layout,
            sampler,
            images: HashMap::new(),
            next_id: 1,
            staging: Vec::new(),
            instances: GrowBuffer::new(device, "tekes image instances", 16),
        }
    }

    pub fn register(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        rgba8_premul: &[u8],
        width: u32,
        height: u32,
    ) -> Result<ImageId, RenderError> {
        if width == 0 || height == 0 {
            return Err(RenderError::InvalidArgument("图像尺寸不能为 0"));
        }
        let max = device.limits().max_texture_dimension_2d;
        if width > max || height > max {
            return Err(RenderError::InvalidArgument("图像超过纹理尺寸上限"));
        }
        if rgba8_premul.len() as u64 != width as u64 * height as u64 * 4 {
            return Err(RenderError::InvalidArgument(
                "图像像素数据长度必须是 宽 * 高 * 4",
            ));
        }
        let texture = device.create_texture_with_data(
            queue,
            &wgpu::TextureDescriptor {
                label: Some("tekes image"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                // 不用 sRGB 格式：数值原样当作编码值，不让硬件转换。
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
            wgpu::util::TextureDataOrder::LayerMajor,
            rgba8_premul,
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("tekes image"),
            layout: &self.image_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });
        let id = self.next_id;
        self.next_id += 1;
        self.images.insert(
            id,
            GpuImage {
                _texture: texture,
                bind_group,
            },
        );
        Ok(ImageId(id))
    }

    pub fn unregister(&mut self, id: ImageId) -> bool {
        self.images.remove(&id.0).is_some()
    }

    pub fn contains(&self, id: ImageId) -> bool {
        self.images.contains_key(&id.0)
    }

    pub fn bind_group(&self, id: ImageId) -> Option<&wgpu::BindGroup> {
        self.images.get(&id.0).map(|i| &i.bind_group)
    }

    pub fn len(&self) -> usize {
        self.images.len()
    }
}
