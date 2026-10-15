// 实例化图像：把一张预乘 alpha 的纹理线性采样画进矩形。
// 不做颜色转换：纹理里的数值原样当作 sRGB 编码值，和其他图元在同一个空间里混合。

struct Globals {
    viewport_px: vec2<f32>,
    scale: f32,
    _pad: f32,
}

@group(0) @binding(0) var<uniform> globals: Globals;
@group(1) @binding(0) var image: texture_2d<f32>;
@group(1) @binding(1) var image_sampler: sampler;

struct Instance {
    // x, y, width, height，单位 pt。
    @location(0) rect: vec4<f32>,
    // x: 不透明度，yzw 保留。
    @location(1) params: vec4<f32>,
}

struct VsOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) opacity: f32,
}

@vertex
fn vs_main(@builtin(vertex_index) vi: u32, inst: Instance) -> VsOut {
    let corner = vec2<f32>(f32(vi & 1u), f32(vi >> 1u));
    let pos_px = (inst.rect.xy + corner * inst.rect.zw) * globals.scale;
    let ndc = vec2<f32>(
        pos_px.x / globals.viewport_px.x * 2.0 - 1.0,
        1.0 - pos_px.y / globals.viewport_px.y * 2.0,
    );
    var out: VsOut;
    out.position = vec4<f32>(ndc, 0.0, 1.0);
    out.uv = corner;
    out.opacity = inst.params.x;
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    return textureSample(image, image_sampler, in.uv) * in.opacity;
}
