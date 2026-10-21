struct Camera { matrix:mat4x4<f32>, model:mat4x4<f32> }
struct Vertex { p:vec4<f32>, n:vec4<f32>, uv:vec4<f32>, color:vec4<f32> }
struct Policy { counts:vec4<u32>, sizes:vec4<f32>, motion:vec4<f32> }
// Both draw layouts share count at byte 4; the fifth word is only used by indexed draws.
struct Arguments { vertices:u32, count:u32, first:u32, base:u32, first_instance:u32 }
@group(0) @binding(0) var<uniform> camera:Camera;
@group(0) @binding(4) var<uniform> viewport:vec4<f32>;
@group(0) @binding(5) var<storage,read> vertices:array<Vertex>;
@group(0) @binding(6) var<storage,read_write> visible:array<Instance>;
@group(0) @binding(7) var<storage,read_write> args:array<Arguments>;
@group(0) @binding(8) var<uniform> bounds_policy:Policy;
@group(0) @binding(9) var<storage,read_write> extent:array<vec4<f32>>;
@group(0) @binding(10) var<storage,read> policies:array<Policy>;
var<workgroup> lower:array<vec3<f32>,64>;
var<workgroup> upper:array<vec3<f32>,64>;
@compute @workgroup_size(64) fn bounds(@builtin(local_invocation_index) lane:u32) {
    let policy=bounds_policy;
    var lo=vec3(1e30);var hi=vec3(-1e30);
    for(var j=lane;j<policy.counts.y;j+=64u) {
        lo=min(lo,vertices[j].p.xyz);hi=max(hi,vertices[j].p.xyz);
    }
    lower[lane]=lo;upper[lane]=hi;workgroupBarrier();
    for(var stride=32u;stride>0u;stride/=2u) {
        if lane<stride {lower[lane]=min(lower[lane],lower[lane+stride]);upper[lane]=max(upper[lane],upper[lane+stride]);}
        workgroupBarrier();
    }
    if lane!=0u {return;}
    extent[policy.counts.w*2u]=vec4((lower[0]+upper[0])*0.5,0.0);
    extent[policy.counts.w*2u+1u]=vec4((upper[0]-lower[0])*0.5,0.0);
}
@compute @workgroup_size(1) fn compact(@builtin(global_invocation_id) id:vec3<u32>) {
    if id.x>=arrayLength(&policies) {return;}
    let policy=policies[id.x];
    let center=extent[id.x*2u].xyz;
    let base_radius=length(extent[id.x*2u+1u].xyz);
    let matrix=camera.matrix*camera.model;
    var count=0u;
    // Stable order, no atomics or CPU readback; one independently scheduled workgroup per primitive in one dispatch.
    for(var j=0u;j<policy.counts.x;j++) {
        var instance=instances[policy.counts.z+j];
        let moved=move_value(center,vec3(0.0,1.0,0.0),instance);
        let scale=max(instance.scale.x,max(instance.scale.y,instance.scale.z));
        var padding=policy.sizes.w;
        if instance.effect.x==1.0 {
            let h=max(center.y+base_radius-instance.pivot.y,0.0);
            padding+=2.0*length(environment.clock_wind.yz)*abs(environment.clock_wind.w*instance.effect.y)*h*h;
        } else if instance.effect.x==2.0 || instance.effect.x==6.0 || instance.effect.x==7.0 {
            padding+=3.0*abs(instance.effect.y);
        }
        let radius=(base_radius+padding)*scale;
        let clip=matrix*vec4(moved.p,1.0);
        // Six homogeneous frustum planes, conservatively expanded by sphere.
        var inside=true;
        for(var k=0u;k<6u;k++) {
            let axis=k/2u;let sign=select(-1.0,1.0,k%2u==0u && k!=4u);
            var plane=vec4(matrix[0][3]+sign*matrix[0][axis],matrix[1][3]+sign*matrix[1][axis],matrix[2][3]+sign*matrix[2][axis],matrix[3][3]+sign*matrix[3][axis]);
            if k==5u {plane=vec4(matrix[0][2],matrix[1][2],matrix[2][2],matrix[3][2]);}
            if dot(plane,vec4(moved.p,1.0)) < -radius*length(plane.xyz) {inside=false;}
        }
        if !inside || moved.visibility<=0.0 {continue;}
        if instance.removal.w>0.0 && instance_time(instance)>=instance.removal.x+instance.removal.y {continue;}
        let diameter=select(base_radius*2.0,policy.sizes.x,policy.sizes.x>0.0)*scale;
        let row_y=vec3(matrix[0][1],matrix[1][1],matrix[2][1]);
        let pixels=diameter*length(row_y)*viewport.y*0.5/max(abs(clip.w),0.00001);
        let fade=select(1.0,smoothstep(policy.sizes.y,policy.sizes.z,pixels),policy.sizes.z>0.0);
        if fade<=0.0 {continue;}
        instance.detail.x=fade;
        if policy.motion.y>0.0 && (instance.effect.x==1.0 || instance.effect.x==3.0) {
            let motion=smoothstep(policy.motion.x,policy.motion.y,pixels);
            instance.effect.y*=motion;
            if motion<=0.0 {instance.effect.x=0.0;}
        }
        visible[policy.counts.z+count]=instance;count++;
    }
    args[id.x].count=count;
}
