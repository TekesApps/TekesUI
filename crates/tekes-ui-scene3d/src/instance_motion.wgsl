struct Environment { clock_wind: vec4<f32>, ambient_emission: vec4<f32>, direction: vec4<f32>, directional: vec4<f32>, clocks:vec4<f32> }
struct Instance {
    position_yaw: vec4<f32>, scale: vec4<f32>, tint: vec4<f32>,
    route: vec4<f32>, effect: vec4<f32>, pivot: vec4<f32>, axis: vec4<f32>, removal: vec4<f32>, timeline:vec4<f32>, detail:vec4<f32>,
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
// Height and analytic surface derivatives, shared by water, boats and trails.
fn wave_field(p:vec2<f32>,t:f32,amplitude:f32,frequency:f32,phase:f32)->vec3<f32> {
    let a=p.x*1.7+p.y*0.9+t*frequency+phase;
    let b=-p.x*0.9+p.y*1.7-t*frequency*0.7+phase;
    return amplitude*vec3(sin(a)+0.5*sin(b),1.7*cos(a)-0.45*cos(b),0.9*cos(a)+0.85*cos(b));
}
fn instance_time(instance:Instance)->f32 {
    if instance.timeline.x>0.0 {return environment.clocks[u32(instance.timeline.x)-1u];}
    return environment.clock_wind.x;
}
fn dimming_curve(curve:vec4<f32>)->f32 {
    if curve.x<=0.0 {return 1.0;}
    return mix(1.0,curve.w,smoothstep(curve.y,curve.z,environment.clocks[u32(curve.x)-1u]));
}
struct Moved { p:vec3<f32>, n:vec3<f32>, color:vec4<f32>, reveal:f32, visibility:f32 }
fn move_value(position:vec3<f32>,normal:vec3<f32>,instance:Instance)->Moved {
    let t=instance_time(instance);
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
        let wave=wave_field(p.xz,t,amplitude,frequency,phase);
        p.y+=wave.x;
        n=vec3(n.x-wave.y*n.y,n.y,n.z-wave.z*n.y);
        let crest=pow(max(0.0,0.67*sin(a)+0.33*sin(b)),6.0);
        color=vec4(color.rgb*(0.78+0.18*cos(a)*cos(b)+0.75*crest),color.a);
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
    if instance.removal.w>0.0 {
        if t>=instance.removal.x+instance.removal.y { reveal=-1.0; }
        else if t>instance.removal.x {
            let remaining=1.0-smoothstep(instance.removal.x,instance.removal.x+instance.removal.y,t);
            reveal=min(reveal,instance.removal.z*remaining-p.y);
        }
    }
    var offset=instance.position_yaw.xyz;var yaw=instance.position_yaw.w;
    if instance.route.y>=4.0 {
        let start=u32(instance.route.x);let count=u32(instance.route.y);
        let length=routes[start+count-1u].w;
        // A trail vertex follows an earlier position on the same route.
        let lag=select(0.0,p.z*instance.scale.z,instance.effect.x==7.0);
        let distance=fract((lag+instance.route.z+t*instance.route.w+instance.scale.w*(sin(t+instance.axis.w)-sin(instance.axis.w)))/length)*length;
        for(var j=1u;j<count;j++) {
            let a=routes[start+j-1u];let b=routes[start+j];
            if distance<=b.w {
                offset+=mix(a.xyz,b.xyz,(distance-a.w)/(b.w-a.w));
                let tangent=normalize(b.xyz-a.xyz);offset+=vec3(tangent.z,0.0,-tangent.x)*instance.pivot.w;yaw+=atan2(tangent.x,tangent.z);break;
            }
        }
    }
    if instance.effect.x==7.0 { p.z=0.0; }
    var out:Moved;
    out.p=yaw_rotation(p*instance.scale.xyz,yaw)+offset;
    out.n=yaw_rotation(n/instance.scale.xyz,yaw);
    if instance.effect.x==6.0 {
        let wave=wave_field(offset.xz,t,amplitude,frequency,phase);
        let tilt=vec3(-wave.z,0.0,wave.y);
        let slope=length(tilt);
        if slope>0.000001 {
            out.p=axis_rotation(out.p-offset,tilt/slope,atan(slope))+offset;
            out.n=axis_rotation(out.n,tilt/slope,atan(slope));
        }
        out.p.y+=wave.x;
    } else if instance.effect.x==7.0 {
        let wave=wave_field(out.p.xz,t,amplitude,frequency,phase);
        out.p.y+=wave.x;
        out.n=vec3(out.n.x-wave.y*out.n.y,out.n.y,out.n.z-wave.z*out.n.y);
    }
    out.visibility=1.0;
    if instance.timeline.y>0.0 {
        out.visibility=1.0-smoothstep(instance.timeline.z,instance.timeline.z+instance.timeline.w,environment.clocks[u32(instance.timeline.y)-1u]);
    }
    out.visibility*=instance.detail.x;
    out.color=color;out.reveal=reveal;return out;
}

fn move_instance(position:vec3<f32>,normal:vec3<f32>,index:u32)->Moved {
    return move_value(position,normal,instances[index]);
}
