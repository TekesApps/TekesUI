struct InteriorEnvironment { clock_wind:vec4<f32>, ambient_emission:vec4<f32>, direction:vec4<f32>, directional:vec4<f32>, clocks:vec4<f32> }
@group(0) @binding(0) var<uniform> interior_environment:InteriorEnvironment;
@group(0) @binding(1) var<storage,read_write> style_output:array<RoomStyle,32>;
@compute @workgroup_size(32) fn evaluate_styles(@builtin(global_invocation_id) id:vec3<u32>) {
    if id.x>=32u {return;}
    let t=interior_environment.clock_wind.x;
    let night_t=max(0.0,t-interior_environment.directional.w);
    style_output[id.x]=evaluate_room_style(id.x,t,night_t,clamp(interior_environment.ambient_emission.w,0.0,1.0));
}
