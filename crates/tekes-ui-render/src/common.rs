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
    viewport_px: [f32; 2],
    scale: f32,
    head: u32,
    origin: [f32; 2],
    clip_mode: u32,
    max_steps: u32,
}
/// Shared dynamic uniform and immutable clip-record storage, group 0 in every pass.
pub(crate) struct GlobalsBinding {
    uniform: wgpu::Buffer,
    records: wgpu::Buffer,
    uniform_capacity: u64,
    record_capacity: u64,
    uniform_uploaded: Vec<u8>,
    records_uploaded: Vec<u8>,
    pub alignment: u32,
    pub layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
}
impl GlobalsBinding {
    fn buffer(
        device: &wgpu::Device,
        label: &str,
        size: u64,
        usage: wgpu::BufferUsages,
    ) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size,
            usage: usage | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }
    fn group(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        uniform: &wgpu::Buffer,
        records: &wgpu::Buffer,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("tekes validated clip globals"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: uniform,
                        offset: 0,
                        size: std::num::NonZeroU64::new(size_of::<Globals>() as u64),
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: records.as_entire_binding(),
                },
            ],
        })
    }
    pub fn new(device: &wgpu::Device) -> Self {
        let alignment = device.limits().min_uniform_buffer_offset_alignment.max(32);
        let uniform = Self::buffer(
            device,
            "tekes dynamic globals",
            u64::from(alignment),
            wgpu::BufferUsages::UNIFORM,
        );
        let records = Self::buffer(
            device,
            "tekes clip records",
            48,
            wgpu::BufferUsages::STORAGE,
        );
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("tekes globals layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        min_binding_size: std::num::NonZeroU64::new(size_of::<Globals>() as u64),
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: std::num::NonZeroU64::new(48),
                    },
                    count: None,
                },
            ],
        });
        let bind_group = Self::group(device, &layout, &uniform, &records);
        Self {
            uniform,
            records,
            uniform_capacity: u64::from(alignment),
            record_capacity: 48,
            uniform_uploaded: Vec::new(),
            records_uploaded: Vec::new(),
            alignment,
            layout,
            bind_group,
        }
    }
    pub fn write(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        size: (u32, u32),
        scale: f32,
        frame: &crate::clip::ClipFrame,
    ) {
        let mut uniforms = vec![0_u8; frame.uniform_slots() * self.alignment as usize];
        for slot in 0..frame.uniform_slots() {
            // Slot zero is the exact legacy path; slot one is an empty analytic
            // chain. Further slots map one-to-one to nonzero chain heads.
            let head = slot.saturating_sub(1);
            let header = Globals {
                viewport_px: [size.0 as f32, size.1 as f32],
                scale,
                head: head as u32,
                origin: [0., 0.],
                clip_mode: u32::from(slot != 0),
                max_steps: frame.records.len() as u32,
            };
            let offset = slot * self.alignment as usize;
            uniforms[offset..offset + size_of::<Globals>()]
                .copy_from_slice(bytemuck::bytes_of(&header));
        }
        let empty = [crate::clip::ClipRecord::default()];
        let records = bytemuck::cast_slice(if frame.records.is_empty() {
            &empty[..]
        } else {
            &frame.records[..]
        });
        let mut changed = false;
        if uniforms.len() as u64 > self.uniform_capacity {
            self.uniform = Self::buffer(
                device,
                "tekes dynamic globals",
                uniforms.len() as u64,
                wgpu::BufferUsages::UNIFORM,
            );
            self.uniform_capacity = uniforms.len() as u64;
            self.uniform_uploaded.clear();
            changed = true;
        }
        if records.len() as u64 > self.record_capacity {
            self.records = Self::buffer(
                device,
                "tekes clip records",
                records.len() as u64,
                wgpu::BufferUsages::STORAGE,
            );
            self.record_capacity = records.len() as u64;
            self.records_uploaded.clear();
            changed = true;
        }
        if changed {
            self.bind_group = Self::group(device, &self.layout, &self.uniform, &self.records);
        }
        if uniforms != self.uniform_uploaded {
            queue.write_buffer(&self.uniform, 0, &uniforms);
            self.uniform_uploaded = uniforms;
        }
        if records != self.records_uploaded {
            queue.write_buffer(&self.records, 0, records);
            self.records_uploaded = records.to_vec();
        }
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
            // Complete-frame preflight checked required bytes. Clamp growth to the
            // device capacity too, so doubling cannot exceed that valid reservation.
            let device_capacity =
                (device.limits().max_buffer_size / size_of::<T>() as u64) as usize;
            let new_capacity = items
                .len()
                .max(self.capacity.saturating_mul(2))
                .min(device_capacity);
            assert!(new_capacity >= items.len(), "validated instance capacity");
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
