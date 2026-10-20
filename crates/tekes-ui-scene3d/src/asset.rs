use crate::{Error, animation::Channel, err};
use glam::{Mat4, Quat, Vec3};
use gltf::{animation::util::ReadOutputs, mesh::Mode};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct ClipInfo {
    pub name: String,
    pub duration: f32,
}
#[derive(Debug)]
pub(crate) struct Clip {
    pub info: ClipInfo,
    pub channels: Vec<Channel>,
}
#[derive(Debug, Clone)]
struct Node {
    matrix: Mat4,
    t: Vec3,
    r: Quat,
    s: Vec3,
    weights: Vec<f32>,
    children: Vec<usize>,
}
#[derive(Debug)]
pub(crate) struct Skin {
    pub joints: Vec<usize>,
    pub inverse: Vec<Mat4>,
}
#[derive(Debug, Clone)]
pub(crate) struct Mesh {
    pub node: usize,
    pub skin: Option<usize>,
    pub positions: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    pub joints: Vec<[u16; 4]>,
    pub weights: Vec<[f32; 4]>,
    pub uv: Vec<[f32; 2]>,
    pub colors: Vec<[f32; 4]>,
    pub morphs: Vec<(Vec<Vec3>, Vec<Vec3>)>,
    pub indices: Vec<u32>,
    pub material: usize,
}
#[derive(Debug)]
pub(crate) struct Material {
    pub color: [f32; 4],
    pub image: Option<usize>,
    pub cutoff: f32,
}
#[derive(Debug)]
pub(crate) struct Image {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    pub color: [f32; 4],
}
#[derive(Debug)]
pub(crate) struct Draw {
    pub vertices: Vec<Vertex>,
}

/// Compact animated state, independent of the number of mesh vertices.
#[derive(Debug)]
pub(crate) struct PoseFrame {
    pub world: Vec<Mat4>,
    pub active: Vec<bool>,
    pub weights: Vec<Vec<f32>>,
}

/// Imported local glTF 2.0 scene, independent of a GPU or UI host.
/// `.gltf` resolves adjacent resources; `.glb` can be self-contained.
#[derive(Debug)]
pub struct Asset3D {
    pub(crate) node_names: Vec<String>,
    pub(crate) id: u64,
    nodes: Vec<Node>,
    roots: Vec<usize>,
    pub(crate) meshes: Vec<Mesh>,
    pub(crate) skins: Vec<Skin>,
    pub(crate) clips: Vec<Clip>,
    pub(crate) materials: Vec<Material>,
    pub(crate) images: Vec<Image>,
    pub(crate) center: Vec3,
    pub(crate) radius: f32,
}
impl Asset3D {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, Error> {
        let path = path.as_ref();
        if !path
            .extension()
            .and_then(|s| s.to_str())
            .is_some_and(|s| s.eq_ignore_ascii_case("glb") || s.eq_ignore_ascii_case("gltf"))
        {
            return Err(err("supported formats: .glb and .gltf (glTF 2.0)"));
        }
        let (document, buffers, images) =
            gltf::import(path).map_err(|e| err(format!("{}: {e}", path.display())))?;
        Self::import(document, buffers, images)
    }
    /// Load an embedded GLB or glTF with data URIs. External files need `load`.
    pub fn from_slice(bytes: &[u8]) -> Result<Self, Error> {
        let g = gltf::Gltf::from_slice(bytes).map_err(|e| err(e.to_string()))?;
        // gltf 1.4's image importer requires a base even for data URIs. Reject
        // external images before supplying a base, so slice loading never reads
        // an ambient working-directory file.
        if g.document.images().any(|i| matches!(i.source(), gltf::image::Source::Uri { uri, .. } if !uri.starts_with("data:"))) {
            return Err(err("external images require Asset3D::load with a source path"));
        }
        let buffers =
            gltf::import_buffers(&g.document, None, g.blob).map_err(|e| err(e.to_string()))?;
        let images = gltf::import_images(&g.document, Some(Path::new("")), &buffers)
            .map_err(|e| err(e.to_string()))?;
        Self::import(g.document, buffers, images)
    }
    pub fn clips(&self) -> impl ExactSizeIterator<Item = &ClipInfo> {
        self.clips.iter().map(|c| &c.info)
    }
    pub fn mesh_count(&self) -> usize {
        self.meshes.len()
    }
    fn import(
        d: gltf::Document,
        buffers: Vec<gltf::buffer::Data>,
        images: Vec<gltf::image::Data>,
    ) -> Result<Self, Error> {
        if let Some(extension) = d.extensions_required().next() {
            return Err(err(format!("unsupported required extension: {extension}")));
        }
        let data = |b: gltf::Buffer<'_>| Some(buffers[b.index()].0.as_slice());
        let scene = d
            .default_scene()
            .or_else(|| d.scenes().next())
            .ok_or_else(|| err("asset has no scene"))?;
        let roots = scene.nodes().map(|n| n.index()).collect();
        let nodes = d
            .nodes()
            .map(|n| {
                let (t, r, s) = n.transform().decomposed();
                Node {
                    matrix: Mat4::from_cols_array_2d(&n.transform().matrix()),
                    t: t.into(),
                    r: Quat::from_array(r),
                    s: s.into(),
                    weights: n
                        .weights()
                        .or_else(|| n.mesh().and_then(|m| m.weights()))
                        .unwrap_or(&[])
                        .to_vec(),
                    children: n.children().map(|c| c.index()).collect(),
                }
            })
            .collect();
        let skins = d
            .skins()
            .map(|s| {
                let joints: Vec<_> = s.joints().map(|j| j.index()).collect();
                let inverse = s
                    .reader(data)
                    .read_inverse_bind_matrices()
                    .map(|it| it.map(|m| Mat4::from_cols_array_2d(&m)).collect())
                    .unwrap_or_else(|| vec![Mat4::IDENTITY; joints.len()]);
                Skin { joints, inverse }
            })
            .collect();
        let mut materials: Vec<_> = d
            .materials()
            .map(|m| {
                let pbr = m.pbr_metallic_roughness();
                if m.alpha_mode() == gltf::material::AlphaMode::Blend {
                    return Err(err(
                        "alpha BLEND materials are not yet supported; use OPAQUE or MASK",
                    ));
                }
                if pbr.base_color_texture().is_some_and(|t| t.tex_coord() != 0) {
                    return Err(err("only TEXCOORD_0 base color textures are supported"));
                }
                Ok(Material {
                    color: pbr.base_color_factor(),
                    image: pbr
                        .base_color_texture()
                        .map(|t| t.texture().source().index()),
                    cutoff: if m.alpha_mode() == gltf::material::AlphaMode::Mask {
                        m.alpha_cutoff().unwrap_or(0.5)
                    } else {
                        -1.
                    },
                })
            })
            .collect::<Result<_, Error>>()?;
        materials.push(Material {
            color: [1.; 4],
            image: None,
            cutoff: -1.,
        });
        let mut meshes = Vec::new();
        for node in d.nodes() {
            if let Some(mesh) = node.mesh() {
                for p in mesh.primitives() {
                    if p.mode() != Mode::Triangles {
                        return Err(err("only TRIANGLES primitives are supported"));
                    }
                    if p.get(&gltf::Semantic::Joints(1)).is_some() {
                        return Err(err(
                            "preview supports at most four skin influences (JOINTS_0/WEIGHTS_0)",
                        ));
                    }
                    let reader = p.reader(data);
                    let positions: Vec<Vec3> = reader
                        .read_positions()
                        .ok_or_else(|| err("mesh missing POSITION"))?
                        .map(Vec3::from_array)
                        .collect();
                    let n = positions.len();
                    let indices: Vec<_> = reader
                        .read_indices()
                        .map(|i| i.into_u32().collect())
                        .unwrap_or_else(|| (0..n as u32).collect());
                    if indices.len() % 3 != 0 || indices.iter().any(|i| *i as usize >= n) {
                        return Err(err("invalid triangle indices"));
                    }
                    let normals = reader
                        .read_normals()
                        .map(|i| i.map(Vec3::from_array).collect())
                        .unwrap_or_default();
                    let uv = reader
                        .read_tex_coords(0)
                        .map(|i| i.into_f32().collect())
                        .unwrap_or_else(|| vec![[0.; 2]; n]);
                    let colors = reader
                        .read_colors(0)
                        .map(|i| i.into_rgba_f32().collect())
                        .unwrap_or_else(|| vec![[1.; 4]; n]);
                    let joints = reader
                        .read_joints(0)
                        .map(|i| i.into_u16().collect())
                        .unwrap_or_default();
                    let weights = reader
                        .read_weights(0)
                        .map(|i| i.into_f32().collect())
                        .unwrap_or_default();
                    let morphs = reader
                        .read_morph_targets()
                        .map(|(pos, norm, _)| {
                            (
                                pos.map(|i| i.map(Vec3::from_array).collect())
                                    .unwrap_or_else(|| vec![Vec3::ZERO; n]),
                                norm.map(|i| i.map(Vec3::from_array).collect())
                                    .unwrap_or_else(|| vec![Vec3::ZERO; n]),
                            )
                        })
                        .collect();
                    meshes.push(Mesh {
                        node: node.index(),
                        skin: node.skin().map(|s| s.index()),
                        positions,
                        normals,
                        joints,
                        weights,
                        uv,
                        colors,
                        morphs,
                        indices,
                        material: p.material().index().unwrap_or(materials.len() - 1),
                    });
                }
            }
        }
        let mut clips = Vec::new();
        for a in d.animations() {
            let mut channels = Vec::new();
            let mut duration: f32 = 0.;
            for c in a.channels() {
                if matches!(
                    c.target().node().transform(),
                    gltf::scene::Transform::Matrix { .. }
                ) {
                    return Err(err("animated nodes must use TRS transforms"));
                }
                let r = c.reader(data);
                let times: Vec<_> = r
                    .read_inputs()
                    .ok_or_else(|| err("animation missing times"))?
                    .collect();
                let (values, width) = match r
                    .read_outputs()
                    .ok_or_else(|| err("animation missing values"))?
                {
                    ReadOutputs::Translations(i) | ReadOutputs::Scales(i) => {
                        (i.flatten().collect(), 3)
                    }
                    ReadOutputs::Rotations(i) => (i.into_f32().flatten().collect(), 4),
                    ReadOutputs::MorphTargetWeights(i) => {
                        let width = c
                            .target()
                            .node()
                            .mesh()
                            .and_then(|m| m.primitives().next())
                            .map_or(0, |p| p.morph_targets().count());
                        (i.into_f32().collect(), width)
                    }
                };
                let channel = Channel {
                    node: c.target().node().index(),
                    property: c.target().property(),
                    interpolation: c.sampler().interpolation(),
                    times,
                    values,
                    width,
                };
                channel.validate()?;
                duration = duration.max(*channel.times.last().unwrap());
                channels.push(channel);
            }
            clips.push(Clip {
                info: ClipInfo {
                    name: a.name().unwrap_or("Animation").into(),
                    duration,
                },
                channels,
            });
        }
        let images = images
            .into_iter()
            .map(convert_image)
            .collect::<Result<_, _>>()?;
        static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        let mut asset = Self {
            node_names: d
                .nodes()
                .map(|n| n.name().unwrap_or_default().to_owned())
                .collect(),
            id: NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            nodes,
            roots,
            meshes,
            skins,
            clips,
            materials,
            images,
            center: Vec3::ZERO,
            radius: 1.,
        };
        asset.validate()?;
        let draws = asset.pose(None, 0.);
        let mut min = Vec3::splat(f32::INFINITY);
        let mut max = Vec3::splat(f32::NEG_INFINITY);
        for v in draws.iter().flat_map(|d| &d.vertices) {
            let p = Vec3::from_array(v.position);
            min = min.min(p);
            max = max.max(p);
        }
        if !min.is_finite() || !max.is_finite() {
            return Err(err("selected scene contains no finite triangle geometry"));
        }
        asset.center = (min + max) * 0.5;
        asset.radius = (max - min).length().max(0.001) * 0.5;
        Ok(asset)
    }
    fn validate(&self) -> Result<(), Error> {
        // Graph validation also prevents recursive cycles in malformed documents.
        fn visit(i: usize, nodes: &[Node], state: &mut [u8]) -> Result<(), Error> {
            if state[i] == 1 {
                return Err(err("cyclic node graph"));
            }
            if state[i] == 2 {
                return Ok(());
            }
            state[i] = 1;
            for &c in &nodes[i].children {
                visit(c, nodes, state)?;
            }
            state[i] = 2;
            Ok(())
        }
        let mut state = vec![0; self.nodes.len()];
        let mut parents = vec![0; self.nodes.len()];
        for (i, n) in self.nodes.iter().enumerate() {
            if !n.matrix.is_finite() {
                return Err(err("non-finite node transform"));
            }
            for &c in &n.children {
                parents[c] += 1;
                if parents[c] > 1 {
                    return Err(err("node has multiple parents"));
                }
            }
            visit(i, &self.nodes, &mut state)?;
        }
        for m in &self.meshes {
            let n = m.positions.len();
            if m.uv.len() != n
                || m.colors.len() != n
                || (!m.normals.is_empty() && m.normals.len() != n)
                || m.morphs
                    .iter()
                    .any(|(p, norm)| p.len() != n || norm.len() != n)
                || m.positions.iter().chain(&m.normals).any(|p| !p.is_finite())
                || m.uv
                    .iter()
                    .flatten()
                    .chain(m.colors.iter().flatten())
                    .any(|v| !v.is_finite())
                || m.morphs
                    .iter()
                    .any(|(p, n)| p.iter().chain(n).any(|v| !v.is_finite()))
            {
                return Err(err("inconsistent or non-finite mesh attributes"));
            }
            if let Some(s) = m.skin {
                let skin = &self.skins[s];
                if skin.inverse.len() != skin.joints.len()
                    || m.joints.len() != n
                    || m.weights.len() != n
                    || skin.inverse.iter().any(|m| !m.is_finite())
                    || m.joints
                        .iter()
                        .flatten()
                        .any(|j| *j as usize >= skin.joints.len())
                    || m.weights.iter().any(|w| {
                        w.iter().any(|v| !v.is_finite() || *v < 0.) || w.iter().sum::<f32>() <= 0.
                    })
                {
                    return Err(err("invalid skin joints, inverse bind matrices or weights"));
                }
            }
        }
        Ok(())
    }
    pub(crate) fn sample_pose(&self, clip: Option<usize>, time: f32) -> PoseFrame {
        let mut nodes = self.nodes.clone();
        if let Some(clip) = clip {
            for c in &self.clips[clip].channels {
                let v = c.sample(time);
                let n = &mut nodes[c.node];
                match c.property {
                    gltf::animation::Property::Translation => n.t = Vec3::from_slice(&v),
                    gltf::animation::Property::Rotation => n.r = Quat::from_slice(&v),
                    gltf::animation::Property::Scale => n.s = Vec3::from_slice(&v),
                    gltf::animation::Property::MorphTargetWeights => n.weights = v,
                }
                if c.property != gltf::animation::Property::MorphTargetWeights {
                    n.matrix = Mat4::from_scale_rotation_translation(n.s, n.r, n.t);
                }
            }
        }
        let mut world = vec![Mat4::IDENTITY; nodes.len()];
        let mut active = vec![false; nodes.len()];
        fn walk(
            i: usize,
            parent: Mat4,
            nodes: &[Node],
            world: &mut [Mat4],
            active: &mut [bool],
            visible: bool,
        ) {
            world[i] = parent * nodes[i].matrix;
            active[i] |= visible;
            for &c in &nodes[i].children {
                walk(c, world[i], nodes, world, active, visible);
            }
        }
        // Compute all joint transforms, including joints outside the selected scene.
        let children: std::collections::HashSet<_> = nodes
            .iter()
            .flat_map(|n| n.children.iter().copied())
            .collect();
        for i in 0..nodes.len() {
            if !children.contains(&i) {
                walk(i, Mat4::IDENTITY, &nodes, &mut world, &mut active, false);
            }
        }
        for &i in &self.roots {
            walk(i, Mat4::IDENTITY, &nodes, &mut world, &mut active, true);
        }
        PoseFrame {
            world,
            active,
            weights: nodes.into_iter().map(|n| n.weights).collect(),
        }
    }
    // Reference geometry for load-time framing and differential GPU tests only.
    pub(crate) fn pose(&self, clip: Option<usize>, time: f32) -> Vec<Draw> {
        let pose = self.sample_pose(clip, time);
        let world = &pose.world;
        let active = &pose.active;
        self.meshes
            .iter()
            .filter(|m| active[m.node])
            .map(|m| {
                let transforms: Vec<_> = m
                    .skin
                    .map(|s| {
                        self.skins[s]
                            .joints
                            .iter()
                            .zip(&self.skins[s].inverse)
                            .map(|(j, b)| world[*j] * *b)
                            .collect()
                    })
                    .unwrap_or_default();
                let posed: Vec<_> = m
                    .positions
                    .iter()
                    .enumerate()
                    .map(|(i, p)| {
                        let mut p = *p;
                        let mut n = m.normals.get(i).copied().unwrap_or(Vec3::ZERO);
                        for (k, (dp, dn)) in m.morphs.iter().enumerate() {
                            let w = pose.weights[m.node].get(k).copied().unwrap_or(0.);
                            p += dp[i] * w;
                            n += dn[i] * w;
                        }
                        let matrix = if m.skin.is_some() {
                            let w = m.weights[i];
                            let sum: f32 = w.iter().sum();
                            (0..4).fold(Mat4::ZERO, |acc, k| {
                                acc + transforms[m.joints[i][k] as usize] * (w[k] / sum)
                            })
                        } else {
                            world[m.node]
                        };
                        let normal = if matrix.determinant().abs() > 1e-10 {
                            matrix
                                .inverse()
                                .transpose()
                                .transform_vector3(n)
                                .normalize_or_zero()
                        } else {
                            Vec3::ZERO
                        };
                        Vertex {
                            position: matrix.transform_point3(p).to_array(),
                            normal: normal.to_array(),
                            uv: m.uv[i],
                            color: m.colors[i],
                        }
                    })
                    .collect();
                let mut vertices: Vec<_> = m.indices.iter().map(|i| posed[*i as usize]).collect();
                if m.normals.is_empty() {
                    for tri in vertices.chunks_exact_mut(3) {
                        let a = Vec3::from_array(tri[0].position);
                        let b = Vec3::from_array(tri[1].position);
                        let c = Vec3::from_array(tri[2].position);
                        let normal = (b - a).cross(c - a).normalize_or_zero().to_array();
                        for v in tri {
                            v.normal = normal;
                        }
                    }
                }
                Draw { vertices }
            })
            .collect()
    }
}

fn convert_image(i: gltf::image::Data) -> Result<Image, Error> {
    use gltf::image::Format::*;
    let channels = match i.format {
        R8 => 1,
        R8G8 => 2,
        R8G8B8 => 3,
        R8G8B8A8 => 4,
        _ => return Err(err("preview supports 8-bit base color textures only")),
    };
    let mut rgba = Vec::with_capacity(i.width as usize * i.height as usize * 4);
    for p in i.pixels.chunks_exact(channels) {
        rgba.extend_from_slice(&match channels {
            1 => [p[0], p[0], p[0], 255],
            2 => [p[0], p[0], p[0], p[1]],
            3 => [p[0], p[1], p[2], 255],
            _ => [p[0], p[1], p[2], p[3]],
        });
    }
    Ok(Image {
        width: i.width,
        height: i.height,
        rgba,
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    pub(crate) fn depth_fixture(backing: bool) -> Asset3D {
        let mut asset =
            Asset3D::from_slice(include_bytes!("../fixtures/animated-tetra.glb")).unwrap();
        asset.nodes = vec![Node {
            matrix: Mat4::IDENTITY,
            t: Vec3::ZERO,
            r: Quat::IDENTITY,
            s: Vec3::ONE,
            weights: vec![],
            children: vec![],
        }];
        asset.roots = vec![0];
        asset.clips.clear();
        asset.skins.clear();
        asset.images.clear();
        asset.materials = vec![Material {
            color: [1.; 4],
            image: None,
            cutoff: 0.,
        }];
        let mut mesh = Mesh {
            node: 0,
            skin: None,
            positions: vec![],
            normals: vec![],
            joints: vec![],
            weights: vec![],
            uv: vec![],
            colors: vec![],
            morphs: vec![],
            indices: vec![],
            material: 0,
        };
        for (half, z, color) in [(1.2, 0., [0., 0., 0., 1.]), (1., 0.007, [1., 0., 0., 1.])] {
            if z == 0. && !backing {
                continue;
            }
            let first = mesh.positions.len() as u32;
            mesh.positions.extend([
                Vec3::new(-half, -half, z),
                Vec3::new(half, -half, z),
                Vec3::new(half, half, z),
                Vec3::new(-half, half, z),
            ]);
            mesh.normals.extend([Vec3::Z; 4]);
            mesh.uv.extend([[0.; 2]; 4]);
            mesh.colors.extend([color; 4]);
            mesh.indices
                .extend([first, first + 1, first + 2, first, first + 2, first + 3]);
        }
        asset.meshes = vec![mesh];
        asset.center = Vec3::ZERO;
        asset.radius = 2.;
        asset
    }
    // Mixed four-weight skin, joint hierarchy, inverse binds, reflected/nonuniform
    // transforms and two simultaneous morph targets. Used by real GPU comparisons.
    pub(crate) fn deformation_fixture(authored_normals: bool, singular: bool) -> Asset3D {
        let mut asset =
            Asset3D::from_slice(include_bytes!("../fixtures/skinned-tetra.glb")).unwrap();
        let scale = if singular {
            Vec3::ZERO
        } else {
            Vec3::new(1.25, -0.8, 0.6)
        };
        asset.nodes[1].s = scale;
        asset.nodes[1].matrix = Mat4::from_scale(scale);
        let child = asset.nodes.len();
        let t = Vec3::new(0.3, 0.2, -0.1);
        let r = Quat::from_rotation_x(0.4);
        let s = Vec3::new(0.5, 1.7, 1.1);
        asset.nodes.push(Node {
            matrix: Mat4::from_scale_rotation_translation(s, r, t),
            t,
            r,
            s,
            weights: Vec::new(),
            children: Vec::new(),
        });
        asset.nodes[1].children.push(child);
        asset.nodes[0].matrix = Mat4::from_translation(Vec3::new(5., 6., 7.));
        asset.skins[0].joints = vec![1, child];
        asset.skins[0].inverse = vec![
            Mat4::from_translation(Vec3::new(-0.1, 0.2, 0.)),
            Mat4::from_translation(Vec3::new(0.1, 0., 0.2)),
        ];
        let mesh = &mut asset.meshes[0];
        let n = mesh.positions.len();
        mesh.joints = vec![[0, 1, 0, 1]; n];
        mesh.weights = vec![[0.1, 0.2, 0.3, 0.4]; n];
        if authored_normals {
            mesh.normals = vec![Vec3::new(0.2, 1., 0.5).normalize(); n];
        }
        mesh.morphs[0].1 = vec![Vec3::new(0.1, -0.15, 0.05); n];
        mesh.morphs.push((
            vec![Vec3::new(0.2, 0.1, -0.15); n],
            vec![Vec3::new(-0.1, 0.2, 0.1); n],
        ));
        let mut morph_clip = asset.clips.remove(1);
        let channel = &mut morph_clip.channels[0];
        channel.width = 2;
        channel.values = vec![0., 0.25, 1., -0.5, 0., 0.25];
        channel.validate().unwrap();
        asset.clips[0].channels.extend(morph_clip.channels);
        asset.validate().unwrap();
        asset
    }
    pub(crate) fn seam_fixture() -> Asset3D {
        let mut asset = deformation_fixture(true, false);
        let mesh = &mut asset.meshes[0];
        let index = mesh.positions.len() as u32;
        mesh.positions.push(mesh.positions[0]);
        mesh.normals.push(-mesh.normals[0]);
        mesh.uv.push([1., 1.]);
        mesh.colors.push(mesh.colors[0]);
        mesh.joints.push(mesh.joints[0]);
        mesh.weights.push(mesh.weights[0]);
        for (p, n) in &mut mesh.morphs {
            p.push(p[0]);
            n.push(n[0]);
        }
        mesh.indices[0] = index;
        asset
    }
    pub(crate) fn repeat_geometry(mesh: &mut Mesh, copies: usize) {
        let count = mesh.positions.len() as u32;
        let indices = mesh.indices.clone();
        mesh.positions = mesh.positions.repeat(copies);
        mesh.normals = mesh.normals.repeat(copies);
        mesh.uv = mesh.uv.repeat(copies);
        mesh.colors = mesh.colors.repeat(copies);
        mesh.joints = mesh.joints.repeat(copies);
        mesh.weights = mesh.weights.repeat(copies);
        for (p, n) in &mut mesh.morphs {
            *p = p.repeat(copies);
            *n = n.repeat(copies);
        }
        mesh.indices = (0..copies as u32)
            .flat_map(|i| indices.iter().map(move |index| index + i * count))
            .collect();
    }
    #[test]
    fn node_skin_and_morph_produce_expected_world_geometry() {
        let plain = Asset3D::from_slice(include_bytes!("../fixtures/animated-tetra.glb")).unwrap();
        let skin = Asset3D::from_slice(include_bytes!("../fixtures/skinned-tetra.glb")).unwrap();
        let initial = plain.pose(Some(0), 0.);
        let turned = plain.pose(Some(0), 1.);
        let skinned = skin.pose(Some(0), 1.);
        for ((a, b), s) in initial[0]
            .vertices
            .iter()
            .zip(&turned[0].vertices)
            .zip(&skinned[0].vertices)
        {
            let expected =
                Quat::from_rotation_y(std::f32::consts::PI) * Vec3::from_array(a.position);
            assert!(expected.abs_diff_eq(Vec3::from_array(b.position), 1e-5));
            assert!(Vec3::from_array(s.position).abs_diff_eq(expected, 1e-5));
        }
        let morph = plain.pose(Some(1), 1.);
        assert!((morph[0].vertices[2].position[1] - 1.7).abs() < 1e-5);
    }
}
