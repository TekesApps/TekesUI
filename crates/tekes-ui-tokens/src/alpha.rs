//! Opacity tokens generated from design/tokens.json.

pub use crate::generated::{Alpha, TEXT_SELECTION_INCREASED};
#[cfg(test)]
use crate::theme::Contrast;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_values() {
        assert_eq!(Alpha::Faint.value(), 0.07);
        assert_eq!(Alpha::Soft.value(), 0.14);
        assert_eq!(Alpha::Wash.value(), 0.45);
    }

    #[test]
    fn increased_contrast_raises_faint_to_soft() {
        assert_eq!(Alpha::Faint.resolve(Contrast::Standard), 0.07);
        assert_eq!(Alpha::Faint.resolve(Contrast::Increased), 0.14);
        assert_eq!(Alpha::Soft.resolve(Contrast::Increased), 0.14);
        assert_eq!(Alpha::Wash.resolve(Contrast::Increased), 0.45);
    }
}
