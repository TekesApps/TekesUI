//! 窗口初始位置的选择：在哪块屏幕、放在屏幕的什么位置。
//!
//! 这个模块是纯逻辑，不含 AppKit；屏幕列表由平台代码填好 [`ScreenInfo`] 传进来，
//! 所以规则可以直接单测。
//!
//! 坐标是 AppKit 的全局屏幕坐标：原点在主屏左下角，y 向上。这是引擎坐标约定
//! （左上角为原点）的唯一例外，因为它描述的是整个桌面，不是某个视图。

use crate::event::{Point, Size};

/// 矩形，AppKit 全局屏幕坐标，原点是左下角。
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub origin: Point,
    pub size: Size,
}

/// 一块屏幕的信息。
#[derive(Clone, Debug, PartialEq)]
pub struct ScreenInfo {
    pub name: String,
    /// `CGDisplayIsBuiltin`。
    pub builtin: bool,
    /// `backingScaleFactor`。
    pub scale: f64,
    /// `maximumFramesPerSecond`，ProMotion 屏为 120。
    pub max_fps: u32,
    pub frame: Rect,
    /// 扣掉菜单栏和 Dock 之后的可用区域。
    pub visible_frame: Rect,
}

/// 选哪一块屏幕。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScreenChoice {
    /// 内建显示器；没有内建屏时退回主屏。
    Builtin,
    /// 第一块外接显示器；没有外接屏时退回内建屏。
    External,
}

/// 窗口放哪里。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowPlacement {
    /// 交给 AppKit 的 `center()`，在默认屏幕上居中（偏上）。
    Centered,
    /// 按开发规则自动选：
    /// 1. 有外接显示器且不要求 120Hz：放外接显示器，居中；
    /// 2. 要求 120Hz（ProMotion，只有内建屏有）：放内建屏；
    /// 3. 落在内建屏时，放在右半边，不盖住左边的工作窗口；
    ///    `size_check` 为 true（缩放、窗口尺寸类的检查）时免除右半边规则，居中。
    Auto { need_120hz: bool, size_check: bool },
    /// 指定屏幕的右半边。
    RightHalf(ScreenChoice),
    /// 指定屏幕居中。
    Screen(ScreenChoice),
}

/// 在屏幕上的布局方式。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layout {
    /// AppKit 的 `center()`。
    AppKitCenter,
    /// 在屏幕可用区域内居中。
    Center,
    /// 在屏幕可用区域的右半边居中；窗口比半边宽时靠右对齐。
    RightHalf,
}

/// 选择结果。
#[derive(Clone, Debug, PartialEq)]
pub struct PlacementReport {
    /// 选中的屏幕在 `NSScreen.screens` 里的下标。
    pub screen_index: usize,
    pub screen: ScreenInfo,
    pub layout: Layout,
    /// 为什么这样选，便于日志。
    pub reason: String,
    /// 选择时看到的全部屏幕。
    pub all_screens: Vec<ScreenInfo>,
}

/// 环境变量名。取值：`auto`、`external`、`builtin`、`builtin-right-half`、`centered`。
pub const PLACEMENT_ENV: &str = "TEKESUI_WINDOW_PLACEMENT";

/// 解析环境变量值。`auto` 保留配置里 `Auto` 的两个标志（配置不是 `Auto` 时取 false）。
/// 不认识的值返回 `None`。
pub fn parse_override(value: &str, configured: WindowPlacement) -> Option<WindowPlacement> {
    Some(match value.trim() {
        "auto" => match configured {
            WindowPlacement::Auto { .. } => configured,
            _ => WindowPlacement::Auto {
                need_120hz: false,
                size_check: false,
            },
        },
        "external" => WindowPlacement::Screen(ScreenChoice::External),
        "builtin" => WindowPlacement::Screen(ScreenChoice::Builtin),
        "builtin-right-half" => WindowPlacement::RightHalf(ScreenChoice::Builtin),
        "centered" => WindowPlacement::Centered,
        _ => return None,
    })
}

fn pick(screens: &[ScreenInfo], choice: ScreenChoice) -> usize {
    let builtin = screens.iter().position(|s| s.builtin);
    let external = screens.iter().position(|s| !s.builtin);
    match choice {
        ScreenChoice::Builtin => builtin.unwrap_or(0),
        ScreenChoice::External => external.or(builtin).unwrap_or(0),
    }
}

/// 按规则选屏幕与布局。`screens` 为空时返回 `None`（用 AppKit 默认行为）。
pub fn resolve(placement: WindowPlacement, screens: &[ScreenInfo]) -> Option<PlacementReport> {
    if screens.is_empty() {
        return None;
    }
    let (index, layout, reason) = match placement {
        WindowPlacement::Centered => (0, Layout::AppKitCenter, "Centered：AppKit 默认".to_string()),
        WindowPlacement::Screen(c) => (pick(screens, c), Layout::Center, format!("Screen({c:?})")),
        WindowPlacement::RightHalf(c) => (
            pick(screens, c),
            Layout::RightHalf,
            format!("RightHalf({c:?})"),
        ),
        WindowPlacement::Auto {
            need_120hz,
            size_check,
        } => {
            let has_external = screens.iter().any(|s| !s.builtin);
            if has_external && !need_120hz {
                (
                    pick(screens, ScreenChoice::External),
                    Layout::Center,
                    "Auto：有外接显示器且不要求 120Hz，放外接屏".to_string(),
                )
            } else {
                let why = if need_120hz {
                    "要求 120Hz，放内建屏"
                } else {
                    "没有外接显示器，放内建屏"
                };
                let layout = if size_check {
                    Layout::Center
                } else {
                    Layout::RightHalf
                };
                let extra = if size_check {
                    "；尺寸类检查免除右半边规则，居中"
                } else {
                    "，右半边"
                };
                (
                    pick(screens, ScreenChoice::Builtin),
                    layout,
                    format!("Auto：{why}{extra}"),
                )
            }
        }
    };
    Some(PlacementReport {
        screen_index: index,
        screen: screens[index].clone(),
        layout,
        reason,
        all_screens: screens.to_vec(),
    })
}

/// 窗口 frame 左下角应该放的位置。`window` 是整个窗口（含标题栏）的尺寸。
/// `Layout::AppKitCenter` 不由这里处理，返回 `None`。
pub fn origin_for(layout: Layout, window: Size, visible: Rect) -> Option<Point> {
    let (area_x, area_w) = match layout {
        Layout::AppKitCenter => return None,
        Layout::Center => (visible.origin.x, visible.size.width),
        Layout::RightHalf => (
            visible.origin.x + visible.size.width / 2.0,
            visible.size.width / 2.0,
        ),
    };
    let mut x = area_x + (area_w - window.width) / 2.0;
    // 比区域宽：靠右对齐可用区域（右半边）或居中（整屏），都不让右边缘出屏。
    let right = visible.origin.x + visible.size.width;
    if x + window.width > right {
        x = right - window.width;
    }
    if x < visible.origin.x {
        x = visible.origin.x;
    }
    let mut y = visible.origin.y + (visible.size.height - window.height) / 2.0;
    let top = visible.origin.y + visible.size.height;
    if y < visible.origin.y {
        y = visible.origin.y;
    }
    // 最后处理顶部：窗口比可用区域高时保证标题栏可见（底部出屏）。
    if y + window.height > top {
        y = top - window.height;
    }
    Some(Point { x, y })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: f64, y: f64, w: f64, h: f64) -> Rect {
        Rect {
            origin: Point { x, y },
            size: Size {
                width: w,
                height: h,
            },
        }
    }

    fn builtin() -> ScreenInfo {
        ScreenInfo {
            name: "Built-in".into(),
            builtin: true,
            scale: 2.0,
            max_fps: 120,
            frame: rect(0.0, 0.0, 1512.0, 982.0),
            visible_frame: rect(0.0, 0.0, 1512.0, 950.0),
        }
    }

    fn external() -> ScreenInfo {
        ScreenInfo {
            name: "External".into(),
            builtin: false,
            scale: 2.0,
            max_fps: 60,
            frame: rect(1512.0, 0.0, 1920.0, 1080.0),
            visible_frame: rect(1512.0, 0.0, 1920.0, 1050.0),
        }
    }

    const AUTO: WindowPlacement = WindowPlacement::Auto {
        need_120hz: false,
        size_check: false,
    };

    #[test]
    fn auto_prefers_external() {
        // 内建屏在前、外接屏在后，与 NSScreen.screens 的常见顺序一致；反过来也一样。
        for screens in [[builtin(), external()], [external(), builtin()]] {
            let r = resolve(AUTO, &screens).unwrap();
            assert!(!r.screen.builtin);
            assert_eq!(r.layout, Layout::Center);
        }
    }

    #[test]
    fn auto_120hz_goes_builtin_right_half() {
        let r = resolve(
            WindowPlacement::Auto {
                need_120hz: true,
                size_check: false,
            },
            &[external(), builtin()],
        )
        .unwrap();
        assert!(r.screen.builtin);
        assert_eq!(r.screen_index, 1);
        assert_eq!(r.layout, Layout::RightHalf);
    }

    #[test]
    fn auto_without_external_is_right_half_unless_size_check() {
        let r = resolve(AUTO, &[builtin()]).unwrap();
        assert!(r.screen.builtin);
        assert_eq!(r.layout, Layout::RightHalf);
        let r = resolve(
            WindowPlacement::Auto {
                need_120hz: false,
                size_check: true,
            },
            &[builtin()],
        )
        .unwrap();
        assert_eq!(r.layout, Layout::Center);
    }

    #[test]
    fn explicit_choices_and_fallbacks() {
        let both = [builtin(), external()];
        let r = resolve(WindowPlacement::Screen(ScreenChoice::External), &both).unwrap();
        assert_eq!(r.screen_index, 1);
        let r = resolve(WindowPlacement::RightHalf(ScreenChoice::Builtin), &both).unwrap();
        assert_eq!((r.screen_index, r.layout), (0, Layout::RightHalf));
        // 没有外接屏时 External 退回内建屏
        let r = resolve(
            WindowPlacement::Screen(ScreenChoice::External),
            &[builtin()],
        )
        .unwrap();
        assert!(r.screen.builtin);
        assert!(resolve(AUTO, &[]).is_none());
        let r = resolve(WindowPlacement::Centered, &both).unwrap();
        assert_eq!(r.layout, Layout::AppKitCenter);
    }

    #[test]
    fn env_values() {
        let c = WindowPlacement::Centered;
        assert_eq!(
            parse_override("external", c),
            Some(WindowPlacement::Screen(ScreenChoice::External))
        );
        assert_eq!(
            parse_override("builtin", c),
            Some(WindowPlacement::Screen(ScreenChoice::Builtin))
        );
        assert_eq!(
            parse_override("builtin-right-half", c),
            Some(WindowPlacement::RightHalf(ScreenChoice::Builtin))
        );
        assert_eq!(parse_override("auto", c), Some(AUTO));
        let a = WindowPlacement::Auto {
            need_120hz: true,
            size_check: true,
        };
        assert_eq!(parse_override("auto", a), Some(a));
        assert_eq!(parse_override("nope", c), None);
    }

    #[test]
    fn origin_center_and_right_half() {
        let vis = rect(0.0, 0.0, 1512.0, 950.0);
        let win = Size {
            width: 400.0,
            height: 300.0,
        };
        assert_eq!(
            origin_for(Layout::Center, win, vis),
            Some(Point { x: 556.0, y: 325.0 })
        );
        // 右半边 [756, 1512]，宽 756，居中：756 + (756-400)/2 = 934
        assert_eq!(
            origin_for(Layout::RightHalf, win, vis),
            Some(Point { x: 934.0, y: 325.0 })
        );
        assert_eq!(origin_for(Layout::AppKitCenter, win, vis), None);
    }

    #[test]
    fn origin_clamps_big_windows() {
        let vis = rect(1512.0, 0.0, 1920.0, 1050.0);
        // 比右半边（960）还宽：靠右对齐，右边缘不出屏
        let win = Size {
            width: 1200.0,
            height: 2000.0,
        };
        let p = origin_for(Layout::RightHalf, win, vis).unwrap();
        assert_eq!(p.x, 1512.0 + 1920.0 - 1200.0);
        // 比屏幕还高：顶部对齐，保证标题栏可见
        assert_eq!(p.y, 1050.0 - 2000.0);
        // 非零起点屏幕上的居中
        let p = origin_for(
            Layout::Center,
            Size {
                width: 800.0,
                height: 600.0,
            },
            vis,
        )
        .unwrap();
        assert_eq!(
            p,
            Point {
                x: 1512.0 + 560.0,
                y: 225.0
            }
        );
    }
}
