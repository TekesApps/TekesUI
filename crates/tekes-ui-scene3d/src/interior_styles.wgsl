// A bounded visual style bank, shared across every opt-in window/material.
struct RoomStyle { pose:vec4<f32>, state:vec4<f32>, fabric:vec4<f32> }
fn room_hash(p:vec3<f32>)->f32 {return fract(sin(dot(p,vec3(127.1,311.7,74.7)))*43758.5453);}
fn evaluate_room_style(index:u32,t:f32,night_t:f32,night_mix:f32)->RoomStyle {
    let seed=room_hash(vec3(f32(index),43.0,17.0));
    let kind=index%4u;
    let period=42.0+seed*53.0;let clock=t/period+seed*13.0;
    let epoch=floor(clock);
    let awake=1.0-smoothstep(50.0,260.0,night_t);
    let bedtime=smoothstep(-0.015,0.015,0.065+0.82*awake-room_hash(vec3(f32(index),53.0,0.0)));
    let current=step(0.20,room_hash(vec3(f32(index),epoch*19.0,11.0)))*bedtime;
    let previous=step(0.20,room_hash(vec3(f32(index),(epoch-1.0)*19.0,11.0)))*bedtime;
    let lit=mix(previous,current,smoothstep(0.0,1.8,fract(clock)*period));
    let initial=0.15+room_hash(vec3(f32(index),29.0,0.0))*0.50;
    let pull_start=20.0+room_hash(vec3(f32(index),97.0,0.0))*180.0;
    let closure=mix(initial,0.94,smoothstep(pull_start,pull_start+9.0,night_t)*night_mix);
    let cycle=fract(t/(28.0+seed*25.0)+seed*7.0);
    let seated=select(0.0,smoothstep(0.32,0.40,cycle)*(1.0-smoothstep(0.89,0.98,cycle)),kind==1u);
    let walking=1.0-seated;
    let x=mix(0.50+0.23*sin(t*(0.35+seed*0.15)+seed*30.0),0.72,seated);
    let swing=sin(t*(1.8+seed))*0.065*walking;
    let reading=smoothstep(0.48,0.56,cycle)*seated;
    let curtains=select(0.0,1.0,kind>=2u || seed<0.25);
    let actor=select(0.0,1.0,kind<2u && seed<0.86);
    let palette=f32((index/4u)%4u)/3.0;
    var style:RoomStyle;
    style.pose=vec4(x,mix(0.47,0.63,seated),swing,reading);
    // Closure, curtain availability, actor availability, shared lamp activity.
    style.state=vec4(closure,curtains,actor,lit);
    style.fabric=vec4(mix(vec3(0.34,0.26,0.17),vec3(0.31,0.33,0.32),palette),f32((index/4u)%2u));
    return style;
}
