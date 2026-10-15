//! **临时占位：数值取自 Tekes 现有 Spacing / CornerRadius / Stroke / Motion / ControlSize / Alpha / ColorRole / TextRole，等 Design 交付的 tokens.json 生成后替换。**
//!
//! 长度（pt）与时间（秒）类 token。

/// 间距（pt，2pt 栅格）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Spacing {
    None,
    Micro,
    Tight,
    Snug,
    Regular,
    Relaxed,
    Loose,
    Section,
}

impl Spacing {
    pub const fn value(self) -> f32 {
        match self {
            Spacing::None => 0.0,
            Spacing::Micro => 2.0,
            Spacing::Tight => 4.0,
            Spacing::Snug => 6.0,
            Spacing::Regular => 8.0,
            Spacing::Relaxed => 12.0,
            Spacing::Loose => 16.0,
            Spacing::Section => 24.0,
        }
    }
}

/// 圆角（pt）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CornerRadius {
    /// 硬角。
    Square,
    /// 身份标记。
    AgentMarker,
    /// 小标记；同心圆角推导的下限。
    Badge,
    /// 列表行选中、hover 底。
    Row,
    /// 文本框、chip、搜索框（等于 pillHeight，即胶囊）。
    Control,
    /// 输入卡与其叠放卡片。
    Input,
    /// 卡片、分组面板。
    Panel,
    /// 嵌入窗口列的卡片。
    Column,
    /// 浮层、popover。
    Floating,
}

impl CornerRadius {
    pub const fn value(self) -> f32 {
        match self {
            CornerRadius::Square => 0.0,
            CornerRadius::AgentMarker => 3.0,
            CornerRadius::Badge => 4.0,
            CornerRadius::Row => 6.0,
            CornerRadius::Control => 28.0,
            CornerRadius::Input => 16.0,
            CornerRadius::Panel => 12.0,
            CornerRadius::Column => 18.0,
            CornerRadius::Floating => 14.0,
        }
    }

    /// 同心圆角：子形状圆角 = 外圆角 - 内缩距离，下限 `Badge`。
    pub fn concentric(outer: f32, inset: f32) -> f32 {
        (outer - inset).max(CornerRadius::Badge.value())
    }
}

/// 描边宽度（pt）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Stroke {
    Hairline,
    Rule,
    Emphasis,
}

impl Stroke {
    pub const fn value(self) -> f32 {
        match self {
            Stroke::Hairline => 0.5,
            Stroke::Rule => 1.0,
            Stroke::Emphasis => 2.0,
        }
    }
}

/// 动效时长（秒）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Motion {
    /// 硬切。减弱动态效果时所有档位退化为它，而不是缩短淡入。
    Instant,
    /// 颜色、透明度。
    StateChange,
    /// 位置、尺寸。
    Geometry,
    /// 紧凑列表展开。
    ListExpansion,
    /// 较大面的展开。
    Expansion,
}

impl Motion {
    pub const fn seconds(self) -> f32 {
        match self {
            Motion::Instant => 0.0,
            Motion::StateChange => 0.12,
            Motion::Geometry => 0.18,
            Motion::ListExpansion => 0.3,
            Motion::Expansion => 0.5,
        }
    }

    /// 减弱动态效果时一律硬切。
    pub const fn resolve(self, reduce_motion: bool) -> f32 {
        if reduce_motion { 0.0 } else { self.seconds() }
    }
}

/// 控件尺寸（pt）。名字承载语义，不同 case 允许同值，不要合并。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ControlSize {
    FocusBar,
    Dot,
    AgentMarkerWidth,
    AgentMarkerHeight,
    InlineBadge,
    SelectorTrack,
    InlineAction,
    CompactRow,
    SelectorKnob,
    ActionButton,
    IconButton,
    PillHeight,
    RowHeight,
    TruncationSlot,
    TruncationSlotWide,
}

impl ControlSize {
    pub const fn value(self) -> f32 {
        match self {
            ControlSize::FocusBar => 2.0,
            ControlSize::Dot => 6.0,
            ControlSize::AgentMarkerWidth => 8.0,
            ControlSize::AgentMarkerHeight => 14.0,
            ControlSize::InlineBadge => 16.0,
            ControlSize::SelectorTrack => 20.0,
            ControlSize::InlineAction
            | ControlSize::CompactRow
            | ControlSize::SelectorKnob
            | ControlSize::ActionButton => 24.0,
            ControlSize::IconButton | ControlSize::PillHeight => 28.0,
            ControlSize::RowHeight => 32.0,
            ControlSize::TruncationSlot => 180.0,
            ControlSize::TruncationSlotWide => 240.0,
        }
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
    fn corner_control_equals_pill_height() {
        assert_eq!(
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
