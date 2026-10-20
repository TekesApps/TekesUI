use crate::{
    Asset3D, Camera3D, Error, Scene3D,
    deform::{Deformer, GpuVertex},
    err,
};
use tekes_ui_render::gpu::{GpuContext, GpuImage};
use wgpu::util::DeviceExt;

/// Platform-neutral wgpu renderer. Backend selection belongs to wgpu and the
/// host build configuration, not to component state or asset formats.
/// Renders into GPU storage shared with the UI renderer; no CPU pixel transfer.
#[derive(Debug)]
pub struct Renderer3D {
    asset_id: u64,
    context: GpuContext,
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::RenderPipeline,
    flat_pipeline: wgpu::RenderPipeline,
    opaque_order: Vec<usize>,
    opaque_bundle: Option<wgpu::RenderBundle>,
    bundle_builds: u64,
    last_camera: Option<[[[f32; 4]; 4]; 2]>,
    camera_upload_bytes: u64,
    flat_bindings: Vec<wgpu::BindGroup>,
    camera: wgpu::Buffer,
    camera_bind: wgpu::BindGroup,
    materials: Vec<wgpu::BindGroup>,
    deformer: Deformer,
    last_frame_pose_upload_bytes: u64,
    camera_layout: wgpu::BindGroupLayout,
    environment_buffer: wgpu::Buffer,
    environment_state: crate::Environment3D,
    identity_instances: wgpu::Buffer,
    route_buffer: wgpu::Buffer,
    instance_batches: Vec<Option<GpuBatch>>,
    instance_upload_bytes: u64,
    environment_upload_bytes: u64,
    size: (u32, u32),
    color: GpuImage,
    depth: wgpu::Texture,
    depth_view: wgpu::TextureView,
    msaa: Option<wgpu::Texture>,
    msaa_view: Option<wgpu::TextureView>,
    samples: u32,
}
impl Renderer3D {
    /// Renderer resources belong to this component's immutable asset. Build a new
    /// renderer after replacing an asset; pass the component to `render`.
    pub fn new(context: &GpuContext, component: &Scene3D, size: (u32, u32)) -> Result<Self, Error> {
        Self::with_samples(context, component, size, 1)
    }
    /// Four-sample GPU antialiasing with direct resolve into the shared image.
    /// Uses an additional multisampled color/depth target; no CPU readback.
    pub fn new_antialiased(
        context: &GpuContext,
        component: &Scene3D,
        size: (u32, u32),
    ) -> Result<Self, Error> {
        Self::with_samples(context, component, size, 4)
    }
    fn with_samples(
        context: &GpuContext,
        component: &Scene3D,
        size: (u32, u32),
        samples: u32,
    ) -> Result<Self, Error> {
        let device = context.device().clone();
        let queue = context.queue().clone();
        check_size(size, &device)?;
        let camera_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("3D camera"),
            entries: &[
                buffer_entry(0, wgpu::ShaderStages::VERTEX),
                buffer_entry(1, wgpu::ShaderStages::VERTEX),
                storage_entry(2),
                storage_entry(3),
            ],
        });
        let camera = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("3D camera"),
            size: 128,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let environment_state = crate::Environment3D::default();
        let environment_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("3D environment clock"),
            contents: bytemuck::cast_slice(&[0_f32, 0.8, 0.4, 1.]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let identity_instances = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("3D identity instance"),
            contents: bytemuck::bytes_of(&crate::instances::GpuInstance::identity()),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let route_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("3D default route"),
            contents: bytemuck::cast_slice(&[0_f32; 4]),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let camera_bind = instance_binding(
            &device,
            &camera_layout,
            &camera,
            &environment_buffer,
            &identity_instances,
            &route_buffer,
        );
        let material_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("3D material"),
            entries: &[
                buffer_entry(0, wgpu::ShaderStages::FRAGMENT),
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let materials = upload_materials(&device, &queue, &material_layout, &component.asset)?;
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Scene3D"),
            source: wgpu::ShaderSource::Wgsl(
                format!(
                    "{}\n{}",
                    include_str!("instance_motion.wgsl"),
                    include_str!("scene3d.wgsl")
                )
                .into(),
            ),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&camera_layout), Some(&material_layout)],
            immediate_size: 0,
        });
        let flat_geometry_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("3D indexed-pull geometry"),
                entries: &(0..2)
                    .map(|binding| wgpu::BindGroupLayoutEntry {
                        binding,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    })
                    .collect::<Vec<_>>(),
            });
        let flat_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("3D indexed-pull layout"),
            bind_group_layouts: &[
                Some(&camera_layout),
                Some(&material_layout),
                Some(&flat_geometry_layout),
            ],
            immediate_size: 0,
        });
        let pipeline = render_pipeline(&device, &shader, &layout, false, "fs", samples);
        let flat_pipeline = render_pipeline(&device, &shader, &flat_layout, true, "fs", samples);
        let deformer = Deformer::new(&device, &component.asset)?;
        let opaque_order = (0..deformer.meshes.len()).collect();
        let flat_bindings = deformer
            .meshes
            .iter()
            .map(|mesh| {
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("3D shared indexed geometry"),
                    layout: &flat_geometry_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: mesh.output.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: mesh.indices.as_entire_binding(),
                        },
                    ],
                })
            })
            .collect();
        let color = context.create_image(size).map_err(|e| err(e.to_string()))?;
        let depth = depth_target(&device, size, samples);
        let msaa = (samples > 1).then(|| multisample_target(&device, size, samples));
        Ok(Self {
            asset_id: component.asset.id,
            context: context.clone(),
            device,
            queue,
            pipeline,
            flat_pipeline,
            opaque_order,
            opaque_bundle: None,
            bundle_builds: 0,
            last_camera: None,
            camera_upload_bytes: 0,
            flat_bindings,
            camera,
            camera_bind,
            materials,
            deformer,
            last_frame_pose_upload_bytes: 0,
            camera_layout,
            environment_buffer,
            environment_state,
            identity_instances,
            route_buffer,
            instance_batches: vec![None; component.asset.meshes.len()],
            instance_upload_bytes: 0,
            environment_upload_bytes: 0,
            size,
            color,
            depth_view: depth.create_view(&Default::default()),
            depth,
            msaa_view: msaa.as_ref().map(|t| t.create_view(&Default::default())),
            msaa,
            samples,
        })
    }
    pub fn resize(&mut self, size: (u32, u32)) -> Result<(), Error> {
        check_size(size, &self.device)?;
        if size != self.size {
            self.color = self
                .context
                .create_image(size)
                .map_err(|e| err(e.to_string()))?;
            self.depth = depth_target(&self.device, size, self.samples);
            self.msaa =
                (self.samples > 1).then(|| multisample_target(&self.device, size, self.samples));

            self.depth_view = self.depth.create_view(&Default::default());
            self.msaa_view = self
                .msaa
                .as_ref()
                .map(|t| t.create_view(&Default::default()));
            self.size = size;
        }
        Ok(())
    }
    /// Render to the shared GPU output and submit before UI composition.
    /// Queue ordering provides producer/consumer synchronization without CPU waits.
    /// Fixed camera frames the rest pose; animated geometry may leave its bounds.
    /// `yaw` is radians around the vertical axis, `zoom` is a positive distance factor.
    pub fn render(&mut self, component: &Scene3D, yaw: f32, zoom: f32) -> Result<(), Error> {
        self.render_camera(component, &Camera3D::legacy(yaw, zoom)?)
    }
    /// Render an orbit/elevation/zoom view with independent model orientation.
    pub fn render_camera(&mut self, component: &Scene3D, camera: &Camera3D) -> Result<(), Error> {
        let mut encoder = self.device.create_command_encoder(&Default::default());
        self.encode_camera(component, camera, &mut encoder)?;
        self.queue.submit([encoder.finish()]);
        Ok(())
    }
    /// Encode into the host's frame on the same GpuContext. Submit this encoder
    /// exactly once, before the UI samples output(). No CPU synchronization.
    /// Do not drop encoded deformation work: call only after acquiring a frame.
    pub fn encode_camera(
        &mut self,
        component: &Scene3D,
        camera: &Camera3D,
        encoder: &mut wgpu::CommandEncoder,
    ) -> Result<(), Error> {
        let asset = &component.asset;
        if self.asset_id != asset.id {
            return Err(err("renderer asset mismatch"));
        }
        if self.opaque_bundle.is_none() {
            self.rebuild_opaque_bundle();
        }
        let matrices = camera.matrices(
            asset.center,
            asset.radius,
            self.size.0 as f32 / self.size.1 as f32,
        )?;
        if self.last_camera.as_ref() != Some(&matrices) {
            self.queue
                .write_buffer(&self.camera, 0, bytemuck::cast_slice(&matrices));
            self.last_camera = Some(matrices);
            self.camera_upload_bytes += 128;
        }
        self.last_frame_pose_upload_bytes =
            self.deformer
                .update_pose(&self.queue, asset, component.clip, component.time);
        self.deformer.encode(encoder);
        let msaa_view = &self.msaa_view;
        let resolved_view = self.color.render_view();
        let color_view = msaa_view.as_ref().unwrap_or(resolved_view);

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Scene3D"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: color_view,
                    depth_slice: None,
                    resolve_target: msaa_view.as_ref().map(|_| resolved_view),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(0.),
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            pass.execute_bundles(self.opaque_bundle.iter());
        }

        Ok(())
    }
    /// Number of immutable opaque command-list builds (not frame count).
    pub fn draw_bundle_builds(&self) -> u64 {
        self.bundle_builds
    }
    pub fn camera_upload_bytes(&self) -> u64 {
        self.camera_upload_bytes
    }
    fn rebuild_opaque_bundle(&mut self) {
        let mut bundle =
            self.device
                .create_render_bundle_encoder(&wgpu::RenderBundleEncoderDescriptor {
                    label: Some("3D cached opaque draws"),
                    color_formats: &[Some(wgpu::TextureFormat::Rgba8UnormSrgb)],
                    depth_stencil: Some(wgpu::RenderBundleDepthStencil {
                        format: wgpu::TextureFormat::Depth32Float,
                        depth_read_only: false,
                        stencil_read_only: true,
                    }),
                    sample_count: self.samples,
                    multiview: None,
                });
        for &i in &self.opaque_order {
            let draw = &self.deformer.meshes[i];
            let batch = self.instance_batches.get(i).and_then(Option::as_ref);
            let count = batch.map_or(1, |b| b.count);
            if count == 0 {
                continue;
            }
            bundle.set_pipeline(if draw.flat_normals {
                &self.flat_pipeline
            } else {
                &self.pipeline
            });
            bundle.set_bind_group(0, batch.map_or(&self.camera_bind, |b| &b.bind), &[]);
            bundle.set_bind_group(1, &self.materials[draw.material], &[]);
            if draw.flat_normals {
                bundle.set_bind_group(2, &self.flat_bindings[i], &[]);
                bundle.draw(0..draw.index_count, 0..count);
            } else {
                bundle.set_vertex_buffer(0, draw.output.slice(..));
                bundle.set_index_buffer(draw.indices.slice(..), wgpu::IndexFormat::Uint32);
                bundle.draw_indexed(0..draw.index_count, 0, 0..count);
            }
        }
        self.opaque_bundle = Some(bundle.finish(&wgpu::RenderBundleDescriptor {
            label: Some("3D cached opaque draws"),
        }));
        self.bundle_builds += 1;
    }
    /// Upload immutable instances/routes once. Omitted nodes retain one ordinary
    /// draw; an explicitly empty batch hides its node. Transparent instancing is
    /// rejected because primitive-center sorting cannot order its instances.
    pub fn configure_instances(
        &mut self,
        view: &Scene3D,
        config: &crate::InstanceScene3D,
    ) -> Result<(), Error> {
        if view.asset.id != self.asset_id {
            return Err(err("renderer asset mismatch"));
        }
        let (points, spans) = crate::instances::pack_routes(&config.routes)?;
        let limit = self.device.limits().max_storage_buffer_binding_size;
        let mut prepared = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for batch in &config.batches {
            if !seen.insert(&batch.node) {
                return Err(err("duplicate instance node batch"));
            }
            let matches: Vec<_> = self
                .deformer
                .meshes
                .iter()
                .enumerate()
                .filter(|(_, d)| {
                    view.asset.node_names[view.asset.meshes[d.mesh_index].node] == batch.node
                })
                .map(|(i, _)| i)
                .collect();
            if matches.is_empty() {
                return Err(err(format!("instance node not found: {}", batch.node)));
            }
            let packed: Vec<_> = batch
                .instances
                .iter()
                .map(|i| crate::instances::GpuInstance::from_instance(i, &spans))
                .collect::<Result<_, _>>()?;
            if (packed.len().max(1) * std::mem::size_of::<crate::instances::GpuInstance>()) as u64
                > limit
            {
                return Err(err("instance batch exceeds GPU storage limit"));
            }
            prepared.push((matches, packed));
        }
        if points.len() as u64 * 16 > limit {
            return Err(err("routes exceed GPU storage limit"));
        }
        let route_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("3D route arc lengths"),
                contents: bytemuck::cast_slice(&points),
                usage: wgpu::BufferUsages::STORAGE,
            });
        let mut bytes = route_buffer.size();
        let mut batches = vec![None; self.deformer.meshes.len()];
        for (indices, packed) in prepared {
            let count = packed.len() as u32;
            let contents = if packed.is_empty() {
                vec![crate::instances::GpuInstance::identity()]
            } else {
                packed
            };
            let buffer = self
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("3D immutable GPU instances"),
                    contents: bytemuck::cast_slice(&contents),
                    usage: wgpu::BufferUsages::STORAGE,
                });
            bytes += buffer.size();
            let bind = instance_binding(
                &self.device,
                &self.camera_layout,
                &self.camera,
                &self.environment_buffer,
                &buffer,
                &route_buffer,
            );
            let batch = GpuBatch {
                _buffer: buffer,
                bind,
                count,
            };
            for index in indices {
                batches[index] = Some(batch.clone());
            }
        }
        self.camera_bind = instance_binding(
            &self.device,
            &self.camera_layout,
            &self.camera,
            &self.environment_buffer,
            &self.identity_instances,
            &route_buffer,
        );
        self.route_buffer = route_buffer;
        self.instance_batches = batches;
        self.opaque_bundle = None;
        self.instance_upload_bytes += bytes;
        Ok(())
    }
    /// A 16-byte uniform update; all motion and normal calculations execute on GPU.
    pub fn set_environment(&mut self, environment: crate::Environment3D) -> Result<(), Error> {
        let values = [
            environment.time_seconds,
            environment.wind_direction[0],
            environment.wind_direction[1],
            environment.wind_strength,
        ];
        if values.iter().any(|v| !v.is_finite())
            || environment.time_seconds < 0.
            || environment.wind_strength < 0.
        {
            return Err(err("environment parameters must be finite and nonnegative"));
        }
        let old = self.environment_state;
        if values
            != [
                old.time_seconds,
                old.wind_direction[0],
                old.wind_direction[1],
                old.wind_strength,
            ]
        {
            self.queue
                .write_buffer(&self.environment_buffer, 0, bytemuck::cast_slice(&values));
            self.environment_upload_bytes += 16;
            self.environment_state = environment;
        }
        Ok(())
    }
    pub fn instance_upload_bytes(&self) -> u64 {
        self.instance_upload_bytes
    }
    pub fn environment_upload_bytes(&self) -> u64 {
        self.environment_upload_bytes
    }
    /// Register once with the issuing UI renderer; after resize, replace its
    /// ImageId backing storage with this output. No per-frame re-registration.
    pub fn output(&self) -> &GpuImage {
        &self.color
    }
    /// Diagnostic: immutable base and GPU output buffers, allocated at initialization.
    pub fn vertex_buffer_allocations(&self) -> usize {
        self.deformer.meshes.len() * 2
    }
    /// Base vertices, morph deltas and indices uploaded once during construction.
    pub fn geometry_upload_bytes(&self) -> u64 {
        self.deformer.geometry_upload_bytes
    }
    /// Resident geometry buffers: base vertices, morph deltas, indices and GPU
    /// output. Excludes materials, pose buffers and viewport color/depth targets.
    pub fn geometry_resident_bytes(&self) -> u64 {
        self.deformer.geometry_resident_bytes
    }
    /// Meshes dispatched for deformation in the most recent render. Camera-only
    /// frames reuse existing GPU vertices and return zero.
    pub fn last_frame_deformed_meshes(&self) -> usize {
        self.deformer.dirty_mesh_count()
    }
    /// Pose uploads for the last frame: matrices/weights only, excluding camera.
    /// This scales with joints/morph targets, not vertex count.
    pub fn last_frame_pose_upload_bytes(&self) -> u64 {
        self.last_frame_pose_upload_bytes
    }
}
#[derive(Clone, Debug)]
struct GpuBatch {
    _buffer: wgpu::Buffer,
    bind: wgpu::BindGroup,
    count: u32,
}
fn storage_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::VERTEX,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only: true },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}
fn instance_binding(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    camera: &wgpu::Buffer,
    environment: &wgpu::Buffer,
    instances: &wgpu::Buffer,
    routes: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("3D camera and instance motion"),
        layout,
        entries: &[camera, environment, instances, routes]
            .into_iter()
            .enumerate()
            .map(|(i, b)| wgpu::BindGroupEntry {
                binding: i as u32,
                resource: b.as_entire_binding(),
            })
            .collect::<Vec<_>>(),
    })
}
fn render_pipeline(
    device: &wgpu::Device,
    shader: &wgpu::ShaderModule,
    layout: &wgpu::PipelineLayout,
    flat: bool,
    fragment: &str,
    samples: u32,
) -> wgpu::RenderPipeline {
    let attributes = wgpu::vertex_attr_array![0=>Float32x4,1=>Float32x4,2=>Float32x4,3=>Float32x4];
    let vertex_buffers = [Some(wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<GpuVertex>() as u64,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &attributes,
    })];
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(if flat {
            "3D indexed-pull flat shading"
        } else {
            "3D indexed smooth shading"
        }),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some(if flat { "vs_flat" } else { "vs" }),
            compilation_options: Default::default(),
            buffers: if flat { &[] } else { &vertex_buffers },
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some(fragment),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState {
            cull_mode: None,
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::Greater),
            stencil: Default::default(),
            bias: Default::default(),
        }),
        multisample: wgpu::MultisampleState {
            count: samples,
            ..Default::default()
        },
        multiview_mask: None,
        cache: None,
    })
}

fn check_size(size: (u32, u32), device: &wgpu::Device) -> Result<(), Error> {
    let limit = device.limits().max_texture_dimension_2d;
    if size.0 == 0 || size.1 == 0 || size.0 > limit || size.1 > limit {
        return Err(err("3D viewport exceeds device limits or is empty"));
    }
    Ok(())
}
fn buffer_entry(binding: u32, visibility: wgpu::ShaderStages) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}
fn extent(size: (u32, u32)) -> wgpu::Extent3d {
    wgpu::Extent3d {
        width: size.0,
        height: size.1,
        depth_or_array_layers: 1,
    }
}
fn multisample_target(device: &wgpu::Device, size: (u32, u32), samples: u32) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("3D MSAA color"),
        size: extent(size),
        mip_level_count: 1,
        sample_count: samples,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    })
}
fn depth_target(device: &wgpu::Device, size: (u32, u32), samples: u32) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("3D depth target"),
        size: extent(size),
        mip_level_count: 1,
        sample_count: samples,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    })
}
fn upload_materials(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    asset: &Asset3D,
) -> Result<Vec<wgpu::BindGroup>, Error> {
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::MipmapFilterMode::Linear,
        anisotropy_clamp: 8,
        address_mode_u: wgpu::AddressMode::Repeat,
        address_mode_v: wgpu::AddressMode::Repeat,
        ..Default::default()
    });
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("3D GPU mipmaps"),
        source: wgpu::ShaderSource::Wgsl(include_str!("mipmap.wgsl").into()),
    });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("3D immutable texture downsampling"),
        layout: None,
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs"),
            compilation_options: Default::default(),
            buffers: &[],
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: Default::default(),
        depth_stencil: None,
        multisample: Default::default(),
        multiview_mask: None,
        cache: None,
    });
    asset
        .materials
        .iter()
        .map(|m| {
            let (size, pixels) = m
                .image
                .map(|i| {
                    let im = &asset.images[i];
                    ((im.width, im.height), im.rgba.as_slice())
                })
                .unwrap_or(((1, 1), &[255; 4]));
            check_size(size, device)?;
            let mip_count = 32 - size.0.max(size.1).leading_zeros();
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("3D base color"),
                size: extent(size),
                mip_level_count: mip_count,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::COPY_DST
                    | wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                pixels,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(size.0 * 4),
                    rows_per_image: Some(size.1),
                },
                extent(size),
            );
            if mip_count > 1 {
                let mut encoder = device.create_command_encoder(&Default::default());
                for level in 1..mip_count {
                    let source = texture.create_view(&wgpu::TextureViewDescriptor {
                        base_mip_level: level - 1,
                        mip_level_count: Some(1),
                        ..Default::default()
                    });
                    let target = texture.create_view(&wgpu::TextureViewDescriptor {
                        base_mip_level: level,
                        mip_level_count: Some(1),
                        ..Default::default()
                    });
                    let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: None,
                        layout: &pipeline.get_bind_group_layout(0),
                        entries: &[
                            wgpu::BindGroupEntry {
                                binding: 0,
                                resource: wgpu::BindingResource::TextureView(&source),
                            },
                            wgpu::BindGroupEntry {
                                binding: 1,
                                resource: wgpu::BindingResource::Sampler(&sampler),
                            },
                        ],
                    });
                    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("3D mip level"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &target,
                            depth_slice: None,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        ..Default::default()
                    });
                    pass.set_pipeline(&pipeline);
                    pass.set_bind_group(0, &bind, &[]);
                    pass.draw(0..3, 0..1);
                }
                queue.submit([encoder.finish()]);
            }
            let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("3D material"),
                contents: bytemuck::cast_slice(&[
                    m.color[0], m.color[1], m.color[2], m.color[3], m.cutoff, 0., 0., 0.,
                ]),
                usage: wgpu::BufferUsages::UNIFORM,
            });
            Ok(device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: uniform.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(
                            &texture.create_view(&Default::default()),
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    },
                ],
            }))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tekes_ui_render::Renderer;
    use tekes_ui_scene::{GlyphAtlas, ImageId, Rect, Scene};
    use tekes_ui_tokens::Rgba;

    fn capture(ui: &mut Renderer, view: &Scene3D, image: ImageId) -> Vec<u8> {
        let mut scene = Scene::new();
        view.paint(&mut scene, Rect::new(0., 0., 192., 192.), image);
        ui.render_to_rgba8(
            &scene,
            &mut GlyphAtlas::new(Default::default()),
            (192, 192),
            1.,
            Rgba::new(0., 0., 0., 0.),
        )
        .unwrap()
    }
    #[test]
    #[ignore = "requires a real wgpu GPU adapter"]
    fn environment_normals_match_numerical_reference_and_reuse_buffers() {
        use crate::{Deformation3D, Environment3D, Instance3D, InstanceBatch3D, InstanceScene3D};
        use glam::{Mat3, Quat, Vec3};
        let mut ui = Renderer::new_headless().unwrap();
        for water in [false, true] {
            let mut source = crate::asset::tests::depth_fixture(false);
            for p in &mut source.meshes[0].positions {
                if water {
                    *p = Vec3::new(p.x, 0., p.y);
                } else {
                    p.y += 1.3;
                }
            }
            if water {
                source.meshes[0].normals.fill(Vec3::Y);
            }
            let mut reference = crate::asset::tests::depth_fixture(false);
            reference.meshes = source.meshes.clone();
            let time = 0.7_f32;
            let amplitude = 0.08;
            let frequency = 1.6;
            let phase = 0.2;
            let scale = Vec3::new(0.8, 1.1, 0.9);
            let rotation = Quat::from_rotation_y(0.2);
            let deform = |p: Vec3| {
                let q = if water {
                    let a = p.x * 1.7 + p.z * 0.9 + time * frequency + phase;
                    let b = -p.x * 0.9 + p.z * 1.7 - time * frequency * 0.7 + phase;
                    p + Vec3::Y * amplitude * (a.sin() + 0.5 * b.sin())
                } else {
                    let h = p.y.max(0.);
                    p + Vec3::new(0.8, 0., 0.4)
                        * amplitude
                        * (time * frequency + phase).sin()
                        * h
                        * h
                };
                rotation * (q * scale)
            };
            let mesh = &mut reference.meshes[0];
            for (p, n) in mesh.positions.iter_mut().zip(&mut mesh.normals) {
                let original = *p;
                let eps = 0.001;
                let jacobian = Mat3::from_cols(
                    (deform(original + Vec3::X * eps) - deform(original - Vec3::X * eps))
                        / (2. * eps),
                    (deform(original + Vec3::Y * eps) - deform(original - Vec3::Y * eps))
                        / (2. * eps),
                    (deform(original + Vec3::Z * eps) - deform(original - Vec3::Z * eps))
                        / (2. * eps),
                );
                *p = deform(original);
                *n = (jacobian.inverse().transpose() * *n).normalize();
            }
            let node = source.node_names[0].clone();
            let view = Scene3D::new(source);
            let reference = Scene3D::new(reference);
            let mut gpu = Renderer3D::new(&ui.gpu_context(), &view, (192, 192)).unwrap();
            let mut cpu = Renderer3D::new(&ui.gpu_context(), &reference, (192, 192)).unwrap();
            normal_pipelines(&mut gpu);
            normal_pipelines(&mut cpu);
            let instance = Instance3D {
                yaw: 0.2,
                scale: scale.to_array(),
                deformation: if water {
                    Deformation3D::Waves {
                        amplitude,
                        frequency,
                        phase,
                    }
                } else {
                    Deformation3D::Wind {
                        amplitude,
                        frequency,
                        phase,
                        root_y: 0.,
                    }
                },
                ..Default::default()
            };
            gpu.configure_instances(
                &view,
                &InstanceScene3D {
                    routes: vec![],
                    batches: vec![InstanceBatch3D {
                        node,
                        instances: vec![instance],
                    }],
                },
            )
            .unwrap();
            gpu.set_environment(Environment3D {
                time_seconds: time,
                ..Default::default()
            })
            .unwrap();
            gpu.render(&view, 0.3, 1.).unwrap();
            cpu.render(&reference, 0.3, 1.).unwrap();
            let image = ui.register_gpu_image(gpu.output()).unwrap();
            let reference_image = ui.register_gpu_image(cpu.output()).unwrap();
            let actual = capture(&mut ui, &view, image);
            let expected = capture(&mut ui, &reference, reference_image);
            let mut compared = 0;
            for y in 1..191 {
                for x in 1..191 {
                    let offset = (y * 192 + x) * 4;
                    if [(x, y), (x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)]
                        .into_iter()
                        .all(|(x, y)| {
                            actual[(y * 192 + x) * 4 + 3] == 255
                                && expected[(y * 192 + x) * 4 + 3] == 255
                        })
                    {
                        assert!(
                            actual[offset..offset + 3]
                                .iter()
                                .zip(&expected[offset..offset + 3])
                                .all(|(a, b)| a.abs_diff(*b) <= 3),
                            "analytic GPU normals must match numerical Jacobian; water={water}"
                        );
                        compared += 1;
                    }
                }
            }
            assert!(compared > 100);
            let bytes = gpu.instance_upload_bytes();
            let geometry = gpu.geometry_resident_bytes();
            gpu.set_environment(Environment3D {
                time_seconds: 1.5,
                ..Default::default()
            })
            .unwrap();
            gpu.render(&view, 0.3, 1.).unwrap();
            assert_eq!(bytes, gpu.instance_upload_bytes());
            assert_eq!(geometry, gpu.geometry_resident_bytes());
            assert_eq!(gpu.last_frame_pose_upload_bytes(), 0);
            assert_eq!(gpu.last_frame_deformed_meshes(), 0);
        }
    }

    #[test]
    #[ignore = "requires a real wgpu GPU adapter"]
    fn distant_close_surfaces_do_not_z_fight() {
        let mut ui = Renderer::new_headless().unwrap();
        let views = [
            Scene3D::new(crate::asset::tests::depth_fixture(true)),
            Scene3D::new(crate::asset::tests::depth_fixture(false)),
        ];
        let mut renderers: Vec<_> = views
            .iter()
            .map(|view| Renderer3D::new(&ui.gpu_context(), view, (192, 192)).unwrap())
            .collect();
        let images: Vec<_> = renderers
            .iter()
            .map(|gpu| ui.register_gpu_image(gpu.output()).unwrap())
            .collect();
        for zoom in [1., 2., 4., 8.] {
            for yaw in [-0.4, 0., 0.4] {
                let mut camera = Camera3D::default();
                camera.orbit(yaw, -camera.pitch() + 0.2).unwrap();
                camera.zoom_by(zoom).unwrap();
                let frames: Vec<_> = renderers
                    .iter_mut()
                    .zip(&views)
                    .zip(&images)
                    .map(|((gpu, view), image)| {
                        gpu.render_camera(view, &camera).unwrap();
                        capture(&mut ui, view, *image)
                    })
                    .collect();
                let mut checked = 0;
                for y in 1..191 {
                    for x in 1..191 {
                        let pixel = (y * 192 + x) * 4;
                        if [(x, y), (x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)]
                            .iter()
                            .all(|(x, y)| frames[1][(y * 192 + x) * 4 + 3] == 255)
                        {
                            assert!(
                                frames[0][pixel] > 150,
                                "black intrusion zoom={zoom} yaw={yaw} pixel={x},{y}"
                            );
                            checked += 1;
                        }
                    }
                }
                assert!(checked > 5);
            }
        }
    }
    #[test]
    #[ignore = "requires a real wgpu GPU adapter"]
    fn camera_motion_reuses_deformation_and_seek_invalidates_it() {
        let mut ui = Renderer::new_headless().unwrap();
        let mut view = Scene3D::new(
            Asset3D::from_slice(include_bytes!("../fixtures/skinned-tetra.glb")).unwrap(),
        );
        view.select_clip(0).unwrap();
        let mut gpu = Renderer3D::new(&ui.gpu_context(), &view, (192, 192)).unwrap();
        let image = ui.register_gpu_image(gpu.output()).unwrap();
        let mut camera = Camera3D::default();
        gpu.render_camera(&view, &camera).unwrap();
        let original = capture(&mut ui, &view, image);
        assert!(gpu.last_frame_pose_upload_bytes() > 0);
        for _ in 0..5 {
            camera.rotate_model(0.1, 0.2, 0.).unwrap();
            gpu.render_camera(&view, &camera).unwrap();
            assert_eq!(gpu.last_frame_pose_upload_bytes(), 0);
            assert_eq!(gpu.last_frame_deformed_meshes(), 0);
        }
        camera.reset();
        gpu.render_camera(&view, &camera).unwrap();
        assert_eq!(original, capture(&mut ui, &view, image));
        view.seek(0.5).unwrap();
        gpu.render_camera(&view, &camera).unwrap();
        assert!(gpu.last_frame_pose_upload_bytes() > 0);
        assert_ne!(original, capture(&mut ui, &view, image));
        view.seek(0.).unwrap();
        gpu.render_camera(&view, &camera).unwrap();
        assert_eq!(original, capture(&mut ui, &view, image));
    }
    // Test-only alternate fragment output preserves normal direction/sign in RGB,
    // so the production lighting's two-sided abs(dot(...)) cannot hide a flip.
    fn normal_pipelines(gpu: &mut Renderer3D) {
        let shader = gpu
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("test-only normal direction output"),
                source: wgpu::ShaderSource::Wgsl(
                    format!(
                        "{}\n{}\n{}",
                        include_str!("instance_motion.wgsl"),
                        include_str!("scene3d.wgsl"),
                        r#"
                @fragment fn fs_test_normals(i: Out) -> @location(0) vec4<f32> {
                    let n = i.normal/max(length(i.normal),0.00001);
                    return vec4(n*0.5+vec3(0.5),1.0);
                }
            "#
                    )
                    .into(),
                ),
            });
        let camera = gpu.pipeline.get_bind_group_layout(0);
        let material = gpu.pipeline.get_bind_group_layout(1);
        let geometry = gpu.flat_pipeline.get_bind_group_layout(2);
        let layout = gpu
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[Some(&camera), Some(&material)],
                immediate_size: 0,
            });
        let flat_layout = gpu
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[Some(&camera), Some(&material), Some(&geometry)],
                immediate_size: 0,
            });
        gpu.pipeline = render_pipeline(&gpu.device, &shader, &layout, false, "fs_test_normals", 1);
        gpu.flat_pipeline = render_pipeline(
            &gpu.device,
            &shader,
            &flat_layout,
            true,
            "fs_test_normals",
            1,
        );
    }
    // Only the reference side expands triangles and uploads CPU geometry. The
    // production side being tested uses original vertices and immutable indices.
    fn cpu_reference(gpu: &Renderer3D, view: &Scene3D) {
        let draws = view.asset.pose(view.clip, view.time);
        let buffers: Vec<_> = draws
            .iter()
            .filter(|d| !d.vertices.is_empty())
            .map(|d| {
                let vertices: Vec<_> = d
                    .vertices
                    .iter()
                    .map(|v| GpuVertex {
                        position: [v.position[0], v.position[1], v.position[2], 1.],
                        normal: [v.normal[0], v.normal[1], v.normal[2], 0.],
                        uv: [v.uv[0], v.uv[1], 0., 0.],
                        color: v.color,
                    })
                    .collect();
                (
                    gpu.device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("test-only CPU reference"),
                            contents: bytemuck::cast_slice(&vertices),
                            usage: wgpu::BufferUsages::VERTEX,
                        }),
                    vertices.len() as u32,
                )
            })
            .collect();
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        let depth = gpu.depth.create_view(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: gpu.color.render_view(),
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(0.),
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            pass.set_pipeline(&gpu.pipeline);
            pass.set_bind_group(0, &gpu.camera_bind, &[]);
            for (mesh, (buffer, count)) in gpu.deformer.meshes.iter().zip(&buffers) {
                pass.set_bind_group(1, &gpu.materials[mesh.material], &[]);
                pass.set_vertex_buffer(0, buffer.slice(..));
                pass.draw(0..*count, 0..1);
            }
        }
        gpu.queue.submit([encoder.finish()]);
    }
    #[test]
    #[ignore = "requires a real wgpu GPU adapter"]
    fn indexed_smooth_and_flat_normal_images_match_expanded_cpu_reference() {
        let mut ui = Renderer::new_headless().unwrap();
        let cases = [
            Asset3D::from_slice(include_bytes!("../fixtures/animated-tetra.glb")).unwrap(),
            crate::asset::tests::deformation_fixture(false, false),
            crate::asset::tests::deformation_fixture(true, false),
            crate::asset::tests::seam_fixture(),
        ];
        for (case, asset) in cases.into_iter().enumerate() {
            let mut view = Scene3D::new(asset);
            view.select_clip(0).unwrap();
            let mut gpu = Renderer3D::new(&ui.gpu_context(), &view, (192, 192)).unwrap();
            normal_pipelines(&mut gpu);
            let image = ui.register_gpu_image(gpu.output()).unwrap();
            for time in [0., 0.4, 0.8, 1.2, 1.6, 2.] {
                view.seek(time).unwrap();
                gpu.render(&view, 0.3, 1.).unwrap();
                let actual = capture(&mut ui, &view, image);
                cpu_reference(&gpu, &view);
                let expected = capture(&mut ui, &view, image);
                let mut edge_pixels = 0;
                let mut geometry_pixels = 0;
                let mut normal_edge_pixels = 0;
                for (index, (a, e)) in actual
                    .chunks_exact(4)
                    .zip(expected.chunks_exact(4))
                    .enumerate()
                {
                    if a[3] != e[3] {
                        edge_pixels += 1;
                        continue;
                    }
                    if a[3] == 0 {
                        continue;
                    }
                    geometry_pixels += 1;
                    if !a.iter().zip(e).all(|(a, e)| a.abs_diff(*e) <= 2) {
                        // A subpixel position roundoff can choose the other face
                        // exactly on a shared edge. Only a few such edge pixels
                        // are allowed; every surface interior must match normals.
                        let x = index % 192;
                        let y = index / 192;
                        let edge = (-1_i32..=1).any(|dy| {
                            (-1_i32..=1).any(|dx| {
                                let nx = x as i32 + dx;
                                let ny = y as i32 + dy;
                                if !(0..192).contains(&nx) || !(0..192).contains(&ny) {
                                    return true;
                                }
                                let offset = (ny as usize * 192 + nx as usize) * 4;
                                expected[offset..offset + 4]
                                    .iter()
                                    .zip(e)
                                    .any(|(neighbor, e)| neighbor.abs_diff(*e) > 8)
                            })
                        });
                        assert!(
                            edge,
                            "case={case} time={time} interior normal: GPU={a:?} CPU={e:?}"
                        );
                        normal_edge_pixels += 1;
                    }
                }
                assert!(
                    normal_edge_pixels <= 8,
                    "case={case} time={time}: {normal_edge_pixels} face-edge mismatches"
                );
                assert!(geometry_pixels > 20, "fixture must render visible surfaces");
                assert!(
                    edge_pixels <= 4,
                    "case={case} t={time}: {edge_pixels} coverage mismatches"
                );
            }
            ui.unregister_image(image);
        }
    }
}
