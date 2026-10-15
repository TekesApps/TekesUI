//! Gallery 应用：恒定的人工目测验收目标，见章程第 2 节。
//!
//! 结构：
//! - `host.rs`：平台层与渲染层的接线（窗口事件 → Renderer、主题跟随系统外观）；
//! - `gallery.rs`：Gallery 自己的状态（页面、布局、主题相关的内容），目前只有占位页；
//! - 本文件：入口、窗口配置、自测模式。
//!
//! 自测模式 `TEKES_SELFTEST=1`：按时间表缩放窗口、截图、最小化再恢复，然后正常退出，
//! 截图写到 `TEKES_SELFTEST_OUT` 指定的目录。见 docs/handoffs/F0-measurements.md。

#[cfg(target_os = "macos")]
mod gallery;
#[cfg(target_os = "macos")]
mod host;

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("Gallery 目前只支持 macOS");
}

#[cfg(target_os = "macos")]
fn main() {
    use tekes_ui_platform_macos::{
        App, AppConfig, MenuBar, Size, WindowConfig, WindowPlacement, selftest,
    };

    let selftest = std::env::var("TEKES_SELFTEST").is_ok();
    let window_config = WindowConfig {
        title: "TekesUI Gallery".into(),
        // 固定的内容区尺寸，Gallery 的每个页面都按它排版，保证可复现。
        size: Size {
            width: gallery::CONTENT_WIDTH,
            height: gallery::CONTENT_HEIGHT,
        },
        min_size: Some(Size {
            width: 720.0,
            height: 480.0,
        }),
        // 窗口摆放的开发规则：外接屏优先，否则内建屏右半边。自测会缩放窗口，属于尺寸类检查。
        // 可用环境变量 TEKESUI_WINDOW_PLACEMENT 覆盖。
        placement: WindowPlacement::Auto {
            need_120hz: false,
            size_check: selftest,
        },
        // 不保存窗口位置与尺寸：需要固定、可复现的尺寸。
        frame_autosave_name: None,
    };
    if selftest {
        selftest::run_script(selftest_script());
    }
    App::new(AppConfig {
        name: Some("TekesUI Gallery".into()),
        ..AppConfig::default()
    })
    .run(window_config, MenuBar::default(), host::Host::new());
}

/// 自测时间表：初始截图；缩放到几个尺寸各截一张；最小化再恢复，截图确认内容还在。
#[cfg(target_os = "macos")]
fn selftest_script() -> Vec<(u64, tekes_ui_platform_macos::selftest::Action)> {
    use tekes_ui_platform_macos::selftest::Action;

    let out = std::env::var("TEKES_SELFTEST_OUT").unwrap_or_else(|_| ".".into());
    let cap = |name: &str| Action::Capture {
        path: format!("{out}/{name}.png"),
    };
    let sizes = [
        ("s1_800x600", 800.0, 600.0),
        ("s2_1000x520", 1000.0, 520.0),
        ("s3_1500x900", 1500.0, 900.0),
        ("s4_1200x800", 1200.0, 800.0),
    ];
    let mut steps = vec![(1500, cap("s0_initial_1200x800"))];
    let mut t = 2500;
    for (name, w, h) in sizes {
        steps.push((t, Action::Resize { w, h }));
        steps.push((t + 700, cap(name)));
        t += 1500;
    }
    steps.push((t, Action::Log("最小化".into())));
    steps.push((t + 100, Action::Miniaturize));
    steps.push((t + 2000, Action::Log("恢复".into())));
    steps.push((t + 2100, Action::Deminiaturize));
    steps.push((t + 3500, cap("s5_after_restore")));
    steps.push((t + 4500, Action::Close));
    steps
}
