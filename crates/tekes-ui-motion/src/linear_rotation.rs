//! Host-clocked constant angular velocity, in turns and turns/second.
//! Speed edits and reversal hold the current angle; they change velocity
//! immediately. This motor deliberately has no acceleration or easing.
use crate::{MotionError, Sample};
use std::time::Duration;
#[derive(Clone, Debug)]
pub struct LinearRotation {
    now: Duration,
    turns: f64,
    speed: f64,
    active: bool,
}
impl LinearRotation {
    pub fn new(speed: f64) -> Result<Self, MotionError> {
        if !speed.is_finite() {
            return Err(MotionError::NonFiniteValue);
        }
        Ok(Self {
            now: Duration::ZERO,
            turns: 0.0,
            speed,
            active: false,
        })
    }
    pub fn sample(&self) -> Sample<f64> {
        Sample {
            value: self.turns,
            velocity: if self.active { self.speed } else { 0.0 },
        }
    }
    pub fn is_active(&self) -> bool {
        self.active && self.speed != 0.0
    }
    pub fn advance(&mut self, now: Duration) -> Result<Sample<f64>, MotionError> {
        if now < self.now {
            return Err(MotionError::TimeWentBackwards);
        }
        let turns = if self.active {
            self.turns + self.speed * (now - self.now).as_secs_f64()
        } else {
            self.turns
        };
        if !turns.is_finite() {
            return Err(MotionError::NumericalOverflow);
        }
        self.turns = turns;
        self.now = now;
        Ok(self.sample())
    }
    pub fn set_speed(&mut self, speed: f64, now: Duration) -> Result<(), MotionError> {
        if !speed.is_finite() {
            return Err(MotionError::NonFiniteValue);
        }
        self.advance(now)?;
        self.speed = speed;
        Ok(())
    }
    pub fn pause(&mut self, now: Duration) -> Result<(), MotionError> {
        self.advance(now)?;
        self.active = false;
        Ok(())
    }
    pub fn resume(&mut self, now: Duration) -> Result<(), MotionError> {
        self.advance(now)?;
        self.active = true;
        Ok(())
    }
}
