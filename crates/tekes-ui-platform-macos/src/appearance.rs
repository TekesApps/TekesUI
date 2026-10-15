//! 系统外观：深浅色与“增强对比度”。

use objc2_app_kit::{
    NSAppearanceCustomization, NSAppearanceNameAqua, NSAppearanceNameDarkAqua, NSView, NSWorkspace,
};
use objc2_foundation::NSArray;

/// 读取视图当前生效的外观。返回 `(dark, increase_contrast)`。
///
/// 深浅色取自视图的 `effectiveAppearance`，所以窗口或应用单独设置的外观覆盖也会反映出来。
/// 增强对比度取自 `NSWorkspace.accessibilityDisplayShouldIncreaseContrast`，
/// 这是系统设置“辅助功能 > 显示 > 增强对比度”的开关。
pub(crate) fn read(view: &NSView) -> (bool, bool) {
    let appearance = view.effectiveAppearance();
    // SAFETY: 这两个是 AppKit 导出的全局常量，进程生命周期内有效。
    let names = unsafe { NSArray::from_slice(&[NSAppearanceNameAqua, NSAppearanceNameDarkAqua]) };
    let best = appearance.bestMatchFromAppearancesWithNames(&names);
    let dark = best
        .map(|n| &*n == unsafe { NSAppearanceNameDarkAqua })
        .unwrap_or(false);
    let increase_contrast =
        NSWorkspace::sharedWorkspace().accessibilityDisplayShouldIncreaseContrast();
    (dark, increase_contrast)
}
