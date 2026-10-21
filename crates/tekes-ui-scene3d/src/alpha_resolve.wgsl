@group(0) @binding(0) var linear_composite: texture_2d<f32>;
@vertex fn vs(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
    let p = array<vec2<f32>,3>(vec2(-1.0,-1.0),vec2(3.0,-1.0),vec2(-1.0,3.0));
    return vec4(p[i],0.0,1.0);
}
fn encode(c: vec3<f32>) -> vec3<f32> {
    return select(1.055*pow(max(c,vec3(0.0)),vec3(1.0/2.4))-0.055,c*12.92,c<=vec3(0.0031308));
}
fn decode(c: vec3<f32>) -> vec3<f32> {
    return select(pow((c+0.055)/1.055,vec3(2.4)),c/12.92,c<=vec3(0.04045));
}
@fragment fn fs(@builtin(position) p: vec4<f32>) -> @location(0) vec4<f32> {
    let c=textureLoad(linear_composite,vec2<i32>(p.xy),0);
    let encoded_premultiplied=encode(c.rgb/max(c.a,0.00001))*c.a;
    // Output target is sRGB, so undo its automatic encoding once here.
    return vec4(decode(encoded_premultiplied),c.a);
}
