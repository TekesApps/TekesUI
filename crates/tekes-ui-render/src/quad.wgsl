// 实例化圆角矩形。
//
// 颜色全程在 sRGB 编码空间里处理：输入是 token 的 sRGB 值，输出写进非 sRGB
// 格式的纹理，不做任何 sRGB <-> 线性转换。混合由管线的 premultiplied alpha
// 混合状态完成。见 lib.rs 的"颜色管线"。


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
    // 可选阴影颜色及 offset x/y、CSS blur radius、outset，单位 pt。
    @location(5) shadow_color: vec4<f32>,
    @location(6) shadow_params: vec4<f32>,
    @location(7) clip_meta: vec4<u32>,
}

struct VsOut {
    @builtin(position) position: vec4<f32>,
    @location(0) @interpolate(flat) center_px: vec2<f32>,
    @location(1) @interpolate(flat) half_px: vec2<f32>,
    @location(2) @interpolate(flat) radii_px: vec4<f32>,
    @location(3) @interpolate(flat) border_px: f32,
    @location(4) @interpolate(flat) fill: vec4<f32>,
    @location(5) @interpolate(flat) border_color: vec4<f32>,
    @location(6) @interpolate(flat) shadow_center_px: vec2<f32>,
    @location(7) @interpolate(flat) shadow_half_px: vec2<f32>,
    @location(8) @interpolate(flat) shadow_radii_px: vec4<f32>,
    @location(9) @interpolate(flat) shadow_sigma_px: f32,
    @location(10) @interpolate(flat) shadow_color: vec4<f32>,
    @location(11) @interpolate(flat) clip_meta: vec3<u32>,
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
    let shadow_sigma_px = inst.shadow_params.z * 0.5 * s;
    let shadow_offset_px = inst.shadow_params.xy * s;
    let shadow_outset_px = inst.shadow_params.w * s;
    var shadow_margin = vec2<f32>(0.0);
    if inst.shadow_color.a > 0.0 {
        shadow_margin = abs(shadow_offset_px) + max(shadow_outset_px, 0.0)
            + vec2<f32>(5.0 * shadow_sigma_px);
    }
    let pos_px = center_px + (corner * 2.0 - 1.0) * (half_px + shadow_margin + AA_PAD_PX);
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
    out.shadow_center_px = center_px + shadow_offset_px;
    out.shadow_half_px = max(half_px + vec2<f32>(shadow_outset_px), vec2<f32>(0.0));
    out.shadow_radii_px = max(inst.radii * s + vec4<f32>(shadow_outset_px), vec4<f32>(0.0));
    out.shadow_sigma_px = shadow_sigma_px;
    out.shadow_color = inst.shadow_color;
    out.clip_meta=inst.clip_meta.xyz;
    return out;
}

// 圆角矩形的有符号距离：p 相对矩形中心，b 是半宽高，r 是当前象限的圆角半径。
// 外部为正，内部为负，单位与输入相同（这里是设备像素）。
// b 允许为负（描边吃满整个矩形时内矩形退化），此时处处为正。


fn premultiply(c: vec4<f32>) -> vec4<f32> {
    return vec4<f32>(c.rgb * c.a, c.a);
}

// Abramowitz-Stegun erf approximation, max absolute error < 1.5e-7.
fn erf_approx(x: f32) -> f32 {
    let sign = select(-1.0, 1.0, x >= 0.0);
    let a = abs(x);
    let t = 1.0 / (1.0 + 0.3275911 * a);
    let polynomial = (((((1.061405429 * t - 1.453152027) * t) + 1.421413741) * t
        - 0.284496736) * t + 0.254829592) * t;
    return sign * (1.0 - polynomial * exp(-a * a));
}

fn normal_cdf(x: f32, sigma: f32) -> f32 {
    return 0.5 * (1.0 + erf_approx(x / (sigma * 1.41421356237)));
}

// Horizontal cross-section of the rounded source shape at source y.
fn rounded_x_bounds(y: f32, half: vec2<f32>, radii: vec4<f32>) -> vec2<f32> {
    var left = -half.x;
    var right = half.x;
    if y < -half.y + radii.x {
        let dy = y - (-half.y + radii.x);
        left = -half.x + radii.x - sqrt(max(radii.x * radii.x - dy * dy, 0.0));
    } else if y > half.y - radii.w {
        let dy = y - (half.y - radii.w);
        left = -half.x + radii.w - sqrt(max(radii.w * radii.w - dy * dy, 0.0));
    }
    if y < -half.y + radii.y {
        let dy = y - (-half.y + radii.y);
        right = half.x - radii.y + sqrt(max(radii.y * radii.y - dy * dy, 0.0));
    } else if y > half.y - radii.z {
        let dy = y - (half.y - radii.z);
        right = half.x - radii.z + sqrt(max(radii.z * radii.z - dy * dy, 0.0));
    }
    return vec2<f32>(left, right);
}

fn gaussian_rounded_coverage(p: vec2<f32>, half: vec2<f32>, radii: vec4<f32>, sigma: f32) -> f32 {
    if all(radii == vec4<f32>(0.0)) {
        let x = normal_cdf(half.x - p.x, sigma) - normal_cdf(-half.x - p.x, sigma);
        let y = normal_cdf(half.y - p.y, sigma) - normal_cdf(-half.y - p.y, sigma);
        return clamp(x * y, 0.0, 1.0);
    }

    let y0 = max(-half.y, p.y - 4.0 * sigma);
    let y1 = min(half.y, p.y + 4.0 * sigma);
    if y1 <= y0 { return 0.0; }
    let steps = 24.0;
    let dy = (y1 - y0) / steps;
    let inv_norm = 0.3989422804 / sigma;
    var coverage = 0.0;
    for (var i = 0.0; i < steps; i = i + 1.0) {
        let source_y = y0 + (i + 0.5) * dy;
        let delta_y = (source_y - p.y) / sigma;
        let weight_y = exp(-0.5 * delta_y * delta_y) * inv_norm * dy;
        let bounds = rounded_x_bounds(source_y, half, radii);
        let coverage_x = normal_cdf(bounds.y - p.x, sigma)
            - normal_cdf(bounds.x - p.x, sigma);
        coverage = coverage + weight_y * coverage_x;
    }
    return clamp(coverage, 0.0, 1.0);
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
    var outer_cov = clamp(0.5 - d_outer, 0.0, 1.0);
    if globals.clip_mode!=0u && in.clip_meta.z!=0u {outer_cov=0.0;}

    // 描边带：外形内缩 border_px，圆角半径也相应缩小。
    var border_cov = 0.0;
    var inner_cov = 0.0;
    if in.border_px > 0.0 {
        let d_inner = sd_round_rect(p, in.half_px - vec2<f32>(in.border_px), max(r - in.border_px, 0.0));
        inner_cov = clamp(0.5 - d_inner, 0.0, 1.0);
        border_cov = 1.0 - inner_cov;
    }

    // Ring coverage is the difference between outer and inner coverage.
    // Multiplying the two AA ramps makes subpixel hairlines pulse in brightness
    // during scrolling. Composite the ring exactly once over the covered fill.
    let fill = premultiply(in.fill);
    var foreground: vec4<f32>;
    if globals.clip_mode!=0u {
        // Shadow instances have no foreground. Do not traverse the chain here;
        // their original gradient is multiplied by the full chain once below.
        var k=1.0;
        if in.shadow_color.a<=0.0 {k=clip_coverage(in.position.xy,in.clip_meta.x);}
        let u=min(outer_cov,k);
        var i=outer_cov;
        if in.border_px>0.0 {
            i=0.0;
            if in.clip_meta.y==0u {
                let d=sd_round_rect(p,in.half_px-vec2<f32>(in.border_px),max(r-in.border_px,0.0));
                i=clamp(0.5-d,0.0,1.0);
            }
        }
        // Geometric inner subset is checked separately on CPU; this clamp only
        // constrains the f32 AA approximation to 0<=V<=U<=1.
        let v=min(min(i,k),u);
        let ring=max(u-v,0.0);
        foreground=fill*u;
        if in.border_px>0.0 {
            let b=premultiply(in.border_color);
            foreground=b*ring+fill*(u-b.a*ring);
        }
    } else if outer_cov < 0.99999 && inner_cov > 0.00001 {
        let ring = max(outer_cov - inner_cov, 0.0);
        let border = premultiply(in.border_color) * ring;
        foreground = border + fill * (outer_cov - border.a);
    } else {
        // Preserve the calibrated output where the two AA ramps do not overlap.
        let border = premultiply(in.border_color) * border_cov;
        foreground = (border + fill * (1.0 - border.a)) * outer_cov;
    }
    // The no-shadow path returns the exact same calibrated foreground pixels.
    if in.shadow_color.a <= 0.0 { return foreground; }
    var shadow_coverage: f32;
    if in.shadow_sigma_px <= 0.0 {
        let shadow_p = in.position.xy - in.shadow_center_px;
        let shadow_r = select(
            select(in.shadow_radii_px.x, in.shadow_radii_px.w, shadow_p.y > 0.0),
            select(in.shadow_radii_px.y, in.shadow_radii_px.z, shadow_p.y > 0.0),
            shadow_p.x > 0.0,
        );
        shadow_coverage = clamp(0.5 - sd_round_rect(shadow_p, in.shadow_half_px, shadow_r), 0.0, 1.0);
    } else {
        let shadow_p = in.position.xy - in.shadow_center_px;
        let max_radius = min(in.shadow_half_px.x, in.shadow_half_px.y);
        let shadow_radii = min(in.shadow_radii_px, vec4<f32>(max_radius));
        shadow_coverage = gaussian_rounded_coverage(
            shadow_p,
            in.shadow_half_px,
            shadow_radii,
            in.shadow_sigma_px,
        );
    }
    let shadow = premultiply(in.shadow_color) * shadow_coverage * clip_coverage(in.position.xy,0u);
    return foreground + shadow * (1.0 - foreground.a);
}
