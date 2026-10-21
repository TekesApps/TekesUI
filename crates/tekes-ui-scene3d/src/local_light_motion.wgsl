struct LightInput { attachment:Instance, center_radius:vec4<f32>, direction_cone:vec4<f32>, color:vec4<f32>, dimming:vec4<f32> }
struct LocalLight { position_radius:vec4<f32>, direction_cone:vec4<f32>, color:vec4<f32> }
struct ShadowProxy { center:vec4<f32>, radii:vec4<f32> }
struct ShadowList { count:u32, indices:array<u32,96> }
struct LightScene { counts:vec4<u32>, lights:array<LocalLight,64>, shadows:array<ShadowProxy,96>, shadow_lists:array<ShadowList,64>, grid:vec4<f32>, light_cells:array<vec2<u32>,1024> }
@group(1) @binding(0) var<storage,read> light_inputs:array<LightInput>;
@group(1) @binding(1) var<storage,read_write> light_output:LightScene;
@compute @workgroup_size(64) fn evaluate(@builtin(global_invocation_id) id:vec3<u32>){
    let index=id.x;let lights=light_output.counts.x;
    if index>=lights+light_output.counts.y {return;}
    let input=light_inputs[index];
    let moved=move_value(input.center_radius.xyz,input.direction_cone.xyz*input.attachment.scale.xyz,input.attachment);
    if index<lights {
        light_output.lights[index].position_radius=vec4(moved.p,input.center_radius.w);
        light_output.lights[index].direction_cone=vec4(normalize(moved.n),input.direction_cone.w);
        var energy=dimming_curve(input.dimming);
        if input.attachment.removal.w>0.0 {
            energy*=1.0-smoothstep(input.attachment.removal.x,input.attachment.removal.x+input.attachment.removal.y,instance_time(input.attachment));
        }
        light_output.lights[index].color=vec4(input.color.rgb*energy,0.0);
    } else {
        light_output.shadows[index-lights].center=vec4(moved.p,moved.visibility);
        light_output.shadows[index-lights].radii=vec4(input.direction_cone.xyz*input.attachment.scale.xyz,0.0);
    }
}

// A receiver inside the light's radius and the light itself lie in a convex
// sphere. Their entire shadow ray stays inside that sphere. An ellipsoid whose
// bounding box misses the sphere cannot occlude any such receiver. Build this
// conservative list after attachment motion, once per light, entirely on GPU.
// Keep original proxy order and the exact per-fragment soft-shadow calculation.
@compute @workgroup_size(64) fn cull_shadows(@builtin(global_invocation_id) id:vec3<u32>){
    let index=id.x;
    if index>=light_output.counts.x {return;}
    let light=light_output.lights[index];
    var count=0u;
    if any(light.color.rgb>vec3(0.0)) {
        // Include the receiver's 0.035 normal bias and the entire 1.12-radius
        // soft penumbra; culling must not remove grazing shadows at the edge.
        let radius=light.position_radius.w+0.035;
        for(var k=0u;k<light_output.counts.y;k++){
            let proxy=light_output.shadows[k];
            if proxy.center.w<=0.01 {continue;}
            let r=proxy.radii.xyz*max(0.02,proxy.center.w)*1.12;
            let nearest=max(abs(proxy.center.xyz-light.position_radius.xyz)-r,vec3(0.0));
            if dot(nearest,nearest)<=radius*radius*1.00001 {
                light_output.shadow_lists[index].indices[count]=k;
                count++;
            }
        }
    }
    light_output.shadow_lists[index].count=count;
}

// Conservative world-XZ broad phase: all animated light spheres contribute to
// bounds; each cell keeps two ordered bit masks, with no truncation/overflow.
// Independent dispatches publish the bounds before cell construction.
@compute @workgroup_size(1) fn grid_bounds(){
    var lo=vec2(1e30);var hi=vec2(-1e30);
    for(var j=0u;j<light_output.counts.x;j++){
        let light=light_output.lights[j];
        let radius=light.position_radius.w*1.00001+0.00001;
        lo=min(lo,light.position_radius.xz-vec2(radius));
        hi=max(hi,light.position_radius.xz+vec2(radius));
    }
    if light_output.counts.x==0u {lo=vec2(-1.0);hi=vec2(1.0);}
    let step=max((hi-lo)/32.0,vec2(0.00001));
    light_output.grid=vec4(lo,1.0/step);
}
@compute @workgroup_size(64) fn cull_lights(@builtin(global_invocation_id) id:vec3<u32>){
    let cell=id.x;if cell>=1024u {return;}
    let step=1.0/light_output.grid.zw;
    let lo=light_output.grid.xy+vec2(f32(cell%32u),f32(cell/32u))*step;
    let hi=lo+step;
    var mask=vec2(0u);
    for(var j=0u;j<light_output.counts.x;j++){
        let light=light_output.lights[j];
        let nearest=max(max(lo-light.position_radius.xz,light.position_radius.xz-hi),vec2(0.0));
        let radius=light.position_radius.w*1.00001+0.00001;
        if any(light.color.rgb>vec3(0.0)) && dot(nearest,nearest)<=radius*radius {
            if j<32u {mask.x|=1u<<j;}else {mask.y|=1u<<(j-32u);}
        }
    }
    light_output.light_cells[cell]=mask;
}
