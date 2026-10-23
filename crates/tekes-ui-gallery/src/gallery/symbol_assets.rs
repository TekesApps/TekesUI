//! Approved SVG geometry + stable animation annotations, rasterized into three
//! reusable GPU images. No SVG reparsing or texture allocation in steady playback.
use resvg::{tiny_skia as sk, usvg};
use std::sync::{Arc, OnceLock};
use tekes_ui_icons::Icon;
use tekes_ui_motion::{
    recipes::RecipeParameters,
    symbols::{self, SymbolCapabilities, SymbolEffect, SymbolOptions, SymbolScope},
};
use tekes_ui_render::Renderer;
use tekes_ui_scene::{ImageId, ImageQuad, Rect, Scene, TransformedImageQuad};
use tekes_ui_tokens::{Theme, proposal};

pub const CANVAS: f32 = 112.0;
const ICON_SIZE: f32 = 64.0;
#[derive(Clone, Debug)]
struct Shape {
    path: sk::Path,
    fill: bool,
    stroke: Option<sk::Stroke>,
    contours: Vec<Vec<sk::Point>>,
    arc_length: f32,
}
#[derive(Clone, Debug)]
struct Geometry {
    layers: Vec<Vec<Shape>>,
    originals: Vec<usvg::Tree>,
}
#[derive(Clone, Debug, PartialEq)]
struct PreparedPose {
    icon: Icon,
    effect: SymbolEffect,
    progress: f64,
    params: RecipeParameters,
    reduced: bool,
    theme: Theme,
    size: u32,
    linear: bool,
}
#[derive(Clone, Copy, Debug, PartialEq)]
struct BaseKey {
    icon: Icon,
    theme: Theme,
    size: u32,
}
#[derive(Debug)]
struct BaseTextures {
    key: BaseKey,
    whole: ImageId,
    high: ImageId,
    layers: ImageId,
    count: usize,
}
#[derive(Clone, Copy, Debug, Default)]
pub struct UploadStats {
    pub rasterizations: u64,
    pub texture_creations: u64,
    pub uploads: u64,
    pub bytes: u64,
}
#[derive(Debug)]
pub struct SymbolAssets {
    geometry: Arc<Vec<Geometry>>,
    /// Dynamic raster fallback only; geometric effects use cached base textures.
    pub images: [Option<ImageId>; 3],
    prepared: Option<PreparedPose>,
    size: u32,
    bases: Vec<BaseTextures>,
    draws: [Vec<TransformedImageQuad>; 3],
    pub stats: UploadStats,
}
impl Default for SymbolAssets {
    fn default() -> Self {
        static GEOMETRY: OnceLock<Arc<Vec<Geometry>>> = OnceLock::new();
        Self {
            geometry: GEOMETRY
                .get_or_init(|| Arc::new(Icon::ALL.into_iter().map(load).collect()))
                .clone(),
            images: [None; 3],
            prepared: None,
            size: 0,
            bases: Vec::new(),
            draws: std::array::from_fn(|_| Vec::new()),
            stats: UploadStats::default(),
        }
    }
}
// GPU IDs have one owner; a cloned controller prepares its own device resources.
impl Clone for SymbolAssets {
    fn clone(&self) -> Self {
        Self::default()
    }
}
fn index(icon: Icon) -> usize {
    Icon::ALL.iter().position(|v| *v == icon).unwrap()
}
fn collect(group: &usvg::Group, paths: &mut Vec<Shape>) {
    for node in group.children() {
        match node {
            usvg::Node::Path(path) => {
                let path_data = path
                    .data()
                    .clone()
                    .transform(path.abs_transform())
                    .expect("approved finite path");
                let contours = flatten(&path_data);
                paths.push(Shape {
                    arc_length: contour_length(&contours),
                    contours,
                    path: path_data,
                    fill: path.fill().is_some(),
                    stroke: path.stroke().map(usvg::Stroke::to_tiny_skia),
                });
            }
            usvg::Node::Group(group) => collect(group, paths),
            _ => panic!("approved icons contain only vector paths"),
        }
    }
}
fn load(icon: Icon) -> Geometry {
    let tree = usvg::Tree::from_str(icon.svg(), &usvg::Options::default()).expect("approved SVG");
    let mut elements = Vec::new();
    collect(tree.root(), &mut elements);
    let layers = icon
        .animation_layers()
        .iter()
        .map(|layer| {
            layer
                .paths
                .iter()
                .map(|r| {
                    let mut shape = elements
                        .get(r.element)
                        .expect("annotation references existing SVG element")
                        .clone();
                    if let Some(contour) = r.contour {
                        let points = shape
                            .contours
                            .get(contour)
                            .expect("annotation references existing subpath")
                            .clone();
                        shape.path =
                            polyline(std::slice::from_ref(&points)).expect("nonempty subpath");
                        shape.contours = vec![points];
                        shape.arc_length = contour_length(&shape.contours);
                    }
                    shape
                })
                .collect()
        })
        .collect();
    let originals = Theme::ALL
        .into_iter()
        .map(|theme| {
            let ink = proposal::render_color("primary-text", theme).unwrap();
            let tint = format!(
                "#{:02x}{:02x}{:02x}",
                (ink.r * 255.0).round() as u8,
                (ink.g * 255.0).round() as u8,
                (ink.b * 255.0).round() as u8
            );
            usvg::Tree::from_str(
                &icon.svg().replace("currentColor", &tint),
                &usvg::Options::default(),
            )
            .unwrap()
        })
        .collect();
    Geometry { layers, originals }
}
fn flatten(path: &sk::Path) -> Vec<Vec<sk::Point>> {
    use sk::PathSegment::*;
    let mut result: Vec<Vec<sk::Point>> = Vec::new();
    let mut at = sk::Point::from_xy(0.0, 0.0);
    for segment in path.segments() {
        match segment {
            MoveTo(p) => {
                result.push(vec![p]);
                at = p;
            }
            LineTo(p) => {
                result.last_mut().unwrap().push(p);
                at = p;
            }
            QuadTo(c, p) => {
                let a = at;
                for step in 1..=32 {
                    let t = step as f32 / 32.0;
                    let s = 1.0 - t;
                    result.last_mut().unwrap().push(sk::Point::from_xy(
                        s * s * a.x + 2.0 * s * t * c.x + t * t * p.x,
                        s * s * a.y + 2.0 * s * t * c.y + t * t * p.y,
                    ));
                }
                at = p;
            }
            CubicTo(c, d, p) => {
                let a = at;
                for step in 1..=32 {
                    let t = step as f32 / 32.0;
                    let s = 1.0 - t;
                    result.last_mut().unwrap().push(sk::Point::from_xy(
                        s * s * s * a.x
                            + 3.0 * s * s * t * c.x
                            + 3.0 * s * t * t * d.x
                            + t * t * t * p.x,
                        s * s * s * a.y
                            + 3.0 * s * s * t * c.y
                            + 3.0 * s * t * t * d.y
                            + t * t * t * p.y,
                    ));
                }
                at = p;
            }
            Close => {
                let first = result.last().unwrap()[0];
                result.last_mut().unwrap().push(first);
                at = first;
            }
        }
    }
    result
}
fn polyline(contours: &[Vec<sk::Point>]) -> Option<sk::Path> {
    let mut b = sk::PathBuilder::new();
    for contour in contours {
        if let Some(first) = contour.first() {
            b.move_to(first.x, first.y);
            for p in &contour[1..] {
                b.line_to(p.x, p.y);
            }
        }
    }
    b.finish()
}
fn contour_length(contours: &[Vec<sk::Point>]) -> f32 {
    contours
        .iter()
        .flat_map(|c| c.windows(2))
        .map(|p| (p[1].x - p[0].x).hypot(p[1].y - p[0].y))
        .sum()
}
fn length(shape: &Shape) -> f32 {
    shape.arc_length
}

fn trim(shape: &Shape, start: f32, end: f32) -> Option<sk::Path> {
    let mut b = sk::PathBuilder::new();
    let mut offset = 0.0;
    for c in &shape.contours {
        let mut connected = false;
        for pair in c.windows(2) {
            let a = pair[0];
            let z = pair[1];
            let len = (z.x - a.x).hypot(z.y - a.y);
            if len <= 1e-8 {
                continue;
            }
            let lo = (start - offset).clamp(0.0, len);
            let hi = (end - offset).clamp(0.0, len);
            offset += len;
            if hi <= lo {
                connected = false;
                continue;
            }
            let p = |d: f32| {
                sk::Point::from_xy(a.x + (z.x - a.x) * d / len, a.y + (z.y - a.y) * d / len)
            };
            let a = p(lo);
            let z = p(hi);
            if !connected {
                b.move_to(a.x, a.y);
            }
            b.line_to(z.x, z.y);
            connected = true;
        }
    }
    b.finish()
}
fn morph(a: &sk::Path, b: &sk::Path, t: f32) -> Option<sk::Path> {
    use sk::PathSegment::*;
    if a.verbs() != b.verbs() {
        return None;
    }
    let mix = |a: sk::Point, b: sk::Point| {
        sk::Point::from_xy(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t)
    };
    let mut out = sk::PathBuilder::new();
    for (a, b) in a.segments().zip(b.segments()) {
        match (a, b) {
            (MoveTo(a), MoveTo(b)) => {
                let p = mix(a, b);
                out.move_to(p.x, p.y);
            }
            (LineTo(a), LineTo(b)) => {
                let p = mix(a, b);
                out.line_to(p.x, p.y);
            }
            (QuadTo(a, c), QuadTo(b, d)) => {
                let p = mix(a, b);
                let q = mix(c, d);
                out.quad_to(p.x, p.y, q.x, q.y);
            }
            (CubicTo(a, c, e), CubicTo(b, d, f)) => {
                let p = mix(a, b);
                let q = mix(c, d);
                let r = mix(e, f);
                out.cubic_to(p.x, p.y, q.x, q.y, r.x, r.y);
            }
            (Close, Close) => out.close(),
            _ => return None,
        }
    }
    out.finish()
}
impl SymbolAssets {
    pub fn resolved_effect(&self, icon: Icon, effect: SymbolEffect) -> SymbolEffect {
        if effect != SymbolEffect::MagicReplace {
            return effect;
        }
        let partner = icon.animation_partner();
        let a = &self.geometry[index(icon)];
        let b = &self.geometry[index(partner)];
        let correspondence = icon.animation_layers().iter().enumerate().any(|(i, l)| {
            l.shared_key.is_some()
                && partner.animation_layers().iter().enumerate().any(|(j, r)| {
                    l.shared_key == r.shared_key
                        && a.layers[i].len() == b.layers[j].len()
                        && a.layers[i].iter().zip(&b.layers[j]).all(|(a, b)| {
                            a.path.verbs() == b.path.verbs()
                                && a.path.points().len() == b.path.points().len()
                        })
                })
        });
        effect.resolve(SymbolCapabilities {
            ordered_paths: true,
            correspondence,
        })
    }
    #[allow(clippy::too_many_arguments)]
    pub fn raster(
        &self,
        icon: Icon,
        effect: SymbolEffect,
        progress: f64,
        params: RecipeParameters,
        scope: SymbolScope,
        reduced: bool,
        theme: Theme,
        scale: f32,
    ) -> sk::Pixmap {
        let size = (CANVAS * scale).round() as u32;
        let mut pixmap = sk::Pixmap::new(size, size).unwrap();
        let ink = proposal::render_color("primary-text", theme).unwrap();
        let mut paint = sk::Paint::default();
        paint.set_color_rgba8(
            (ink.r * 255.0).round() as u8,
            (ink.g * 255.0).round() as u8,
            (ink.b * 255.0).round() as u8,
            255,
        );
        let target = icon.animation_partner();
        let effect = self.resolved_effect(icon, effect);
        let options = SymbolOptions {
            scope,
            strength: params.symbol_strength,
            cycles: params.symbol_cycles,
            layer_delay: params.delay,
            ..Default::default()
        };
        // Split contours may overlap at junctions. At an identity endpoint use
        // the original compound SVG paint operation, preserving its exact AA
        // coverage instead of compositing the same junction multiple times.
        let replacing = matches!(effect, SymbolEffect::Replace | SymbolEffect::MagicReplace);
        let identity = icon.animation_layers().iter().enumerate().all(|(i, _)| {
            let f = symbols::sample(
                effect,
                progress,
                i,
                icon.animation_layers().len(),
                options,
                reduced,
            )
            .unwrap();
            (f.scale - 1.0).abs() < 1e-9
                && f.rotation.sin().abs() < 1e-9
                && (f.rotation.cos() - 1.0).abs() < 1e-9
                && f.offset_x.abs() < 1e-9
                && f.offset_y.abs() < 1e-9
                && (f.opacity - 1.0).abs() < 1e-9
                && f.draw_start == 0.0
                && f.draw_end == 1.0
        });
        let endpoint = if replacing && progress >= 1.0 {
            Some(target)
        } else if identity && (!replacing || progress <= 0.0) {
            Some(icon)
        } else {
            None
        };
        if let Some(endpoint) = endpoint {
            let tree = &self.geometry[index(endpoint)].originals
                [Theme::ALL.iter().position(|t| *t == theme).unwrap()];
            let zoom = ICON_SIZE / 24.0 * scale;
            let pad = (CANVAS - ICON_SIZE) * 0.5 * scale;
            resvg::render(
                tree,
                sk::Transform::from_row(zoom, 0.0, 0.0, zoom, pad, pad),
                &mut pixmap.as_mut(),
            );
            return pixmap;
        }
        let draw_layer = |pixmap: &mut sk::Pixmap,
                          icon: Icon,
                          i: usize,
                          shapes: &[Shape],
                          weight: f32,
                          frame: symbols::SymbolFrame| {
            if weight <= 0.0 {
                return;
            }
            let anchor = if scope == SymbolScope::Whole {
                [12.0, 12.0]
            } else {
                icon.animation_layers()[i].anchor
            };
            let zoom = ICON_SIZE / 24.0;
            let s = frame.scale as f32;
            let angle = frame.rotation as f32;
            let c = angle.cos() * s;
            let n = angle.sin() * s;
            let tx = (CANVAS - ICON_SIZE) * 0.5
                + zoom * (anchor[0] - c * anchor[0] + n * anchor[1])
                + frame.offset_x as f32;
            let ty = (CANVAS - ICON_SIZE) * 0.5
                + zoom * (anchor[1] - n * anchor[0] - c * anchor[1])
                + frame.offset_y as f32;
            let transform = sk::Transform::from_row(
                c * zoom * scale,
                n * zoom * scale,
                -n * zoom * scale,
                c * zoom * scale,
                tx * scale,
                ty * scale,
            );
            let total: f32 = shapes.iter().map(length).sum();
            let mut offset = 0.0;
            for shape in shapes {
                let len = length(shape);
                let lo = (frame.draw_start as f32 * total - offset).clamp(0.0, len);
                let hi = (frame.draw_end as f32 * total - offset).clamp(0.0, len);
                offset += len;
                let fraction = if len > 0.0 { (hi - lo) / len } else { 0.0 };
                if fraction <= 0.0 {
                    continue;
                }
                let mut paint = paint.clone();
                paint.set_color_rgba8(
                    (ink.r * 255.0).round() as u8,
                    (ink.g * 255.0).round() as u8,
                    (ink.b * 255.0).round() as u8,
                    (255.0 * weight * frame.opacity as f32).round() as u8,
                );
                if shape.fill {
                    let mut fill_paint = paint.clone();
                    fill_paint.set_color_rgba8(
                        (ink.r * 255.0).round() as u8,
                        (ink.g * 255.0).round() as u8,
                        (ink.b * 255.0).round() as u8,
                        (255.0 * weight * frame.opacity as f32 * fraction).round() as u8,
                    );
                    pixmap.fill_path(
                        &shape.path,
                        &fill_paint,
                        sk::FillRule::Winding,
                        transform,
                        None,
                    );
                }
                if let Some(stroke) = &shape.stroke {
                    if lo == 0.0 && hi >= len {
                        pixmap.stroke_path(&shape.path, &paint, stroke, transform, None);
                    } else if let Some(path) = trim(shape, lo, hi) {
                        pixmap.stroke_path(&path, &paint, stroke, transform, None);
                    }
                }
            }
        };
        let source = &self.geometry[index(icon)];
        let destination = &self.geometry[index(target)];
        let replacing = matches!(effect, SymbolEffect::Replace | SymbolEffect::MagicReplace);
        let mut shared = Vec::new();
        for (i, shapes) in source.layers.iter().enumerate() {
            let frame = symbols::sample(effect, progress, i, source.layers.len(), options, reduced)
                .unwrap();
            let key = icon.animation_layers()[i].shared_key;
            let matched = if effect == SymbolEffect::MagicReplace {
                key.and_then(|key| {
                    target
                        .animation_layers()
                        .iter()
                        .position(|l| l.shared_key == Some(key))
                })
            } else {
                None
            };
            if let Some(j) = matched {
                let paths: Option<Vec<Shape>> = shapes
                    .iter()
                    .zip(&destination.layers[j])
                    .map(|(a, b)| {
                        let path = morph(&a.path, &b.path, frame.replacement as f32)?;
                        Some(Shape {
                            contours: Vec::new(),
                            arc_length: a.arc_length
                                + (b.arc_length - a.arc_length) * frame.replacement as f32,
                            path,
                            fill: a.fill,
                            stroke: a.stroke.clone(),
                        })
                    })
                    .collect();
                if let Some(paths) = paths {
                    draw_layer(&mut pixmap, icon, i, &paths, 1.0, frame);
                    shared.push(j);
                    continue;
                }
            }
            draw_layer(
                &mut pixmap,
                icon,
                i,
                shapes,
                if replacing {
                    1.0 - frame.replacement as f32
                } else {
                    1.0
                },
                frame,
            );
        }
        if replacing {
            for (i, shapes) in destination.layers.iter().enumerate() {
                if shared.contains(&i) {
                    continue;
                }
                let frame = symbols::sample(
                    effect,
                    progress,
                    i,
                    destination.layers.len(),
                    options,
                    reduced,
                )
                .unwrap();
                draw_layer(
                    &mut pixmap,
                    target,
                    i,
                    shapes,
                    frame.replacement as f32,
                    frame,
                );
            }
        }
        pixmap
    }
    fn base_tile(&self, icon: Icon, layer: Option<usize>, theme: Theme, scale: f32) -> sk::Pixmap {
        let size = (CANVAS * scale).round() as u32;
        let mut out = sk::Pixmap::new(size, size).unwrap();
        let zoom = ICON_SIZE / 24.0 * scale;
        let pad = (CANVAS - ICON_SIZE) * 0.5 * scale;
        let transform = sk::Transform::from_row(zoom, 0.0, 0.0, zoom, pad, pad);
        let geometry = &self.geometry[index(icon)];
        if let Some(layer) = layer {
            let ink = proposal::render_color("primary-text", theme).unwrap();
            let mut paint = sk::Paint::default();
            paint.set_color_rgba8(
                (ink.r * 255.0).round() as u8,
                (ink.g * 255.0).round() as u8,
                (ink.b * 255.0).round() as u8,
                255,
            );
            for shape in &geometry.layers[layer] {
                if shape.fill {
                    out.fill_path(&shape.path, &paint, sk::FillRule::Winding, transform, None);
                }
                if let Some(stroke) = &shape.stroke {
                    out.stroke_path(&shape.path, &paint, stroke, transform, None);
                }
            }
        } else {
            let tree = &geometry.originals[Theme::ALL.iter().position(|t| *t == theme).unwrap()];
            resvg::render(tree, transform, &mut out.as_mut());
        }
        out
    }
    fn register(
        &mut self,
        renderer: &mut Renderer,
        pixmap: &sk::Pixmap,
    ) -> Result<ImageId, String> {
        let id = renderer
            .register_image(pixmap.data(), pixmap.width(), pixmap.height())
            .map_err(|e| e.to_string())?;
        self.stats.texture_creations += 1;
        self.stats.uploads += 1;
        self.stats.bytes += pixmap.data().len() as u64;
        Ok(id)
    }
    fn ensure_base(&mut self, renderer: &mut Renderer, key: BaseKey) -> Result<(), String> {
        if self.bases.iter().any(|b| b.key == key) {
            return Ok(());
        }
        let scale = key.size as f32 / CANVAS;
        let native = self.base_tile(key.icon, None, key.theme, scale);
        self.stats.rasterizations += 1;
        let high_size = key.size * 3 / 2;
        let high_scale = high_size as f32 / CANVAS;
        let high = self.base_tile(key.icon, None, key.theme, high_scale);
        self.stats.rasterizations += 1;
        let count = self.geometry[index(key.icon)].layers.len();
        let mut atlas = sk::Pixmap::new(high_size * count as u32, high_size).unwrap();
        for layer in 0..count {
            let tile = self.base_tile(key.icon, Some(layer), key.theme, high_scale);
            self.stats.rasterizations += 1;
            for row in 0..high_size as usize {
                let src = row * high_size as usize * 4;
                let dst = (row * atlas.width() as usize + layer * high_size as usize) * 4;
                atlas.data_mut()[dst..dst + high_size as usize * 4]
                    .copy_from_slice(&tile.data()[src..src + high_size as usize * 4]);
            }
        }
        let whole = self.register(renderer, &native)?;
        let high = match self.register(renderer, &high) {
            Ok(id) => id,
            Err(e) => {
                renderer.unregister_image(whole);
                return Err(e);
            }
        };
        let layers = match self.register(renderer, &atlas) {
            Ok(id) => id,
            Err(e) => {
                renderer.unregister_image(whole);
                renderer.unregister_image(high);
                return Err(e);
            }
        };
        self.bases.push(BaseTextures {
            key,
            whole,
            high,
            layers,
            count,
        });
        Ok(())
    }
    pub fn is_prepared(&self) -> bool {
        self.prepared.is_some()
    }
    pub fn append_scope(&self, scene: &mut Scene, scope: usize, origin: Rect) {
        for image in &self.draws[scope] {
            let mut image = *image;
            image.quad.rect = image.quad.rect.offset(origin.min_x(), origin.min_y());
            scene.push_transformed_image(image);
        }
    }
    fn pose(
        effect: SymbolEffect,
        progress: f64,
        layer: usize,
        layers: usize,
        options: SymbolOptions,
        reduced: bool,
        linear: bool,
    ) -> symbols::SymbolFrame {
        if linear {
            let phase = if options.scope == SymbolScope::Individually {
                options.layer_delay * layer as f64
            } else {
                0.0
            };
            symbols::SymbolFrame {
                rotation: std::f64::consts::TAU * (progress - phase).rem_euclid(1.0),
                ..Default::default()
            }
        } else {
            symbols::sample(effect, progress, layer, layers, options, reduced).unwrap()
        }
    }
    fn transformed(
        image: ImageId,
        uv: Rect,
        frame: symbols::SymbolFrame,
        anchor: [f32; 2],
        weight: f64,
    ) -> TransformedImageQuad {
        let scale = frame.scale as f32;
        let rotation =
            if frame.rotation.sin().abs() < 1e-9 && (frame.rotation.cos() - 1.0).abs() < 1e-9 {
                0.0
            } else {
                frame.rotation.rem_euclid(std::f64::consts::TAU) as f32
            };
        let (sin, cos) = rotation.sin_cos();
        let pad = (CANVAS - ICON_SIZE) * 0.5;
        let zoom = ICON_SIZE / 24.0;
        let px = pad + anchor[0] * zoom;
        let py = pad + anchor[1] * zoom;
        let dx = (CANVAS * 0.5 - px) * scale;
        let dy = (CANVAS * 0.5 - py) * scale;
        let cx = px + cos * dx - sin * dy + frame.offset_x as f32;
        let cy = py + sin * dx + cos * dy + frame.offset_y as f32;
        TransformedImageQuad {
            quad: ImageQuad {
                rect: Rect::new(
                    cx - CANVAS * scale * 0.5,
                    cy - CANVAS * scale * 0.5,
                    CANVAS * scale,
                    CANVAS * scale,
                ),
                image,
                opacity: (frame.opacity * weight).clamp(0.0, 1.0) as f32,
            },
            rotation,
            uv,
        }
    }
    #[allow(clippy::too_many_arguments)]
    fn gpu_draws(
        &self,
        icon: Icon,
        effect: SymbolEffect,
        progress: f64,
        params: RecipeParameters,
        scope: SymbolScope,
        reduced: bool,
        theme: Theme,
        size: u32,
        linear: bool,
    ) -> Vec<TransformedImageQuad> {
        let options = SymbolOptions {
            scope,
            strength: params.symbol_strength,
            cycles: params.symbol_cycles,
            layer_delay: params.delay,
            ..Default::default()
        };
        let replacing = effect == SymbolEffect::Replace;
        let mut out = Vec::with_capacity(8);
        for (which, icon) in [icon, icon.animation_partner()].into_iter().enumerate() {
            if which == 1 && !replacing {
                break;
            }
            let base = self
                .bases
                .iter()
                .find(|b| b.key == BaseKey { icon, theme, size })
                .unwrap();
            let frames: Vec<_> = (0..base.count)
                .map(|layer| {
                    Self::pose(
                        effect, progress, layer, base.count, options, reduced, linear,
                    )
                })
                .collect();
            let weights: Vec<_> = frames
                .iter()
                .map(|f| {
                    if replacing {
                        if which == 0 {
                            1.0 - f.replacement
                        } else {
                            f.replacement
                        }
                    } else {
                        1.0
                    }
                })
                .collect();
            let idle = frames.iter().all(|f| {
                (f.scale - 1.0).abs() < 1e-9
                    && f.rotation.sin().abs() < 1e-9
                    && (f.rotation.cos() - 1.0).abs() < 1e-9
                    && f.offset_x.abs() < 1e-9
                    && f.offset_y.abs() < 1e-9
                    && (f.opacity - 1.0).abs() < 1e-9
            });
            if (scope == SymbolScope::Whole && effect != SymbolEffect::VariableColor)
                || (idle && weights.iter().all(|w| (w - weights[0]).abs() < 1e-9))
            {
                let frame = frames[0];
                let image = if idle { base.whole } else { base.high };
                if frame.opacity * weights[0] > 0.0 {
                    out.push(Self::transformed(
                        image,
                        Rect::new(0.0, 0.0, 1.0, 1.0),
                        frame,
                        [12.0, 12.0],
                        weights[0],
                    ));
                }
            } else {
                for (layer, frame) in frames.into_iter().enumerate() {
                    if frame.opacity * weights[layer] <= 0.0 {
                        continue;
                    }
                    out.push(Self::transformed(
                        base.layers,
                        Rect::new(
                            layer as f32 / base.count as f32,
                            0.0,
                            1.0 / base.count as f32,
                            1.0,
                        ),
                        frame,
                        icon.animation_layers()[layer].anchor,
                        weights[layer],
                    ));
                }
            }
        }
        out
    }
    #[allow(clippy::too_many_arguments)]
    fn prepare_pose(
        &mut self,
        renderer: &mut Renderer,
        icon: Icon,
        effect: SymbolEffect,
        progress: f64,
        params: RecipeParameters,
        reduced: bool,
        theme: Theme,
        scale: f32,
        linear: bool,
    ) -> Result<(), String> {
        let scale = scale.clamp(0.5, 3.0);
        let size = (CANVAS * scale).round() as u32;
        let key = PreparedPose {
            icon,
            effect,
            progress,
            params,
            reduced,
            theme,
            size,
            linear,
        };
        if self.prepared.as_ref() == Some(&key) {
            return Ok(());
        }
        let resolved = self.resolved_effect(icon, effect);
        let gpu = !resolved.requires_paths() && resolved != SymbolEffect::MagicReplace;
        if gpu {
            for i in (0..self.bases.len()).rev() {
                let k = self.bases[i].key;
                if k.theme != theme
                    || k.size != size
                    || (k.icon != icon
                        && !(resolved == SymbolEffect::Replace
                            && k.icon == icon.animation_partner()))
                {
                    let b = self.bases.remove(i);
                    for id in [b.whole, b.high, b.layers] {
                        renderer.unregister_image(id);
                    }
                }
            }
            self.ensure_base(renderer, BaseKey { icon, theme, size })?;
            if resolved == SymbolEffect::Replace {
                self.ensure_base(
                    renderer,
                    BaseKey {
                        icon: icon.animation_partner(),
                        theme,
                        size,
                    },
                )?;
            }
            for (i, scope) in [
                SymbolScope::Whole,
                SymbolScope::ByLayer,
                SymbolScope::Individually,
            ]
            .into_iter()
            .enumerate()
            {
                self.draws[i] = self.gpu_draws(
                    icon, resolved, progress, params, scope, reduced, theme, size, linear,
                );
            }
        } else {
            if self.size != size {
                for id in self.images.iter_mut().filter_map(Option::take) {
                    renderer.unregister_image(id);
                }
                self.size = size;
            }
            for (i, scope) in [
                SymbolScope::Whole,
                SymbolScope::ByLayer,
                SymbolScope::Individually,
            ]
            .into_iter()
            .enumerate()
            {
                let pixmap =
                    self.raster(icon, effect, progress, params, scope, reduced, theme, scale);
                self.stats.rasterizations += 1;
                if let Some(id) = self.images[i] {
                    renderer
                        .update_image(id, pixmap.data())
                        .map_err(|e| e.to_string())?;
                    self.stats.uploads += 1;
                    self.stats.bytes += pixmap.data().len() as u64;
                } else {
                    self.images[i] = Some(self.register(renderer, &pixmap)?);
                }
                self.draws[i].clear();
                self.draws[i].push(TransformedImageQuad::new(
                    ImageQuad {
                        rect: Rect::new(0.0, 0.0, CANVAS, CANVAS),
                        image: self.images[i].unwrap(),
                        opacity: 1.0,
                    },
                    0.0,
                ));
            }
        }
        self.prepared = Some(key);
        Ok(())
    }
    #[allow(clippy::too_many_arguments)]
    pub fn prepare(
        &mut self,
        renderer: &mut Renderer,
        icon: Icon,
        effect: SymbolEffect,
        progress: f64,
        params: RecipeParameters,
        reduced: bool,
        theme: Theme,
        scale: f32,
    ) -> Result<(), String> {
        self.prepare_pose(
            renderer, icon, effect, progress, params, reduced, theme, scale, false,
        )
    }
    #[allow(clippy::too_many_arguments)]
    pub fn prepare_linear(
        &mut self,
        renderer: &mut Renderer,
        icon: Icon,
        turns: f64,
        params: RecipeParameters,
        reduced: bool,
        theme: Theme,
        scale: f32,
    ) -> Result<(), String> {
        self.prepare_pose(
            renderer,
            icon,
            SymbolEffect::Rotate,
            turns,
            params,
            reduced,
            theme,
            scale,
            true,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_effect_renders_on_every_approved_icon() {
        let assets = SymbolAssets::default();
        for icon in Icon::ALL {
            for effect in SymbolEffect::ALL {
                let pixmap = assets.raster(
                    icon,
                    effect,
                    0.37,
                    RecipeParameters::default(),
                    SymbolScope::ByLayer,
                    false,
                    Theme::default(),
                    1.0,
                );
                assert!(
                    pixmap.data().chunks_exact(4).any(|p| p[3] > 0),
                    "empty {icon:?}/{effect:?}"
                );
            }
        }
    }

    #[test]
    fn annotations_cover_every_original_svg_contour_once() {
        for icon in Icon::ALL {
            let tree = usvg::Tree::from_str(icon.svg(), &usvg::Options::default()).unwrap();
            let mut shapes = Vec::new();
            collect(tree.root(), &mut shapes);
            let mut covered = std::collections::HashSet::new();
            let mut ids = std::collections::HashSet::new();
            for layer in icon.animation_layers() {
                assert!(ids.insert(layer.id));
                assert!(layer.anchor.iter().all(|v| v.is_finite()));
                for path in layer.paths {
                    let count = shapes[path.element].contours.len();
                    let selected: Vec<usize> = path
                        .contour
                        .map_or_else(|| (0..count).collect(), |i| vec![i]);
                    for i in selected {
                        assert!(i < count);
                        assert!(covered.insert((path.element, i)), "duplicate {icon:?}");
                    }
                }
            }
            assert_eq!(
                covered.len(),
                shapes.iter().map(|s| s.contours.len()).sum::<usize>()
            );
        }
    }
    #[test]
    fn completed_drawing_matches_original_approved_svg_pixels() {
        let assets = SymbolAssets::default();
        for icon in Icon::ALL {
            for theme in Theme::ALL {
                for scale in [1.0, 2.0] {
                    let actual = assets.raster(
                        icon,
                        SymbolEffect::DrawOn,
                        1.0,
                        RecipeParameters::default(),
                        SymbolScope::Whole,
                        false,
                        theme,
                        scale,
                    );
                    let ink = proposal::render_color("primary-text", theme).unwrap();
                    let tint = format!(
                        "#{:02x}{:02x}{:02x}",
                        (ink.r * 255.0).round() as u8,
                        (ink.g * 255.0).round() as u8,
                        (ink.b * 255.0).round() as u8
                    );
                    let svg = icon.svg().replace("currentColor", &tint);
                    let tree = usvg::Tree::from_str(&svg, &usvg::Options::default()).unwrap();
                    let mut expected = sk::Pixmap::new(actual.width(), actual.height()).unwrap();
                    let zoom = ICON_SIZE / 24.0 * scale;
                    let pad = (CANVAS - ICON_SIZE) * 0.5 * scale;
                    resvg::render(
                        &tree,
                        sk::Transform::from_row(zoom, 0.0, 0.0, zoom, pad, pad),
                        &mut expected.as_mut(),
                    );
                    assert!(
                        actual.data() == expected.data(),
                        "rest geometry changed: {icon:?}/{theme:?}/{scale}"
                    );
                }
            }
        }
    }
    #[test]
    fn stroke_progress_filled_layers_and_magic_pairs_are_real_assets() {
        let assets = SymbolAssets::default();
        for icon in Icon::ALL {
            let zero = assets.raster(
                icon,
                SymbolEffect::DrawOn,
                0.0,
                RecipeParameters::default(),
                SymbolScope::Whole,
                false,
                Theme::default(),
                1.0,
            );
            let half = assets.raster(
                icon,
                SymbolEffect::DrawOn,
                0.5,
                RecipeParameters::default(),
                SymbolScope::Whole,
                false,
                Theme::default(),
                1.0,
            );
            let full = assets.raster(
                icon,
                SymbolEffect::DrawOn,
                1.0,
                RecipeParameters::default(),
                SymbolScope::Whole,
                false,
                Theme::default(),
                1.0,
            );
            assert!(zero.data().iter().all(|v| *v == 0));
            assert_ne!(half.data(), full.data());
            assert!(half.data().chunks_exact(4).any(|p| p[3] > 0));
            let expected = if matches!(icon, Icon::NewGoal | Icon::GoalCompleted) {
                SymbolEffect::MagicReplace
            } else {
                SymbolEffect::Replace
            };
            assert_eq!(
                assets.resolved_effect(icon, SymbolEffect::MagicReplace),
                expected
            );
            let replaced = assets.raster(
                icon,
                SymbolEffect::MagicReplace,
                1.0,
                RecipeParameters::default(),
                SymbolScope::Whole,
                false,
                Theme::default(),
                1.0,
            );
            let target = assets.raster(
                icon.animation_partner(),
                SymbolEffect::DrawOn,
                1.0,
                RecipeParameters::default(),
                SymbolScope::Whole,
                false,
                Theme::default(),
                1.0,
            );
            assert!(
                replaced.data() == target.data(),
                "wrong target endpoint {icon:?}"
            );
        }
    }
}

#[cfg(test)]
mod gpu_tests {
    use super::*;
    use tekes_ui_scene::GlyphAtlas;
    use tekes_ui_tokens::Rgba;
    fn renderer() -> Option<Renderer> {
        match Renderer::new_headless() {
            Ok(r) => Some(r),
            Err(tekes_ui_render::RenderError::NoAdapter) => {
                eprintln!("SKIP: no GPU adapter");
                None
            }
            Err(e) => panic!("GPU init {e}"),
        }
    }
    #[test]
    fn cached_geometry_has_no_playback_raster_or_upload_and_paths_keep_fallback() {
        let Some(mut r) = renderer() else {
            return;
        };
        let mut assets = SymbolAssets::default();
        let mut params = RecipeParameters::default();
        for effect in [
            SymbolEffect::Breathe,
            SymbolEffect::Bounce,
            SymbolEffect::Pulse,
            SymbolEffect::Rotate,
            SymbolEffect::Wiggle,
            SymbolEffect::Scale,
            SymbolEffect::Appear,
            SymbolEffect::Disappear,
            SymbolEffect::VariableColor,
            SymbolEffect::Replace,
        ] {
            assets
                .prepare(
                    &mut r,
                    Icon::NewGoal,
                    effect,
                    0.1,
                    params,
                    false,
                    Theme::default(),
                    2.0,
                )
                .unwrap();
            let baseline = assets.stats;
            let textures = r.image_count();
            for step in 2..=15 {
                params.symbol_strength = step as f64 / 20.0;
                params.delay = step as f64 / 30.0;
                assets
                    .prepare(
                        &mut r,
                        Icon::NewGoal,
                        effect,
                        step as f64 / 16.0,
                        params,
                        false,
                        Theme::default(),
                        2.0,
                    )
                    .unwrap();
            }
            assert_eq!(
                assets.stats.rasterizations, baseline.rasterizations,
                "{effect:?}"
            );
            assert_eq!(assets.stats.uploads, baseline.uploads, "{effect:?}");
            assert_eq!(assets.stats.texture_creations, baseline.texture_creations);
            assert_eq!(r.image_count(), textures);
        }
        let before = assets.stats;
        assets
            .prepare(
                &mut r,
                Icon::Router,
                SymbolEffect::DrawOn,
                0.3,
                params,
                false,
                Theme::default(),
                2.0,
            )
            .unwrap();
        assets
            .prepare(
                &mut r,
                Icon::Router,
                SymbolEffect::DrawOn,
                0.4,
                params,
                false,
                Theme::default(),
                2.0,
            )
            .unwrap();
        assert_eq!(assets.stats.rasterizations - before.rasterizations, 6);
        assert_eq!(assets.stats.uploads - before.uploads, 6);
    }
    #[test]
    fn gpu_endpoints_match_original_svg_texture_in_four_themes_and_two_scales() {
        let Some(mut r) = renderer() else {
            return;
        };
        let mut assets = SymbolAssets::default();
        let mut atlas = GlyphAtlas::default();
        for icon in Icon::ALL {
            for theme in Theme::ALL {
                for scale in [1.0, 2.0] {
                    assets
                        .prepare(
                            &mut r,
                            icon,
                            SymbolEffect::Rotate,
                            1.0,
                            RecipeParameters::default(),
                            false,
                            theme,
                            scale,
                        )
                        .unwrap();
                    let mut actual = Scene::new();
                    assets.append_scope(&mut actual, 0, Rect::new(0.0, 0.0, CANVAS, CANVAS));
                    let pixels = assets.base_tile(icon, None, theme, scale);
                    let id = r
                        .register_image(pixels.data(), pixels.width(), pixels.height())
                        .unwrap();
                    let mut expected = Scene::new();
                    expected.push_image(ImageQuad {
                        rect: Rect::new(0.0, 0.0, CANVAS, CANVAS),
                        image: id,
                        opacity: 1.0,
                    });
                    let size = (pixels.width(), pixels.height());
                    let a = r
                        .render_to_rgba8(&actual, &mut atlas, size, scale, Rgba::gray(0.25))
                        .unwrap();
                    let b = r
                        .render_to_rgba8(&expected, &mut atlas, size, scale, Rgba::gray(0.25))
                        .unwrap();
                    assert!(a == b, "GPU endpoint changed {icon:?}/{theme:?}/{scale}");
                    r.unregister_image(id);
                }
            }
        }
    }
}

#[cfg(test)]
mod transformed_pose_tests {
    use super::*;
    use tekes_ui_scene::GlyphAtlas;
    use tekes_ui_tokens::Rgba;
    fn moments(bytes: &[u8], width: u32) -> (f64, f64, f64) {
        let (mut mass, mut x, mut y) = (0.0, 0.0, 0.0);
        for (i, p) in bytes.chunks_exact(4).enumerate() {
            let a = f64::from(p[3]);
            mass += a;
            x += (i % width as usize) as f64 * a;
            y += (i / width as usize) as f64 * a;
        }
        (mass, x / mass.max(1.0), y / mass.max(1.0))
    }
    #[test]
    fn transformed_layers_match_vector_coverage_and_centers() {
        let mut r = match Renderer::new_headless() {
            Ok(r) => r,
            Err(tekes_ui_render::RenderError::NoAdapter) => {
                eprintln!("SKIP no GPU");
                return;
            }
            Err(e) => panic!("{e}"),
        };
        let mut assets = SymbolAssets::default();
        let mut atlas = GlyphAtlas::default();
        let scale = 2.0;
        let size = (CANVAS * scale) as u32;
        for icon in Icon::ALL {
            for effect in [
                SymbolEffect::Breathe,
                SymbolEffect::Wiggle,
                SymbolEffect::Rotate,
                SymbolEffect::VariableColor,
                SymbolEffect::Replace,
            ] {
                for (i, scope) in [
                    SymbolScope::Whole,
                    SymbolScope::ByLayer,
                    SymbolScope::Individually,
                ]
                .into_iter()
                .enumerate()
                {
                    let params = RecipeParameters::default();
                    let theme = Theme::default();
                    assets
                        .prepare(&mut r, icon, effect, 0.37, params, false, theme, scale)
                        .unwrap();
                    let mut scene = Scene::new();
                    assets.append_scope(&mut scene, i, Rect::new(0.0, 0.0, CANVAS, CANVAS));
                    let gpu = r
                        .render_to_rgba8(
                            &scene,
                            &mut atlas,
                            (size, size),
                            scale,
                            Rgba::new(0.0, 0.0, 0.0, 0.0),
                        )
                        .unwrap();
                    let vector =
                        assets.raster(icon, effect, 0.37, params, scope, false, theme, scale);
                    let a = moments(&gpu, size);
                    let b = moments(vector.data(), size);
                    assert!(
                        (a.0 - b.0).abs() / b.0.max(1.0) < 0.035,
                        "coverage {icon:?}/{effect:?}/{scope:?}: {a:?} vs {b:?}"
                    );
                    assert!(
                        (a.1 - b.1).abs() < 0.6 && (a.2 - b.2).abs() < 0.6,
                        "center {icon:?}/{effect:?}/{scope:?}: {a:?} vs {b:?}"
                    );
                }
            }
        }
    }
}
