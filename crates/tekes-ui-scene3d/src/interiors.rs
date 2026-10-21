//! Shared procedural window poses, curtains and lamp states. Hosts provide only
//! the existing environment clock; the GPU evaluates 32 visual styles per frame.
#[derive(Debug)]
pub(crate) struct InteriorGpu {
    pub buffer: wgpu::Buffer,
    compute: Option<(wgpu::ComputePipeline, wgpu::BindGroup)>,
    previous: Option<[f32; 3]>,
    pub evaluations: u64,
}
impl InteriorGpu {
    pub fn new(device: &wgpu::Device, environment: &wgpu::Buffer, enabled: bool) -> Self {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("3D shared window styles (32)"),
            size: 32 * 48,
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        let compute = enabled.then(|| {
            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("3D evaluate shared window styles"),
                source: wgpu::ShaderSource::Wgsl(
                    format!(
                        "{}\n{}",
                        include_str!("interior_styles.wgsl"),
                        include_str!("interior_compute.wgsl")
                    )
                    .into(),
                ),
            });
            let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("3D shared interior states"),
                layout: None,
                module: &shader,
                entry_point: Some("evaluate_styles"),
                compilation_options: Default::default(),
                cache: None,
            });
            let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &pipeline.get_bind_group_layout(0),
                entries: &[(0, environment), (1, &buffer)].map(|(binding, b)| {
                    wgpu::BindGroupEntry {
                        binding,
                        resource: b.as_entire_binding(),
                    }
                }),
            });
            (pipeline, bind)
        });
        Self {
            buffer,
            compute,
            previous: None,
            evaluations: 0,
        }
    }
    pub fn encode_in_pass(&mut self, pass: &mut wgpu::ComputePass<'_>, input: [f32; 3]) {
        let Some((pipeline, bind)) = &self.compute else {
            return;
        };
        if self.previous == Some(input) {
            return;
        }
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, bind, &[]);
        pass.dispatch_workgroups(1, 1, 1);
        self.previous = Some(input);
        self.evaluations += 32;
    }
}
