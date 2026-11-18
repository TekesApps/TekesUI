//! Deterministic selftest action schedules driving the default AppHost Gallery window.
use crate::gallery::workbench;
use tekes_ui_platform_macos::Modifiers;
use tekes_ui_platform_macos::selftest::Action;

const CMD_APPEARANCE_LIGHT: u32 = 201;
const CMD_APPEARANCE_DARK: u32 = 202;
const CMD_CONTRAST_ON: u32 = 301;
const CMD_CONTRAST_STANDARD: u32 = 302;
const CMD_SCALE_100: u32 = 540;
const CMD_SCALE_125: u32 = 541;
const CMD_SCALE_150: u32 = 542;
const CMD_ROUTE_BASE: u32 = 1000;

fn route_command(slug: &str) -> u32 {
    let idx = workbench::entries()
        .iter()
        .position(|entry| entry.slug() == slug)
        .unwrap_or(0);
    CMD_ROUTE_BASE + idx as u32
}

fn cap(out: &str, name: &str) -> Action {
    Action::Capture {
        path: format!("{out}/{name}.png"),
    }
}

pub fn script_for_mode(mode: &str) -> Vec<(u64, Action)> {
    let out = std::env::var("TEKES_SELFTEST_OUT").unwrap_or_else(|_| ".".into());
    match mode {
        "gallery" => gallery_script(&out),
        "workbench" | "workbench-extended" | "components" => route_tour_script(
            &out,
            mode,
            &[
                "component-button",
                "component-checkbox",
                "component-switch",
                "component-slider",
                "component-select",
                "component-tabs",
            ],
        ),
        "foundation" => route_tour_script(
            &out,
            mode,
            &[
                "foundation-tokens",
                "foundation-color-surfaces",
                "foundation-typography",
                "foundation-motion",
            ],
        ),
        "text" | "reflow" | "scroll-repair" => reflow_script(&out, mode),
        "motion-layout" => motion_layout_script(&out),
        "settings-native207" => settings_route_script(&out),
        "metal-lifecycle" | "metal-lifecycle-hold" => lifecycle_script(&out),
        _ => resize_script(&out),
    }
}

fn resize_script(out: &str) -> Vec<(u64, Action)> {
    let sizes = [
        ("s1_800x600", 800.0, 600.0),
        ("s2_1000x520", 1000.0, 520.0),
        ("s3_1500x900", 1500.0, 900.0),
        ("s4_1200x800", 1200.0, 800.0),
    ];
    let mut steps = vec![(1500, cap(out, "s0_initial_1200x800"))];
    let mut t = 2500;
    for (name, w, h) in sizes {
        steps.push((t, Action::Resize { w, h }));
        steps.push((t + 700, cap(out, name)));
        t += 1500;
    }
    steps.push((t, Action::Log("minimize".into())));
    steps.push((t + 100, Action::Miniaturize));
    steps.push((t + 2000, Action::Log("restore".into())));
    steps.push((t + 2100, Action::Deminiaturize));
    steps.push((t + 3500, cap(out, "s5_after_restore")));
    steps.push((t + 4500, Action::Close));
    steps
}

fn gallery_script(out: &str) -> Vec<(u64, Action)> {
    let mut steps = Vec::new();
    let mut t = 1400;
    let mut at = |dt: u64, action: Action| {
        t += dt;
        steps.push((t, action));
    };
    at(0, Action::WindowDark(false));
    at(0, Action::MenuCommand(CMD_APPEARANCE_LIGHT));
    at(0, Action::MenuCommand(CMD_CONTRAST_STANDARD));
    at(100, Action::MenuCommand(route_command("foundation-tokens")));
    at(700, cap(out, "g01_tokens_light"));
    at(100, Action::MenuCommand(CMD_APPEARANCE_DARK));
    at(700, cap(out, "g02_tokens_dark"));
    at(100, Action::MenuCommand(CMD_CONTRAST_ON));
    at(700, cap(out, "g03_tokens_dark_ic"));
    at(200, Action::Close);
    steps
}

fn route_tour_script(out: &str, prefix: &str, routes: &[&str]) -> Vec<(u64, Action)> {
    let mut steps = Vec::new();
    let mut t = 1400;
    let mut at = |dt: u64, action: Action| {
        t += dt;
        steps.push((t, action));
    };
    at(0, Action::MenuCommand(CMD_APPEARANCE_LIGHT));
    at(0, Action::MenuCommand(CMD_CONTRAST_STANDARD));
    for (idx, slug) in routes.iter().enumerate() {
        at(100, Action::MenuCommand(route_command(slug)));
        at(700, cap(out, &format!("{prefix}_{idx:02}_{slug}")));
    }
    at(200, Action::Close);
    steps
}

fn reflow_script(out: &str, prefix: &str) -> Vec<(u64, Action)> {
    let mut steps = Vec::new();
    let mut t = 1400;
    let mut at = |dt: u64, action: Action| {
        t += dt;
        steps.push((t, action));
    };
    at(0, Action::MenuCommand(CMD_APPEARANCE_LIGHT));
    at(0, Action::MenuCommand(CMD_CONTRAST_STANDARD));
    for slug in ["component-scroll-area", "component-text-area", "lab-text"] {
        at(100, Action::MenuCommand(route_command(slug)));
        at(200, Action::MenuCommand(CMD_SCALE_150));
        at(200, Action::Resize { w: 720.0, h: 480.0 });
        at(700, cap(out, &format!("{prefix}_{slug}_720_150")));
        at(200, Action::MenuCommand(CMD_SCALE_100));
        at(
            200,
            Action::Resize {
                w: 1200.0,
                h: 800.0,
            },
        );
    }
    at(200, Action::Close);
    steps
}

fn motion_layout_script(out: &str) -> Vec<(u64, Action)> {
    let mut steps = Vec::new();
    let mut t = 1400;
    let mut at = |dt: u64, action: Action| {
        t += dt;
        steps.push((t, action));
    };
    at(0, Action::MenuCommand(CMD_APPEARANCE_LIGHT));
    at(0, Action::MenuCommand(CMD_CONTRAST_STANDARD));
    at(100, Action::MenuCommand(route_command("foundation-motion")));
    for (label, scale_cmd, w, h) in [
        ("720_125", CMD_SCALE_125, 720.0, 640.0),
        ("800_150", CMD_SCALE_150, 800.0, 640.0),
        ("1200_100", CMD_SCALE_100, 1200.0, 800.0),
    ] {
        at(100, Action::MenuCommand(scale_cmd));
        at(100, Action::Resize { w, h });
        at(700, cap(out, &format!("motion_{label}")));
    }
    at(200, Action::Close);
    steps
}

fn settings_route_script(out: &str) -> Vec<(u64, Action)> {
    let mut steps = Vec::new();
    let mut t = 1400;
    let mut at = |dt: u64, action: Action| {
        t += dt;
        steps.push((t, action));
    };
    at(0, Action::MenuCommand(CMD_APPEARANCE_LIGHT));
    at(0, Action::MenuCommand(CMD_CONTRAST_STANDARD));
    at(100, Action::MenuCommand(route_command("example-settings")));
    at(
        200,
        Action::Resize {
            w: 1440.0,
            h: 960.0,
        },
    );
    at(200, Action::MenuCommand(CMD_SCALE_150));
    at(200, Action::Resize { w: 720.0, h: 512.0 });
    at(200, Action::TextCommand("insertTab:"));
    at(
        200,
        Action::KeyCharactersModified {
            key_code: 48,
            chars: "\u{19}",
            modifiers: Modifiers {
                shift: true,
                ..Modifiers::NONE
            },
        },
    );
    at(400, cap(out, "settings_native207_apphost"));
    at(200, Action::Close);
    steps
}

fn lifecycle_script(out: &str) -> Vec<(u64, Action)> {
    let mut steps = Vec::new();
    let mut t = 1400;
    let mut at = |dt: u64, action: Action| {
        t += dt;
        steps.push((t, action));
    };
    at(100, Action::Resize { w: 960.0, h: 640.0 });
    at(500, Action::Miniaturize);
    at(1500, Action::Deminiaturize);
    at(700, cap(out, "metal_lifecycle_restored"));
    at(300, Action::Close);
    steps
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_selftest_modes_produce_bounded_scripts_ending_in_close() {
        for mode in [
            "1",
            "gallery",
            "workbench",
            "workbench-extended",
            "settings-native207",
            "scroll-repair",
            "reflow",
            "text",
            "foundation",
            "components",
            "motion-layout",
            "metal-lifecycle",
            "metal-lifecycle-hold",
        ] {
            let script = script_for_mode(mode);
            assert!(!script.is_empty(), "empty script for {mode}");
            assert!(
                matches!(script.last().unwrap().1, Action::Close),
                "script for {mode} must terminate with Close"
            );
        }
    }
}
