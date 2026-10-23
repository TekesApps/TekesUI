//! Platform-neutral public TekesUI icons. `Glyph` exposes the full canonical catalog.
//! The eight-variant `Icon` API retains its approved animated subset for compatibility.
//! SVGs use square 24 pt artboards, currentColor and optical stroke weights.

pub mod animation;
mod catalog;
pub use catalog::Glyph;
mod style;
pub use style::{ARTBOARD_SIZE, REGULAR_STROKE_WIDTH};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Icon {
    Settings,
    NewGoal,
    GoalCompleted,
    Memory,
    Think,
    Router,
    Provider,
    Appearance,
}

impl Icon {
    pub const ALL: [Self; 8] = [
        Self::Settings,
        Self::NewGoal,
        Self::GoalCompleted,
        Self::Memory,
        Self::Think,
        Self::Router,
        Self::Provider,
        Self::Appearance,
    ];

    /// Stable asset name shared across consumers.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Settings => "tekes-settings",
            Self::NewGoal => "tekes-new-goal",
            Self::GoalCompleted => "tekes-goal-completed",
            Self::Memory => "tekes-memory",
            Self::Think => "tekes-think",
            Self::Router => "tekes-router",
            Self::Provider => "tekes-provider",
            Self::Appearance => "tekes-appearance",
        }
    }

    /// The approved default artwork. Rendering and tinting are left to the caller.
    pub const fn svg(self) -> &'static str {
        match self {
            Self::Settings => include_str!("../assets/tekes-settings.svg"),
            Self::NewGoal => include_str!("../assets/tekes-new-goal.svg"),
            Self::GoalCompleted => include_str!("../assets/tekes-goal-completed.svg"),
            Self::Memory => include_str!("../assets/tekes-memory.svg"),
            Self::Think => include_str!("../assets/tekes-think.svg"),
            Self::Router => include_str!("../assets/tekes-router.svg"),
            Self::Provider => include_str!("../assets/tekes-provider.svg"),
            Self::Appearance => include_str!("../assets/tekes-appearance.svg"),
        }
    }

    /// Exact canonical lookup; unknown names remain available for the caller's fallback.
    pub fn named(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|icon| icon.name() == name)
    }
}

#[cfg(test)]
mod tests {
    use super::Icon;

    #[test]
    fn canonical_names_resolve_to_the_approved_artwork() {
        for icon in Icon::ALL {
            assert_eq!(Icon::named(icon.name()), Some(icon));
            assert!(icon.svg().contains("viewBox=\"0 0 24 24\""));
            assert!(icon.svg().contains("currentColor"));
        }
        assert_eq!(Icon::named("tekes-search"), None);
        assert_eq!(Icon::named("tekes-settings-old"), None);
    }
}
