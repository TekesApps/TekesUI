// Opt-in atlas interiors, no geometry/sprite uploads and no host-owned timers.
@group(0) @binding(4) var<storage,read> room_styles:array<RoomStyle,32>;
struct InteriorSample { state:vec4<f32>, fabric:vec3<f32>, lamp:vec3<f32> }
fn capsule_distance(p:vec2<f32>,a:vec2<f32>,b:vec2<f32>,r:f32)->f32 {
    let v=b-a;let h=clamp(dot(p-a,v)/max(dot(v,v),0.000001),0.0,1.0);
    return length(p-a-v*h)-r;
}
// x = silhouette coverage, y = room lighting. UV cell and face distinguish
// rooms while keeping each room stable under camera/model rotation.
fn interior_state(uv:vec2<f32>,normal:vec3<f32>)->InteriorSample{
    let grid=uv*material.interior.xy;let q=fract(grid);
    let key=vec3(floor(grid),material.interior.z+dot(normalize(normal),vec3(13.0,17.0,23.0)));
    let style=room_styles[min(u32(room_hash(key)*32.0),31u)];
    // Permute each 4x4 tile: 50% occupancy assigns exactly eight cells,
    // permanently, independently of night light activity and actor pose.
    let cell=vec2<u32>(floor(fract(grid/4.0)*4.0));
    let offset=u32(room_hash(vec3(floor(grid/4.0),key.z))*16.0);
    let slot=((cell.x+cell.y*4u)*5u+offset)%16u;
    var ready=1.0;
    if material.interior_activation.x>0.0 {
        ready=smoothstep(material.interior_activation.y,material.interior_activation.z,
            environment.clocks[u32(material.interior_activation.x)-1u]);
    }
    let resident=step(f32(slot)+0.5,material.interior.w*16.0)*ready;
    let dx=dpdx(grid);let dy=dpdy(grid);
    let footprint=max(length(vec2(dx.x,dy.x)),length(vec2(dx.y,dy.y)));
    let pixels=1.0/max(footprint,0.00001);
    let actor_detail=smoothstep(12.0,16.0,pixels);
    let curtain_detail=smoothstep(6.0,8.0,pixels);
    let fold_detail=smoothstep(20.0,24.0,pixels);
    let closure=style.state.x;
    let edge_aa=max(length(vec2(dx.x,dy.x)),0.003);
    var panels=0.0;
    if curtain_detail>0.0 { panels=clamp(1.0-smoothstep(0.16+0.35*closure-edge_aa,0.16+0.35*closure+edge_aa,q.x)
        +smoothstep(0.86-0.35*closure-edge_aa,0.86-0.35*closure+edge_aa,q.x),0.0,1.0); }
    let curtain=panels*style.state.y*resident*curtain_detail;
    var silhouette=0.0;
    if actor_detail>0.0 {
    let x=style.pose.x;let head_y=style.pose.y;
    let walking=1.0-clamp((head_y-0.47)/0.16,0.0,1.0);
    let head=vec2(x,head_y);let shoulder=head+vec2(0.0,0.09);let hip=head+vec2(0.0,0.26);
    var distance=length(q-head)-0.041;
    distance=min(distance,capsule_distance(q,shoulder,hip,0.052));
    let swing=style.pose.z;
    distance=min(distance,capsule_distance(q,hip,hip+vec2(-0.035-swing,0.15*walking),0.022));
    distance=min(distance,capsule_distance(q,hip,hip+vec2(0.035+swing,0.15*walking),0.022));
    let reading=style.pose.w;
    let hand=shoulder+vec2(0.10,0.14-0.08*reading);
    distance=min(distance,capsule_distance(q,shoulder,hand,0.023));
    distance=min(distance,capsule_distance(q,shoulder+vec2(-0.035,0.0),shoulder+vec2(-0.08,0.15+swing),0.023));
    let book=abs(q-(hand+vec2(0.02,-0.01)))-vec2(0.045,0.023);
    distance=min(distance,max(book.x,book.y)+(1.0-reading));
    let aa=max(footprint,0.003);
    silhouette=(1.0-smoothstep(-aa,aa,distance))*style.state.z*resident*actor_detail;
    }
    var sample:InteriorSample;
    sample.state=vec4(silhouette,style.state.w*resident,resident,curtain);
    let contrast=0.17*fold_detail;
    var fold=0.0;
    if fold_detail>0.0 {fold=contrast*cos(q.x*62.83185);}
    sample.fabric=style.fabric.rgb*(0.83+fold);
    sample.lamp=mix(vec3(1.0,0.90,0.76),vec3(0.90,0.96,1.0),style.fabric.w);
    return sample;
}
