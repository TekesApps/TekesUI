struct Environment { clock_wind: vec4<f32> }
struct Instance {
    position_yaw: vec4<f32>, scale: vec4<f32>, tint: vec4<f32>,
    route: vec4<f32>, effect: vec4<f32>, pivot: vec4<f32>, axis: vec4<f32>,
}
@group(0) @binding(1) var<uniform> environment: Environment;
@group(0) @binding(2) var<storage,read> instances: array<Instance>;
@group(0) @binding(3) var<storage,read> routes: array<vec4<f32>>;
fn yaw_rotation(p:vec3<f32>,a:f32)->vec3<f32> {
    return vec3(cos(a)*p.x+sin(a)*p.z,p.y,-sin(a)*p.x+cos(a)*p.z);
}
fn axis_rotation(p:vec3<f32>,axis:vec3<f32>,a:f32)->vec3<f32> {
    return p*cos(a)+cross(axis,p)*sin(a)+axis*dot(axis,p)*(1.0-cos(a));
}
struct Moved { p:vec3<f32>, n:vec3<f32>, color:vec4<f32>, reveal:f32 }
fn move_instance(position:vec3<f32>,normal:vec3<f32>,index:u32)->Moved {
    let instance=instances[index];let t=environment.clock_wind.x;
    var p=position;var n=normal;var color=instance.tint;var reveal=1.0;
    let amplitude=instance.effect.y;let frequency=instance.effect.z;let phase=instance.effect.w;
    if instance.effect.x == 1.0 {
        let h=max(p.y-instance.pivot.y,0.0);
        let bend=environment.clock_wind.yz*environment.clock_wind.w*amplitude*sin(t*frequency+phase);
        p+=vec3(bend.x*h*h,0.0,bend.y*h*h);
        n=vec3(n.x,n.y-2.0*h*(bend.x*n.x+bend.y*n.z),n.z);
    } else if instance.effect.x == 2.0 {
        let a=p.x*1.7+p.z*0.9+t*frequency+phase;
        let b=-p.x*0.9+p.z*1.7-t*frequency*0.7+phase;
        let dx=amplitude*(1.7*cos(a)-0.45*cos(b));
        let dz=amplitude*(0.9*cos(a)+0.85*cos(b));
        p.y+=amplitude*(sin(a)+0.5*sin(b));
        n=vec3(n.x-dx*n.y,n.y,n.z-dz*n.y);
        color=vec4(color.rgb*(0.95+0.12*cos(a)*cos(b)),color.a);
    } else if instance.effect.x == 3.0 {
        let angle=amplitude*sin(t*frequency+phase);
        p=axis_rotation(p-instance.pivot.xyz,instance.axis.xyz,angle)+instance.pivot.xyz;
        n=axis_rotation(n,instance.axis.xyz,angle);
    }
    if instance.effect.x == 4.0 || instance.effect.x == 5.0 {
        // Smooth floor-by-floor assembly; the completed frame remains at full height.
        let progress=max(0.0,t*frequency+phase);
        var floors=min(instance.pivot.y,floor(progress)+smoothstep(0.15,0.85,fract(progress)));
        if instance.effect.x == 5.0 {
            let completion=max(0.0,(ceil(instance.pivot.y)-phase)/frequency);
            let remove=max(0.0,(t-completion-instance.pivot.x)*instance.axis.x);
            let removed=floor(remove)+smoothstep(0.15,0.85,fract(remove));
            floors=min(floors,max(0.0,instance.pivot.y-removed));
        }
        reveal=floors*amplitude-p.y;
    }
    var offset=instance.position_yaw.xyz;var yaw=instance.position_yaw.w;
    if instance.route.y>=4.0 {
        let start=u32(instance.route.x);let count=u32(instance.route.y);
        let length=routes[start+count-1u].w;
        let distance=fract((instance.route.z+t*instance.route.w+instance.scale.w*(sin(t+instance.axis.w)-sin(instance.axis.w)))/length)*length;
        for(var j=1u;j<count;j++) {
            let a=routes[start+j-1u];let b=routes[start+j];
            if distance<=b.w {
                offset+=mix(a.xyz,b.xyz,(distance-a.w)/(b.w-a.w));
                let tangent=normalize(b.xyz-a.xyz);offset+=vec3(tangent.z,0.0,-tangent.x)*instance.pivot.w;yaw+=atan2(tangent.x,tangent.z);break;
            }
        }
    }
    var out:Moved;
    out.p=yaw_rotation(p*instance.scale.xyz,yaw)+offset;
    out.n=yaw_rotation(n/instance.scale.xyz,yaw);
    out.color=color;out.reveal=reveal;return out;
}
