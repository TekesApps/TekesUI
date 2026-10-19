struct Camera { matrix: mat4x4<f32>, model: mat4x4<f32> }
@group(0) @binding(0) var<uniform> camera: Camera;
struct Material { color: vec4<f32>, cutoff: vec4<f32> }
@group(1) @binding(0) var<uniform> material: Material;
@group(1) @binding(1) var base: texture_2d<f32>;
@group(1) @binding(2) var filtering: sampler;
struct Out {
    @builtin(position) position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
}
@vertex fn vs(@location(0) p: vec4<f32>, @location(1) n: vec4<f32>,
    @location(2) uv: vec4<f32>, @location(3) color: vec4<f32>) -> Out {
    var o: Out; o.position = camera.matrix * camera.model * p; o.normal = (camera.model * vec4(n.xyz,0.0)).xyz; o.uv = uv.xy; o.color = color; return o;
}
// Face-local normals cannot be represented by one normal on a shared vertex.
// Pull through the original index list for flat shading, without vertex splits,
// face buffers, primitive_index features or screen-space derivative approximations.
struct Vertex { position: vec4<f32>, normal: vec4<f32>, uv: vec4<f32>, color: vec4<f32> }
@group(2) @binding(0) var<storage,read> deformed_vertices: array<Vertex>;
@group(2) @binding(1) var<storage,read> indices: array<u32>;
fn unit(v: vec3<f32>) -> vec3<f32> {
    let len = length(v);
    if len > 0.0 { return v/len; }
    return vec3(0.0);
}
@vertex fn vs_flat(@builtin(vertex_index) corner: u32) -> Out {
    let vertex = deformed_vertices[indices[corner]];
    let first = (corner/3u)*3u;
    let a = deformed_vertices[indices[first]].position.xyz;
    let b = deformed_vertices[indices[first+1u]].position.xyz;
    let c = deformed_vertices[indices[first+2u]].position.xyz;
    var o: Out;
    o.position = camera.matrix * camera.model * vertex.position;
    o.normal = (camera.model * vec4(unit(cross(b-a,c-a)),0.0)).xyz;
    o.uv = vertex.uv.xy; o.color = vertex.color;
    return o;
}
@fragment fn fs(i: Out) -> @location(0) vec4<f32> {
    let c = textureSample(base,filtering,i.uv) * material.color * i.color;
    if c.a < material.cutoff.x { discard; }
    let n = i.normal / max(length(i.normal),0.00001);
    let light = 0.3 + 0.7 * abs(dot(n,normalize(vec3(0.4,0.7,1.0))));
    // sRGB render target performs linear -> sRGB encoding. Opaque/mask only.
    return vec4(c.rgb * light,1.0);
}
