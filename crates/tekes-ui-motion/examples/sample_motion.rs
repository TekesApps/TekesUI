//! A reproducible trajectory with a mid-flight retarget, without a GUI.
use std::time::Duration;
use tekes_ui_motion::{Animated, presets};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut x = Animated::new(0.0)?;
    x.animate_to(100.0, presets::selection(), Duration::ZERO)?;
    println!("seconds,target,value,velocity,generation,needs_frame");
    for tick in 0..=240 {
        let now = Duration::from_secs_f64(f64::from(tick) / 120.0);
        let frame = if tick == 18 {
            x.animate_to(25.0, presets::selection(), now)?
        } else {
            x.advance(now)?
        };
        println!(
            "{:.6},{:.6},{:.6},{:.6},{},{}",
            now.as_secs_f64(),
            x.target(),
            frame.sample.value,
            frame.sample.velocity,
            frame.generation,
            frame.needs_frame
        );
    }
    Ok(())
}
