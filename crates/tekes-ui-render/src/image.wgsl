// 实例化图像：把一张预乘 alpha 的纹理线性采样画进矩形。
// 不做颜色转换：纹理里的数值原样当作 sRGB 编码值，和其他图元在同一个空间里混合。

@group(1) @binding(0) var image: texture_2d<f32>;
@group(1) @binding(1) var image_sampler: sampler;

struct Instance {
    // x, y, width, height，单位 pt。
    @location(0) rect: vec4<f32>,
    // x: 不透明度，yzw 保留。
    @location(1) params: vec4<f32>,
    @location(2) uv: vec4<f32>,
}

struct VsOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) opacity: f32,
}

@vertex
fn vs_main(@builtin(vertex_index) vi: u32, inst: Instance) -> VsOut {
    let corner = vec2<f32>(f32(vi & 1u), f32(vi >> 1u));
    var position = inst.rect.xy + corner * inst.rect.zw;
    // Preserve the legacy zero-angle pixel path exactly.
    if inst.params.y != 0.0 {
        let offset = (corner - vec2<f32>(0.5)) * inst.rect.zw;
        let c = cos(inst.params.y);
        let s = sin(inst.params.y);
        position = inst.rect.xy + inst.rect.zw * 0.5 + vec2<f32>(c * offset.x - s * offset.y, s * offset.x + c * offset.y);
    }
    let pos_px = position * globals.scale;
    let ndc = vec2<f32>(
        pos_px.x / globals.viewport_px.x * 2.0 - 1.0,
        1.0 - pos_px.y / globals.viewport_px.y * 2.0,
    );
    var out: VsOut;
    out.position = vec4<f32>(ndc, 0.0, 1.0);
    out.uv = inst.uv.xy + corner * inst.uv.zw;
    out.opacity = inst.params.x;
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    return textureSample(image, image_sampler, in.uv) * in.opacity * clip_coverage(in.position.xy,0u);
}
