// 实例化圆角矩形。
//
// 颜色全程在 sRGB 编码空间里处理：输入是 token 的 sRGB 值，输出写进非 sRGB
// 格式的纹理，不做任何 sRGB <-> 线性转换。混合由管线的 premultiplied alpha
// 混合状态完成。见 lib.rs 的"颜色管线"。

struct Globals {
    // 渲染目标的像素尺寸。
    viewport_px: vec2<f32>,
    // 逻辑点到设备像素的比例。
    scale: f32,
    _pad: f32,
}

@group(0) @binding(0) var<uniform> globals: Globals;

struct Instance {
    // x, y, width, height，单位 pt。
    @location(0) rect: vec4<f32>,
    // 非预乘 sRGB RGBA。
    @location(1) fill: vec4<f32>,
    @location(2) border_color: vec4<f32>,
    // 圆角半径，顺序 左上、右上、右下、左下，单位 pt。
    @location(3) radii: vec4<f32>,
    // x: 描边宽度（pt），yzw 保留。
    @location(4) params: vec4<f32>,
}

struct VsOut {
    @builtin(position) position: vec4<f32>,
    @location(0) @interpolate(flat) center_px: vec2<f32>,
    @location(1) @interpolate(flat) half_px: vec2<f32>,
    @location(2) @interpolate(flat) radii_px: vec4<f32>,
    @location(3) @interpolate(flat) border_px: f32,
    @location(4) @interpolate(flat) fill: vec4<f32>,
    @location(5) @interpolate(flat) border_color: vec4<f32>,
}

// 包围盒向外多画 1 个设备像素，给抗锯齿的外侧半个像素留位置。
const AA_PAD_PX: f32 = 1.0;

@vertex
fn vs_main(@builtin(vertex_index) vi: u32, inst: Instance) -> VsOut {
    // triangle strip 的 4 个顶点：(0,0) (1,0) (0,1) (1,1)。
    let corner = vec2<f32>(f32(vi & 1u), f32(vi >> 1u));
    let s = globals.scale;
    let half_px = inst.rect.zw * s * 0.5;
    let center_px = inst.rect.xy * s + half_px;
    let pos_px = center_px + (corner * 2.0 - 1.0) * (half_px + AA_PAD_PX);
    // 像素坐标（原点左上、y 向下）到 NDC。
    let ndc = vec2<f32>(
        pos_px.x / globals.viewport_px.x * 2.0 - 1.0,
        1.0 - pos_px.y / globals.viewport_px.y * 2.0,
    );
    var out: VsOut;
    out.position = vec4<f32>(ndc, 0.0, 1.0);
    out.center_px = center_px;
    out.half_px = half_px;
    out.radii_px = inst.radii * s;
    out.border_px = inst.params.x * s;
    out.fill = inst.fill;
    out.border_color = inst.border_color;
    return out;
}

// 圆角矩形的有符号距离：p 相对矩形中心，b 是半宽高，r 是当前象限的圆角半径。
// 外部为正，内部为负，单位与输入相同（这里是设备像素）。
// b 允许为负（描边吃满整个矩形时内矩形退化），此时处处为正。
fn sd_round_rect(p: vec2<f32>, b: vec2<f32>, r: f32) -> f32 {
    let q = abs(p) - b + vec2<f32>(r);
    return min(max(q.x, q.y), 0.0) + length(max(q, vec2<f32>(0.0))) - r;
}

fn premultiply(c: vec4<f32>) -> vec4<f32> {
    return vec4<f32>(c.rgb * c.a, c.a);
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    // position.xy 是像素中心坐标（x + 0.5, y + 0.5）。
    let p = in.position.xy - in.center_px;
    // y 向下：p.y > 0 是下半部分。
    let bottom = p.y > 0.0;
    let right = p.x > 0.0;
    // radii: 左上 x、右上 y、右下 z、左下 w。
    let r = select(
        select(in.radii_px.x, in.radii_px.w, bottom),
        select(in.radii_px.y, in.radii_px.z, bottom),
        right,
    );

    // 覆盖率：距离边界 0.5 像素以内线性过渡，总宽约 1 个设备像素。
    let d_outer = sd_round_rect(p, in.half_px, r);
    let outer_cov = clamp(0.5 - d_outer, 0.0, 1.0);

    // 描边带：外形内缩 border_px，圆角半径也相应缩小。
    var border_cov = 0.0;
    if in.border_px > 0.0 {
        let d_inner = sd_round_rect(p, in.half_px - vec2<f32>(in.border_px), max(r - in.border_px, 0.0));
        border_cov = 1.0 - clamp(0.5 - d_inner, 0.0, 1.0);
    }

    // 先在形状内部把 fill 和 border 叠好，最后整体乘外轮廓覆盖率，
    // 这样外缘只被抗锯齿一次，不会出现 fill 与 border 各自抗锯齿叠加造成的发暗边。
    let fill = premultiply(in.fill);
    let border = premultiply(in.border_color) * border_cov;
    let stack = border + fill * (1.0 - border.a);
    return stack * outer_cov;
}
