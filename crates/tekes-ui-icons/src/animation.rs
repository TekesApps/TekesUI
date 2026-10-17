//! Animation annotations reference the approved SVG's original element and
//! contour order. Geometry stays in that SVG; there is no second hand-drawn copy.
use crate::Icon;
#[derive(Clone, Copy, Debug)]
pub struct PathRef {
    pub element: usize,
    pub contour: Option<usize>,
}
#[derive(Clone, Copy, Debug)]
pub struct Layer {
    pub id: &'static str,
    /// In the original 24pt viewBox, not the raster bounds.
    pub anchor: [f32; 2],
    pub paths: &'static [PathRef],
    /// Matching semantic keys may morph only after topology validation.
    pub shared_key: Option<&'static str>,
}
const fn path(element: usize) -> PathRef {
    PathRef {
        element,
        contour: None,
    }
}
const fn part(element: usize, contour: usize) -> PathRef {
    PathRef {
        element,
        contour: Some(contour),
    }
}
impl Icon {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Settings => "设置",
            Self::NewGoal => "新建目标",
            Self::GoalCompleted => "目标完成",
            Self::Memory => "记忆",
            Self::Think => "思考",
            Self::Router => "路由",
            Self::Provider => "服务提供方",
            Self::Appearance => "外观",
        }
    }
    /// Array order is the explicit draw/choreography order; path references keep
    /// compound SVG subpaths independently addressable without changing artwork.
    pub const fn animation_layers(self) -> &'static [Layer] {
        match self {
            Self::Settings => {
                const {
                    &[
                        Layer {
                            id: "housing",
                            anchor: [12.0, 12.0],
                            paths: &[path(0)],
                            shared_key: None,
                        },
                        Layer {
                            id: "hub",
                            anchor: [12.0, 12.0],
                            paths: &[path(1)],
                            shared_key: None,
                        },
                    ]
                }
            }
            Self::NewGoal => {
                const {
                    &[
                        Layer {
                            id: "goal-ring",
                            anchor: [10.0, 11.0],
                            paths: &[path(0)],
                            shared_key: Some("goal-ring"),
                        },
                        Layer {
                            id: "target-core",
                            anchor: [10.0, 11.0],
                            paths: &[path(1)],
                            shared_key: None,
                        },
                        Layer {
                            id: "add-marker",
                            anchor: [18.0, 17.5],
                            paths: &[path(2)],
                            shared_key: None,
                        },
                    ]
                }
            }
            Self::GoalCompleted => {
                const {
                    &[
                        Layer {
                            id: "goal-ring",
                            anchor: [12.0, 12.0],
                            paths: &[path(0)],
                            shared_key: Some("goal-ring"),
                        },
                        Layer {
                            id: "status-check",
                            anchor: [12.0, 12.0],
                            paths: &[path(1)],
                            shared_key: None,
                        },
                    ]
                }
            }
            Self::Memory => {
                const {
                    &[
                        Layer {
                            id: "package",
                            anchor: [12.0, 12.0],
                            paths: &[path(0)],
                            shared_key: None,
                        },
                        Layer {
                            id: "die",
                            anchor: [12.0, 12.0],
                            paths: &[path(1)],
                            shared_key: None,
                        },
                        Layer {
                            id: "pins",
                            anchor: [12.0, 12.0],
                            paths: &[path(2)],
                            shared_key: None,
                        },
                    ]
                }
            }
            Self::Think => {
                const {
                    &[
                        Layer {
                            id: "cloud",
                            anchor: [12.0, 10.0],
                            paths: &[path(0)],
                            shared_key: None,
                        },
                        Layer {
                            id: "thought-near",
                            anchor: [8.0, 19.0],
                            paths: &[path(1)],
                            shared_key: None,
                        },
                        Layer {
                            id: "thought-far",
                            anchor: [5.0, 21.0],
                            paths: &[path(2)],
                            shared_key: None,
                        },
                    ]
                }
            }
            Self::Router => {
                const {
                    &[
                        Layer {
                            id: "trunk",
                            anchor: [9.0, 12.0],
                            paths: &[part(0, 0)],
                            shared_key: None,
                        },
                        Layer {
                            id: "branch-up",
                            anchor: [9.0, 12.0],
                            paths: &[part(0, 1)],
                            shared_key: None,
                        },
                        Layer {
                            id: "branch-down",
                            anchor: [9.0, 12.0],
                            paths: &[part(0, 2)],
                            shared_key: None,
                        },
                        Layer {
                            id: "tip-up",
                            anchor: [21.0, 6.0],
                            paths: &[part(0, 3)],
                            shared_key: None,
                        },
                        Layer {
                            id: "tip-down",
                            anchor: [21.0, 18.0],
                            paths: &[part(0, 4)],
                            shared_key: None,
                        },
                    ]
                }
            }
            Self::Provider => {
                const {
                    &[
                        Layer {
                            id: "server-upper",
                            anchor: [12.0, 7.0],
                            paths: &[path(0), path(2), part(4, 0)],
                            shared_key: None,
                        },
                        Layer {
                            id: "server-lower",
                            anchor: [12.0, 17.0],
                            paths: &[path(1), path(3), part(4, 1)],
                            shared_key: None,
                        },
                    ]
                }
            }
            Self::Appearance => {
                const {
                    &[
                        Layer {
                            id: "outline",
                            anchor: [12.0, 12.0],
                            paths: &[path(0)],
                            shared_key: None,
                        },
                        Layer {
                            id: "shade",
                            anchor: [12.0, 12.0],
                            paths: &[path(1)],
                            shared_key: None,
                        },
                    ]
                }
            }
        }
    }
    pub const fn animation_partner(self) -> Self {
        match self {
            Self::NewGoal => Self::GoalCompleted,
            Self::GoalCompleted => Self::NewGoal,
            Self::Settings => Self::Appearance,
            Self::Appearance => Self::Settings,
            Self::Memory => Self::Think,
            Self::Think => Self::Memory,
            Self::Router => Self::Provider,
            Self::Provider => Self::Router,
        }
    }
}
