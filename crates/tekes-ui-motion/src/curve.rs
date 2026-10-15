use crate::MotionError;

/// A bounded timing curve. Bezier y controls are intentionally restricted to
/// [0, 1], so opacity/other bounded properties do not overshoot their endpoints.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TimingCurve {
    controls: Option<[f64; 4]>,
}

impl TimingCurve {
    pub const LINEAR: Self = Self { controls: None };

    pub fn cubic_bezier(x1: f64, y1: f64, x2: f64, y2: f64) -> Result<Self, MotionError> {
        let controls = [x1, y1, x2, y2];
        if controls
            .iter()
            .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
        {
            return Err(MotionError::InvalidCurve);
        }
        // x'(1/2)=0 for this sole interior-degenerate timing curve. Only
        // y=x has a finite slope there; other y controls produce a cusp.
        if x1 == 1.0 && x2 == 0.0 && (y1 != 1.0 || y2 != 0.0) {
            return Err(MotionError::InvalidCurve);
        }
        Ok(Self {
            controls: Some(controls),
        })
    }

    /// Returns (progress, derivative with respect to input progress).
    /// For degenerate endpoint tangents the one-sided slope may be infinite.
    pub fn evaluate(self, progress: f64) -> Result<(f64, f64), MotionError> {
        if !progress.is_finite() {
            return Err(MotionError::NonFiniteValue);
        }
        let u = progress.clamp(0.0, 1.0);
        let Some([x1, y1, x2, y2]) = self.controls else {
            return Ok((u, 1.0));
        };
        // Invert x(t) using bisection. This also handles zero dx/dt safely.
        let t = if u == 0.0 || u == 1.0 {
            u
        } else {
            let (mut low, mut high) = (0.0, 1.0);
            for _ in 0..64 {
                let middle = (low + high) * 0.5;
                if cubic(middle, x1, x2) < u {
                    low = middle;
                } else {
                    high = middle;
                }
            }
            (low + high) * 0.5
        };
        let mut dx = derivative(t, x1, x2);
        let mut dy = derivative(t, y1, y2);
        if dx == 0.0 && dy == 0.0 {
            // Endpoint limit using the first non-zero polynomial coefficient.
            let (x, y) = if t == 0.0 {
                ([x1, x2, 1.0], [y1, y2, 1.0])
            } else {
                ([1.0 - x2, 1.0 - x1, 1.0], [1.0 - y2, 1.0 - y1, 1.0])
            };
            for i in 0..3 {
                if x[i] != 0.0 || y[i] != 0.0 {
                    dx = x[i];
                    dy = y[i];
                    break;
                }
            }
        }
        Ok((cubic(t, y1, y2), dy / dx))
    }
}

fn cubic(t: f64, a: f64, b: f64) -> f64 {
    let s = 1.0 - t;
    3.0 * s * s * t * a + 3.0 * s * t * t * b + t * t * t
}

fn derivative(t: f64, a: f64, b: f64) -> f64 {
    3.0 * ((1.0 - t).powi(2) * a + 2.0 * (1.0 - t) * t * (b - a) + t * t * (1.0 - b))
}
