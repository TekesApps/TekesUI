//! Immutable mesh storage and GPU deformation. No per-frame vertex traversal,
//! vertex upload, buffer allocation, readback or CPU synchronization.
use crate::{Asset3D, Error, asset::Mesh, err};
use bytemuck::{Pod, Zeroable};
use glam::Mat4;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct BaseVertex {
    position: [f32; 4],
    normal: [f32; 4],
    uv: [f32; 4],
    color: [f32; 4],
    joints: [u32; 4],
    weights: [f32; 4],
}
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct Delta {
    position: [f32; 4],
    normal: [f32; 4],
}
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub(crate) struct GpuVertex {
    pub position: [f32; 4],
    pub normal: [f32; 4],
    pub uv: [f32; 4],
    pub color: [f32; 4],
}
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
struct Params {
    model: [f32; 16],
    counts: [u32; 4],
}

#[derive(Debug)]
pub(crate) struct MeshDeformer {
    pub(crate) mesh_index: usize,
    last_params: Option<Params>,
    last_palette: Vec<[f32; 16]>,
    last_weights: Vec<f32>,
    dirty: bool,
    pub material: usize,
    pub vertex_count: u32,
    pub index_count: u32,
    pub indices: wgpu::Buffer,
    pub flat_normals: bool,
    pub output: wgpu::Buffer,
    palette: wgpu::Buffer,
    weights: wgpu::Buffer,
    params: wgpu::Buffer,
    bind: wgpu::BindGroup,
}
#[derive(Debug)]
pub(crate) struct Deformer {
    pipeline: wgpu::ComputePipeline,
    last_pose: Option<(Option<usize>, f32)>,
    pub meshes: Vec<MeshDeformer>,
    pub geometry_upload_bytes: u64,
    pub geometry_resident_bytes: u64,
}
impl Deformer {
    pub fn new(device: &wgpu::Device, asset: &Asset3D) -> Result<Self, Error> {
        let limits = device.limits();
        if limits.max_storage_buffers_per_shader_stage < 5
            || limits.max_compute_workgroup_size_x < 64
            || limits.max_compute_invocations_per_workgroup < 64
        {
            return Err(err(
                "GPU deformation requires five compute storage bindings and 64-thread workgroups",
            ));
        }
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("3D deformation layout"),
            entries: &(0..6)
                .map(|binding| wgpu::BindGroupLayoutEntry {
                    binding,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: if binding == 5 {
                            wgpu::BufferBindingType::Uniform
                        } else {
                            wgpu::BufferBindingType::Storage {
                                read_only: binding != 4,
                            }
                        },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                })
                .collect::<Vec<_>>(),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("3D deformation"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("3D morph and skin"),
            source: wgpu::ShaderSource::Wgsl(include_str!("deform.wgsl").into()),
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("3D GPU deformation"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("deform"),
            compilation_options: Default::default(),
            cache: None,
        });
        let pose = asset.sample_pose(None, 0.);
        let mut meshes = Vec::new();
        let mut geometry_upload_bytes = 0;
        let mut geometry_resident_bytes = 0;
        for (index, mesh) in asset.meshes.iter().enumerate() {
            if !pose.active[mesh.node] || mesh.indices.is_empty() {
                continue;
            }
            let count = mesh.positions.len();
            check_storage(device, mesh.indices.len() as u64 * 4)?;
            let delta_count = count
                .checked_mul(mesh.morphs.len())
                .ok_or_else(|| err("morph storage size overflow"))?;
            check_storage(
                device,
                count as u64 * std::mem::size_of::<BaseVertex>() as u64,
            )?;
            check_storage(
                device,
                count as u64 * std::mem::size_of::<GpuVertex>() as u64,
            )?;
            check_storage(
                device,
                delta_count.max(1) as u64 * std::mem::size_of::<Delta>() as u64,
            )?;
            let workgroups = (count as u64).div_ceil(64);
            if count > u32::MAX as usize
                || mesh.indices.len() > u32::MAX as usize
                || mesh.morphs.len() > u32::MAX as usize
                || workgroups > limits.max_compute_workgroups_per_dimension as u64
            {
                return Err(err("mesh exceeds GPU deformation dispatch limits"));
            }
            let vertices: Vec<_> = (0..count)
                .map(|i| BaseVertex {
                    position: mesh.positions[i].extend(1.).to_array(),
                    normal: mesh
                        .normals
                        .get(i)
                        .copied()
                        .unwrap_or_default()
                        .extend(0.)
                        .to_array(),
                    uv: [mesh.uv[i][0], mesh.uv[i][1], 0., 0.],
                    color: mesh.colors[i],
                    joints: mesh
                        .joints
                        .get(i)
                        .copied()
                        .unwrap_or_default()
                        .map(u32::from),
                    weights: mesh.weights.get(i).copied().unwrap_or_default(),
                })
                .collect();
            let mut deltas: Vec<_> = mesh
                .morphs
                .iter()
                .flat_map(|(p, n)| {
                    p.iter().zip(n).map(|(p, n)| Delta {
                        position: p.extend(0.).to_array(),
                        normal: n.extend(0.).to_array(),
                    })
                })
                .collect();
            if deltas.is_empty() {
                deltas.push(Delta::zeroed());
            }
            let base = storage(
                device,
                "3D immutable base geometry",
                bytemuck::cast_slice(&vertices),
                false,
            )?;
            let deltas = storage(
                device,
                "3D immutable morph deltas",
                bytemuck::cast_slice(&deltas),
                false,
            )?;
            let indices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("3D immutable indices"),
                contents: bytemuck::cast_slice(&mesh.indices),
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::STORAGE,
            });
            geometry_upload_bytes += base.size() + deltas.size() + indices.size();
            let joint_count = mesh.skin.map_or(1, |s| asset.skins[s].joints.len()).max(1);
            let palette = storage(
                device,
                "3D joint palette",
                bytemuck::cast_slice(&vec![Mat4::IDENTITY.to_cols_array(); joint_count]),
                true,
            )?;
            let weights = storage(
                device,
                "3D morph weights",
                bytemuck::cast_slice(&vec![0_f32; mesh.morphs.len().max(1)]),
                true,
            )?;
            let params = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("3D mesh parameters"),
                contents: bytemuck::bytes_of(&mesh_params(mesh, Mat4::IDENTITY)),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });
            let output_usage = wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::VERTEX;
            #[cfg(test)]
            let output_usage = output_usage | wgpu::BufferUsages::COPY_SRC;
            let output = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("3D GPU deformed vertices"),
                size: count as u64 * std::mem::size_of::<GpuVertex>() as u64,
                usage: output_usage,
                mapped_at_creation: false,
            });
            geometry_resident_bytes += base.size() + deltas.size() + indices.size() + output.size();
            let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("3D deformation mesh"),
                layout: &layout,
                entries: &[&base, &deltas, &palette, &weights, &output, &params]
                    .into_iter()
                    .enumerate()
                    .map(|(binding, buffer)| wgpu::BindGroupEntry {
                        binding: binding as u32,
                        resource: buffer.as_entire_binding(),
                    })
                    .collect::<Vec<_>>(),
            });
            meshes.push(MeshDeformer {
                mesh_index: index,
                last_params: None,
                last_palette: Vec::new(),
                last_weights: Vec::new(),
                dirty: true,
                material: mesh.material,
                vertex_count: count as u32,
                index_count: mesh.indices.len() as u32,
                indices,
                flat_normals: mesh.normals.is_empty(),
                output,
                palette,
                weights,
                params,
                bind,
            });
        }
        Ok(Self {
            pipeline,
            last_pose: None,
            meshes,
            geometry_upload_bytes,
            geometry_resident_bytes,
        })
    }
    /// Only compact pose values cross the CPU/GPU boundary after initialization.
    pub fn update_pose(
        &mut self,
        queue: &wgpu::Queue,
        asset: &Asset3D,
        clip: Option<usize>,
        time: f32,
    ) -> u64 {
        let key = (clip, if clip.is_some() { time } else { 0. });
        for draw in &mut self.meshes {
            draw.dirty = false;
        }
        if self.last_pose == Some(key) {
            return 0;
        }
        self.last_pose = Some(key);
        let pose = asset.sample_pose(clip, time);
        let mut bytes = 0;
        for draw in &mut self.meshes {
            let mesh = &asset.meshes[draw.mesh_index];
            let params = mesh_params(mesh, pose.world[mesh.node]);
            if draw.last_params != Some(params) {
                queue.write_buffer(&draw.params, 0, bytemuck::bytes_of(&params));
                bytes += std::mem::size_of::<Params>() as u64;
                draw.last_params = Some(params);
                draw.dirty = true;
            }
            if let Some(s) = mesh.skin {
                let skin = &asset.skins[s];
                let palette: Vec<_> = skin
                    .joints
                    .iter()
                    .zip(&skin.inverse)
                    .map(|(joint, bind)| (pose.world[*joint] * *bind).to_cols_array())
                    .collect();
                if draw.last_palette != palette {
                    let data = bytemuck::cast_slice(&palette);
                    queue.write_buffer(&draw.palette, 0, data);
                    bytes += data.len() as u64;
                    draw.last_palette = palette;
                    draw.dirty = true;
                }
            }
            if !mesh.morphs.is_empty() {
                let weights: Vec<_> = (0..mesh.morphs.len())
                    .map(|i| pose.weights[mesh.node].get(i).copied().unwrap_or(0.))
                    .collect();
                if draw.last_weights != weights {
                    let data = bytemuck::cast_slice(&weights);
                    queue.write_buffer(&draw.weights, 0, data);
                    bytes += data.len() as u64;
                    draw.last_weights = weights;
                    draw.dirty = true;
                }
            }
        }
        bytes
    }
    pub fn dirty_mesh_count(&self) -> usize {
        self.meshes.iter().filter(|mesh| mesh.dirty).count()
    }
    /// Pass boundary makes storage writes visible to the subsequent vertex pass.
    pub fn encode(&self, encoder: &mut wgpu::CommandEncoder) {
        if !self.meshes.iter().any(|mesh| mesh.dirty) {
            return;
        }
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("3D morph + skin"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&self.pipeline);
        for mesh in self.meshes.iter().filter(|mesh| mesh.dirty) {
            pass.set_bind_group(0, &mesh.bind, &[]);
            pass.dispatch_workgroups(mesh.vertex_count.div_ceil(64), 1, 1);
        }
    }
}
fn mesh_params(mesh: &Mesh, model: Mat4) -> Params {
    Params {
        model: model.to_cols_array(),
        counts: [
            mesh.positions.len() as u32,
            mesh.morphs.len() as u32,
            u32::from(mesh.skin.is_some()),
            u32::from(mesh.normals.is_empty()),
        ],
    }
}
fn check_storage(device: &wgpu::Device, bytes: u64) -> Result<(), Error> {
    if bytes > device.limits().max_buffer_size
        || bytes > device.limits().max_storage_buffer_binding_size
    {
        return Err(err("mesh storage exceeds device limits"));
    }
    Ok(())
}
fn storage(
    device: &wgpu::Device,
    label: &str,
    data: &[u8],
    dynamic: bool,
) -> Result<wgpu::Buffer, Error> {
    check_storage(device, data.len() as u64)?;
    let usage = wgpu::BufferUsages::STORAGE
        | if dynamic {
            wgpu::BufferUsages::COPY_DST
        } else {
            wgpu::BufferUsages::empty()
        };
    Ok(
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents: data,
            usage,
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tekes_ui_render::Renderer;

    // Readback belongs only to this differential test. Production output buffers
    // do not have COPY_SRC or COPY_DST usage, and production has no map/poll.
    fn capture(context: &tekes_ui_render::gpu::GpuContext, deformer: &Deformer) -> Vec<GpuVertex> {
        let size: u64 = deformer.meshes.iter().map(|m| m.output.size()).sum();
        let readback = context.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("test-only deformed geometry readback"),
            size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = context.device().create_command_encoder(&Default::default());
        deformer.encode(&mut encoder);
        let mut offset = 0;
        for mesh in &deformer.meshes {
            assert!(!mesh.output.usage().contains(wgpu::BufferUsages::COPY_DST));
            encoder.copy_buffer_to_buffer(&mesh.output, 0, &readback, offset, mesh.output.size());
            offset += mesh.output.size();
        }
        context.queue().submit([encoder.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        readback
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                tx.send(result).unwrap();
            });
        context
            .device()
            .poll(wgpu::PollType::wait_indefinitely())
            .unwrap();
        rx.recv().unwrap().unwrap();
        let data = readback.slice(..).get_mapped_range().unwrap();
        let vertices = bytemuck::cast_slice(&data).to_vec();
        drop(data);
        readback.unmap();
        vertices
    }

    #[test]
    #[ignore = "requires a real wgpu GPU adapter"]
    fn gpu_morph_and_skin_match_cpu_geometry_including_normals() {
        let ui = Renderer::new_headless().unwrap();
        let context = ui.gpu_context();
        let cases = [
            Asset3D::from_slice(include_bytes!("../fixtures/animated-tetra.glb")).unwrap(),
            Asset3D::from_slice(include_bytes!("../fixtures/skinned-tetra.glb")).unwrap(),
            crate::asset::tests::deformation_fixture(true, false),
            crate::asset::tests::deformation_fixture(false, false),
            crate::asset::tests::deformation_fixture(true, true),
            crate::asset::tests::seam_fixture(),
        ];
        for (case, asset) in cases.iter().enumerate() {
            let mut deformer = Deformer::new(context.device(), asset).unwrap();
            for clip in std::iter::once(None).chain((0..asset.clips.len()).map(Some)) {
                for time in [0., 0.25, 0.5, 1., 1.5, 2.] {
                    deformer.update_pose(context.queue(), asset, clip, time);
                    let actual = capture(&context, &deformer);
                    let expected = asset.pose(clip, time);
                    let expected: Vec<_> = expected.iter().flat_map(|d| &d.vertices).collect();
                    let mut indexed = Vec::new();
                    let mut flat_flags = Vec::new();
                    let mut offset = 0;
                    for draw in &deformer.meshes {
                        let mesh = &asset.meshes[draw.mesh_index];
                        for &index in &mesh.indices {
                            indexed.push(actual[offset + index as usize]);
                            flat_flags.push(draw.flat_normals);
                        }
                        offset += draw.vertex_count as usize;
                    }
                    assert_eq!(indexed.len(), expected.len());
                    for (i, (a, e)) in indexed.iter().zip(expected).enumerate() {
                        for (label, values, reference) in [
                            ("position", a.position[..3].to_vec(), e.position.to_vec()),
                            ("normal", a.normal[..3].to_vec(), e.normal.to_vec()),
                            ("uv", a.uv[..2].to_vec(), e.uv.to_vec()),
                            ("color", a.color.to_vec(), e.color.to_vec()),
                        ] {
                            // A shared vertex has no unique flat normal. That path's
                            // per-face shader normals are checked by render comparisons.
                            if label == "normal" && flat_flags[i] {
                                continue;
                            }
                            for (actual, reference) in values.into_iter().zip(reference) {
                                assert!(
                                    actual.is_finite() && (actual - reference).abs() < 5e-5,
                                    "case={case} clip={clip:?} t={time} vertex={i} {label}: GPU={actual} CPU={reference}"
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    #[ignore = "requires a real wgpu GPU adapter"]
    fn shared_indices_preserve_geometry_and_frame_uploads() {
        let ui = Renderer::new_headless().unwrap();
        let context = ui.gpu_context();
        let small = crate::asset::tests::deformation_fixture(true, false);
        let mut large = crate::asset::tests::deformation_fixture(true, false);
        large.meshes[0].indices = large.meshes[0].indices.repeat(4096);
        let mut a = Deformer::new(context.device(), &small).unwrap();
        let mut b = Deformer::new(context.device(), &large).unwrap();
        assert_eq!(a.geometry_resident_bytes, 944);
        let extra_indices =
            (large.meshes[0].indices.len() - small.meshes[0].indices.len()) as u64 * 4;
        assert_eq!(
            b.geometry_upload_bytes,
            a.geometry_upload_bytes + extra_indices
        );
        assert_eq!(
            b.geometry_resident_bytes,
            a.geometry_resident_bytes + extra_indices
        );
        assert_eq!(b.meshes[0].output.size(), a.meshes[0].output.size());
        let expanded_bytes = large.meshes[0].indices.len() as u64 * (96 + 64 + 32 * 2);
        assert!(b.geometry_resident_bytes < expanded_bytes / 50);
        assert_eq!(b.geometry_resident_bytes, 197_504);
        let output = b.meshes[0].output.clone();
        for t in [0., 0.1, 0.5, 1., 1.5, 2.] {
            let a_bytes = a.update_pose(context.queue(), &small, Some(0), t);
            let b_bytes = b.update_pose(context.queue(), &large, Some(0), t);
            assert!((1..=216).contains(&a_bytes)); // Unchanged model metadata is skipped.
            assert_eq!(a_bytes, b_bytes);
            assert_eq!(b.meshes[0].output, output);
            let mut encoder = context.device().create_command_encoder(&Default::default());
            a.encode(&mut encoder);
            b.encode(&mut encoder);
            context.queue().submit([encoder.finish()]);
        }
        // A final assertion capture is permitted, but no frame in the loop waits.
        let result = capture(&context, &b);
        assert_eq!(result.len(), large.meshes[0].positions.len());
    }
    #[test]
    #[ignore = "requires a real wgpu GPU adapter"]
    fn vertex_count_does_not_change_frame_uploads() {
        let ui = Renderer::new_headless().unwrap();
        let context = ui.gpu_context();
        let small = crate::asset::tests::deformation_fixture(true, false);
        let mut large = crate::asset::tests::deformation_fixture(true, false);
        crate::asset::tests::repeat_geometry(&mut large.meshes[0], 4096);
        let mut a = Deformer::new(context.device(), &small).unwrap();
        let mut b = Deformer::new(context.device(), &large).unwrap();
        assert_eq!(b.meshes[0].vertex_count, a.meshes[0].vertex_count * 4096);
        assert_eq!(b.geometry_resident_bytes, a.geometry_resident_bytes * 4096);
        for time in [0., 0.5, 1., 2.] {
            let uploaded = a.update_pose(context.queue(), &small, Some(0), time);
            assert!((1..=216).contains(&uploaded));
            assert_eq!(
                uploaded,
                b.update_pose(context.queue(), &large, Some(0), time)
            );
            let mut encoder = context.device().create_command_encoder(&Default::default());
            a.encode(&mut encoder);
            b.encode(&mut encoder);
            context.queue().submit([encoder.finish()]);
        }
        assert_eq!(capture(&context, &b).len(), large.meshes[0].positions.len());
    }
}
