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
    visibility_batch: Option<wgpu::BindGroup>,
    visibility_bounds_dirty: bool,
    visibility: crate::visibility::VisibilityGpu,
    visible_draws: Vec<Option<crate::visibility::VisibleDraw>>,
    asset_id: u64,
    context: GpuContext,
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::RenderPipeline,
    flat_pipeline: wgpu::RenderPipeline,
    blend_pipeline: wgpu::RenderPipeline,
    flat_blend_pipeline: wgpu::RenderPipeline,
    opaque_order: Vec<usize>,
    opaque_bundle: Option<wgpu::RenderBundle>,
    bundle_builds: u64,
    last_camera: Option<[[[f32; 4]; 4]; 2]>,
    camera_upload_bytes: u64,
    transparent_order: Vec<(usize, f32)>,
    local_centers: Vec<glam::Vec3>,
    flat_bindings: Vec<wgpu::BindGroup>,
    camera: wgpu::Buffer,
    camera_bind: wgpu::BindGroup,
    materials: Vec<wgpu::BindGroup>,
    deformer: Deformer,
    last_frame_pose_upload_bytes: u64,
    camera_layout: wgpu::BindGroupLayout,
    environment_buffer: wgpu::Buffer,
    environment_state: crate::Environment3D,
    lighting_state: crate::Lighting3D,
    motion_clocks: [f32; 4],
    local_lighting: crate::local_lighting::LocalLightingGpu,
    interiors: crate::interiors::InteriorGpu,
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
    alpha_resolve: Option<AlphaResolve>,
}
impl Renderer3D {
    /// Total shared window styles evaluated on GPU. Each changed style-bank
    /// input evaluates 32 styles, independent of the number of visible windows.
    pub fn interior_style_evaluations(&self) -> u64 {
        self.interiors.evaluations
    }
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
        Self::with_lighting_shader(
            context,
            component,
            size,
            samples,
            include_str!("local_lighting.wgsl"),
        )
    }
    fn with_lighting_shader(
        context: &GpuContext,
        component: &Scene3D,
        size: (u32, u32),
        samples: u32,
        local_shader: &str,
    ) -> Result<Self, Error> {
        let device = context.device().clone();
        let queue = context.queue().clone();
        check_size(size, &device)?;
        let camera_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("3D camera"),
            entries: &[
                buffer_entry(0, wgpu::ShaderStages::VERTEX),
                buffer_entry(1, wgpu::ShaderStages::VERTEX_FRAGMENT),
                storage_entry(2),
                storage_entry(3),
                wgpu::BindGroupLayoutEntry {
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ..storage_entry(4)
                },
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
            contents: bytemuck::cast_slice(&[
                0_f32, 0.8, 0.4, 1., 0.3, 0.3, 0.3, 1., 0.4, 0.7, 1., 0., 0.7, 0.7, 0.7, 0., 0.,
                0., 0., 0.,
            ]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let identity_instances = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("3D identity instance"),
            contents: bytemuck::bytes_of(&crate::instances::GpuInstance::identity()),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        });
        let route_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("3D default route"),
            contents: bytemuck::cast_slice(&[0_f32; 4]),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let interiors = crate::interiors::InteriorGpu::new(
            &device,
            &environment_buffer,
            component.asset.materials.iter().any(|m| m.interior[0] > 0.),
        );
        let camera_bind = instance_binding(
            &device,
            &camera_layout,
            &camera,
            &environment_buffer,
            identity_instances.as_entire_buffer_binding(),
            &route_buffer,
            &interiors.buffer,
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
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });
        let materials = upload_materials(&device, &queue, &material_layout, &component.asset)?;
        let local_lighting = crate::local_lighting::LocalLightingGpu::new(&device);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Scene3D"),
            source: wgpu::ShaderSource::Wgsl(
                format!(
                    "{}\n{}\n{}\n{}\n{}",
                    include_str!("instance_motion.wgsl"),
                    local_shader,
                    include_str!("interior_styles.wgsl"),
                    include_str!("interior.wgsl"),
                    include_str!("scene3d.wgsl")
                )
                .into(),
            ),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[
                Some(&camera_layout),
                Some(&material_layout),
                Some(&local_lighting.layout),
            ],
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
                Some(&local_lighting.layout),
                Some(&flat_geometry_layout),
            ],
            immediate_size: 0,
        });
        let pipeline = render_pipeline(&device, &shader, &layout, false, "fs", samples, false);
        let flat_pipeline =
            render_pipeline(&device, &shader, &flat_layout, true, "fs", samples, false);
        let blend_pipeline = render_pipeline(&device, &shader, &layout, false, "fs", samples, true);
        let flat_blend_pipeline =
            render_pipeline(&device, &shader, &flat_layout, true, "fs", samples, true);
        let local_centers = component
            .asset
            .meshes
            .iter()
            .map(|m| {
                let min = m
                    .positions
                    .iter()
                    .copied()
                    .fold(glam::Vec3::splat(f32::INFINITY), glam::Vec3::min);
                let max = m
                    .positions
                    .iter()
                    .copied()
                    .fold(glam::Vec3::splat(f32::NEG_INFINITY), glam::Vec3::max);
                (min + max) * 0.5
            })
            .collect();
        let deformer = Deformer::new(&device, &component.asset)?;
        let opaque_order: Vec<_> = deformer
            .meshes
            .iter()
            .enumerate()
            .filter_map(|(i, m)| (!component.asset.materials[m.material].blend).then_some(i))
            .collect();
        let alpha_resolve =
            (opaque_order.len() != deformer.meshes.len()).then(|| AlphaResolve::new(&device, size));
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
        let visibility = crate::visibility::VisibilityGpu::new(&device);
        Ok(Self {
            visibility,
            visibility_batch: None,
            visibility_bounds_dirty: true,
            visible_draws: Vec::new(),
            asset_id: component.asset.id,
            context: context.clone(),
            device,
            queue,
            pipeline,
            flat_pipeline,
            blend_pipeline,
            flat_blend_pipeline,
            opaque_order,
            opaque_bundle: None,
            bundle_builds: 0,
            last_camera: None,
            camera_upload_bytes: 0,
            transparent_order: Vec::with_capacity(component.asset.meshes.len()),
            local_centers,
            flat_bindings,
            camera,
            camera_bind,
            materials,
            deformer,
            last_frame_pose_upload_bytes: 0,
            camera_layout,
            environment_buffer,
            environment_state,
            lighting_state: crate::Lighting3D::default(),
            motion_clocks: [0.; 4],
            local_lighting,
            interiors,
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
            alpha_resolve,
        })
    }
    pub fn resize(&mut self, size: (u32, u32)) -> Result<(), Error> {
        check_size(size, &self.device)?;
        if size != self.size {
            self.queue.write_buffer(
                &self.visibility.viewport,
                0,
                bytemuck::cast_slice(&[size.0 as f32, size.1 as f32, 0., 0.]),
            );
            self.color = self
                .context
                .create_image(size)
                .map_err(|e| err(e.to_string()))?;
            self.depth = depth_target(&self.device, size, self.samples);
            self.msaa =
                (self.samples > 1).then(|| multisample_target(&self.device, size, self.samples));
            if let Some(resolve) = &mut self.alpha_resolve {
                resolve.resize(&self.device, size);
            }
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
        // Sort transparent primitive centers back-to-front. This traverses only
        // small node transforms, never vertices; skin/morph still stay on GPU.
        if self.alpha_resolve.is_some() {
            let pose = asset.sample_pose(component.clip, component.time);
            let matrix = glam::Mat4::from_cols_array_2d(&matrices[0])
                * glam::Mat4::from_cols_array_2d(&matrices[1]);
            self.transparent_order.clear();
            for (i, draw) in self.deformer.meshes.iter().enumerate() {
                let mesh = &asset.meshes[draw.mesh_index];
                if asset.materials[mesh.material].blend {
                    let p = matrix
                        * pose.world[mesh.node]
                            .transform_point3(self.local_centers[draw.mesh_index])
                            .extend(1.);
                    self.transparent_order.push((i, p.z / p.w));
                }
            }
            // Reversed Z: distant fragments have smaller depth.
            self.transparent_order
                .sort_unstable_by(|a, b| a.1.total_cmp(&b.1));
        }
        self.deformer.encode(encoder);
        if let Some(batch_bind) = &self.visibility_batch {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("3D GPU projected LOD and frustum culling"),
                timestamp_writes: None,
            });
            if self.visibility_bounds_dirty || self.last_frame_pose_upload_bytes > 0 {
                pass.set_pipeline(&self.visibility.bounds_pipeline);
                for draw in self.visible_draws.iter().flatten() {
                    pass.set_bind_group(0, &draw.compute, &[]);
                    pass.dispatch_workgroups(1, 1, 1);
                }
                self.visibility_bounds_dirty = false;
            }
            pass.set_pipeline(&self.visibility.pipeline);
            pass.set_bind_group(0, batch_bind, &[]);
            pass.dispatch_workgroups(self.visible_draws.len() as u32, 1, 1);
        }
        self.interiors.encode(
            encoder,
            [
                self.environment_state.time_seconds,
                self.lighting_state.effect_time_origin,
                self.lighting_state.emissive_strength,
            ],
        );
        if self.lighting_state.emissive_strength > 0. {
            self.local_lighting.encode(encoder);
        }
        let msaa_view = &self.msaa_view;
        let resolved_view = self
            .alpha_resolve
            .as_ref()
            .map_or(self.color.render_view(), |r| &r.view);
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
            for i in self.transparent_order.iter().map(|p| p.0) {
                let draw = &self.deformer.meshes[i];
                let blend = asset.materials[draw.material].blend;
                pass.set_pipeline(match (draw.flat_normals, blend) {
                    (false, false) => &self.pipeline,
                    (true, false) => &self.flat_pipeline,
                    (false, true) => &self.blend_pipeline,
                    (true, true) => &self.flat_blend_pipeline,
                });
                let batch = self.instance_batches.get(i).and_then(Option::as_ref);
                let count = batch.map_or(1, |b| b.count);
                pass.set_bind_group(0, batch.map_or(&self.camera_bind, |b| &b.bind), &[]);
                pass.set_bind_group(1, &self.materials[draw.material], &[]);
                pass.set_bind_group(2, &self.local_lighting.bind, &[]);
                if draw.flat_normals {
                    pass.set_bind_group(3, &self.flat_bindings[i], &[]);
                    pass.draw(0..draw.index_count, 0..count);
                } else {
                    pass.set_vertex_buffer(0, draw.output.slice(..));
                    pass.set_index_buffer(draw.indices.slice(..), wgpu::IndexFormat::Uint32);
                    pass.draw_indexed(0..draw.index_count, 0, 0..count);
                }
            }
        }
        if let Some(resolve) = &self.alpha_resolve {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("3D gamma-correct premultiplied output"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: self.color.render_view(),
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            pass.set_pipeline(&resolve.pipeline);
            pass.set_bind_group(0, &resolve.bind, &[]);
            pass.draw(0..3, 0..1);
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
        // Render bundles are replayed as backend commands on Metal. Avoid
        // redundant state commands even though the bundle is built only once.
        let mut pipeline_state = None;
        let mut material_state = None;
        let mut camera_state = None;
        for &i in &self.opaque_order {
            let draw = &self.deformer.meshes[i];
            let batch = self.instance_batches.get(i).and_then(Option::as_ref);
            let count = batch.map_or(1, |b| b.count);
            if count == 0 {
                continue;
            }
            if pipeline_state != Some(draw.flat_normals) {
                bundle.set_pipeline(if draw.flat_normals {
                    &self.flat_pipeline
                } else {
                    &self.pipeline
                });
                bundle.set_bind_group(2, &self.local_lighting.bind, &[]);
                pipeline_state = Some(draw.flat_normals);
            }
            let visible = self.visible_draws.get(i).and_then(Option::as_ref);
            let camera_key = if visible.is_some() {
                Some(i)
            } else {
                batch.map(|_| i)
            };
            if camera_state != Some(camera_key) {
                bundle.set_bind_group(
                    0,
                    visible
                        .map_or_else(|| batch.map_or(&self.camera_bind, |b| &b.bind), |v| &v.draw),
                    &[],
                );
                camera_state = Some(camera_key);
            }
            if material_state != Some(draw.material) {
                bundle.set_bind_group(1, &self.materials[draw.material], &[]);
                material_state = Some(draw.material);
            }
            if draw.flat_normals {
                bundle.set_bind_group(3, &self.flat_bindings[i], &[]);
                if let Some(v) = visible {
                    bundle.draw_indirect(&v.args, v.args_offset);
                } else {
                    bundle.draw(0..draw.index_count, 0..count);
                }
            } else {
                bundle.set_vertex_buffer(0, draw.output.slice(..));
                bundle.set_index_buffer(draw.indices.slice(..), wgpu::IndexFormat::Uint32);
                if let Some(v) = visible {
                    bundle.draw_indexed_indirect(&v.args, v.args_offset);
                } else {
                    bundle.draw_indexed(0..draw.index_count, 0, 0..count);
                }
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
            if matches
                .iter()
                .any(|i| view.asset.materials[self.deformer.meshes[*i].material].blend)
            {
                return Err(err(
                    "transparent instancing requires per-instance transparency sorting",
                ));
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
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                });
            bytes += buffer.size();
            let bind = instance_binding(
                &self.device,
                &self.camera_layout,
                &self.camera,
                &self.environment_buffer,
                buffer.as_entire_buffer_binding(),
                &route_buffer,
                &self.interiors.buffer,
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
            self.identity_instances.as_entire_buffer_binding(),
            &route_buffer,
            &self.interiors.buffer,
        );
        self.route_buffer = route_buffer;
        self.instance_batches = batches;
        self.visible_draws.clear();
        self.visibility_batch = None;
        self.opaque_bundle = None;
        self.instance_upload_bytes += bytes;
        Ok(())
    }
    /// Enable GPU frustum culling for opaque draws and pixel LOD for selected
    /// nodes. Call after configuring instances; reconfigure after replacing them.
    /// Transparent primitives retain their existing sorted rendering path.
    pub fn configure_screen_detail(
        &mut self,
        view: &Scene3D,
        policies: &[crate::ScreenDetail3D],
    ) -> Result<(), Error> {
        if view.asset.id != self.asset_id {
            return Err(err("renderer asset mismatch"));
        }
        let mut seen = std::collections::HashSet::new();
        for p in policies {
            if !seen.insert(&p.node)
                || !view.asset.node_names.contains(&p.node)
                || [
                    p.reference_size,
                    p.fade_pixels[0],
                    p.fade_pixels[1],
                    p.motion_pixels[0],
                    p.motion_pixels[1],
                    p.motion_padding,
                ]
                .iter()
                .any(|v| !v.is_finite() || *v < 0.)
                || (p.fade_pixels[1] > 0. && p.fade_pixels[1] <= p.fade_pixels[0])
                || (p.motion_pixels[1] > 0. && p.motion_pixels[1] <= p.motion_pixels[0])
            {
                return Err(err("invalid screen detail policy"));
            }
        }
        if self.deformer.meshes.is_empty() {
            return Ok(());
        }
        let stride = std::mem::size_of::<crate::instances::GpuInstance>() as u64;
        let alignment = u64::from(self.device.limits().min_storage_buffer_offset_alignment);
        let mut total = 0_u64;
        let mut records = Vec::new();
        let mut offsets = Vec::new();
        let mut arguments = Vec::new();
        for (i, mesh) in self.deformer.meshes.iter().enumerate() {
            // LCM with the instance stride keeps both storage offsets and element offsets aligned.
            let segment_alignment = alignment * stride;
            // stride=160 and backend alignment are powers of two times five.
            let segment_alignment = segment_alignment / 32.min(alignment);
            total = total.div_ceil(segment_alignment) * segment_alignment;
            offsets.push(total);
            let count = if view.asset.materials[mesh.material].blend {
                0
            } else {
                self.instance_batches
                    .get(i)
                    .and_then(Option::as_ref)
                    .map_or(1, |b| b.count)
            };
            let node = &view.asset.node_names[view.asset.meshes[mesh.mesh_index].node];
            let p = policies.iter().find(|p| &p.node == node);
            let mut record = Vec::new();
            record.extend_from_slice(bytemuck::cast_slice(&[
                count,
                mesh.vertex_count,
                (total / stride) as u32,
                i as u32,
            ]));
            record.extend_from_slice(bytemuck::cast_slice(&[
                p.map_or(0., |p| p.reference_size),
                p.map_or(0., |p| p.fade_pixels[0]),
                p.map_or(0., |p| p.fade_pixels[1]),
                p.map_or(0., |p| p.motion_padding),
                p.map_or(0., |p| p.motion_pixels[0]),
                p.map_or(0., |p| p.motion_pixels[1]),
                0.,
                0.,
            ]));
            records.extend_from_slice(&record);
            arguments.extend([mesh.index_count, 0_u32, 0, 0, 0]);
            total += u64::from(count.max(1)) * stride;
        }
        if total > self.device.limits().max_storage_buffer_binding_size {
            return Err(err("visible instances exceed GPU storage limit"));
        }
        self.visibility_batch = None;
        let make = |label, size, usage| {
            self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size,
                usage,
                mapped_at_creation: false,
            })
        };
        let input = make(
            "3D packed immutable visibility inputs",
            total,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        );
        let output = make(
            "3D compact visible instance segments",
            total,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        );
        let bounds = make(
            "3D cached GPU bounds",
            self.deformer.meshes.len() as u64 * 32,
            wgpu::BufferUsages::STORAGE,
        );
        let args = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("3D packed indirect arguments"),
                contents: bytemuck::cast_slice(&arguments),
                usage: wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::INDIRECT
                    | wgpu::BufferUsages::COPY_SRC,
            });
        let policy_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("3D packed pixel policies"),
                contents: &records,
                usage: wgpu::BufferUsages::STORAGE,
            });
        let mut encoder = self.device.create_command_encoder(&Default::default());
        let mut draws = Vec::new();
        for (i, mesh) in self.deformer.meshes.iter().enumerate() {
            let batch = self.instance_batches.get(i).and_then(Option::as_ref);
            let count = batch.map_or(1, |b| b.count);
            let source = batch.map_or(&self.identity_instances, |b| &b._buffer);
            if view.asset.materials[mesh.material].blend {
                draws.push(None);
                continue;
            }
            if count > 0 {
                encoder.copy_buffer_to_buffer(
                    source,
                    0,
                    &input,
                    offsets[i],
                    u64::from(count) * stride,
                );
            }
            let params = self
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("3D bounds metadata"),
                    contents: &records[i * 48..i * 48 + 48],
                    usage: wgpu::BufferUsages::UNIFORM,
                });
            let binding = |vertices: &wgpu::Buffer| {
                self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("3D batched visibility and bounds"),
                    layout: &self.visibility.layout,
                    entries: &[
                        &self.camera,
                        &self.environment_buffer,
                        &input,
                        &self.route_buffer,
                        &self.visibility.viewport,
                        vertices,
                        &output,
                        &args,
                        &params,
                        &bounds,
                        &policy_buffer,
                    ]
                    .into_iter()
                    .enumerate()
                    .map(|(binding, b)| wgpu::BindGroupEntry {
                        binding: binding as u32,
                        resource: b.as_entire_binding(),
                    })
                    .collect::<Vec<_>>(),
                })
            };
            let compute = binding(&mesh.output);
            if self.visibility_batch.is_none() {
                self.visibility_batch = Some(compute.clone());
            }
            let draw = instance_binding(
                &self.device,
                &self.camera_layout,
                &self.camera,
                &self.environment_buffer,
                wgpu::BufferBinding {
                    buffer: &output,
                    offset: offsets[i],
                    size: std::num::NonZeroU64::new(u64::from(count.max(1)) * stride),
                },
                &self.route_buffer,
                &self.interiors.buffer,
            );
            draws.push(Some(crate::visibility::VisibleDraw {
                compute,
                draw,
                args: args.clone(),
                args_offset: i as u64 * 20,
                _output: output.clone(),
            }));
        }
        self.queue.submit([encoder.finish()]);
        self.queue.write_buffer(
            &self.visibility.viewport,
            0,
            bytemuck::cast_slice(&[self.size.0 as f32, self.size.1 as f32, 0., 0.]),
        );
        self.visible_draws = draws;
        self.visibility_bounds_dirty = true;
        self.opaque_bundle = None;
        Ok(())
    }
    /// Upload local lights/analytic shadow proxies once. Motion evaluates on GPU
    /// and does not invalidate cached draw bundles or depend on instance setup order.
    pub fn configure_local_lighting(
        &mut self,
        recipe: &crate::LocalLighting3D,
    ) -> Result<(), Error> {
        self.local_lighting
            .configure(&self.device, &self.queue, &self.environment_buffer, recipe)
    }
    /// Cached 48-byte update on lighting changes; no mesh/material re-upload.
    pub fn set_lighting(&mut self, lighting: crate::Lighting3D) -> Result<(), Error> {
        let values = [
            lighting.ambient[0],
            lighting.ambient[1],
            lighting.ambient[2],
            lighting.emissive_strength,
            lighting.direction[0],
            lighting.direction[1],
            lighting.direction[2],
            0.,
            lighting.directional[0],
            lighting.directional[1],
            lighting.directional[2],
            lighting.effect_time_origin,
        ];
        if values.iter().any(|v| !v.is_finite())
            || lighting.direction.iter().map(|v| v * v).sum::<f32>() < 1e-8
            || lighting
                .ambient
                .iter()
                .chain(lighting.directional.iter())
                .any(|v| *v < 0.)
            || lighting.emissive_strength < 0.
            || lighting.effect_time_origin < 0.
        {
            return Err(err(
                "lighting must be finite with nonnegative energy and a nonzero direction",
            ));
        }
        if self.lighting_state != lighting {
            self.queue
                .write_buffer(&self.environment_buffer, 16, bytemuck::cast_slice(&values));
            self.environment_upload_bytes += 48;
            self.lighting_state = lighting;
        }
        Ok(())
    }
    /// Four independent host clocks, shared by instance motion, visibility and
    /// material/local-light dimming. Frozen channels require no repeated upload.
    pub fn set_motion_clocks(&mut self, clocks: [f32; 4]) -> Result<(), Error> {
        if clocks.iter().any(|v| !v.is_finite() || *v < 0.) {
            return Err(err("motion clocks must be finite and nonnegative"));
        }
        if clocks != self.motion_clocks {
            self.queue
                .write_buffer(&self.environment_buffer, 64, bytemuck::cast_slice(&clocks));
            self.motion_clocks = clocks;
            self.environment_upload_bytes += 16;
        }
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
    instances: wgpu::BufferBinding<'_>,
    routes: &wgpu::Buffer,
    interiors: &wgpu::Buffer,
) -> wgpu::BindGroup {
    let mut entries = [camera, environment, routes, interiors]
        .into_iter()
        .zip([0, 1, 3, 4])
        .map(|(b, binding)| wgpu::BindGroupEntry {
            binding,
            resource: b.as_entire_binding(),
        })
        .collect::<Vec<_>>();
    entries.push(wgpu::BindGroupEntry {
        binding: 2,
        resource: wgpu::BindingResource::Buffer(instances),
    });
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("3D camera and instance motion"),
        layout,
        entries: &entries,
    })
}
// The UI consumes gamma-encoded premultiplied channels. Blend in linear light
// first, then convert on GPU; encoding premultiplied linear channels directly
// would make glass over a transparent background too bright in the host UI.
#[derive(Debug)]
struct AlphaResolve {
    pipeline: wgpu::RenderPipeline,
    _texture: wgpu::Texture,
    view: wgpu::TextureView,
    bind: wgpu::BindGroup,
}
impl AlphaResolve {
    fn attachment(
        device: &wgpu::Device,
        size: (u32, u32),
        layout: &wgpu::BindGroupLayout,
    ) -> (wgpu::Texture, wgpu::TextureView, wgpu::BindGroup) {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("3D linear alpha composition"),
            size: extent(size),
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&Default::default());
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("3D alpha resolve"),
            layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            }],
        });
        (texture, view, bind)
    }
    fn new(device: &wgpu::Device, size: (u32, u32)) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("3D alpha resolve"),
            source: wgpu::ShaderSource::Wgsl(include_str!("alpha_resolve.wgsl").into()),
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("3D alpha resolve"),
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
        let (texture, view, bind) =
            Self::attachment(device, size, &pipeline.get_bind_group_layout(0));
        Self {
            pipeline,
            _texture: texture,
            view,
            bind,
        }
    }
    fn resize(&mut self, device: &wgpu::Device, size: (u32, u32)) {
        let (texture, view, bind) =
            Self::attachment(device, size, &self.pipeline.get_bind_group_layout(0));
        self._texture = texture;
        self.view = view;
        self.bind = bind;
    }
}
fn render_pipeline(
    device: &wgpu::Device,
    shader: &wgpu::ShaderModule,
    layout: &wgpu::PipelineLayout,
    flat: bool,
    fragment: &str,
    samples: u32,
    blend: bool,
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
                blend: blend.then_some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState {
            cull_mode: None,
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: Some(!blend),
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
    let textures = asset
        .images
        .iter()
        .map(Some)
        .chain(std::iter::once(None))
        .map(|image| {
            let (size, pixels) = image
                .map(|im| ((im.width, im.height), im.rgba.as_slice()))
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
            Ok(texture.create_view(&Default::default()))
        })
        .collect::<Result<Vec<_>, Error>>()?;
    let white = textures.len() - 1;
    asset
        .materials
        .iter()
        .map(|m| {
            let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("3D material"),
                contents: bytemuck::cast_slice(&[
                    m.color[0],
                    m.color[1],
                    m.color[2],
                    m.color[3],
                    m.cutoff,
                    if m.blend { 1. } else { 0. },
                    0.,
                    0.,
                    m.emissive[0],
                    m.emissive[1],
                    m.emissive[2],
                    0.,
                    m.interior[0],
                    m.interior[1],
                    m.interior[2],
                    m.interior[3],
                    m.dimming[0],
                    m.dimming[1],
                    m.dimming[2],
                    m.dimming[3],
                    m.interior_activation[0],
                    m.interior_activation[1],
                    m.interior_activation[2],
                    m.interior_activation[3],
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
                            &textures[m.image.unwrap_or(white)],
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(
                            &textures[m.emissive_image.unwrap_or(white)],
                        ),
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
    fn read_gpu_bytes(gpu: &Renderer3D, source: &wgpu::Buffer, size: u64) -> Vec<u8> {
        let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("test-only visibility readback"),
            size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        encoder.copy_buffer_to_buffer(source, 0, &buffer, 0, size);
        gpu.queue.submit([encoder.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |r| tx.send(r).unwrap());
        gpu.device
            .poll(wgpu::PollType::wait_indefinitely())
            .unwrap();
        rx.recv().unwrap().unwrap();
        buffer.slice(..).get_mapped_range().unwrap().to_vec()
    }
    #[test]
    #[ignore = "requires a real wgpu GPU adapter"]
    fn gpu_visibility_bounds_follow_skin_morph_and_model_rotation() {
        let mut ui = Renderer::new_headless().unwrap();
        for bytes in [
            include_bytes!("../fixtures/animated-tetra.glb").as_slice(),
            include_bytes!("../fixtures/skinned-tetra.glb").as_slice(),
        ] {
            let mut view = Scene3D::new(Asset3D::from_slice(bytes).unwrap());
            view.select_clip(0).unwrap();
            let mut direct = Renderer3D::new(&ui.gpu_context(), &view, (192, 192)).unwrap();
            let mut culled = Renderer3D::new(&ui.gpu_context(), &view, (192, 192)).unwrap();
            culled.configure_screen_detail(&view, &[]).unwrap();
            let a = ui.register_gpu_image(direct.output()).unwrap();
            let b = ui.register_gpu_image(culled.output()).unwrap();
            for time in [0., 0.25, 0.5, 1., 1.5] {
                view.seek(time).unwrap();
                let mut camera = Camera3D::default();
                camera.orbit(time * 0.1, 0.05).unwrap();
                camera.rotate_model(time * 0.15, 0.1, 0.).unwrap();
                direct.render_camera(&view, &camera).unwrap();
                culled.render_camera(&view, &camera).unwrap();
                let expected = capture(&mut ui, &view, a);
                let actual = capture(&mut ui, &view, b);
                assert!(
                    actual == expected,
                    "indirect animated geometry differs at {time}"
                );
            }
        }
    }
    #[test]
    #[ignore = "requires a real wgpu GPU adapter"]
    fn gpu_visibility_indirect_counts_fade_and_frustum_match_reference() {
        use crate::{Instance3D, InstanceBatch3D, InstanceScene3D, ScreenDetail3D};
        let mut ui = Renderer::new_headless().unwrap();
        for flat in [false, true] {
            let mut asset = crate::asset::tests::depth_fixture(false);
            if flat {
                asset.meshes[0].normals.clear();
            }
            let node = asset.node_names[asset.meshes[0].node].clone();
            let view = Scene3D::new(asset);
            let mut gpu = Renderer3D::new(&ui.gpu_context(), &view, (192, 192)).unwrap();
            let image = ui.register_gpu_image(gpu.output()).unwrap();
            let mut camera = Camera3D::default();
            camera.set_orthographic(true);
            gpu.render_camera(&view, &camera).unwrap();
            let baseline = capture(&mut ui, &view, image);
            gpu.configure_instances(
                &view,
                &InstanceScene3D {
                    routes: vec![],
                    batches: vec![InstanceBatch3D {
                        node: node.clone(),
                        instances: vec![
                            Instance3D::default(),
                            Instance3D {
                                translation: [100., 0., 0.],
                                ..Default::default()
                            },
                        ],
                    }],
                },
            )
            .unwrap();
            gpu.configure_screen_detail(
                &view,
                &[ScreenDetail3D {
                    node,
                    reference_size: 1.,
                    fade_pixels: [20., 40.],
                    motion_pixels: [0., 0.],
                    motion_padding: 0.,
                }],
            )
            .unwrap();
            let uploaded = gpu.instance_upload_bytes();
            let geometry = gpu.geometry_upload_bytes();
            camera.zoom_by(0.2).unwrap();
            gpu.render_camera(&view, &camera).unwrap();
            let visible = gpu.visible_draws[0].as_ref().unwrap();
            assert_eq!(
                bytemuck::cast_slice::<u8, u32>(&read_gpu_bytes(&gpu, &visible.args, 20))[1],
                1
            );
            // Identical full-detail image to direct rendering, at the same camera.
            camera = Camera3D::default();
            camera.set_orthographic(true);
            // Disable size fade, retain offscreen culling, compare exact pixels.
            let policies = [ScreenDetail3D {
                node: view.asset.node_names[view.asset.meshes[0].node].clone(),
                reference_size: 1.,
                fade_pixels: [0., 0.],
                motion_pixels: [0., 0.],
                motion_padding: 0.,
            }];
            gpu.configure_screen_detail(&view, &policies).unwrap();
            gpu.render_camera(&view, &camera).unwrap();
            assert_eq!(capture(&mut ui, &view, image), baseline);
            let mut p = policies[0].clone();
            p.fade_pixels = [20., 40.];
            gpu.configure_screen_detail(&view, &[p]).unwrap();
            let mut fades = Vec::new();
            for zoom in [0.5_f32, 0.75, 1., 1.25, 1.5, 2., 4., 8.] {
                let mut c = Camera3D::default();
                c.set_orthographic(true);
                c.zoom_by(zoom).unwrap();
                gpu.render_camera(&view, &c).unwrap();
                let d = gpu.visible_draws[0].as_ref().unwrap();
                let count = bytemuck::cast_slice::<u8, u32>(&read_gpu_bytes(&gpu, &d.args, 20))[1];
                let fade = if count == 0 {
                    0.
                } else {
                    bytemuck::cast_slice::<u8, f32>(&read_gpu_bytes(&gpu, &d._output, 160))[36]
                };
                fades.push(fade);
            }
            assert!(fades.windows(2).all(|w| w[0] >= w[1]), "{fades:?}");
            assert!(
                fades.iter().any(|x| *x > 0. && *x < 1.),
                "fade band exercised: {fades:?}"
            );
            assert_eq!(fades.last(), Some(&0.));
            assert_eq!(gpu.instance_upload_bytes(), uploaded);
            assert_eq!(gpu.geometry_upload_bytes(), geometry);
            assert_eq!(gpu.bundle_builds, 4); // direct, then three explicit reconfigurations
        }
    }
    #[test]
    #[ignore = "requires a real wgpu GPU adapter"]
    fn shared_window_bank_updates_once_and_reuses_states_across_camera_frames() {
        let mut ui = Renderer::new_headless().unwrap();
        let mut asset = crate::asset::tests::depth_fixture(false);
        asset.meshes[0].colors.fill([1.; 4]);
        let mesh = &mut asset.meshes[0];
        for (p, uv) in mesh.positions.iter().zip(&mut mesh.uv) {
            *uv = [(p.x + 1.) * 0.5, (p.y + 1.) * 0.5];
        }
        asset.materials[0].interior = [4., 4., 5., 0.5];
        asset.materials[0].emissive = [0.8; 3];
        let view = Scene3D::new(asset);
        let mut gpu = Renderer3D::new_antialiased(&ui.gpu_context(), &view, (192, 192)).unwrap();
        let image = ui.register_gpu_image(gpu.output()).unwrap();
        let mut camera = Camera3D::default();
        gpu.render_camera(&view, &camera).unwrap();
        assert_eq!(gpu.interior_style_evaluations(), 32);
        let initial = capture(&mut ui, &view, image);
        for _ in 0..4 {
            camera.rotate_model(0.1, 0., 0.).unwrap();
            gpu.render_camera(&view, &camera).unwrap();
            assert_eq!(
                gpu.interior_style_evaluations(),
                32,
                "camera motion must reuse the bank"
            );
        }
        camera = Camera3D::default();
        gpu.set_environment(crate::Environment3D {
            time_seconds: 18.,
            ..Default::default()
        })
        .unwrap();
        gpu.render_camera(&view, &camera).unwrap();
        assert_eq!(gpu.interior_style_evaluations(), 64);
        assert_ne!(
            initial,
            capture(&mut ui, &view, image),
            "shared styles must produce visible animation"
        );
        gpu.render_camera(&view, &camera).unwrap();
        assert_eq!(
            gpu.interior_style_evaluations(),
            64,
            "frozen time must not reevaluate styles"
        );
        let lighting = crate::Lighting3D {
            emissive_strength: 0.,
            ..Default::default()
        };
        gpu.set_lighting(lighting).unwrap();
        gpu.render_camera(&view, &camera).unwrap();
        assert_eq!(
            gpu.interior_style_evaluations(),
            96,
            "day/night changes invalidate the bank"
        );
    }
    #[test]
    #[ignore = "requires a real wgpu GPU adapter"]
    fn culled_local_lights_and_shadows_match_full_scan_with_motion_and_fades() {
        use crate::{
            Deformation3D, Dimming3D, Environment3D, Instance3D, LocalLight3D, LocalLighting3D,
            ShadowProxy3D, Visibility3D,
        };
        let mut ui = Renderer::new_headless().unwrap();
        let mut asset = crate::asset::tests::depth_fixture(false);
        asset.meshes[0].colors.fill([1.; 4]);
        let view = Scene3D::new(asset);
        // Reference keeps the original full scan and exact shadow arithmetic.
        let reference = include_str!("local_lighting.wgsl")
            .replace(
                "var candidates=light_candidates(p);",
                "var candidates=vec2(0xffffffffu);",
            )
            .replace(
                "candidate<local_scene.shadow_lists[j].count",
                "candidate<local_scene.counts.y",
            )
            .replace(
                "let k=local_scene.shadow_lists[j].indices[candidate];",
                "let k=candidate;",
            );
        let mut gpu = Renderer3D::new_antialiased(&ui.gpu_context(), &view, (192, 192)).unwrap();
        let mut full =
            Renderer3D::with_lighting_shader(&ui.gpu_context(), &view, (192, 192), 4, &reference)
                .unwrap();
        let actual_image = ui.register_gpu_image(gpu.output()).unwrap();
        let expected_image = ui.register_gpu_image(full.output()).unwrap();
        let mut recipe = LocalLighting3D::default();
        for i in 0..64 {
            recipe.lights.push(LocalLight3D {
                attachment: Instance3D {
                    deformation: Deformation3D::Swing {
                        amplitude: 0.3,
                        frequency: 1.,
                        phase: i as f32 * 0.1,
                        pivot: [0.; 3],
                        axis: [0., 0., 1.],
                    },
                    ..Default::default()
                },
                position: [
                    ((i % 8) as f32 - 3.5) * 0.9,
                    ((i / 8) as f32 - 3.5) * 0.8,
                    0.7,
                ],
                direction: [0., 0., -1.],
                color: [0.06, 0.04, 0.03],
                radius: 0.6 + (i % 5) as f32 * 0.7,
                outer_cos: if i % 2 == 0 { -1. } else { 0.5 },
                dimming: Some(Dimming3D {
                    clock: 0,
                    start_seconds: 0.,
                    end_seconds: 10.,
                    minimum: 0.1,
                }),
            });
        }
        for i in 0..96 {
            recipe.shadows.push(ShadowProxy3D {
                attachment: Instance3D {
                    scale: [0.7, 1.3, 0.9],
                    deformation: Deformation3D::Swing {
                        amplitude: 0.45,
                        frequency: 1.2,
                        phase: i as f32 * 0.2,
                        pivot: [0.; 3],
                        axis: [0., 0., 1.],
                    },
                    visibility: Some(Visibility3D {
                        clock: 0,
                        start_seconds: 2.,
                        fade_seconds: 4.,
                    }),
                    ..Default::default()
                },
                center: [
                    ((i % 12) as f32 - 5.5) * 0.29,
                    ((i / 12) as f32 - 3.5) * 0.3,
                    0.25,
                ],
                radii: [0.10, 0.16, 0.07],
            });
        }
        let mut shadow_difference = 0;
        for time in [0., 3., 5.9, 6., 10.] {
            for renderer in [&mut gpu, &mut full] {
                renderer.configure_local_lighting(&recipe).unwrap();
                renderer.set_motion_clocks([time, 0., 0., 0.]).unwrap();
                renderer
                    .set_environment(Environment3D {
                        time_seconds: time,
                        ..Default::default()
                    })
                    .unwrap();
                renderer.render(&view, 0.3, 1.).unwrap();
            }
            let actual = capture(&mut ui, &view, actual_image);
            let expected = capture(&mut ui, &view, expected_image);
            let maximum_error = actual
                .iter()
                .zip(&expected)
                .map(|(a, b)| a.abs_diff(*b))
                .max()
                .unwrap();
            assert!(
                maximum_error <= 1,
                "full-scan reference differs at t={time}: max={maximum_error}"
            );
            if time == 0. {
                let mut unshadowed = recipe.clone();
                unshadowed.shadows.clear();
                full.configure_local_lighting(&unshadowed).unwrap();
                full.render(&view, 0.3, 1.).unwrap();
                shadow_difference = actual
                    .iter()
                    .zip(capture(&mut ui, &view, expected_image))
                    .filter(|(a, b)| a.abs_diff(*b) > 2)
                    .count();
            }
        }
        assert!(
            shadow_difference > 100,
            "fixture must exercise visible shadows"
        );
        // Exercise both bit-mask halves and reconfiguration without stale lights.
        for count in [1, 31, 32, 33, 63] {
            let mut subset = recipe.clone();
            subset.lights.truncate(count);
            for renderer in [&mut gpu, &mut full] {
                renderer.configure_local_lighting(&subset).unwrap();
                renderer.render(&view, 0.3, 1.).unwrap();
            }
            let actual = capture(&mut ui, &view, actual_image);
            let expected = capture(&mut ui, &view, expected_image);
            assert!(
                actual.iter().zip(expected).all(|(a, b)| a.abs_diff(b) <= 1),
                "light mask count {count}"
            );
        }
        // Reconfigure to zero lights/shadows; no stale candidate entries may leak.
        for renderer in [&mut gpu, &mut full] {
            renderer
                .configure_local_lighting(&LocalLighting3D::default())
                .unwrap();
            renderer.render(&view, 0.3, 1.).unwrap();
        }
        assert_eq!(
            capture(&mut ui, &view, actual_image),
            capture(&mut ui, &view, expected_image)
        );
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
    fn glass_alpha_is_premultiplied_in_ui_space_and_skipped_meshes_are_safe() {
        let mut ui = Renderer::new_headless().unwrap();
        let mut asset = crate::asset::tests::depth_fixture(false);
        let mut skipped = asset.meshes[0].clone();
        skipped.indices.clear();
        asset.meshes.insert(0, skipped);
        let opaque = Scene3D::new(asset);
        let mut opaque_gpu = Renderer3D::new(&ui.gpu_context(), &opaque, (192, 192)).unwrap();
        opaque_gpu.render(&opaque, 0., 1.).unwrap();
        let opaque_id = ui.register_gpu_image(opaque_gpu.output()).unwrap();
        let expected = capture(&mut ui, &opaque, opaque_id);
        let middle = (96 * 192 + 96) * 4;
        assert_eq!(expected[middle + 3], 255);
        for msaa in [false, true] {
            let mut asset = crate::asset::tests::depth_fixture(false);
            let mut skipped = asset.meshes[0].clone();
            skipped.indices.clear();
            asset.meshes.insert(0, skipped);
            asset.materials[0].blend = true;
            asset.materials[0].color[3] = 0.25;
            let view = Scene3D::new(asset);
            let mut gpu = if msaa {
                Renderer3D::new_antialiased(&ui.gpu_context(), &view, (192, 192))
            } else {
                Renderer3D::new(&ui.gpu_context(), &view, (192, 192))
            }
            .unwrap();
            let id = ui.register_gpu_image(gpu.output()).unwrap();
            for size in [(192, 192), (240, 180)] {
                gpu.resize(size).unwrap();
                ui.replace_gpu_image(id, gpu.output()).unwrap();
                gpu.render(&view, 0., 1.).unwrap();
                let actual = capture(&mut ui, &view, id);
                assert!(actual[middle + 3].abs_diff(64) <= 1);
                assert!(
                    actual[middle].abs_diff((expected[middle] as f32 * 0.25).round() as u8) <= 3,
                    "glass over transparent UI must not brighten: {:?}",
                    &actual[middle..middle + 4]
                );
                assert_eq!(actual[middle + 1], 0);
                assert_eq!(actual[middle + 2], 0);
                gpu.render(&view, 0.2, 1.).unwrap();
                assert_eq!(gpu.last_frame_pose_upload_bytes(), 0);
                assert_eq!(gpu.last_frame_deformed_meshes(), 0);
            }
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
                        "{}\n{}\n{}\n{}\n{}\n{}",
                        include_str!("instance_motion.wgsl"),
                        include_str!("local_lighting.wgsl"),
                        include_str!("interior_styles.wgsl"),
                        include_str!("interior.wgsl"),
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
        let geometry = gpu.flat_pipeline.get_bind_group_layout(3);
        let layout = gpu
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[
                    Some(&camera),
                    Some(&material),
                    Some(&gpu.local_lighting.layout),
                ],
                immediate_size: 0,
            });
        let flat_layout = gpu
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[
                    Some(&camera),
                    Some(&material),
                    Some(&gpu.local_lighting.layout),
                    Some(&geometry),
                ],
                immediate_size: 0,
            });
        gpu.pipeline = render_pipeline(
            &gpu.device,
            &shader,
            &layout,
            false,
            "fs_test_normals",
            1,
            false,
        );
        gpu.flat_pipeline = render_pipeline(
            &gpu.device,
            &shader,
            &flat_layout,
            true,
            "fs_test_normals",
            1,
            false,
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
            pass.set_bind_group(2, &gpu.local_lighting.bind, &[]);
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
