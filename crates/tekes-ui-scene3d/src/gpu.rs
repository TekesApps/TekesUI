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
    flat_bindings: Vec<wgpu::BindGroup>,
    camera: wgpu::Buffer,
    camera_bind: wgpu::BindGroup,
    materials: Vec<wgpu::BindGroup>,
    deformer: Deformer,
    last_frame_pose_upload_bytes: u64,
    size: (u32, u32),
    color: GpuImage,
    depth: wgpu::Texture,
    msaa: Option<wgpu::Texture>,
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
            entries: &[buffer_entry(0, wgpu::ShaderStages::VERTEX)],
        });
        let camera = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("3D camera"),
            size: 128,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let camera_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &camera_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera.as_entire_binding(),
            }],
        });
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
            source: wgpu::ShaderSource::Wgsl(include_str!("scene3d.wgsl").into()),
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
            flat_bindings,
            camera,
            camera_bind,
            materials,
            deformer,
            last_frame_pose_upload_bytes: 0,
            size,
            color,
            depth,
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
        let asset = &component.asset;
        if self.asset_id != asset.id {
            return Err(err("renderer asset mismatch"));
        }
        let matrices = camera.matrices(
            asset.center,
            asset.radius,
            self.size.0 as f32 / self.size.1 as f32,
        )?;
        self.queue
            .write_buffer(&self.camera, 0, bytemuck::cast_slice(&matrices));
        self.last_frame_pose_upload_bytes =
            self.deformer
                .update_pose(&self.queue, asset, component.clip, component.time);
        let mut encoder = self.device.create_command_encoder(&Default::default());
        self.deformer.encode(&mut encoder);
        let msaa_view = self
            .msaa
            .as_ref()
            .map(|t| t.create_view(&Default::default()));
        let color_view = msaa_view.as_ref().unwrap_or(self.color.render_view());
        let depth_view = self.depth.create_view(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Scene3D"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: color_view,
                    depth_slice: None,
                    resolve_target: msaa_view.as_ref().map(|_| self.color.render_view()),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(0.),
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            for (i, draw) in self.deformer.meshes.iter().enumerate() {
                pass.set_pipeline(if draw.flat_normals {
                    &self.flat_pipeline
                } else {
                    &self.pipeline
                });
                pass.set_bind_group(0, &self.camera_bind, &[]);
                pass.set_bind_group(1, &self.materials[draw.material], &[]);
                if draw.flat_normals {
                    pass.set_bind_group(2, &self.flat_bindings[i], &[]);
                    pass.draw(0..draw.index_count, 0..1);
                } else {
                    pass.set_vertex_buffer(0, draw.output.slice(..));
                    pass.set_index_buffer(draw.indices.slice(..), wgpu::IndexFormat::Uint32);
                    pass.draw_indexed(0..draw.index_count, 0, 0..1);
                }
            }
        }
        self.queue.submit([encoder.finish()]);
        Ok(())
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
        address_mode_u: wgpu::AddressMode::Repeat,
        address_mode_v: wgpu::AddressMode::Repeat,
        ..Default::default()
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
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("3D base color"),
                size: extent(size),
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
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
                        "{}\n{}",
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
