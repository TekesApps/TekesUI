//! quad 的 GPU 管线与实例缓冲。

use crate::common::{GrowBuffer, instanced_pipeline};
use crate::instance::QuadInstance;
use std::mem::size_of;

/// 实例缓冲的最小容量（实例数）。
const MIN_INSTANCE_CAPACITY: usize = 256;

pub(crate) struct QuadPass {
    pub pipeline: wgpu::RenderPipeline,
    pub instances: GrowBuffer<QuadInstance>,
    /// CPU 侧打包用的暂存，每帧复用。
    pub staging: Vec<QuadInstance>,
}

impl QuadPass {
    pub fn new(device: &wgpu::Device, globals_layout: &wgpu::BindGroupLayout) -> QuadPass {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("tekes quad shader"),
            source: wgpu::ShaderSource::Wgsl(
                concat!(include_str!("clip.wgsl"), "\n", include_str!("quad.wgsl")).into(),
            ),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("tekes quad pipeline layout"),
            bind_group_layouts: &[Some(globals_layout)],
            immediate_size: 0,
        });
        // 属性偏移与 QuadInstance 的字段一一对应，全部是 vec4<f32>。
        let attributes = wgpu::vertex_attr_array![
            0 => Float32x4,
            1 => Float32x4,
            2 => Float32x4,
            3 => Float32x4,
            4 => Float32x4,
            5 => Float32x4,
            6 => Float32x4,
            7 => Uint32x4,
        ];
        QuadPass {
            pipeline: instanced_pipeline(
                device,
                "tekes quad pipeline",
                &shader,
                &layout,
                size_of::<QuadInstance>() as u64,
                &attributes,
            ),
            instances: GrowBuffer::new(device, "tekes quad instances", MIN_INSTANCE_CAPACITY),
            staging: Vec::new(),
        }
    }
}
