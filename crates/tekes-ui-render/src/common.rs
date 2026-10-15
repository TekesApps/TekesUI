//! 几个管线共用的东西：像素格式、混合状态、全局 uniform、实例缓冲。

use bytemuck::{Pod, Zeroable};
use std::marker::PhantomData;
use std::mem::size_of;

/// 渲染目标和 surface 使用的像素格式。**刻意不用 sRGB 格式**，见 crate 文档"颜色管线"。
pub(crate) const COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8Unorm;

/// premultiplied alpha：src 已乘过 alpha。颜色和 alpha 通道用同一个公式。
pub(crate) fn premul_blend() -> wgpu::BlendState {
    let c = wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::One,
        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
        operation: wgpu::BlendOperation::Add,
    };
    wgpu::BlendState { color: c, alpha: c }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Globals {
    /// 渲染目标的像素尺寸。
    viewport_px: [f32; 2],
    /// 逻辑点到设备像素的比例。
    scale: f32,
    _pad: f32,
}

/// 全局 uniform（bind group 0），所有管线共用同一份布局与数据。
pub(crate) struct GlobalsBinding {
    buffer: wgpu::Buffer,
    pub layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
}

impl GlobalsBinding {
    pub fn new(device: &wgpu::Device) -> GlobalsBinding {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("tekes globals"),
            size: size_of::<Globals>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });
        GlobalsBinding {
            buffer,
            layout,
            bind_group,
        }
    }

    pub fn write(&self, queue: &wgpu::Queue, size_px: (u32, u32), scale: f32) {
        queue.write_buffer(
            &self.buffer,
            0,
            bytemuck::bytes_of(&Globals {
                viewport_px: [size_px.0 as f32, size_px.1 as f32],
                scale,
                _pad: 0.0,
            }),
        );
    }
}

/// 建一条实例化管线：4 个顶点的 triangle strip，一个实例缓冲，premultiplied alpha 混合。
/// 入口点固定为 `vs_main` / `fs_main`。
pub(crate) fn instanced_pipeline(
    device: &wgpu::Device,
    label: &str,
    shader: &wgpu::ShaderModule,
    layout: &wgpu::PipelineLayout,
    stride: u64,
    attributes: &[wgpu::VertexAttribute],
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_main"),
            compilation_options: Default::default(),
            buffers: &[Some(wgpu::VertexBufferLayout {
                array_stride: stride,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes,
            })],
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs_main"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: COLOR_FORMAT,
                blend: Some(premul_blend()),
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
    })
}

/// 可增长的实例缓冲，初始容量 `min_capacity` 个实例。
///
/// 不够时分配 `max(需要的数量, 旧容量 * 2)`，够用就原样复用，**不缩小**。
/// 每帧只用 `queue.write_buffer` 往里写，稳定状态下没有 GPU 缓冲分配。
pub(crate) struct GrowBuffer<T: Pod> {
    label: &'static str,
    buffer: wgpu::Buffer,
    capacity: usize,
    allocations: usize,
    _marker: PhantomData<T>,
}

impl<T: Pod> GrowBuffer<T> {
    pub fn new(device: &wgpu::Device, label: &'static str, min_capacity: usize) -> GrowBuffer<T> {
        GrowBuffer {
            label,
            buffer: Self::alloc(device, label, min_capacity),
            capacity: min_capacity,
            allocations: 1,
            _marker: PhantomData,
        }
    }

    fn alloc(device: &wgpu::Device, label: &'static str, capacity: usize) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: (capacity * size_of::<T>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// 缓冲重新分配过多少次（含初始分配）。
    pub fn allocations(&self) -> usize {
        self.allocations
    }

    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    /// 把 `items` 写到缓冲开头，必要时先增长。空切片什么也不做。
    pub fn write(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, items: &[T]) {
        if items.is_empty() {
            return;
        }
        if items.len() > self.capacity {
            let new_capacity = items.len().max(self.capacity * 2);
            self.buffer = Self::alloc(device, self.label, new_capacity);
            self.capacity = new_capacity;
            self.allocations += 1;
        }
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(items));
    }
}
