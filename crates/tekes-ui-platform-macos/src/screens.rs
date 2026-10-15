//! 枚举屏幕、按 `WindowPlacement` 摆放窗口。规则本身在 `placement.rs`，这里只做 AppKit 部分。

use objc2::rc::Retained;
use objc2_app_kit::{NSScreen, NSWindow};
use objc2_core_graphics::CGDisplayIsBuiltin;
use objc2_foundation::{MainThreadMarker, NSNumber, NSPoint, NSRect, NSString};

use crate::event::{Point, Size};
use crate::placement::{self, PlacementReport, Rect, ScreenInfo, WindowPlacement};

fn rect(r: NSRect) -> Rect {
    Rect {
        origin: Point {
            x: r.origin.x,
            y: r.origin.y,
        },
        size: Size {
            width: r.size.width,
            height: r.size.height,
        },
    }
}

fn is_builtin(screen: &NSScreen) -> bool {
    let key = NSString::from_str("NSScreenNumber");
    let desc = screen.deviceDescription();
    let Some(obj) = desc.objectForKey(&key) else {
        return false;
    };
    let Ok(number) = obj.downcast::<NSNumber>() else {
        return false;
    };
    CGDisplayIsBuiltin(number.unsignedIntValue())
}

pub(crate) fn info(screen: &NSScreen) -> ScreenInfo {
    ScreenInfo {
        name: screen.localizedName().to_string(),
        builtin: is_builtin(screen),
        scale: screen.backingScaleFactor(),
        max_fps: screen.maximumFramesPerSecond().max(0) as u32,
        frame: rect(screen.frame()),
        visible_frame: rect(screen.visibleFrame()),
    }
}

/// `NSScreen.screens`，下标 0 是主屏。
pub(crate) fn enumerate(mtm: MainThreadMarker) -> Vec<ScreenInfo> {
    NSScreen::screens(mtm).iter().map(|s| info(&s)).collect()
}

/// 窗口当前所在屏幕的信息。
pub(crate) fn current(window: &NSWindow) -> Option<ScreenInfo> {
    let screen: Retained<NSScreen> = window.screen()?;
    Some(info(&screen))
}

/// 读环境变量覆盖，再按规则把窗口摆到位。返回选择结果（日志用）。
/// `Centered` 走 AppKit 的 `center()`。
pub(crate) fn place(
    mtm: MainThreadMarker,
    window: &NSWindow,
    configured: WindowPlacement,
) -> Option<PlacementReport> {
    let mut placement = configured;
    if let Ok(v) = std::env::var(placement::PLACEMENT_ENV) {
        match placement::parse_override(&v, configured) {
            Some(p) => placement = p,
            None => eprintln!(
                "{} 的值 {v:?} 不认识，忽略（可用：auto external builtin builtin-right-half centered）",
                placement::PLACEMENT_ENV
            ),
        }
    }
    let screens = enumerate(mtm);
    let report = placement::resolve(placement, &screens);
    match &report {
        Some(r) => {
            let frame = window.frame();
            let size = Size {
                width: frame.size.width,
                height: frame.size.height,
            };
            match placement::origin_for(r.layout, size, r.screen.visible_frame) {
                Some(p) => window.setFrameOrigin(NSPoint::new(p.x, p.y)),
                None => window.center(),
            }
        }
        None => window.center(),
    }
    report
}
