//! Logical-point dimensions and motion generated from design/tokens.json.

pub use crate::generated::{ControlSize, CornerRadius, Motion, Spacing, Stroke};

impl CornerRadius {
    /// Child radius = outer - inset, with the semantic Badge floor.
    pub fn concentric(outer: f32, inset: f32) -> f32 {
        (outer - inset).max(CornerRadius::Badge.value())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spacing_matches_brief() {
        let got = [
            Spacing::None,
            Spacing::Micro,
            Spacing::Tight,
            Spacing::Snug,
            Spacing::Regular,
            Spacing::Relaxed,
            Spacing::Loose,
            Spacing::Section,
        ]
        .map(Spacing::value);
        assert_eq!(got, [0.0, 2.0, 4.0, 6.0, 8.0, 12.0, 16.0, 24.0]);
    }

    #[test]
    fn control_radius_uses_proposal_shape_not_old_pill_alias() {
        assert_eq!(
            CornerRadius::Control.value(),
            crate::proposal::metric("radius", "radius-control").unwrap()
        );
        assert_ne!(
            CornerRadius::Control.value(),
            ControlSize::PillHeight.value()
        );
    }

    #[test]
    fn concentric_has_floor() {
        assert_eq!(CornerRadius::concentric(12.0, 4.0), 8.0);
        assert_eq!(CornerRadius::concentric(6.0, 5.0), 4.0);
    }

    #[test]
    fn reduce_motion_is_instant() {
        assert_eq!(Motion::Expansion.resolve(true), 0.0);
        assert_eq!(Motion::Expansion.resolve(false), 0.5);
    }

    #[test]
    fn equal_control_sizes_stay_distinct() {
        assert_eq!(
            ControlSize::InlineAction.value(),
            ControlSize::SelectorKnob.value()
        );
        assert_ne!(ControlSize::InlineAction, ControlSize::SelectorKnob);
    }
}
