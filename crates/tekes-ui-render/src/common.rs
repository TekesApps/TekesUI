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
#[derive(Clone, Copy, PartialEq, Pod, Zeroable)]
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
    last_written: Option<Globals>,
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
            last_written: None,
        }
    }

    pub fn write(&mut self, queue: &wgpu::Queue, size_px: (u32, u32), scale: f32) {
        let globals = Globals {
            viewport_px: [size_px.0 as f32, size_px.1 as f32],
            scale,
            _pad: 0.0,
        };
        if self.last_written == Some(globals) {
            return;
        }
        queue.write_buffer(&self.buffer, 0, bytemuck::bytes_of(&globals));
        self.last_written = Some(globals);
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
    /// Exact bytes last queued on this buffer. Invalidated on reallocation.
    uploaded: Vec<u8>,
    _marker: PhantomData<T>,
}

impl<T: Pod> GrowBuffer<T> {
    pub fn new(device: &wgpu::Device, label: &'static str, min_capacity: usize) -> GrowBuffer<T> {
        GrowBuffer {
            label,
            buffer: Self::alloc(device, label, min_capacity),
            capacity: min_capacity,
            allocations: 1,
            uploaded: Vec::new(),
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

    /// Upload only changed, 4-byte aligned words. Returns bytes actually queued.
    /// Queue ordering keeps these writes after earlier frames and before this one.
    pub fn write(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, items: &[T]) -> usize {
        if items.is_empty() {
            self.uploaded.clear();
            return 0;
        }
        if items.len() > self.capacity {
            let new_capacity = items.len().max(self.capacity * 2);
            self.buffer = Self::alloc(device, self.label, new_capacity);
            self.capacity = new_capacity;
            self.allocations += 1;
            self.uploaded.clear();
        }
        let bytes = bytemuck::cast_slice(items);
        let changed = changed_words(&self.uploaded, bytes);
        let count = changed.as_ref().map_or(0, |range| range.len());
        if let Some(range) = changed {
            queue.write_buffer(&self.buffer, range.start as u64, &bytes[range.clone()]);
            self.uploaded.resize(bytes.len(), 0);
            self.uploaded[range.clone()].copy_from_slice(&bytes[range]);
        } else {
            self.uploaded.truncate(bytes.len());
        }
        count
    }
}

/// Buffer writes require a four-byte offset and size. Compare exact words,
/// including float bit patterns; a hash collision can never hide a change.
fn changed_words(previous: &[u8], current: &[u8]) -> Option<std::ops::Range<usize>> {
    debug_assert_eq!(previous.len() % 4, 0);
    debug_assert_eq!(current.len() % 4, 0);
    let common = previous.len().min(current.len());
    let start = previous[..common]
        .chunks_exact(4)
        .zip(current[..common].chunks_exact(4))
        .position(|(a, b)| a != b)
        .map_or(common, |word| word * 4);
    if start == current.len() {
        return None;
    }
    let end = if previous.len() < current.len() {
        current.len()
    } else {
        let equal_tail = previous[start..current.len()]
            .rchunks_exact(4)
            .zip(current[start..].rchunks_exact(4))
            .take_while(|(a, b)| a == b)
            .count()
            * 4;
        current.len() - equal_tail
    };
    Some(start..end)
}

#[cfg(test)]
mod upload_tests {
    use super::{GrowBuffer, changed_words};

    #[test]
    #[cfg(target_os = "macos")]
    fn cached_buffer_queues_only_changed_bytes_and_invalidates_on_growth() {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = pollster::block_on(instance.request_adapter(&Default::default()))
            .expect("GPU adapter required for upload-cache validation");
        let (device, queue) =
            pollster::block_on(adapter.request_device(&Default::default())).unwrap();
        let mut buffer = GrowBuffer::<u32>::new(&device, "upload cache validation", 4);
        assert_eq!(buffer.write(&device, &queue, &[1, 2, 3, 4]), 16);
        assert_eq!(buffer.write(&device, &queue, &[1, 2, 3, 4]), 0);
        assert_eq!(buffer.write(&device, &queue, &[1, 7, 3, 4]), 4);
        assert_eq!(buffer.write(&device, &queue, &[1, 7]), 0);
        assert_eq!(buffer.write(&device, &queue, &[1, 7, 3, 4]), 8);
        // New buffer: equal prefix still needs uploading.
        assert_eq!(buffer.write(&device, &queue, &[1, 7, 3, 4, 5]), 20);
        assert_eq!(buffer.allocations(), 2);
        queue.submit([]);
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    }

    #[test]
    fn changes_are_exact_aligned_and_include_new_tail() {
        let a = [1_u32, 2, 3, 4];
        let b = [1_u32, 7, 3, 4];
        let bytes = |v: &[u32]| bytemuck::cast_slice::<u32, u8>(v).to_vec();
        assert_eq!(changed_words(&bytes(&a), &bytes(&a)), None);
        assert_eq!(changed_words(&bytes(&a), &bytes(&b)), Some(4..8));
        assert_eq!(changed_words(&bytes(&a), &bytes(&a[..2])), None);
        assert_eq!(changed_words(&bytes(&a[..2]), &bytes(&a)), Some(8..16));
        assert_eq!(changed_words(&[], &bytes(&a)), Some(0..16));
        assert_eq!(changed_words(&bytes(&a), &[]), None);
    }

    #[test]
    fn multiple_changes_and_float_bit_patterns_are_preserved() {
        let a = [0_u32, 1, 2, 3, 4];
        let b = [0_u32, 8, 2, 9, 4];
        assert_eq!(
            changed_words(bytemuck::cast_slice(&a), bytemuck::cast_slice(&b)),
            Some(4..16)
        );
        assert_eq!(
            changed_words(&0_f32.to_ne_bytes(), &(-0_f32).to_ne_bytes()),
            Some(0..4)
        );
    }
}
