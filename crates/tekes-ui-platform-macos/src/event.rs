//! 平台层投递给引擎的规整事件，以及事件里用到的纯数据类型。
//!
//! 这个模块不含任何 AppKit 类型，引擎只依赖这里的定义。
//! 坐标约定：视图坐标，单位是点（pt），原点在左上角，y 向下，与引擎一致。

use crate::accessibility::{AccessibilityAction, AccessibilityKey};
use crate::menu::CommandId;

/// 点坐标，单位 pt，原点在视图左上角。
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

/// Native pointer shape, independent of AppKit types.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CursorIcon {
    IBeam,
    ResizeHorizontal,
    ResizeVertical,
}

/// A cursor hit region in view coordinates (points, top-left origin).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CursorRegion {
    pub origin: Point,
    pub size: Size,
    pub icon: CursorIcon,
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
    /// Monotonic native event time (seconds since boot).
    pub timestamp: f64,
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
    /// `submission_deadline - callback_time` is the CPU submission budget.
    pub callback_time: f64,
    /// CPU submission/present deadline. CADisplayLink uses the display target;
    /// CAMetalDisplayLink supplies a separate deadline before presentation.
    pub submission_deadline: f64,
    /// Actual handler start after any reentrant event queue delay.
    pub dispatch_time: f64,
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
            submission_deadline: target_timestamp,
            dispatch_time: callback_time,
            refresh_interval,
        }
    }

    pub fn with_submission_deadline(mut self, deadline: f64) -> Self {
        self.submission_deadline = deadline;
        self
    }

    pub fn queue_delay(&self) -> f64 {
        (self.dispatch_time - self.callback_time).max(0.0)
    }

    /// Seconds left at handler start to submit CPU work; negative when overdue.
    pub fn budget(&self) -> f64 {
        self.submission_deadline - self.dispatch_time
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

pub(crate) fn utf16_len(text: &str) -> usize {
    text.chars().map(char::len_utf16).sum()
}

pub(crate) fn utf16_slice_exact(text: &str, range: std::ops::Range<usize>) -> Option<&str> {
    if range.start > range.end {
        return None;
    }
    let mut u16_pos = 0usize;
    let mut start_byte = (range.start == 0).then_some(0);
    let mut end_byte = (range.end == 0).then_some(0);
    for (byte_idx, ch) in text.char_indices() {
        if start_byte.is_some() && end_byte.is_some() {
            break;
        }
        let next_u16 = u16_pos + ch.len_utf16();
        let next_byte = byte_idx + ch.len_utf8();
        if next_u16 == range.start {
            start_byte = Some(next_byte);
        }
        if next_u16 == range.end {
            end_byte = Some(next_byte);
        }
        if (u16_pos < range.start && range.start < next_u16)
            || (u16_pos < range.end && range.end < next_u16)
        {
            return None;
        }
        u16_pos = next_u16;
    }
    Some(&text[start_byte?..end_byte?])
}

pub(crate) fn utf16_slice_clamped_expanding_surrogates(
    text: &str,
    raw_start: usize,
    raw_end: usize,
) -> Option<(usize, usize, &str)> {
    if raw_start > raw_end {
        return None;
    }
    let mut u16_pos = 0usize;
    let mut start = (raw_start == 0).then_some((0usize, 0usize));
    let mut end = (raw_end == 0).then_some((0usize, 0usize));
    for (byte_idx, ch) in text.char_indices() {
        let next_u16 = u16_pos + ch.len_utf16();
        let next_byte = byte_idx + ch.len_utf8();
        if start.is_none() {
            if raw_start == u16_pos || raw_start < next_u16 {
                start = Some((u16_pos, byte_idx));
            } else if next_u16 == raw_start {
                start = Some((next_u16, next_byte));
            }
        }
        if end.is_none() {
            if raw_end == u16_pos {
                end = Some((u16_pos, byte_idx));
            } else if raw_end < next_u16 || next_u16 == raw_end {
                end = Some((next_u16, next_byte));
            }
        }
        u16_pos = next_u16;
        if start.is_some() && end.is_some() {
            break;
        }
    }
    let (s_u16, s_byte) = start.unwrap_or((u16_pos, text.len()));
    let (e_u16, e_byte) = end.unwrap_or((u16_pos, text.len()));
    (s_u16 <= e_u16).then_some((s_u16, e_u16, &text[s_byte..e_byte]))
}

impl TextInputState {
    pub(crate) fn character_index_for_point(&self, p: Point) -> Option<usize> {
        let valid = |v: &&(usize, Point, Size)| v.2.height > 0.0 && v.2.width >= 0.0;
        let line_y = self
            .carets
            .iter()
            .filter(valid)
            .min_by(|a, b| {
                let da = (p.y - (a.1.y + a.2.height * 0.5)).abs();
                let db = (p.y - (b.1.y + b.2.height * 0.5)).abs();
                da.total_cmp(&db)
                    .then_with(|| (p.x - a.1.x).abs().total_cmp(&(p.x - b.1.x).abs()))
            })?
            .1
            .y;
        let mut line: Vec<_> = self
            .carets
            .iter()
            .filter(|v| valid(v) && (v.1.y - line_y).abs() < 0.01)
            .copied()
            .collect();
        line.sort_by_key(|v| v.0);
        for pair in line.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            if a.0 >= b.0 {
                continue;
            }
            let lo = a.1.x.min(b.1.x);
            let hi = a.1.x.max(b.1.x);
            if p.x >= lo && p.x <= hi {
                return Some(if (a.1.x - p.x).abs() <= (b.1.x - p.x).abs() {
                    a.0
                } else {
                    b.0
                });
            }
        }
        line.iter()
            .min_by(|a, b| (p.x - a.1.x).abs().total_cmp(&(p.x - b.1.x).abs()))
            .map(|v| v.0)
    }

    pub(crate) fn conversion_ended(previous: Option<&Self>, current: Option<&Self>) -> bool {
        previous.is_some_and(|old| {
            current.is_none()
                || (old.marked.is_some() && current.is_some_and(|s| s.marked.is_none()))
        })
    }

    /// First visible line fragment of a UTF-16 range, in content-view points.
    /// Geometry comes from the same shaped carets used by the editor. Invalid
    /// ranges or unavailable geometry return None instead of a different caret.
    pub fn first_rect_for_range(
        &self,
        range: std::ops::Range<usize>,
    ) -> Option<(std::ops::Range<usize>, Point, Size)> {
        utf16_slice_exact(&self.text, range.clone())?;
        Self::first_rect_for_validated_range(
            self.carets.iter(),
            Some((self.caret_index, self.caret_origin, self.caret_size)),
            range,
        )
    }

    pub(crate) fn first_rect_for_validated_range<'a>(
        carets: impl Iterator<Item = &'a (usize, Point, Size)> + Clone,
        fallback: Option<(usize, Point, Size)>,
        range: std::ops::Range<usize>,
    ) -> Option<(std::ops::Range<usize>, Point, Size)> {
        let valid = |p: Point, s: Size| {
            p.x.is_finite() && p.y.is_finite() && s.height.is_finite() && s.height > 0.
        };
        let (first, start, size) = carets
            .clone()
            .filter(|c| c.0 >= range.start && c.0 <= range.end && valid(c.1, c.2))
            .min_by_key(|c| c.0)
            .copied()
            .or_else(|| {
                fallback.filter(|&(idx, origin, sz)| idx == range.start && valid(origin, sz))
            })?;
        if range.is_empty() {
            let has_following = carets
                .clone()
                .any(|c| c.0 > range.start && valid(c.1, c.2) && (c.1.y - start.y).abs() < 0.01);
            let chosen = if has_following {
                carets
                    .clone()
                    .filter(|c| {
                        c.0 == range.start && valid(c.1, c.2) && (c.1.y - start.y).abs() < 0.01
                    })
                    .last()
                    .copied()
                    .unwrap_or((first, start, size))
            } else {
                (first, start, size)
            };
            return Some((
                range,
                chosen.1,
                Size {
                    width: 0.,
                    height: chosen.2.height,
                },
            ));
        }
        let mut line_carets: Vec<_> = carets
            .clone()
            .filter(|c| valid(c.1, c.2) && (c.1.y - start.y).abs() < 0.01)
            .copied()
            .collect();
        line_carets.sort_by_key(|c| c.0);
        let mut span_first: Option<usize> = None;
        let mut span_last: usize = 0;
        let mut span_min_x = f64::INFINITY;
        let mut span_max_x = f64::NEG_INFINITY;
        for pair in line_carets.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            if a.0 < b.0 && a.0 >= range.start && b.0 <= range.end {
                span_first.get_or_insert(a.0);
                span_last = b.0;
                span_min_x = span_min_x.min(a.1.x.min(b.1.x));
                span_max_x = span_max_x.max(a.1.x.max(b.1.x));
            }
        }
        if let Some(first_idx) = span_first
            && (span_max_x - span_min_x).is_finite()
        {
            return Some((
                first_idx..span_last,
                Point {
                    x: span_min_x,
                    y: start.y,
                },
                Size {
                    width: span_max_x - span_min_x,
                    height: size.height,
                },
            ));
        }
        let end = carets
            .clone()
            .filter(|c| {
                c.0 > first && c.0 <= range.end && valid(c.1, c.2) && (c.1.y - start.y).abs() < 0.01
            })
            .max_by_key(|c| c.0)?;
        let mut min_x = start.x.min(end.1.x);
        let mut max_x = start.x.max(end.1.x);
        for c in carets.filter(|c| {
            c.0 > first && c.0 < end.0 && valid(c.1, c.2) && (c.1.y - start.y).abs() < 0.01
        }) {
            min_x = min_x.min(c.1.x);
            max_x = max_x.max(c.1.x);
        }
        let width = max_x - min_x;
        width.is_finite().then_some((
            first..end.0,
            Point {
                x: min_x,
                y: start.y,
            },
            Size {
                width,
                height: size.height,
            },
        ))
    }
}

/// 平台无关的文本编辑命令，从 AppKit `NSStandardKeyBindingResponding` selector
/// 归一化而来。所有 Objective-C selector 字符串解析均收敛在本类型内。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TextEditCommand {
    MoveLeft,
    MoveLeftAndModifySelection,
    MoveRight,
    MoveRightAndModifySelection,
    MoveWordLeft,
    MoveWordLeftAndModifySelection,
    MoveWordRight,
    MoveWordRightAndModifySelection,
    MoveUp,
    MoveUpAndModifySelection,
    MoveDown,
    MoveDownAndModifySelection,
    MoveToBeginningOfLine,
    MoveToBeginningOfLineAndModifySelection,
    MoveToEndOfLine,
    MoveToEndOfLineAndModifySelection,
    MoveToBeginningOfParagraph,
    MoveToBeginningOfParagraphAndModifySelection,
    MoveToEndOfParagraph,
    MoveToEndOfParagraphAndModifySelection,
    MoveParagraphBackwardAndModifySelection,
    MoveParagraphForwardAndModifySelection,
    MoveToBeginningOfDocument,
    MoveToBeginningOfDocumentAndModifySelection,
    MoveToEndOfDocument,
    MoveToEndOfDocumentAndModifySelection,
    PageUp,
    PageDown,
    SelectAll,
    SelectLine,
    SelectParagraph,
    DeleteSelection,
    DeleteBackward,
    DeleteBackwardDecomposing,
    DeleteForward,
    DeleteWordBackward,
    DeleteWordForward,
    DeleteToBeginningOfLine,
    DeleteToEndOfLine,
    DeleteToBeginningOfParagraph,
    DeleteToEndOfParagraph,
    Yank,
    Transpose,
    InsertNewline,
    InsertLineBreak,
    InsertNewlineIgnoringFieldEditor,
    InsertTab,
    InsertBacktab,
    InsertTabIgnoringFieldEditor,
    CancelOperation,
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
}

impl TextEditCommand {
    /// Parse an AppKit `doCommandBySelector:` name into a platform-neutral editing command.
    pub fn from_objc_selector(selector: &str) -> Option<Self> {
        Some(match selector {
            "moveLeft:" | "moveBackward:" => Self::MoveLeft,
            "moveLeftAndModifySelection:" | "moveBackwardAndModifySelection:" => {
                Self::MoveLeftAndModifySelection
            }
            "moveRight:" | "moveForward:" => Self::MoveRight,
            "moveRightAndModifySelection:" | "moveForwardAndModifySelection:" => {
                Self::MoveRightAndModifySelection
            }
            "moveWordLeft:" | "moveWordBackward:" => Self::MoveWordLeft,
            "moveWordLeftAndModifySelection:" | "moveWordBackwardAndModifySelection:" => {
                Self::MoveWordLeftAndModifySelection
            }
            "moveWordRight:" | "moveWordForward:" => Self::MoveWordRight,
            "moveWordRightAndModifySelection:" | "moveWordForwardAndModifySelection:" => {
                Self::MoveWordRightAndModifySelection
            }
            "moveUp:" => Self::MoveUp,
            "moveUpAndModifySelection:" => Self::MoveUpAndModifySelection,
            "moveDown:" => Self::MoveDown,
            "moveDownAndModifySelection:" => Self::MoveDownAndModifySelection,
            "moveToBeginningOfLine:" | "moveToLeftEndOfLine:" => Self::MoveToBeginningOfLine,
            "moveToBeginningOfLineAndModifySelection:"
            | "moveToLeftEndOfLineAndModifySelection:" => {
                Self::MoveToBeginningOfLineAndModifySelection
            }
            "moveToEndOfLine:" | "moveToRightEndOfLine:" => Self::MoveToEndOfLine,
            "moveToEndOfLineAndModifySelection:" | "moveToRightEndOfLineAndModifySelection:" => {
                Self::MoveToEndOfLineAndModifySelection
            }
            "moveToBeginningOfParagraph:" => Self::MoveToBeginningOfParagraph,
            "moveToBeginningOfParagraphAndModifySelection:" => {
                Self::MoveToBeginningOfParagraphAndModifySelection
            }
            "moveToEndOfParagraph:" => Self::MoveToEndOfParagraph,
            "moveToEndOfParagraphAndModifySelection:" => {
                Self::MoveToEndOfParagraphAndModifySelection
            }
            "moveParagraphBackwardAndModifySelection:" => {
                Self::MoveParagraphBackwardAndModifySelection
            }
            "moveParagraphForwardAndModifySelection:" => {
                Self::MoveParagraphForwardAndModifySelection
            }
            "moveToBeginningOfDocument:" => Self::MoveToBeginningOfDocument,
            "moveToBeginningOfDocumentAndModifySelection:" => {
                Self::MoveToBeginningOfDocumentAndModifySelection
            }
            "moveToEndOfDocument:" => Self::MoveToEndOfDocument,
            "moveToEndOfDocumentAndModifySelection:" => Self::MoveToEndOfDocumentAndModifySelection,
            "pageUp:" | "scrollPageUp:" => Self::PageUp,
            "pageDown:" | "scrollPageDown:" => Self::PageDown,
            "selectAll:" => Self::SelectAll,
            "selectLine:" => Self::SelectLine,
            "selectParagraph:" => Self::SelectParagraph,
            "delete:" => Self::DeleteSelection,
            "deleteBackward:" => Self::DeleteBackward,
            "deleteBackwardByDecomposingPreviousCharacter:" => Self::DeleteBackwardDecomposing,
            "deleteForward:" => Self::DeleteForward,
            "deleteWordBackward:" => Self::DeleteWordBackward,
            "deleteWordForward:" => Self::DeleteWordForward,
            "deleteToBeginningOfLine:" => Self::DeleteToBeginningOfLine,
            "deleteToEndOfLine:" => Self::DeleteToEndOfLine,
            "deleteToBeginningOfParagraph:" => Self::DeleteToBeginningOfParagraph,
            "deleteToEndOfParagraph:" => Self::DeleteToEndOfParagraph,
            "yank:" => Self::Yank,
            "transpose:" => Self::Transpose,
            "insertNewline:" => Self::InsertNewline,
            "insertLineBreak:" => Self::InsertLineBreak,
            "insertNewlineIgnoringFieldEditor:" => Self::InsertNewlineIgnoringFieldEditor,
            "insertTab:" => Self::InsertTab,
            "insertBacktab:" => Self::InsertBacktab,
            "insertTabIgnoringFieldEditor:" => Self::InsertTabIgnoringFieldEditor,
            "cancelOperation:" => Self::CancelOperation,
            "undo:" => Self::Undo,
            "redo:" => Self::Redo,
            "cut:" => Self::Cut,
            "copy:" => Self::Copy,
            "paste:" => Self::Paste,
            _ => return None,
        })
    }
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
    /// A background `WindowWake` requested snapshot processing. Read the latest
    /// synchronized host data, then request redraw if needed. No animation or
    /// automatic periodic frame polling is enabled by this notification.
    Wake,
    /// 持续动画模式（[`Window::set_animating`](crate::Window::set_animating)）下，显示链接每次
    /// 刷新的回调。引擎在这里推进动画并画一帧，不需要再调 `request_redraw`。
    /// 只在动画模式开着的时候才会收到；关掉之后不再有回调，空闲时仍是 0 CPU。
    AnimationFrame(AnimationFrame),
    Mouse(MouseEvent),
    Scroll(ScrollEvent),
    Key(KeyEvent),
    /// Opt-in ordered provenance markers around the existing text callbacks.
    TextKey(crate::TextKeyEvent),
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
    /// 标准文本框与输入法路由后的编辑命令，已从 AppKit `NSStandardKeyBindingResponding`
    /// Objective-C selector 收敛为强类型命令；未知 selector 在平台边界忽略。
    TextCommand(TextEditCommand),
    /// 修饰键变化（`flagsChanged`）。
    ModifiersChanged(Modifiers),
    /// 窗口成为或不再是 key window。
    /// Active keyboard scope: application active AND window key. Application
    /// deactivation emits false even if this application's key window is retained.
    Focus(bool),
    /// Application quit was deferred by a window with pending work.
    QuitRequested,
    /// 菜单项被选中。
    Command(CommandId),
    /// VoiceOver 或其他辅助技术请求对宿主发布的节点执行动作。
    Accessibility {
        target: AccessibilityKey,
        action: AccessibilityAction,
    },
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
    fn conversion_discard_tracks_cleared_marked_text_and_owner_removal() {
        let idle = TextInputState::default();
        let marked = TextInputState {
            marked: Some(0..1),
            ..Default::default()
        };
        assert!(!TextInputState::conversion_ended(None, Some(&idle)));
        assert!(!TextInputState::conversion_ended(
            Some(&idle),
            Some(&marked)
        ));
        assert!(!TextInputState::conversion_ended(
            Some(&marked),
            Some(&marked)
        ));
        assert!(TextInputState::conversion_ended(Some(&marked), Some(&idle)));
        assert!(TextInputState::conversion_ended(Some(&marked), None));
        assert!(TextInputState::conversion_ended(Some(&idle), None));
        assert!(!TextInputState::conversion_ended(None, None));
    }

    #[test]
    fn candidate_rect_uses_the_first_visible_shaped_line_and_exact_utf16_fragment() {
        let mut state = TextInputState {
            text: "A😀BC\nDE".into(),
            ..Default::default()
        };
        state.carets = [
            (0, 0., 0.),
            (1, 10., 0.),
            (3, 35., 0.),
            (4, 45., 0.),
            (5, 55., 0.),
            (6, 0., 20.),
            (7, 10., 20.),
            (8, 20., 20.),
        ]
        .into_iter()
        .map(|(i, x, y)| {
            (
                i,
                Point { x, y },
                Size {
                    width: 1.,
                    height: 18.,
                },
            )
        })
        .collect();
        let (actual, origin, size) = state.first_rect_for_range(1..8).unwrap();
        assert_eq!(actual, 1..5);
        assert_eq!(origin, Point { x: 10., y: 0. });
        assert_eq!(size.width, 45.);
        let (actual, _, size) = state.first_rect_for_range(3..3).unwrap();
        assert_eq!(actual, 3..3);
        assert_eq!(size.width, 0.);
        assert!(state.first_rect_for_range(2..3).is_none());
        assert!(state.first_rect_for_range(0..usize::MAX).is_none());
        state.carets.retain(|c| c.0 >= 6);
        let (actual, _, _) = state.first_rect_for_range(0..8).unwrap();
        assert_eq!(actual, 6..8);
    }

    #[test]
    fn candidate_rect_supports_reverse_visual_carets_and_rejects_missing_geometry() {
        let mut state = TextInputState {
            text: "אב".into(),
            ..Default::default()
        };
        state.carets = vec![
            (
                0,
                Point { x: 80., y: 0. },
                Size {
                    width: 1.,
                    height: 18.,
                },
            ),
            (
                2,
                Point { x: 20., y: 0. },
                Size {
                    width: 1.,
                    height: 18.,
                },
            ),
        ];
        let (_, p, s) = state.first_rect_for_range(0..2).unwrap();
        assert_eq!(p.x, 20.);
        assert_eq!(s.width, 60.);
        state.carets.clear();
        assert!(state.first_rect_for_range(0..2).is_none());
    }

    #[test]
    fn animation_frame_interval_is_sanitized() {
        // 正常：用 duration。
        let f = AnimationFrame::new(10.0, 10.008, 10.002, 1.0 / 120.0);
        assert!((f.refresh_interval - 1.0 / 120.0).abs() < 1e-12);
        assert!((f.budget() - 0.006).abs() < 1e-12);
        // duration 为 0：退回 target - timestamp。
        let metal =
            AnimationFrame::new(10.0, 10.016, 10.002, 1.0 / 120.0).with_submission_deadline(10.008);
        assert_eq!(metal.target_timestamp, 10.016);
        assert!((metal.budget() - 0.006).abs() < 1e-12);
        let mut queued = metal;
        queued.dispatch_time = 10.010;
        assert!((queued.queue_delay() - 0.008).abs() < 1e-12);
        assert!((queued.budget() + 0.002).abs() < 1e-12);
        assert_eq!(queued.target_timestamp, 10.016);

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

#[cfg(test)]
mod issue102_no_geometry_tests {
    use super::*;
    #[test]
    fn issue102_no_geometry_keeps_marked_but_never_guesses_point_or_first_rect() {
        let old = TextInputState {
            text: "中".into(),
            selection: 1..1,
            marked: Some(0..1),
            caret_index: 1,
            ..Default::default()
        };
        let no_geometry = old.clone();
        assert!(!TextInputState::conversion_ended(
            Some(&old),
            Some(&no_geometry)
        ));
        assert_eq!(
            no_geometry.character_index_for_point(Point { x: 200., y: 200. }),
            None
        );
        assert!(no_geometry.first_rect_for_range(0..1).is_none());
        assert_eq!(no_geometry.marked, old.marked);
    }

    #[test]
    fn utf16_slice_helpers_match_surrogate_and_clamping_contracts() {
        let s = "a😀b";
        assert_eq!(utf16_len(s), 4);
        assert_eq!(utf16_slice_exact(s, 0..0), Some(""));
        assert_eq!(utf16_slice_exact(s, 1..1), Some(""));
        assert_eq!(utf16_slice_exact(s, 4..4), Some(""));
        assert_eq!(utf16_slice_exact(s, 1..3), Some("😀"));
        assert_eq!(utf16_slice_exact(s, 0..2), None);
        assert_eq!(utf16_slice_exact(s, 2..3), None);
        assert_eq!(utf16_slice_exact(s, 0..5), None);

        assert_eq!(
            utf16_slice_clamped_expanding_surrogates(s, 1, 1),
            Some((1, 1, ""))
        );
        assert_eq!(
            utf16_slice_clamped_expanding_surrogates(s, 2, 2),
            Some((1, 3, "😀"))
        );
        assert_eq!(
            utf16_slice_clamped_expanding_surrogates(s, 2, 100),
            Some((1, 4, "😀b"))
        );
        assert_eq!(
            utf16_slice_clamped_expanding_surrogates(s, 50, 100),
            Some((4, 4, ""))
        );
    }

    #[test]
    fn bidi_split_carets_resolve_exact_character_rect_and_hit_test() {
        let state = TextInputState {
            text: "Hello שלום".into(),
            carets: vec![
                (0, 0., 0.),
                (5, 32., 0.),
                (6, 36., 0.),
                (6, 68., 0.),
                (7, 57., 0.),
                (8, 50., 0.),
                (9, 46., 0.),
                (10, 36., 0.),
            ]
            .into_iter()
            .map(|(i, x, y)| {
                (
                    i,
                    Point { x, y },
                    Size {
                        width: 1.,
                        height: 18.,
                    },
                )
            })
            .collect(),
            ..Default::default()
        };
        let (actual, origin, size) = state.first_rect_for_range(6..7).unwrap();
        assert_eq!(actual, 6..7);
        assert_eq!(origin, Point { x: 57., y: 0. });
        assert_eq!(size.width, 11.);

        let (actual, origin, size) = state.first_rect_for_range(5..6).unwrap();
        assert_eq!(actual, 5..6);
        assert_eq!(origin, Point { x: 32., y: 0. });
        assert_eq!(size.width, 4.);

        let (actual, origin, size) = state.first_rect_for_range(6..10).unwrap();
        assert_eq!(actual, 6..10);
        assert_eq!(origin, Point { x: 36., y: 0. });
        assert_eq!(size.width, 32.);

        assert_eq!(
            state.character_index_for_point(Point { x: 33., y: 9. }),
            Some(5)
        );
        assert_eq!(
            state.character_index_for_point(Point { x: 66., y: 9. }),
            Some(6)
        );
        assert_eq!(
            state.character_index_for_point(Point { x: 59., y: 9. }),
            Some(7)
        );
    }

    #[test]
    fn objc_selectors_map_all_supported_editing_commands_and_aliases() {
        assert_eq!(
            TextEditCommand::from_objc_selector("moveBackward:"),
            Some(TextEditCommand::MoveLeft)
        );
        assert_eq!(
            TextEditCommand::from_objc_selector("moveLeftAndModifySelection:"),
            Some(TextEditCommand::MoveLeftAndModifySelection)
        );
        assert_eq!(
            TextEditCommand::from_objc_selector("moveBackwardAndModifySelection:"),
            Some(TextEditCommand::MoveLeftAndModifySelection)
        );
        assert_eq!(
            TextEditCommand::from_objc_selector("moveToLeftEndOfLine:"),
            Some(TextEditCommand::MoveToBeginningOfLine)
        );
        assert_eq!(
            TextEditCommand::from_objc_selector("moveToRightEndOfLineAndModifySelection:"),
            Some(TextEditCommand::MoveToEndOfLineAndModifySelection)
        );
        assert_eq!(
            TextEditCommand::from_objc_selector("scrollPageUp:"),
            Some(TextEditCommand::PageUp)
        );
        assert_eq!(
            TextEditCommand::from_objc_selector("scrollPageDown:"),
            Some(TextEditCommand::PageDown)
        );
        assert_eq!(
            TextEditCommand::from_objc_selector("deleteBackwardByDecomposingPreviousCharacter:"),
            Some(TextEditCommand::DeleteBackwardDecomposing)
        );
        assert_eq!(
            TextEditCommand::from_objc_selector("insertNewlineIgnoringFieldEditor:"),
            Some(TextEditCommand::InsertNewlineIgnoringFieldEditor)
        );
        assert_eq!(
            TextEditCommand::from_objc_selector("insertTabIgnoringFieldEditor:"),
            Some(TextEditCommand::InsertTabIgnoringFieldEditor)
        );
        assert_eq!(
            TextEditCommand::from_objc_selector("unknownSelector:"),
            None
        );
    }
}
