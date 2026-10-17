//! Logical-point dimensions and motion generated from design/tokens.json.

pub use crate::generated::{ControlSize, CornerRadius, Motion, Spacing, Stroke};

impl CornerRadius {
    /// Child radius for concentric nested geometry, floored by `radius-square`.
    ///
    /// Negative outer radii and insets are treated as zero. Non-finite inputs
    /// return the safe square-radius floor. Non-concentric shapes must not use
    /// this helper.
    pub fn concentric(outer: f32, inset: f32) -> f32 {
        let token_floor = CornerRadius::Square.value();
        let floor = if token_floor.is_finite() {
            token_floor.max(0.0)
        } else {
            0.0
        };
        if !outer.is_finite() || !inset.is_finite() {
            return floor;
        }
        (outer.max(floor) - inset.max(0.0)).max(floor)
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
    fn concentric_uses_square_token_floor_and_clamps_insets() {
        assert_eq!(CornerRadius::concentric(12.0, 4.0), 8.0);
        assert_eq!(CornerRadius::concentric(6.0, 5.0), 1.0);
        assert_eq!(
            CornerRadius::concentric(6.0, 6.0),
            CornerRadius::Square.value()
        );
        assert_eq!(
            CornerRadius::concentric(6.0, 8.0),
            CornerRadius::Square.value()
        );
        assert_eq!(CornerRadius::concentric(6.0, -2.0), 6.0);
        assert_eq!(
            CornerRadius::concentric(-2.0, 0.0),
            CornerRadius::Square.value()
        );
    }

    #[test]
    fn concentric_non_finite_inputs_return_square_token_floor() {
        let floor = CornerRadius::Square.value();
        assert_eq!(CornerRadius::concentric(f32::NAN, 1.0), floor);
        assert_eq!(CornerRadius::concentric(1.0, f32::NAN), floor);
        assert_eq!(CornerRadius::concentric(f32::INFINITY, 0.0), floor);
        assert_eq!(CornerRadius::concentric(1.0, f32::NEG_INFINITY), floor);
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
