//! quad 的 GPU 绘制：管线、uniform、可增长的实例缓冲。

use crate::instance::QuadInstance;
use std::mem::size_of;
use tekes_ui_scene::{Primitive, Scene};
use tekes_ui_tokens::Rgba;
use wgpu::util::DeviceExt as _;

/// 渲染目标和 surface 使用的像素格式。**刻意不用 sRGB 格式**，见 crate 文档"颜色管线"。
pub(crate) const COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8Unorm;

/// 实例缓冲的最小容量（实例数）。
const MIN_INSTANCE_CAPACITY: usize = 256;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Globals {
    viewport_px: [f32; 2],
    scale: f32,
    _pad: f32,
}

pub(crate) struct QuadPass {
    pipeline: wgpu::RenderPipeline,
    globals: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    instances: wgpu::Buffer,
    /// `instances` 能放多少个实例。
    capacity: usize,
    /// 缓冲重新分配过多少次（含初始分配），测试用。
    allocations: usize,
    /// CPU 侧打包用的暂存，每帧复用。
    staging: Vec<QuadInstance>,
}

fn instance_buffer(device: &wgpu::Device, capacity: usize) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("tekes quad instances"),
        size: (capacity * size_of::<QuadInstance>()) as u64,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

impl QuadPass {
    pub fn new(device: &wgpu::Device) -> QuadPass {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("tekes quad shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("quad.wgsl").into()),
        });

        let globals = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("tekes globals"),
            contents: bytemuck::bytes_of(&Globals {
                viewport_px: [1.0, 1.0],
                scale: 1.0,
                _pad: 0.0,
            }),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("tekes globals layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("tekes globals"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: globals.as_entire_binding(),
            }],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("tekes quad pipeline layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        // 属性偏移与 QuadInstance 的字段一一对应，全部是 vec4<f32>。
        let attributes = wgpu::vertex_attr_array![
            0 => Float32x4,
            1 => Float32x4,
            2 => Float32x4,
            3 => Float32x4,
            4 => Float32x4,
        ];

        // premultiplied alpha：src 已乘过 alpha。颜色和 alpha 通道用同一个公式。
        let blend = wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
        };

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("tekes quad pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: size_of::<QuadInstance>() as u64,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &attributes,
                })],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: COLOR_FORMAT,
                    blend: Some(blend),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        QuadPass {
            pipeline,
            globals,
            bind_group,
            instances: instance_buffer(device, MIN_INSTANCE_CAPACITY),
            capacity: MIN_INSTANCE_CAPACITY,
            allocations: 1,
            staging: Vec::new(),
        }
    }

    pub fn instance_capacity(&self) -> usize {
        self.capacity
    }

    pub fn buffer_allocations(&self) -> usize {
        self.allocations
    }

    /// 保证实例缓冲至少能放 `needed` 个实例。
    /// 容量不够时按 2 倍增长（至少到 `needed`），已经够用就原样复用。
    fn ensure_capacity(&mut self, device: &wgpu::Device, needed: usize) {
        if needed <= self.capacity {
            return;
        }
        let new_capacity = needed.max(self.capacity * 2);
        self.instances = instance_buffer(device, new_capacity);
        self.capacity = new_capacity;
        self.allocations += 1;
    }

    /// 把 scene 编码进 `encoder`：一个 render pass，先清屏再画所有 quad。
    #[allow(clippy::too_many_arguments)]
    pub fn encode(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        size_px: (u32, u32),
        scale: f32,
        scene: &Scene,
        clear: Rgba,
    ) {
        // 目前 scene 里只有 quad，所有 quad 合成一次实例化绘制。
        // F1 加入文字等图元后，在这里把"连续的同类图元"切成批，按顺序分别提交。
        self.staging.clear();
        for p in scene.primitives() {
            // Primitive 是 non_exhaustive，新增的图元种类在这里要显式处理。
            #[allow(clippy::single_match)]
            match p {
                Primitive::Quad(q) => self.staging.push(QuadInstance::from_quad(q)),
                _ => {}
            }
        }
        let count = self.staging.len();
        self.ensure_capacity(device, count);
        if count > 0 {
            queue.write_buffer(&self.instances, 0, bytemuck::cast_slice(&self.staging));
        }
        queue.write_buffer(
            &self.globals,
            0,
            bytemuck::bytes_of(&Globals {
                viewport_px: [size_px.0 as f32, size_px.1 as f32],
                scale,
                _pad: 0.0,
            }),
        );

        // clear 是非预乘 sRGB，目标里存的是预乘值，所以清屏色也要预乘。
        let a = clear.a as f64;
        let clear_color = wgpu::Color {
            r: clear.r as f64 * a,
            g: clear.g as f64 * a,
            b: clear.b as f64 * a,
            a,
        };
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("tekes quad pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(clear_color),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        if count > 0 {
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.set_vertex_buffer(0, self.instances.slice(..));
            pass.draw(0..4, 0..count as u32);
        }
    }
}
