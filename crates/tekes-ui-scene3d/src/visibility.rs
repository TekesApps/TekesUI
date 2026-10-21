//! GPU instance compaction and indirect counts. Hosts supply semantic pixel policy.
use wgpu::util::DeviceExt;

/// Projected detail policy for a named opaque node. Bounds are taken from the
/// current deformed GPU vertices, including skin, morph and node transforms.
/// Sizes are physical render-target pixels, not logical UI points.
#[derive(Clone, Debug)]
pub struct ScreenDetail3D {
    pub node: String,
    /// Reference world-space diameter before instance scale. Zero uses bounds.
    pub reference_size: f32,
    /// Disappear below x pixels, fully present at y pixels (smoothstep).
    pub fade_pixels: [f32; 2],
    /// Wind/swing becomes static below x, full animation above y. Zero disables.
    pub motion_pixels: [f32; 2],
    /// Extra local-space bound radius for procedural motion, before scale.
    pub motion_padding: f32,
}
#[derive(Debug)]
pub(crate) struct VisibilityGpu {
    pub pipeline: wgpu::ComputePipeline,
    pub bounds_pipeline: wgpu::ComputePipeline,
    pub layout: wgpu::BindGroupLayout,
    pub viewport: wgpu::Buffer,
}
#[derive(Debug)]
pub(crate) struct VisibleDraw {
    pub compute: wgpu::BindGroup,
    pub draw: wgpu::BindGroup,
    pub args: wgpu::Buffer,
    pub args_offset: u64,
    pub _output: wgpu::Buffer,
}
impl VisibilityGpu {
    pub fn new(device: &wgpu::Device) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("3D visibility layout"),
            entries: &(0..11)
                .map(|binding| wgpu::BindGroupLayoutEntry {
                    binding,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: if matches!(binding, 0 | 1 | 4 | 8) {
                            wgpu::BufferBindingType::Uniform
                        } else {
                            wgpu::BufferBindingType::Storage {
                                read_only: !matches!(binding, 6 | 7 | 9),
                            }
                        },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                })
                .collect::<Vec<_>>(),
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("3D projected visibility"),
            source: wgpu::ShaderSource::Wgsl(
                format!(
                    "{}\n{}",
                    include_str!("instance_motion.wgsl"),
                    include_str!("visibility.wgsl")
                )
                .into(),
            ),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("3D visibility"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("3D GPU LOD and culling"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("compact"),
            compilation_options: Default::default(),
            cache: None,
        });
        let bounds_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("3D deformed GPU bounds"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("bounds"),
            compilation_options: Default::default(),
            cache: None,
        });
        let viewport = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("3D physical viewport"),
            contents: bytemuck::cast_slice(&[1_f32; 4]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        Self {
            pipeline,
            bounds_pipeline,
            layout,
            viewport,
        }
    }
}
