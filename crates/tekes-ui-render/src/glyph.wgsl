// 实例化字形：每个实例是 atlas 里的一块位图，按设备像素 1:1 贴到目标上。
//
// 位图不缩放，也没有亚像素偏移：quad 的原点和尺寸都是整数设备像素，
// 所以每个片元恰好对应位图里的一个像素，直接用 textureLoad 取整数坐标
// （等价于 nearest 采样，且没有任何过滤和 uv 取整的歧义）。
//
// 颜色全程在 sRGB 编码空间里处理，不做伽马修正，也不做对比度增强，
// 覆盖率线性混合；混合由管线的 premultiplied alpha 混合状态完成。
// 见 lib.rs 的"颜色管线"。

@group(1) @binding(0) var atlas: texture_2d_array<f32>;

struct Instance {
    // 目标位置和大小，设备像素，都是整数值：x, y, width, height。
    @location(0) dest: vec4<f32>,
    // atlas 里的位置：x, y（像素）, 页号, 模式（0 = 单色 mask，1 = 彩色）。
    @location(1) src: vec4<u32>,
    // 非预乘 sRGB RGBA。
    @location(2) color: vec4<f32>,
}

struct VsOut {
    @builtin(position) position: vec4<f32>,
    @location(0) @interpolate(flat) dest_origin: vec2<f32>,
    @location(1) @interpolate(flat) src: vec4<u32>,
    @location(2) @interpolate(flat) color: vec4<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vi: u32, inst: Instance) -> VsOut {
    // triangle strip 的 4 个顶点：(0,0) (1,0) (0,1) (1,1)。
    let corner = vec2<f32>(f32(vi & 1u), f32(vi >> 1u));
    let pos_px = inst.dest.xy + corner * inst.dest.zw;
    let ndc = vec2<f32>(
        pos_px.x / globals.viewport_px.x * 2.0 - 1.0,
        1.0 - pos_px.y / globals.viewport_px.y * 2.0,
    );
    var out: VsOut;
    out.position = vec4<f32>(ndc, 0.0, 1.0);
    out.dest_origin = inst.dest.xy;
    out.src = inst.src;
    out.color = inst.color;
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    // position.xy 是像素中心（k + 0.5），减去整数原点后取 floor 得到位图内的整数坐标。
    let local = vec2<i32>(floor(in.position.xy - in.dest_origin));
    let texel = vec2<i32>(in.src.xy) + local;
    let s = textureLoad(atlas, texel, i32(in.src.z), 0);
    if in.src.w == 0u {
        // 单色字形：覆盖率在 alpha，颜色来自 run。
        let a = in.color.a * s.a;
        return vec4<f32>(in.color.rgb * a, a) * clip_coverage(in.position.xy,0u);
    }
    // 彩色字形：采样值已预乘，color.a 只当不透明度。
    return s * in.color.a * clip_coverage(in.position.xy,0u);
}
