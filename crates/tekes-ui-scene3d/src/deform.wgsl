// One invocation deforms one original vertex. Indices and morph deltas remain
// unexpanded; flat normals are evaluated per face by the indexed-pull vertex path.
struct BaseVertex {
    position: vec4<f32>, normal: vec4<f32>, uv: vec4<f32>, color: vec4<f32>,
    joints: vec4<u32>, weights: vec4<f32>,
}
struct Delta { position: vec4<f32>, normal: vec4<f32> }
struct Vertex { position: vec4<f32>, normal: vec4<f32>, uv: vec4<f32>, color: vec4<f32> }
struct Params { model: mat4x4<f32>, counts: vec4<u32> }
@group(0) @binding(0) var<storage,read> vertices: array<BaseVertex>;
@group(0) @binding(1) var<storage,read> deltas: array<Delta>;
@group(0) @binding(2) var<storage,read> palette: array<mat4x4<f32>>;
@group(0) @binding(3) var<storage,read> morph_weights: array<f32>;
@group(0) @binding(4) var<storage,read_write> output: array<Vertex>;
@group(0) @binding(5) var<uniform> params: Params;

fn unit(v: vec3<f32>) -> vec3<f32> {
    let len = length(v);
    if len > 0.0 { return v / len; }
    return vec3(0.0);
}
fn deformed(index: u32) -> Vertex {
    let v = vertices[index];
    var p = v.position.xyz;
    var n = v.normal.xyz;
    // glTF order is morph first, then skin/world transform.
    for (var k = 0u; k < params.counts.y; k++) {
        let delta = deltas[k * params.counts.x + index];
        let weight = morph_weights[k];
        p += delta.position.xyz * weight;
        n += delta.normal.xyz * weight;
    }
    var matrix = params.model;
    if params.counts.z != 0u {
        let w = v.weights / dot(v.weights,vec4(1.0));
        matrix = palette[v.joints.x] * w.x + palette[v.joints.y] * w.y
            + palette[v.joints.z] * w.z + palette[v.joints.w] * w.w;
    }
    // Inverse transpose of the blended affine transform's 3x3 linear part,
    // including non-uniform and negative scales. Singular matrices yield zero.
    let a = matrix[0].xyz; let b = matrix[1].xyz; let c = matrix[2].xyz;
    let c0 = cross(b,c); let c1 = cross(c,a); let c2 = cross(a,b);
    let determinant = dot(a,c0);
    var normal = vec3(0.0);
    if abs(determinant) > 1e-10 { normal = unit((c0*n.x+c1*n.y+c2*n.z)/determinant); }
    var result: Vertex;
    result.position = vec4((matrix * vec4(p,1.0)).xyz,1.0);
    result.normal = vec4(normal,0.0);
    if params.counts.w != 0u { result.normal = vec4(0.0); }
    result.uv = v.uv; result.color = v.color;
    return result;
}
@compute @workgroup_size(64)
fn deform(@builtin(global_invocation_id) gid: vec3<u32>) {
    let index = gid.x;
    if index >= params.counts.x { return; }
    output[index] = deformed(index);
}
