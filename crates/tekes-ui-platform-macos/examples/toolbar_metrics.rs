//! Hidden native geometry oracle. Never shown, used as the default-height reference.
#[cfg(not(target_os = "macos"))]
fn main() {}
#[cfg(target_os = "macos")]
fn main() {
    use objc2::MainThreadOnly;
    use objc2_app_kit::{
        NSApplication, NSApplicationActivationPolicy, NSBackingStoreType, NSFont, NSToolbar,
        NSWindow, NSWindowButton, NSWindowStyleMask, NSWindowTitleVisibility, NSWindowToolbarStyle,
    };
    use objc2_foundation::{MainThreadMarker, NSPoint, NSRect, NSSize, NSString};
    let mtm = MainThreadMarker::new().unwrap();
    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(NSApplicationActivationPolicy::Prohibited);
    let mut rows = Vec::new();
    for (name, style, full_size, hidden) in [
        ("automatic", NSWindowToolbarStyle::Automatic, false, false),
        ("unified", NSWindowToolbarStyle::Unified, false, false),
        (
            "unified-compact",
            NSWindowToolbarStyle::UnifiedCompact,
            false,
            false,
        ),
        ("expanded", NSWindowToolbarStyle::Expanded, false, false),
        (
            "unified-fullsize-hidden",
            NSWindowToolbarStyle::Unified,
            true,
            true,
        ),
    ] {
        let window = unsafe {
            NSWindow::initWithContentRect_styleMask_backing_defer(
                NSWindow::alloc(mtm),
                NSRect::new(NSPoint::new(0., 0.), NSSize::new(720., 480.)),
                NSWindowStyleMask::Titled
                    | NSWindowStyleMask::Closable
                    | NSWindowStyleMask::Resizable
                    | if full_size {
                        NSWindowStyleMask::FullSizeContentView
                    } else {
                        NSWindowStyleMask::empty()
                    },
                NSBackingStoreType::Buffered,
                false,
            )
        };
        unsafe { window.setReleasedWhenClosed(false) };
        window.setTitle(&NSString::from_str(
            "TekesUI hidden toolbar geometry oracle",
        ));
        window.setToolbarStyle(style);
        if hidden {
            window.setTitleVisibility(NSWindowTitleVisibility::Hidden);
            window.setTitlebarAppearsTransparent(true);
        }
        let toolbar = NSToolbar::initWithIdentifier(
            NSToolbar::alloc(mtm),
            &NSString::from_str(&format!("TekesUI.Toolbar.Metrics.{name}")),
        );
        window.setToolbar(Some(&toolbar));
        if let Some(view) = window.contentView() {
            view.layoutSubtreeIfNeeded();
        }
        let frame = window.frame();
        let layout = window.contentLayoutRect();
        let content = window.contentRectForFrameRect(frame);
        let mut controls = Vec::new();
        if let Some(view) = window.contentView() {
            for (kind, label) in [
                (NSWindowButton::CloseButton, "close"),
                (NSWindowButton::MiniaturizeButton, "minimize"),
                (NSWindowButton::ZoomButton, "zoom"),
            ] {
                if let Some(button) = window.standardWindowButton(kind) {
                    let r = view.convertRect_fromView(button.bounds(), Some(&button));
                    let top = view.bounds().size.height - r.origin.y - r.size.height;
                    controls.push(serde_json::json!({"kind":label,"hidden":button.isHidden(),"x":r.origin.x,"top":top,"width":r.size.width,"height":r.size.height,"center_y_from_top":top+r.size.height/2.}));
                }
            }
        }
        rows.push(serde_json::json!({"native_controls":controls,"style":name,"frame_height":frame.size.height,"content_height":content.size.height,"content_layout_height":layout.size.height,"combined_title_toolbar_height":frame.size.height-layout.size.height,"toolbar_below_title_height":content.size.height-layout.size.height,"window_shown":window.isVisible(),"view_height":window.contentView().map(|v|v.bounds().size.height)}));
        window.close();
    }
    let fonts = [11.,12.,13.,14.,15.,16.,17.,18.,20.].map(|size| {
        let font = NSFont::systemFontOfSize(size);
        serde_json::json!({"point_size":size,"cap_height":font.capHeight(),"ascender":font.ascender(),"descender":font.descender(),"x_height":font.xHeight()})
    });
    println!("{}",serde_json::to_string_pretty(&serde_json::json!({"kind":"AppKit native hidden-window geometry and font metrics; not GPU/native interaction acceptance","cases":rows,"system_fonts":fonts})).unwrap());
}
