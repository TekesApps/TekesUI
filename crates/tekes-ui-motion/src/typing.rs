//! Time-based text reveal without owning strings, layout, timers or frames.
//! Callers supply complete display units (graphemes or shaped clusters), never
//! raw UTF-8 bytes. A single append/advance can reveal any number of units.
use crate::MotionError;
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TypingMode {
    Arrivals,
    Paced,
}
#[derive(Clone, Copy, Debug)]
pub struct TypingConfig {
    pub mode: TypingMode,
    pub units_per_second: f64,
    pub fade_duration: Duration,
    /// Optional backlog ceiling; None preserves a fully paced scripted reveal.
    pub max_latency: Option<Duration>,
}
impl Default for TypingConfig {
    fn default() -> Self {
        Self {
            mode: TypingMode::Paced,
            units_per_second: 28.0,
            fade_duration: Duration::from_millis(180),
            max_latency: None,
        }
    }
}
impl TypingConfig {
    fn validate(self) -> Result<(), MotionError> {
        if !self.units_per_second.is_finite()
            || !(1.0..=10000.0).contains(&self.units_per_second)
            || self.fade_duration > Duration::from_secs(5)
        {
            return Err(MotionError::InvalidOptions);
        }
        Ok(())
    }
}
#[derive(Clone, Debug)]
pub struct TypingEngine {
    config: TypingConfig,
    due: Vec<Duration>,
    visible: usize,
    settled_prefix: usize,
    clock: Duration,
    last_host: Duration,
    paused: bool,
    reduced: bool,
}
impl TypingEngine {
    pub fn new(config: TypingConfig) -> Result<Self, MotionError> {
        config.validate()?;
        Ok(Self {
            config,
            due: Vec::new(),
            visible: 0,
            settled_prefix: 0,
            clock: Duration::ZERO,
            last_host: Duration::ZERO,
            paused: false,
            reduced: false,
        })
    }
    pub fn available(&self) -> usize {
        self.due.len()
    }
    pub fn visible(&self) -> usize {
        self.visible
    }
    pub fn logical_time(&self) -> Duration {
        self.clock
    }
    pub fn is_paused(&self) -> bool {
        self.paused
    }
    pub fn append(&mut self, units: usize, now: Duration) -> Result<usize, MotionError> {
        if self.due.len().checked_add(units).is_none_or(|n| n > 65536) {
            return Err(MotionError::InvalidOptions);
        }
        self.advance(now)?;
        let interval = Duration::from_secs_f64(1.0 / self.config.units_per_second);
        let mut next = self
            .due
            .last()
            .map(|at| at.saturating_add(interval))
            .unwrap_or(self.clock);
        for _ in 0..units {
            let at =
                if self.reduced || self.config.mode == TypingMode::Arrivals || next <= self.clock {
                    self.clock
                } else {
                    next
                };
            let at = if let Some(limit) = self.config.max_latency {
                at.min(self.clock.saturating_add(limit))
            } else {
                at
            };
            self.due.push(at);
            next = at.saturating_add(interval);
        }
        self.advance(now)
    }
    pub fn advance(&mut self, now: Duration) -> Result<usize, MotionError> {
        if now < self.last_host {
            return Err(MotionError::TimeWentBackwards);
        }
        if !self.paused {
            self.clock = self
                .clock
                .checked_add(now - self.last_host)
                .ok_or(MotionError::InvalidOptions)?;
        }
        self.last_host = now;
        if !self.paused {
            self.visible = self.due.partition_point(|at| *at <= self.clock);
            if self.reduced {
                self.settled_prefix = self.visible;
            }
        }
        Ok(self.visible)
    }
    pub fn opacity(&self, index: usize) -> f64 {
        if index >= self.visible {
            return 0.0;
        }
        if index < self.settled_prefix || self.reduced || self.config.fade_duration.is_zero() {
            return 1.0;
        }
        let t = (self.clock.saturating_sub(self.due[index]).as_secs_f64()
            / self.config.fade_duration.as_secs_f64())
        .clamp(0.0, 1.0);
        // Smoothstep, opacity only: stable text baselines and no layout drift.
        t * t * (3.0 - 2.0 * t)
    }
    pub fn needs_frame(&self) -> bool {
        !self.paused
            && (self.visible < self.due.len()
                || (self.visible > 0 && self.opacity(self.visible - 1) < 1.0))
    }
    pub fn pause(&mut self, now: Duration) -> Result<(), MotionError> {
        self.advance(now)?;
        self.paused = true;
        Ok(())
    }
    pub fn resume(&mut self, now: Duration) -> Result<(), MotionError> {
        self.advance(now)?;
        self.paused = false;
        self.advance(now)?;
        Ok(())
    }
    pub fn set_config(&mut self, config: TypingConfig) -> Result<(), MotionError> {
        config.validate()?;
        self.config = config;
        let interval = Duration::from_secs_f64(1.0 / config.units_per_second);
        for index in self.visible..self.due.len() {
            let offset = if config.mode == TypingMode::Arrivals {
                Duration::ZERO
            } else {
                interval.mul_f64((index - self.visible) as f64)
            };
            let offset = config.max_latency.map_or(offset, |limit| offset.min(limit));
            self.due[index] = self.clock.saturating_add(offset);
        }
        Ok(())
    }
    pub fn set_reduced_motion(&mut self, reduced: bool) -> Result<(), MotionError> {
        self.reduced = reduced;
        if reduced {
            for at in &mut self.due[self.visible..] {
                *at = self.clock;
            }
            self.visible = self.due.len();
            self.settled_prefix = self.visible;
        }
        Ok(())
    }
    pub fn flush(&mut self, now: Duration) -> Result<(), MotionError> {
        self.advance(now)?;
        for at in &mut self.due[self.visible..] {
            *at = self.clock;
        }
        self.visible = self.due.len();
        Ok(())
    }
}
