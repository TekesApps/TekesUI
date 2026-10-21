//! Optional analytic local lighting. Immutable recipes; GPU evaluates attachment
//! motion once per frame, then shades surfaces with bounded ellipsoid shadows.
use crate::{
    Error, Instance3D, Route3D, err,
    instances::{GpuInstance, pack_routes},
};
use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;
const LIGHTS: usize = 64;
const SHADOWS: usize = 96;
/// Clock-driven local-light energy ramp. Clock channels belong to the host.
#[derive(Clone, Copy, Debug)]
pub struct Dimming3D {
    pub clock: usize,
    pub start_seconds: f32,
    pub end_seconds: f32,
    pub minimum: f32,
}
/// Point light, or a spot light when `outer_cos` is greater than -1.
/// Position/direction use the attachment's local coordinates; energy follows
/// Lighting3D::emissive_strength, allowing the host to switch night lighting.
#[derive(Clone, Debug)]
pub struct LocalLight3D {
    pub attachment: Instance3D,
    pub position: [f32; 3],
    pub direction: [f32; 3],
    pub color: [f32; 3],
    pub radius: f32,
    pub outer_cos: f32,
    pub dimming: Option<Dimming3D>,
}
/// Coarse soft-shadow proxy, not a mesh shadow map. Radii are world-axis
/// ellipsoid extents after attachment scale, suitable for distant actors/trees.
#[derive(Clone, Debug)]
pub struct ShadowProxy3D {
    pub attachment: Instance3D,
    pub center: [f32; 3],
    pub radii: [f32; 3],
}
#[derive(Clone, Debug, Default)]
pub struct LocalLighting3D {
    pub routes: Vec<Route3D>,
    pub lights: Vec<LocalLight3D>,
    pub shadows: Vec<ShadowProxy3D>,
}
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Input {
    attachment: GpuInstance,
    center_radius: [f32; 4],
    direction_cone: [f32; 4],
    color: [f32; 4],
    dimming: [f32; 4],
}
#[derive(Debug)]
pub(crate) struct LocalLightingGpu {
    pub layout: wgpu::BindGroupLayout,
    pub bind: wgpu::BindGroup,
    output: wgpu::Buffer,
    compute: wgpu::ComputePipeline,
    cull: wgpu::ComputePipeline,
    grid_bounds: wgpu::ComputePipeline,
    cull_lights: wgpu::ComputePipeline,
    motion_layout: wgpu::BindGroupLayout,
    output_layout: wgpu::BindGroupLayout,
    compute_binds: Option<(wgpu::BindGroup, wgpu::BindGroup)>,
    count: u32,
}
fn entry(
    binding: u32,
    uniform: bool,
    read_only: bool,
    visibility: wgpu::ShaderStages,
) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility,
        ty: wgpu::BindingType::Buffer {
            ty: if uniform {
                wgpu::BufferBindingType::Uniform
            } else {
                wgpu::BufferBindingType::Storage { read_only }
            },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}
impl LocalLightingGpu {
    pub fn new(device: &wgpu::Device) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("3D local light sampling"),
            entries: &[entry(0, false, true, wgpu::ShaderStages::FRAGMENT)],
        });
        let output = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("3D GPU evaluated lights and shadows"),
            contents: &vec![
                0;
                16 + LIGHTS * 48
                    + SHADOWS * 32
                    + LIGHTS * (4 + SHADOWS * 4)
                    + 16
                    + 1024 * 8
            ],
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: output.as_entire_binding(),
            }],
        });
        let motion_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("3D local light attachments"),
            entries: &[
                entry(1, true, true, wgpu::ShaderStages::COMPUTE),
                entry(2, false, true, wgpu::ShaderStages::COMPUTE),
                entry(3, false, true, wgpu::ShaderStages::COMPUTE),
            ],
        });
        let output_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("3D local light output"),
            entries: &[
                entry(0, false, true, wgpu::ShaderStages::COMPUTE),
                entry(1, false, false, wgpu::ShaderStages::COMPUTE),
            ],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("3D local light motion"),
            source: wgpu::ShaderSource::Wgsl(
                format!(
                    "{}\n{}",
                    include_str!("instance_motion.wgsl"),
                    include_str!("local_light_motion.wgsl")
                )
                .into(),
            ),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&motion_layout), Some(&output_layout)],
            immediate_size: 0,
        });
        let compute = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("3D evaluate light attachments"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("evaluate"),
            compilation_options: Default::default(),
            cache: None,
        });
        let cull = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("3D cull per-light shadow candidates"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("cull_shadows"),
            compilation_options: Default::default(),
            cache: None,
        });
        let grid_pipeline = |entry| {
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(entry),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: Some(entry),
                compilation_options: Default::default(),
                cache: None,
            })
        };
        let grid_bounds = grid_pipeline("grid_bounds");
        let cull_lights = grid_pipeline("cull_lights");
        Self {
            layout,
            bind,
            output,
            compute,
            cull,
            grid_bounds,
            cull_lights,
            motion_layout,
            output_layout,
            compute_binds: None,
            count: 0,
        }
    }
    pub fn configure(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        environment: &wgpu::Buffer,
        recipe: &LocalLighting3D,
    ) -> Result<(), Error> {
        if recipe.lights.len() > LIGHTS || recipe.shadows.len() > SHADOWS {
            return Err(err("local lighting exceeds 64 lights / 96 shadow proxies"));
        }
        let (points, spans) = pack_routes(&recipe.routes)?;
        let mut inputs = Vec::new();
        for l in &recipe.lights {
            let dimming = if let Some(c) = l.dimming {
                if c.clock >= 4
                    || !c.start_seconds.is_finite()
                    || !c.end_seconds.is_finite()
                    || c.start_seconds < 0.
                    || c.end_seconds <= c.start_seconds
                    || !(0.0..=1.0).contains(&c.minimum)
                {
                    return Err(err("invalid local light dimming curve"));
                }
                [
                    (c.clock + 1) as f32,
                    c.start_seconds,
                    c.end_seconds,
                    c.minimum,
                ]
            } else {
                [0.; 4]
            };
            if !l.radius.is_finite()
                || l.radius <= 0.
                || !(-1. ..1.).contains(&l.outer_cos)
                || l.position
                    .iter()
                    .chain(l.direction.iter())
                    .chain(l.color.iter())
                    .any(|x| !x.is_finite())
                || l.color.iter().any(|x| *x < 0.)
                || l.direction.iter().map(|x| x * x).sum::<f32>() < 1e-8
            {
                return Err(err(
                    "invalid local light position, direction, color, radius or cone",
                ));
            }
            inputs.push(Input {
                attachment: GpuInstance::from_instance(&l.attachment, &spans)?,
                dimming,
                center_radius: [l.position[0], l.position[1], l.position[2], l.radius],
                direction_cone: [l.direction[0], l.direction[1], l.direction[2], l.outer_cos],
                color: [l.color[0], l.color[1], l.color[2], 0.],
            });
        }
        for s in &recipe.shadows {
            if s.center
                .iter()
                .chain(s.radii.iter())
                .any(|x| !x.is_finite())
                || s.radii.iter().any(|x| *x <= 0.)
            {
                return Err(err("invalid shadow ellipsoid"));
            }
            inputs.push(Input {
                attachment: GpuInstance::from_instance(&s.attachment, &spans)?,
                dimming: [0.; 4],
                center_radius: [s.center[0], s.center[1], s.center[2], 0.],
                direction_cone: [s.radii[0], s.radii[1], s.radii[2], 0.],
                color: [0.; 4],
            });
        }
        let count = inputs.len() as u32;
        if inputs.is_empty() {
            inputs.push(Input::zeroed());
        }
        let input = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("3D immutable light recipe"),
            contents: bytemuck::cast_slice(&inputs),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let routes = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("3D light attachment routes"),
            contents: bytemuck::cast_slice(&points),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let dummy = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::bytes_of(&GpuInstance::identity()),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let motion = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.motion_layout,
            entries: &[(1, environment), (2, &dummy), (3, &routes)].map(|(binding, b)| {
                wgpu::BindGroupEntry {
                    binding,
                    resource: b.as_entire_binding(),
                }
            }),
        });
        let output = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.output_layout,
            entries: &[(0, &input), (1, &self.output)].map(|(binding, b)| wgpu::BindGroupEntry {
                binding,
                resource: b.as_entire_binding(),
            }),
        });
        queue.write_buffer(
            &self.output,
            0,
            bytemuck::cast_slice(&[
                recipe.lights.len() as u32,
                recipe.shadows.len() as u32,
                0,
                0,
            ]),
        );
        self.compute_binds = Some((motion, output));
        self.count = count;
        Ok(())
    }
    pub fn encode(&self, encoder: &mut wgpu::CommandEncoder) {
        if self.count == 0 {
            return;
        }
        let (motion, output) = self.compute_binds.as_ref().unwrap();
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("3D GPU local lights"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&self.compute);
        pass.set_bind_group(0, motion, &[]);
        pass.set_bind_group(1, output, &[]);
        pass.dispatch_workgroups(self.count.div_ceil(64), 1, 1);
        // Dispatch boundary makes every evaluated attachment available before
        // the candidate pass reads it; no CPU readback or per-frame list upload.
        pass.set_pipeline(&self.cull);
        pass.dispatch_workgroups(1, 1, 1);
        pass.set_pipeline(&self.grid_bounds);
        pass.dispatch_workgroups(1, 1, 1);
        pass.set_pipeline(&self.cull_lights);
        pass.dispatch_workgroups(16, 1, 1);
    }
}
