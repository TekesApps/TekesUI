//! 一帧的编码：把 scene 切成批，上传实例和 atlas，在一个 render pass 里按顺序画。
//!
//! # 批
//!
//! scene 是一个有序列表。这里线性扫描，把**连续的同类图元**并成一批：
//! 一批对应一次实例化 draw。不同种类的图元交错出现时，批按原顺序排列，
//! 所以 painter's order 跨种类成立（文字压在 quad 上、另一个 quad 又压在文字上）。
//! 字形批里允许多个 `GlyphRun`（颜色在实例里）；图像批要求是同一张图。

use crate::common::GlobalsBinding;
use crate::error::RenderError;
use crate::glyph_pass::{GlyphInstance, GlyphPass};
use crate::image_pass::{ImageInstance, ImagePass};
use crate::instance::QuadInstance;
use crate::quad_pass::QuadPass;
use std::ops::Range;
use tekes_ui_scene::{GlyphAtlas, ImageId, Primitive, Scene};
use tekes_ui_tokens::Rgba;

#[derive(Clone, Debug, PartialEq, Eq)]
enum Batch {
    Quads(Range<u32>),
    Glyphs(Range<u32>),
    Image(ImageId, Range<u32>),
}

pub(crate) struct Pipelines {
    globals: GlobalsBinding,
    pub quad: QuadPass,
    pub glyph: GlyphPass,
    pub image: ImagePass,
    batches: Vec<Batch>,
}

/// 每帧要画的各类实例数，测试用。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct FrameCounts {
    pub quads: usize,
    pub glyphs: usize,
    pub images: usize,
    pub draws: usize,
}

impl Pipelines {
    pub fn new(device: &wgpu::Device, consumer: u64) -> Pipelines {
        let globals = GlobalsBinding::new(device);
        let quad = QuadPass::new(device, &globals.layout);
        let glyph = GlyphPass::new(device, &globals.layout, consumer);
        let image = ImagePass::new(device, &globals.layout);
        Pipelines {
            globals,
            quad,
            glyph,
            image,
            batches: Vec::new(),
        }
    }

    pub fn register_image(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        rgba8_premul: &[u8],
        width: u32,
        height: u32,
    ) -> Result<ImageId, RenderError> {
        self.image
            .register(device, queue, rgba8_premul, width, height)
    }

    /// 把 scene 编码进 `encoder`：先同步 atlas，再一个 render pass，清屏后按批顺序画。
    #[allow(clippy::too_many_arguments)]
    pub fn encode(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        size_px: (u32, u32),
        scale: f32,
        scene: &Scene,
        atlas: &mut GlyphAtlas,
        clear: Rgba,
    ) -> FrameCounts {
        // 先让 GPU 纹理跟上 CPU 页。write_texture 排在本次提交所有命令缓冲之前。
        self.glyph.sync(device, queue, atlas);
        let atlas_layers = self.glyph.atlas.as_ref().map_or(0, |t| t.layers());

        self.quad.staging.clear();
        self.glyph.staging.clear();
        self.image.staging.clear();
        self.batches.clear();
        for p in scene.primitives() {
            match p {
                Primitive::Quad(q) => {
                    let start = self.quad.staging.len() as u32;
                    self.quad.staging.push(QuadInstance::from_quad(q));
                    push_batch(&mut self.batches, Kind::Quads, start, start + 1);
                }
                Primitive::Glyphs(run) => {
                    let start = self.glyph.staging.len() as u32;
                    for g in &run.glyphs {
                        // 页号超出当前纹理层数：这个字形不是这个 atlas 编出来的，跳过。
                        if g.glyph.page >= atlas_layers {
                            continue;
                        }
                        if let Some(inst) = GlyphInstance::new(g.origin, &g.glyph, run.color, scale)
                        {
                            self.glyph.staging.push(inst);
                        }
                    }
                    let end = self.glyph.staging.len() as u32;
                    if end > start {
                        push_batch(&mut self.batches, Kind::Glyphs, start, end);
                    }
                }
                Primitive::Image(q) => {
                    // 没登记（或已注销）的图像不画。
                    if !self.image.contains(q.image) {
                        continue;
                    }
                    let start = self.image.staging.len() as u32;
                    self.image.staging.push(ImageInstance::from_quad(q));
                    push_batch(&mut self.batches, Kind::Image(q.image), start, start + 1);
                }
                // Primitive 是 non_exhaustive，新增的图元种类在这里要显式处理。
                _ => {}
            }
        }

        self.quad.instances.write(device, queue, &self.quad.staging);
        self.glyph
            .instances
            .write(device, queue, &self.glyph.staging);
        self.image
            .instances
            .write(device, queue, &self.image.staging);
        self.globals.write(queue, size_px, scale);

        // clear 是非预乘 sRGB，目标里存的是预乘值，所以清屏色也要预乘。
        let a = clear.a as f64;
        let clear_color = wgpu::Color {
            r: clear.r as f64 * a,
            g: clear.g as f64 * a,
            b: clear.b as f64 * a,
            a,
        };
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("tekes frame pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(clear_color),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_bind_group(0, &self.globals.bind_group, &[]);
        for b in &self.batches {
            match b {
                Batch::Quads(r) => {
                    pass.set_pipeline(&self.quad.pipeline);
                    pass.set_vertex_buffer(0, self.quad.instances.buffer().slice(..));
                    pass.draw(0..4, r.clone());
                }
                Batch::Glyphs(r) => {
                    let Some(tex) = &self.glyph.atlas else {
                        continue;
                    };
                    pass.set_pipeline(&self.glyph.pipeline);
                    pass.set_bind_group(1, &tex.bind_group, &[]);
                    pass.set_vertex_buffer(0, self.glyph.instances.buffer().slice(..));
                    pass.draw(0..4, r.clone());
                }
                Batch::Image(id, r) => {
                    let Some(bg) = self.image.bind_group(*id) else {
                        continue;
                    };
                    pass.set_pipeline(&self.image.pipeline);
                    pass.set_bind_group(1, bg, &[]);
                    pass.set_vertex_buffer(0, self.image.instances.buffer().slice(..));
                    pass.draw(0..4, r.clone());
                }
            }
        }
        FrameCounts {
            quads: self.quad.staging.len(),
            glyphs: self.glyph.staging.len(),
            images: self.image.staging.len(),
            draws: self.batches.len(),
        }
    }
}

enum Kind {
    Quads,
    Glyphs,
    Image(ImageId),
}

/// 把 `start..end` 并入最后一批（同类且相邻），否则新开一批。
fn push_batch(batches: &mut Vec<Batch>, kind: Kind, start: u32, end: u32) {
    match (batches.last_mut(), &kind) {
        (Some(Batch::Quads(r)), Kind::Quads) if r.end == start => r.end = end,
        (Some(Batch::Glyphs(r)), Kind::Glyphs) if r.end == start => r.end = end,
        (Some(Batch::Image(id, r)), Kind::Image(new)) if id == new && r.end == start => r.end = end,
        _ => batches.push(match kind {
            Kind::Quads => Batch::Quads(start..end),
            Kind::Glyphs => Batch::Glyphs(start..end),
            Kind::Image(id) => Batch::Image(id, start..end),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn consecutive_same_kind_merge_and_order_is_kept() {
        let mut b = Vec::new();
        push_batch(&mut b, Kind::Quads, 0, 1);
        push_batch(&mut b, Kind::Quads, 1, 2);
        push_batch(&mut b, Kind::Glyphs, 0, 5);
        push_batch(&mut b, Kind::Glyphs, 5, 7);
        push_batch(&mut b, Kind::Quads, 2, 3);
        push_batch(&mut b, Kind::Image(ImageId(1)), 0, 1);
        push_batch(&mut b, Kind::Image(ImageId(1)), 1, 2);
        push_batch(&mut b, Kind::Image(ImageId(2)), 2, 3);
        assert_eq!(
            b,
            vec![
                Batch::Quads(0..2),
                Batch::Glyphs(0..7),
                Batch::Quads(2..3),
                Batch::Image(ImageId(1), 0..2),
                Batch::Image(ImageId(2), 2..3),
            ]
        );
    }
}
