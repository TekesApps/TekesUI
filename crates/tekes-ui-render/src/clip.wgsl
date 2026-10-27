// Analytic clip contract v1. builtin position already names the physical pixel
// center. Current target API has origin=(0,0); p_logical=origin+p_px/scale.
// Distance is evaluated in physical units, equivalent under this uniform scale.
struct Globals {
    viewport_px: vec2<f32>, scale: f32, head: u32,
    origin: vec2<f32>, clip_mode: u32, max_steps: u32,
}
@group(0) @binding(0) var<uniform> globals: Globals;
struct ClipRecord {rect:vec4<f32>,radii:vec4<f32>,links:vec4<u32>}
@group(0) @binding(1) var<storage,read> clip_records:array<ClipRecord>;
fn sd_round_rect(p:vec2<f32>,b:vec2<f32>,r:f32)->f32 {
    let q=abs(p)-b+vec2<f32>(r);
    return min(max(q.x,q.y),0.0)+length(max(q,vec2<f32>(0.0)))-r;
}
fn clip_shape_coverage(p_px:vec2<f32>,shape:ClipRecord)->f32 {
    if shape.links.z!=0u {return 0.0;}
    let half_px=shape.rect.zw*globals.scale*0.5;
    let center_px=(shape.rect.xy-globals.origin)*globals.scale+half_px;
    let p=p_px-center_px;
    let radii=shape.radii*globals.scale;
    let r=select(select(radii.x,radii.w,p.y>0.0),select(radii.y,radii.z,p.y>0.0),p.x>0.0);
    return clamp(0.5-sd_round_rect(p,half_px,r),0.0,1.0);
}
fn clip_coverage(p_px:vec2<f32>,exclude_identity:u32)->f32 {
    if globals.clip_mode==0u {return 1.0;}
    var head=globals.head;
    var coverage=1.0;
    for(var step=0u;step<globals.max_steps && head!=0u;step=step+1u) {
        // CPU preflight already checked DAG/depth/capacity. Defensive shader checks
        // are fail-closed too; they never return an unclipped fallback.
        if head>arrayLength(&clip_records) || head>globals.max_steps {return 0.0;}
        let shape=clip_records[head-1u];
        if shape.links.y!=exclude_identity {coverage=min(coverage,clip_shape_coverage(p_px,shape));}
        head=shape.links.x;
    }
    if head!=0u {return 0.0;}
    return coverage;
}
