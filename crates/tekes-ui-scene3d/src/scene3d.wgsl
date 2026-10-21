struct Camera { matrix: mat4x4<f32>, model: mat4x4<f32> }
@group(0) @binding(0) var<uniform> camera: Camera;
struct Material { color: vec4<f32>, cutoff: vec4<f32>, emissive: vec4<f32>, interior:vec4<f32>, dimming:vec4<f32>, interior_activation:vec4<f32> }
@group(1) @binding(0) var<uniform> material: Material;
@group(1) @binding(1) var base: texture_2d<f32>;
@group(1) @binding(2) var filtering: sampler;
@group(1) @binding(3) var emission: texture_2d<f32>;
struct Out {
    @builtin(position) position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) reveal: f32,
    @location(4) world:vec3<f32>,
    @location(5) world_normal:vec3<f32>,
    @location(6) visibility:f32,
}
@vertex fn vs(@location(0) p: vec4<f32>, @location(1) n: vec4<f32>,
    @location(2) uv: vec4<f32>, @location(3) color: vec4<f32>, @builtin(instance_index) instance:u32) -> Out {
    let moved=move_instance(p.xyz,n.xyz,instance);
    var o: Out; o.position = camera.matrix * camera.model * vec4(moved.p,1.0); o.normal = (camera.model * vec4(moved.n,0.0)).xyz; o.uv = uv.xy; o.color = color*moved.color; o.reveal=moved.reveal; o.world=moved.p; o.world_normal=moved.n; o.visibility=moved.visibility; return o;
}
// Face-local normals cannot be represented by one normal on a shared vertex.
// Pull through the original index list for flat shading, without vertex splits,
// face buffers, primitive_index features or screen-space derivative approximations.
struct Vertex { position: vec4<f32>, normal: vec4<f32>, uv: vec4<f32>, color: vec4<f32> }
@group(3) @binding(0) var<storage,read> deformed_vertices: array<Vertex>;
@group(3) @binding(1) var<storage,read> indices: array<u32>;
fn unit(v: vec3<f32>) -> vec3<f32> {
    let len = length(v);
    if len > 0.0 { return v/len; }
    return vec3(0.0);
}
@vertex fn vs_flat(@builtin(vertex_index) corner: u32, @builtin(instance_index) instance:u32) -> Out {
    let vertex = deformed_vertices[indices[corner]];
    let first = (corner/3u)*3u;
    let a = deformed_vertices[indices[first]].position.xyz;
    let b = deformed_vertices[indices[first+1u]].position.xyz;
    let c = deformed_vertices[indices[first+2u]].position.xyz;
    var o: Out;
    let moved=move_instance(vertex.position.xyz,unit(cross(b-a,c-a)),instance);
    o.position = camera.matrix * camera.model * vec4(moved.p,1.0);
    o.normal = (camera.model * vec4(moved.n,0.0)).xyz;
    o.uv = vertex.uv.xy; o.color = vertex.color*moved.color; o.reveal=moved.reveal; o.world=moved.p; o.world_normal=moved.n; o.visibility=moved.visibility;
    return o;
}
@fragment fn fs(i: Out) -> @location(0) vec4<f32> {
    var c = textureSample(base,filtering,i.uv) * material.color * i.color;
    var emission_sample = textureSample(emission,filtering,i.uv).rgb;
    var room_light=1.0;
    var silhouette=0.0;
    var room_tint=vec3(1.0);
    if material.interior.x>0.0 {
        let sample=interior_state(i.uv,i.world_normal);
        let state=sample.state;
        room_tint=sample.lamp;
        room_light=state.y;
        let glass_mask=smoothstep(0.01,0.10,max(emission_sample.r,max(emission_sample.g,emission_sample.b)));
        c=vec4(mix(c.rgb,vec3(0.014,0.021,0.026),glass_mask*(1.0-state.z)*0.94),c.a);
        let curtain=state.w*glass_mask;
        let fabric=sample.fabric;
        c=vec4(mix(c.rgb,fabric,curtain),c.a);
        emission_sample=mix(emission_sample,fabric*1.15,curtain);
        silhouette=state.x*glass_mask*(1.0-curtain);
        c=vec4(mix(c.rgb,vec3(0.045,0.038,0.033),silhouette*0.85),c.a);
    }
    // Stable world-space stipple fades opaque instances without a sorted alpha pass.
    if i.reveal < 0.0 || c.a < material.cutoff.x || i.visibility<=0.0 { discard; }
    if i.visibility<1.0 {
        let stipple=fract(sin(dot(floor(i.world*240.0),vec3(12.9898,78.233,37.719)))*43758.5453);
        if i.visibility<=stipple {discard;}
    }
    let n = i.normal / max(length(i.normal),0.00001);
    let light = environment.ambient_emission.rgb + environment.directional.rgb * abs(dot(n,normalize(environment.direction.xyz))) + local_illumination(i.world,i.world_normal);
    var glow = vec3(0.0);
    if environment.ambient_emission.w > 0.0 && any(material.emissive.rgb > vec3(0.0)) {
        glow = emission_sample * room_tint * material.emissive.rgb * environment.ambient_emission.w * room_light * (1.0-0.94*silhouette) * dimming_curve(material.dimming);
    }
    // Transparent draws follow opaque draws, use source-alpha blending and
    // test depth without writing it. The sRGB target encodes the final result.
    return vec4(c.rgb * light + glow,select(1.0,c.a,material.cutoff.y > 0.5));
}
