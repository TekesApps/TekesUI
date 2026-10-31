//! #207: local AppKit selectors and NSEvents on the original Gallery Host.
use tekes_ui_platform_macos::selftest::Action;
pub(crate) fn script() -> Vec<(u64, Action)> {
    let out = std::env::var("TEKES_SELFTEST_OUT").expect("#207 output");
    let mut steps = Vec::new();
    let mut time = 500;
    let mut add = |action| {
        time += 600;
        steps.push((time, action));
    };
    let click = |label| Action::ClickPublished {
        label,
        secondary: false,
    };
    let cap = |name| Action::Capture {
        path: format!("{out}/{name}.png"),
    };
    add(Action::MenuCommand(201));
    add(Action::MenuCommand(302));
    add(Action::Resize { w: 1440., h: 960. });
    add(cap("01-ready"));
    add(Action::MenuCommand(542));
    add(Action::Resize { w: 720., h: 512. });
    // Traverse the original Ready page with selectors, then local NSEvents.
    add(Action::Log("207-selector-forward".into()));
    add(Action::TextCommand("insertTab:"));
    for _ in 0..24 {
        add(Action::TextCommand("insertTab:"));
    }
    add(cap("06-selector-forward"));
    add(Action::Log("207-selector-reverse".into()));
    for _ in 0..24 {
        add(Action::TextCommand("insertBacktab:"));
    }
    add(cap("07-selector-reverse"));
    add(Action::Log("207-local-forward".into()));
    for _ in 0..24 {
        add(Action::KeyCharacters {
            key_code: 48,
            chars: "\t",
        });
    }
    add(Action::Log("207-local-reverse".into()));
    for _ in 0..24 {
        add(Action::KeyCharactersModified {
            key_code: 48,
            chars: "\u{19}",
            modifiers: tekes_ui_platform_macos::Modifiers {
                shift: true,
                ..tekes_ui_platform_macos::Modifiers::NONE
            },
        });
    }
    add(cap("08-native-key-reverse"));
    add(Action::MenuCommand(540));
    add(Action::Resize { w: 1440., h: 960. });
    let p =
        crate::gallery::workbench::Workbench::fixture_point("example-settings/playground", 1000);
    add(Action::Click {
        x: p.x as f64,
        y: p.y as f64,
    });
    add(Action::TextCommand("selectAll:"));
    add(Action::TextCommand("deleteBackward:"));
    add(Action::ExpectText {
        text: "".into(),
        marked: false,
    });
    add(click("Save settings"));
    add(cap("02-invalid"));
    add(Action::Click {
        x: p.x as f64,
        y: p.y as f64,
    });
    add(Action::Text("Settings native 207".into()));
    add(Action::ExpectText {
        text: "Settings native 207".into(),
        marked: false,
    });
    add(Action::TextCommand("moveToBeginningOfDocument:"));
    add(Action::TextCommand("moveToEndOfDocument:"));
    add(click("Save settings"));
    add(cap("03-pending"));
    add(click("Resolve fixture"));
    add(cap("04-failed"));
    add(click("Retry save"));
    add(click("Resolve fixture"));
    add(cap("05-saved"));
    add(Action::Close);
    steps
}
