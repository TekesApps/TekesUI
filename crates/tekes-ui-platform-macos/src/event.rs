//! 平台层投递给引擎的规整事件，以及事件里用到的纯数据类型。
//!
//! 这个模块不含任何 AppKit 类型，引擎只依赖这里的定义。
//! 坐标约定：视图坐标，单位是点（pt），原点在左上角，y 向下，与引擎一致。

use crate::menu::CommandId;

/// 点坐标，单位 pt，原点在视图左上角。
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

/// 尺寸，单位 pt。
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Size {
    pub width: f64,
    pub height: f64,
}

/// 二维增量，单位见使用处的说明。
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec2 {
    pub x: f64,
    pub y: f64,
}

impl Size {
    /// 按缩放因子换算出 drawable 的像素尺寸。四舍五入，每一维至少为 1，
    /// 因为 CAMetalLayer 不接受 0 宽或 0 高的 drawable。
    pub fn to_drawable(self, scale: f64) -> (u32, u32) {
        fn px(v: f64) -> u32 {
            // 负数与 NaN 在 `as u32` 时饱和为 0，再抬到 1。
            ((v.round()) as u32).max(1)
        }
        (px(self.width * scale), px(self.height * scale))
    }
}

/// 修饰键状态。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Modifiers {
    pub shift: bool,
    pub control: bool,
    /// Option（⌥）。
    pub alt: bool,
    /// Command（⌘）。
    pub command: bool,
    pub caps_lock: bool,
    /// fn / Globe 键。
    pub function: bool,
}

// NSEventModifierFlags 的位，值是 AppKit 的 ABI 常量。
// 这里写成常量而不是引用 objc2-app-kit，让转换函数保持纯函数、可跨平台单测；
// 平台相关的单测会对照 objc2-app-kit 的常量，防止抄错。
pub(crate) const FLAG_CAPS_LOCK: usize = 1 << 16;
pub(crate) const FLAG_SHIFT: usize = 1 << 17;
pub(crate) const FLAG_CONTROL: usize = 1 << 18;
pub(crate) const FLAG_OPTION: usize = 1 << 19;
pub(crate) const FLAG_COMMAND: usize = 1 << 20;
pub(crate) const FLAG_FUNCTION: usize = 1 << 23;

impl Modifiers {
    pub const NONE: Modifiers = Modifiers {
        shift: false,
        control: false,
        alt: false,
        command: false,
        caps_lock: false,
        function: false,
    };

    pub const COMMAND: Modifiers = Modifiers {
        command: true,
        ..Modifiers::NONE
    };

    /// 从 `NSEventModifierFlags` 的原始位转换。未知位（数字小键盘、help 等）忽略。
    pub fn from_ns_flags(flags: usize) -> Modifiers {
        Modifiers {
            shift: flags & FLAG_SHIFT != 0,
            control: flags & FLAG_CONTROL != 0,
            alt: flags & FLAG_OPTION != 0,
            command: flags & FLAG_COMMAND != 0,
            caps_lock: flags & FLAG_CAPS_LOCK != 0,
            function: flags & FLAG_FUNCTION != 0,
        }
    }

    /// 转回 `NSEventModifierFlags` 的原始位。
    pub fn to_ns_flags(self) -> usize {
        let mut f = 0;
        if self.shift {
            f |= FLAG_SHIFT;
        }
        if self.control {
            f |= FLAG_CONTROL;
        }
        if self.alt {
            f |= FLAG_OPTION;
        }
        if self.command {
            f |= FLAG_COMMAND;
        }
        if self.caps_lock {
            f |= FLAG_CAPS_LOCK;
        }
        if self.function {
            f |= FLAG_FUNCTION;
        }
        f
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    /// 第 4 个及之后的按键，值是 AppKit 的 `buttonNumber`。
    Other(u16),
}

impl MouseButton {
    /// 从 `NSEvent.buttonNumber` 转换：0 左，1 右，2 中。
    pub fn from_button_number(n: isize) -> MouseButton {
        match n {
            0 => MouseButton::Left,
            1 => MouseButton::Right,
            2 => MouseButton::Middle,
            n => MouseButton::Other(n.clamp(0, u16::MAX as isize) as u16),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseEventKind {
    Down {
        button: MouseButton,
        click_count: u32,
    },
    Up {
        button: MouseButton,
        click_count: u32,
    },
    /// 没有按键按下时的移动（来自 NSTrackingArea）。
    Moved,
    /// 按键按下时的移动。
    Dragged {
        button: MouseButton,
    },
    Entered,
    Exited,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MouseEvent {
    pub kind: MouseEventKind,
    /// 视图坐标，单位 pt，左上角为原点。
    pub position: Point,
    pub modifiers: Modifiers,
}

/// `NSEvent.phase` / `momentumPhase` 的原样转录。
///
/// 章程原则 4：惯性滚动使用系统的 momentum phase，不自己模拟。
/// 平台层只转发，不解释；引擎据此识别手势开始、结束和惯性阶段。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScrollPhase {
    /// 没有阶段信息（普通鼠标滚轮，或者该字段不适用）。
    None,
    MayBegin,
    Began,
    Stationary,
    Changed,
    Ended,
    Cancelled,
    /// AppKit 给出了不在上面列表里的值，原样保留。
    Other(usize),
}

impl ScrollPhase {
    pub fn from_ns_phase(bits: usize) -> ScrollPhase {
        match bits {
            0 => ScrollPhase::None,
            0x20 => ScrollPhase::MayBegin,
            0x1 => ScrollPhase::Began,
            0x2 => ScrollPhase::Stationary,
            0x4 => ScrollPhase::Changed,
            0x8 => ScrollPhase::Ended,
            0x10 => ScrollPhase::Cancelled,
            other => ScrollPhase::Other(other),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollEvent {
    /// 滚动增量，取自 `scrollingDeltaX/Y`，符号不改：已经包含系统“自然滚动”设置。
    /// `delta.y > 0` 表示内容应向下移动（向 y 增大的方向，即用户在向上看更早的内容）。
    /// `precise` 为 true（触控板、妙控鼠标）时单位是点；为 false（普通滚轮）时单位是“行”，
    /// 行高由引擎决定。
    pub delta: Vec2,
    /// `hasPreciseScrollingDeltas`。
    pub precise: bool,
    /// 手指阶段。
    pub phase: ScrollPhase,
    /// 惯性阶段。手指离开后系统继续发的事件 `phase` 为 None，`momentum_phase` 非 None。
    pub momentum_phase: ScrollPhase,
    /// `isDirectionInvertedFromDevice`：true 表示用户开启了“自然滚动”。
    /// `delta` 已经反映了这一点，这个字段只供需要知道原始方向的场合使用。
    pub inverted: bool,
    /// 鼠标位置，视图坐标。
    pub position: Point,
    pub modifiers: Modifiers,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyEventKind {
    Down,
    Up,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyEvent {
    pub kind: KeyEventKind,
    /// 硬件键码（`NSEvent.keyCode`），与键盘布局无关。
    pub key_code: u16,
    /// 带修饰键后产生的字符（`characters`）。功能键、方向键给的是 AppKit 的私有区字符。
    pub characters: String,
    /// 忽略修饰键（除 Shift 外）后的字符（`charactersIgnoringModifiers`），用于快捷键匹配。
    pub characters_ignoring_modifiers: String,
    pub modifiers: Modifiers,
    /// 长按产生的重复事件。`Up` 事件永远是 false。
    pub is_repeat: bool,
}

/// 显示链接（`CADisplayLink`）一次回调携带的时间信息，随 [`Event::AnimationFrame`] 投递。
///
/// 所有时间都是秒（`f64`），基准是 `CACurrentMediaTime()`（Mach 绝对时间，休眠时不走），
/// 四个字段在同一时间轴上，可以直接相减。
///
/// 掉帧检测靠 `target_timestamp`：连续两次回调的 `target_timestamp` 之差如果超过
/// 1.5 个 `refresh_interval`，中间至少有一次显示刷新没有得到新帧（见 `tekes_ui_render::stats`）。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AnimationFrame {
    /// `CADisplayLink.timestamp`：上一帧被显示（上一次刷新）的时刻。
    pub timestamp: f64,
    /// `CADisplayLink.targetTimestamp`：这次回调画出的帧预计被显示的时刻（下一次刷新）。
    /// 动画应当按它推进，而不是按回调实际发生的时刻，这样回调抖动不会变成画面抖动。
    pub target_timestamp: f64,
    /// 平台层的回调入口实际被调用的时刻（进入回调后第一件事读取）。
    /// `target_timestamp - callback_time` 是这一帧在上屏截止之前剩下的时间预算。
    pub callback_time: f64,
    /// 显示器当前的刷新间隔，`CADisplayLink.duration`；系统给出 0 或负数时退回
    /// `target_timestamp - timestamp`，两者都不可用则为 0。
    pub refresh_interval: f64,
}

impl AnimationFrame {
    /// 从显示链接的原始读数构造，规整 `refresh_interval`。
    pub fn new(timestamp: f64, target_timestamp: f64, callback_time: f64, duration: f64) -> Self {
        let from_stamps = target_timestamp - timestamp;
        let refresh_interval = if duration.is_finite() && duration > 0.0 {
            duration
        } else if from_stamps.is_finite() && from_stamps > 0.0 {
            from_stamps
        } else {
            0.0
        };
        AnimationFrame {
            timestamp,
            target_timestamp,
            callback_time,
            refresh_interval,
        }
    }

    /// 离上屏截止还剩多少秒；回调已经晚于目标时为负。
    pub fn budget(&self) -> f64 {
        self.target_timestamp - self.callback_time
    }
}

/// 平台层向引擎投递的事件。
///
/// 事件在主线程上、AppKit 回调里同步投递；同一时刻只有一个 `handle` 调用在运行，
/// 回调期间产生的新事件排队，按顺序在当前调用返回后投递（见 docs/platform-boundary.md）。
///
/// # 预留：输入法（F5）
///
/// UTF-16 text services state published by the Rust editor after each update.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextInputState {
    /// Active insertion end, which may precede selection.start after a reverse drag.
    pub caret_index: usize,
    pub text: String,
    pub selection: std::ops::Range<usize>,
    pub marked: Option<std::ops::Range<usize>>,
    pub caret_origin: Point,
    pub caret_size: Size,
    pub read_only: bool,
    pub carets: Vec<(usize, Point, Size)>,
}

#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum Event {
    /// 窗口、视图和 CAMetalLayer 已经创建。此后 `Window::metal_layer` 可用。
    /// 紧接着会收到一次 `SystemAppearanceChanged`、一次 `Resized` 和一次 `Redraw`。
    Created,
    /// 尺寸或缩放因子变化（含初始）。`drawable_size` 已经应用到 CAMetalLayer。
    /// 同步的 `Redraw` 紧随其后。
    Resized {
        /// 视图尺寸，单位 pt。
        size: Size,
        /// `backingScaleFactor`。
        scale: f64,
        /// 像素尺寸，等于 `size * scale` 取整。
        drawable_size: (u32, u32),
    },
    /// 现在应该画一帧并呈现。
    Redraw,
    /// 持续动画模式（[`Window::set_animating`](crate::Window::set_animating)）下，显示链接每次
    /// 刷新的回调。引擎在这里推进动画并画一帧，不需要再调 `request_redraw`。
    /// 只在动画模式开着的时候才会收到；关掉之后不再有回调，空闲时仍是 0 CPU。
    AnimationFrame(AnimationFrame),
    Mouse(MouseEvent),
    Scroll(ScrollEvent),
    Key(KeyEvent),
    TextCommit {
        text: String,
        replacement: Option<std::ops::Range<usize>>,
    },
    TextPreedit {
        text: String,
        clauses: Vec<std::ops::Range<usize>>,
        selection: std::ops::Range<usize>,
        replacement: Option<std::ops::Range<usize>>,
    },
    TextUnmark,
    CaretBlink,
    TextCommand(String),
    /// 修饰键变化（`flagsChanged`）。
    ModifiersChanged(Modifiers),
    /// 窗口成为或不再是 key window。
    Focus(bool),
    /// 菜单项被选中。
    Command(CommandId),
    /// 系统外观或“增强对比度”设置变化（含启动时的初始值）。
    SystemMotionChanged(bool),
    SystemAppearanceChanged {
        dark: bool,
        increase_contrast: bool,
    },
    /// 用户请求关闭窗口（红色按钮、⌘W）。平台层不会自己关窗，引擎决定：
    /// 要关就调用 `Window::close`，要拦截（例如有未保存内容）就不调用。
    CloseRequested,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn animation_frame_interval_is_sanitized() {
        // 正常：用 duration。
        let f = AnimationFrame::new(10.0, 10.008, 10.002, 1.0 / 120.0);
        assert!((f.refresh_interval - 1.0 / 120.0).abs() < 1e-12);
        assert!((f.budget() - 0.006).abs() < 1e-12);
        // duration 为 0：退回 target - timestamp。
        let f = AnimationFrame::new(10.0, 10.016, 10.010, 0.0);
        assert!((f.refresh_interval - 0.016).abs() < 1e-12);
        // 都不可用：0。
        let f = AnimationFrame::new(10.0, 10.0, 10.0, f64::NAN);
        assert_eq!(f.refresh_interval, 0.0);
        // 晚于目标：预算为负。
        let f = AnimationFrame::new(10.0, 10.008, 10.012, 1.0 / 120.0);
        assert!(f.budget() < 0.0);
    }

    #[test]
    fn modifiers_roundtrip() {
        let m = Modifiers {
            shift: true,
            alt: true,
            command: true,
            ..Modifiers::NONE
        };
        assert_eq!(Modifiers::from_ns_flags(m.to_ns_flags()), m);
        assert_eq!(Modifiers::from_ns_flags(0), Modifiers::NONE);
        let all = Modifiers {
            shift: true,
            control: true,
            alt: true,
            command: true,
            caps_lock: true,
            function: true,
        };
        assert_eq!(Modifiers::from_ns_flags(all.to_ns_flags()), all);
    }

    /// 对照 objc2-app-kit 的常量，防止位值抄错。
    #[cfg(target_os = "macos")]
    #[test]
    fn modifier_bits_match_appkit() {
        use objc2_app_kit::{NSEventModifierFlags as F, NSEventPhase as P};
        assert_eq!(FLAG_CAPS_LOCK, F::CapsLock.bits());
        assert_eq!(FLAG_SHIFT, F::Shift.bits());
        assert_eq!(FLAG_CONTROL, F::Control.bits());
        assert_eq!(FLAG_OPTION, F::Option.bits());
        assert_eq!(FLAG_COMMAND, F::Command.bits());
        assert_eq!(FLAG_FUNCTION, F::Function.bits());
        assert_eq!(
            ScrollPhase::from_ns_phase(P::MayBegin.bits()),
            ScrollPhase::MayBegin
        );
        assert_eq!(
            ScrollPhase::from_ns_phase(P::Began.bits()),
            ScrollPhase::Began
        );
        assert_eq!(
            ScrollPhase::from_ns_phase(P::Stationary.bits()),
            ScrollPhase::Stationary
        );
        assert_eq!(
            ScrollPhase::from_ns_phase(P::Changed.bits()),
            ScrollPhase::Changed
        );
        assert_eq!(
            ScrollPhase::from_ns_phase(P::Ended.bits()),
            ScrollPhase::Ended
        );
        assert_eq!(
            ScrollPhase::from_ns_phase(P::Cancelled.bits()),
            ScrollPhase::Cancelled
        );
    }

    #[test]
    fn modifiers_ignore_unknown_bits() {
        // 数字小键盘（1<<21）、help（1<<22）以及低 16 位的设备相关位都不应该泄漏出来。
        let flags = FLAG_COMMAND | (1 << 21) | (1 << 22) | 0x0102;
        assert_eq!(Modifiers::from_ns_flags(flags), Modifiers::COMMAND);
    }

    #[test]
    fn button_numbers() {
        assert_eq!(MouseButton::from_button_number(0), MouseButton::Left);
        assert_eq!(MouseButton::from_button_number(1), MouseButton::Right);
        assert_eq!(MouseButton::from_button_number(2), MouseButton::Middle);
        assert_eq!(MouseButton::from_button_number(4), MouseButton::Other(4));
        assert_eq!(MouseButton::from_button_number(-1), MouseButton::Other(0));
    }

    #[test]
    fn scroll_phase_passthrough() {
        assert_eq!(ScrollPhase::from_ns_phase(0), ScrollPhase::None);
        assert_eq!(ScrollPhase::from_ns_phase(1), ScrollPhase::Began);
        assert_eq!(ScrollPhase::from_ns_phase(4), ScrollPhase::Changed);
        assert_eq!(ScrollPhase::from_ns_phase(8), ScrollPhase::Ended);
        assert_eq!(ScrollPhase::from_ns_phase(0x20), ScrollPhase::MayBegin);
        assert_eq!(ScrollPhase::from_ns_phase(0x40), ScrollPhase::Other(0x40));
    }

    #[test]
    fn drawable_size_rounds_and_clamps() {
        let s = Size {
            width: 800.0,
            height: 600.5,
        };
        assert_eq!(s.to_drawable(2.0), (1600, 1201));
        assert_eq!(s.to_drawable(1.0), (800, 601));
        // 非整数缩放（如 1.5）
        assert_eq!(
            Size {
                width: 101.0,
                height: 33.0
            }
            .to_drawable(1.5),
            (152, 50)
        );
        // 0 尺寸（窗口被压到最小/刚创建）不能产生 0 像素的 drawable
        assert_eq!(Size::default().to_drawable(2.0), (1, 1));
        assert_eq!(
            Size {
                width: f64::NAN,
                height: -5.0
            }
            .to_drawable(2.0),
            (1, 1)
        );
    }
}
