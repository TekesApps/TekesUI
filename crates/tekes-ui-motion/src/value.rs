use std::fmt::Debug;

/// A finite vector with component-wise arithmetic. Value and velocity share
/// coordinates; velocity is expressed in value units per second.
///
/// Implementations must preserve components in `map`/`zip_map`. Scalars and
/// fixed-size vectors are supplied; semantic colors/angles belong to callers.
pub trait MotionValue: Copy + Debug {
    fn map(self, f: impl FnMut(f64) -> f64) -> Self;
    fn zip_map(self, other: Self, f: impl FnMut(f64, f64) -> f64) -> Self;
    fn all(self, predicate: impl FnMut(f64) -> bool) -> bool;

    fn zero(self) -> Self {
        self.map(|_| 0.0)
    }

    fn is_finite(self) -> bool {
        self.all(f64::is_finite)
    }
}

impl MotionValue for f64 {
    fn map(self, mut f: impl FnMut(f64) -> f64) -> Self {
        f(self)
    }

    fn zip_map(self, other: Self, mut f: impl FnMut(f64, f64) -> f64) -> Self {
        f(self, other)
    }

    fn all(self, mut predicate: impl FnMut(f64) -> bool) -> bool {
        predicate(self)
    }
}

impl<const N: usize> MotionValue for [f64; N] {
    fn map(self, f: impl FnMut(f64) -> f64) -> Self {
        self.map(f)
    }

    fn zip_map(self, other: Self, mut f: impl FnMut(f64, f64) -> f64) -> Self {
        std::array::from_fn(|i| f(self[i], other[i]))
    }

    fn all(self, predicate: impl FnMut(f64) -> bool) -> bool {
        self.into_iter().all(predicate)
    }
}
