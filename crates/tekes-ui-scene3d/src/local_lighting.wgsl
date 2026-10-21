struct LocalLight { position_radius:vec4<f32>, direction_cone:vec4<f32>, color:vec4<f32> }
struct ShadowProxy { center:vec4<f32>, radii:vec4<f32> }
struct ShadowList { count:u32, indices:array<u32,96> }
struct LightScene { counts:vec4<u32>, lights:array<LocalLight,64>, shadows:array<ShadowProxy,96>, shadow_lists:array<ShadowList,64>, grid:vec4<f32>, light_cells:array<vec2<u32>,1024> }
@group(2) @binding(0) var<storage,read> local_scene:LightScene;
fn light_candidates(p:vec3<f32>)->vec2<u32>{
    // Outside bounds clamps to an edge cell, conservatively retaining extra
    // candidates rather than risking a boundary precision false negative.
    let cell=vec2<u32>(clamp(floor((p.xz-local_scene.grid.xy)*local_scene.grid.zw),vec2(0.0),vec2(31.0)));
    return local_scene.light_cells[cell.y*32u+cell.x];
}
fn local_illumination(p:vec3<f32>,normal:vec3<f32>)->vec3<f32>{
    var result=vec3(0.0);
    if environment.ambient_emission.w<=0.0 {return result;}
    let n=normalize(normal);
    var candidates=light_candidates(p);
    loop {
        if all(candidates==vec2(0u)) {break;}
        var j=0u;
        if candidates.x!=0u {
            j=firstTrailingBit(candidates.x);candidates.x&=candidates.x-1u;
        }else {
            j=32u+firstTrailingBit(candidates.y);candidates.y&=candidates.y-1u;
        }
        if j>=local_scene.counts.x {break;}
        let light=local_scene.lights[j];
        if all(light.color.rgb<=vec3(0.0)) {continue;}
        let delta=light.position_radius.xyz-p;
        let d2=dot(delta,delta);let radius=light.position_radius.w;
        if d2>=radius*radius || d2<0.00001 {continue;}
        let direction=delta*inverseSqrt(d2);let cosine=max(dot(n,direction),0.0);
        if cosine<=0.0 {continue;}
        var spot=1.0;
        if light.direction_cone.w > -0.99 {
            spot=smoothstep(light.direction_cone.w,min(1.0,light.direction_cone.w+0.18),dot(-direction,light.direction_cone.xyz));
        }
        if spot<=0.0 {continue;}
        var visibility=1.0;
        // Offset receiver to avoid self-shadow. Segment/ellipsoid intersection
        // gives bounded soft proxy shadows without CPU rasterization/readback.
        let origin=p+n*0.035;
        for(var candidate=0u;candidate<local_scene.shadow_lists[j].count;candidate++){
            let k=local_scene.shadow_lists[j].indices[candidate];
            let proxy=local_scene.shadows[k];
            if proxy.center.w<=0.01 {continue;}
            let r=proxy.radii.xyz*max(0.02,proxy.center.w);
            let o=(origin-proxy.center.xyz)/r;
            if dot(o,o)<1.05 {continue;}
            let ray=(light.position_radius.xyz-origin)/r;
            let along=clamp(-dot(o,ray)/dot(ray,ray),0.0,1.0);
            let nearest=o+ray*along;
            if along>0.005 && along<0.995 {
                visibility=min(visibility,smoothstep(0.75,1.12,length(nearest)));
            }
        }
        let falloff=pow(1.0-d2/(radius*radius),2.0)/(1.0+0.24*d2);
        result+=light.color.rgb*(cosine*spot*falloff*visibility);
    }
    return result*environment.ambient_emission.w;
}
